#include "nh_test.h"

#include "../../src/core/kv.h"

#include <stdlib.h>

static NhKv parse(const char *text)
{
    NhKv kv;
    CHECK(nh_kv_parse(&kv, text, strlen(text)));
    return kv;
}

static void test_parse_basic(void)
{
    NhKv kv = parse("a=1\nb=deux\n");
    CHECK_INT(kv.count, 2);
    CHECK_STR(nh_kv_get(&kv, "a"), "1");
    CHECK_STR(nh_kv_get(&kv, "b"), "deux");
    CHECK(nh_kv_get(&kv, "c") == NULL);
    nh_kv_free(&kv);
}

static void test_parse_tolerance(void)
{
    /* commentaires, lignes vides, lignes sans '=', espaces autour des clés, CRLF, dernière ligne sans \n */
    NhKv kv = parse("# commentaire\n; autre\n\n  cle  =  valeur avec espaces  \r\nsans_egal\n=orpheline\nx=1\r\nfin=ok");
    CHECK_INT(kv.count, 3);
    CHECK_STR(nh_kv_get(&kv, "cle"), "  valeur avec espaces  ");
    CHECK_STR(nh_kv_get(&kv, "x"), "1");
    CHECK_STR(nh_kv_get(&kv, "fin"), "ok");
    CHECK(nh_kv_get(&kv, "sans_egal") == NULL);
    CHECK(nh_kv_get(&kv, "#") == NULL);
    nh_kv_free(&kv);
}

static void test_parse_value_keeps_equals(void)
{
    NhKv kv = parse("k=a=b=c\nvide=\n");
    CHECK_STR(nh_kv_get(&kv, "k"), "a=b=c");
    CHECK_STR(nh_kv_get(&kv, "vide"), "");
    nh_kv_free(&kv);
}

static void test_parse_last_wins(void)
{
    NhKv kv = parse("a=1\na=2\n");
    CHECK_INT(kv.count, 2);
    CHECK_STR(nh_kv_get(&kv, "a"), "2");
    nh_kv_free(&kv);
}

static void test_parse_empty_and_binary(void)
{
    NhKv kv = parse("");
    CHECK_INT(kv.count, 0);
    CHECK(nh_kv_get(&kv, "a") == NULL);
    nh_kv_free(&kv);

    /* un octet nul au milieu ne doit pas faire lire au-delà du tampon */
    const char raw[] = {'a', '=', '1', '\0', 'b', '=', '2', '\n'};
    CHECK(nh_kv_parse(&kv, raw, sizeof raw));
    CHECK_STR(nh_kv_get(&kv, "a"), "1");
    nh_kv_free(&kv);
}

static void test_parse_too_many_pairs(void)
{
    size_t lines = 9000;
    char *text = malloc(lines * 12);
    size_t len = 0;
    for (size_t i = 0; i < lines; i++)
        len += (size_t)sprintf(text + len, "k%zu=1\n", i);
    NhKv kv;
    CHECK(!nh_kv_parse(&kv, text, len));
    CHECK(kv.pairs == NULL && kv.text == NULL); /* rien à libérer côté appelant */
    free(text);
}

static void test_int(void)
{
    NhKv kv = parse("ok=42\nneg=-7\nsp=  5  \nvide=\ntexte=abc\nmix=12x\nhuge=99999999999999999999999\nbas=3\nhaut=200\n");
    long v = -1;

    CHECK_INT(nh_kv_int(&kv, "ok", 0, 100, &v), NH_KV_OK);
    CHECK_INT(v, 42);
    CHECK_INT(nh_kv_int(&kv, "neg", -10, 10, &v), NH_KV_OK);
    CHECK_INT(v, -7);
    CHECK_INT(nh_kv_int(&kv, "sp", 0, 10, &v), NH_KV_OK);
    CHECK_INT(v, 5);

    v = 1234;
    CHECK_INT(nh_kv_int(&kv, "absente", 0, 10, &v), NH_KV_MISSING);
    CHECK_INT(nh_kv_int(&kv, "vide", 0, 10, &v), NH_KV_INVALID);
    CHECK_INT(nh_kv_int(&kv, "texte", 0, 10, &v), NH_KV_INVALID);
    CHECK_INT(nh_kv_int(&kv, "mix", 0, 100, &v), NH_KV_INVALID);
    CHECK_INT(nh_kv_int(&kv, "huge", 0, 1000000, &v), NH_KV_INVALID);
    CHECK_INT(nh_kv_int(&kv, "bas", 5, 10, &v), NH_KV_INVALID);
    CHECK_INT(nh_kv_int(&kv, "haut", 0, 100, &v), NH_KV_INVALID);
    CHECK_INT(v, 1234); /* *out n'est jamais touché en cas d'échec */

    /* les bornes sont incluses */
    CHECK_INT(nh_kv_int(&kv, "bas", 3, 3, &v), NH_KV_OK);
    CHECK_INT(v, 3);
    nh_kv_free(&kv);
}

static void test_writer(void)
{
    NhKvWriter w;
    nh_kvw_init(&w);
    CHECK_STR(nh_kvw_text(&w), ""); /* jamais NULL, même sans rien écrire */

    nh_kvw_int(&w, "n", -12);
    nh_kvw_str(&w, "nom", "Case");
    CHECK_STR(nh_kvw_text(&w), "n=-12\nnom=Case\n");
    CHECK(!w.failed);
    nh_kvw_free(&w);
}

static void test_writer_sanitizes(void)
{
    NhKvWriter w;
    nh_kvw_init(&w);
    /* une fin de ligne dans la valeur ferait apparaître une fausse clé à la relecture */
    nh_kvw_str(&w, "a=b", "x\ny=1\r\tz\x7f");
    NhKv kv = parse(nh_kvw_text(&w));
    CHECK_INT(kv.count, 1);
    CHECK_STR(nh_kv_get(&kv, "a_b"), "x y=1  z ");
    nh_kv_free(&kv);
    nh_kvw_free(&w);
}

static void test_writer_grows_and_roundtrips(void)
{
    NhKvWriter w;
    nh_kvw_init(&w);
    for (int i = 0; i < 3000; i++)
    {
        char key[32];
        snprintf(key, sizeof key, "cle.%d", i);
        nh_kvw_int(&w, key, i * 7L);
    }
    CHECK(!w.failed);

    NhKv kv = parse(nh_kvw_text(&w));
    CHECK_INT(kv.count, 3000);
    long v = 0;
    CHECK_INT(nh_kv_int(&kv, "cle.2999", 0, 1000000, &v), NH_KV_OK);
    CHECK_INT(v, 2999 * 7);
    CHECK_INT(nh_kv_int(&kv, "cle.0", 0, 1000000, &v), NH_KV_OK);
    CHECK_INT(v, 0);
    nh_kv_free(&kv);
    nh_kvw_free(&w);
}

int main(void)
{
    test_parse_basic();
    test_parse_tolerance();
    test_parse_value_keeps_equals();
    test_parse_last_wins();
    test_parse_empty_and_binary();
    test_parse_too_many_pairs();
    test_int();
    test_writer();
    test_writer_sanitizes();
    test_writer_grows_and_roundtrips();
    return NH_TEST_REPORT("kv");
}
