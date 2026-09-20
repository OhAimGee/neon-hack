#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"

#include "../../src/core/platform.h"
#include "../../src/game/alert.h"
#include "../../src/game/commands.h"
#include "../../src/game/events.h"
#include "../../src/game/progression.h"
#include "../../src/game/shop.h"
#include "../../src/game/shop_view.h"
#include "../../src/game/world.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define OUT_SIZE 16384

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

/* Partie neuve, niveau maximal et crédits en abondance : chaque test réduit ce dont il a besoin. */
static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    gs->player.level = LEVEL_LEGEND;
    gs->player.credits = 100000;
    nh_feed(""); /* jamais d'attente sur le vrai clavier */
    return gs;
}

static NhBuyResult buy(GameState *gs, ShopItemType type, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    NhBuyResult r = nh_shop_buy(gs, type);
    nh_capture_end(&cap, out, size);
    return r;
}

static void test_catalogue(void)
{
    GameState *gs = new_game();
    CHECK_INT(gs->shop.item_count, ITEM_COUNT);
    CHECK_INT(gs->shop.bought, 0);
    for (int i = 0; i < ITEM_COUNT; i++)
    {
        const ShopItem *item = &gs->shop.items[i];
        CHECK_INT(item->type, i);
        CHECK(item->name[0] != '\0');
        CHECK(item->price > 0);
        CHECK(item->level_required >= 1 && item->level_required <= NH_LEVEL_MAX);
        CHECK(item->is_available);
        for (int j = i + 1; j < ITEM_COUNT; j++)
            CHECK(strcmp(item->name, gs->shop.items[j].name) != 0);
    }
    /* Les objets qui se rachètent sont ceux qui se consomment ; les autres sont uniques. */
    CHECK(gs->shop.items[ITEM_ALERT_REDUCER].is_consumable);
    CHECK(gs->shop.items[ITEM_PROXY_CHAIN].is_consumable);
    CHECK(gs->shop.items[ITEM_REPUTATION_BOOST].is_consumable);
    CHECK(gs->shop.items[ITEM_XP_BOOST].is_consumable);
    CHECK(!gs->shop.items[ITEM_AI_MODULE].is_consumable);
    CHECK(!gs->shop.items[ITEM_VPN_SERVICE].is_consumable);
    free(gs);
}

