#include "nh_test.h"

#include "nh_capture.h"
#include "nh_tmp.h"

#include "../../src/core/platform.h"
#include "../../src/game/commands.h"
#include "../../src/game/progression.h"
#include "../../src/game/save.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    return gs;
}

/* Exécute une ligne et récupère ce qu'elle a affiché. */
static NhDispatch run_line(GameState *gs, const char *line, char *out, size_t out_size)
{
    NhCapture cap = nh_capture_begin();
    NhDispatch result = nh_dispatch(gs, line);
    nh_capture_end(&cap, out, out_size);
    return result;
}

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static void test_table_integrity(void)
{
    size_t count;
    const NhCommand *t = nh_commands(&count);
    CHECK(count > 20);

    for (size_t i = 0; i < count; i++)
    {
        CHECK(t[i].name != NULL && t[i].name[0] != '\0');
        CHECK(t[i].fn != NULL);
        CHECK(t[i].category >= 0 && t[i].category < NH_CAT_COUNT);
        CHECK(t[i].help >= 0 && t[i].help < NH_STR_COUNT);
        CHECK(t[i].unlock == NH_NO_UNLOCK || (t[i].unlock >= 0 && t[i].unlock < MAX_COMMANDS));
        CHECK(t[i].min_level >= 0 && t[i].min_level <= 10);
        CHECK(t[i].alias == NULL || t[i].alias[0] != '\0');

        /* Noms en minuscules ASCII (la recherche ignore la casse, l'aide les affiche tels quels). */
        for (const char *p = t[i].name; *p; p++)
            CHECK((*p >= 'a' && *p <= 'z') || (*p >= '0' && *p <= '9') || *p == '_');

        /* La description existe dans les deux langues. */
        CHECK(nh_tr_lang(t[i].help, NH_LANG_FR)[0] != '\0');
        CHECK(nh_tr_lang(t[i].help, NH_LANG_EN)[0] != '\0');

        /* Aucun nom ni alias en double dans toute la table. */
        for (size_t j = i + 1; j < count; j++)
        {
            CHECK(strcmp(t[i].name, t[j].name) != 0);
            if (t[i].alias)
            {
                CHECK(strcmp(t[i].alias, t[j].name) != 0);
                if (t[j].alias)
                    CHECK(strcmp(t[i].alias, t[j].alias) != 0);
            }
            if (t[j].alias)
                CHECK(strcmp(t[i].name, t[j].alias) != 0);
        }
    }
}

static void test_find(void)
{
    const NhCommand *scan = nh_find_command("scan");
    CHECK(scan != NULL);
    CHECK(nh_find_command("SCAN") == scan);
    CHECK(nh_find_command("Scan") == scan);

    /* Les noms qui figuraient dans l'ancienne aide fonctionnent, comme ceux du code. */
    CHECK(nh_find_command("upload_virus") == nh_find_command("uploadvirus"));
    CHECK(nh_find_command("ai_hack") == nh_find_command("aihack"));
    CHECK(nh_find_command("quantum_decrypt") == nh_find_command("quantumdecrypt"));
    CHECK(nh_find_command("exit") == nh_find_command("quit"));
    CHECK(nh_find_command("uploadvirus") != NULL);

    CHECK(nh_find_command("nope") == NULL);
    CHECK(nh_find_command("") == NULL);
    CHECK(nh_find_command("sca") == NULL);
    CHECK(nh_find_command("scans") == NULL);
    CHECK(nh_find_command("exploit") != NULL);
}

static void test_state_init(void)
{
    GameState *gs = new_game();
    CHECK_INT(gs->player.level, LEVEL_NOVICE);
    CHECK_INT(gs->player.credits, 100);
    CHECK(gs->running);
    CHECK(!gs->player.game_over);
    CHECK_STR(gs->nodes[0].name, "localhost");
    CHECK(gs->nodes[0].is_discovered); /* votre poste est connu d'emblée… */
    CHECK(!gs->nodes[1].is_discovered); /* …le reste se découvre par scan */
    CHECK(gs->player.commands_unlocked[CMD_SCAN]);
    CHECK(!gs->player.commands_unlocked[CMD_BRUTEFORCE]);

    /* Réinitialiser repart de zéro. */
    gs->player.credits = 9999;
    gs->running = false;
    init_game(gs);
    CHECK_INT(gs->player.credits, 100);
    CHECK(gs->running);
    free(gs);
}

