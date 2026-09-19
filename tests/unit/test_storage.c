#include "nh_test.h"

#include "nh_tmp.h"

#include "../../src/core/storage.h"

#include <stdlib.h>

static void test_dir_precedence(void)
{
    char out[NH_PATH_MAX];

    /* --data-dir l'emporte sur tout, tel quel */
    CHECK(nh_storage_dir(out, sizeof out, "/donnees", "/xdg", "/home/j", "C:/Roaming"));
    CHECK_STR(out, "/donnees");

    CHECK(nh_storage_dir(out, sizeof out, NULL, "/xdg", "/home/j", "C:/Roaming"));
    CHECK_STR(out, "C:/Roaming/neon-hack");
    CHECK(nh_storage_dir(out, sizeof out, "", "/xdg", "/home/j", ""));
    CHECK_STR(out, "/xdg/neon-hack");
    CHECK(nh_storage_dir(out, sizeof out, NULL, NULL, "/home/j", NULL));
    CHECK_STR(out, "/home/j/.local/share/neon-hack");
}

static void test_dir_xdg_must_be_absolute(void)
{
    char out[NH_PATH_MAX];
    /* un XDG_DATA_HOME relatif est ignoré (spécification XDG) : on retombe sur $HOME */
    CHECK(nh_storage_dir(out, sizeof out, NULL, "relatif/dir", "/home/j", NULL));
    CHECK_STR(out, "/home/j/.local/share/neon-hack");
}

static void test_dir_unavailable(void)
{
    char out[NH_PATH_MAX];
    CHECK(!nh_storage_dir(out, sizeof out, NULL, NULL, NULL, NULL));
    CHECK_STR(out, "");
    CHECK(!nh_storage_dir(out, sizeof out, "", "", "", ""));

    char tiny[8];
    CHECK(!nh_storage_dir(tiny, sizeof tiny, "/un/chemin/trop/long", NULL, NULL, NULL));
    CHECK(!nh_storage_dir(tiny, sizeof tiny, NULL, NULL, "/home/jean", NULL));
    CHECK(!nh_storage_dir(tiny, 0, "/x", NULL, NULL, NULL));
}

static void test_paths_from_dir(void)
{
    NhPaths p;
    nh_paths_from_dir(&p, "/donnees");
    CHECK(p.ok);
    CHECK_STR(p.dir, "/donnees");
    CHECK_STR(p.save, "/donnees/savegame.sav");
    CHECK_STR(p.settings, "/donnees/settings.cfg");

    nh_paths_from_dir(&p, "");
    CHECK(!p.ok);
    CHECK_STR(p.save, "");
    nh_paths_from_dir(&p, NULL);
    CHECK(!p.ok);

    char too_long[NH_PATH_MAX + 10];
    memset(too_long, 'a', sizeof too_long);
    too_long[sizeof too_long - 1] = '\0';
    nh_paths_from_dir(&p, too_long);
    CHECK(!p.ok);
    CHECK_STR(p.dir, ""); /* aucun chemin tronqué ne doit traîner */
}

static void test_mkdirs(void)
{
    char root[256], deep[512];
    nh_tmp_make(root, sizeof root);

    snprintf(deep, sizeof deep, "%s/a/b/c", root);
    CHECK(nh_storage_mkdirs(deep));
    CHECK(nh_storage_exists(deep));
    CHECK(nh_storage_mkdirs(deep)); /* déjà là : pas une erreur */

    snprintf(deep, sizeof deep, "%s/a/b/c///", root);
    CHECK(nh_storage_mkdirs(deep)); /* séparateurs finaux tolérés */

    CHECK(!nh_storage_mkdirs(""));
    CHECK(!nh_storage_mkdirs(NULL));

    /* un fichier n'est pas un dossier */
    snprintf(deep, sizeof deep, "%s/fichier", root);
    nh_tmp_write(deep, "x");
    CHECK(!nh_storage_mkdirs(deep));

    nh_tmp_remove(root);
}

