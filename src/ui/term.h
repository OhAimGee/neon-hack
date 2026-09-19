#ifndef NH_TERM_H
#define NH_TERM_H

#include <stdbool.h>
#include <stddef.h>

typedef enum
{
    NH_C_RESET,
    NH_C_RED,
    NH_C_GREEN,
    NH_C_YELLOW,
    NH_C_BLUE,
    NH_C_MAGENTA,
    NH_C_CYAN,
    NH_C_WHITE,
    NH_C_BRIGHT_GREEN,
    NH_C_BRIGHT_CYAN,
    NH_C_COUNT
} NhColor;

typedef enum
{
    NH_ALIGN_LEFT,
    NH_ALIGN_RIGHT,
    NH_ALIGN_CENTER
} NhAlign;

void nh_term_set_color(bool on);
bool nh_term_color_enabled(void);

/* Séquence ANSI de la couleur, ou "" si les couleurs sont désactivées. */
const char *nh_c(NhColor color);

/*
 * Largeur d'affichage en colonnes d'un texte UTF-8 : ignore les séquences
 * ANSI (\033[...m), compte 2 pour les caractères larges et les emoji,
 * 0 pour les accents combinants. Indispensable pour aligner des boîtes,
 * car printf("%-20s") compte des octets, pas des colonnes.
 */
size_t nh_display_width(const char *utf8);

/*
 * Copie `s` dans out en complétant avec des espaces pour atteindre `width`
 * colonnes. Ne tronque jamais le texte ; retourne le nombre d'octets écrits
 * (hors '\0'), tronqué à la taille de out si besoin.
 */
size_t nh_pad(char *out, size_t out_size, const char *s, size_t width, NhAlign align);

/* Affiche le texte caractère par caractère (sans pause en mode rapide). */
void nh_typewriter(const char *text, unsigned delay_ms);

#endif /* NH_TERM_H */
