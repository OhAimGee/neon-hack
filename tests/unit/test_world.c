#include "nh_test.h"

#include "nh_capture.h"

#include "../../src/core/io.h"
#include "../../src/core/platform.h"
#include "../../src/game/commands.h"
#include "../../src/game/progression.h"
#include "../../src/game/world.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>

static FILE *g_fed = NULL;

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static void feed(const char *text)
{
    FILE *f = tmpfile();
    fputs(text, f);
    rewind(f);
    nh_io_set_input(f);
    if (g_fed != NULL)
        fclose(g_fed);
    g_fed = f;
}

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    feed(""); /* une lecture rencontre la fin d'entrée, jamais le vrai clavier */
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);
    return gs;
}

static NhDispatch run_line(GameState *gs, const char *line, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    NhDispatch r = nh_dispatch(gs, line);
    nh_capture_end(&cap, out, size);
    return r;
}

/* Indice d'un système par son nom, sans passer par la découverte. */
static int idx_of(const GameState *gs, const char *name)
{
    for (int i = 0; i < nh_world_count(); i++)
        if (strcmp(gs->nodes[i].name, name) == 0)
            return i;
    return -1;
}

/* Découvre tout et ouvre toutes les routes, pour les tests qui visent un système précis. */
static void know_everything(GameState *gs)
{
    for (int i = 0; i < nh_world_count(); i++)
        gs->nodes[i].is_discovered = true;
}

/* ---- Structure du graphe ------------------------------------------------------------------ */

static void test_table_integrity(void)
{
    GameState *gs = new_game();
    int n = nh_world_count();
    CHECK_INT(n, NH_MAX_NODES);
    CHECK_INT(gs->advanced.target_count, 5);

    int roots = 0;
    int adv_used[16] = {0};
    for (int i = 0; i < n; i++)
    {
        const NetworkNode *node = &gs->nodes[i];
        CHECK(node->name[0] != '\0');
        CHECK(strchr(node->name, ' ') == NULL); /* le nom est un argument de commande */
        CHECK(node->corporation[0] != '\0');
        CHECK(node->data_value > 0);
        CHECK(node->file_count >= 1 && node->file_count <= 5);
        CHECK(node->security >= SECURITY_LOW && node->security <= SECURITY_CRITICAL);
        for (int f = 0; f < node->file_count; f++)
        {
            CHECK(node->secret_files[f].filename[0] != '\0');
            CHECK(node->secret_files[f].credits_value > 0);
            CHECK(!node->secret_files[f].is_unlocked);
        }
        /* Tout est vierge au départ. */
        CHECK(!node->is_compromised && !node->has_backdoor && !node->has_virus && !node->is_traced &&
              !node->has_intel);

        /* Noms uniques. */
        for (int j = i + 1; j < n; j++)
            CHECK(strcmp(node->name, gs->nodes[j].name) != 0);

        int level = nh_world_min_level(i);
        CHECK(level >= 1 && level <= NH_LEVEL_MAX);

        int up = nh_world_uplink(i);
        CHECK(up >= -1 && up < n && up != i);
        if (up < 0)
            roots++;
        else
            CHECK(nh_world_min_level(up) <= level); /* jamais visible avant son relais */

        /* Les relais mènent tous à la racine, sans cycle. */
        int steps = 0;
        for (int cur = i; cur >= 0 && steps <= n; cur = nh_world_uplink(cur))
            steps++;
        CHECK(steps <= n);

        int adv = nh_world_adv_target(i);
        CHECK(adv >= -1 && adv < gs->advanced.target_count);
        if (adv >= 0)
            adv_used[adv]++;
    }
    CHECK_INT(roots, 1);
    CHECK_INT(nh_world_uplink(0), -1); /* la racine est localhost */
    CHECK_INT(nh_world_min_level(0), 1);

    /* Chaque cible avancée correspond à un et un seul système. */
    for (int t = 0; t < gs->advanced.target_count; t++)
        CHECK_INT(adv_used[t], 1);

    /* Hors bornes : valeurs neutres, pas de lecture hors tableau. */
    CHECK_INT(nh_world_uplink(-1), -1);
    CHECK_INT(nh_world_uplink(n), -1);
    CHECK_INT(nh_world_min_level(n), 0);
    CHECK_INT(nh_world_adv_target(-5), -1);
    CHECK(!nh_world_reachable(gs->nodes, -1));
    CHECK(!nh_world_reachable(gs->nodes, n));
    free(gs);
}

/* Les quêtes et le code de jeu nomment un système par son indice (NhNode) : il doit désigner le bon. */
static void test_node_enum_matches_table(void)
{
    static const char *const k_names[NH_NODE_COUNT] = {
        [NH_NODE_LOCALHOST] = "localhost",      [NH_NODE_CORP_SERVER] = "corp-server-01",
        [NH_NODE_NEXUS] = "nexus-mainframe",    [NH_NODE_MARKET] = "underground-market",
        [NH_NODE_LAB] = "research-lab",         [NH_NODE_BANK] = "banking-network",
        [NH_NODE_GOV] = "gov-database",
    };
    GameState *gs = new_game();
    CHECK_INT(NH_NODE_COUNT, nh_world_count());
    for (int i = 0; i < NH_NODE_COUNT; i++)
        CHECK_STR(gs->nodes[i].name, k_names[i]);
    /* et le graphe raconte la même histoire que l'énumération */
    CHECK_INT(nh_world_uplink(NH_NODE_LOCALHOST), -1);
    CHECK_INT(nh_world_uplink(NH_NODE_CORP_SERVER), NH_NODE_LOCALHOST);
    CHECK_INT(nh_world_uplink(NH_NODE_NEXUS), NH_NODE_CORP_SERVER);
    CHECK_INT(nh_world_uplink(NH_NODE_GOV), NH_NODE_NEXUS);
    free(gs);
}