static void test_availability(void)
{
    GameState *gs = new_game();

    CHECK(nh_command_available(gs, nh_find_command("scan")));
    CHECK(nh_command_available(gs, nh_find_command("status")));
    CHECK(nh_command_available(gs, nh_find_command("help")));
    CHECK(nh_command_available(gs, nh_find_command("shop")));
    CHECK(nh_command_available(gs, nh_find_command("stealthmode")));
    CHECK(!nh_command_available(gs, nh_find_command("bruteforce")));
    CHECK(!nh_command_available(gs, nh_find_command("decrypt")));
    CHECK(!nh_command_available(gs, nh_find_command("socialeng")));
    CHECK(!nh_command_available(gs, nh_find_command("advhack")));
    CHECK(!nh_command_available(gs, nh_find_command("temporalhack")));

    gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;
    CHECK(nh_command_available(gs, nh_find_command("bruteforce")));

    gs->player.level = LEVEL_HACKER; /* 3 */
    CHECK(nh_command_available(gs, nh_find_command("socialeng")));
    CHECK(nh_command_available(gs, nh_find_command("advhack")));
    CHECK(nh_command_available(gs, nh_find_command("aiassist")));
    CHECK(!nh_command_available(gs, nh_find_command("neuralsync")));

    gs->player.level = (HackerLevel)6;
    CHECK(nh_command_available(gs, nh_find_command("neuralsync")));
    CHECK(nh_command_available(gs, nh_find_command("temporalhack")));
    /* Un niveau élevé ne débloque pas une commande à drapeau. */
    CHECK(!nh_command_available(gs, nh_find_command("decrypt")));
    free(gs);
}

