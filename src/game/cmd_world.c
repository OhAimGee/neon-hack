/*
 * Commandes du monde : boutique et alerte (les quêtes, `quests`, sont dans quest_system.c ; les
 * contacts et les messages, dans contacts.c).
 *
 * Code d'origine (neon_hack.c, v2.087) déplacé tel quel et adapté à GameState :
 * les variables globales sont devenues des champs de `gs`. La logique sera
 * réécrite en Phase 3 ; d'ici là, seuls les accès à l'état ont changé.
 */
#include "game.h"

#include "../core/io.h"
#include "../core/platform.h"
#include "../i18n/i18n.h"
#include "legacy_colors.h"
#include "progression.h"
#include "shop_view.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>


bool cmd_shop(GameState *gs, const char *arg)
{
    (void)arg;

    // Vérifier si la boutique est accessible
    if (nh_alert_shop_closed(&gs->alert))
    {
        printf(nh_tr(NH_STR_SHOP_CLOSED), gs->alert.level);
        printf("\n");
        return false;
    }

    nh_shop_show(&gs->shop, gs->player.credits, gs->player.level);

    char input[16];
    printf("%s%s%s", nh_c(NH_C_MAGENTA), nh_tr(NH_STR_SHOP_PROMPT), nh_c(NH_C_RESET));
    if (nh_read_line(input, sizeof input) != NH_IO_OK)
        return false;

    // Entrée seule, comme 0 : on quitte.
    int choice = 0;
    bool blank = input[strspn(input, " \t")] == '\0';
    if (!blank && !nh_parse_int(input, 0, ITEM_COUNT, &choice))
    {
        printf("%s\n", nh_tr(NH_STR_SHOP_INVALID));
        return true;
    }
    if (choice == 0)
    {
        printf("%s\n", nh_tr(NH_STR_SHOP_BYE));
        return true;
    }

    /* Vérifications, paiement, effet, événement et messages : tout est dans nh_shop_buy. Un refus
     * (niveau, crédits, réserve pleine) est expliqué à l'écran ; la commande, elle, a bien eu lieu. */
    (void)nh_shop_buy(gs, (ShopItemType)(choice - 1));
    return true;
}

bool cmd_lay_low(GameState *gs, const char *arg)
{
    (void)arg;

    nh_alert_print_status(&gs->alert);
    nh_alert_print_menu(&gs->alert);

    char input[16];
    printf("\n%s", nh_tr(NH_STR_ALERT_PROMPT));
    if (nh_read_line(input, sizeof(input)) != NH_IO_OK)
        return false;

    int choice = 0;
    if (!nh_parse_int(input, 0, NH_REDUCTION_COUNT, &choice))
    {
        printf("%s\n", nh_tr(NH_STR_ALERT_INVALID_CHOICE));
        return false;
    }

    if (choice == 0)
    {
        printf("%s\n", nh_tr(NH_STR_ALERT_STAY_HIDDEN));
        return true;
    }
    static const NhStr k_done[NH_REDUCTION_COUNT] = {
        [NH_REDUCTION_TIME] = NH_STR_ALERT_DONE_TIME,     [NH_REDUCTION_VPN] = NH_STR_ALERT_DONE_VPN,
        [NH_REDUCTION_PROXY] = NH_STR_ALERT_DONE_PROXY,   [NH_REDUCTION_GHOST] = NH_STR_ALERT_DONE_GHOST,
        [NH_REDUCTION_LAYLOW] = NH_STR_ALERT_DONE_LAYLOW, [NH_REDUCTION_FRAME] = NH_STR_ALERT_DONE_FRAME,
    };

    NhReduction method = (NhReduction)(choice - 1);
    int before = gs->alert.level;
    switch (nh_alert_apply_reduction(&gs->alert, method, &gs->player.credits, NULL))
    {
    case NH_REDUCE_NO_CREDITS:
        printf(nh_tr(NH_STR_ALERT_NO_CREDITS), nh_alert_reduction_cost(method));
        printf("\n");
        return false;
    case NH_REDUCE_NO_GHOST:
        printf("%s\n", nh_tr(NH_STR_ALERT_NO_GHOST));
        return false;
    case NH_REDUCE_OK:
        break;
    }

    printf("%s\n", nh_tr(k_done[method]));
    if (method == NH_REDUCTION_FRAME)
    {
        nh_grant_reputation(gs, -5);
        printf("%s\n", nh_tr(NH_STR_ALERT_KARMA));
    }
    printf(nh_tr(NH_STR_ALERT_NOW), before, gs->alert.level);
    printf("\n");
    return true;
}