static void test_init_state(void)
{
    NetworkNode nodes[NH_MAX_NODES];
    memset(nodes, 0xAB, sizeof nodes); /* du bruit : nh_world_init doit tout remettre à zéro */
    nh_world_init(nodes);
    for (int i = 0; i < nh_world_count(); i++)
    {
        CHECK(!nodes[i].is_discovered);
        CHECK(!nodes[i].is_compromised);
        CHECK(!nodes[i].has_intel);
    }
    CHECK_STR(nodes[0].name, "localhost");

    /* En jeu, seul le poste du joueur est connu d'emblée. */
    GameState *gs = new_game();
    CHECK(gs->nodes[0].is_discovered);
    int known = 0;
    for (int i = 0; i < nh_world_count(); i++)
        known += gs->nodes[i].is_discovered ? 1 : 0;
    CHECK_INT(known, 1);
    free(gs);
}

static void test_discover(void)
{
    NetworkNode nodes[NH_MAX_NODES];
    nh_world_init(nodes);
    int total = 0;
    for (int level = 1; level <= NH_LEVEL_MAX; level++)
    {
        int fresh = nh_world_discover(nodes, level);
        total += fresh;
        int known = 0;
        for (int i = 0; i < nh_world_count(); i++)
        {
            known += nodes[i].is_discovered ? 1 : 0;
            /* Découvert si et seulement si le niveau suffit. */
            CHECK_INT(nodes[i].is_discovered ? 1 : 0, level >= nh_world_min_level(i) ? 1 : 0);
        }
        CHECK_INT(known, total);
        CHECK_INT(nh_world_discover(nodes, level), 0); /* idempotent */
    }
    CHECK_INT(total, nh_world_count()); /* au niveau maximal, tout est visible */

    /* Niveau trop bas ou absurde : rien de nouveau, rien ne se perd. */
    CHECK_INT(nh_world_discover(nodes, 0), 0);
    CHECK_INT(nh_world_discover(nodes, -7), 0);
    CHECK(nodes[0].is_discovered);

    NetworkNode fresh[NH_MAX_NODES];
    nh_world_init(fresh);
    CHECK_INT(nh_world_discover(fresh, 0), 0);
    CHECK(!fresh[0].is_discovered);
}

static void test_find_and_reachable(void)
{
    GameState *gs = new_game();
    CHECK_INT(nh_world_find(gs->nodes, "localhost"), 0);
    CHECK_INT(nh_world_find(gs->nodes, "corp-server-01"), -1); /* pas encore découvert */
    CHECK_INT(nh_world_find(gs->nodes, "inconnu"), -1);
    CHECK_INT(nh_world_find(gs->nodes, ""), -1);
    CHECK_INT(nh_world_find(gs->nodes, "LOCALHOST"), -1); /* nom exact */
    CHECK_INT(nh_world_find(gs->nodes, "localhos"), -1);
    CHECK_INT(nh_world_find(gs->nodes, "localhost "), -1);

    know_everything(gs);
    int corp = idx_of(gs, "corp-server-01");
    int nexus = idx_of(gs, "nexus-mainframe");
    CHECK_INT(nh_world_find(gs->nodes, "corp-server-01"), corp);

    CHECK(nh_world_reachable(gs->nodes, 0)); /* la racine n'a pas de relais */
    CHECK(!nh_world_reachable(gs->nodes, corp));
    CHECK(!nh_world_reachable(gs->nodes, nexus));
    gs->nodes[0].is_compromised = true;
    CHECK(nh_world_reachable(gs->nodes, corp));
    CHECK(!nh_world_reachable(gs->nodes, nexus)); /* le relais de nexus est corp, pas localhost */
    gs->nodes[corp].is_compromised = true;
    CHECK(nh_world_reachable(gs->nodes, nexus));
    free(gs);
}

static void test_resolve_messages(void)
{
    GameState *gs = new_game();
    char out[4096];
    nh_set_lang(NH_LANG_FR);

    NhCapture cap = nh_capture_begin();
    int r = nh_world_resolve(gs, "corp-server-01", true);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(r, -1);
    CHECK(has(out, "introuvable"));
    CHECK(has(out, "corp-server-01"));

    gs->nodes[1].is_discovered = true;
    cap = nh_capture_begin();
    r = nh_world_resolve(gs, "corp-server-01", true);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(r, -1);
    CHECK(has(out, "Route fermée"));
    CHECK(has(out, "localhost")); /* on dit quel relais compromettre */

    cap = nh_capture_begin();
    r = nh_world_resolve(gs, "corp-server-01", false); /* une méthode qui perce la route */
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(r, 1);
    CHECK_STR(out, "");

    gs->nodes[0].is_compromised = true;
    CHECK_INT(nh_world_resolve(gs, "corp-server-01", true), 1);

    nh_set_lang(NH_LANG_EN);
    gs->nodes[0].is_compromised = false;
    cap = nh_capture_begin();
    nh_world_resolve(gs, "corp-server-01", true);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "No route"));
    cap = nh_capture_begin();
    nh_world_resolve(gs, "nope", true);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "not found"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

/* ---- Chances de succès -------------------------------------------------------------------- */

static NetworkNode node_with(SecurityLevel sec)
{
    NetworkNode n;
    memset(&n, 0, sizeof n);
    n.security = sec;
    return n;
}

