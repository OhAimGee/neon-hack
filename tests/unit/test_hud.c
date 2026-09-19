#include "nh_test.h"

#include "../../src/i18n/i18n.h"
#include "../../src/ui/hud.h"
#include "../../src/ui/term.h"

#include <string.h>

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static void test_should_enable(void)
{
    CHECK(nh_hud_should_enable(false, true, 80, 24));
    CHECK(nh_hud_should_enable(false, true, 200, 60));
    CHECK(!nh_hud_should_enable(true, true, 120, 40));  /* --no-hud */
    CHECK(!nh_hud_should_enable(false, false, 120, 40)); /* sortie redirigée */
    CHECK(!nh_hud_should_enable(false, true, 79, 24));
    CHECK(!nh_hud_should_enable(false, true, 80, 23));
    CHECK(!nh_hud_should_enable(false, true, 0, 0));
    CHECK(!nh_hud_should_enable(false, true, -1, 40));
}

static void test_truncate(void)
{
    char s[64];

    strcpy(s, "hello");
    CHECK_INT(nh_truncate_width(s, 10), 5);
    CHECK_STR(s, "hello");
    CHECK_INT(nh_truncate_width(s, 3), 3);
    CHECK_STR(s, "hel");
    CHECK_INT(nh_truncate_width(s, 0), 0);
    CHECK_STR(s, "");

    strcpy(s, "héllo"); /* é = 2 octets, 1 colonne : jamais coupé en deux */
    CHECK_INT(nh_truncate_width(s, 2), 2);
    CHECK_STR(s, "hé");

    strcpy(s, "a🧠b"); /* emoji = 2 colonnes */
    CHECK_INT(nh_truncate_width(s, 3), 3);
    CHECK_STR(s, "a🧠");
    strcpy(s, "a🧠b");
    CHECK_INT(nh_truncate_width(s, 2), 1); /* l'emoji ne tient pas à moitié : on s'arrête avant */
    CHECK_STR(s, "a");
}

static void test_gauge(void)
{
    char g[128];
    nh_gauge(g, sizeof g, 0, 100, 10);
    CHECK_STR(g, "░░░░░░░░░░");
    nh_gauge(g, sizeof g, 100, 100, 10);
    CHECK_STR(g, "██████████");
    nh_gauge(g, sizeof g, 1, 100, 10);
    CHECK_STR(g, "█░░░░░░░░░"); /* au moins une case dès que value > 0 */
    nh_gauge(g, sizeof g, 5, 10, 10);
    CHECK_STR(g, "█████░░░░░");
    nh_gauge(g, sizeof g, -3, 100, 4);
    CHECK_STR(g, "░░░░");
    nh_gauge(g, sizeof g, 1000, 100, 4);
    CHECK_STR(g, "████");
    nh_gauge(g, sizeof g, 50, 0, 4); /* max invalide */
    CHECK_STR(g, "");
    nh_gauge(g, sizeof g, 50, 100, 0);
    CHECK_STR(g, "");
    char tiny[8];
    nh_gauge(tiny, sizeof tiny, 100, 100, 10); /* 2 cases de 3 octets + NUL au plus */
    CHECK(strlen(tiny) < sizeof tiny);
    CHECK_INT(strlen(tiny) % 3, 0);
}

static NhHudData sample(const char *name, int alert, bool stealth, const char *const *cmds, size_t n)
{
    NhHudData d = {.name = name,
                   .level = 3,
                   .credits = 1234,
                   .alert = alert,
                   .alert_color = NH_C_YELLOW,
                   .stealth = stealth,
                   .commands = cmds,
                   .command_count = n};
    return d;
}

