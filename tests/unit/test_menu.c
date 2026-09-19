#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"
#include "nh_tmp.h"

#include "../../src/core/platform.h"
#include "../../src/core/settings.h"
#include "../../src/game/menu.h"
#include "../../src/game/save.h"
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

typedef struct
{
    char dir[256];
    NhPaths paths;
    NhConfig cfg;
    NhSettings settings;
    GameState *gs;
    bool resumed;
    char out[65536];
} Env;

static void env_open(Env *e)
{
    memset(e, 0, sizeof *e);
    nh_tmp_make(e->dir, sizeof e->dir);
    nh_paths_from_dir(&e->paths, e->dir);
    nh_config_defaults(&e->cfg, "fr_FR.UTF-8", NULL);
    nh_settings_from_config(&e->settings, &e->cfg);
    e->gs = calloc(1, sizeof *e->gs);
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
}

static void env_close(Env *e)
{
    free(e->gs);
    nh_tmp_remove(e->dir);
    nh_set_lang(NH_LANG_FR);
}

static NhStart menu(Env *e, const char *input)
{
    nh_feed(input);
    NhCapture cap = nh_capture_begin();
    NhStart r = nh_menu_run(&e->cfg, &e->settings, &e->paths, e->gs, &e->resumed);
    nh_capture_end(&cap, e->out, sizeof e->out);
    return r;
}

/* Écrit une partie « Molly, niveau 2, 321 ¢ » dans le fichier de sauvegarde de `e`. */
static void make_save(Env *e)
{
    GameState *g = calloc(1, sizeof *g);
    init_game(g);
    strcpy(g->player.name, "Molly");
    g->player.level = LEVEL_APPRENTICE;
    g->player.credits = 321;
    snprintf(g->save_path, sizeof g->save_path, "%s", e->paths.save);
    CHECK_INT(nh_save_game(g), NH_SAVE_OK);
    free(g);
}

static char *settings_file(Env *e) { return nh_tmp_slurp(e->paths.settings); }

/* ---- Nouvelle partie --------------------------------------------------------------------------- */

static void test_new_game_default_flow(void)
{
    Env e;
    env_open(&e);

    /* aucune sauvegarde : Entrée choisit « Nouvelle partie » ; puis nom, confirmation, tutoriel */
    CHECK_INT(menu(&e, "\n\n\n\n"), NH_START_PLAY);
    CHECK(!e.resumed);
    CHECK_STR(e.gs->player.name, "Case");
    CHECK(nh_tutorial_active(e.gs));
    CHECK_STR(e.gs->save_path, e.paths.save);
    CHECK(has(e.out, "Continuer (aucune sauvegarde)"));

    /* la partie est enregistrée tout de suite : quitter aussitôt ne perd pas le prologue */
    NhSaveInfo info;
    CHECK_INT(nh_save_peek(e.paths.save, &info), NH_SAVE_OK);
    CHECK_STR(info.name, "Case");
    CHECK(info.tutorial_active);
    env_close(&e);
}

static void test_new_game_named_and_skipped(void)
{
    Env e;
    env_open(&e);
    CHECK_INT(menu(&e, "2\nMolly\n\n2\n"), NH_START_PLAY);
    CHECK_STR(e.gs->player.name, "Molly");
    CHECK(!nh_tutorial_active(e.gs));
    NhSaveInfo info;
    CHECK_INT(nh_save_peek(e.paths.save, &info), NH_SAVE_OK);
    CHECK_STR(info.name, "Molly");
    env_close(&e);
}

