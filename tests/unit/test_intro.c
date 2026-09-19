#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"

#include "../../src/core/parse.h"
#include "../../src/core/platform.h"
#include "../../src/game/intro.h"
#include "../../src/game/tutorial.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static size_t count(const char *text, const char *needle)
{
    size_t n = 0;
    for (const char *p = text; (p = strstr(p, needle)) != NULL; p += strlen(needle))
        n++;
    return n;
}

/* Joue le prologue avec `input` comme frappes du joueur. */
static NhIntroResult play(GameState *gs, const char *input, char *out, size_t size)
{
    init_game(gs);
    nh_feed(input);
    NhCapture cap = nh_capture_begin();
    NhIntroResult r = nh_intro_run(gs);
    nh_capture_end(&cap, out, size);
    return r;
}

static GameState *alloc_state(void) { return calloc(1, sizeof(GameState)); }

static void test_default_name(void)
{
    GameState *gs = alloc_state();
    char out[16384];
    nh_set_lang(NH_LANG_FR);

    /* Entrée pour le nom, Entrée pour confirmer, Entrée pour le tutoriel */
    CHECK_INT(play(gs, "\n\n\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Case");
    CHECK_STR(gs->player.name, NH_DEFAULT_NAME);
    CHECK(nh_tutorial_active(gs));
    CHECK_INT(gs->tutorial.step, NH_TUT_QUESTS);
    CHECK(has(out, "ECHO-7"));
    CHECK(has(out, "Case"));
    CHECK(has(out, "[O/n]")); /* la confirmation est posée en français */
    free(gs);
}

static void test_chosen_name_and_skip(void)
{
    GameState *gs = alloc_state();
    char out[16384];
    nh_set_lang(NH_LANG_FR);

    CHECK_INT(play(gs, "Molly\n\n2\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Molly");
    CHECK(has(out, "Molly"));
    CHECK(!nh_tutorial_active(gs));
    CHECK(gs->tutorial.done);
    CHECK_INT(gs->quests.quests[QUEST_INTRO_TUTORIAL].status, QUEST_STATUS_COMPLETED);
    CHECK(has(out, "help")); /* pas de tutoriel : on indique au moins l'aide */
    free(gs);
}

static void test_retry_on_refusal(void)
{
    GameState *gs = alloc_state();
    char out[16384];
    nh_set_lang(NH_LANG_FR);

    /* « non » : on redemande ; le nom retenu est le dernier confirmé */
    CHECK_INT(play(gs, "Molly\nn\nJack\noui\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Jack");
    CHECK(has(out, "reprenons"));
    CHECK_INT(count(out, "[O/n]"), 2);

    /* plusieurs refus de suite */
    CHECK_INT(play(gs, "A\nnon\nB\nN\nC\n\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "C");

    /* une réponse quelconque vaut « oui » (la valeur par défaut) */
    CHECK_INT(play(gs, "Neo\nn'importe quoi\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Neo");
    free(gs);
}

static void test_name_cleaning(void)
{
    GameState *gs = alloc_state();
    char out[16384];

    /* vide, espaces, caractères de contrôle : « Case » */
    CHECK_INT(play(gs, "     \n\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Case");
    CHECK_INT(play(gs, "\x01\x02\x03\n\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Case");

    /* une séquence d'échappement ne doit jamais atteindre l'écran ni la sauvegarde */
    CHECK_INT(play(gs, "\x1b[2JPirate\n\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK(strchr(gs->player.name, '\x1b') == NULL);
    CHECK(strstr(out, "\x1b[2J") == NULL);

    /* espaces superflus */
    CHECK_INT(play(gs, "   Jean    Luc   \n\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Jean Luc");

    /* trop long : coupé */
    CHECK_INT(play(gs, "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\n\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_INT(strlen(gs->player.name), NH_NAME_MAX_CHARS);
    CHECK(strncmp(gs->player.name, "ABCDEFGHIJKLMNOPQRST", 20) == 0);

    /* UTF-8 : les caractères accentués sont conservés, jamais coupés au milieu */
    CHECK_INT(play(gs, "Zoë\n\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Zoë");
    CHECK_INT(play(gs, "ééééééééééééééééééééééééé\n\n1\n", out, sizeof out), NH_INTRO_OK);
    CHECK_INT(strlen(gs->player.name), 2 * NH_NAME_MAX_CHARS);
    free(gs);
}

static void test_tutorial_choice_validation(void)
{
    GameState *gs = alloc_state();
    char out[16384];
    nh_set_lang(NH_LANG_FR);

    CHECK_INT(play(gs, "\n\n7\nabc\n0\n2\n", out, sizeof out), NH_INTRO_OK);
    CHECK_INT(count(out, "Option invalide."), 3);
    CHECK(!nh_tutorial_active(gs));
    CHECK(gs->tutorial.done);

    CHECK_INT(play(gs, "\n\n 1 \n", out, sizeof out), NH_INTRO_OK);
    CHECK(nh_tutorial_active(gs));
    free(gs);
}

static void test_eof_at_every_prompt(void)
{
    GameState *gs = alloc_state();
    char out[16384];

    CHECK_INT(play(gs, "", out, sizeof out), NH_INTRO_EOF);          /* rien du tout */
    CHECK_INT(play(gs, "Molly", out, sizeof out), NH_INTRO_EOF);     /* nom sans fin de ligne */
    CHECK_INT(play(gs, "Molly\n", out, sizeof out), NH_INTRO_EOF);   /* fin avant la confirmation */
    CHECK_INT(play(gs, "Molly\nn\n", out, sizeof out), NH_INTRO_EOF); /* fin après un refus */
    CHECK_INT(play(gs, "Molly\n\n", out, sizeof out), NH_INTRO_EOF); /* fin avant le choix du tutoriel */
    CHECK_INT(play(gs, "Molly\n\nzzz\n", out, sizeof out), NH_INTRO_EOF); /* invalide puis fin */
    CHECK(!nh_tutorial_active(gs)); /* rien n'a été lancé */
    free(gs);
}

static void test_english(void)
{
    GameState *gs = alloc_state();
    char out[16384];
    nh_set_lang(NH_LANG_EN);

    CHECK_INT(play(gs, "\n\n\n", out, sizeof out), NH_INTRO_OK);
    CHECK_STR(gs->player.name, "Case");
    CHECK(has(out, "[Y/n]"));
    CHECK(has(out, "Your handle"));
    CHECK(!has(out, "Votre handle"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

int main(void)
{
    nh_set_fast(true); /* pas d'effet machine à écrire ni de pause */
    nh_term_set_color(false);

    test_default_name();
    test_chosen_name_and_skip();
    test_retry_on_refusal();
    test_name_cleaning();
    test_tutorial_choice_validation();
    test_eof_at_every_prompt();
    test_english();

    nh_unfeed();
    return NH_TEST_REPORT("intro");
}
