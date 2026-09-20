#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"

#include "../../src/core/platform.h"
#include "../../src/game/commands.h"
#include "../../src/game/progression.h"
#include "../../src/game/shop.h"
#include "../../src/game/world.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Invariant anti-farm (règle de la phase 3.4) : chaque source de crédits ou d'expérience est bornée
 * par l'état de la partie. Une fois tout épuisé, AUCUNE commande, répétée à volonté, ne doit encore
 * rapporter quoi que ce soit — ce test échoue dès qu'un « farm » réapparaît.
 */

#define OUT_SIZE 65536

/* Partie où tout ce que le monde offre a été pris : systèmes piratés, fichiers ouverts, jalons obtenus. */
static GameState *exhausted_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    Player *p = &gs->player;
    p->level = LEVEL_LEGEND;
    p->experience = 5000;
    p->credits = 1000;
    p->scans_done = 1000;
    p->milestones = (1u << NH_MS_COUNT) - 1u;
    p->has_ai_assistant = p->has_quantum_computer = p->has_encryption_key = true;
    p->virus_library_size = gs->virus_count;
    for (int i = 0; i < MAX_COMMANDS; i++)
        p->commands_unlocked[i] = true;
    gs->advanced.neural_interface_sync = 100;

    for (int i = 0; i < NH_MAX_NODES; i++)
    {
        NetworkNode *n = &gs->nodes[i];
        n->is_discovered = n->is_compromised = n->has_backdoor = n->has_virus = n->is_traced = n->has_intel = true;
        for (int f = 0; f < n->file_count; f++)
            n->secret_files[f].is_unlocked = true;
    }
    for (int i = 0; i < gs->shop.item_count; i++)
        gs->shop.items[i].is_available = false;
    return gs;
}

/* Une commande, entrée du clavier vide (les menus rendent la main sur fin d'entrée). */
static void run(GameState *gs, const char *line)
{
    char out[OUT_SIZE];
    nh_feed("");
    NhCapture cap = nh_capture_begin();
    (void)nh_dispatch(gs, line);
    nh_capture_end(&cap, out, sizeof out);
}

static void test_exhausted_world_pays_nothing(void)
{
    nh_set_lang(NH_LANG_FR);
    GameState *gs = exhausted_game();

    size_t count;
    const NhCommand *table = nh_commands(&count);
    const char *args[] = {"", "localhost", "corp-server-01", "nexus-mainframe", "underground-market", "research-lab",
                          "banking-network", "gov-database", "ECHO-7", "R4Z0R", "Phoenix", "AURA", "1", "2"};

    for (int seed = 1; seed <= 3; seed++)
    {
        srand((unsigned)seed);
        for (size_t c = 0; c < count; c++)
        {
            const char *name = table[c].name;
            /* Hors sujet ici : `quit` termine la partie, `save` écrit dans le vrai dossier de données. */
            if (strcmp(name, "quit") == 0 || strcmp(name, "save") == 0 || strcmp(name, "clear") == 0)
                continue;
            for (size_t a = 0; a < sizeof args / sizeof args[0]; a++)
            {
                char line[128];
                snprintf(line, sizeof line, "%s %s", name, args[a]);
                int credits = gs->player.credits;
                int xp = gs->player.experience;
                gs->alert.level = 0; /* un échec ne doit pas finir la partie en cours de test */
                run(gs, line);
                if (gs->player.credits > credits || gs->player.experience > xp)
                    fprintf(stderr, "    (farm : « %s » : crédits %d -> %d, XP %d -> %d)\n", line, credits,
                            gs->player.credits, xp, gs->player.experience);
                CHECK(gs->player.credits <= credits);
                CHECK(gs->player.experience <= xp);
            }
        }
    }
    free(gs);
}

static void test_shop_cannot_be_farmed(void)
{
    nh_set_lang(NH_LANG_FR);
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    gs->player.level = LEVEL_LEGEND;
    gs->player.credits = 100000;
    int xp = gs->player.experience;

    /* Tout racheter, cent fois : chaque achat coûte, aucun ne rembourse ni ne rapporte d'expérience. */
    char out[OUT_SIZE];
    for (int round = 0; round < 100; round++)
        for (int i = 0; i < ITEM_COUNT; i++)
        {
            int before = gs->player.credits;
            NhCapture cap = nh_capture_begin();
            NhBuyResult r = nh_shop_buy(gs, (ShopItemType)i);
            nh_capture_end(&cap, out, sizeof out);
            if (r == NH_BUY_OK)
                CHECK_INT(gs->player.credits, before - gs->shop.items[i].price);
            else
                CHECK_INT(gs->player.credits, before);
        }
    CHECK_INT(gs->player.experience, xp);
    CHECK(gs->player.credits <= 100000);
    /* Les plafonds tiennent, quel que soit le nombre d'achats. */
    CHECK(gs->player.stealth_rating <= NH_STEALTH_MAX);
    CHECK(gs->alert.ghost_protocols_available <= NH_GHOST_MAX);
    CHECK(gs->alert.proxy_hacks_left <= NH_PROXY_HACKS_MAX);
    CHECK(gs->player.xp_boost <= NH_XP_BOOST_CHARGES);
    CHECK(gs->player.virus_library_size <= gs->virus_count);
    free(gs);
}

/* Le boost d'expérience double un gain déjà borné : il ne crée pas de source. */
static void test_xp_boost_is_bounded(void)
{
    nh_set_lang(NH_LANG_FR);
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    gs->player.level = LEVEL_LEGEND;
    char out[OUT_SIZE];

    /* Un achat = au plus NH_XP_BOOST_CHARGES gains majorés de NH_XP_BOOST_BONUS_CAP points chacun. */
    gs->player.xp_boost = NH_XP_BOOST_CHARGES;
    int base = gs->player.experience;
    int granted = 0;
    NhCapture cap = nh_capture_begin();
    for (int i = 0; i < 50; i++)
    {
        (void)nh_grant_xp(gs, 1000);
        granted += 1000;
    }
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.experience - base, granted + NH_XP_BOOST_CHARGES * NH_XP_BOOST_BONUS_CAP);
    CHECK_INT(gs->player.xp_boost, 0);

    /* Même avec un montant extrême, pas de dépassement d'entier ni de plafond crevé. */
    gs->player.xp_boost = NH_XP_BOOST_CHARGES;
    cap = nh_capture_begin();
    (void)nh_grant_xp(gs, 2000000000);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.experience, NH_XP_CAP);
    free(gs);
}

int main(void)
{
    nh_set_fast(true);
    nh_term_set_color(false);
    nh_set_lang(NH_LANG_FR);

    test_exhausted_world_pays_nothing();
    test_shop_cannot_be_farmed();
    test_xp_boost_is_bounded();
    nh_unfeed();
    return NH_TEST_REPORT("economy");
}
