#include "nh_test.h"

#include "nh_tmp.h"

#include "../../src/core/config.h"
#include "../../src/core/settings.h"

#include <stdlib.h>

static NhSettings defaults(void)
{
    NhSettings s = {.lang = NH_LANG_EN, .color = true, .fast = false, .hud = true};
    return s;
}

static void from_text(NhSettings *s, const char *text) { nh_settings_from_text(s, text, strlen(text)); }

static void test_roundtrip_text(void)
{
    NhSettings s = {.lang = NH_LANG_FR, .color = false, .fast = true, .hud = false};
    char text[256];
    nh_settings_to_text(&s, text, sizeof text);
    CHECK_STR(text, "lang=fr\ncolor=0\nfast=1\nhud=0\n");

    NhSettings back = defaults();
    from_text(&back, text);
    CHECK_INT(back.lang, NH_LANG_FR);
    CHECK(!back.color);
    CHECK(back.fast);
    CHECK(!back.hud);
}

static void test_partial_and_invalid_values_keep_defaults(void)
{
    NhSettings s = defaults();
    from_text(&s, "lang=fr\n");
    CHECK_INT(s.lang, NH_LANG_FR);
    CHECK(s.color && !s.fast && s.hud); /* le reste n'a pas bougé */

    s = defaults();
    from_text(&s, "lang=klingon\ncolor=2\nfast=oui\nhud=-1\n");
    CHECK_INT(s.lang, NH_LANG_EN);
    CHECK(s.color && !s.fast && s.hud);

    /* valeur invalide pour une clé n'empêche pas les autres */
    s = defaults();
    from_text(&s, "color=peut-être\nfast=1\n");
    CHECK(s.color);
    CHECK(s.fast);

    s = defaults();
    from_text(&s, "");
    CHECK_INT(s.lang, NH_LANG_EN);
    from_text(&s, "\x01\x02\xff garbage\n===\n");
    CHECK_INT(s.lang, NH_LANG_EN);
    CHECK(s.color && !s.fast && s.hud);
}

static void test_to_text_truncation_and_bad_lang(void)
{
    NhSettings s = defaults();
    char tiny[6];
    nh_settings_to_text(&s, tiny, sizeof tiny);
    CHECK_INT(strlen(tiny), 5); /* tronqué mais terminé */
    nh_settings_to_text(&s, tiny, 0); /* ne doit rien écrire */

    s.lang = (NhLang)99; /* valeur hors énumération : on n'écrit jamais un code inventé */
    char text[128];
    nh_settings_to_text(&s, text, sizeof text);
    CHECK(strncmp(text, "lang=en\n", 8) == 0);
}

static void test_file_roundtrip(void)
{
    char root[256], path[512];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/sous/settings.cfg", root);

    NhSettings loaded = defaults();
    CHECK(!nh_settings_load(&loaded, path)); /* absent : faux, réglages inchangés */
    CHECK_INT(loaded.lang, NH_LANG_EN);

    NhSettings s = {.lang = NH_LANG_FR, .color = false, .fast = true, .hud = true};
    CHECK(nh_settings_save(&s, path)); /* crée le dossier */
    CHECK(nh_settings_load(&loaded, path));
    CHECK_INT(loaded.lang, NH_LANG_FR);
    CHECK(!loaded.color && loaded.fast && loaded.hud);

    nh_tmp_write(path, "lang=en\n"); /* édité à la main */
    CHECK(nh_settings_load(&loaded, path));
    CHECK_INT(loaded.lang, NH_LANG_EN);
    CHECK(!loaded.color); /* clé absente : la valeur déjà en place reste */

    /* un fichier absurdement gros n'est pas le nôtre */
    char *big = malloc(10000);
    memset(big, 'x', 9999);
    big[9999] = '\0';
    nh_tmp_write(path, big);
    free(big);
    NhSettings untouched = defaults();
    CHECK(!nh_settings_load(&untouched, path));
    CHECK_INT(untouched.lang, NH_LANG_EN);

    nh_tmp_remove(root);
}

/* ---- Priorité : environnement < fichier < ligne de commande (config.c) ---------------------- */

static void test_apply_settings_precedence(void)
{
    NhConfig cfg;
    nh_config_defaults(&cfg, "en_US.UTF-8", NULL);
    NhSettings saved = {.lang = NH_LANG_FR, .color = false, .fast = true, .hud = false};

    nh_config_apply_settings(&cfg, &saved, false);
    CHECK_INT(cfg.lang, NH_LANG_FR);
    CHECK(!cfg.color);
    CHECK(cfg.fast);
    CHECK(!cfg.hud);

    /* un choix explicite en ligne de commande passe avant le fichier */
    nh_config_defaults(&cfg, "en_US.UTF-8", NULL);
    cfg.lang = NH_LANG_EN;
    cfg.lang_set = true;
    cfg.color = true;
    cfg.color_set = true;
    nh_config_apply_settings(&cfg, &saved, false);
    CHECK_INT(cfg.lang, NH_LANG_EN);
    CHECK(cfg.color);
    CHECK(cfg.fast); /* non imposé : le fichier s'applique */
}

static void test_apply_settings_no_color(void)
{
    NhConfig cfg;
    NhSettings saved = {.lang = NH_LANG_EN, .color = true, .fast = false, .hud = true};

    /* NO_COLOR est une demande de l'utilisateur pour tout son environnement : le fichier ne la lève pas */
    nh_config_defaults(&cfg, "fr_FR.UTF-8", "1");
    nh_config_apply_settings(&cfg, &saved, true);
    CHECK(!cfg.color);

    /* seule l'option explicite --color la surpasse */
    nh_config_defaults(&cfg, "fr_FR.UTF-8", "1");
    cfg.color = true;
    cfg.color_set = true;
    nh_config_apply_settings(&cfg, &saved, true);
    CHECK(cfg.color);
}

static void test_settings_from_config(void)
{
    NhConfig cfg;
    nh_config_defaults(&cfg, "fr_FR.UTF-8", "1");
    NhSettings s;
    nh_settings_from_config(&s, &cfg);
    CHECK_INT(s.lang, NH_LANG_FR);
    CHECK(!s.color); /* défauts issus de l'environnement */
}

int main(void)
{
    test_roundtrip_text();
    test_partial_and_invalid_values_keep_defaults();
    test_to_text_truncation_and_bad_lang();
    test_file_roundtrip();
    test_apply_settings_precedence();
    test_apply_settings_no_color();
    test_settings_from_config();
    return NH_TEST_REPORT("settings");
}