static void test_new_game_overwrite_confirmation(void)
{
    Env e;
    env_open(&e);
    make_save(&e);
    char *before = nh_tmp_slurp(e.paths.save);

    /* refus (Entrée = non) : retour au menu, sauvegarde intacte ; puis on quitte */
    CHECK_INT(menu(&e, "2\n\n0\n"), NH_START_QUIT);
    CHECK(has(e.out, "remplacera"));
    char *after = nh_tmp_slurp(e.paths.save);
    CHECK_STR(after, before);
    free(after);

    CHECK_INT(menu(&e, "2\nnon\n0\n"), NH_START_QUIT);
    after = nh_tmp_slurp(e.paths.save);
    CHECK_STR(after, before);
    free(after);

    /* acceptation : le prologue se joue, l'ancienne partie est remplacée */
    CHECK_INT(menu(&e, "2\no\nJack\n\n2\n"), NH_START_PLAY);
    CHECK_STR(e.gs->player.name, "Jack");
    NhSaveInfo info;
    CHECK_INT(nh_save_peek(e.paths.save, &info), NH_SAVE_OK);
    CHECK_STR(info.name, "Jack");
    CHECK_INT(info.credits, e.gs->player.credits);

    free(before);
    env_close(&e);
}

static void test_new_game_over_corrupt_save_asks_too(void)
{
    Env e;
    env_open(&e);
    nh_tmp_write(e.paths.save, "pas une sauvegarde");

    CHECK_INT(menu(&e, "2\n\n0\n"), NH_START_QUIT);
    CHECK(has(e.out, "remplacera"));
    char *data = nh_tmp_slurp(e.paths.save);
    CHECK_STR(data, "pas une sauvegarde");
    free(data);
    env_close(&e);
}

/* ---- Continuer ---------------------------------------------------------------------------------- */

static void test_continue(void)
{
    Env e;
    env_open(&e);
    make_save(&e);

    /* Entrée = Continuer quand une sauvegarde existe */
    CHECK_INT(menu(&e, "\n"), NH_START_PLAY);
    CHECK(e.resumed);
    CHECK_STR(e.gs->player.name, "Molly");
    CHECK_INT(e.gs->player.level, LEVEL_APPRENTICE);
    CHECK_INT(e.gs->player.credits, 321);
    CHECK_STR(e.gs->save_path, e.paths.save); /* la sauvegarde automatique reprend au même endroit */
    CHECK(has(e.out, "Molly"));                /* la ligne de résumé */
    CHECK(has(e.out, "321"));

    /* le choix explicite fait la même chose */
    CHECK_INT(menu(&e, "1\n"), NH_START_PLAY);
    CHECK(e.resumed);
    env_close(&e);
}

static void test_continue_without_save(void)
{
    Env e;
    env_open(&e);

    /* pas de sauvegarde : message, et on reste dans le menu (aucune boucle, aucun plantage) */
    CHECK_INT(menu(&e, "1\n0\n"), NH_START_QUIT);
    CHECK(has(e.out, "Aucune sauvegarde à charger"));
    CHECK_INT(count(e.out, "MENU PRINCIPAL"), 2);
    CHECK(!e.resumed);
    env_close(&e);
}

static void test_continue_corrupt_and_future(void)
{
    Env e;
    env_open(&e);

    nh_tmp_write(e.paths.save, "neon-hack-save=1\nplayer.level=abc\nend=1\n");
    CHECK_INT(menu(&e, "1\n0\n"), NH_START_QUIT);
    CHECK(has(e.out, "sauvegarde illisible")); /* le menu ne prétend pas qu'il n'y en a pas */
    CHECK(!has(e.out, "aucune sauvegarde"));
    CHECK(has(e.out, "corrompue"));
    CHECK(!e.resumed);

    nh_tmp_write(e.paths.save, "neon-hack-save=9\nend=1\n");
    CHECK_INT(menu(&e, "1\n0\n"), NH_START_QUIT);
    CHECK(has(e.out, "version plus récente"));

    /* Entrée sur une sauvegarde illisible propose la nouvelle partie, pas un chargement voué à l'échec */
    CHECK_INT(menu(&e, "\n\n0\n"), NH_START_QUIT); /* nouvelle partie, refus d'écraser, quitter */
    CHECK(has(e.out, "remplacera"));
    env_close(&e);
}

/* ---- Langue et options ------------------------------------------------------------------------- */

