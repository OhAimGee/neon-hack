#include "nh_test.h"

#include "../../src/core/config.h"

static NhCfgResult parse(NhConfig *cfg, int argc, char **argv, char *err, size_t err_size, FILE *out)
{
    nh_config_defaults(cfg, "en_US.UTF-8", NULL);
    return nh_config_parse(argc, argv, cfg, err, err_size, out);
}

static void test_locale_and_env(void)
{
    CHECK_INT(nh_lang_from_locale("fr_FR.UTF-8"), NH_LANG_FR);
    CHECK_INT(nh_lang_from_locale("fr"), NH_LANG_FR);
    CHECK_INT(nh_lang_from_locale("FR_ca"), NH_LANG_FR);
    CHECK_INT(nh_lang_from_locale("en_US.UTF-8"), NH_LANG_EN);
    CHECK_INT(nh_lang_from_locale("de_DE"), NH_LANG_EN);
    CHECK_INT(nh_lang_from_locale("C"), NH_LANG_EN);
    CHECK_INT(nh_lang_from_locale(""), NH_LANG_EN);
    CHECK_INT(nh_lang_from_locale(NULL), NH_LANG_EN);

    NhConfig cfg;
    nh_config_defaults(&cfg, "fr_FR.UTF-8", NULL);
    CHECK_INT(cfg.lang, NH_LANG_FR);
    CHECK(cfg.color);
    CHECK(!cfg.has_seed);
    CHECK(!cfg.fast);
    CHECK(!cfg.new_game);
    CHECK(cfg.hud); /* interface fixe activée par défaut (si le terminal le permet) */

    nh_config_defaults(&cfg, NULL, "1");
    CHECK(!cfg.color); /* NO_COLOR défini et non vide */
    nh_config_defaults(&cfg, NULL, "");
    CHECK(cfg.color); /* NO_COLOR vide = ignoré, comme le veut la convention */
}

