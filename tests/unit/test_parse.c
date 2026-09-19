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

int main(void)
{
    test_split();
    test_split_truncation();
    test_nocase();
    test_trim_incomplete();
    test_read_line_truncates_on_char_boundary();
    return NH_TEST_REPORT("parse");
}
