#include "nh_test.h"

#include "nh_capture.h"

#include "../../src/core/io.h"
#include "../../src/core/platform.h"
#include "../../src/game/alert.h"
#include "../../src/game/commands.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <limits.h>
#include <stdlib.h>

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static FILE *g_fed = NULL;

/* Fournit `text` comme entrée du joueur pour la suite du test (ferme le flux précédent). */
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

static void unfeed(void)
{
    nh_io_set_input(NULL);
    if (g_fed != NULL)
        fclose(g_fed);
    g_fed = NULL;
}

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    feed(""); /* jamais d'attente sur le vrai clavier : une lecture rencontre la fin d'entrée */
    return gs;
}

static NhDispatch run_line(GameState *gs, const char *line, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    NhDispatch r = nh_dispatch(gs, line);
    nh_capture_end(&cap, out, size);
    return r;
}

static void test_add_reduce(void)
{
    AlertSystem a;
    nh_alert_init(&a);
    CHECK_INT(a.level, 0);
    CHECK_INT(a.max_level, 0);

    CHECK_INT(nh_alert_add(&a, 5), 5);
    CHECK_INT(a.level, 5);
    CHECK_INT(nh_alert_add(&a, 0), 0);
    CHECK_INT(nh_alert_add(&a, -7), 0); /* une valeur négative ne réduit pas, ne monte pas */
    CHECK_INT(a.level, 5);

    CHECK_INT(nh_alert_add(&a, 90), 90);
    CHECK_INT(a.level, 95);
    CHECK_INT(nh_alert_add(&a, 20), 5); /* plafonné : seule la hausse réelle est retournée */
    CHECK_INT(a.level, 100);
    CHECK_INT(nh_alert_add(&a, 1), 0);
    CHECK_INT(nh_alert_add(&a, INT_MAX), 0);
    CHECK_INT(a.level, 100);
    CHECK_INT(a.max_level, 100);

    CHECK_INT(nh_alert_reduce(&a, 30), 30);
    CHECK_INT(a.level, 70);
    CHECK_INT(a.max_level, 100); /* le maximum ne redescend pas */
    CHECK_INT(nh_alert_reduce(&a, -5), 0);
    CHECK_INT(nh_alert_reduce(&a, 1000), 70); /* plancher à 0, baisse réelle retournée */
    CHECK_INT(a.level, 0);
    CHECK_INT(nh_alert_reduce(&a, INT_MAX), 0);

    nh_alert_init(&a);
    CHECK_INT(nh_alert_add(&a, INT_MAX), 100); /* pas de dépassement d'entier */
    CHECK_INT(a.level, 100);
}

static void test_decay(void)
{
    AlertSystem a;
    nh_alert_init(&a);
    a.level = 10;
    CHECK_INT(nh_alert_decay(&a), 1);
    CHECK_INT(a.level, 9);
    a.vpn_active = true;
    CHECK_INT(nh_alert_decay(&a), 2);
    a.proxy_active = true;
    CHECK_INT(nh_alert_decay(&a), 3);
    CHECK_INT(a.level, 4); /* 10 - 1 - 2 - 3 */
    CHECK_INT(nh_alert_decay(&a), 3);
    CHECK_INT(nh_alert_decay(&a), 1); /* ne descend pas sous 0 : seule la baisse réelle est retournée */
    CHECK_INT(a.level, 0);
    CHECK_INT(nh_alert_decay(&a), 0);
}