static void test_chance_values(void)
{
    NetworkNode low = node_with(SECURITY_LOW), med = node_with(SECURITY_MEDIUM);
    NetworkNode high = node_with(SECURITY_HIGH), crit = node_with(SECURITY_CRITICAL);

    /* Valeurs exactes des formules d'origine : base + niveau×a − sécurité×b (− malus critique). */
    CHECK_INT(nh_world_chance(NH_HACK_BRUTE, &low, 1, 0), 65);
    CHECK_INT(nh_world_chance(NH_HACK_BRUTE, &med, 1, 0), 50);
    CHECK_INT(nh_world_chance(NH_HACK_BRUTE, &high, 1, 0), 35);
    CHECK_INT(nh_world_chance(NH_HACK_BRUTE, &crit, 1, 0), 20);
    CHECK_INT(nh_world_chance(NH_HACK_BRUTE, &med, 6, 0), 50); /* la force brute ignore le niveau */

    CHECK_INT(nh_world_chance(NH_HACK_BACKDOOR, &low, 1, 0), 65);
    CHECK_INT(nh_world_chance(NH_HACK_BACKDOOR, &med, 3, 0), 70);
    CHECK_INT(nh_world_chance(NH_HACK_BACKDOOR, &crit, 6, 0), 70);
    CHECK_INT(nh_world_chance(NH_HACK_BACKDOOR, &high, 2, 20), 65); /* +20 : mode furtif */

    CHECK_INT(nh_world_chance(NH_HACK_VIRUS, &low, 1, 0), 50);
    CHECK_INT(nh_world_chance(NH_HACK_VIRUS, &high, 1, 0), 30);
    CHECK_INT(nh_world_chance(NH_HACK_VIRUS, &high, 1, 5 + 30), 65); /* discrétion + backdoor */

    CHECK_INT(nh_world_chance(NH_HACK_AI, &low, 5, 0), 95); /* 85+25 : plafonné */
    CHECK_INT(nh_world_chance(NH_HACK_AI, &high, 5, 0), 95);
    CHECK_INT(nh_world_chance(NH_HACK_AI, &crit, 5, 0), 90); /* 85+25−20 */
    CHECK_INT(nh_world_chance(NH_HACK_AI, &crit, 1, 0), 70);

    CHECK_INT(nh_world_chance(NH_HACK_EXPLOIT, &med, 4, 0), 56); /* 60+20−24 */
    CHECK_INT(nh_world_chance(NH_HACK_EXPLOIT, &high, 4, 0), 44);
}

static void test_chance_bounds_and_intel(void)
{
    NetworkNode node = node_with(SECURITY_HIGH);
    for (int m = 0; m < NH_HACK_METHOD_COUNT; m++)
    {
        CHECK_INT(nh_world_chance((NhHackMethod)m, &node, 1, 1000), NH_CHANCE_MAX);
        CHECK_INT(nh_world_chance((NhHackMethod)m, &node, 1, -1000), NH_CHANCE_MIN);
        CHECK_INT(nh_world_chance((NhHackMethod)m, &node, 6, 1000000), NH_CHANCE_MAX);
    }
    CHECK_INT(nh_world_chance(NH_HACK_METHOD_COUNT, &node, 1, 0), NH_CHANCE_MIN);
    CHECK_INT(nh_world_chance((NhHackMethod)-1, &node, 1, 0), NH_CHANCE_MIN);
    CHECK(NH_CHANCE_MIN > 0 && NH_CHANCE_MAX < 100); /* ni impossible ni garanti */

    /* Des accès internes ajoutent NH_INTEL_BONUS, une fois, à toutes les méthodes. */
    CHECK_INT(nh_world_node_bonus(&node), 0);
    int before[NH_HACK_METHOD_COUNT];
    for (int m = 0; m < NH_HACK_METHOD_COUNT; m++)
        before[m] = nh_world_chance((NhHackMethod)m, &node, 1, 0);
    node.has_intel = true;
    CHECK_INT(nh_world_node_bonus(&node), NH_INTEL_BONUS);
    for (int m = 0; m < NH_HACK_METHOD_COUNT; m++)
    {
        int after = nh_world_chance((NhHackMethod)m, &node, 1, 0);
        CHECK_INT(after, before[m] + NH_INTEL_BONUS > NH_CHANCE_MAX ? NH_CHANCE_MAX : before[m] + NH_INTEL_BONUS);
    }
}

static void test_chance_monotonic(void)
{
    /* Plus de sécurité : jamais plus facile. Plus de niveau ou de bonus : jamais plus dur. */
    for (int m = 0; m < NH_HACK_METHOD_COUNT; m++)
    {
        for (int level = 1; level <= NH_LEVEL_MAX; level++)
        {
            int prev = 101;
            for (int sec = SECURITY_LOW; sec <= SECURITY_CRITICAL; sec++)
            {
                NetworkNode n = node_with((SecurityLevel)sec);
                int c = nh_world_chance((NhHackMethod)m, &n, level, 0);
                CHECK(c <= prev);
                CHECK(c >= NH_CHANCE_MIN && c <= NH_CHANCE_MAX);
                prev = c;
            }
        }
        NetworkNode n = node_with(SECURITY_MEDIUM);
        int prev_level = -1;
        for (int level = 1; level <= NH_LEVEL_MAX; level++)
        {
            int c = nh_world_chance((NhHackMethod)m, &n, level, 0);
            CHECK(c >= prev_level);
            prev_level = c;
        }
        int prev_bonus = -1;
        for (int bonus = -60; bonus <= 60; bonus += 5)
        {
            int c = nh_world_chance((NhHackMethod)m, &n, 3, bonus);
            CHECK(c >= prev_bonus);
            prev_bonus = c;
        }
    }
}

static void test_social_chance(void)
{
    GameState *gs = new_game();
    const AdvancedTarget *nexus = &gs->advanced.targets[nh_world_adv_target(idx_of(gs, "nexus-mainframe"))];
    const AdvancedTarget *market = &gs->advanced.targets[nh_world_adv_target(idx_of(gs, "underground-market"))];
    gs->player.level = 3;
    gs->player.reputation = 0;
    /* 50 + 15 − rating/2 : possible même contre les plus gros (avant : toujours impossible). */
    CHECK_INT(social_engineering_chance(&gs->player, market), 50 + 15 - market->security_rating / 2);
    CHECK_INT(social_engineering_chance(&gs->player, nexus), 50 + 15 - nexus->security_rating / 2);
    CHECK(social_engineering_chance(&gs->player, nexus) > NH_CHANCE_MIN);
    CHECK(social_engineering_chance(&gs->player, market) > social_engineering_chance(&gs->player, nexus));
    gs->player.level = 6;
    CHECK(social_engineering_chance(&gs->player, nexus) > 50 + 15 - nexus->security_rating / 2);
    gs->player.reputation = 1000;
    CHECK_INT(social_engineering_chance(&gs->player, market), NH_CHANCE_MAX);
    gs->player.reputation = -1000;
    CHECK_INT(social_engineering_chance(&gs->player, market), NH_CHANCE_MIN);
    free(gs);
}