static void test_valid_options(void)
{
    NhConfig cfg;
    char err[128];

    char *a1[] = {"neon_hack", "--lang", "fr", "--seed", "42", "--fast", "--new", "--no-color"};
    CHECK_INT(parse(&cfg, 8, a1, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK_INT(cfg.lang, NH_LANG_FR);
    CHECK(cfg.has_seed && cfg.seed == 42u);
    CHECK(cfg.fast && cfg.new_game && !cfg.color);

    char *a2[] = {"neon_hack", "--lang=en", "--seed=7", "--color"};
    CHECK_INT(parse(&cfg, 4, a2, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK_INT(cfg.lang, NH_LANG_EN);
    CHECK(cfg.has_seed && cfg.seed == 7u);
    CHECK(cfg.color);

    char *a3[] = {"neon_hack", "--seed", "0"};
    CHECK_INT(parse(&cfg, 3, a3, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(cfg.has_seed && cfg.seed == 0u); /* la graine 0 est valide */

    char *a4[] = {"neon_hack", "--no-color", "--color"};
    CHECK_INT(parse(&cfg, 3, a4, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(cfg.color); /* la dernière option gagne */

    char *a5[] = {"neon_hack"};
    CHECK_INT(parse(&cfg, 1, a5, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK_STR(err, "");
    CHECK(cfg.hud);

    char *a6[] = {"neon_hack", "--no-hud"};
    CHECK_INT(parse(&cfg, 2, a6, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(!cfg.hud);
}

static void expect_error(char **argv, int argc, const char *needle)
{
    NhConfig cfg;
    char err[128];
    CHECK_INT(parse(&cfg, argc, argv, err, sizeof err, stdout), NH_CFG_ERROR);
    CHECK(strstr(err, needle) != NULL);
    if (strstr(err, needle) == NULL)
        fprintf(stderr, "    message obtenu: \"%s\"\n", err);
}

static void test_errors(void)
{
    char *e1[] = {"neon_hack", "--lang", "de"};
    expect_error(e1, 3, "--lang");
    char *e2[] = {"neon_hack", "--lang"};
    expect_error(e2, 2, "--lang");
    char *e3[] = {"neon_hack", "--seed", "abc"};
    expect_error(e3, 3, "--seed");
    char *e4[] = {"neon_hack", "--seed", "-1"};
    expect_error(e4, 3, "--seed");
    char *e5[] = {"neon_hack", "--seed"};
    expect_error(e5, 2, "--seed");
    char *e6[] = {"neon_hack", "--seed", "99999999999999999999999"};
    expect_error(e6, 3, "--seed");
    char *e7[] = {"neon_hack", "--bogus"};
    expect_error(e7, 2, "--bogus");
    char *e8[] = {"neon_hack", "--languages", "fr"}; /* pas un préfixe valide de --lang */
    expect_error(e8, 3, "unknown option");
    char *e9[] = {"neon_hack", "positional"};
    expect_error(e9, 2, "positional");
}

/* Lit ce qui a été écrit dans un fichier temporaire. */
static void slurp(FILE *f, char *buf, size_t size)
{
    rewind(f);
    size_t n = fread(buf, 1, size - 1, f);
    buf[n] = '\0';
}

static void test_help_and_version(void)
{
    NhConfig cfg;
    char err[64];
    char text[2048];

    FILE *out = tmpfile();
    char *h[] = {"neon_hack", "--help"};
    CHECK_INT(parse(&cfg, 2, h, err, sizeof err, out), NH_CFG_EXIT_OK);
    slurp(out, text, sizeof text);
    CHECK(strstr(text, "Usage:") != NULL);
    CHECK(strstr(text, "--seed") != NULL);
    CHECK(strstr(text, "--data-dir") != NULL);
    CHECK(strstr(text, "--new") != NULL);
    fclose(out);

    out = tmpfile();
    char *v[] = {"neon_hack", "--fast", "-V"};
    CHECK_INT(parse(&cfg, 3, v, err, sizeof err, out), NH_CFG_EXIT_OK);
    slurp(out, text, sizeof text);
    CHECK(strstr(text, "neon_hack ") != NULL);
    CHECK(strstr(text, NH_VERSION) != NULL);
    fclose(out);
}

static void test_data_dir(void)
{
    NhConfig cfg;
    char err[128];

    nh_config_defaults(&cfg, "en_US.UTF-8", NULL);
    CHECK_STR(cfg.data_dir, ""); /* par défaut : dossier de l'utilisateur */

    char *a1[] = {"neon_hack", "--data-dir", "/tmp/nh-données"};
    CHECK_INT(parse(&cfg, 3, a1, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK_STR(cfg.data_dir, "/tmp/nh-données");

    char *a2[] = {"neon_hack", "--data-dir=/un dossier/avec espaces"};
    CHECK_INT(parse(&cfg, 2, a2, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK_STR(cfg.data_dir, "/un dossier/avec espaces");

    char *e1[] = {"neon_hack", "--data-dir"};
    expect_error(e1, 2, "--data-dir");
    char *e2[] = {"neon_hack", "--data-dir="};
    expect_error(e2, 2, "--data-dir");
    char *e3[] = {"neon_hack", "--data-dir", ""};
    expect_error(e3, 3, "--data-dir");
    char *e4[] = {"neon_hack", "--data-dirs", "x"};
    expect_error(e4, 3, "unknown option");

    /* un chemin trop long pour le tampon est refusé au lieu d'être tronqué */
    char too_long[600];
    memset(too_long, 'a', sizeof too_long);
    too_long[sizeof too_long - 1] = '\0';
    char *e5[] = {"neon_hack", "--data-dir", too_long};
    expect_error(e5, 3, "--data-dir");
}

static void test_explicit_flags_are_tracked(void)
{
    NhConfig cfg;
    char err[128];

    /* rien en ligne de commande : rien n'est « imposé », les réglages enregistrés s'appliqueront */
    char *none[] = {"neon_hack"};
    CHECK_INT(parse(&cfg, 1, none, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(!cfg.lang_set && !cfg.color_set && !cfg.fast_set && !cfg.hud_set);

    char *lang[] = {"neon_hack", "--lang", "fr"};
    CHECK_INT(parse(&cfg, 3, lang, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(cfg.lang_set && !cfg.color_set && !cfg.fast_set && !cfg.hud_set);

    char *color[] = {"neon_hack", "--no-color"};
    CHECK_INT(parse(&cfg, 2, color, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(cfg.color_set && !cfg.color);
    char *color2[] = {"neon_hack", "--color"};
    CHECK_INT(parse(&cfg, 2, color2, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(cfg.color_set && cfg.color);

    char *fast[] = {"neon_hack", "--fast"};
    CHECK_INT(parse(&cfg, 2, fast, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(cfg.fast_set && cfg.fast);

    char *hud[] = {"neon_hack", "--no-hud"};
    CHECK_INT(parse(&cfg, 2, hud, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(cfg.hud_set && !cfg.hud);

    /* ces options ne touchent à aucun réglage persistant */
    char *other[] = {"neon_hack", "--seed", "3", "--new", "--data-dir", "/tmp/x"};
    CHECK_INT(parse(&cfg, 6, other, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(!cfg.lang_set && !cfg.color_set && !cfg.fast_set && !cfg.hud_set);
    CHECK(cfg.new_game);
}

static void test_new_game_default_off(void)
{
    NhConfig cfg;
    char err[128];
    char *none[] = {"neon_hack"};
    CHECK_INT(parse(&cfg, 1, none, err, sizeof err, stdout), NH_CFG_RUN);
    CHECK(!cfg.new_game); /* sans --new, c'est le menu qui décide */
}

int main(void)
{
    test_locale_and_env();
    test_valid_options();
    test_errors();
    test_help_and_version();
    test_data_dir();
    test_explicit_flags_are_tracked();
    test_new_game_default_off();
    return NH_TEST_REPORT("config");
}
