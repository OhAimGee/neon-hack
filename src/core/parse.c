#include "parse.h"

#include "utf8.h"

#include <string.h>

static bool is_space(char c) { return c == ' ' || c == '\t' || c == '\r' || c == '\n'; }

static char lower(char c) { return (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : c; }

bool nh_split_command(const char *line, char *cmd, size_t cmd_size, char *arg, size_t arg_size)
{
    if (cmd_size > 0)
        cmd[0] = '\0';
    if (arg_size > 0)
        arg[0] = '\0';
    if (line == NULL || cmd_size == 0 || arg_size == 0)
        return false;

    while (is_space(*line))
        line++;

    size_t n = 0;
    while (*line != '\0' && !is_space(*line))
    {
        if (n + 1 < cmd_size)
            cmd[n++] = *line;
        line++;
    }
    cmd[n] = '\0';
    nh_utf8_trim_incomplete(cmd);

    while (is_space(*line))
        line++;

    size_t len = strlen(line);
    while (len > 0 && is_space(line[len - 1]))
        len--;
    if (len >= arg_size)
        len = arg_size - 1;
    memcpy(arg, line, len);
    arg[len] = '\0';
    nh_utf8_trim_incomplete(arg);

    return cmd[0] != '\0';
}

bool nh_str_eq_nocase(const char *a, const char *b)
{
    while (*a != '\0' && *b != '\0')
    {
        if (lower(*a) != lower(*b))
            return false;
        a++;
        b++;
    }
    return *a == *b;
}

bool nh_str_has_prefix_nocase(const char *s, const char *prefix)
{
    for (; *prefix != '\0'; prefix++, s++)
        if (*s == '\0' || lower(*s) != lower(*prefix))
            return false;
    return true;
}

/* Longueur (1 à 4) de la séquence UTF-8 valide qui commence en `s`, ou 0 si elle est invalide. */
static size_t utf8_seq_len(const unsigned char *s)
{
    size_t len;
    if (s[0] < 0x80)
        return 1;
    if (s[0] >= 0xC2 && s[0] <= 0xDF)
        len = 2;
    else if (s[0] >= 0xE0 && s[0] <= 0xEF)
        len = 3;
    else if (s[0] >= 0xF0 && s[0] <= 0xF4)
        len = 4;
    else
        return 0;

    for (size_t i = 1; i < len; i++)
        if ((s[i] & 0xC0) != 0x80)
            return 0;
    return len;
}

size_t nh_clean_name(const char *in, char *out, size_t out_size, size_t max_chars)
{
    if (out_size == 0)
        return 0;
    out[0] = '\0';
    if (in == NULL)
        return 0;

    const unsigned char *s = (const unsigned char *)in;
    size_t used = 0;
    size_t chars = 0;
    bool pending_space = false;

    while (*s != '\0')
    {
        size_t len = utf8_seq_len(s);
        if (len == 0)
        {
            s++; /* octet isolé ou séquence cassée : on l'écarte */
            continue;
        }

        /* Contrôles C0, DEL et C1 (U+0080..U+009F, codés C2 80..C2 9F) : jamais dans un nom. */
        bool control = s[0] < 0x20 || s[0] == 0x7F || (s[0] == 0xC2 && s[1] >= 0x80 && s[1] <= 0x9F);
        if (control)
        {
            /* Une tabulation sépare deux mots comme une espace ; les autres contrôles disparaissent. */
            if (s[0] == '\t')
                pending_space = true;
            s += len;
            continue;
        }
        if (s[0] == ' ')
        {
            pending_space = true;
            s++;
            continue;
        }

        bool space = pending_space && used > 0;
        if (chars + (space ? 2 : 1) > max_chars || used + len + (space ? 1 : 0) + 1 > out_size)
            break;
        if (space)
        {
            out[used++] = ' ';
            chars++;
        }
        pending_space = false;
        memcpy(out + used, s, len);
        used += len;
        chars++;
        s += len;
    }
    out[used] = '\0';
    return used;
}

bool nh_parse_yes_no(const char *line, bool default_yes)
{
    static const char *const yes[] = {"o", "oui", "y", "yes"};
    static const char *const no[] = {"n", "non", "no"};

    if (line == NULL)
        return default_yes;
    while (is_space(*line))
        line++;
    size_t len = strlen(line);
    while (len > 0 && is_space(line[len - 1]))
        len--;

    char word[8];
    if (len == 0 || len >= sizeof word)
        return default_yes;
    for (size_t i = 0; i < len; i++)
        word[i] = lower(line[i]);
    word[len] = '\0';

    for (size_t i = 0; i < sizeof yes / sizeof yes[0]; i++)
        if (strcmp(word, yes[i]) == 0)
            return true;
    for (size_t i = 0; i < sizeof no / sizeof no[0]; i++)
        if (strcmp(word, no[i]) == 0)
            return false;
    return default_yes;
}