/* ---- Récompense unique -------------------------------------------------------------------- */

static void test_compromise_pays_once(void)
{
    GameState *gs = new_game();
    char out[4096];
    int corp = idx_of(gs, "corp-server-01");
    int credits = gs->player.credits;
    int xp = gs->player.experience;
    int data = gs->nodes[corp].data_value;

    NhCapture cap = nh_capture_begin();
    bool first = nh_world_compromise(gs, corp, false);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(first);
    CHECK(gs->nodes[corp].is_compromised);
    CHECK_INT(gs->player.credits, credits + data); /* le prix annoncé est bien versé */
    CHECK(gs->player.experience > xp);
    CHECK(has(out, "Accès obtenu à corp-server-01"));
    CHECK(has(out, "Données récupérées"));
    /* Sans « deep », les fichiers restent verrouillés. */
    CHECK_INT(nh_world_locked_files(&gs->nodes[corp]), gs->nodes[corp].file_count);

    /* Re-compromettre ne rapporte plus rien du tout. */
    credits = gs->player.credits;
    xp = gs->player.experience;
    for (int i = 0; i < 25; i++)
    {
        cap = nh_capture_begin();
        CHECK(!nh_world_compromise(gs, corp, true));
        CHECK(!nh_world_compromise(gs, corp, false));
        nh_capture_end(&cap, out, sizeof out);
        CHECK_STR(out, "");
    }
    CHECK_INT(gs->player.credits, credits);
    CHECK_INT(gs->player.experience, xp);
    CHECK_INT(nh_world_locked_files(&gs->nodes[corp]), gs->nodes[corp].file_count);

    /* Indices invalides : refus propre. */
    CHECK(!nh_world_compromise(gs, -1, false));
    CHECK(!nh_world_compromise(gs, nh_world_count(), false));
    CHECK_INT(nh_world_extract(gs, -1), 0);
    CHECK_INT(nh_world_extract(gs, nh_world_count()), 0);
    free(gs);
}

static void test_deep_and_extract(void)
{
    GameState *gs = new_game();
    char out[4096];
    int nexus = idx_of(gs, "nexus-mainframe");
    NetworkNode *node = &gs->nodes[nexus];
    int files_total = 0;
    for (int f = 0; f < node->file_count; f++)
        files_total += node->secret_files[f].credits_value;
    int credits = gs->player.credits;

    NhCapture cap = nh_capture_begin();
    CHECK(nh_world_compromise(gs, nexus, true));
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.credits, credits + node->data_value + files_total);
    CHECK_INT(nh_world_locked_files(node), 0);
    CHECK(has(out, "project_ghost.dat"));

    /* Extraire de nouveau ne repaie rien. */
    credits = gs->player.credits;
    cap = nh_capture_begin();
    CHECK_INT(nh_world_extract(gs, nexus), 0);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.credits, credits);
    CHECK_STR(out, "");

    /* Compromis « en surface » puis extraction : les fichiers se paient une fois, ensuite plus. */
    int corp = idx_of(gs, "corp-server-01");
    cap = nh_capture_begin();
    nh_world_compromise(gs, corp, false);
    nh_capture_end(&cap, out, sizeof out);
    credits = gs->player.credits;
    int expected = 0;
    for (int f = 0; f < gs->nodes[corp].file_count; f++)
        expected += gs->nodes[corp].secret_files[f].credits_value;
    cap = nh_capture_begin();
    CHECK_INT(nh_world_extract(gs, corp), gs->nodes[corp].file_count);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.credits, credits + expected);
    CHECK_INT(nh_world_extract(gs, corp), 0);
    CHECK_INT(gs->player.credits, credits + expected);
    free(gs);
}

/* ---- Les commandes sur le monde unifié ---------------------------------------------------- */

/* Répète `cmd` (alerte remise à zéro à chaque essai) jusqu'à ce que `done` soit vrai. */
static bool repeat_until(GameState *gs, const char *cmd, const char *input, bool (*done)(const GameState *))
{
    char out[32768];
    for (int i = 0; i < 300; i++)
    {
        if (done(gs))
            return true;
        gs->alert.level = 0;
        if (input != NULL)
            feed(input);
        run_line(gs, cmd, out, sizeof out);
    }
    return done(gs);
}

static bool corp_compromised(const GameState *gs) { return gs->nodes[1].is_compromised; }
static bool local_compromised(const GameState *gs) { return gs->nodes[0].is_compromised; }

static void test_bruteforce_respects_graph(void)
{
    GameState *gs = new_game();
    char out[8192];
    gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;
    srand(11);

    run_line(gs, "bruteforce corp-server-01", out, sizeof out);
    CHECK(has(out, "introuvable")); /* pas découvert */
    CHECK(!gs->nodes[1].is_compromised);

    gs->nodes[1].is_discovered = true;
    run_line(gs, "bruteforce corp-server-01", out, sizeof out);
    CHECK(has(out, "Route fermée")); /* découvert mais relais non compromis */
    CHECK(!gs->nodes[1].is_compromised);
    CHECK(!has(out, "ATTAQUE BRUTE FORCE"));

    /* Compromettre localhost ouvre la route. */
    CHECK(repeat_until(gs, "bruteforce localhost", NULL, local_compromised));
    CHECK(repeat_until(gs, "bruteforce corp-server-01", NULL, corp_compromised));

    /* Un système compromis ne se re-pirate pas : ni crédits ni expérience. */
    int credits = gs->player.credits, xp = gs->player.experience;
    for (int i = 0; i < 10; i++)
    {
        run_line(gs, "bruteforce corp-server-01", out, sizeof out);
        CHECK(has(out, "déjà compromis"));
    }
    CHECK_INT(gs->player.credits, credits);
    CHECK_INT(gs->player.experience, xp);
    free(gs);
}