static void test_thresholds(void)
{
    CHECK_INT(nh_alert_band(0), NH_ALERT_SAFE);
    CHECK_INT(nh_alert_band(29), NH_ALERT_SAFE);
    CHECK_INT(nh_alert_band(30), NH_ALERT_WARN);
    CHECK_INT(nh_alert_band(69), NH_ALERT_WARN);
    CHECK_INT(nh_alert_band(70), NH_ALERT_DANGER_BAND);
    CHECK_INT(nh_alert_band(100), NH_ALERT_DANGER_BAND);

    AlertSystem a;
    nh_alert_init(&a);
    a.level = 99;
    CHECK(!nh_alert_is_game_over(&a));
    a.level = 100;
    CHECK(nh_alert_is_game_over(&a));

    a.level = 79;
    CHECK(!nh_alert_shop_closed(&a));
    a.level = 80;
    CHECK(nh_alert_shop_closed(&a));

    int expected[][2] = {{0, 0},  {29, 0}, {30, 5},  {49, 5},
                         {50, 10}, {79, 10}, {80, 20}, {100, 20}};
    for (size_t i = 0; i < sizeof expected / sizeof expected[0]; i++)
    {
        a.level = expected[i][0];
        CHECK_INT(nh_alert_success_penalty(&a), expected[i][1]);
    }
}

static void test_reduction(void)
{
    AlertSystem a;
    int credits, applied;

    for (int m = 0; m < NH_REDUCTION_COUNT; m++)
    {
        CHECK(nh_alert_reduction_amount((NhReduction)m) > 0);
        CHECK(nh_alert_reduction_cost((NhReduction)m) >= 0);
    }

    /* Crédits insuffisants : rien ne change. */
    nh_alert_init(&a);
    a.level = 50;
    credits = 39;
    CHECK_INT(nh_alert_apply_reduction(&a, NH_REDUCTION_LAYLOW, &credits, NULL), NH_REDUCE_NO_CREDITS);
    CHECK_INT(credits, 39);
    CHECK_INT(a.level, 50);

    /* Réduction payante. */
    credits = 100;
    CHECK_INT(nh_alert_apply_reduction(&a, NH_REDUCTION_LAYLOW, &credits, &applied), NH_REDUCE_OK);
    CHECK_INT(credits, 60);
    CHECK_INT(applied, 25);
    CHECK_INT(a.level, 25);

    /* On ne descend pas sous 0 et `applied` dit la vérité. */
    a.level = 3;
    CHECK_INT(nh_alert_apply_reduction(&a, NH_REDUCTION_FRAME, &credits, &applied), NH_REDUCE_OK);
    CHECK_INT(a.level, 0);
    CHECK_INT(applied, 3);

    /* VPN et proxy : activent la protection passive. */
    CHECK(!a.vpn_active && !a.proxy_active);
    credits = 100;
    CHECK_INT(nh_alert_apply_reduction(&a, NH_REDUCTION_VPN, &credits, NULL), NH_REDUCE_OK);
    CHECK(a.vpn_active);
    CHECK_INT(credits, 80);
    CHECK_INT(nh_alert_apply_reduction(&a, NH_REDUCTION_PROXY, &credits, NULL), NH_REDUCE_OK);
    CHECK(a.proxy_active);
    CHECK_INT(credits, 50);

    /* Ghost Protocol : consomme un exemplaire, gratuit, refusé s'il n'y en a plus. */
    credits = 0;
    a.level = 60;
    CHECK_INT(nh_alert_apply_reduction(&a, NH_REDUCTION_GHOST, &credits, NULL), NH_REDUCE_NO_GHOST);
    CHECK_INT(a.level, 60);
    a.ghost_protocols_available = 1;
    CHECK_INT(nh_alert_apply_reduction(&a, NH_REDUCTION_GHOST, &credits, NULL), NH_REDUCE_OK);
    CHECK_INT(a.ghost_protocols_available, 0);
    CHECK_INT(a.level, 30);
    CHECK_INT(credits, 0);

    /* Attendre est gratuit, même sans crédit. */
    CHECK_INT(nh_alert_apply_reduction(&a, NH_REDUCTION_TIME, &credits, NULL), NH_REDUCE_OK);
    CHECK_INT(a.level, 22);
}

static int count_of(const char *text, const char *cell)
{
    int n = 0;
    for (const char *p = text; (p = strstr(p, cell)) != NULL; p += strlen(cell))
        n++;
    return n;
}

