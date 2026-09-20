#include "shop.h"

#include <stdio.h>
#include <string.h>

#include "../i18n/i18n.h"
#include "../ui/term.h"
#include "game.h"
#include "progression.h"
#include "world.h"

/*
 * Le catalogue. Les noms sont des marques : identiques en français et en anglais. Les résumés
 * affichés par shop_view.c décrivent les effets d'apply() ci-dessous ; test_shop.c vérifie que les
 * nombres qu'ils citent sont bien ceux du code.
 *
 * Unique : acheté une fois, puis « épuisé ». Consommable : peut être racheté quand la réserve n'est
 * pas pleine.
 */
static const struct
{
    const char *name;
    int price;
    int level;
    bool consumable;
} k_items[ITEM_COUNT] = {
    [ITEM_STEALTH_UPGRADE] = {"Stealth Module v2.0", 150, 2, false},
    [ITEM_ALERT_REDUCER] = {"Ghost Protocol", 80, 1, true},
    [ITEM_VIRUS_PACK] = {"Malware Arsenal", 120, 4, false}, /* niveau 4 : `uploadvirus` s'y débloque */
    [ITEM_PROXY_CHAIN] = {"Proxy Chain Pro", 100, 2, true},
    [ITEM_ENCRYPTION_KEY] = {"Quantum Encryption Key", 200, 3, false},
    [ITEM_AI_MODULE] = {"Neural Assistant v3.1", 500, 4, false},
    [ITEM_QUANTUM_CHIP] = {"Quantum Processing Chip", 800, 5, false},
    [ITEM_REPUTATION_BOOST] = {"Street Cred Booster", 75, 1, true},
    [ITEM_XP_BOOST] = {"Neural Accelerator", 90, 2, true},
    [ITEM_VPN_SERVICE] = {"Dark Web VPN", 60, 1, false}, /* un abonnement : acheté une fois, actif ensuite */
};

void init_shop(CyberShop *shop)
{
    memset(shop, 0, sizeof *shop);
    snprintf(shop->vendor_name, sizeof shop->vendor_name, "%s", "R4Z0R");
    snprintf(shop->shop_location, sizeof shop->shop_location, "%s", "Underground Market - Sector 7");
    shop->is_open = true;
    shop->item_count = ITEM_COUNT;

    for (int i = 0; i < ITEM_COUNT; i++)
    {
        ShopItem *item = &shop->items[i];
        item->type = (ShopItemType)i;
        snprintf(item->name, sizeof item->name, "%s", k_items[i].name);
        item->price = k_items[i].price;
        item->level_required = k_items[i].level;
        item->is_consumable = k_items[i].consumable;
        item->is_available = true;
    }
}

/* ---- Effets ---------------------------------------------------------------------------------- */

/* L'objet ne changerait plus rien : on refuse l'achat plutôt que de prendre les crédits pour rien. */
static bool is_maxed(const GameState *gs, ShopItemType type)
{
    const Player *p = &gs->player;
    switch (type)
    {
    case ITEM_STEALTH_UPGRADE:
        return p->stealth_rating >= NH_STEALTH_MAX;
    case ITEM_ALERT_REDUCER:
        return gs->alert.ghost_protocols_available >= NH_GHOST_MAX;
    case ITEM_VIRUS_PACK:
        return p->virus_library_size >= gs->virus_count;
    case ITEM_PROXY_CHAIN:
        return gs->alert.proxy_hacks_left >= NH_PROXY_HACKS_MAX;
    case ITEM_ENCRYPTION_KEY:
        return p->has_encryption_key;
    case ITEM_AI_MODULE:
        return p->has_ai_assistant;
    case ITEM_QUANTUM_CHIP:
        return p->has_quantum_computer;
    case ITEM_REPUTATION_BOOST:
        return false;
    case ITEM_XP_BOOST:
        return p->xp_boost >= NH_XP_BOOST_CHARGES;
    case ITEM_VPN_SERVICE:
        return gs->alert.vpn_active;
    case ITEM_COUNT:
        break;
    }
    return true;
}

static void say(NhStr text, int a, int b)
{
    printf("%s", nh_c(NH_C_BRIGHT_GREEN));
    printf(nh_tr(text), a, b);
    printf("%s\n", nh_c(NH_C_RESET));
}

