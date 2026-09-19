#ifndef NH_CONFIG_H
#define NH_CONFIG_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#ifndef NH_VERSION
#define NH_VERSION "0.0.0-unknown"
#endif

typedef enum
{
    NH_LANG_FR = 0,
    NH_LANG_EN = 1,
    NH_LANG_COUNT
} NhLang;

typedef struct
{
    NhLang lang;
    bool has_seed; /* --seed donné : parties reproductibles */
    uint64_t seed;
    bool fast;     /* pas de pauses d'animation */
    bool color;    /* couleurs ANSI (nouvelle UI) */
    bool new_game; /* ignorer une éventuelle sauvegarde */
} NhConfig;

typedef enum
{
    NH_CFG_RUN = 0,  /* lancer le jeu */
    NH_CFG_EXIT_OK,  /* --help / --version déjà traités : quitter avec le code 0 */
    NH_CFG_ERROR     /* option invalide : message dans err, quitter avec le code 2 */
} NhCfgResult;

/* "fr_FR.UTF-8" -> FR ; "en_US" -> EN ; inconnu ou NULL -> EN. */
NhLang nh_lang_from_locale(const char *locale);

/* Valeurs par défaut, d'après l'environnement (passé en paramètre pour rester testable). */
void nh_config_defaults(NhConfig *cfg, const char *env_lang, const char *env_no_color);

/* Analyse argv. Écrit l'aide / la version sur `out` quand demandées. */
NhCfgResult nh_config_parse(int argc, char **argv, NhConfig *cfg,
                            char *err, size_t err_size, FILE *out);

void nh_print_usage(FILE *out);
const char *nh_version(void);

#endif /* NH_CONFIG_H */