static void test_language_toggle_persists(void)
{
    Env e;
    env_open(&e);
    CHECK_INT(e.cfg.lang, NH_LANG_FR);

    CHECK_INT(menu(&e, "3\n0\n"), NH_START_QUIT);
    CHECK_INT(e.cfg.lang, NH_LANG_EN);
    CHECK_INT(e.settings.lang, NH_LANG_EN);
    CHECK_INT(nh_get_lang(), NH_LANG_EN); /* effectif tout de suite */
    CHECK(has(e.out, "Langue : Français"));
    CHECK(has(e.out, "Language: English")); /* le menu suivant est déjà en anglais */
    CHECK(has(e.out, "MAIN MENU"));

    char *file = settings_file(&e);
    CHECK(file != NULL && has(file, "lang=en"));
    free(file);

    /* deuxième bascule : retour au français */
    CHECK_INT(menu(&e, "3\n0\n"), NH_START_QUIT);
    CHECK_INT(e.cfg.lang, NH_LANG_FR);
    file = settings_file(&e);
    CHECK(file != NULL && has(file, "lang=fr"));
    free(file);

    /* nouvelle session : le réglage est relu */
    NhSettings again;
    nh_settings_from_config(&again, &e.cfg);
    again.lang = NH_LANG_EN;
    CHECK(nh_settings_load(&again, e.paths.settings));
    CHECK_INT(again.lang, NH_LANG_FR);
    env_close(&e);
}

static void test_options_toggles(void)
{
    Env e;
    env_open(&e);
    bool color0 = e.cfg.color, fast0 = e.cfg.fast, hud0 = e.cfg.hud;

    CHECK_INT(menu(&e, "4\n1\n2\n3\n0\n0\n"), NH_START_QUIT);
    CHECK_INT(e.cfg.color, !color0);
    CHECK_INT(e.cfg.fast, !fast0);
    CHECK_INT(e.cfg.hud, !hud0);
    CHECK_INT(e.settings.color, !color0);
    CHECK_INT(e.settings.fast, !fast0);
    CHECK_INT(e.settings.hud, !hud0);
    CHECK(has(e.out, "OPTIONS"));
    CHECK(has(e.out, "ACTIVÉ"));
    CHECK(has(e.out, "DÉSACTIVÉ"));

    NhSettings loaded;
    nh_settings_from_config(&loaded, &e.cfg);
    CHECK(nh_settings_load(&loaded, e.paths.settings));
    CHECK_INT(loaded.color, !color0);
    CHECK_INT(loaded.fast, !fast0);
    CHECK_INT(loaded.hud, !hud0);

    /* les couleurs suivent immédiatement (tests : sortie sans couleur, on vérifie l'état) */
    CHECK_INT(nh_term_color_enabled(), e.cfg.color);
    nh_term_set_color(false);
    env_close(&e);
}

static void test_settings_not_overwritten_by_cli_choices(void)
{
    Env e;
    env_open(&e);
    /* Réglage enregistré : couleurs actives. Cette session : --no-color en ligne de commande. */
    e.settings.color = true;
    e.cfg.color = false;
    e.cfg.color_set = true;

    CHECK_INT(menu(&e, "3\n0\n"), NH_START_QUIT); /* on ne change que la langue */
    char *file = settings_file(&e);
    CHECK(file != NULL && has(file, "color=1")); /* la valeur de la ligne de commande n'est pas persistée */
    free(file);
    env_close(&e);
}

/* ---- Entrées invalides et fermeture ---------------------------------------------------------- */

static void test_invalid_choices(void)
{
    Env e;
    env_open(&e);
    CHECK_INT(menu(&e, "abc\n9\n-1\n2x\n0\n"), NH_START_QUIT);
    CHECK_INT(count(e.out, "Option invalide."), 4);
    env_close(&e);
}

