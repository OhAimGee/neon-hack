#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"

#include "../../src/core/platform.h"
#include "../../src/game/commands.h"
#include "../../src/game/progression.h"
#include "../../src/game/tutorial.h"
#include "../../src/game/world.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    nh_feed(""); /* jamais d'attente sur le vrai clavier */
    return gs;
}

static GameState *new_tutorial(void)
{
    GameState *gs = new_game();
    nh_tutorial_start(gs);
    return gs;
}

static NhDispatch run(GameState *gs, const char *line, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    NhDispatch r = nh_dispatch(gs, line);
    nh_capture_end(&cap, out, size);
    return r;
}

static void compromise_localhost(GameState *gs)
{
    int idx = nh_world_find(gs->nodes, "localhost");
    CHECK(idx >= 0);
    if (idx >= 0)
        gs->nodes[idx].is_compromised = true;
}

static void test_inactive_by_default(void)
{
    GameState *gs = new_game();
    char out[8192];
    CHECK(!nh_tutorial_active(gs));
    CHECK_INT(gs->tutorial.step, NH_TUT_NONE);
    CHECK(!gs->tutorial.done);

    /* un état neuf ne déclenche jamais de réplique, quoi qu'on tape */
    run(gs, "help", out, sizeof out);
    CHECK(!has(out, "ECHO-7"));
    run(gs, "n'importe quoi", out, sizeof out);
    CHECK(!has(out, "ECHO-7"));

    NhCapture cap = nh_capture_begin();
    nh_tutorial_announce(gs, false);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_STR(out, "");

    free(gs);
}

static void test_start_and_mission_log(void)
{
    GameState *gs = new_tutorial();
    char out[8192];
    nh_set_lang(NH_LANG_FR);

    CHECK(nh_tutorial_active(gs));
    CHECK_INT(gs->tutorial.step, NH_TUT_QUESTS);

    /* `quests` montre la mission d'ECHO-7 au lieu de l'ancien journal (et sans rien demander) */
    CHECK_INT(run(gs, "quests", out, sizeof out) == NH_DISPATCH_OK, 1);
    CHECK(has(out, "ECHO-7"));
    CHECK(has(out, "Premiers Pas"));
    CHECK(has(out, "Ouvrir le journal de mission"));
    CHECK(!has(out, "Voulez-vous voir les détails"));
    free(gs);
}

