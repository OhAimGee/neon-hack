#include "nh_test.h"

#include "nh_capture.h"

#include "../../src/game/shop.h"
#include "../../src/game/shop_view.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>
#include <string.h>

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static int occurrences(const char *text, const char *needle)
{
    int n = 0;
    for (const char *p = text; (p = strstr(p, needle)) != NULL; p += strlen(needle))
        n++;
    return n;
}

static int count_lines(const char *text)
{
    int n = 0;
    for (const char *p = text; *p; p++)
        if (*p == '\n')
            n++;
    return n;
}

/* Plus grande largeur d'affichage d'une ligne (les séquences de couleur ne comptent pas). */
static int max_width(const char *text)
{
    int widest = 0;
    char line[1024];
    while (*text)
    {
        size_t n = strcspn(text, "\n");
        if (n >= sizeof line)
            n = sizeof line - 1;
        memcpy(line, text, n);
        line[n] = '\0';
        int w = (int)nh_display_width(line);
        if (w > widest)
            widest = w;
        text += n;
        if (*text == '\n')
            text++;
    }
    return widest;
}

/* Le texte de la ligne qui contient `needle` (la première), ou "". */
static const char *line_with(const char *text, const char *needle, char *line, size_t size)
{
    const char *hit = strstr(text, needle);
    line[0] = '\0';
    if (hit == NULL)
        return line;
    const char *start = hit;
    while (start > text && start[-1] != '\n')
        start--;
    size_t n = strcspn(start, "\n");
    if (n >= size)
        n = size - 1;
    memcpy(line, start, n);
    line[n] = '\0';
    return line;
}

static CyberShop new_shop(void)
{
    CyberShop shop;
    init_shop(&shop);
    return shop;
}

static void compose(char *out, size_t size, const CyberShop *shop, int credits, int level, int cols,
                    int max_lines)
{
    nh_shop_compose(out, size, shop, credits, level, cols, max_lines);
}

/* ---- États ----------------------------------------------------------------------------------- */

static void test_state_order(void)
{
    CyberShop shop = new_shop();
    ShopItem *stealth = &shop.items[ITEM_STEALTH_UPGRADE]; /* 150 ¢, niveau 2, unique */

    CHECK_INT(nh_shop_state(stealth, 1000, 5), NH_SHOP_OK);
    CHECK_INT(nh_shop_state(stealth, 100, 5), NH_SHOP_LOW_CREDITS);
    CHECK_INT(nh_shop_state(stealth, 1000, 1), NH_SHOP_LOW_LEVEL);
    CHECK_INT(nh_shop_state(stealth, 100, 1), NH_SHOP_LOW_LEVEL); /* le niveau prime sur les crédits */
    CHECK_INT(nh_shop_state(stealth, 150, 2), NH_SHOP_OK);        /* juste assez */
    stealth->is_available = false;
    CHECK_INT(nh_shop_state(stealth, 1000000, 6), NH_SHOP_SOLD_OUT); /* épuisé prime sur tout */
    CHECK_INT(nh_shop_state(stealth, 0, 1), NH_SHOP_SOLD_OUT);
}

static void test_blurbs(void)
{
    for (int lang = 0; lang < 2; lang++)
    {
        nh_set_lang(lang == 0 ? NH_LANG_FR : NH_LANG_EN);
        for (int i = 0; i < ITEM_COUNT; i++)
        {
            const char *blurb = nh_shop_blurb((ShopItemType)i);
            CHECK(blurb[0] != '\0');
            CHECK((int)nh_display_width(blurb) <= NH_SHOP_BLURB_WIDTH);
            if ((int)nh_display_width(blurb) > NH_SHOP_BLURB_WIDTH)
                fprintf(stderr, "    résumé %d trop long : \"%s\"\n", i, blurb);
            for (int j = 0; j < i; j++) /* deux objets ne partagent pas leur résumé */
                CHECK(strcmp(blurb, nh_shop_blurb((ShopItemType)j)) != 0);
        }
    }
    CHECK_STR(nh_shop_blurb((ShopItemType)-1), "");
    CHECK_STR(nh_shop_blurb(ITEM_COUNT), "");
    nh_set_lang(NH_LANG_FR);
}

