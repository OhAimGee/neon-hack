#include "nh_test.h"

#include "../../src/ui/term.h"

static void test_width(void)
{
    CHECK_INT(nh_display_width(""), 0);
    CHECK_INT(nh_display_width("abc"), 3);
    CHECK_INT(nh_display_width("é"), 1);                /* 2 octets, 1 colonne */
    CHECK_INT(nh_display_width("Sécurité élevée"), 15); /* les accents ne décalent rien */
    CHECK_INT(nh_display_width("e\xCC\x81"), 1);        /* e + accent combinant */
    CHECK_INT(nh_display_width("日本語"), 6);           /* caractères larges */
    CHECK_INT(nh_display_width("✅"), 2);               /* emoji : 2 colonnes */
    CHECK_INT(nh_display_width("🚨 ALERTE"), 9);
    CHECK_INT(nh_display_width("═══"), 3);              /* traits de boîte : 1 colonne */
    CHECK_INT(nh_display_width("█▓▒░"), 4);
    CHECK_INT(nh_display_width("\033[31mrouge\033[0m"), 5); /* ANSI ignoré */
    CHECK_INT(nh_display_width("\033[1;38;5;208mx\033[0m"), 1);
    CHECK_INT(nh_display_width("a\033[31"), 4); /* séquence tronquée : traitée comme texte */
    CHECK_INT(nh_display_width("\xFF\xFEok"), 4); /* octets invalides : 1 colonne chacun */
}

static void test_pad(void)
{
    char out[64];

    nh_pad(out, sizeof out, "ab", 5, NH_ALIGN_LEFT);
    CHECK_STR(out, "ab   ");
    nh_pad(out, sizeof out, "ab", 5, NH_ALIGN_RIGHT);
    CHECK_STR(out, "   ab");
    nh_pad(out, sizeof out, "ab", 5, NH_ALIGN_CENTER);
    CHECK_STR(out, " ab  ");

    /* Le remplissage se règle sur les colonnes, pas sur les octets. */
    nh_pad(out, sizeof out, "élevée", 8, NH_ALIGN_LEFT);
    CHECK_STR(out, "élevée  ");
    CHECK_INT(nh_display_width(out), 8);
    nh_pad(out, sizeof out, "✅ OK", 8, NH_ALIGN_LEFT);
    CHECK_INT(nh_display_width(out), 8);

    /* Texte plus large que la cible : jamais tronqué. */
    nh_pad(out, sizeof out, "longtexte", 3, NH_ALIGN_LEFT);
    CHECK_STR(out, "longtexte");

    /* Petit tampon : coupe sans déborder, toujours terminé par '\0'. */
    char small[4];
    size_t n = nh_pad(small, sizeof small, "abcdef", 10, NH_ALIGN_LEFT);
    CHECK_INT(n, 3);
    CHECK_STR(small, "abc");
    CHECK_INT(nh_pad(small, 0, "x", 1, NH_ALIGN_LEFT), 0);
}

static void test_colors(void)
{
    nh_term_set_color(true);
    CHECK(nh_term_color_enabled());
    CHECK_STR(nh_c(NH_C_RED), "\033[31m");
    CHECK_STR(nh_c(NH_C_RESET), "\033[0m");

    nh_term_set_color(false);
    CHECK(!nh_term_color_enabled());
    CHECK_STR(nh_c(NH_C_RED), "");
    CHECK_STR(nh_c(NH_C_RESET), "");
    CHECK_STR(nh_c(NH_C_COUNT), ""); /* valeur hors plage : pas de crash */

    nh_term_set_color(true);
}

int main(void)
{
    test_width();
    test_pad();
    test_colors();
    return NH_TEST_REPORT("term");
}
