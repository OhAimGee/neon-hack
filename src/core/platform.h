#ifndef NH_PLATFORM_H
#define NH_PLATFORM_H

#include <stdbool.h>

/* Prépare la console (Windows : sortie UTF-8 + séquences ANSI ; POSIX : rien). */
void nh_platform_init(void);

/* Mode rapide : toutes les pauses d'animation deviennent des no-op. */
void nh_set_fast(bool fast);
bool nh_is_fast(void);

/* Pause en millisecondes, ignorée en mode rapide. */
void nh_sleep_ms(unsigned ms);

/* La sortie standard est-elle un terminal interactif ? */
bool nh_stdout_is_tty(void);

/* Taille du terminal en colonnes x lignes. Faux si elle est inconnue (pas un terminal). */
bool nh_term_size(int *cols, int *rows);

#endif /* NH_PLATFORM_H */
