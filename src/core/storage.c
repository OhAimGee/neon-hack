#include "storage.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#ifdef _WIN32
#include <direct.h>
#include <io.h>
#include <windows.h>
#else
#include <unistd.h>
#endif

#ifndef S_ISDIR
#define S_ISDIR(m) (((m) & S_IFMT) == S_IFDIR)
#endif

static bool present(const char *s) { return s != NULL && s[0] != '\0'; }

static bool is_sep(char c) { return c == '/' || c == '\\'; }

/* Écrit "<base>/neon-hack" ; faux si cela ne tient pas. */
static bool with_app_dir(char *out, size_t size, const char *base)
{
    int n = snprintf(out, size, "%s/neon-hack", base);
    return n > 0 && (size_t)n < size;
}

bool nh_storage_dir(char *out, size_t size, const char *override_dir, const char *xdg_data_home,
                    const char *home, const char *appdata)
{
    if (size == 0)
        return false;
    out[0] = '\0';

    if (present(override_dir))
    {
        int n = snprintf(out, size, "%s", override_dir);
        return n > 0 && (size_t)n < size;
    }
    if (present(appdata))
        return with_app_dir(out, size, appdata);
    /* La spécification XDG exige un chemin absolu : un chemin relatif est ignoré. */
    if (present(xdg_data_home) && is_sep(xdg_data_home[0]))
        return with_app_dir(out, size, xdg_data_home);
    if (present(home))
    {
        char base[NH_PATH_MAX];
        int n = snprintf(base, sizeof base, "%s/.local/share", home);
        return n > 0 && (size_t)n < sizeof base && with_app_dir(out, size, base);
    }
    return false;
}

static bool join(char *out, size_t size, const char *dir, const char *name)
{
    int n = snprintf(out, size, "%s/%s", dir, name);
    return n > 0 && (size_t)n < size;
}

void nh_paths_from_dir(NhPaths *p, const char *dir)
{
    memset(p, 0, sizeof *p);
    if (!present(dir))
        return;

    int n = snprintf(p->dir, sizeof p->dir, "%s", dir);
    if (n <= 0 || (size_t)n >= sizeof p->dir ||
        !join(p->save, sizeof p->save, p->dir, NH_SAVE_FILE) ||
        !join(p->settings, sizeof p->settings, p->dir, NH_SETTINGS_FILE))
    {
        memset(p, 0, sizeof *p);
        return;
    }
    p->ok = true;
}

void nh_paths_init(NhPaths *p, const char *override_dir)
{
    char dir[NH_PATH_MAX];
    if (!nh_storage_dir(dir, sizeof dir, override_dir, getenv("XDG_DATA_HOME"), getenv("HOME"),
                        getenv("APPDATA")))
        dir[0] = '\0';
    nh_paths_from_dir(p, dir);
}

/* ---- Fichiers ------------------------------------------------------------ */

static bool make_dir(const char *path)
{
#ifdef _WIN32
    int rc = _mkdir(path);
#else
    int rc = mkdir(path, 0700);
#endif
    if (rc == 0)
        return true;
    if (errno != EEXIST)
        return false;
    struct stat st;
    return stat(path, &st) == 0 && S_ISDIR(st.st_mode);
}

bool nh_storage_mkdirs(const char *dir)
{
    char tmp[NH_PATH_MAX];
    size_t n = dir != NULL ? strlen(dir) : 0;
    if (n == 0 || n >= sizeof tmp)
        return false;
    memcpy(tmp, dir, n + 1);

    while (n > 1 && is_sep(tmp[n - 1]))
        tmp[--n] = '\0';

    /* Les parents qui existent déjà peuvent refuser mkdir : seul le résultat final compte. */
    for (char *p = tmp + 1; *p != '\0'; p++)
    {
        if (is_sep(*p))
        {
            char saved = *p;
            *p = '\0';
            (void)make_dir(tmp);
            *p = saved;
        }
    }
    return make_dir(tmp);
}

bool nh_storage_exists(const char *path)
{
    struct stat st;
    return path != NULL && stat(path, &st) == 0;
}

bool nh_storage_remove(const char *path) { return path != NULL && remove(path) == 0; }

NhStorageStatus nh_storage_read(const char *path, size_t max_len, char **data, size_t *len)
{
    *data = NULL;
    *len = 0;

    FILE *f = fopen(path, "rb");
    if (f == NULL)
        return errno == ENOENT ? NH_STORAGE_MISSING : NH_STORAGE_ERROR;

    size_t cap = 4096;
    size_t used = 0;
    char *buf = malloc(cap);
    NhStorageStatus status = NH_STORAGE_OK;

    while (buf != NULL)
    {
        if (used + 1 >= cap)
        {
            char *grown = realloc(buf, cap * 2);
            if (grown == NULL)
            {
                free(buf);
                buf = NULL;
                break;
            }
            buf = grown;
            cap *= 2;
        }
        size_t got = fread(buf + used, 1, cap - used - 1, f);
        used += got;
        if (used > max_len)
        {
            status = NH_STORAGE_TOO_BIG;
            break;
        }
        if (got == 0)
        {
            if (ferror(f))
                status = NH_STORAGE_ERROR;
            break;
        }
    }
    fclose(f);

    if (buf == NULL)
        return NH_STORAGE_ERROR;
    if (status != NH_STORAGE_OK)
    {
        free(buf);
        return status;
    }
    buf[used] = '\0';
    *data = buf;
    *len = used;
    return NH_STORAGE_OK;
}

static bool replace_file(const char *from, const char *to)
{
#ifdef _WIN32
    return MoveFileExA(from, to, MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) != 0;
#else
    return rename(from, to) == 0;
#endif
}

/* Force les octets sur le disque. Un système de fichiers qui ne sait pas le faire n'est pas une erreur. */
static bool flush_to_disk(FILE *f)
{
    if (fflush(f) != 0)
        return false;
#ifdef _WIN32
    return _commit(_fileno(f)) == 0;
#else
    if (fsync(fileno(f)) != 0 && errno != EINVAL && errno != ENOTSUP)
        return false;
    return true;
#endif
}

bool nh_storage_write_atomic(const char *path, const char *data, size_t len)
{
    char tmp[NH_PATH_MAX + 64];
    int n = snprintf(tmp, sizeof tmp, "%s.tmp", path);
    if (n <= 0 || (size_t)n >= sizeof tmp)
        return false;

    FILE *f = fopen(tmp, "wb");
    if (f == NULL && errno == ENOENT)
    {
        /* Premier enregistrement : le dossier n'existe pas encore. */
        char parent[sizeof tmp];
        memcpy(parent, path, strlen(path) + 1);
        char *cut = NULL;
        for (char *p = parent; *p != '\0'; p++)
            if (is_sep(*p))
                cut = p;
        if (cut != NULL && cut != parent)
        {
            *cut = '\0';
            if (nh_storage_mkdirs(parent))
                f = fopen(tmp, "wb");
        }
    }
    if (f == NULL)
        return false;

    bool ok = fwrite(data, 1, len, f) == len && flush_to_disk(f);
    if (fclose(f) != 0)
        ok = false;

    if (ok)
        ok = replace_file(tmp, path);
    if (!ok)
        (void)remove(tmp);
    return ok;
}
