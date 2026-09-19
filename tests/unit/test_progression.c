#include "nh_test.h"

#include "nh_capture.h"

#include "../../src/core/io.h"
#include "../../src/core/platform.h"
#include "../../src/game/commands.h"
#include "../../src/game/progression.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <limits.h>
#include <stdlib.h>

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static int count_of_str(const char *text, const char *needle)
{
    int n = 0;
    for (const char *p = text; (p = strstr(p, needle)) != NULL; p += strlen(needle))
        n++;
    return n;
}

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    return gs;
}

static NhDispatch run_line(GameState *gs, const char *line, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    NhDispatch r = nh_dispatch(gs, line);
    nh_capture_end(&cap, out, size);
    return r;
}

/* Gain d'expérience en capturant l'affichage. */
static int grant(GameState *gs, int xp, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    int gained = nh_grant_xp(gs, xp);
    nh_capture_end(&cap, out, size);
    return gained;
}

static void test_curve(void)
{
    CHECK_INT(nh_level_xp_required(1), 0);
    CHECK_INT(nh_level_xp_required(0), -1);
    CHECK_INT(nh_level_xp_required(NH_LEVEL_MAX + 1), -1);
    CHECK_INT(nh_level_xp_required(-3), -1);

    /* Strictement croissante : chaque niveau demande plus que le précédent. */
    for (int l = 2; l <= NH_LEVEL_MAX; l++)
        CHECK(nh_level_xp_required(l) > nh_level_xp_required(l - 1));

    /* Bornes exactes de chaque niveau. */
    for (int l = 1; l <= NH_LEVEL_MAX; l++)
    {
        int need = nh_level_xp_required(l);
        CHECK_INT(nh_level_for_xp(need), l);
        if (l > 1)
            CHECK_INT(nh_level_for_xp(need - 1), l - 1);
    }
    CHECK_INT(nh_level_for_xp(0), 1);
    CHECK_INT(nh_level_for_xp(-50), 1);
    CHECK_INT(nh_level_for_xp(INT_MAX), NH_LEVEL_MAX); /* plafond */
    CHECK_INT(nh_level_for_xp(NH_XP_CAP), NH_LEVEL_MAX);
}

static void test_names(void)
{
    nh_set_lang(NH_LANG_FR);
    CHECK_STR(nh_level_name(1), "Novice");
    CHECK_STR(nh_level_name(5), "Maître");
    CHECK_STR(nh_level_name(NH_LEVEL_MAX), "Légende");
    CHECK_STR(nh_level_name(0), "");
    CHECK_STR(nh_level_name(NH_LEVEL_MAX + 1), "");
    nh_set_lang(NH_LANG_EN);
    CHECK_STR(nh_level_name(2), "Apprentice");
    CHECK_STR(nh_level_name(5), "Master");
    nh_set_lang(NH_LANG_FR);
}

static void test_scan_xp(void)
{
    int total = 0;
    for (int i = 0; i < 5; i++)
    {
        CHECK(nh_scan_xp(i) > 0);
        if (i > 0)
            CHECK(nh_scan_xp(i) < nh_scan_xp(i - 1)); /* décroît */
        total += nh_scan_xp(i);
    }
    CHECK_INT(total, nh_level_xp_required(2)); /* cinq scans = exactement le niveau 2 */
    CHECK_INT(nh_scan_xp(5), 0);
    CHECK_INT(nh_scan_xp(1000), 0);
    CHECK_INT(nh_scan_xp(-1), 0);
}

static void test_milestones(void)
{
    Player p;
    memset(&p, 0, sizeof p);
    for (int m = 0; m < NH_MS_COUNT; m++)
    {
        CHECK(nh_milestone_claim(&p, (NhMilestone)m));
        CHECK(!nh_milestone_claim(&p, (NhMilestone)m)); /* une seule fois */
    }
    CHECK(!nh_milestone_claim(&p, NH_MS_COUNT));
    CHECK(!nh_milestone_claim(&p, (NhMilestone)-1));

    /* Les jalons sont indépendants. */
    memset(&p, 0, sizeof p);
    CHECK(nh_milestone_claim(&p, NH_MS_QUANTUM_FIRST));
    CHECK(nh_milestone_claim(&p, NH_MS_DECRYPT_TEST));
}

