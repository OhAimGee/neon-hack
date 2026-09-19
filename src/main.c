#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "core/config.h"
#include "core/platform.h"
#include "core/settings.h"
#include "core/storage.h"
#include "game/game.h"
#include "game/menu.h"
#include "game/tutorial.h"
#include "i18n/i18n.h"
#include "ui/hud.h"
#include "ui/term.h"

int main(int argc, char **argv)
{
    NhConfig config;
    NhSettings settings;
    char err[128];

    const char *env_no_color = getenv("NO_COLOR");
    nh_config_defaults(&config, getenv("LANG"), env_no_color);
    nh_settings_from_config(&settings, &config); /* défauts issus de l'environnement */

    switch (nh_config_parse(argc, argv, &config, err, sizeof(err), stdout))
    {
    case NH_CFG_EXIT_OK:
        return 0;
    case NH_CFG_ERROR:
        fprintf(stderr, "neon_hack: %s\n", err);
        nh_print_usage(stderr);
        return 2;
    case NH_CFG_RUN:
        break;
    }

    /* Réglages enregistrés (langue, couleurs…) : ils passent après l'environnement, avant la ligne de commande. */
    NhPaths paths;
    nh_paths_init(&paths, config.data_dir);
    if (paths.ok)
        (void)nh_settings_load(&settings, paths.settings);
    nh_config_apply_settings(&config, &settings, env_no_color != NULL && env_no_color[0] != '\0');

    nh_platform_init();
    nh_set_lang(config.lang);
    nh_term_set_color(config.color);
    // Sans terminal (sortie redirigée), les animations n'ont aucun intérêt.
    nh_set_fast(config.fast || !nh_stdout_is_tty());
    srand(config.has_seed ? (unsigned)config.seed : (unsigned)time(NULL));

    /* L'état est volumineux (~170 Ko) : sur le tas plutôt que sur la pile. */
    GameState *gs = calloc(1, sizeof *gs);
    if (gs == NULL)
    {
        fprintf(stderr, "neon_hack: out of memory\n");
        return 1;
    }

    printf("\n");
    print_cyberpunk_art();
    printf("\n");

    /* --new : nouvelle partie tout de suite ; sinon le menu (Continuer, Nouvelle partie, langue…). */
    bool resumed = false;
    NhStart start = config.new_game ? nh_start_new_game(gs, &paths)
                                    : nh_menu_run(&config, &settings, &paths, gs, &resumed);

    if (start == NH_START_EOF)
        printf("\n%s\n", nh_tr(NH_STR_INPUT_CLOSED));
    if (start == NH_START_PLAY)
    {
        nh_hud_start(!config.hud);
        /* Après le démarrage de l'interface fixe, qui repousse tout ce qui s'affichait avant elle. */
        nh_tutorial_announce(gs, resumed);
        game_loop(gs);
    }

    printf("\n%s\n", nh_tr(NH_STR_BYE_1));
    printf("%s\n", nh_tr(NH_STR_BYE_2));
    nh_hud_stop();

    free(gs);
    return 0;
}
