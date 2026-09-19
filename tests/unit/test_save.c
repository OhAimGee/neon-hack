#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"
#include "nh_tmp.h"

#include "../../src/core/kv.h"
#include "../../src/core/parse.h"
#include "../../src/core/platform.h"
#include "../../src/game/progression.h"
#include "../../src/game/quest_system.h"
#include "../../src/game/save.h"
#include "../../src/game/tutorial.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>

static GameState *new_state(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    return gs;
}

/* Remplace la valeur d'une clé dans le texte d'une sauvegarde (la clé doit exister). */
static char *with_value(const char *text, const char *key, const char *value)
{
    char needle[96];
    snprintf(needle, sizeof needle, "\n%s=", key);
    const char *at = strstr(text, needle);
    if (at == NULL)
    {
        fprintf(stderr, "  clé absente du test: %s\n", key);
        exit(2);
    }
    at += strlen(needle);
    const char *eol = strchr(at, '\n');

    char *out = malloc(strlen(text) + strlen(value) + 8);
    size_t head = (size_t)(at - text);
    memcpy(out, text, head);
    strcpy(out + head, value);
    strcat(out, eol);
    return out;
}

/* Partie bien avancée : de quoi toucher chaque famille de champs enregistrés. */
static void make_rich(GameState *gs)
{
    Player *p = &gs->player;
    strcpy(p->name, "Molly");
    p->level = LEVEL_APPRENTICE;
    p->experience = 137;
    p->scans_done = 9;
    p->milestones = 5;
    p->credits = 4321;
    p->reputation = -12;
    p->stealth_rating = 44;
    p->commands_unlocked[3] = true;
    p->has_quantum_computer = true;
    p->has_ai_assistant = true;
    p->virus_library_size = 2;
    p->backdoors_active = 1;

    gs->stealth_mode = true;
    gs->viruses[0].is_detected = true;

    for (int i = 0; i < NH_MAX_NODES; i++)
        gs->nodes[i].firewall_strength = 10 + i;
    gs->nodes[0].is_discovered = true;
    gs->nodes[0].is_compromised = true;
    gs->nodes[1].is_discovered = true;
    gs->nodes[1].has_backdoor = true;
    gs->nodes[1].is_traced = true;
    gs->nodes[2].has_virus = true;
    gs->nodes[2].has_intel = true;
    gs->nodes[0].secret_files[0].is_unlocked = true;

    gs->shop.items[1].is_available = false;
    gs->shop.bought = (1u << ITEM_ALERT_REDUCER) | (1u << ITEM_VIRUS_PACK);

    gs->alert.level = 33;
    gs->alert.max_level = 51;
    gs->alert.vpn_active = true;
    gs->alert.proxy_active = true;
    gs->alert.ghost_protocols_available = 3;
    gs->alert.reductions_done = 4;

    gs->advanced.neural_interface_sync = 17;

    gs->contacts.inbox[0].is_read = true;
    gs->contacts.contacts[1].is_unlocked = true;
    gs->contacts.contacts[1].is_discovered = true;
    gs->contacts.contacts[1].interactions_count = 6;
    gs->contacts.active_contacts = 2;

    /* Tutoriel terminé, deuxième quête active avec un objectif accompli sur deux. */
    gs->quests.quests[QUEST_INTRO_TUTORIAL].status = QUEST_STATUS_COMPLETED;
    gs->quests.quests[QUEST_INTRO_TUTORIAL].progress[0] = 1;
    gs->quests.quests[QUEST_FIRST_INFILTRATION].status = QUEST_STATUS_ACTIVE;
    gs->quests.quests[QUEST_FIRST_INFILTRATION].progress[0] = 1;

    gs->tutorial.step = NH_TUT_HELP;
    gs->tutorial.done = false;
}