static void test_read_write_roundtrip(void)
{
    char root[256], path[512];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/f.txt", root);

    CHECK(!nh_storage_exists(path));
    char *data = NULL;
    size_t len = 99;
    CHECK_INT(nh_storage_read(path, 1000, &data, &len), NH_STORAGE_MISSING);
    CHECK(data == NULL);
    CHECK_INT(len, 0);

    CHECK(nh_storage_write_atomic(path, "bonjour\nmonde\n", 14));
    CHECK(nh_storage_exists(path));
    CHECK_INT(nh_storage_read(path, 1000, &data, &len), NH_STORAGE_OK);
    CHECK_INT(len, 14);
    CHECK_STR(data, "bonjour\nmonde\n");
    free(data);

    /* réécriture : l'ancien contenu est entièrement remplacé, y compris par plus court */
    CHECK(nh_storage_write_atomic(path, "ab", 2));
    CHECK_INT(nh_storage_read(path, 1000, &data, &len), NH_STORAGE_OK);
    CHECK_STR(data, "ab");
    free(data);

    /* écriture atomique : aucun fichier temporaire ne reste */
    char tmp[600];
    snprintf(tmp, sizeof tmp, "%s.tmp", path);
    CHECK(!nh_storage_exists(tmp));

    CHECK(nh_storage_remove(path));
    CHECK(!nh_storage_exists(path));
    CHECK(!nh_storage_remove(path));

    nh_tmp_remove(root);
}

static void test_write_creates_parents(void)
{
    char root[256], path[512];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/neon-hack/sous/save.sav", root);

    CHECK(nh_storage_write_atomic(path, "x=1\n", 4));
    char *data = NULL;
    size_t len = 0;
    CHECK_INT(nh_storage_read(path, 100, &data, &len), NH_STORAGE_OK);
    CHECK_STR(data, "x=1\n");
    free(data);

    nh_tmp_remove(root);
}

static void test_read_limits(void)
{
    char root[256], path[512];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/gros", root);

    char *big = malloc(20000);
    memset(big, 'z', 20000);
    CHECK(nh_storage_write_atomic(path, big, 20000));

    char *data = NULL;
    size_t len = 0;
    CHECK_INT(nh_storage_read(path, 19999, &data, &len), NH_STORAGE_TOO_BIG);
    CHECK(data == NULL);
    CHECK_INT(nh_storage_read(path, 20000, &data, &len), NH_STORAGE_OK); /* la limite est incluse */
    CHECK_INT(len, 20000);
    CHECK(data[20000] == '\0');
    free(data);

    /* fichier vide */
    CHECK(nh_storage_write_atomic(path, "", 0));
    CHECK_INT(nh_storage_read(path, 10, &data, &len), NH_STORAGE_OK);
    CHECK_INT(len, 0);
    CHECK_STR(data, "");
    free(data);

    free(big);
    nh_tmp_remove(root);
}

static void test_write_failure_keeps_old_file(void)
{
    char root[256], path[512];
    nh_tmp_make(root, sizeof root);
    snprintf(path, sizeof path, "%s/f", root);
    CHECK(nh_storage_write_atomic(path, "ancien", 6));

    /* la destination est un dossier : le renommage échoue, le contenu d'origine et le dossier restent */
    char blocked[600];
    snprintf(blocked, sizeof blocked, "%s/d", root);
    CHECK(nh_storage_mkdirs(blocked));
    CHECK(!nh_storage_write_atomic(blocked, "nouveau", 7));
    char tmp[700];
    snprintf(tmp, sizeof tmp, "%s.tmp", blocked);
    CHECK(!nh_storage_exists(tmp)); /* le fichier temporaire est nettoyé */

    char *data = NULL;
    size_t len = 0;
    CHECK_INT(nh_storage_read(path, 100, &data, &len), NH_STORAGE_OK);
    CHECK_STR(data, "ancien");
    free(data);

    nh_tmp_remove(root);
}

int main(void)
{
    test_dir_precedence();
    test_dir_xdg_must_be_absolute();
    test_dir_unavailable();
    test_paths_from_dir();
    test_mkdirs();
    test_read_write_roundtrip();
    test_write_creates_parents();
    test_read_limits();
    test_write_failure_keeps_old_file();
    return NH_TEST_REPORT("storage");
}