static void test_grant_basic(void)
{
    GameState *gs = new_game();
    char out[4096];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);

    /* Rien pour 0 ou négatif : ni expérience, ni affichage. */
    CHECK_INT(grant(gs, 0, out, sizeof out), 0);
    CHECK_STR(out, "");
    CHECK_INT(grant(gs, -10, out, sizeof out), 0);
    CHECK_STR(out, "");
    CHECK_INT(gs->player.experience, 0);

    CHECK_INT(grant(gs, 14, out, sizeof out), 0);
    CHECK(has(out, "[+14 EXP]"));
    CHECK(!has(out, "NIVEAU"));
    CHECK_INT(gs->player.level, LEVEL_NOVICE);
    CHECK_INT(gs->player.experience, 14);

    /* 15 : niveau 2, bruteforce débloquée et annoncée. */
    CHECK_INT(grant(gs, 1, out, sizeof out), 1);
    CHECK(has(out, "NIVEAU SUPÉRIEUR"));
    CHECK(has(out, "niveau 2 : Apprenti"));
    CHECK(has(out, "bruteforce"));
    CHECK_INT(gs->player.level, LEVEL_APPRENTICE);
    CHECK(gs->player.commands_unlocked[CMD_BRUTEFORCE]);
    CHECK(!gs->player.commands_unlocked[CMD_DECRYPT]);

    nh_set_lang(NH_LANG_EN);
    grant(gs, 45, out, sizeof out); /* 60 : niveau 3 */
    CHECK(has(out, "[+45 XP]"));
    CHECK(has(out, "LEVEL UP"));
    CHECK(has(out, "level 3: Hacker"));
    CHECK(has(out, "New commands unlocked"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_unlocks_per_level(void)
{
    GameState *gs = new_game();
    char out[8192];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    grant(gs, 60, out, sizeof out); /* 1 -> 3 d'un coup : deux montées, deux annonces */
    CHECK_INT(gs->player.level, LEVEL_HACKER);
    CHECK_INT(count_of_str(out, "NIVEAU SUPÉRIEUR"), 2);
    CHECK(has(out, "niveau 2 : Apprenti"));
    CHECK(has(out, "niveau 3 : Hacker"));
    CHECK(gs->player.commands_unlocked[CMD_BRUTEFORCE]);
    CHECK(gs->player.commands_unlocked[CMD_DECRYPT]);
    CHECK(gs->player.commands_unlocked[CMD_BACKDOOR]);
    CHECK_INT(gs->player.virus_library_size, 1);
    /* Commandes à niveau minimum : socialeng (2) et advhack (3) sont annoncées. */
    CHECK(has(out, "socialeng"));
    CHECK(has(out, "advhack"));

    int stealth = gs->player.stealth_rating;
    grant(gs, 80, out, sizeof out); /* 140 : niveau 4 */
    CHECK_INT(gs->player.level, LEVEL_EXPERT);
    CHECK(gs->player.commands_unlocked[CMD_TRACE_ROUTE]);
    CHECK(gs->player.commands_unlocked[CMD_UPLOAD_VIRUS]);
    CHECK_INT(gs->player.virus_library_size, 2);
    CHECK_INT(gs->player.stealth_rating, stealth + 2);
    CHECK(has(out, "traceroute"));
    CHECK(has(out, "uploadvirus"));
    CHECK(has(out, "exploit")); /* enfin implémentée : annoncée avec les autres */
    CHECK(!gs->player.has_ai_assistant);

    int credits = gs->player.credits;
    stealth = gs->player.stealth_rating;
    grant(gs, 120, out, sizeof out); /* 260 : niveau 5 */
    CHECK_INT(gs->player.level, LEVEL_MASTER);
    CHECK(gs->player.has_ai_assistant);
    CHECK(gs->player.has_quantum_computer);
    CHECK_INT(gs->player.virus_library_size, 3);
    CHECK_INT(gs->player.stealth_rating, stealth + 3);
    CHECK_INT(gs->player.credits, credits + 5000);
    CHECK(has(out, "aihack"));
    CHECK(has(out, "quantumdecrypt"));
    CHECK(has(out, "neuralsync"));
    CHECK(has(out, "Équipement débloqué"));
    CHECK(has(out, "[+5000 crédits]"));
    CHECK(!has(out, "temporalhack"));

    /* Niveau 6 : plafond. temporalhack (niveau minimum 6) devient utilisable. */
    credits = gs->player.credits;
    grant(gs, 160, out, sizeof out);
    CHECK_INT(gs->player.level, LEVEL_LEGEND);
    CHECK(has(out, "niveau 6 : Légende"));
    CHECK(has(out, "temporalhack"));
    CHECK(has(out, "Niveau maximum atteint"));
    CHECK_INT(gs->player.credits, credits); /* pas de bonus de crédits ici */
    CHECK(nh_command_available(gs, nh_find_command("temporalhack")));

    /* Au plafond : l'expérience continue d'être comptée mais le niveau ne bouge plus. */
    CHECK_INT(grant(gs, 100000, out, sizeof out), 0);
    CHECK_INT(gs->player.level, LEVEL_LEGEND);
    CHECK(!has(out, "NIVEAU SUPÉRIEUR"));
    CHECK_INT(grant(gs, INT_MAX, out, sizeof out), 0); /* pas de dépassement d'entier */
    CHECK_INT(gs->player.experience, NH_XP_CAP);
    CHECK_INT(gs->player.level, LEVEL_LEGEND);
    free(gs);
}

static void test_status_shows_progress(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    run_line(gs, "status", out, sizeof out);
    CHECK(has(out, "Niveau: 1 (Novice)"));
    CHECK(has(out, "Expérience: 0/15"));

    grant(gs, 20, out, sizeof out);
    run_line(gs, "status", out, sizeof out);
    CHECK(has(out, "Niveau: 2 (Apprenti)"));
    CHECK(has(out, "Expérience: 20/60"));

    grant(gs, 1000, out, sizeof out);
    run_line(gs, "status", out, sizeof out);
    CHECK(has(out, "(Légende)"));
    CHECK(has(out, "(MAX)"));
    free(gs);
}

/* ---- Fin des sources d'expérience répétables (farm) ---------------------------------- */

static void test_scan_is_not_a_farm(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);

    for (int i = 0; i < 5; i++)
        run_line(gs, "scan", out, sizeof out);
    CHECK_INT(gs->player.experience, 15);
    CHECK_INT(gs->player.level, LEVEL_APPRENTICE);
    CHECK(has(out, "NIVEAU SUPÉRIEUR")); /* la 5e et dernière */

    for (int i = 0; i < 200; i++)
        run_line(gs, "scan", out, sizeof out);
    CHECK_INT(gs->player.experience, 15); /* 200 scans de plus : rien */
    CHECK_INT(gs->player.level, LEVEL_APPRENTICE);
    CHECK(has(out, "déjà cartographié"));
    free(gs);
}

static void test_decrypt_is_not_a_farm(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);
    gs->player.commands_unlocked[CMD_DECRYPT] = true;

    run_line(gs, "decrypt WKLV#LV#D#WHVW", out, sizeof out);
    CHECK_INT(gs->player.experience, 20);
    CHECK(has(out, "[+20 EXP]"));
    for (int i = 0; i < 10; i++)
        run_line(gs, "decrypt WKLV#LV#D#WHVW", out, sizeof out);
    CHECK_INT(gs->player.experience, 20);
    CHECK(has(out, "déjà obtenu"));
    CHECK(has(out, "THIS IS A TEST")); /* le décryptage lui-même fonctionne toujours */

    /* Un autre texte ne rapporte rien non plus. */
    run_line(gs, "decrypt ABC", out, sizeof out);
    CHECK_INT(gs->player.experience, 20);
    free(gs);
}