static void test_roundtrip(void)
{
    GameState *a = new_state();
    make_rich(a);
    char *text = nh_save_to_text(a);
    CHECK(text != NULL);

    GameState *b = new_state();
    CHECK_INT(nh_save_from_text(b, text, strlen(text)), NH_SAVE_OK);

    /* relecture fidèle : réécrire l'état chargé redonne exactement le même fichier */
    char *again = nh_save_to_text(b);
    CHECK_STR(again, text);

    CHECK_STR(b->player.name, "Molly");
    CHECK_INT(b->player.level, LEVEL_APPRENTICE);
    CHECK_INT(b->player.experience, 137);
    CHECK_INT(b->player.credits, 4321);
    CHECK_INT(b->player.reputation, -12);
    CHECK(b->player.commands_unlocked[3]);
    CHECK(b->player.has_quantum_computer && b->player.has_ai_assistant);
    CHECK(b->stealth_mode);
    CHECK(b->viruses[0].is_detected && !b->viruses[1].is_detected);
    CHECK(b->nodes[0].is_discovered && b->nodes[0].is_compromised);
    CHECK(b->nodes[1].has_backdoor && b->nodes[1].is_traced && !b->nodes[1].is_compromised);
    CHECK(b->nodes[2].has_virus && b->nodes[2].has_intel);
    CHECK(b->nodes[0].secret_files[0].is_unlocked);
    CHECK_INT(b->nodes[3].firewall_strength, 13);
    CHECK(!b->shop.items[1].is_available && b->shop.items[0].is_available);
    CHECK_INT(b->shop.bought, (1u << ITEM_ALERT_REDUCER) | (1u << ITEM_VIRUS_PACK));
    CHECK_INT(b->alert.level, 33);
    CHECK_INT(b->alert.max_level, 51);
    CHECK(b->alert.vpn_active && b->alert.proxy_active);
    CHECK_INT(b->alert.ghost_protocols_available, 3);
    CHECK_INT(b->alert.reductions_done, 4);
    CHECK_INT(b->advanced.neural_interface_sync, 17);
    CHECK(b->contacts.inbox[0].is_read);
    CHECK(b->contacts.contacts[1].is_unlocked && b->contacts.contacts[1].is_discovered);
    CHECK_INT(b->contacts.contacts[1].interactions_count, 6);
    CHECK_INT(b->quests.quests[QUEST_INTRO_TUTORIAL].status, QUEST_STATUS_COMPLETED);
    CHECK(nh_quest_objective_done(&b->quests, QUEST_INTRO_TUTORIAL, 0));
    CHECK_INT(b->quests.quests[QUEST_FIRST_INFILTRATION].status, QUEST_STATUS_ACTIVE);
    CHECK(nh_quest_objective_done(&b->quests, QUEST_FIRST_INFILTRATION, 0));
    CHECK(!nh_quest_objective_done(&b->quests, QUEST_FIRST_INFILTRATION, 1));
    CHECK_INT(nh_quests_count(&b->quests, QUEST_STATUS_COMPLETED), 1);
    CHECK_INT(nh_quests_count(&b->quests, QUEST_STATUS_ACTIVE), 1);
    CHECK_INT(b->tutorial.step, NH_TUT_HELP);
    CHECK(!b->tutorial.done);

    free(again);
    free(text);
    free(a);
    free(b);
}

static void test_fresh_state_roundtrip(void)
{
    GameState *a = new_state();
    char *text = nh_save_to_text(a);
    GameState *b = new_state();
    CHECK_INT(nh_save_from_text(b, text, strlen(text)), NH_SAVE_OK);
    char *again = nh_save_to_text(b);
    CHECK_STR(again, text);
    CHECK_STR(b->player.name, NH_DEFAULT_NAME);
    CHECK(!nh_tutorial_active(b)); /* un état neuf n'a pas de tutoriel : c'est l'introduction qui le lance */
    free(again);
    free(text);
    free(a);
    free(b);
}

static void test_frame(void)
{
    GameState *a = new_state();
    char *text = nh_save_to_text(a);

    CHECK(strncmp(text, "neon-hack-save=1\n", 17) == 0); /* la version vient en premier */
    size_t n = strlen(text);
    CHECK(n > 10 && strcmp(text + n - 6, "end=1\n") == 0); /* et la marque de fin en dernier */

    free(text);
    free(a);
}