static void test_backdoor_and_virus_need_route(void)
{
    GameState *gs = new_game();
    char out[8192];
    gs->player.commands_unlocked[CMD_BACKDOOR] = true;
    gs->player.commands_unlocked[CMD_UPLOAD_VIRUS] = true;
    gs->player.virus_library_size = 1;
    know_everything(gs);

    run_line(gs, "backdoor nexus-mainframe", out, sizeof out);
    CHECK(has(out, "Route fermée"));
    CHECK(!gs->nodes[2].has_backdoor);
    CHECK_INT(gs->player.backdoors_active, 0);
    run_line(gs, "uploadvirus nexus-mainframe", out, sizeof out);
    CHECK(has(out, "Route fermée"));
    CHECK(!gs->nodes[2].has_virus);

    /* Un système inconnu : introuvable. */
    run_line(gs, "backdoor fantome", out, sizeof out);
    CHECK(has(out, "introuvable"));
    free(gs);
}

static void test_traceroute_needs_no_route(void)
{
    GameState *gs = new_game();
    char out[8192];
    gs->player.commands_unlocked[CMD_TRACE_ROUTE] = true;
    know_everything(gs);

    /* Passif : trace même un système dont la route est fermée, et nomme son relais. */
    run_line(gs, "traceroute nexus-mainframe", out, sizeof out);
    CHECK(gs->nodes[2].is_traced);
    CHECK(has(out, "Relais: corp-server-01"));
    run_line(gs, "traceroute localhost", out, sizeof out);
    CHECK(!has(out, "Relais:")); /* la racine n'en a pas */
    free(gs);
}

static void test_exploit(void)
{
    GameState *gs = new_game();
    char out[8192];
    gs->player.commands_unlocked[CMD_EXPLOIT] = true;
    gs->player.commands_unlocked[CMD_TRACE_ROUTE] = true;
    gs->player.level = LEVEL_EXPERT;
    know_everything(gs);
    srand(5);

    run_line(gs, "exploit", out, sizeof out);
    CHECK(has(out, "Usage : exploit"));
    run_line(gs, "exploit fantome", out, sizeof out);
    CHECK(has(out, "introuvable"));

    /* Sans faille connue (pas de traceroute) : refus, sans effet. */
    int alert = gs->alert.level;
    run_line(gs, "exploit corp-server-01", out, sizeof out);
    CHECK(has(out, "Aucune faille connue"));
    CHECK(has(out, "traceroute corp-server-01"));
    CHECK(!gs->nodes[1].is_compromised);
    CHECK_INT(gs->alert.level, alert);

    /* Après traceroute, l'exploit perce SANS que le relais (localhost) soit compromis. */
    run_line(gs, "traceroute corp-server-01", out, sizeof out);
    CHECK(gs->nodes[1].is_traced);
    CHECK(!gs->nodes[0].is_compromised);
    CHECK(repeat_until(gs, "exploit corp-server-01", NULL, corp_compromised));
    CHECK(!gs->nodes[0].is_compromised);
    CHECK_INT(nh_world_locked_files(&gs->nodes[1]), gs->nodes[1].file_count); /* pas « deep » */

    /* Une fois compromis : plus de récompense. */
    int credits = gs->player.credits, xp = gs->player.experience;
    run_line(gs, "exploit corp-server-01", out, sizeof out);
    CHECK(has(out, "déjà compromis"));
    CHECK_INT(gs->player.credits, credits);
    CHECK_INT(gs->player.experience, xp);

    /* Un échec coûte de l'alerte ; un succès aussi. */
    int nexus = 2;
    run_line(gs, "traceroute nexus-mainframe", out, sizeof out);
    CHECK(gs->nodes[nexus].is_traced);
    gs->alert.level = 0;
    gs->alert.vpn_active = gs->alert.proxy_active = false;
    int before = gs->alert.level;
    run_line(gs, "exploit nexus-mainframe", out, sizeof out);
    CHECK(gs->alert.level > before);
    free(gs);
}

static void test_exploit_is_locked_until_level_4(void)
{
    GameState *gs = new_game();
    char out[8192];
    CHECK_INT(run_line(gs, "exploit localhost", out, sizeof out), NH_DISPATCH_LOCKED);
    gs->player.commands_unlocked[CMD_EXPLOIT] = true;
    CHECK(nh_command_available(gs, nh_find_command("exploit")));
    free(gs);
}

static bool nexus_compromised(const GameState *gs) { return gs->nodes[2].is_compromised; }
static bool corp_files_out(const GameState *gs) { return nh_world_locked_files(&gs->nodes[1]) == 0; }