static void test_traceroute_is_not_a_farm(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);
    gs->player.commands_unlocked[CMD_TRACE_ROUTE] = true;

    run_line(gs, "traceroute localhost", out, sizeof out);
    CHECK_INT(gs->player.experience, 8);
    for (int i = 0; i < 10; i++)
        run_line(gs, "traceroute localhost", out, sizeof out);
    CHECK_INT(gs->player.experience, 8);
    CHECK(has(out, "déjà tracée"));

    /* Un système encore inconnu ne se trace pas… */
    run_line(gs, "traceroute corp-server-01", out, sizeof out);
    CHECK(has(out, "introuvable"));
    CHECK_INT(gs->player.experience, 8);
    /* …un autre système, une fois découvert, rapporte, lui, une fois. */
    gs->nodes[1].is_discovered = true;
    run_line(gs, "traceroute corp-server-01", out, sizeof out);
    CHECK_INT(gs->player.experience, 16);
    free(gs);
}

static void test_aihack_is_not_a_farm(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);
    gs->player.commands_unlocked[CMD_AI_HACK] = true;
    gs->player.has_ai_assistant = true;
    gs->player.level = LEVEL_LEGEND;
    gs->player.experience = nh_level_xp_required(NH_LEVEL_MAX);
    srand(7);

    int first_success = -1;
    for (int i = 0; i < 40 && first_success < 0; i++)
    {
        run_line(gs, "aihack localhost", out, sizeof out);
        if (gs->nodes[0].is_compromised)
            first_success = i;
    }
    CHECK(first_success >= 0);

    int credits = gs->player.credits;
    int xp = gs->player.experience;
    for (int i = 0; i < 20; i++)
        run_line(gs, "aihack localhost", out, sizeof out);
    CHECK_INT(gs->player.credits, credits); /* plus de fichiers recrédités à chaque appel */
    CHECK_INT(gs->player.experience, xp);
    CHECK(has(out, "Aucun fichier à extraire")); /* tout a été extrait à la compromission */
    free(gs);
}

