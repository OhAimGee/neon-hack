#include "menu.h"

#include <stdio.h>
#include <string.h>

#include "../core/io.h"
#include "../core/parse.h"
#include "../core/platform.h"
#include "../core/settings.h"
#include "../i18n/i18n.h"
#include "../ui/term.h"
#include "intro.h"
#include "progression.h"
#include "save.h"

#define BOX_WIDTH 44

/* Le nom d'une langue s'écrit dans cette langue, quelle que soit celle de l'interface. */
static const char *const k_lang_names[NH_LANG_COUNT] = {
    [NH_LANG_FR] = "Français",
    [NH_LANG_EN] = "English",
};

/* ---- Réglages -------------------------------------------------------------------------------- */

/* Rend effectifs les réglages de la session (langue, couleurs, animations). */
static void apply_runtime(const NhConfig *cfg)
{
    nh_set_lang(cfg->lang);
    nh_term_set_color(cfg->color);
    nh_set_fast(cfg->fast || !nh_stdout_is_tty());
}

/* Un fichier de réglages impossible à écrire ne vaut pas d'interrompre le menu. */
static void persist(const NhPaths *paths, const NhSettings *settings)
{
    if (paths->ok)
        (void)nh_settings_save(settings, paths->settings);
}

/* ---- Affichage ------------------------------------------------------------------------------- */

static void repeat(const char *s, int n)
{
    for (int i = 0; i < n; i++)
        fputs(s, stdout);
}

static void print_title(NhStr title)
{
    char inner[256];
    nh_pad(inner, sizeof inner, nh_tr(title), BOX_WIDTH, NH_ALIGN_CENTER);

    printf("\n%s╔", nh_c(NH_C_CYAN));
    repeat("═", BOX_WIDTH);
    printf("╗\n║%s%s%s║\n╚", nh_c(NH_C_BRIGHT_CYAN), inner, nh_c(NH_C_CYAN));
    repeat("═", BOX_WIDTH);
    printf("╝%s\n\n", nh_c(NH_C_RESET));
}

static void print_choice(int number, const char *label)
{
    printf("  %s[%d]%s %s", nh_c(NH_C_YELLOW), number, nh_c(NH_C_RESET), label);
}

static bool is_blank(const char *s)
{
    for (; *s != '\0'; s++)
        if (*s != ' ' && *s != '\t')
            return false;
    return true;
}

/* Lit un choix dans [0, max] ; Entrée seule donne `default_choice`. Les saisies invalides sont redemandées. */
static NhIoStatus read_choice(int max, int default_choice, int *out)
{
    for (;;)
    {
        printf("\n%s%s%s", nh_c(NH_C_YELLOW), nh_tr(NH_STR_MENU_PROMPT), nh_c(NH_C_RESET));
        fflush(stdout);

        char line[64];
        NhIoStatus status = nh_read_line(line, sizeof line);
        if (status != NH_IO_OK)
            return status;
        if (is_blank(line))
        {
            *out = default_choice;
            return NH_IO_OK;
        }
        if (nh_parse_int(line, 0, max, out))
            return NH_IO_OK;
        printf("%s\n", nh_tr(NH_STR_INVALID_OPTION));
    }
}

static void print_load_error(NhSaveStatus status)
{
    NhStr text = NH_STR_MENU_NO_SAVE;
    switch (status)
    {
    case NH_SAVE_IO:
        text = NH_STR_MENU_LOAD_IO;
        break;
    case NH_SAVE_CORRUPT:
        text = NH_STR_MENU_LOAD_CORRUPT;
        break;
    case NH_SAVE_TOO_NEW:
        text = NH_STR_MENU_LOAD_VERSION;
        break;
    case NH_SAVE_OK:
    case NH_SAVE_NO_PATH:
    case NH_SAVE_MISSING:
        break;
    }
    printf("\n%s%s%s\n", nh_c(NH_C_RED), nh_tr(text), nh_c(NH_C_RESET));
}

/* ---- Options --------------------------------------------------------------------------------- */

static NhIoStatus options_menu(NhConfig *cfg, NhSettings *settings, const NhPaths *paths)
{
    for (;;)
    {
        char text[192];

        print_title(NH_STR_OPT_TITLE);
        snprintf(text, sizeof text, nh_tr(NH_STR_OPT_COLOR),
                 nh_tr(cfg->color ? NH_STR_VALUE_ON : NH_STR_VALUE_OFF));
        print_choice(1, text);
        printf("\n");
        snprintf(text, sizeof text, nh_tr(NH_STR_OPT_ANIM),
                 nh_tr(cfg->fast ? NH_STR_VALUE_OFF : NH_STR_VALUE_ON));
        print_choice(2, text);
        printf("\n");
        snprintf(text, sizeof text, nh_tr(NH_STR_OPT_HUD),
                 nh_tr(cfg->hud ? NH_STR_VALUE_ON : NH_STR_VALUE_OFF));
        print_choice(3, text);
        printf("\n");
        print_choice(0, nh_tr(NH_STR_OPT_BACK));
        printf("\n\n%s\n", nh_tr(NH_STR_OPT_HUD_NOTE));

        int choice = 0;
        NhIoStatus status = read_choice(3, 0, &choice);
        if (status != NH_IO_OK)
            return status;

        switch (choice)
        {
        case 1:
            cfg->color = !cfg->color;
            settings->color = cfg->color;
            break;
        case 2:
            cfg->fast = !cfg->fast;
            settings->fast = cfg->fast;
            break;
        case 3:
            cfg->hud = !cfg->hud;
            settings->hud = cfg->hud;
            break;
        default:
            return NH_IO_OK;
        }
        apply_runtime(cfg);
        persist(paths, settings);
    }
}