/* ---- Le cas qui a motivé la vitrine : 24 lignes avec HUD, 80 colonnes ------------------------ */

static void test_fits_hud_terminal(void)
{
    CyberShop shop = new_shop();
    char out[NH_SHOP_BUFFER];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    /* 24 lignes - 2 barres - 1 ligne de question = 21 lignes disponibles. */
    compose(out, sizeof out, &shop, 5000, 3, 80, 21);
    CHECK(count_lines(out) <= 21);
    CHECK(max_width(out) <= 79);
    for (int i = 0; i < ITEM_COUNT; i++)
    {
        CHECK_INT(occurrences(out, shop.items[i].name), 1);
        CHECK(has(out, nh_shop_blurb((ShopItemType)i))); /* cartes complètes, pas la liste de secours */
        char tag[16];
        snprintf(tag, sizeof tag, "[%02d]", i + 1);
        CHECK_INT(occurrences(out, tag), 1);
    }

    /* Deux colonnes, rangées par colonne : 1 face à 6, 5 face à 10. */
    char line[512];
    line_with(out, "[01]", line, sizeof line);
    CHECK(has(line, "[06]") && strstr(line, "[01]") < strstr(line, "[06]"));
    line_with(out, "[05]", line, sizeof line);
    CHECK(has(line, "[10]"));
    line_with(out, "[02]", line, sizeof line);
    CHECK(has(line, "[07]"));

    /* Une ligne vide avant et après : la vitrine respire, sans pousser le prompt contre le dernier objet. */
    CHECK(out[0] == '\n');
    CHECK(strlen(out) > 2 && out[strlen(out) - 1] == '\n' && out[strlen(out) - 2] == '\n');

    CHECK(has(out, "MARCHÉ NOIR DE R4Z0R"));
    CHECK(has(out, "Crédits : 5000 ¢ · Niveau 3"));
    CHECK(!has(out, "█▀▀")); /* le bandeau est réservé aux écrans hauts */
}

static void test_roomy_when_tall_or_unlimited(void)
{
    CyberShop shop = new_shop();
    char out[NH_SHOP_BUFFER];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    compose(out, sizeof out, &shop, 5000, 3, 80, 0); /* sans limite : sortie redirigée */
    CHECK(has(out, "█▀▀"));
    CHECK(has(out, "Bienvenue dans le marché noir du deep web..."));
    CHECK(max_width(out) <= 79);
    for (int i = 0; i < ITEM_COUNT; i++)
        CHECK_INT(occurrences(out, shop.items[i].name), 1);
    int roomy_lines = count_lines(out);

    compose(out, sizeof out, &shop, 5000, 3, 80, 60);
    CHECK(has(out, "█▀▀"));
    CHECK_INT(count_lines(out), roomy_lines);

    compose(out, sizeof out, &shop, 5000, 3, 80, 21);
    CHECK(!has(out, "█▀▀"));
    CHECK(count_lines(out) < roomy_lines);
}

static void test_list_when_very_short(void)
{
    CyberShop shop = new_shop();
    char out[NH_SHOP_BUFFER];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    /* 13 lignes : la liste (une ligne par objet) tient tout juste, les cartes non. */
    compose(out, sizeof out, &shop, 5000, 3, 80, 13);
    CHECK(count_lines(out) <= 13);
    CHECK(max_width(out) <= 79);
    for (int i = 0; i < ITEM_COUNT; i++)
        CHECK_INT(occurrences(out, shop.items[i].name), 1);
    CHECK(!has(out, nh_shop_blurb(ITEM_STEALTH_UPGRADE))); /* le résumé est sacrifié */
    CHECK(has(out, "unique · disponible") || has(out, "consommable · disponible"));

    /* Même une limite absurde donne la liste, pas un catalogue tronqué. */
    compose(out, sizeof out, &shop, 5000, 3, 80, 3);
    for (int i = 0; i < ITEM_COUNT; i++)
        CHECK_INT(occurrences(out, shop.items[i].name), 1);
    CHECK(count_lines(out) <= 14);
}

/* ---- Statuts affichés -------------------------------------------------------------------------- */