static void test_aihack_extracts_once(void)
{
    GameState *gs = new_game();
    char out[8192];
    gs->player.commands_unlocked[CMD_AI_HACK] = true;
    gs->player.has_ai_assistant = true;
    gs->player.level = LEVEL_LEGEND;
    gs->player.experience = nh_level_xp_required(NH_LEVEL_MAX);
    know_everything(gs);
    gs->nodes[0].is_compromised = true; /* route ouverte vers corp-server-01 */
    gs->nodes[1].is_compromised = true; /* compromis en surface : fichiers verrouillés */
    srand(3);

    int credits = gs->player.credits;
    int expected = 0;
    for (int f = 0; f < gs->nodes[1].file_count; f++)
        expected += gs->nodes[1].secret_files[f].credits_value;
    CHECK(repeat_until(gs, "aihack corp-server-01", NULL, corp_files_out));
    CHECK_INT(gs->player.credits, credits + expected); /* extraction payée exactement une fois */
    run_line(gs, "aihack corp-server-01", out, sizeof out);
    CHECK(has(out, "Aucun fichier à extraire"));
    CHECK_INT(gs->player.credits, credits + expected);

    /* Sur un système sain, l'IA compromet ET extrait tout d'un coup. */
    gs->alert.level = 0;
    int nexus_total = gs->nodes[2].data_value;
    for (int f = 0; f < gs->nodes[2].file_count; f++)
        nexus_total += gs->nodes[2].secret_files[f].credits_value;
    credits = gs->player.credits;
    CHECK(repeat_until(gs, "aihack nexus-mainframe", NULL, nexus_compromised));
    CHECK_INT(gs->player.credits, credits + nexus_total);
    CHECK_INT(nh_world_locked_files(&gs->nodes[2]), 0);
    free(gs);
}

static bool market_compromised(const GameState *gs) { return gs->nodes[3].is_compromised; }

static void test_advhack_on_world_nodes(void)
{
    GameState *gs = new_game();
    char out[32768];
    gs->player.level = LEVEL_HACKER; /* advhack exige le niveau 3 */
    gs->player.experience = nh_level_xp_required(3);
    know_everything(gs);
    srand(9);

    /* Sans argument : liste les cibles avancées (par leur nom de système). */
    feed("");
    run_line(gs, "advhack", out, sizeof out);
    CHECK(has(out, "nexus-mainframe"));
    CHECK(has(out, "underground-market"));
    CHECK(!has(out, "Nexus Corp MainFrame Corporate")); /* pas de doublon de noms d'affichage collés */

    /* Un système classique sans profil avancé : refus expliqué. */
    run_line(gs, "advhack localhost", out, sizeof out);
    CHECK(has(out, "pas de profil de sécurité avancé"));
    /* Le nom d'affichage d'origine ne fonctionne plus : un seul vocabulaire. */
    run_line(gs, "advhack Nexus Corp MainFrame", out, sizeof out);
    CHECK(has(out, "introuvable"));
    /* Un système avancé dont la route est fermée. */
    run_line(gs, "advhack nexus-mainframe", out, sizeof out);
    CHECK(has(out, "Route fermée"));

    /* Méthode invalide : rien ne se passe. */
    gs->nodes[0].is_compromised = true;
    feed("9\n");
    run_line(gs, "advhack underground-market", out, sizeof out);
    CHECK(has(out, "Méthode invalide"));
    CHECK(!gs->nodes[3].is_compromised);

    /* Sans l'outil requis (ici l'ordinateur quantique), la méthode 1 est refusée, sans effet. */
    gs->player.has_quantum_computer = false;
    feed("1\n");
    run_line(gs, "advhack underground-market", out, sizeof out);
    CHECK(has(out, "requis mais non actif"));
    CHECK(!gs->nodes[3].is_compromised);

    /* Méthode 1 (calcul quantique) : finit par réussir ; la récompense est celle du nœud, une fois. */
    gs->player.has_quantum_computer = true;
    int credits = gs->player.credits;
    int data = gs->nodes[3].data_value;
    int files = 0;
    for (int f = 0; f < gs->nodes[3].file_count; f++)
        files += gs->nodes[3].secret_files[f].credits_value;
    CHECK(repeat_until(gs, "advhack underground-market", "1\n", market_compromised));
    /* Les échecs précédents n'ont rien payé ; le succès paie data_value + fichiers, pas plus. */
    CHECK_INT(gs->player.credits, credits + data + files);
    CHECK_INT(nh_world_locked_files(&gs->nodes[3]), 0);
    CHECK_INT(gs->advanced.tools[0].battery_life, 100 - gs->advanced.methods[0].energy_cost);

    /* Re-pirater ne rapporte rien : le vieux farm (data_value à chaque succès) est fermé. */
    credits = gs->player.credits;
    int xp = gs->player.experience;
    for (int i = 0; i < 15; i++)
    {
        feed("1\n");
        run_line(gs, "advhack underground-market", out, sizeof out);
        CHECK(has(out, "déjà compromis"));
    }
    CHECK_INT(gs->player.credits, credits);
    CHECK_INT(gs->player.experience, xp);
    free(gs);
}


static void test_tools_follow_player_state(void)
{
    GameState *gs = new_game();
    know_everything(gs);
    AdvancedHackingSystem *adv = &gs->advanced;
    CHECK_INT(adv->tool_count, 8);

    /* Départ : rien de possédé, tout est inactif. */
    nh_world_sync_tools(gs);
    for (int i = 0; i < adv->tool_count; i++)
        CHECK(!adv->tools[i].is_active);

#define ACTIVE(t) (adv->tools[(t)].is_active)
    gs->player.has_quantum_computer = true;
    nh_world_sync_tools(gs);
    CHECK(ACTIVE(TOOL_QUANTUM_COMPUTER));
    CHECK(!ACTIVE(TOOL_AI_ASSISTANT));

    gs->player.has_ai_assistant = true;
    adv->neural_interface_sync = 49;
    nh_world_sync_tools(gs);
    CHECK(ACTIVE(TOOL_AI_ASSISTANT));
    CHECK(!ACTIVE(TOOL_NEURAL_INTERFACE));
    adv->neural_interface_sync = 50;
    nh_world_sync_tools(gs);
    CHECK(ACTIVE(TOOL_NEURAL_INTERFACE));

    gs->stealth_mode = true;
    gs->player.virus_library_size = 1;
    gs->alert.ghost_protocols_available = 1;
    gs->player.commands_unlocked[CMD_EXPLOIT] = true;
    nh_world_sync_tools(gs);
    CHECK(ACTIVE(TOOL_STEALTH_CLOAK));
    CHECK(ACTIVE(TOOL_VIRUS_LABORATORY));
    CHECK(ACTIVE(TOOL_GHOST_PROTOCOL));
    CHECK(ACTIVE(TOOL_ZERO_DAY_EXPLOIT));
    CHECK(!ACTIVE(TOOL_SOCIAL_PROFILE_DB));
    gs->nodes[4].has_intel = true;
    nh_world_sync_tools(gs);
    CHECK(ACTIVE(TOOL_SOCIAL_PROFILE_DB));

    /* Les outils se DÉSACTIVENT quand la condition disparaît (état dérivé, pas un interrupteur). */
    gs->player.has_quantum_computer = false;
    gs->stealth_mode = false;
    gs->alert.ghost_protocols_available = 0;
    nh_world_sync_tools(gs);
    CHECK(!ACTIVE(TOOL_QUANTUM_COMPUTER));
    CHECK(!ACTIVE(TOOL_STEALTH_CLOAK));
    CHECK(!ACTIVE(TOOL_GHOST_PROTOCOL));
    CHECK(ACTIVE(TOOL_AI_ASSISTANT));
#undef ACTIVE
    free(gs);
}