static void test_bar(void)
{
    char bar[256];

    nh_alert_bar(bar, sizeof bar, 0, 20);
    CHECK_INT(count_of(bar, "█"), 0);
    CHECK_INT(count_of(bar, "░"), 20);
    nh_alert_bar(bar, sizeof bar, 1, 20);
    CHECK_INT(count_of(bar, "█"), 1); /* un peu d'alerte reste visible */
    nh_alert_bar(bar, sizeof bar, 50, 20);
    CHECK_INT(count_of(bar, "█"), 10);
    nh_alert_bar(bar, sizeof bar, 100, 20);
    CHECK_INT(count_of(bar, "█"), 20);
    CHECK_INT(count_of(bar, "░"), 0);
    CHECK_INT(nh_display_width(bar), 20);
    nh_alert_bar(bar, sizeof bar, 500, 20); /* hors bornes : borné */
    CHECK_INT(count_of(bar, "█"), 20);
    nh_alert_bar(bar, sizeof bar, -5, 20);
    CHECK_INT(count_of(bar, "█"), 0);

    /* Petit tampon : jamais de débordement, toujours terminé, jamais de caractère coupé. */
    char tiny[8];
    nh_alert_bar(tiny, sizeof tiny, 100, 20);
    CHECK(strlen(tiny) < sizeof tiny);
    CHECK_INT(strlen(tiny) % 3, 0);
    nh_alert_bar(tiny, 1, 100, 20);
    CHECK_STR(tiny, "");
    nh_alert_bar(bar, sizeof bar, 50, 0);
    CHECK_STR(bar, "");
}

static void test_raise_output(void)
{
    AlertSystem a;
    char out[1024];
    nh_alert_init(&a);
    nh_term_set_color(false);
    nh_set_lang(NH_LANG_FR);

    NhCapture cap = nh_capture_begin();
    nh_alert_raise(&a, 5);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "[ALERTE +5]"));
    CHECK(!has(out, "ÉLEVÉ"));
    CHECK_INT(a.level, 5);

    cap = nh_capture_begin();
    nh_alert_raise(&a, 50); /* 55 : niveau élevé */
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "[ALERTE +50]"));
    CHECK(has(out, "NIVEAU D'ALERTE ÉLEVÉ"));

    cap = nh_capture_begin();
    nh_alert_raise(&a, 30); /* 85 : critique */
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "DANGER CRITIQUE"));

    /* Le montant annoncé est la hausse réelle, pas la demandée. */
    cap = nh_capture_begin();
    nh_alert_raise(&a, 40);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "[ALERTE +15]"));
    CHECK_INT(a.level, 100);

    /* Rien à annoncer si rien n'a changé. */
    cap = nh_capture_begin();
    nh_alert_raise(&a, 10);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_STR(out, "");
    nh_alert_init(&a);
    cap = nh_capture_begin();
    nh_alert_raise(&a, 0);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_STR(out, "");

    nh_set_lang(NH_LANG_EN);
    cap = nh_capture_begin();
    nh_alert_raise(&a, 3);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "[ALERT +3]"));
    nh_set_lang(NH_LANG_FR);

    /* Panneau d'état : niveau, jauge, conséquences. */
    a.level = 75;
    a.vpn_active = true;
    cap = nh_capture_begin();
    nh_alert_print_status(&a);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "75/100"));
    CHECK(has(out, "[DANGER]"));
    CHECK(has(out, "Traçage actif"));
    CHECK(strchr(out, '\033') == NULL); /* --no-color respecté */

    a.level = 5;
    cap = nh_capture_begin();
    nh_alert_print_status(&a);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "[SÉCURISÉ]"));
    CHECK(!has(out, "Traçage"));
    CHECK(!has(out, "Surveillance"));

    /* Menu : Ghost indisponible tant qu'on n'en a pas. */
    cap = nh_capture_begin();
    nh_alert_print_menu(&a);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "Ghost Protocol (indisponible)"));
    a.ghost_protocols_available = 2;
    cap = nh_capture_begin();
    nh_alert_print_menu(&a);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(!has(out, "indisponible"));
    nh_term_set_color(true);
}