static void test_dispatch(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_fast(true);
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(true);

    CHECK_INT(run_line(gs, "", out, sizeof out), NH_DISPATCH_EMPTY);
    CHECK_STR(out, "");
    CHECK_INT(run_line(gs, "   \t ", out, sizeof out), NH_DISPATCH_EMPTY);

    CHECK_INT(run_line(gs, "foobar arg", out, sizeof out), NH_DISPATCH_UNKNOWN);
    CHECK(has(out, "Commande inconnue: foobar"));
    CHECK(has(out, "help"));

    /* Commande à drapeau non débloquée / à niveau insuffisant : messages distincts. */
    CHECK_INT(run_line(gs, "bruteforce localhost", out, sizeof out), NH_DISPATCH_LOCKED);
    CHECK(has(out, "Commande non disponible à votre niveau."));
    CHECK_INT(run_line(gs, "advhack nexus-mainframe", out, sizeof out), NH_DISPATCH_LOCKED);
    CHECK(has(out, "niveau 3 requis"));
    CHECK(gs->player.experience == 0); /* rien n'a été exécuté */

    /* Casse et espaces ignorés ; le statut affiche l'état. */
    CHECK_INT(run_line(gs, "  STATUS  ", out, sizeof out), NH_DISPATCH_OK);
    CHECK(has(out, "Testeur"));
    CHECK(has(out, "Niveau"));
    CHECK(has(out, "0/100"));

    /* Un gestionnaire qui échoue (usage) est distingué d'une commande bloquée. */
    CHECK_INT(run_line(gs, "read", out, sizeof out), NH_DISPATCH_FAILED);
    CHECK(has(out, "Usage"));

    /* Commande débloquée et exécutée pour de bon. */
    gs->player.commands_unlocked[CMD_DECRYPT] = true;
    CHECK_INT(run_line(gs, "DECRYPT WKLV#LV#D#WHVW", out, sizeof out), NH_DISPATCH_OK);
    CHECK(has(out, "THIS IS A TEST"));
    CHECK_INT(gs->player.level, LEVEL_APPRENTICE); /* le message de test rapporte 20 XP : niveau 2 */
    gs->player.level = LEVEL_NOVICE;               /* on revient au niveau 1 pour tester les verrous */

    /* quit / exit arrêtent la partie. */
    CHECK(gs->running);
    CHECK_INT(run_line(gs, "exit", out, sizeof out), NH_DISPATCH_OK);
    CHECK(!gs->running);
    CHECK(has(out, "Au revoir"));

    nh_set_lang(NH_LANG_EN);
    CHECK_INT(run_line(gs, "foobar", out, sizeof out), NH_DISPATCH_UNKNOWN);
    CHECK(has(out, "Unknown command: foobar"));
    CHECK_INT(run_line(gs, "socialeng x", out, sizeof out), NH_DISPATCH_LOCKED);
    CHECK(has(out, "level 2 required"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_help(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(true);

    CHECK_INT(run_line(gs, "help", out, sizeof out), NH_DISPATCH_OK);
    CHECK(has(out, "COMMANDES DISPONIBLES"));
    CHECK(has(out, "scan"));
    CHECK(has(out, "quit"));
    CHECK(has(out, "shop"));
    /* Ni les commandes verrouillées, ni celles qui sont cachées. */
    CHECK(!has(out, "bruteforce"));
    CHECK(!has(out, "advhack"));
    CHECK(!has(out, "temporalhack"));
    CHECK(!has(out, "ancien système"));

    gs->player.level = LEVEL_HACKER;
    gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;
    run_line(gs, "help", out, sizeof out);
    CHECK(has(out, "bruteforce"));
    CHECK(has(out, "advhack"));
    CHECK(!has(out, "temporalhack"));

    /* Une commande affichée dans l'aide est exécutable : c'est la garantie de la table unique. */
    size_t count;
    const NhCommand *t = nh_commands(&count);
    for (size_t i = 0; i < count; i++)
    {
        char needle[64];
        snprintf(needle, sizeof needle, "  %-16s- ", t[i].name);
        if (has(out, needle))
            CHECK(nh_command_available(gs, &t[i]) && !t[i].hidden);
    }

    nh_set_lang(NH_LANG_EN);
    run_line(gs, "help", out, sizeof out);
    CHECK(has(out, "AVAILABLE COMMANDS"));
    CHECK(has(out, "Scan the network"));
    nh_set_lang(NH_LANG_FR);

    /* --no-color : aucune séquence d'échappement dans l'aide. */
    nh_term_set_color(false);
    run_line(gs, "help", out, sizeof out);
    CHECK(strchr(out, '\033') == NULL);
    nh_term_set_color(true);
    run_line(gs, "help", out, sizeof out);
    CHECK(strchr(out, '\033') != NULL);
    free(gs);
}

static void test_progression(void)
{
    GameState *gs = new_game();
    char out[4096];

    NhCapture cap = nh_capture_begin();
    nh_grant_xp(gs, 14);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.level, LEVEL_NOVICE);

    cap = nh_capture_begin();
    nh_grant_xp(gs, 1);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.level, LEVEL_APPRENTICE);
    CHECK(gs->player.commands_unlocked[CMD_BRUTEFORCE]);
    CHECK(has(out, "bruteforce"));
    free(gs);
}

static void test_save_command(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    /* « save » figure dans l'aide, dans la catégorie système */
    run_line(gs, "help", out, sizeof out);
    CHECK(has(out, "save"));

    /* sans dossier de données : dit pourquoi, n'écrit rien, et quit n'en parle pas */
    CHECK_INT(run_line(gs, "save", out, sizeof out), NH_DISPATCH_FAILED);
    CHECK(has(out, "Sauvegarde indisponible"));
    CHECK_INT(run_line(gs, "quit", out, sizeof out), NH_DISPATCH_OK);
    CHECK(has(out, "Au revoir"));
    CHECK(!has(out, "sauvegard"));
    CHECK(!gs->running);
    gs->running = true;

    char root[256], path[512];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/dossier/partie.sav", root);
    snprintf(gs->save_path, sizeof gs->save_path, "%s", path);

    CHECK_INT(run_line(gs, "save", out, sizeof out), NH_DISPATCH_OK);
    CHECK(has(out, "Partie sauvegardée."));
    CHECK(nh_storage_exists(path));

    /* quit sauvegarde l'état courant avant de partir */
    gs->player.credits = 4242;
    CHECK_INT(run_line(gs, "quit", out, sizeof out), NH_DISPATCH_OK);
    CHECK(has(out, "Au revoir"));
    CHECK(has(out, "Partie sauvegardée."));
    CHECK(!gs->running);
    GameState *loaded = calloc(1, sizeof *loaded);
    CHECK_INT(nh_load_game(loaded, path), NH_SAVE_OK);
    CHECK_INT(loaded->player.credits, 4242);
    free(loaded);

    /* l'échec est annoncé, avec le chemin, et la commande est marquée en échec */
    char blocker[600];
    snprintf(blocker, sizeof blocker, "%s/bloc", root);
    nh_tmp_write(blocker, "un fichier, pas un dossier");
    snprintf(gs->save_path, sizeof gs->save_path, "%s/bloc/partie.sav", root);
    gs->running = true;
    CHECK_INT(run_line(gs, "save", out, sizeof out), NH_DISPATCH_FAILED);
    CHECK(has(out, "Impossible de sauvegarder"));
    CHECK(has(out, "bloc/partie.sav"));

    /* quit part quand même, même si la sauvegarde échoue */
    CHECK_INT(run_line(gs, "quit", out, sizeof out), NH_DISPATCH_OK);
    CHECK(!gs->running);

    nh_set_lang(NH_LANG_EN);
    gs->running = true;
    CHECK_INT(run_line(gs, "save", out, sizeof out), NH_DISPATCH_FAILED);
    CHECK(has(out, "Could not save the game"));
    nh_set_lang(NH_LANG_FR);

    nh_tmp_remove(root);
    free(gs);
}

int main(void)
{
    test_table_integrity();
    test_find();
    test_state_init();
    test_availability();
    test_dispatch();
    test_help();
    test_progression();
    test_save_command();
    return NH_TEST_REPORT("commands");
}