static void test_eof_everywhere(void)
{
    Env e;
    env_open(&e);

    CHECK_INT(menu(&e, ""), NH_START_EOF);           /* menu principal */
    CHECK_INT(menu(&e, "4\n"), NH_START_EOF);        /* options */
    CHECK_INT(menu(&e, "4\n1\n"), NH_START_EOF);     /* options après un réglage */
    CHECK_INT(menu(&e, "2\nMolly\n"), NH_START_EOF); /* prologue : fin avant la confirmation */
    CHECK_INT(menu(&e, "2\nMolly\n\n"), NH_START_EOF);
    CHECK_INT(menu(&e, "1"), NH_START_EOF); /* dernière ligne sans \n : le choix est lu, puis fin */

    /* fin pendant la confirmation d'écrasement */
    make_save(&e);
    CHECK_INT(menu(&e, "2\n"), NH_START_EOF);
    env_close(&e);
}

static void test_eof_in_prologue_leaves_no_save(void)
{
    Env e;
    env_open(&e);
    CHECK_INT(menu(&e, "2\nMolly\n"), NH_START_EOF);
    CHECK(!nh_storage_exists(e.paths.save)); /* rien n'est écrit tant que le prologue n'est pas terminé */
    env_close(&e);
}

/* ---- Sans dossier de données ------------------------------------------------------------------- */

static void test_no_storage(void)
{
    Env e;
    env_open(&e);
    nh_paths_from_dir(&e.paths, ""); /* aucun dossier utilisable */
    CHECK(!e.paths.ok);

    CHECK_INT(menu(&e, "1\n2\n\n\n\n"), NH_START_PLAY);
    CHECK(has(e.out, "Aucun dossier de données"));
    CHECK(has(e.out, "Aucune sauvegarde à charger"));
    CHECK_STR(e.gs->save_path, ""); /* jamais de sauvegarde vers un chemin vide ou inventé */
    CHECK_STR(e.gs->player.name, "Case");
    CHECK_INT(nh_save_game(e.gs), NH_SAVE_NO_PATH);

    /* les réglages ne sont pas écrits non plus, sans plantage */
    CHECK_INT(menu(&e, "3\n0\n"), NH_START_QUIT);
    CHECK_INT(e.cfg.lang, NH_LANG_EN);
    env_close(&e);
}

static void test_start_new_game_direct(void)
{
    Env e;
    env_open(&e);

    nh_feed("Trinity\n\n1\n");
    NhCapture cap = nh_capture_begin();
    NhStart r = nh_start_new_game(e.gs, &e.paths);
    nh_capture_end(&cap, e.out, sizeof e.out);
    CHECK_INT(r, NH_START_PLAY);
    CHECK_STR(e.gs->player.name, "Trinity");
    CHECK_STR(e.gs->save_path, e.paths.save);
    CHECK(nh_storage_exists(e.paths.save));

    /* un état déjà avancé est bien remplacé par un état neuf */
    e.gs->player.credits = 99999;
    nh_feed("\n\n2\n");
    cap = nh_capture_begin();
    r = nh_start_new_game(e.gs, &e.paths);
    nh_capture_end(&cap, e.out, sizeof e.out);
    CHECK_INT(r, NH_START_PLAY);
    CHECK(e.gs->player.credits != 99999);
    CHECK_STR(e.gs->player.name, "Case");

    nh_feed("");
    cap = nh_capture_begin();
    r = nh_start_new_game(e.gs, &e.paths);
    nh_capture_end(&cap, e.out, sizeof e.out);
    CHECK_INT(r, NH_START_EOF);
    env_close(&e);
}

int main(void)
{
    nh_set_fast(true);

    test_new_game_default_flow();
    test_new_game_named_and_skipped();
    test_new_game_overwrite_confirmation();
    test_new_game_over_corrupt_save_asks_too();
    test_continue();
    test_continue_without_save();
    test_continue_corrupt_and_future();
    test_language_toggle_persists();
    test_options_toggles();
    test_settings_not_overwritten_by_cli_choices();
    test_invalid_choices();
    test_eof_everywhere();
    test_eof_in_prologue_leaves_no_save();
    test_no_storage();
    test_start_new_game_direct();

    nh_unfeed();
    return NH_TEST_REPORT("menu");
}