static void test_full_walkthrough(void)
{
    GameState *gs = new_tutorial();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    int credits0 = gs->player.credits;
    int rep0 = gs->player.reputation;

    /* étape 1 : ouvrir le journal */
    run(gs, "quests", out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_HELP);
    CHECK(has(out, "Consulter l'aide")); /* le journal montre déjà la suite */

    /* étape 2 : help */
    run(gs, "help", out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_SCAN);
    CHECK(has(out, "ECHO-7"));

    /* étape 3 : scan */
    run(gs, "scan", out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_STATUS);
    CHECK(has(out, "ECHO-7"));

    /* étape 4 : status — l'expérience du premier scan ne suffit pas pour le niveau 2 */
    CHECK_INT(gs->player.level, LEVEL_NOVICE);
    run(gs, "status", out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_LEVELUP);

    /* étape 5 : le niveau 2 se lit dans l'état du jeu, quelle que soit la façon de l'obtenir */
    NhCapture cap = nh_capture_begin();
    nh_grant_xp(gs, 15);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.level, LEVEL_APPRENTICE);
    run(gs, "status", out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_BRUTEFORCE);

    /* étape 6 : localhost compromis */
    compromise_localhost(gs);
    run(gs, "status", out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_LAYLOW);
    CHECK_INT(gs->player.credits, credits0); /* pas de récompense avant la fin */

    /* étape 7 : annuler le menu ne compte pas, appliquer une méthode oui */
    gs->alert.level = 25;
    nh_feed("0\n");
    CHECK(run(gs, "laylow", out, sizeof out) == NH_DISPATCH_OK);
    CHECK_INT(gs->tutorial.step, NH_TUT_LAYLOW);
    CHECK(nh_tutorial_active(gs));

    int credits_before = gs->player.credits;
    nh_feed("1\n");
    CHECK(run(gs, "laylow", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(!nh_tutorial_active(gs));
    CHECK(gs->tutorial.done);
    CHECK_INT(gs->tutorial.step, NH_TUT_NONE);
    CHECK(has(out, "MISSION ACCOMPLIE"));
    CHECK(has(out, "Testeur")); /* ECHO-7 s'adresse au joueur par son nom */
    CHECK_INT(gs->player.reputation, rep0 + NH_TUT_REWARD_REPUTATION);
    /* les crédits comprennent la récompense (le coût éventuel de la méthode est déjà décompté) */
    CHECK(gs->player.credits >= credits_before + NH_TUT_REWARD_CREDITS - 100);

    /* la récompense n'est versée qu'une fois : plus rien ne bouge ensuite */
    int credits_done = gs->player.credits;
    int rep_done = gs->player.reputation;
    run(gs, "status", out, sizeof out);
    run(gs, "help", out, sizeof out);
    run(gs, "quests", out, sizeof out);
    CHECK_INT(gs->player.credits, credits_done);
    CHECK_INT(gs->player.reputation, rep_done);
    CHECK(!has(out, "ECHO-7"));
    CHECK(has(out, "Voulez-vous voir")); /* le journal d'origine est de retour */

    free(gs);
}

static void test_reward_exact(void)
{
    /* même scénario, mais on mesure la récompense au crédit près avec une méthode gratuite */
    GameState *gs = new_tutorial();
    char out[16384];
    gs->tutorial.step = NH_TUT_LAYLOW;
    gs->player.level = LEVEL_APPRENTICE;
    compromise_localhost(gs);
    gs->alert.level = 30;
    gs->player.credits = 500;
    gs->player.reputation = 3;

    /* on applique la réduction en direct pour connaître son coût exact */
    nh_feed("1\n");
    run(gs, "laylow", out, sizeof out);
    CHECK(gs->tutorial.done);
    CHECK_INT(gs->player.reputation, 3 + NH_TUT_REWARD_REPUTATION);
    int spent = 500 + NH_TUT_REWARD_CREDITS - gs->player.credits;
    CHECK(spent >= 0 && spent < 100); /* seul le coût de la méthode a été retiré */
    free(gs);
}

static void test_out_of_order(void)
{
    GameState *gs = new_tutorial();
    char out[8192];

    /* les commandes des étapes suivantes ne font pas avancer l'étape en cours */
    run(gs, "help", out, sizeof out);
    run(gs, "scan", out, sizeof out);
    run(gs, "status", out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_QUESTS);
    CHECK(!has(out, "MISSION")); /* pas de fausse fin */

    run(gs, "quests", out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_HELP); /* une seule étape à la fois */
    free(gs);
}

static void test_state_steps_chain(void)
{
    /* si le joueur a déjà tout fait, les étapes d'état se valident d'un coup, une seule récompense */
    GameState *gs = new_tutorial();
    char out[16384];
    gs->tutorial.step = NH_TUT_STATUS;
    gs->player.level = LEVEL_APPRENTICE;
    compromise_localhost(gs);
    gs->alert.reductions_done = 1;
    int credits0 = gs->player.credits;

    run(gs, "status", out, sizeof out);
    CHECK(gs->tutorial.done);
    CHECK(!nh_tutorial_active(gs));
    CHECK_INT(gs->player.credits, credits0 + NH_TUT_REWARD_CREDITS);
    /* aucune consigne n'est donnée pour une étape déjà accomplie */
    CHECK(!has(out, "Concentre-toi"));
    free(gs);
}

static void test_reminders(void)
{
    GameState *gs = new_tutorial();
    char out[8192];
    nh_set_lang(NH_LANG_FR);

    /* commande inconnue */
    CHECK(run(gs, "xyzzy", out, sizeof out) == NH_DISPATCH_UNKNOWN);
    CHECK(has(out, "ECHO-7"));
    CHECK(has(out, "Ouvrir le journal de mission"));

    /* commande verrouillée par le niveau */
    CHECK(run(gs, "bruteforce localhost", out, sizeof out) == NH_DISPATCH_LOCKED);
    CHECK(has(out, "ECHO-7"));

    /* commande qui échoue (argument manquant) */
    CHECK(run(gs, "read", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(has(out, "ECHO-7"));

    /* une commande valide qui ne fait pas avancer ne déclenche pas de rappel */
    CHECK(run(gs, "status", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(!has(out, "ECHO-7"));

    /* ligne vide : silence */
    CHECK(run(gs, "", out, sizeof out) == NH_DISPATCH_EMPTY);
    CHECK(!has(out, "ECHO-7"));
    CHECK_INT(gs->tutorial.step, NH_TUT_QUESTS);

    /* le rappel suit l'étape en cours */
    gs->tutorial.step = NH_TUT_SCAN;
    run(gs, "xyzzy", out, sizeof out);
    CHECK(has(out, "Scanner le réseau"));
    free(gs);
}

static void test_no_reminder_when_step_progresses(void)
{
    char out[8192];

    /* un bruteforce raté est un FAILED : rappel ; réussi : avance sans rappel superflu */
    bool won = false;
    for (unsigned seed = 1; seed < 200 && !won; seed++)
    {
        GameState *t = new_tutorial();
        t->tutorial.step = NH_TUT_BRUTEFORCE;
        t->player.level = LEVEL_APPRENTICE;
        t->player.commands_unlocked[CMD_BRUTEFORCE] = true;
        srand(seed);
        NhDispatch r = run(t, "bruteforce localhost", out, sizeof out);
        if (r == NH_DISPATCH_OK)
        {
            won = true;
            CHECK_INT(t->tutorial.step, NH_TUT_LAYLOW);
            CHECK(!has(out, "Concentre-toi"));
        }
        else
        {
            CHECK_INT(t->tutorial.step, NH_TUT_BRUTEFORCE);
            CHECK(has(out, "Concentre-toi"));
        }
        free(t);
    }
    CHECK(won);
}

static void test_skip(void)
{
    GameState *gs = new_tutorial();
    int credits0 = gs->player.credits;
    int completed0 = gs->quests.completed_quest_count;
    int active0 = gs->quests.active_quest_count;
    CHECK_INT(gs->quests.quests[QUEST_INTRO_TUTORIAL].status, QUEST_STATUS_ACTIVE);

    nh_tutorial_skip(gs);
    CHECK(gs->tutorial.done);
    CHECK(!nh_tutorial_active(gs));
    CHECK_INT(gs->player.credits, credits0); /* passer le tutoriel ne rapporte rien */
    CHECK_INT(gs->quests.quests[QUEST_INTRO_TUTORIAL].status, QUEST_STATUS_COMPLETED);
    CHECK_INT(gs->quests.completed_quest_count, completed0 + 1);
    CHECK_INT(gs->quests.active_quest_count, active0 - 1);

    /* rejouer skip ne double pas la comptabilité */
    nh_tutorial_skip(gs);
    CHECK_INT(gs->quests.completed_quest_count, completed0 + 1);
    CHECK_INT(gs->quests.active_quest_count, active0 - 1);
    free(gs);
}

static void test_legacy_quest_closed_on_finish(void)
{
    GameState *gs = new_tutorial();
    char out[16384];
    int completed0 = gs->quests.completed_quest_count;
    gs->tutorial.step = NH_TUT_LAYLOW;
    gs->alert.reductions_done = 1;

    run(gs, "status", out, sizeof out);
    CHECK(gs->tutorial.done);
    Quest *q = &gs->quests.quests[QUEST_INTRO_TUTORIAL];
    CHECK_INT(q->status, QUEST_STATUS_COMPLETED);
    for (int i = 0; i < q->objective_count; i++)
        CHECK(q->objectives[i].is_completed);
    CHECK_INT(gs->quests.completed_quest_count, completed0 + 1);
    free(gs);
}

static void test_ignored_when_game_stopped(void)
{
    GameState *gs = new_tutorial();
    char out[8192];
    gs->running = false;
    NhCapture cap = nh_capture_begin();
    nh_tutorial_on_command(gs, "quests", NH_DISPATCH_OK);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_QUESTS); /* `quit` ne doit pas faire progresser ni parler */
    CHECK_STR(out, "");

    gs->running = true;
    gs->player.game_over = true;
    cap = nh_capture_begin();
    nh_tutorial_on_command(gs, "quests", NH_DISPATCH_OK);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->tutorial.step, NH_TUT_QUESTS);
    CHECK_STR(out, "");
    free(gs);
}

static void test_announce(void)
{
    GameState *gs = new_tutorial();
    char out[8192];
    nh_set_lang(NH_LANG_FR);

    NhCapture cap = nh_capture_begin();
    nh_tutorial_announce(gs, false);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "ECHO-7"));
    CHECK(has(out, "quests"));
    CHECK(!has(out, "Testeur"));

    /* reprise d'une partie : ECHO-7 salue le joueur, puis rappelle l'étape */
    gs->tutorial.step = NH_TUT_SCAN;
    cap = nh_capture_begin();
    nh_tutorial_announce(gs, true);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "Testeur"));
    CHECK(has(out, "scan"));

    gs->tutorial.step = NH_TUT_NONE;
    cap = nh_capture_begin();
    nh_tutorial_announce(gs, true);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_STR(out, "");
    free(gs);
}

