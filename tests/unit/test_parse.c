#include "nh_test.h"

#include "../../src/core/io.h"
#include "../../src/core/parse.h"
#include "../../src/core/utf8.h"

static void test_split(void)
{
    char cmd[32], arg[64];

    CHECK(nh_split_command("scan", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(cmd, "scan");
    CHECK_STR(arg, "");

    CHECK(nh_split_command("bruteforce localhost", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(cmd, "bruteforce");
    CHECK_STR(arg, "localhost");

    /* Espaces autour ignorés, espaces internes de l'argument conservés. */
    CHECK(nh_split_command("   decrypt \t  WKLV  LV  A  \r\n", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(cmd, "decrypt");
    CHECK_STR(arg, "WKLV  LV  A");

    CHECK(nh_split_command("SCAN", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(cmd, "SCAN"); /* la casse est conservée : la comparaison l'ignore ensuite */

    CHECK(!nh_split_command("", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(cmd, "");
    CHECK(!nh_split_command("   \t ", cmd, sizeof cmd, arg, sizeof arg));
    CHECK(!nh_split_command(NULL, cmd, sizeof cmd, arg, sizeof arg));
    CHECK(!nh_split_command("scan", cmd, 0, arg, sizeof arg));
}

static void test_split_truncation(void)
{
    char cmd[8], arg[8];

    /* Commande trop longue : tronquée, sans dépassement. */
    CHECK(nh_split_command("abcdefghijklmnop x", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(cmd, "abcdefg");
    CHECK_STR(arg, "x");

    /* Argument trop long : tronqué. */
    CHECK(nh_split_command("go 0123456789", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(arg, "0123456");

    /* Jamais au milieu d'un caractère UTF-8 : "é" = 2 octets, 7 octets utiles. */
    CHECK(nh_split_command("go aaaaaaé", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(arg, "aaaaaa"); /* le é (octets 7-8) ne tient pas, il est retiré entier */
    CHECK(nh_split_command("go aaaaaé", cmd, sizeof cmd, arg, sizeof arg));
    CHECK_STR(arg, "aaaaaé"); /* 5 + 2 = 7 octets : tient pile */
}

static void test_nocase(void)
{
    CHECK(nh_str_eq_nocase("scan", "SCAN"));
    CHECK(nh_str_eq_nocase("ScAn", "sCaN"));
    CHECK(nh_str_eq_nocase("", ""));
    CHECK(!nh_str_eq_nocase("scan", "scans"));
    CHECK(!nh_str_eq_nocase("scans", "scan"));
    CHECK(!nh_str_eq_nocase("scan", "sc4n"));
    CHECK(nh_str_eq_nocase("upload_virus", "UPLOAD_VIRUS"));
}

static void test_prefix_nocase(void)
{
    CHECK(nh_str_has_prefix_nocase("scan", "sc"));
    CHECK(nh_str_has_prefix_nocase("scan", "SC"));
    CHECK(nh_str_has_prefix_nocase("ScAn", "sCaN"));
    CHECK(nh_str_has_prefix_nocase("scan", "")); /* un préfixe vide convient à tout */
    CHECK(nh_str_has_prefix_nocase("", ""));
    CHECK(!nh_str_has_prefix_nocase("scan", "scans")); /* préfixe plus long que le mot */
    CHECK(!nh_str_has_prefix_nocase("", "a"));
    CHECK(!nh_str_has_prefix_nocase("scan", "sh"));
    CHECK(nh_str_has_prefix_nocase("R4Z0R", "r4z"));
    CHECK(nh_str_has_prefix_nocase("\xC3\xA9t\xC3\xA9", "\xC3\xA9")); /* UTF-8 : octet par octet, sans changer la casse */
}

static void test_trim_incomplete(void)
{
    char a[] = "abc";
    nh_utf8_trim_incomplete(a);
    CHECK_STR(a, "abc");

    char b[] = "ab\xC3"; /* tête de "é" sans sa suite */
    nh_utf8_trim_incomplete(b);
    CHECK_STR(b, "ab");

    char c[] = "ab\xC3\xA9"; /* "é" complet : intact */
    nh_utf8_trim_incomplete(c);
    CHECK_STR(c, "ab\xC3\xA9");

    char d[] = "x\xE2\x9C"; /* "✅" (3 octets) coupé après 2 */
    nh_utf8_trim_incomplete(d);
    CHECK_STR(d, "x");

    char e[] = "x\xF0\x9F\x9A"; /* emoji 4 octets coupé après 3 */
    nh_utf8_trim_incomplete(e);
    CHECK_STR(e, "x");

    char f[] = "";
    nh_utf8_trim_incomplete(f);
    CHECK_STR(f, "");
}

static void test_read_line_truncates_on_char_boundary(void)
{
    char buf[5]; /* 4 octets utiles */
    FILE *fp = tmpfile();
    fputs("abcéfg\nok\n", fp); /* a b c + é (2 octets) : le é ne tient pas entier */
    rewind(fp);
    nh_io_set_input(fp);

    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "abc"); /* pas de demi-caractère */
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "ok");

    nh_io_set_input(NULL);
    fclose(fp);
}

static void test_clean_name(void)
{
    char out[64];

    CHECK_INT(nh_clean_name("Case", out, sizeof out, NH_NAME_MAX_CHARS), 4);
    CHECK_STR(out, "Case");

    /* rien de présentable : longueur 0, sortie vide (l'appelant prend « Case ») */
    CHECK_INT(nh_clean_name("", out, sizeof out, NH_NAME_MAX_CHARS), 0);
    CHECK_STR(out, "");
    CHECK_INT(nh_clean_name("   \t  ", out, sizeof out, NH_NAME_MAX_CHARS), 0);
    CHECK_INT(nh_clean_name("\x01\x02\x7f", out, sizeof out, NH_NAME_MAX_CHARS), 0);
    CHECK_INT(nh_clean_name(NULL, out, sizeof out, NH_NAME_MAX_CHARS), 0);
    CHECK_STR(out, "");

    /* espaces : bords supprimés, répétitions réduites, tabulation = espace */
    nh_clean_name("  Jean \t  Luc  ", out, sizeof out, NH_NAME_MAX_CHARS);
    CHECK_STR(out, "Jean Luc");

    /* contrôles C0/DEL/C1 supprimés : jamais d'échappement ANSI dans un nom */
    nh_clean_name("\x1b[31mRed\x07\x7f", out, sizeof out, NH_NAME_MAX_CHARS);
    CHECK_STR(out, "[31mRed");
    nh_clean_name("A\xc2\x9b" "31mB", out, sizeof out, NH_NAME_MAX_CHARS); /* CSI codé sur un caractère C1 */
    CHECK_STR(out, "A31mB");

    /* UTF-8 valide conservé, octets invalides écartés */
    nh_clean_name("Zoë ☃", out, sizeof out, NH_NAME_MAX_CHARS);
    CHECK_STR(out, "Zoë ☃");
    nh_clean_name("a\xff" "b\xc3" "c", out, sizeof out, NH_NAME_MAX_CHARS);
    CHECK_STR(out, "abc");

    /* limite en CARACTÈRES, pas en octets, sans couper un caractère en deux */
    CHECK_INT(nh_clean_name("abcdefghij", out, sizeof out, 5), 5);
    CHECK_STR(out, "abcde");
    CHECK_INT(nh_clean_name("ééééééé", out, sizeof out, 3), 6);
    CHECK_STR(out, "ééé");

    /* la limite compte les espaces conservés, et ne laisse pas d'espace final */
    nh_clean_name("ab cd", out, sizeof out, 3);
    CHECK_STR(out, "ab");
    nh_clean_name("ab cd", out, sizeof out, 4);
    CHECK_STR(out, "ab c");

    /* tampon de sortie trop petit : tronqué proprement, toujours terminé */
    char tiny[5];
    CHECK_INT(nh_clean_name("abcdefgh", tiny, sizeof tiny, 20), 4);
    CHECK_STR(tiny, "abcd");
    char tiny2[4];
    nh_clean_name("aéé", tiny2, sizeof tiny2, 20); /* 3 octets utiles : « a » et un seul « é » */
    CHECK_STR(tiny2, "aé");
    CHECK_INT(nh_clean_name("x", tiny2, 0, 20), 0);
}

static void test_parse_yes_no(void)
{
    CHECK(nh_parse_yes_no("o", false));
    CHECK(nh_parse_yes_no("O", false));
    CHECK(nh_parse_yes_no("oui", false));
    CHECK(nh_parse_yes_no("  OUI  ", false));
    CHECK(nh_parse_yes_no("y", false));
    CHECK(nh_parse_yes_no("Yes", false));

    CHECK(!nh_parse_yes_no("n", true));
    CHECK(!nh_parse_yes_no("N", true));
    CHECK(!nh_parse_yes_no("non", true));
    CHECK(!nh_parse_yes_no(" No ", true));

    /* vide ou incompréhensible : la valeur par défaut */
    CHECK(nh_parse_yes_no("", true));
    CHECK(!nh_parse_yes_no("", false));
    CHECK(nh_parse_yes_no("   ", true));
    CHECK(nh_parse_yes_no("peut-être", true));
    CHECK(!nh_parse_yes_no("peut-être", false));
    CHECK(!nh_parse_yes_no("ouiii", false));
    CHECK(nh_parse_yes_no(NULL, true));
    CHECK(!nh_parse_yes_no("un très long texte sans rapport", false));
}

int main(void)
{
    test_split();
    test_split_truncation();
    test_nocase();
    test_prefix_nocase();
    test_trim_incomplete();
    test_read_line_truncates_on_char_boundary();
    test_clean_name();
    test_parse_yes_no();
    return NH_TEST_REPORT("parse");
}