static void test_turn_passes_only_for_hacks(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);

    /* VPN + proxy : refroidissement de 3 avant l'action ; scan ajoute 1. */
    gs->alert.level = 10;
    gs->alert.vpn_active = gs->alert.proxy_active = true;
    CHECK_INT(run_line(gs, "scan", out, sizeof out), NH_DISPATCH_OK);
    CHECK_INT(gs->alert.level, 8);

    /* Aide, statut, quêtes : le temps ne passe pas, l'alerte ne bouge pas. */
    run_line(gs, "help", out, sizeof out);
    run_line(gs, "status", out, sizeof out);
    run_line(gs, "quests", out, sizeof out);
    CHECK_INT(gs->alert.level, 8);

    /* Une commande verrouillée ou inconnue ne fait pas non plus passer le temps. */
    CHECK_INT(run_line(gs, "bruteforce localhost", out, sizeof out), NH_DISPATCH_LOCKED);
    CHECK_INT(run_line(gs, "nope", out, sizeof out), NH_DISPATCH_UNKNOWN);
    CHECK_INT(gs->alert.level, 8);
    free(gs);
}

static void test_status_shows_real_alert(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    gs->alert.level = 42;
    run_line(gs, "status", out, sizeof out);
    CHECK(has(out, "42/100"));
    CHECK(has(out, "[ATTENTION]"));
    gs->alert.level = 85;
    run_line(gs, "status", out, sizeof out);
    CHECK(has(out, "85/100"));
    CHECK(has(out, "[DANGER]"));
    free(gs);
}

