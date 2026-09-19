#ifndef NH_SETTINGS_H
#define NH_SETTINGS_H

/*
 * Réglages du joueur (langue, couleurs, animations, barres fixes), enregistrés dans
 * settings.cfg pour survivre d'une session à l'autre. NhSettings est défini dans config.h,
 * avec l'application sur la configuration (la ligne de commande garde le dernier mot).
 *
 * Un fichier absent, illisible ou à moitié faux ne bloque jamais le jeu : chaque valeur
 * invalide est simplement ignorée et garde son défaut.
 */

#include <stdbool.h>

#include "config.h"

/* Fusionne le fichier dans `s` (les clés absentes ou invalides le laissent inchangé). Faux si le fichier est absent/illisible. */
bool nh_settings_load(NhSettings *s, const char *path);

/* Enregistre (écriture atomique). Faux en cas d'échec. */
bool nh_settings_save(const NhSettings *s, const char *path);

/* Version texte, pour les tests. `out` est terminé par '\0'. */
void nh_settings_to_text(const NhSettings *s, char *out, size_t size);
void nh_settings_from_text(NhSettings *s, const char *text, size_t len);

#endif /* NH_SETTINGS_H */