static void test_transactional_load(void)
{
    GameState *a = new_state();
    make_rich(a);
    char *good = nh_save_to_text(a);
    char *before = nh_save_to_text(a);

    /* chaque valeur hors bornes fait échouer le chargement, sans rien changer à la partie en cours */
    static const struct
    {
        const char *key;
        const char *value;
    } bad[] = {
        {"player.level", "99"},          {"player.level", "0"},        {"player.experience", "-1"},
        {"player.credits", "2000000000"}, {"player.stealth_rating", "101"}, {"player.credits", "beaucoup"},
        {"node.0.flags", "64"},          {"node.0.firewall", "-5"},   {"alert.level", "101"},
        {"alert.vpn", "2"},              {"quest.0.status", "9"},     {"tutorial.step", "99"},
        {"tutorial.done", "7"},          {"contacts.active", "50"},   {"shop.sold_out", "999999"},
        {"player.unlocked", "-1"},       {"player.credits", ""},
        {"quest.1.status", "5"},         {"quest.1.obj.0", "-1"},     {"quest.1.obj.0", "beaucoup"},
        {"shop.bought", "999999"},       {"shop.bought", "-1"},
    };
    for (size_t i = 0; i < sizeof bad / sizeof bad[0]; i++)
    {
        char *broken = with_value(good, bad[i].key, bad[i].value);
        GameState *b = new_state();
        make_rich(b);
        b->player.credits = 777; /* marqueur : doit survivre à l'échec */
        char *snapshot = nh_save_to_text(b);

        CHECK_INT(nh_save_from_text(b, broken, strlen(broken)), NH_SAVE_CORRUPT);
        char *after = nh_save_to_text(b);
        CHECK_STR(after, snapshot);

        free(after);
        free(snapshot);
        free(broken);
        free(b);
    }

    char *after = nh_save_to_text(a);
    CHECK_STR(after, before);
    free(after);
    free(before);
    free(good);
    free(a);
}

static void test_bad_frames(void)
{
    GameState *a = new_state();
    char *text = nh_save_to_text(a);
    GameState *b = new_state();

    CHECK_INT(nh_save_from_text(b, "", 0), NH_SAVE_CORRUPT);
    CHECK_INT(nh_save_from_text(b, "n'importe quoi\n", 15), NH_SAVE_CORRUPT);
    CHECK_INT(nh_save_from_text(b, "player.level=3\nend=1\n", 21), NH_SAVE_CORRUPT); /* pas d'en-tête */
    CHECK_INT(nh_save_from_text(b, "neon-hack-save=abc\nend=1\n", 25), NH_SAVE_CORRUPT);
    CHECK_INT(nh_save_from_text(b, "neon-hack-save=0\nend=1\n", 23), NH_SAVE_CORRUPT);
    CHECK_INT(nh_save_from_text(b, "neon-hack-save=1\nplayer.level=3\n", 32), NH_SAVE_CORRUPT); /* pas de fin */

    /* tronquée à n'importe quel endroit : jamais acceptée */
    size_t n = strlen(text);
    int accepted = 0;
    for (size_t cut = 0; cut + 6 < n; cut += 13)
        if (nh_save_from_text(b, text, cut) == NH_SAVE_OK)
            accepted++;
    CHECK_INT(accepted, 0);

    /* écrite par une version future : refusée sans être déclarée corrompue */
    char forged[] = "neon-hack-save=2\nplayer.level=3\nend=1\n";
    CHECK_INT(nh_save_from_text(b, forged, strlen(forged)), NH_SAVE_TOO_NEW);
    char forged_huge[] = "neon-hack-save=999999\nend=1\n";
    CHECK_INT(nh_save_from_text(b, forged_huge, strlen(forged_huge)), NH_SAVE_TOO_NEW);

    free(text);
    free(a);
    free(b);
}