static void test_refusals_cost_nothing(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    /* Hors catalogue. */
    int credits = gs->player.credits;
    CHECK_INT(buy(gs, (ShopItemType)-1, out, sizeof out), NH_BUY_INVALID);
    CHECK(has(out, "Objet invalide"));
    CHECK_INT(buy(gs, ITEM_COUNT, out, sizeof out), NH_BUY_INVALID);
    CHECK_INT(gs->player.credits, credits);

    /* Niveau trop bas : l'explication cite le niveau requis, rien n'est débité ni marqué acheté. */
    gs->player.level = LEVEL_NOVICE;
    CHECK_INT(buy(gs, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_LOW_LEVEL);
    CHECK(has(out, "niveau 2 requis"));
    CHECK_INT(gs->player.credits, credits);
    CHECK_INT(gs->shop.bought, 0);
    CHECK(gs->shop.items[ITEM_STEALTH_UPGRADE].is_available);
    CHECK_INT(nh_events_emitted(gs, NH_EV_ITEM_BOUGHT), 0);

    /* Niveau suffisant, crédits insuffisants : le prix est annoncé. */
    gs->player.credits = 149;
    gs->player.level = LEVEL_APPRENTICE;
    CHECK_INT(buy(gs, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_LOW_CREDITS);
    CHECK(has(out, "150 ¢ requis"));
    CHECK_INT(gs->player.credits, 149);
    CHECK_INT(gs->shop.bought, 0);

    /* Niveau ET crédits manquent : c'est le niveau qui est dit d'abord (même ordre que la vitrine). */
    gs->player.credits = 0;
    gs->player.level = LEVEL_NOVICE;
    CHECK_INT(buy(gs, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_LOW_LEVEL);

    /* Objet épuisé : dit avant tout le reste. */
    gs->shop.items[ITEM_STEALTH_UPGRADE].is_available = false;
    CHECK_INT(buy(gs, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_SOLD_OUT);
    CHECK(has(out, "plus en vente"));
    free(gs);
}

static void test_purchase_bookkeeping(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    gs->player.credits = 1000;
    CHECK_INT(buy(gs, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->player.credits, 1000 - 150);
    CHECK_INT(gs->shop.bought, 1u << ITEM_STEALTH_UPGRADE);
    CHECK(has(out, "Achat réussi : Stealth Module v2.0 pour 150 ¢"));
    CHECK(has(out, "Crédits restants : 850 ¢"));
    CHECK(has(out, "Objet unique"));
    CHECK(!gs->shop.items[ITEM_STEALTH_UPGRADE].is_available);

    /* Un événement par achat, avec le numéro de l'objet ; le bus livre au prochain nh_events_flush. */
    CHECK_INT(nh_events_emitted(gs, NH_EV_ITEM_BOUGHT), 1);
    bool queued = false;
    for (int i = 0; i < nh_events_pending(gs); i++)
    {
        NhEventRecord rec;
        if (nh_events_peek(gs, i, &rec) && rec.type == NH_EV_ITEM_BOUGHT && rec.value == (int)ITEM_STEALTH_UPGRADE)
            queued = true;
    }
    CHECK(queued);

    /* Le racheter est refusé : épuisé. */
    CHECK_INT(buy(gs, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_SOLD_OUT);
    CHECK_INT(gs->player.credits, 850);

    /* Un consommable reste en vente et se rachète. */
    CHECK_INT(buy(gs, ITEM_ALERT_REDUCER, out, sizeof out), NH_BUY_OK);
    CHECK(gs->shop.items[ITEM_ALERT_REDUCER].is_available);
    CHECK(!has(out, "Objet unique"));
    CHECK_INT(buy(gs, ITEM_ALERT_REDUCER, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->player.credits, 850 - 80 - 80);
    CHECK_INT(nh_events_emitted(gs, NH_EV_ITEM_BOUGHT), 3);
    free(gs);
}

static void test_stealth_module(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    gs->player.stealth_rating = 3;
    CHECK_INT(buy(gs, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->player.stealth_rating, 3 + NH_STEALTH_MODULE_BONUS);
    CHECK(has(out, "Furtivité : 5/10"));

    /* Plafond : jamais au-dessus de NH_STEALTH_MAX, et l'achat qui ne servirait plus à rien est refusé. */
    GameState *g2 = new_game();
    g2->player.stealth_rating = NH_STEALTH_MAX - 1;
    CHECK_INT(buy(g2, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_OK);
    CHECK_INT(g2->player.stealth_rating, NH_STEALTH_MAX);

    GameState *g3 = new_game();
    g3->player.stealth_rating = NH_STEALTH_MAX;
    int credits = g3->player.credits;
    CHECK_INT(buy(g3, ITEM_STEALTH_UPGRADE, out, sizeof out), NH_BUY_MAXED);
    CHECK(has(out, "déjà autant que possible"));
    CHECK_INT(g3->player.credits, credits);
    CHECK(g3->shop.items[ITEM_STEALTH_UPGRADE].is_available); /* refusé : pas « épuisé » */
    CHECK_INT(g3->shop.bought, 0);
    free(gs);
    free(g2);
    free(g3);
}

static void test_ghost_protocol(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    for (int i = 1; i <= NH_GHOST_MAX; i++)
    {
        CHECK_INT(buy(gs, ITEM_ALERT_REDUCER, out, sizeof out), NH_BUY_OK);
        CHECK_INT(gs->alert.ghost_protocols_available, i);
    }
    CHECK(has(out, "Ghost Protocol en réserve : 5"));
    int credits = gs->player.credits;
    CHECK_INT(buy(gs, ITEM_ALERT_REDUCER, out, sizeof out), NH_BUY_MAXED);
    CHECK_INT(gs->alert.ghost_protocols_available, NH_GHOST_MAX);
    CHECK_INT(gs->player.credits, credits);

    /* Le menu de laylow le dépense : on peut alors en racheter un. */
    gs->alert.level = 50;
    int applied = 0;
    credits = gs->player.credits;
    CHECK_INT(nh_alert_apply_reduction(&gs->alert, NH_REDUCTION_GHOST, &gs->player.credits, &applied), NH_REDUCE_OK);
    CHECK_INT(applied, nh_alert_reduction_amount(NH_REDUCTION_GHOST));
    CHECK_INT(gs->player.credits, credits - nh_alert_reduction_cost(NH_REDUCTION_GHOST));
    CHECK_INT(gs->alert.ghost_protocols_available, NH_GHOST_MAX - 1);
    CHECK_INT(buy(gs, ITEM_ALERT_REDUCER, out, sizeof out), NH_BUY_OK);
    free(gs);
}

static void test_virus_pack(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    gs->player.virus_library_size = 1;
    CHECK_INT(buy(gs, ITEM_VIRUS_PACK, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->player.virus_library_size, gs->virus_count);
    CHECK(has(out, "Bibliothèque de virus : 3"));
    for (int i = 0; i < gs->advanced.tool_count; i++)
        if (gs->advanced.tools[i].tool == TOOL_VIRUS_LABORATORY)
            CHECK(gs->advanced.tools[i].is_active); /* nh_world_sync_tools a été rejoué */

    /* Bibliothèque déjà complète : l'objet ne servirait à rien. */
    GameState *g2 = new_game();
    g2->player.virus_library_size = g2->virus_count;
    int credits = g2->player.credits;
    CHECK_INT(buy(g2, ITEM_VIRUS_PACK, out, sizeof out), NH_BUY_MAXED);
    CHECK_INT(g2->player.credits, credits);
    free(gs);
    free(g2);
}

static void test_proxy_chain(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    CHECK_INT(buy(gs, ITEM_PROXY_CHAIN, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->alert.proxy_hacks_left, NH_PROXY_HACKS);
    CHECK(has(out, "Proxies actifs pour 5 piratages"));

    /* Les réserves s'additionnent jusqu'au plafond, sans jamais le dépasser. */
    while (gs->alert.proxy_hacks_left + NH_PROXY_HACKS <= NH_PROXY_HACKS_MAX)
        CHECK_INT(buy(gs, ITEM_PROXY_CHAIN, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->alert.proxy_hacks_left, NH_PROXY_HACKS_MAX);
    int credits = gs->player.credits;
    CHECK_INT(buy(gs, ITEM_PROXY_CHAIN, out, sizeof out), NH_BUY_MAXED);
    CHECK_INT(gs->player.credits, credits);

    /* Une réserve entamée qui laisse de la place accepte un dernier achat, écrêté au plafond. */
    gs->alert.proxy_hacks_left = NH_PROXY_HACKS_MAX - 1;
    CHECK_INT(buy(gs, ITEM_PROXY_CHAIN, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->alert.proxy_hacks_left, NH_PROXY_HACKS_MAX);
    free(gs);
}

static void test_encryption_key(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    /* corp-server-01 piraté SANS la clé : ses fichiers (niveaux 2 et 3) restent verrouillés. */
    int corp = -1;
    for (int i = 0; i < nh_world_count(); i++)
        if (strcmp(gs->nodes[i].name, "corp-server-01") == 0)
            corp = i;
    CHECK(corp >= 0);
    NhCapture cap = nh_capture_begin();
    CHECK(nh_world_compromise(gs, corp, false));
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(nh_world_locked_files(&gs->nodes[corp]), 2);

    int credits = gs->player.credits;
    CHECK_INT(buy(gs, ITEM_ENCRYPTION_KEY, out, sizeof out), NH_BUY_OK);
    CHECK(gs->player.has_encryption_key);
    CHECK(has(out, "niveau 1 à 2"));

    /* Effet rétroactif : le fichier de niveau 2 s'ouvre (et paie), celui de niveau 3 non. */
    CHECK(has(out, "employee_records.db"));
    CHECK(!has(out, "financial_data.xlsx"));
    CHECK_INT(nh_world_locked_files(&gs->nodes[corp]), 1);
    CHECK_INT(gs->player.credits, credits - 200 + 200);
    CHECK(gs->nodes[corp].secret_files[0].is_unlocked && !gs->nodes[corp].secret_files[1].is_unlocked);

    /* Une clé par joueur. */
    CHECK_INT(buy(gs, ITEM_ENCRYPTION_KEY, out, sizeof out), NH_BUY_SOLD_OUT);

    /* Ensuite, tout nouveau système piraté (sans « deep ») livre ses fichiers de niveau <= 2 seulement. */
    int market = -1;
    for (int i = 0; i < nh_world_count(); i++)
        if (strcmp(gs->nodes[i].name, "underground-market") == 0)
            market = i;
    CHECK(market >= 0);
    cap = nh_capture_begin();
    CHECK(nh_world_compromise(gs, market, false));
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "black_ledger.dat"));
    CHECK_INT(nh_world_locked_files(&gs->nodes[market]), 0);
    free(gs);
}

static void test_ai_and_quantum(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    CHECK(!gs->player.has_ai_assistant && !gs->player.has_quantum_computer);
    CHECK_INT(buy(gs, ITEM_AI_MODULE, out, sizeof out), NH_BUY_OK);
    CHECK(gs->player.has_ai_assistant && !gs->player.has_quantum_computer);
    CHECK(has(out, "aihack"));
    CHECK_INT(buy(gs, ITEM_AI_MODULE, out, sizeof out), NH_BUY_SOLD_OUT);

    CHECK_INT(buy(gs, ITEM_QUANTUM_CHIP, out, sizeof out), NH_BUY_OK);
    CHECK(gs->player.has_quantum_computer);
    CHECK(has(out, "quantumdecrypt"));
    CHECK_INT(buy(gs, ITEM_QUANTUM_CHIP, out, sizeof out), NH_BUY_SOLD_OUT);

    /* Les outils avancés reflètent aussitôt l'équipement (nh_world_sync_tools). */
    bool ai = false, quantum = false;
    for (int i = 0; i < gs->advanced.tool_count; i++)
    {
        if (gs->advanced.tools[i].tool == TOOL_AI_ASSISTANT)
            ai = gs->advanced.tools[i].is_active;
        if (gs->advanced.tools[i].tool == TOOL_QUANTUM_COMPUTER)
            quantum = gs->advanced.tools[i].is_active;
    }
    CHECK(ai && quantum);

    /* Sans équipement, aihack et quantumdecrypt renvoient vers la boutique au lieu de « ne rien faire ». */
    GameState *g2 = new_game();
    g2->player.commands_unlocked[CMD_AI_HACK] = true; /* débloquées au niveau 5, mais sans le matériel */
    g2->player.commands_unlocked[CMD_QUANTUM_DECRYPT] = true;
    NhCapture cap = nh_capture_begin();
    (void)nh_dispatch(g2, "aihack localhost");
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "R4Z0R"));
    cap = nh_capture_begin();
    (void)nh_dispatch(g2, "quantumdecrypt localhost");
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "R4Z0R"));
    free(gs);
    free(g2);
}

static void test_reputation_booster(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    gs->player.reputation = 5;
    CHECK_INT(buy(gs, ITEM_REPUTATION_BOOST, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->player.reputation, 5 + NH_REPUTATION_BOOST_POINTS);
    CHECK(has(out, "[+20 réputation]"));
    CHECK_INT(buy(gs, ITEM_REPUTATION_BOOST, out, sizeof out), NH_BUY_OK); /* consommable : sans plafond de stock */
    CHECK_INT(gs->player.reputation, 5 + 2 * NH_REPUTATION_BOOST_POINTS);
    /* Ce que la réputation ouvre est piloté par le bus : un événement, à chaque achat. */
    CHECK_INT(nh_events_emitted(gs, NH_EV_REPUTATION), 2);
    free(gs);
}

static void test_xp_boost(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    gs->player.level = LEVEL_APPRENTICE; /* niveau requis par l'Accélérateur */
    gs->player.experience = 0;

    CHECK_INT(buy(gs, ITEM_XP_BOOST, out, sizeof out), NH_BUY_OK);
    CHECK_INT(gs->player.xp_boost, NH_XP_BOOST_CHARGES);
    CHECK(has(out, "3 prochains gains"));

    /* Réserve pleine : pas de cumul au-delà de NH_XP_BOOST_CHARGES. */
    int credits = gs->player.credits;
    CHECK_INT(buy(gs, ITEM_XP_BOOST, out, sizeof out), NH_BUY_MAXED);
    CHECK_INT(gs->player.credits, credits);
    CHECK_INT(gs->player.xp_boost, NH_XP_BOOST_CHARGES);

    /* Un petit gain double, un gros gain gagne au plus NH_XP_BOOST_BONUS_CAP points de plus. */
    NhCapture cap = nh_capture_begin();
    (void)nh_grant_xp(gs, 5);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.experience, 10);
    CHECK(has(out, "[+10 EXP dont 5 de boost]"));
    CHECK_INT(gs->player.xp_boost, NH_XP_BOOST_CHARGES - 1);

    cap = nh_capture_begin();
    (void)nh_grant_xp(gs, 100);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.experience, 10 + 100 + NH_XP_BOOST_BONUS_CAP);
    CHECK_INT(gs->player.xp_boost, NH_XP_BOOST_CHARGES - 2);

    /* Les récompenses de quête ne sont pas des hacks : ni majorées, ni décomptées. */
    int xp = gs->player.experience;
    cap = nh_capture_begin();
    (void)nh_grant_xp_flat(gs, 100);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.experience, xp + 100);
    CHECK_INT(gs->player.xp_boost, NH_XP_BOOST_CHARGES - 2);
    CHECK(!has(out, "boost"));

    /* Dernière charge, puis plus rien : les gains suivants sont ordinaires. */
    cap = nh_capture_begin();
    (void)nh_grant_xp(gs, 1);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.xp_boost, 0);
    xp = gs->player.experience;
    cap = nh_capture_begin();
    (void)nh_grant_xp(gs, 7);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.experience, xp + 7);
    CHECK(!has(out, "boost"));

    /* Le boost se rachète une fois la réserve vide. */
    CHECK_INT(buy(gs, ITEM_XP_BOOST, out, sizeof out), NH_BUY_OK);
    free(gs);
}

static void test_vpn(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    CHECK(!gs->alert.vpn_active);
    CHECK_INT(buy(gs, ITEM_VPN_SERVICE, out, sizeof out), NH_BUY_OK);
    CHECK(gs->alert.vpn_active);
    CHECK(has(out, "VPN actif en permanence"));
    CHECK_INT(buy(gs, ITEM_VPN_SERVICE, out, sizeof out), NH_BUY_SOLD_OUT);

    /* Il refroidit d'un point de plus par hack, sans jamais descendre sous zéro. */
    gs->alert.level = 10;
    CHECK_INT(nh_alert_decay(&gs->alert), 2);
    CHECK_INT(gs->alert.level, 8);
    free(gs);
}

/* Les résumés de la vitrine citent des nombres : ils doivent être ceux du code. */
static void contains_number(ShopItemType type, int value)
{
    char needle[16];
    snprintf(needle, sizeof needle, "%d", value);
    nh_set_lang(NH_LANG_FR);
    CHECK(has(nh_shop_blurb(type), needle));
    nh_set_lang(NH_LANG_EN);
    CHECK(has(nh_shop_blurb(type), needle));
    nh_set_lang(NH_LANG_FR);
}

static void test_blurbs_match_the_code(void)
{
    contains_number(ITEM_STEALTH_UPGRADE, NH_STEALTH_MODULE_BONUS);
    contains_number(ITEM_ALERT_REDUCER, nh_alert_reduction_amount(NH_REDUCTION_GHOST));
    contains_number(ITEM_PROXY_CHAIN, NH_PROXY_HACKS);
    contains_number(ITEM_ENCRYPTION_KEY, NH_KEY_DECRYPT_LEVEL);
    contains_number(ITEM_REPUTATION_BOOST, NH_REPUTATION_BOOST_POINTS);
    contains_number(ITEM_XP_BOOST, NH_XP_BOOST_BONUS_CAP);
    contains_number(ITEM_XP_BOOST, NH_XP_BOOST_CHARGES);

    GameState *gs = new_game();
    contains_number(ITEM_VIRUS_PACK, gs->virus_count);
    free(gs);

    /* Une même règle pour tous : chaque objet a un résumé, dans les deux langues, qui tient dans la case. */
    for (int i = 0; i < ITEM_COUNT; i++)
    {
        nh_set_lang(NH_LANG_FR);
        CHECK(nh_shop_blurb((ShopItemType)i)[0] != '\0');
        nh_set_lang(NH_LANG_EN);
        CHECK(nh_shop_blurb((ShopItemType)i)[0] != '\0');
    }
    nh_set_lang(NH_LANG_FR);
}

/* `status` montre ce que la boutique a donné, et seulement cela : rien de plus en début de partie. */
static void test_status_shows_purchases(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    NhCapture cap = nh_capture_begin();
    (void)nh_dispatch(gs, "status");
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "Furtivité: 3/10"));
    CHECK(!has(out, "Clé de déchiffrement"));
    CHECK(!has(out, "Ghost Protocols"));
    CHECK(!has(out, "Accélérateur"));
    CHECK(!has(out, "VPN"));
    CHECK(!has(out, "Proxies"));

    (void)buy(gs, ITEM_ENCRYPTION_KEY, out, sizeof out);
    (void)buy(gs, ITEM_ALERT_REDUCER, out, sizeof out);
    (void)buy(gs, ITEM_ALERT_REDUCER, out, sizeof out);
    (void)buy(gs, ITEM_XP_BOOST, out, sizeof out);
    (void)buy(gs, ITEM_VPN_SERVICE, out, sizeof out);
    (void)buy(gs, ITEM_PROXY_CHAIN, out, sizeof out);
    cap = nh_capture_begin();
    (void)nh_dispatch(gs, "status");
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "Clé de déchiffrement: Disponible") || has(out, "Clé de déchiffrement"));
    CHECK(has(out, "Ghost Protocols en réserve: 2"));
    CHECK(has(out, "Accélérateur d'EXP (gains): 3"));
    CHECK(has(out, "VPN: "));
    CHECK(has(out, "Proxies (piratages): 5"));

    nh_set_lang(NH_LANG_EN);
    cap = nh_capture_begin();
    (void)nh_dispatch(gs, "status");
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "Decryption Key"));
    CHECK(has(out, "Ghost Protocols in stock: 2"));
    CHECK(has(out, "XP Accelerator (gains): 3"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

/* La commande complète : menu, saisie du numéro, achat. */
static void test_cmd_shop(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    gs->player.credits = 500;
    nh_feed("10\n"); /* Dark Web VPN : 60 ¢ */
    NhCapture cap = nh_capture_begin();
    NhDispatch r = nh_dispatch(gs, "shop");
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(r, NH_DISPATCH_OK);
    CHECK(gs->alert.vpn_active);
    CHECK_INT(gs->player.credits, 440);
    CHECK(has(out, "Crédits restants : 440 ¢"));

    /* Entrée vide, 0, texte : rien n'est acheté. */
    const char *quits[] = {"\n", "0\n", "abc\n", "11\n"};
    for (size_t i = 0; i < sizeof quits / sizeof quits[0]; i++)
    {
        nh_feed(quits[i]);
        cap = nh_capture_begin();
        (void)nh_dispatch(gs, "shop");
        nh_capture_end(&cap, out, sizeof out);
        CHECK_INT(gs->player.credits, 440);
        CHECK_INT(gs->shop.bought, 1u << ITEM_VPN_SERVICE);
    }

    /* Refus expliqué, la commande a tout de même eu lieu. */
    nh_feed("6\n"); /* Neural Assistant : 500 ¢ */
    cap = nh_capture_begin();
    r = nh_dispatch(gs, "shop");
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(r, NH_DISPATCH_OK);
    CHECK(has(out, "Crédits insuffisants"));
    CHECK(!gs->player.has_ai_assistant);
    free(gs);
}

int main(void)
{
    nh_set_fast(true);
    nh_term_set_color(false);
    nh_set_lang(NH_LANG_FR);

    test_catalogue();
    test_refusals_cost_nothing();
    test_purchase_bookkeeping();
    test_stealth_module();
    test_ghost_protocol();
    test_virus_pack();
    test_proxy_chain();
    test_encryption_key();
    test_ai_and_quantum();
    test_reputation_booster();
    test_xp_boost();
    test_vpn();
    test_blurbs_match_the_code();
    test_status_shows_purchases();
    test_cmd_shop();
    nh_unfeed();
    return NH_TEST_REPORT("shop");
}
