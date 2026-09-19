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

/*
 * Jauge de `width` cases (« ██████░░░░ ») pour `value` sur `max` : au moins une case
 * pleine dès que value > 0, valeurs hors bornes ramenées dans [0, max].
 * Tient dans out_size sans jamais couper un caractère.
 */
void nh_gauge(char *out, size_t out_size, int value, int max, int width);

/*
 * Coupe `s` (texte brut, sans séquences ANSI) pour qu'il tienne dans `max_width`
 * colonnes, sans jamais couper un caractère UTF-8 en deux ni un emoji large.
 * Retourne la largeur obtenue.
 */
size_t nh_truncate_width(char *s, size_t max_width);

/*
 * Coupe `text` aux espaces pour qu'aucune ligne ne dépasse `width` colonnes d'affichage ; les
 * lignes suivantes commencent par `indent` espaces. `start_col` = colonnes déjà occupées sur la
 * première ligne (un préfixe déjà affiché). Un mot plus long que la ligne reste entier sur la
 * sienne. `width` = 0 : aucune coupure. Les "\n" du texte sont conservés. Écrit dans `out`
 * (toujours terminé par '\0', tronqué si trop petit) et retourne la longueur en octets.
 */
size_t nh_wrap_text(char *out, size_t out_size, const char *text, size_t start_col, size_t indent,
                    size_t width);

/*
 * Largeur à donner à nh_wrap_text : celle du terminal moins une colonne (pour ne pas provoquer
 * de retour automatique), ou 0 — aucune coupure — si la sortie n'est pas un terminal (tubes,
 * journaux : le texte reste sur une ligne, plus simple à relire et à tester).
 */
size_t nh_wrap_width(void);

/* Affiche le texte caractère par caractère (sans pause en mode rapide). */
void nh_typewriter(const char *text, unsigned delay_ms);

#endif /* NH_TERM_H */
