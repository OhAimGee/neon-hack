#ifndef NH_HUD_H
#define NH_HUD_H

/*
 * Interface fixe : une barre d'état en haut (joueur, niveau, crédits, jauge d'alerte),
 * une barre de commandes en bas, et entre les deux la zone de texte qui défile seule.
 * Repose sur la « région de défilement » ANSI (DECSTBM) : aucune bibliothèque.
 *
 * Le HUD ne s'active que sur un vrai terminal d'au moins 80x24. Sinon (sortie
 * redirigée, terminal trop petit, --no-hud, TERM=dumb) tout fonctionne comme avant, et
 * les fonctions nh_hud_* ne font rien.
 *
 * Ce module ne connaît pas le jeu : l'appelant lui passe un NhHudData.
 */

#include <stdbool.h>
#include <stddef.h>

#include "term.h"

#define NH_HUD_MIN_COLS 80
#define NH_HUD_MIN_ROWS 24

typedef struct
{
    const char *name;
    int level;
    int credits;
    int alert;   /* 0..100 */
    NhColor alert_color; /* couleur de la jauge (fournie par le jeu : le HUD ignore les seuils) */
    bool stealth;
    /* Commandes à annoncer en bas, par ordre d'importance : les dernières sautent si l'écran est étroit. */
    const char *const *commands;
    size_t command_count;
} NhHudData;

/* Décision d'activation, pure (testable) : terminal réel, assez grand, non désactivé. */
bool nh_hud_should_enable(bool disabled, bool is_tty, int cols, int rows);

/*
 * Composition des barres : chaque ligne fait exactement `cols` colonnes d'affichage
 * (remplie d'espaces), même avec un nom très long ou des emoji. Retournent la taille écrite.
 */
size_t nh_hud_compose_top(char *out, size_t size, const NhHudData *data, int cols);
size_t nh_hud_compose_bottom(char *out, size_t size, const NhHudData *data, int cols);

/* Active le HUD si possible ; retourne true s'il l'est. Enregistre la remise en état à la sortie. */
bool nh_hud_start(bool disabled);
/* Redessine les barres (et s'adapte à un redimensionnement). Sans effet si inactif. */
void nh_hud_refresh(const NhHudData *data);
/* Désactive le HUD et rend le terminal normal. Idempotent ; appelé aussi par atexit et Ctrl+C. */
void nh_hud_stop(void);
bool nh_hud_active(void);
/* Efface la zone de texte (commande `clear`) sans toucher aux barres. */
void nh_hud_clear_body(void);

#endif /* NH_HUD_H */
