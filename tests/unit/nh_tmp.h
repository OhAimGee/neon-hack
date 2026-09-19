#ifndef NH_TMP_H
#define NH_TMP_H

/* Dossiers temporaires pour les tests de fichiers (POSIX). Jamais le vrai dossier de données du joueur. */

#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

/* Crée un dossier neuf sous $TMPDIR (ou /tmp) ; `out` reçoit son chemin. */
static inline void nh_tmp_make(char *out, size_t size)
{
    const char *base = getenv("TMPDIR");
    snprintf(out, size, "%s/nh-test-XXXXXX", base != NULL && base[0] != '\0' ? base : "/tmp");
    if (mkdtemp(out) == NULL)
    {
        perror("mkdtemp");
        exit(2);
    }
}

/* Supprime un dossier et tout son contenu. */
static inline void nh_tmp_remove(const char *path)
{
    struct stat st;
    if (lstat(path, &st) != 0)
        return;
    if (S_ISDIR(st.st_mode))
    {
        DIR *d = opendir(path);
        if (d != NULL)
        {
            struct dirent *e;
            while ((e = readdir(d)) != NULL)
            {
                if (strcmp(e->d_name, ".") == 0 || strcmp(e->d_name, "..") == 0)
                    continue;
                char child[1024];
                int n = snprintf(child, sizeof child, "%s/%s", path, e->d_name);
                if (n > 0 && (size_t)n < sizeof child)
                    nh_tmp_remove(child);
            }
            closedir(d);
        }
        rmdir(path);
    }
    else
    {
        unlink(path);
    }
}

/* Contenu d'un fichier (à libérer) ; NULL s'il n'existe pas. */
static inline char *nh_tmp_slurp(const char *path)
{
    FILE *f = fopen(path, "rb");
    if (f == NULL)
        return NULL;
    char *buf = malloc(1 << 20);
    size_t n = fread(buf, 1, (1 << 20) - 1, f);
    buf[n] = '\0';
    fclose(f);
    return buf;
}

/* Écrit un fichier de test. */
static inline void nh_tmp_write(const char *path, const char *text)
{
    FILE *f = fopen(path, "wb");
    if (f != NULL)
    {
        fputs(text, f);
        fclose(f);
    }
}

#endif /* NH_TMP_H */