static void test_status_texts(void)
{
    CyberShop shop = new_shop();
    char out[NH_SHOP_BUFFER];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    /* Niveau 1, 100 crédits : Stealth (niv. 2) verrouillé, Ghost Protocol (80 ¢) accessible, Quantum
       Encryption Key (200 ¢, niv. 3) verrouillée par le niveau d'abord, Dark Web VPN (60 ¢) accessible. */
    compose(out, sizeof out, &shop, 100, 1, 80, 21);
    char line[512];
    CHECK(has(line_with(out, "Stealth Module", line, sizeof line), "150 ¢"));
    CHECK(has(out, "unique · niveau 2 requis"));
    CHECK(has(out, "consommable · disponible"));
    CHECK(has(out, "niveau 5 requis")); /* la puce quantique */

    /* Niveau 6, 100 crédits : ce qui est trop cher dit combien il manque. */
    compose(out, sizeof out, &shop, 100, 6, 80, 21);
    CHECK(has(out, "unique · il manque 50 ¢"));       /* Stealth : 150 - 100 */
    CHECK(has(out, "consommable · il manque 20 ¢"));  /* Malware Arsenal : 120 - 100 */
    CHECK(has(out, "unique · il manque 700 ¢"));      /* Quantum Processing Chip : 800 - 100 */

    /* Un objet unique acheté reste à sa place, numéro compris, et se dit épuisé. */
    shop.items[ITEM_STEALTH_UPGRADE].is_available = false;
    compose(out, sizeof out, &shop, 5000, 6, 80, 21);
    CHECK_INT(occurrences(out, "[01]"), 1);
    CHECK(has(out, "unique · épuisé"));
    CHECK(has(line_with(out, "Stealth Module", line, sizeof line), "[01]"));
    CHECK_INT(occurrences(out, "Stealth Module"), 1);

    nh_set_lang(NH_LANG_EN);
    compose(out, sizeof out, &shop, 100, 1, 80, 21);
    CHECK(has(out, "R4Z0R'S BLACK MARKET"));
    CHECK(has(out, "Credits: 100 ¢ · Level 1"));
    CHECK(has(out, "one-time · sold out"));
    CHECK(has(out, "consumable · available"));
    CHECK(has(out, "level 5 required"));
    CHECK(has(out, "Passively lowers alert"));
    CHECK(!has(out, "Crédits"));
    nh_set_lang(NH_LANG_FR);
}

/* ---- Toutes les tailles d'écran ------------------------------------------------------------- */

static void test_every_screen_size(void)
{
    CyberShop shop = new_shop();
    shop.items[ITEM_QUANTUM_CHIP].is_available = false;
    char out[NH_SHOP_BUFFER];
    static const int k_cols[] = {30, 40, 50, 51, 60, 70, 71, 72, 80, 100, 120, 200};
    static const int k_lines[] = {0, 5, 13, 14, 16, 20, 21, 25, 28, 30, 40, 60};

    for (int lang = 0; lang < 2; lang++)
        for (int color = 0; color < 2; color++)
        {
            nh_set_lang(lang == 0 ? NH_LANG_FR : NH_LANG_EN);
            nh_term_set_color(color == 1);
            for (size_t c = 0; c < sizeof k_cols / sizeof k_cols[0]; c++)
                for (size_t l = 0; l < sizeof k_lines / sizeof k_lines[0]; l++)
                {
                    int cols = k_cols[c];
                    int lines = k_lines[l];
                    compose(out, sizeof out, &shop, 250, 3, cols, lines);
                    int limit = cols - 1 > 100 ? 100 : cols - 1;

                    bool ok = max_width(out) <= limit && (lines < 14 || count_lines(out) <= lines) &&
                              out[0] == '\n' && strlen(out) < sizeof out - 1;
                    /* Sous 60 colonnes les noms longs sont abrégés (« … ») : on ne retrouve plus que le numéro. */
                    for (int i = 0; ok && i < ITEM_COUNT; i++)
                    {
                        char tag[16];
                        snprintf(tag, sizeof tag, "[%02d]", i + 1);
                        ok = occurrences(out, tag) == 1 &&
                             (cols < 60 || occurrences(out, shop.items[i].name) == 1);
                    }
                    CHECK(ok);
                    if (!ok)
                        fprintf(stderr, "    lang=%d couleur=%d cols=%d lignes=%d : largeur %d, %d lignes\n", lang,
                                color, cols, lines, max_width(out), count_lines(out));
                }
        }
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);
}