static void test_top_bar(void)
{
    static const char *const cmds[] = {"help", "scan"};
    char out[1024];

    const char *names[] = {"Neo", "", "Hacker" "éééééééééééééééééééééééééééééééééé",
                           "🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠🧠"};
    NhLang langs[] = {NH_LANG_FR, NH_LANG_EN};
    int alerts[] = {0, 1, 50, 99, 100, -5, 400};

    for (size_t l = 0; l < 2; l++)
    {
        nh_set_lang(langs[l]);
        for (int color = 0; color < 2; color++)
        {
            nh_term_set_color(color != 0);
            for (size_t nm = 0; nm < sizeof names / sizeof names[0]; nm++)
                for (size_t a = 0; a < sizeof alerts / sizeof alerts[0]; a++)
                    for (int stealth = 0; stealth < 2; stealth++)
                        for (int cols = 40; cols <= 200; cols += 7)
                        {
                            NhHudData d = sample(names[nm], alerts[a], stealth != 0, cmds, 2);
                            nh_hud_compose_top(out, sizeof out, &d, cols);
                            /* Largeur exacte, quelles que soient la langue, la couleur, la taille du nom. */
                            if (nh_display_width(out) != (size_t)cols)
                                CHECK_INT(nh_display_width(out), cols);
                            else
                                CHECK(true);
                        }
        }
    }
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    /* Contenu à la largeur normale. */
    NhHudData d = sample("Neo", 42, false, cmds, 2);
    nh_hud_compose_top(out, sizeof out, &d, 100);
    CHECK(has(out, "Neo"));
    CHECK(has(out, "Niv 3"));
    CHECK(has(out, "1234 ¢"));
    CHECK(has(out, "ALERTE"));
    CHECK(has(out, " 42 "));
    CHECK(has(out, "████")); /* 42 % de 10 cases = 5 pleines */
    CHECK(!has(out, "FURTIF"));
    CHECK(strchr(out, '\033') == NULL);

    d.stealth = true;
    nh_hud_compose_top(out, sizeof out, &d, 100);
    CHECK(has(out, "FURTIF"));

    /* Anglais. */
    nh_set_lang(NH_LANG_EN);
    nh_hud_compose_top(out, sizeof out, &d, 100);
    CHECK(has(out, "Lvl 3"));
    CHECK(has(out, "ALERT"));
    CHECK(has(out, "STEALTH"));
    nh_set_lang(NH_LANG_FR);

    /* Alerte hors bornes : ramenée à 0..100. */
    d.alert = 400;
    nh_hud_compose_top(out, sizeof out, &d, 100);
    CHECK(has(out, " 100 "));
    d.alert = -5;
    nh_hud_compose_top(out, sizeof out, &d, 100);
    CHECK(has(out, " 0 "));

    /* Étroit : la jauge disparaît en premier, la valeur d'alerte reste, le nom est coupé. */
    d = sample("Neo", 42, false, cmds, 2);
    nh_hud_compose_top(out, sizeof out, &d, 34);
    CHECK(has(out, "ALERTE"));
    CHECK(has(out, "42"));
    CHECK(!has(out, "█"));
    d = sample("UnNomVraimentTresLongPourUnHackerDebutant", 42, false, cmds, 2);
    nh_hud_compose_top(out, sizeof out, &d, 50);
    CHECK(has(out, "ALERTE"));
    CHECK(has(out, "42"));
    CHECK_INT(nh_display_width(out), 50);

    /* Couleurs activées : elles ne changent pas la largeur, et la jauge est colorée. */
    nh_term_set_color(true);
    d = sample("Neo", 42, false, cmds, 2);
    nh_hud_compose_top(out, sizeof out, &d, 100);
    CHECK(strchr(out, '\033') != NULL);
    CHECK_INT(nh_display_width(out), 100);
    nh_term_set_color(false);

    /* Valeurs limites : nom NULL, tampon minuscule, largeur nulle. */
    d.name = NULL;
    nh_hud_compose_top(out, sizeof out, &d, 80);
    CHECK_INT(nh_display_width(out), 80);
    char tiny[24];
    nh_hud_compose_top(tiny, sizeof tiny, &d, 100);
    CHECK(strlen(tiny) < sizeof tiny);
    CHECK_INT(nh_hud_compose_top(out, sizeof out, &d, 0), 0);
    CHECK_STR(out, "");
    CHECK_INT(nh_hud_compose_top(out, 0, &d, 80), 0);
}

