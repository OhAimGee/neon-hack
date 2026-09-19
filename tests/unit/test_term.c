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

static void wrap(const char *text, size_t start_col, size_t indent, size_t width, char *out, size_t size)
{
    size_t n = nh_wrap_text(out, size, text, start_col, indent, width);
    CHECK_INT(n, strlen(out)); /* la longueur retournée est celle du texte écrit */
}

static void test_wrap(void)
{
    char out[512];

    /* width 0 : aucune coupure, le texte est repris tel quel (espaces normalisés) */
    wrap("un deux trois quatre", 0, 4, 0, out, sizeof out);
    CHECK_STR(out, "un deux trois quatre");

    /* coupure aux espaces, retrait sur les lignes suivantes */
    wrap("aaa bbb ccc ddd", 0, 2, 7, out, sizeof out);
    CHECK_STR(out, "aaa bbb\n  ccc\n  ddd");
    wrap("aaa bbb ccc ddd", 0, 2, 9, out, sizeof out);
    CHECK_STR(out, "aaa bbb\n  ccc ddd"); /* le retrait compte dans la largeur, la limite est incluse */

    /* préfixe déjà affiché sur la première ligne */
    wrap("aaa bbb ccc", 4, 4, 11, out, sizeof out);
    CHECK_STR(out, "aaa bbb\n    ccc");

    /* un mot plus long que la ligne reste entier sur la sienne */
    wrap("court démesurément court", 0, 0, 6, out, sizeof out);
    CHECK_STR(out, "court\ndémesurément\ncourt");

    /* les "\n" du texte sont conservés, le retrait s'applique après eux */
    wrap("ligne un\nligne deux", 0, 3, 40, out, sizeof out);
    CHECK_STR(out, "ligne un\n   ligne deux");

    /* largeur mesurée en colonnes : les accents comptent pour 1, les emojis larges pour 2 */
    wrap("éé éé éé", 0, 0, 5, out, sizeof out);
    CHECK_STR(out, "éé éé\néé");
    wrap("😀 😀 😀", 0, 0, 5, out, sizeof out);
    CHECK_STR(out, "😀 😀\n😀");

    /* espaces multiples et texte vide */
    wrap("  a    b  ", 0, 0, 0, out, sizeof out);
    CHECK_STR(out, "a b");
    wrap("", 0, 0, 10, out, sizeof out);
    CHECK_STR(out, "");

    /* jamais de ligne plus large que `width` quand chaque mot tient */
    const char *long_text = "Voici comment ça marche, rookie : tout se fait pas à pas. Tape 'quests' pour ouvrir ton journal de mission.";
    wrap(long_text, 9, 9, 40, out, sizeof out);
    for (const char *line = out; line != NULL && *line != '\0';)
    {
        const char *eol = strchr(line, '\n');
        char one[256];
        size_t n = eol != NULL ? (size_t)(eol - line) : strlen(line);
        memcpy(one, line, n);
        one[n] = '\0';
        size_t first = line == out ? 9 : 0; /* la première ligne porte déjà le préfixe */
        CHECK(first + nh_display_width(one) <= 40);
        line = eol != NULL ? eol + 1 : NULL;
    }
}

static void test_wrap_truncation(void)
{
    char tiny[8];
    size_t n = nh_wrap_text(tiny, sizeof tiny, "abcdefghijklmnop", 0, 0, 0);
    CHECK_INT(strlen(tiny), n);
    CHECK(n < sizeof tiny); /* tronqué, terminé */
    CHECK_INT(nh_wrap_text(tiny, 0, "abc", 0, 0, 0), 0);

    /* un caractère UTF-8 n'est jamais coupé en deux par la troncature */
    char small[6];
    nh_wrap_text(small, sizeof small, "ééééé", 0, 0, 0);
    CHECK(strlen(small) % 2 == 0);
    CHECK(strncmp(small, "éé", 4) == 0);
}

static void test_wrap_width_without_terminal(void)
{
    /* sous les tests, la sortie est redirigée : pas de coupure, les sorties restent sur une ligne */
    CHECK_INT(nh_wrap_width(), 0);
}

int main(void)
{
    test_width();
    test_pad();
    test_colors();
    test_wrap();
    test_wrap_truncation();
    test_wrap_width_without_terminal();
    return NH_TEST_REPORT("term");
}
