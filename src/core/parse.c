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