static void test_bottom_bar(void)
{
    char out[1024];
    static const char *const many[] = {"help",    "scan",       "bruteforce", "decrypt",  "backdoor",
                                       "shop",    "laylow",     "quests",     "contacts", "messages",
                                       "advhack", "aiassist",   "neuralsync"};
    static const char *const few[] = {"help", "scan"};
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    NhHudData d = sample("Neo", 0, false, few, 2);
    nh_hud_compose_bottom(out, sizeof out, &d, 100);
    CHECK(has(out, "Commandes :"));
    CHECK(has(out, "help · scan"));
    CHECK(!has(out, "…"));
    CHECK_INT(nh_display_width(out), 100);

    /* Toutes les largeurs : exactes, `help` toujours présente, ellipse quand on coupe. */
    d = sample("Neo", 0, false, many, sizeof many / sizeof many[0]);
    for (int cols = 40; cols <= 220; cols++)
    {
        nh_hud_compose_bottom(out, sizeof out, &d, cols);
        /* Largeur exacte, `help` présente, et si une commande a sauté, l'ellipse le dit. */
        bool all_shown = has(out, "neuralsync");
        if (nh_display_width(out) != (size_t)cols || !has(out, "help") || (!all_shown && !has(out, "…")))
            CHECK_INT(cols, -1); /* échec : on affiche la largeur fautive */
        else
            CHECK(true);
    }
    nh_hud_compose_bottom(out, sizeof out, &d, 60);
    CHECK(has(out, "…"));
    CHECK(!has(out, "neuralsync"));
    nh_hud_compose_bottom(out, sizeof out, &d, 220);
    CHECK(has(out, "neuralsync"));
    CHECK(!has(out, "…"));

    /* Dernière commande qui tient pile : pas d'ellipse inutile. */
    static const char *const two[] = {"help", "scan"};
    d = sample("Neo", 0, false, two, 2);
    size_t exact = strlen(" Commandes : help · scan");
    nh_hud_compose_bottom(out, sizeof out, &d, (int)exact);
    CHECK(has(out, "help · scan"));
    CHECK(!has(out, "…"));

    d = sample("Neo", 0, false, NULL, 0);
    nh_hud_compose_bottom(out, sizeof out, &d, 80);
    CHECK(has(out, "Commandes :"));
    CHECK_INT(nh_display_width(out), 80);

    nh_set_lang(NH_LANG_EN);
    d = sample("Neo", 0, false, few, 2);
    nh_hud_compose_bottom(out, sizeof out, &d, 80);
    CHECK(has(out, "Commands:"));
    nh_set_lang(NH_LANG_FR);

    char tiny[16];
    nh_hud_compose_bottom(tiny, sizeof tiny, &d, 100);
    CHECK(strlen(tiny) < sizeof tiny);
    CHECK_INT(nh_hud_compose_bottom(out, 0, &d, 80), 0);
}

static void test_inactive_is_silent(void)
{
    /* Hors terminal, aucune fonction du HUD n'écrit ni ne plante. */
    CHECK(!nh_hud_active());
    NhHudData d = sample("Neo", 0, false, NULL, 0);
    nh_hud_refresh(&d);
    nh_hud_clear_body();
    nh_hud_stop();
    CHECK(!nh_hud_start(true));
    CHECK(!nh_hud_active());
}

int main(void)
{
    test_should_enable();
    test_truncate();
    test_gauge();
    test_top_bar();
    test_bottom_bar();
    test_inactive_is_silent();
    return NH_TEST_REPORT("hud");
}
