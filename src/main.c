#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "core/config.h"
#include "core/platform.h"
#include "game/game.h"
#include "i18n/i18n.h"
#include "ui/term.h"

int main(int argc, char **argv)
{
    NhConfig config;
    char err[128];

    nh_config_defaults(&config, getenv("LANG"), getenv("NO_COLOR"));
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

    init_game(gs);
    display_intro(gs);
    game_loop(gs);

    printf("\n%s\n", nh_tr(NH_STR_BYE_1));
    printf("%s\n", nh_tr(NH_STR_BYE_2));

    free(gs);
    return 0;
}