static void test_colors_do_not_change_layout(void)
{
    CyberShop shop = new_shop();
    char plain[NH_SHOP_BUFFER];
    char colored[NH_SHOP_BUFFER];
    nh_set_lang(NH_LANG_FR);

    nh_term_set_color(false);
    compose(plain, sizeof plain, &shop, 250, 3, 80, 21);
    nh_term_set_color(true);
    compose(colored, sizeof colored, &shop, 250, 3, 80, 21);
    nh_term_set_color(false);

    CHECK(has(colored, "\033["));
    CHECK(!has(plain, "\033["));
    CHECK_INT(count_lines(plain), count_lines(colored));
    CHECK_INT(max_width(plain), max_width(colored));
}

/* ---- Robustesse ----------------------------------------------------------------------------- */

static void test_small_buffers(void)
{
    CyberShop shop = new_shop();
    char out[NH_SHOP_BUFFER];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    CHECK_INT(nh_shop_compose(NULL, 100, &shop, 250, 3, 80, 21), 0);
    CHECK_INT(nh_shop_compose(out, 0, &shop, 250, 3, 80, 21), 0);

    /* Tampon trop petit : tronqué, terminé, jamais au milieu d'un caractère UTF-8. */
    for (size_t size = 1; size < 400; size += 7)
    {
        memset(out, 'X', sizeof out);
        size_t n = nh_shop_compose(out, size, &shop, 250, 3, 80, 21);
        CHECK(n < size);
        CHECK_INT(strlen(out), n);
        bool clean = true;
        for (size_t i = 0; i < n; i++)
        {
            unsigned char ch = (unsigned char)out[i];
            if (ch >= 0xC0)
            { /* octet de tête : il doit être suivi du bon nombre d'octets de continuation */
                size_t need = ch >= 0xF0 ? 3 : ch >= 0xE0 ? 2 : 1;
                for (size_t k = 1; k <= need; k++)
                    if (i + k >= n || ((unsigned char)out[i + k] & 0xC0) != 0x80)
                        clean = false;
            }
        }
        CHECK(clean);
    }

    /* Une boutique dont item_count est aberrant n'écrit rien hors du catalogue. */
    shop.item_count = 999;
    compose(out, sizeof out, &shop, 250, 3, 80, 0);
    CHECK_INT(occurrences(out, "[10]"), 1);
    CHECK_INT(occurrences(out, "[11]"), 0);
    shop.item_count = -4;
    compose(out, sizeof out, &shop, 250, 3, 80, 0);
    CHECK_INT(occurrences(out, "[01]"), 0);
    shop.item_count = 7; /* nombre impair : la dernière rangée n'a pas de case à droite */
    compose(out, sizeof out, &shop, 250, 3, 80, 21);
    CHECK_INT(occurrences(out, "[07]"), 1);
    CHECK_INT(occurrences(out, "[08]"), 0);
    CHECK(max_width(out) <= 79);
}

static void test_show_on_redirected_output(void)
{
    /* Ici stdout est un fichier, pas un terminal : 80 colonnes, sans limite de hauteur. */
    CyberShop shop = new_shop();
    char out[NH_SHOP_BUFFER];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    NhCapture cap = nh_capture_begin();
    nh_shop_show(&shop, 250, 3);
    nh_capture_end(&cap, out, sizeof out);

    CHECK(has(out, "MARCHÉ NOIR DE R4Z0R"));
    CHECK(has(out, "█▀▀"));
    for (int i = 0; i < ITEM_COUNT; i++)
        CHECK_INT(occurrences(out, shop.items[i].name), 1);
    CHECK(max_width(out) <= 79);
}

int main(void)
{
    test_state_order();
    test_blurbs();
    test_fits_hud_terminal();
    test_roomy_when_tall_or_unlimited();
    test_list_when_very_short();
    test_status_texts();
    test_every_screen_size();
    test_colors_do_not_change_layout();
    test_small_buffers();
    test_show_on_redirected_output();
    return NH_TEST_REPORT("shop_view");
}