static void test_advhack_menu_numbers_match_methods(void)
{
    GameState *gs = new_game();
    char out[32768];
    gs->player.level = LEVEL_HACKER;
    gs->player.has_quantum_computer = true;
    gs->player.has_ai_assistant = true;
    know_everything(gs);
    gs->nodes[0].is_compromised = true;
    srand(4);

    /* Le menu numérote les méthodes dans l'ordre de la table : le numéro tapé désigne la même. */
    for (int m = 0; m < 2; m++)
    {
        gs->alert.level = 0;
        char in[16];
        snprintf(in, sizeof in, "%d\n", m + 1);
        feed(in);
        run_line(gs, "advhack underground-market", out, sizeof out);
        CHECK(has(out, gs->advanced.methods[m].description));
        CHECK(has(out, "Méthode:"));
        for (int other = 0; other < 2; other++)
            if (other != m)
            {
                /* la description de l'autre méthode n'apparaît que dans le menu, pas après « Méthode: » */
                const char *after = strstr(out, "Méthode:");
                CHECK(after != NULL && strstr(after, gs->advanced.methods[other].description) == NULL);
            }
        gs->nodes[3].is_compromised = false; /* on rejoue sur la même cible */
        gs->player.credits = 100;
    }

    /* Le menu indique l'état des outils. */
    gs->player.has_ai_assistant = false;
    feed("0\n");
    run_line(gs, "advhack underground-market", out, sizeof out);
    CHECK(has(out, "[ACTIF]"));
    CHECK(has(out, "[INACTIF]"));
    free(gs);
}

/* Taux affiché par « advhack » (« Taux de succès calculé: 73% »), -1 s'il n'y en a pas. */
static int printed_rate(const char *out)
{
    static const char key[] = "Taux de succès calculé: ";
    const char *p = strstr(out, key);
    return p != NULL ? atoi(p + sizeof key - 1) : -1;
}

static int advhack_rate(GameState *gs, int alert_before)
{
    char out[32768];
    gs->alert.level = alert_before;
    gs->nodes[3].is_compromised = false; /* un succès précédent l'a compromis : on rejoue */
    gs->player.level = LEVEL_HACKER;     /* et son expérience entre dans la formule : on la fige */
    gs->player.experience = nh_level_xp_required(3);
    gs->advanced.tools[TOOL_GHOST_PROTOCOL].battery_life = 100;
    AdvancedTarget *t = &gs->advanced.targets[nh_world_adv_target(3)];
    for (int d = 0; d < t->defense_count; d++)
        t->defenses[d].attack_count = 0; /* les défenses apprennent de chaque tentative : on les remet à zéro */
    t->security_rating = 55;   /* une intrusion détectée renforce la sécurité de la cible : on la remet */
    gs->advanced.player_hacking_level = 1;
    feed("8\n"); /* protocole fantôme : 50 % de base, loin des bornes 1-99 */
    run_line(gs, "advhack underground-market", out, sizeof out);
    return printed_rate(out);
}

static void test_advhack_chance_modifiers(void)
{
    GameState *gs = new_game();
    gs->player.level = LEVEL_HACKER;
    gs->alert.ghost_protocols_available = 1;
    know_everything(gs);
    gs->nodes[0].is_compromised = true;
    srand(6);

    /* Même modèle que les méthodes classiques : l'alerte pénalise (le temps passe d'abord : -1). */
    int calm = advhack_rate(gs, 0);
    CHECK(calm > 30 && calm < 90); /* loin des bornes : les écarts ci-dessous sont lisibles */
    CHECK_INT(advhack_rate(gs, 31), calm - 5);  /* 30 après refroidissement : -5 */
    int warm = advhack_rate(gs, 51);            /* 50 : -10 */
    int hot = advhack_rate(gs, 81);             /* 80 : -20 */
    CHECK_INT(calm - warm, 10);
    CHECK_INT(calm - hot, 20);
    CHECK_INT(advhack_rate(gs, 31), calm - 5);

    /* Des accès internes ajoutent NH_INTEL_BONUS. */
    gs->nodes[3].has_intel = true;
    CHECK_INT(advhack_rate(gs, 0), calm + NH_INTEL_BONUS);
    free(gs);
}

static bool market_intel(const GameState *gs) { return gs->nodes[3].has_intel; }