static void apply(GameState *gs, ShopItemType type)
{
    Player *p = &gs->player;
    switch (type)
    {
    case ITEM_STEALTH_UPGRADE:
        p->stealth_rating += NH_STEALTH_MODULE_BONUS;
        if (p->stealth_rating > NH_STEALTH_MAX)
            p->stealth_rating = NH_STEALTH_MAX;
        say(NH_STR_SHOP_FX_STEALTH, p->stealth_rating, NH_STEALTH_MAX);
        break;
    case ITEM_ALERT_REDUCER:
        gs->alert.ghost_protocols_available++;
        say(NH_STR_SHOP_FX_GHOST, gs->alert.ghost_protocols_available, 0);
        break;
    case ITEM_VIRUS_PACK:
        p->virus_library_size = gs->virus_count;
        nh_world_sync_tools(gs);
        say(NH_STR_SHOP_FX_VIRUS, p->virus_library_size, 0);
        break;
    case ITEM_PROXY_CHAIN:
        gs->alert.proxy_hacks_left += NH_PROXY_HACKS;
        if (gs->alert.proxy_hacks_left > NH_PROXY_HACKS_MAX)
            gs->alert.proxy_hacks_left = NH_PROXY_HACKS_MAX;
        say(NH_STR_SHOP_FX_PROXY, gs->alert.proxy_hacks_left, 0);
        break;
    case ITEM_ENCRYPTION_KEY:
        p->has_encryption_key = true;
        say(NH_STR_SHOP_FX_KEY, NH_KEY_DECRYPT_LEVEL, 0);
        /* La clé ouvre aussi, tout de suite, les fichiers de bas niveau des systèmes déjà compromis. */
        for (int i = 0; i < NH_MAX_NODES; i++)
            if (gs->nodes[i].is_compromised)
                nh_world_extract_upto(gs, i, NH_KEY_DECRYPT_LEVEL);
        break;
    case ITEM_AI_MODULE:
        p->has_ai_assistant = true;
        nh_world_sync_tools(gs);
        say(NH_STR_SHOP_FX_AI, 0, 0);
        break;
    case ITEM_QUANTUM_CHIP:
        p->has_quantum_computer = true;
        nh_world_sync_tools(gs);
        say(NH_STR_SHOP_FX_QUANTUM, 0, 0);
        break;
    case ITEM_REPUTATION_BOOST:
        nh_grant_reputation(gs, NH_REPUTATION_BOOST_POINTS);
        printf("\n");
        break;
    case ITEM_XP_BOOST:
        p->xp_boost = NH_XP_BOOST_CHARGES;
        say(NH_STR_SHOP_FX_XP, NH_XP_BOOST_CHARGES, 0);
        break;
    case ITEM_VPN_SERVICE:
        gs->alert.vpn_active = true;
        say(NH_STR_SHOP_FX_VPN, 0, 0);
        break;
    case ITEM_COUNT:
        break;
    }
}

/* ---- Achat ----------------------------------------------------------------------------------- */

static NhBuyResult refuse(NhBuyResult result, NhStr text, int value)
{
    printf("%s", nh_c(NH_C_RED));
    printf(nh_tr(text), value);
    printf("%s\n", nh_c(NH_C_RESET));
    return result;
}

NhBuyResult nh_shop_buy(GameState *gs, ShopItemType type)
{
    if ((int)type < 0 || type >= ITEM_COUNT)
        return refuse(NH_BUY_INVALID, NH_STR_SHOP_BUY_INVALID, 0);

    ShopItem *item = &gs->shop.items[type];
    if (!item->is_available)
        return refuse(NH_BUY_SOLD_OUT, NH_STR_SHOP_BUY_SOLD_OUT, 0);
    if ((int)gs->player.level < item->level_required)
        return refuse(NH_BUY_LOW_LEVEL, NH_STR_SHOP_BUY_LOW_LEVEL, item->level_required);
    if (gs->player.credits < item->price)
        return refuse(NH_BUY_LOW_CREDITS, NH_STR_SHOP_BUY_LOW_CREDITS, item->price);
    if (is_maxed(gs, type))
        return refuse(NH_BUY_MAXED, NH_STR_SHOP_BUY_MAXED, 0);

    gs->player.credits -= item->price;
    gs->shop.bought |= 1u << (unsigned)type;

    printf("%s", nh_c(NH_C_GREEN));
    printf(nh_tr(NH_STR_SHOP_BOUGHT), item->name, item->price);
    printf("%s\n", nh_c(NH_C_RESET));
    printf("%s", nh_c(NH_C_CYAN));
    printf(nh_tr(NH_STR_SHOP_CREDITS_LEFT), gs->player.credits);
    printf("%s\n", nh_c(NH_C_RESET));

    if (!item->is_consumable)
    {
        item->is_available = false;
        printf("%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_SHOP_UNIQUE_GONE), nh_c(NH_C_RESET));
    }

    apply(gs, type);
    nh_event(gs, NH_EV_ITEM_BOUGHT, (int)type);
    return NH_BUY_OK;
}