static void test_quantum_is_not_a_farm(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);
    gs->player.commands_unlocked[CMD_QUANTUM_DECRYPT] = true;
    gs->player.has_quantum_computer = true;
    gs->player.level = LEVEL_MASTER;
    gs->player.experience = nh_level_xp_required(NH_LEVEL_MAX - 1);
    int credits = gs->player.credits;
    int xp = gs->player.experience;

    run_line(gs, "quantumdecrypt WKLV#LV#D#WHVW", out, sizeof out);
    CHECK_INT(gs->player.credits, credits + 2000);
    CHECK_INT(gs->player.experience, xp + 50);

    credits = gs->player.credits;
    xp = gs->player.experience;
    for (int i = 0; i < 10; i++)
        run_line(gs, "quantumdecrypt WKLV#LV#D#WHVW", out, sizeof out);
    CHECK_INT(gs->player.credits, credits);
    CHECK_INT(gs->player.experience, xp);
    CHECK(has(out, "déjà obtenu"));

    /* Le document ultra-secret : +10 000 crédits, une seule fois. */
    run_line(gs, "quantumdecrypt CLASSIFIED", out, sizeof out);
    CHECK_INT(gs->player.credits, credits + 10000);
    credits = gs->player.credits;
    for (int i = 0; i < 5; i++)
        run_line(gs, "quantumdecrypt CLASSIFIED", out, sizeof out);
    CHECK_INT(gs->player.credits, credits);
    free(gs);
}

static void test_advanced_xp_goes_through_level_ups(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);

    /* Compromettre un système avancé verse son expérience par nh_grant_xp (via le monde) : la
     * montée de niveau et les déblocages ont bien lieu. */
    gs->player.level = LEVEL_APPRENTICE;
    gs->player.experience = 20;
    NhCapture cap = nh_capture_begin();
    nh_grant_xp(gs, 100);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->player.level, LEVEL_HACKER);
    CHECK(gs->player.commands_unlocked[CMD_DECRYPT]);
    free(gs);
}

int main(void)
{
    nh_io_set_input(NULL);
    test_curve();
    test_names();
    test_scan_xp();
    test_milestones();
    test_grant_basic();
    test_unlocks_per_level();
    test_status_shows_progress();
    test_scan_is_not_a_farm();
    test_decrypt_is_not_a_farm();
    test_traceroute_is_not_a_farm();
    test_aihack_is_not_a_farm();
    test_quantum_is_not_a_farm();
    test_advanced_xp_goes_through_level_ups();
    return NH_TEST_REPORT("progression");
}