static void test_tolerance(void)
{
    /* clés inconnues ignorées, clés absentes = valeur d'un état neuf, CRLF et commentaires acceptés */
    const char *text =
        "# fichier édité à la main\r\n"
        "neon-hack-save=1\r\n"
        "player.level=2\r\n"
        "futur.champ=42\r\n"
        "player.credits=500\r\n"
        "end=1\r\n";
    GameState *b = new_state();
    CHECK_INT(nh_save_from_text(b, text, strlen(text)), NH_SAVE_OK);
    CHECK_INT(b->player.level, 2);
    CHECK_INT(b->player.credits, 500);
    CHECK_STR(b->player.name, NH_DEFAULT_NAME);
    CHECK(!nh_tutorial_active(b));

    GameState *fresh = new_state();
    CHECK_INT(b->player.experience, fresh->player.experience);
    CHECK_INT(b->alert.level, fresh->alert.level);
    free(fresh);
    free(b);
}

static void test_name_sanitized(void)
{
    GameState *a = new_state();
    char *text = nh_save_to_text(a);
    GameState *b = new_state();

    /* un nom trafiqué (séquence d'échappement, trop long) ne doit rien injecter dans le terminal */
    char *hacked = with_value(text, "player.name", "\x1b[31mRouge\x1b[0m   et   plus");
    CHECK_INT(nh_save_from_text(b, hacked, strlen(hacked)), NH_SAVE_OK);
    CHECK(strchr(b->player.name, '\x1b') == NULL);
    CHECK_STR(b->player.name, "[31mRouge[0m et plus");
    free(hacked);

    char *longname = with_value(text, "player.name", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    CHECK_INT(nh_save_from_text(b, longname, strlen(longname)), NH_SAVE_OK);
    CHECK_INT(strlen(b->player.name), NH_NAME_MAX_CHARS);
    free(longname);

    /* nom vide ou ne contenant que des caractères de contrôle : on garde « Case » */
    char *empty = with_value(text, "player.name", "\x01\x02");
    CHECK_INT(nh_save_from_text(b, empty, strlen(empty)), NH_SAVE_OK);
    CHECK_STR(b->player.name, NH_DEFAULT_NAME);
    free(empty);

    free(text);
    free(a);
    free(b);
}

static void test_tutorial_state_persisted(void)
{
    GameState *a = new_state();
    a->tutorial.step = NH_TUT_LAYLOW;
    char *text = nh_save_to_text(a);
    GameState *b = new_state();
    CHECK_INT(nh_save_from_text(b, text, strlen(text)), NH_SAVE_OK);
    CHECK_INT(b->tutorial.step, NH_TUT_LAYLOW);
    CHECK(nh_tutorial_active(b));
    free(text);

    /* terminé : plus aucune étape, même si le fichier en garde une */
    a->tutorial.step = NH_TUT_NONE;
    a->tutorial.done = true;
    text = nh_save_to_text(a);
    CHECK_INT(nh_save_from_text(b, text, strlen(text)), NH_SAVE_OK);
    CHECK(b->tutorial.done && !nh_tutorial_active(b));
    free(text);

    const char *odd = "neon-hack-save=1\ntutorial.step=3\ntutorial.done=1\nend=1\n";
    CHECK_INT(nh_save_from_text(b, odd, strlen(odd)), NH_SAVE_OK);
    CHECK_INT(b->tutorial.step, NH_TUT_NONE);

    free(a);
    free(b);
}

static void test_quest_state_persisted(void)
{
    GameState *a = new_state();
    a->quests.quests[QUEST_FIRST_INFILTRATION].status = QUEST_STATUS_ACTIVE;
    a->quests.quests[QUEST_FIRST_INFILTRATION].progress[1] = 1;
    char *text = nh_save_to_text(a);

    /* Un statut par quête, un avancement par objectif ; rien pour une quête pas encore écrite. */
    CHECK(strstr(text, "\nquest.0.status=2\n") != NULL); /* le tutoriel démarre actif */
    CHECK(strstr(text, "\nquest.1.status=2\n") != NULL);
    CHECK(strstr(text, "\nquest.1.obj.1=1\n") != NULL);
    CHECK(strstr(text, "\nquest.1.obj.2=") == NULL);
    CHECK(strstr(text, "\nquest.4.status=0\n") != NULL);
    CHECK(strstr(text, "\nquest.4.obj.0=") == NULL);
    /* Les compteurs d'origine se déduisent des statuts : ils ne sont plus écrits. */
    CHECK(strstr(text, "quests.active") == NULL);
    CHECK(strstr(text, "quests.completed") == NULL);
    CHECK(strstr(text, "quest.0.done") == NULL);

    GameState *b = new_state();
    CHECK_INT(nh_save_from_text(b, text, strlen(text)), NH_SAVE_OK);
    CHECK_INT(b->quests.quests[QUEST_FIRST_INFILTRATION].status, QUEST_STATUS_ACTIVE);
    CHECK(!nh_quest_objective_done(&b->quests, QUEST_FIRST_INFILTRATION, 0));
    CHECK(nh_quest_objective_done(&b->quests, QUEST_FIRST_INFILTRATION, 1));
    CHECK_INT(b->quests.quests[QUEST_UNDERGROUND_CONTACT].status, QUEST_STATUS_LOCKED);

    free(text);
    free(a);
    free(b);
}

static void test_quest_old_keys_and_clamping(void)
{
    GameState *fresh = new_state();
    GameState *b = new_state();

    /* Les clés des versions précédentes sont ignorées : la partie repart d'un état de quêtes neuf. */
    const char *old =
        "neon-hack-save=1\nquests.active=3\nquests.completed=2\nquests.progress=50\n"
        "quest.0.done=1\nquest.1.done=1\nend=1\n";
    CHECK_INT(nh_save_from_text(b, old, strlen(old)), NH_SAVE_OK);
    CHECK(memcmp(&b->quests, &fresh->quests, sizeof b->quests) == 0);

    /* Un avancement au-delà de l'objectif est ramené à l'objectif. */
    const char *over = "neon-hack-save=1\nquest.1.status=2\nquest.1.obj.0=999\nend=1\n";
    CHECK_INT(nh_save_from_text(b, over, strlen(over)), NH_SAVE_OK);
    CHECK_INT(b->quests.quests[QUEST_FIRST_INFILTRATION].progress[0], 1);

    /* Une quête terminée est complète, même si la sauvegarde n'a rien noté de son avancement. */
    const char *done = "neon-hack-save=1\nquest.2.status=3\nend=1\n";
    CHECK_INT(nh_save_from_text(b, done, strlen(done)), NH_SAVE_OK);
    const NhQuestDef *def = nh_quest_def(QUEST_GATHER_INTEL);
    for (int o = 0; o < def->objective_count; o++)
        CHECK(nh_quest_objective_done(&b->quests, QUEST_GATHER_INTEL, o));

    /* Une sauvegarde d'avant le masque d'achats : un objet épuisé a forcément été acheté. */
    const char *sold = "neon-hack-save=1\nshop.sold_out=4\nend=1\n";
    CHECK_INT(nh_save_from_text(b, sold, strlen(sold)), NH_SAVE_OK);
    CHECK_INT(b->shop.bought, 4);
    CHECK(!b->shop.items[2].is_available);

    free(fresh);
    free(b);
}

static void test_save_path_kept_on_load(void)
{
    GameState *a = new_state();
    char *text = nh_save_to_text(a);
    GameState *b = new_state();
    strcpy(b->save_path, "/quelque/part.sav");
    CHECK_INT(nh_save_from_text(b, text, strlen(text)), NH_SAVE_OK);
    CHECK_STR(b->save_path, "/quelque/part.sav"); /* le chemin est du ressort de l'appelant, pas du fichier */
    free(text);
    free(a);
    free(b);
}

static void test_peek(void)
{
    GameState *a = new_state();
    make_rich(a);
    char *text = nh_save_to_text(a);

    NhSaveInfo info;
    CHECK_INT(nh_save_peek_text(text, strlen(text), &info), NH_SAVE_OK);
    CHECK_STR(info.name, "Molly");
    CHECK_INT(info.level, LEVEL_APPRENTICE);
    CHECK_INT(info.credits, 4321);
    CHECK(info.tutorial_active);

    CHECK_INT(nh_save_peek_text("rien", 4, &info), NH_SAVE_CORRUPT);
    CHECK_INT(nh_save_peek_text("neon-hack-save=2\nend=1\n", 23, &info), NH_SAVE_TOO_NEW);
    const char *bad = "neon-hack-save=1\nplayer.level=abc\nend=1\n";
    CHECK_INT(nh_save_peek_text(bad, strlen(bad), &info), NH_SAVE_CORRUPT);

    free(text);
    free(a);
}

static void test_files(void)
{
    char root[256], path[512];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/neon-hack/savegame.sav", root);

    GameState *a = new_state();
    CHECK_INT(nh_save_game(a), NH_SAVE_NO_PATH); /* aucun chemin : rien n'est écrit */

    make_rich(a);
    snprintf(a->save_path, sizeof a->save_path, "%s", path);

    NhSaveInfo info;
    GameState *b = new_state();
    CHECK_INT(nh_load_game(b, path), NH_SAVE_MISSING);
    CHECK_INT(nh_save_peek(path, &info), NH_SAVE_MISSING);

    CHECK_INT(nh_save_game(a), NH_SAVE_OK); /* crée aussi le dossier */
    CHECK_INT(nh_save_peek(path, &info), NH_SAVE_OK);
    CHECK_STR(info.name, "Molly");
    CHECK_INT(nh_load_game(b, path), NH_SAVE_OK);
    CHECK_STR(b->player.name, "Molly");
    CHECK_INT(b->player.credits, 4321);
    CHECK_STR(b->save_path, ""); /* `b` n'avait pas de chemin : le chargement n'en invente pas */

    char tmp[600];
    snprintf(tmp, sizeof tmp, "%s.tmp", path);
    CHECK(!nh_storage_exists(tmp)); /* écriture atomique : pas de fichier temporaire */

    /* fichier tronqué à la main */
    char *data = nh_tmp_slurp(path);
    data[strlen(data) / 2] = '\0';
    nh_tmp_write(path, data);
    free(data);
    CHECK_INT(nh_load_game(b, path), NH_SAVE_CORRUPT);
    CHECK_INT(nh_save_peek(path, &info), NH_SAVE_CORRUPT);
    CHECK_INT(b->player.credits, 4321); /* le chargement raté n'a rien touché */

    /* fichier gigantesque : pas une sauvegarde */
    char *big = malloc(NH_SAVE_MAX_BYTES + 100);
    memset(big, 'a', NH_SAVE_MAX_BYTES + 99);
    big[NH_SAVE_MAX_BYTES + 99] = '\0';
    nh_tmp_write(path, big);
    free(big);
    CHECK_INT(nh_load_game(b, path), NH_SAVE_CORRUPT);
    CHECK_INT(nh_save_peek(path, &info), NH_SAVE_CORRUPT);

    /* chemin impossible à écrire : l'erreur est signalée, pas avalée */
    snprintf(a->save_path, sizeof a->save_path, "%s/fichier/sous.sav", root);
    char blocker[600];
    snprintf(blocker, sizeof blocker, "%s/fichier", root);
    nh_tmp_write(blocker, "je suis un fichier, pas un dossier");
    CHECK_INT(nh_save_game(a), NH_SAVE_IO);

    free(a);
    free(b);
    nh_tmp_remove(root);
}

/* ---- Sauvegarde automatique dans la boucle de jeu ------------------------------------------- */

static size_t count_of(const char *text, const char *needle)
{
    size_t n = 0;
    for (const char *p = text; (p = strstr(p, needle)) != NULL; p += strlen(needle))
        n++;
    return n;
}

static void run_loop(GameState *gs, const char *input, char *out, size_t size)
{
    nh_feed(input);
    NhCapture cap = nh_capture_begin();
    game_loop(gs);
    nh_capture_end(&cap, out, size);
}

static void test_autosave_after_each_command(void)
{
    char root[256], path[512], out[65536];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/auto.sav", root);

    GameState *gs = new_state();
    strcpy(gs->player.name, "Molly");
    snprintf(gs->save_path, sizeof gs->save_path, "%s", path);

    CHECK(!nh_storage_exists(path)); /* rien avant la première commande */
    run_loop(gs, "scan\nstatus\n", out, sizeof out);
    CHECK(nh_storage_exists(path));
    CHECK(strstr(out, "Impossible de sauvegarder") == NULL); /* une sauvegarde réussie se fait en silence */

    GameState *b = new_state();
    CHECK_INT(nh_load_game(b, path), NH_SAVE_OK);
    CHECK_INT(b->player.scans_done, 1); /* l'effet du scan est bien dans le fichier */
    CHECK_STR(b->player.name, "Molly");
    CHECK(b->nodes[0].is_discovered);

    /* quit : sauvegarde annoncée, et la boucle s'arrête sans exécuter la suite */
    run_loop(gs, "scan\nquit\nstatus\n", out, sizeof out);
    CHECK_INT(count_of(out, "Partie sauvegardée."), 1);
    CHECK_INT(count_of(out, "STATUT DU HACKER"), 0);
    CHECK_INT(nh_load_game(b, path), NH_SAVE_OK);
    CHECK_INT(b->player.scans_done, 2);

    free(gs);
    free(b);
    nh_tmp_remove(root);
}

static void test_game_over_is_not_saved(void)
{
    char root[256], path[512], out[65536];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/auto.sav", root);

    GameState *gs = new_state();
    snprintf(gs->save_path, sizeof gs->save_path, "%s", path);
    gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;
    gs->alert.level = 99;
    CHECK_INT(nh_save_game(gs), NH_SAVE_OK); /* la sauvegarde d'avant la commande fatale */
    char *before = nh_tmp_slurp(path);

    run_loop(gs, "bruteforce localhost\nstatus\n", out, sizeof out);
    CHECK(gs->player.game_over);
    char *after = nh_tmp_slurp(path);
    CHECK_STR(after, before); /* la partie perdue n'a pas écrasé la dernière partie vivante */

    GameState *b = new_state();
    CHECK_INT(nh_load_game(b, path), NH_SAVE_OK);
    CHECK_INT(b->alert.level, 99);
    CHECK(!b->player.game_over);

    free(before);
    free(after);
    free(gs);
    free(b);
    nh_tmp_remove(root);
}

static void test_autosave_failure_warns_once(void)
{
    char root[256], blocker[512], out[65536];
    nh_tmp_make(root, sizeof root);
    snprintf(blocker, sizeof blocker, "%s/bloc", root);
    nh_tmp_write(blocker, "un fichier, pas un dossier");

    GameState *gs = new_state();
    snprintf(gs->save_path, sizeof gs->save_path, "%s/bloc/auto.sav", root);
    run_loop(gs, "status\nstatus\nstatus\n", out, sizeof out);
    CHECK_INT(count_of(out, "Impossible de sauvegarder"), 1); /* une seule alerte, pas une par commande */
    CHECK_INT(count_of(out, "STATUT DU HACKER"), 3);          /* et la partie continue normalement */

    free(gs);
    nh_tmp_remove(root);
}

static void test_no_save_path_no_file(void)
{
    char out[65536];
    GameState *gs = new_state();
    CHECK_STR(gs->save_path, "");
    run_loop(gs, "status\n", out, sizeof out);
    CHECK(strstr(out, "Impossible de sauvegarder") == NULL); /* sans chemin : aucun bruit, aucune écriture */
    free(gs);
}

int main(void)
{
    nh_set_fast(true);
    nh_term_set_color(false);
    nh_set_lang(NH_LANG_FR);

    test_roundtrip();
    test_fresh_state_roundtrip();
    test_frame();
    test_transactional_load();
    test_bad_frames();
    test_tolerance();
    test_name_sanitized();
    test_tutorial_state_persisted();
    test_quest_state_persisted();
    test_quest_old_keys_and_clamping();
    test_save_path_kept_on_load();
    test_peek();
    test_files();
    test_autosave_after_each_command();
    test_game_over_is_not_saved();
    test_autosave_failure_warns_once();
    test_no_save_path_no_file();
    nh_unfeed();
    return NH_TEST_REPORT("save");
}
