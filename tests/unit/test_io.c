#include "nh_test.h"

#include "../../src/core/io.h"

/* Fichier temporaire contenant `data`, positionné au début. */
static FILE *feed(const char *data)
{
    FILE *f = tmpfile();
    if (f == NULL)
    {
        perror("tmpfile");
        return NULL;
    }
    fputs(data, f);
    rewind(f);
    nh_io_set_input(f);
    return f;
}

static void test_lines(void)
{
    char buf[32];
    FILE *f = feed("abc\ndef\n\nlast");

    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "abc");
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "def");
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, ""); /* ligne vide != EOF */
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "last"); /* dernière ligne sans saut de ligne */
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_EOF);
    CHECK_STR(buf, "");
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_EOF); /* EOF reste EOF */
    fclose(f);
}

static void test_crlf(void)
{
    char buf[32];
    FILE *f = feed("abc\r\nxyz\r\n");
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "abc");
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "xyz");
    fclose(f);
}

static void test_empty_input(void)
{
    char buf[8] = "junk";
    FILE *f = feed("");
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_EOF);
    CHECK_STR(buf, "");
    fclose(f);
}

static void test_overlong_line_does_not_leak(void)
{
    char buf[5];
    FILE *f = feed("abcdefghij\nnext\nabcd\nafter\n");

    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "abcd"); /* tronquée */
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "next"); /* la fin de la ligne précédente a été jetée */
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "abcd"); /* pile la taille max */
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_OK);
    CHECK_STR(buf, "afte"); /* 5 caractères "after" -> tronquée */
    CHECK_INT(nh_read_line(buf, sizeof buf), NH_IO_EOF);
    fclose(f);
}

static void test_parse_int(void)
{
    int v = -1;
    CHECK(nh_parse_int("42", 0, 100, &v) && v == 42);
    CHECK(nh_parse_int("  7  ", 0, 100, &v) && v == 7);
    CHECK(nh_parse_int("-3", -5, 5, &v) && v == -3);
    CHECK(nh_parse_int("+8", 0, 10, &v) && v == 8);

    v = 99;
    CHECK(!nh_parse_int("", 0, 100, &v));
    CHECK(!nh_parse_int("   ", 0, 100, &v));
    CHECK(!nh_parse_int("abc", 0, 100, &v));
    CHECK(!nh_parse_int("12abc", 0, 100, &v));
    CHECK(!nh_parse_int("1 2", 0, 100, &v));
    CHECK(!nh_parse_int("3.5", 0, 100, &v));
    CHECK(!nh_parse_int("0", 1, 5, &v));
    CHECK(!nh_parse_int("6", 1, 5, &v));
    CHECK(!nh_parse_int("99999999999999999999", 0, 100, &v));
    CHECK(!nh_parse_int(NULL, 0, 100, &v));
    CHECK_INT(v, 99); /* jamais modifié en cas d'échec */
}

static void test_read_int(void)
{
    int v = -1;
    FILE *f = feed("abc\n3\n9\n");

    CHECK_INT(nh_read_int(&v, 1, 5), NH_IO_INVALID);
    CHECK_INT(v, -1);
    CHECK_INT(nh_read_int(&v, 1, 5), NH_IO_OK);
    CHECK_INT(v, 3);
    CHECK_INT(nh_read_int(&v, 1, 5), NH_IO_INVALID); /* hors plage */
    CHECK_INT(v, 3);
    CHECK_INT(nh_read_int(&v, 1, 5), NH_IO_EOF);
    fclose(f);
}

int main(void)
{
    test_lines();
    test_crlf();
    test_empty_input();
    test_overlong_line_does_not_leak();
    test_parse_int();
    test_read_int();
    nh_io_set_input(NULL);
    return NH_TEST_REPORT("io");
}
