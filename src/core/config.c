#include "config.h"

#include <ctype.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>

const char *nh_version(void) { return NH_VERSION; }

static bool starts_with_nocase(const char *s, const char *prefix)
{
    while (*prefix)
    {
        if (tolower((unsigned char)*s) != tolower((unsigned char)*prefix))
            return false;
        s++;
        prefix++;
    }
    return true;
}

NhLang nh_lang_from_locale(const char *locale)
{
    if (locale && starts_with_nocase(locale, "fr"))
        return NH_LANG_FR;
    return NH_LANG_EN;
}

void nh_config_defaults(NhConfig *cfg, const char *env_lang, const char *env_no_color)
{
    memset(cfg, 0, sizeof *cfg);
    cfg->lang = nh_lang_from_locale(env_lang);
    cfg->hud = true;
    cfg->color = !(env_no_color && env_no_color[0] != '\0'); /* convention NO_COLOR */
}

void nh_settings_from_config(NhSettings *s, const NhConfig *cfg)
{
    s->lang = cfg->lang;
    s->color = cfg->color;
    s->fast = cfg->fast;
    s->hud = cfg->hud;
}

void nh_config_apply_settings(NhConfig *cfg, const NhSettings *s, bool env_no_color)
{
    if (!cfg->lang_set)
        cfg->lang = s->lang;
    if (!cfg->color_set)
        cfg->color = s->color && !env_no_color;
    if (!cfg->fast_set)
        cfg->fast = s->fast;
    if (!cfg->hud_set)
        cfg->hud = s->hud;
}

void nh_print_usage(FILE *out)
{
    fputs("Usage: neon_hack [options]\n"
          "\n"
          "Options:\n"
          "  --lang fr|en    language (default: from $LANG)\n"
          "  --seed N        reproducible game (fixed random seed)\n"
          "  --fast          skip animation delays (automatic when output is piped)\n"
          "  --color         force ANSI colors on\n"
          "  --no-color      disable ANSI colors (also: NO_COLOR) [not yet applied to\n"
          "                  screens that have not been ported to the new UI]\n"
          "  --no-hud        no fixed status/command bars (they only appear on a real\n"
          "                  terminal of at least 80x24)\n"
          "  --new           start a new game right away (skips the menu; the first save\n"
          "                  replaces the current one)\n"
          "  --data-dir DIR  where saved games and settings live (default: the user's\n"
          "                  data folder, e.g. ~/.local/share/neon-hack)\n"
          "  -V, --version   print version and exit\n"
          "  -h, --help      print this help and exit\n",
          out);
}

static void set_err(char *err, size_t err_size, const char *msg, const char *arg)
{
    if (err_size == 0)
        return;
    if (arg)
        snprintf(err, err_size, "%s: %s", msg, arg);
    else
        snprintf(err, err_size, "%s", msg);
}

static bool parse_seed(const char *text, uint64_t *out)
{
    if (text == NULL || *text == '\0' || *text == '-' || *text == '+')
        return false;

    char *end = NULL;
    errno = 0;
    unsigned long long v = strtoull(text, &end, 10);
    if (errno == ERANGE || end == text || *end != '\0')
        return false;

    *out = (uint64_t)v;
    return true;
}

static bool parse_lang(const char *text, NhLang *out)
{
    if (text == NULL)
        return false;
    if (strcmp(text, "fr") == 0)
        *out = NH_LANG_FR;
    else if (strcmp(text, "en") == 0)
        *out = NH_LANG_EN;
    else
        return false;
    return true;
}

/* Valeur d'une option "--nom valeur" ou "--nom=valeur". NULL si absente. */
static const char *option_value(const char *arg, const char *name, int argc, char **argv, int *i)
{
    size_t n = strlen(name);
    if (strncmp(arg, name, n) != 0)
        return NULL;
    if (arg[n] == '=')
        return arg + n + 1;
    if (arg[n] == '\0' && *i + 1 < argc)
        return argv[++(*i)];
    return NULL;
}

NhCfgResult nh_config_parse(int argc, char **argv, NhConfig *cfg,
                            char *err, size_t err_size, FILE *out)
{
    if (err_size > 0)
        err[0] = '\0';

    for (int i = 1; i < argc; i++)
    {
        const char *arg = argv[i];

        if (strcmp(arg, "-h") == 0 || strcmp(arg, "--help") == 0)
        {
            nh_print_usage(out);
            return NH_CFG_EXIT_OK;
        }
        if (strcmp(arg, "-V") == 0 || strcmp(arg, "--version") == 0)
        {
            fprintf(out, "neon_hack %s\n", nh_version());
            return NH_CFG_EXIT_OK;
        }
        if (strcmp(arg, "--fast") == 0)
        {
            cfg->fast = true;
            cfg->fast_set = true;
        }
        else if (strcmp(arg, "--color") == 0)
        {
            cfg->color = true;
            cfg->color_set = true;
        }
        else if (strcmp(arg, "--no-color") == 0)
        {
            cfg->color = false;
            cfg->color_set = true;
        }
        else if (strcmp(arg, "--no-hud") == 0)
        {
            cfg->hud = false;
            cfg->hud_set = true;
        }
        else if (strcmp(arg, "--new") == 0)
        {
            cfg->new_game = true;
        }
        else if (strncmp(arg, "--lang", 6) == 0 && (arg[6] == '\0' || arg[6] == '='))
        {
            const char *value = option_value(arg, "--lang", argc, argv, &i);
            if (!parse_lang(value, &cfg->lang))
            {
                set_err(err, err_size, "--lang expects 'fr' or 'en'", value);
                return NH_CFG_ERROR;
            }
            cfg->lang_set = true;
        }
        else if (strncmp(arg, "--data-dir", 10) == 0 && (arg[10] == '\0' || arg[10] == '='))
        {
            const char *value = option_value(arg, "--data-dir", argc, argv, &i);
            if (value == NULL || value[0] == '\0' || strlen(value) >= sizeof cfg->data_dir)
            {
                set_err(err, err_size, "--data-dir expects a folder path", value);
                return NH_CFG_ERROR;
            }
            snprintf(cfg->data_dir, sizeof cfg->data_dir, "%s", value);
        }
        else if (strncmp(arg, "--seed", 6) == 0 && (arg[6] == '\0' || arg[6] == '='))
        {
            const char *value = option_value(arg, "--seed", argc, argv, &i);
            if (!parse_seed(value, &cfg->seed))
            {
                set_err(err, err_size, "--seed expects a non-negative integer", value);
                return NH_CFG_ERROR;
            }
            cfg->has_seed = true;
        }
        else
        {
            set_err(err, err_size, "unknown option", arg);
            return NH_CFG_ERROR;
        }
    }
    return NH_CFG_RUN;
}
