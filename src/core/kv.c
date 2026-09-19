#include "kv.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Une sauvegarde compte quelques centaines de clés : au-delà, le fichier n'est pas de nous. */
#define KV_MAX_PAIRS 8192

static bool is_blank(char c) { return c == ' ' || c == '\t'; }

bool nh_kv_parse(NhKv *kv, const char *text, size_t len)
{
    memset(kv, 0, sizeof *kv);
    kv->text = malloc(len + 1);
    if (kv->text == NULL)
        return false;
    memcpy(kv->text, text, len);
    kv->text[len] = '\0';

    size_t cap = 0;
    char *p = kv->text;
    char *end = kv->text + len;

    while (p < end)
    {
        char *eol = memchr(p, '\n', (size_t)(end - p));
        char *next = end;
        if (eol != NULL)
        {
            *eol = '\0';
            next = eol + 1;
        }

        size_t n = strlen(p);
        while (n > 0 && p[n - 1] == '\r')
            p[--n] = '\0';
        while (is_blank(*p))
            p++;

        char *eq = strchr(p, '=');
        if (*p != '\0' && *p != '#' && *p != ';' && eq != NULL && eq != p)
        {
            char *value = eq + 1;
            char *key_end = eq;
            while (key_end > p && is_blank(key_end[-1]))
                key_end--;

            if (key_end > p)
            {
                *key_end = '\0';
                if (kv->count == cap)
                {
                    if (cap >= KV_MAX_PAIRS)
                    {
                        nh_kv_free(kv);
                        return false;
                    }
                    size_t new_cap = cap == 0 ? 64 : cap * 2;
                    NhKvPair *grown = realloc(kv->pairs, new_cap * sizeof *grown);
                    if (grown == NULL)
                    {
                        nh_kv_free(kv);
                        return false;
                    }
                    kv->pairs = grown;
                    cap = new_cap;
                }
                kv->pairs[kv->count].key = p;
                kv->pairs[kv->count].value = value;
                kv->count++;
            }
        }
        p = next;
    }
    return true;
}

void nh_kv_free(NhKv *kv)
{
    free(kv->text);
    free(kv->pairs);
    memset(kv, 0, sizeof *kv);
}

const char *nh_kv_get(const NhKv *kv, const char *key)
{
    for (size_t i = kv->count; i > 0; i--)
        if (strcmp(kv->pairs[i - 1].key, key) == 0)
            return kv->pairs[i - 1].value;
    return NULL;
}

NhKvStatus nh_kv_int(const NhKv *kv, const char *key, long min, long max, long *out)
{
    const char *text = nh_kv_get(kv, key);
    if (text == NULL)
        return NH_KV_MISSING;

    while (is_blank(*text))
        text++;
    if (*text == '\0')
        return NH_KV_INVALID;

    char *end = NULL;
    errno = 0;
    long value = strtol(text, &end, 10);
    if (errno == ERANGE || end == text)
        return NH_KV_INVALID;
    while (is_blank(*end))
        end++;
    if (*end != '\0' || value < min || value > max)
        return NH_KV_INVALID;

    *out = value;
    return NH_KV_OK;
}

/* ---- Écrivain ------------------------------------------------------------ */

void nh_kvw_init(NhKvWriter *w) { memset(w, 0, sizeof *w); }

void nh_kvw_free(NhKvWriter *w)
{
    free(w->data);
    memset(w, 0, sizeof *w);
}

const char *nh_kvw_text(const NhKvWriter *w) { return w->data != NULL ? w->data : ""; }

static bool reserve(NhKvWriter *w, size_t extra)
{
    if (w->failed)
        return false;
    size_t need = w->len + extra + 1;
    if (need <= w->cap)
        return true;

    size_t cap = w->cap != 0 ? w->cap : 1024;
    while (cap < need)
        cap *= 2;
    char *grown = realloc(w->data, cap);
    if (grown == NULL)
    {
        w->failed = true;
        return false;
    }
    w->data = grown;
    w->cap = cap;
    if (w->len == 0)
        w->data[0] = '\0';
    return true;
}

/* Ajoute `s` en remplaçant ce qui casserait le format (fins de ligne, et '=' dans une clé). */
static void put_clean(NhKvWriter *w, const char *s, bool is_key)
{
    size_t n = strlen(s);
    if (!reserve(w, n))
        return;
    for (size_t i = 0; i < n; i++)
    {
        unsigned char c = (unsigned char)s[i];
        if (c < 0x20 || c == 0x7F)
            c = ' ';
        else if (is_key && c == '=')
            c = '_';
        w->data[w->len++] = (char)c;
    }
    w->data[w->len] = '\0';
}

void nh_kvw_str(NhKvWriter *w, const char *key, const char *value)
{
    if (!reserve(w, 2))
        return;
    put_clean(w, key, true);
    if (!reserve(w, 1))
        return;
    w->data[w->len++] = '=';
    put_clean(w, value, false);
    if (!reserve(w, 1))
        return;
    w->data[w->len++] = '\n';
    w->data[w->len] = '\0';
}

void nh_kvw_int(NhKvWriter *w, const char *key, long value)
{
    char text[32];
    snprintf(text, sizeof text, "%ld", value);
    nh_kvw_str(w, key, text);
}