static void test_socialeng_gives_intel_once(void)
{
    GameState *gs = new_game();
    char out[8192];
    gs->player.level = LEVEL_APPRENTICE; /* socialeng exige le niveau 2 */
    gs->player.experience = nh_level_xp_required(2);
    know_everything(gs);
    gs->nodes[0].is_compromised = true;
    srand(21);

    /* Un système sans profil avancé : refus. */
    run_line(gs, "socialeng localhost", out, sizeof out);
    CHECK(has(out, "pas de profil de sécurité avancé"));
    /* Route fermée : refus. */
    run_line(gs, "socialeng nexus-mainframe", out, sizeof out);
    CHECK(has(out, "Route fermée"));

    int xp = gs->player.experience;
    CHECK(repeat_until(gs, "socialeng underground-market", NULL, market_intel));
    CHECK_INT(gs->player.experience, xp + 20); /* 20 d'expérience, une fois (les échecs : 0) */
    CHECK(!gs->nodes[3].is_compromised); /* c'est un accès, pas une compromission */

    /* Déjà obtenu : pas de nouveau tirage, pas de nouvelle expérience. */
    xp = gs->player.experience;
    int alert = gs->alert.level;
    for (int i = 0; i < 10; i++)
    {
        run_line(gs, "socialeng underground-market", out, sizeof out);
        CHECK(has(out, "déjà des accès internes"));
    }
    CHECK_INT(gs->player.experience, xp);
    CHECK(gs->alert.level <= alert); /* aucune alerte ajoutée (seul le refroidissement joue) */
    free(gs);
}

static void test_temporalhack_needs_no_route_but_pays_once(void)
{
    GameState *gs = new_game();
    char out[8192];
    gs->player.level = LEVEL_LEGEND;
    gs->player.experience = nh_level_xp_required(NH_LEVEL_MAX);
    know_everything(gs);
    gs->advanced.neural_interface_sync = 100;
    gs->advanced.quantum.processing_power = 60; /* chance = 30 + 30 + 60 = 120 % côté formule */
    srand(2);

    int gov = 6;
    CHECK(!nh_world_reachable(gs->nodes, gov));
    int credits = gs->player.credits;
    int expected = gs->nodes[gov].data_value;
    for (int f = 0; f < gs->nodes[gov].file_count; f++)
        expected += gs->nodes[gov].secret_files[f].credits_value;
    bool done = false;
    for (int i = 0; i < 100 && !done; i++)
    {
        run_line(gs, "temporalhack gov-database", out, sizeof out);
        done = gs->nodes[gov].is_compromised;
    }
    CHECK(done); /* le hack temporel contourne la route */
    CHECK_INT(gs->player.credits, credits + expected);

    credits = gs->player.credits;
    run_line(gs, "temporalhack gov-database", out, sizeof out);
    CHECK(has(out, "déjà compromis"));
    CHECK_INT(gs->player.credits, credits);
    free(gs);
}

static void test_scan_message_when_nothing_new(void)
{
    GameState *gs = new_game();
    char out[8192];
    gs->player.scans_done = 10; /* le budget d'expérience des scans est épuisé */
    run_line(gs, "scan", out, sizeof out);
    CHECK(has(out, "Réseau déjà cartographié")); /* rien de nouveau : le message est juste */

    gs->player.level = LEVEL_APPRENTICE;
    run_line(gs, "scan", out, sizeof out);
    CHECK(has(out, "corp-server-01"));
    CHECK(!has(out, "Réseau déjà cartographié")); /* un nouveau système : le scan a servi */
    run_line(gs, "scan", out, sizeof out);
    CHECK(has(out, "Réseau déjà cartographié"));
    free(gs);
}

static void test_scan_reveals_by_level(void)
{
    GameState *gs = new_game();
    char out[8192];
    run_line(gs, "scan", out, sizeof out);
    CHECK(has(out, "localhost"));
    CHECK(!has(out, "corp-server-01")); /* niveau 1 : un seul système */
    CHECK(!has(out, "nexus-mainframe"));
    CHECK(has(out, "SÉCURITÉ: FAIBLE"));

    gs->player.level = LEVEL_APPRENTICE;
    run_line(gs, "scan", out, sizeof out);
    CHECK(has(out, "corp-server-01"));
    CHECK(has(out, "[NOUVEAU]"));
    CHECK(has(out, "[ROUTE FERMÉE]")); /* corp est visible mais son relais n'est pas compromis */
    CHECK(!has(out, "nexus-mainframe"));

    run_line(gs, "scan", out, sizeof out);
    CHECK(!has(out, "[NOUVEAU]")); /* plus rien de nouveau */

    gs->nodes[0].is_compromised = true;
    run_line(gs, "scan", out, sizeof out);
    CHECK(has(out, "[COMPROMIS]"));
    CHECK(!has(out, "[ROUTE FERMÉE]"));

    /* Niveau 6 : tout le monde est visible. */
    gs->player.level = LEVEL_LEGEND;
    run_line(gs, "scan", out, sizeof out);
    for (int i = 0; i < nh_world_count(); i++)
        CHECK(has(out, gs->nodes[i].name));

    /* En anglais. */
    nh_set_lang(NH_LANG_EN);
    gs->nodes[0].is_compromised = false;
    run_line(gs, "scan", out, sizeof out);
    CHECK(has(out, "[NO ROUTE]"));
    CHECK(has(out, "[SECURITY: LOW]"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

int main(void)
{
    nh_io_set_input(NULL);
    test_table_integrity();
    test_node_enum_matches_table();
    test_init_state();
    test_discover();
    test_find_and_reachable();
    test_resolve_messages();
    test_chance_values();
    test_chance_bounds_and_intel();
    test_chance_monotonic();
    test_social_chance();
    test_compromise_pays_once();
    test_deep_and_extract();
    test_bruteforce_respects_graph();
    test_backdoor_and_virus_need_route();
    test_traceroute_needs_no_route();
    test_exploit();
    test_exploit_is_locked_until_level_4();
    test_aihack_extracts_once();
    test_tools_follow_player_state();
    test_advhack_on_world_nodes();
    test_advhack_menu_numbers_match_methods();
    test_advhack_chance_modifiers();
    test_socialeng_gives_intel_once();
    test_temporalhack_needs_no_route_but_pays_once();
    test_scan_reveals_by_level();
    test_scan_message_when_nothing_new();
    return NH_TEST_REPORT("world");
}