/* ---- Nouvelle partie ------------------------------------------------------------------------- */

NhStart nh_start_new_game(GameState *gs, const NhPaths *paths)
{
    init_game(gs);
    if (nh_intro_run(gs) == NH_INTRO_EOF)
        return NH_START_EOF;

    if (paths->ok)
    {
        snprintf(gs->save_path, sizeof gs->save_path, "%s", paths->save);
        /* Dès maintenant : « Continuer » doit reprendre au début du tutoriel si on quitte tout de suite. */
        if (nh_save_game(gs) != NH_SAVE_OK)
        {
            printf(nh_tr(NH_STR_SAVE_FAILED), gs->save_path);
            printf("\n");
        }
    }
    return NH_START_PLAY;
}

/* ---- Menu principal -------------------------------------------------------------------------- */

NhStart nh_menu_run(NhConfig *cfg, NhSettings *settings, const NhPaths *paths, GameState *gs,
                    bool *resumed)
{
    *resumed = false;
    if (!paths->ok)
        printf("\n%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_MENU_NO_STORAGE), nh_c(NH_C_RESET));

    for (;;)
    {
        NhSaveInfo info;
        NhSaveStatus peek = paths->ok ? nh_save_peek(paths->save, &info) : NH_SAVE_NO_PATH;
        bool has_save = peek == NH_SAVE_OK;

        print_title(NH_STR_MENU_TITLE);
        if (has_save)
        {
            char summary[200];
            snprintf(summary, sizeof summary, nh_tr(NH_STR_MENU_CONTINUE_INFO), info.name, info.level,
                     nh_level_name(info.level), info.credits);
            print_choice(1, nh_tr(NH_STR_MENU_CONTINUE));
            printf("  %s%s%s\n", nh_c(NH_C_CYAN), summary, nh_c(NH_C_RESET));
        }
        else
        {
            /* Fichier absent, ou présent mais inutilisable : ce n'est pas la même chose pour le joueur. */
            bool unreadable = peek != NH_SAVE_MISSING && peek != NH_SAVE_NO_PATH;
            print_choice(1, nh_tr(unreadable ? NH_STR_MENU_CONTINUE_BAD : NH_STR_MENU_CONTINUE_NONE));
            printf("\n");
        }
        print_choice(2, nh_tr(NH_STR_MENU_NEW));
        printf("\n");
        char language[96];
        snprintf(language, sizeof language, nh_tr(NH_STR_MENU_LANGUAGE), k_lang_names[cfg->lang]);
        print_choice(3, language);
        printf("\n");
        print_choice(4, nh_tr(NH_STR_MENU_OPTIONS));
        printf("\n");
        print_choice(0, nh_tr(NH_STR_MENU_QUIT));
        printf("\n");

        int choice = 0;
        if (read_choice(4, has_save ? 1 : 2, &choice) != NH_IO_OK)
            return NH_START_EOF;

        switch (choice)
        {
        case 0:
            return NH_START_QUIT;

        case 1:
            if (!has_save)
            {
                print_load_error(peek);
                break;
            }
            {
                NhSaveStatus loaded = nh_load_game(gs, paths->save);
                if (loaded == NH_SAVE_OK)
                {
                    snprintf(gs->save_path, sizeof gs->save_path, "%s", paths->save);
                    *resumed = true;
                    return NH_START_PLAY;
                }
                print_load_error(loaded);
            }
            break;

        case 2:
            /* Même une sauvegarde illisible est une sauvegarde : on ne l'écrase pas sans demander. */
            if (paths->ok && nh_storage_exists(paths->save))
            {
                char answer[64];
                printf("\n%s%s%s", nh_c(NH_C_YELLOW), nh_tr(NH_STR_MENU_OVERWRITE), nh_c(NH_C_RESET));
                fflush(stdout);
                if (nh_read_line(answer, sizeof answer) != NH_IO_OK)
                    return NH_START_EOF;
                if (!nh_parse_yes_no(answer, false))
                    break;
            }
            return nh_start_new_game(gs, paths);

        case 3:
            cfg->lang = (NhLang)(((int)cfg->lang + 1) % NH_LANG_COUNT);
            settings->lang = cfg->lang;
            apply_runtime(cfg);
            persist(paths, settings);
            break;

        case 4:
            if (options_menu(cfg, settings, paths) != NH_IO_OK)
                return NH_START_EOF;
            break;

        default:
            break;
        }
    }
}