static void test_shop_closed(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    gs->alert.level = 80;
    int credits = gs->player.credits;
    CHECK_INT(run_line(gs, "shop", out, sizeof out), NH_DISPATCH_FAILED);
    CHECK(has(out, "MARCHÉ FERMÉ"));
    CHECK(has(out, "80/100"));
    CHECK_INT(gs->player.credits, credits);

    nh_set_lang(NH_LANG_EN);
    run_line(gs, "shop", out, sizeof out);
    CHECK(has(out, "MARKET CLOSED"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_laylow(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);

    /* VPN : coûte 20 ¢, baisse l'alerte, active la protection. */
    gs->alert.level = 50;
    feed("2\n");
    CHECK_INT(run_line(gs, "laylow", out, sizeof out), NH_DISPATCH_OK);
    CHECK_INT(gs->alert.level, 40);
    CHECK_INT(gs->player.credits, 80);
    CHECK(gs->alert.vpn_active);
    CHECK(has(out, "50 → 40/100"));

    /* Annuler ne change rien. */
    feed("0\n");
    CHECK_INT(run_line(gs, "laylow", out, sizeof out), NH_DISPATCH_OK);
    CHECK_INT(gs->alert.level, 40);
    CHECK(has(out, "Vous restez dans l'ombre"));

    /* Saisies invalides : rejetées proprement, rien ne bouge. */
    const char *bad[] = {"abc\n", "7\n", "-1\n", "\n", "2x\n"};
    for (size_t i = 0; i < sizeof bad / sizeof bad[0]; i++)
    {
        feed(bad[i]);
        CHECK_INT(run_line(gs, "laylow", out, sizeof out), NH_DISPATCH_FAILED);
        CHECK(has(out, "Choix invalide"));
        CHECK_INT(gs->alert.level, 40);
        CHECK_INT(gs->player.credits, 80);
    }

    /* Entrée fermée pendant le menu : pas de boucle, échec propre. */
    feed("");
    CHECK_INT(run_line(gs, "laylow", out, sizeof out), NH_DISPATCH_FAILED);

    /* Ghost sans exemplaire ; crédits insuffisants. */
    feed("4\n");
    CHECK_INT(run_line(gs, "laylow", out, sizeof out), NH_DISPATCH_FAILED);
    CHECK(has(out, "Aucun Ghost Protocol"));
    gs->player.credits = 10;
    feed("6\n");
    CHECK_INT(run_line(gs, "laylow", out, sizeof out), NH_DISPATCH_FAILED);
    CHECK(has(out, "60 ¢ requis"));
    CHECK_INT(gs->player.credits, 10);
    CHECK_INT(gs->alert.level, 40);

    /* Bouc émissaire : le karma négatif est réel (réputation), pas seulement affiché. */
    gs->player.credits = 100;
    gs->player.reputation = 12;
    feed("6\n");
    CHECK_INT(run_line(gs, "laylow", out, sizeof out), NH_DISPATCH_OK);
    CHECK_INT(gs->alert.level, 5);
    CHECK_INT(gs->player.reputation, 7);
    CHECK_INT(gs->player.credits, 40);
    unfeed();
    free(gs);
}

static void test_game_over_ends_loop(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);

    /* Alerte à 99 : la moindre action de hacking (bruteforce, gagne comme perd) fait exploser. */
    gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;
    gs->alert.level = 99;
    feed("bruteforce localhost\nstatus\nstatus\n");
    NhCapture cap = nh_capture_begin();
    game_loop(gs);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(gs->player.game_over);
    CHECK(has(out, "GAME OVER"));
    CHECK(has(out, "détecté"));
    CHECK_INT(count_of(out, "STATUT DU HACKER"), 0); /* la boucle s'est arrêtée net */

    free(gs);

    /* En dessous du seuil, la partie continue jusqu'à la fin de l'entrée. */
    gs = new_game();
    gs->alert.level = 60;
    feed("status\n");
    cap = nh_capture_begin();
    game_loop(gs);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(!gs->player.game_over);
    CHECK(has(out, "Entrée fermée"));
    CHECK(!has(out, "GAME OVER"));

    unfeed();
    free(gs);
}

/*
 * Le malus d'alerte est réellement appliqué aux hacks. `rand()` est initialisé avec une
 * graine fixe : le résultat est le même à chaque exécution, et l'écart attendu est très
 * supérieur au bruit statistique de 1500 essais.
 *
 * `measure` retourne, sur 1500 parties neuves à ce niveau d'alerte :
 *   - backdoor   : le nombre de réussites ;
 *   - bruteforce : le nombre d'essais de mot de passe ratés (il en tente 5 de suite, donc
 *                  la réussite finale est presque assurée et ne mesure rien).
 */
static int measure(const char *command, CommandType unlock, int alert_level)
{
    char out[8192];
    int total = 0;
    srand(12345);
    for (int i = 0; i < 1500; i++)
    {
        GameState *gs = new_game();
        gs->player.commands_unlocked[unlock] = true;
        gs->alert.level = alert_level;
        run_line(gs, command, out, sizeof out);
        total += (unlock == CMD_BACKDOOR) ? (gs->nodes[0].has_backdoor ? 1 : 0)
                                          : count_of(out, "ÉCHEC");
        free(gs);
    }
    return total;
}

static void test_penalty_is_applied(void)
{
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
    nh_set_fast(true);

    /* bruteforce localhost : 65 % par essai au calme, 55 % à 79 (malus 10). */
    int fails_calm = measure("bruteforce localhost", CMD_BRUTEFORCE, 0);
    int fails_hot = measure("bruteforce localhost", CMD_BRUTEFORCE, 79);
    CHECK(fails_calm > 500 && fails_calm < 1100); /* le test mesure bien quelque chose */
    CHECK(fails_hot * 10 > fails_calm * 13);      /* nettement plus d'échecs sous alerte */

    /* backdoor localhost : 65 % au calme, 55 % à 79. */
    int door_calm = measure("backdoor localhost", CMD_BACKDOOR, 0);
    int door_hot = measure("backdoor localhost", CMD_BACKDOOR, 79);
    CHECK(door_calm > 600 && door_calm < 1350);
    CHECK(door_hot < door_calm - 80);
}

int main(void)
{
    test_add_reduce();
    test_decay();
    test_thresholds();
    test_reduction();
    test_bar();
    test_raise_output();
    test_turn_passes_only_for_hacks();
    test_status_shows_real_alert();
    test_shop_closed();
    test_laylow();
    test_game_over_ends_loop();
    test_penalty_is_applied();
    unfeed();
    return NH_TEST_REPORT("alert");
}