static void test_english(void)
{
    GameState *gs = new_tutorial();
    char out[8192];
    nh_set_lang(NH_LANG_EN);

    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "Open the mission log"));
    CHECK(!has(out, "Ouvrir"));
    run(gs, "xyzzy", out, sizeof out);
    CHECK(has(out, "Focus, rookie"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_all_texts_exist_in_both_languages(void)
{
    /* chaque étape a un intitulé et une consigne, dans les deux langues */
    static const NhStr texts[] = {
        NH_STR_TUT_OBJ_QUESTS, NH_STR_TUT_OBJ_HELP,       NH_STR_TUT_OBJ_SCAN,   NH_STR_TUT_OBJ_STATUS,
        NH_STR_TUT_OBJ_LEVELUP, NH_STR_TUT_OBJ_BRUTEFORCE, NH_STR_TUT_OBJ_LAYLOW, NH_STR_TUT_SAY_QUESTS,
        NH_STR_TUT_SAY_HELP,   NH_STR_TUT_SAY_SCAN,       NH_STR_TUT_SAY_STATUS, NH_STR_TUT_SAY_LEVELUP,
        NH_STR_TUT_SAY_BRUTEFORCE, NH_STR_TUT_SAY_LAYLOW,
    };
    for (size_t i = 0; i < sizeof texts / sizeof texts[0]; i++)
    {
        CHECK(nh_tr_lang(texts[i], NH_LANG_FR)[0] != '\0');
        CHECK(nh_tr_lang(texts[i], NH_LANG_EN)[0] != '\0');
    }
}

int main(void)
{
    nh_set_fast(true); /* aucune pause : les commandes du jeu attendent entre deux essais */
    nh_term_set_color(false);

    test_inactive_by_default();
    test_start_and_mission_log();
    test_full_walkthrough();
    test_reward_exact();
    test_out_of_order();
    test_state_steps_chain();
    test_reminders();
    test_no_reminder_when_step_progresses();
    test_skip();
    test_legacy_quest_closed_on_finish();
    test_ignored_when_game_stopped();
    test_announce();
    test_english();
    test_all_texts_exist_in_both_languages();

    nh_unfeed();
    return NH_TEST_REPORT("tutorial");
}
