#ifndef NH_MENU_H
#define NH_MENU_H

/*
 * Menu de lancement : Continuer, Nouvelle partie, Langue, Options, Quitter.
 *
 * Il lit ses choix par core/io.h comme le reste du jeu (une entrée fermée est un statut, jamais
 * une boucle infinie) et applique les réglages sur-le-champ : la langue change sous vos yeux,
 * les couleurs et les animations aussi. Chaque réglage est enregistré dès qu'il est modifié
 * (settings.cfg), sans toucher à ce que la ligne de commande a imposé pour cette session.
 *
 * Il tourne avant l'interface fixe : tout ce qu'il affiche défile normalement.
 */

#include <stdbool.h>

#include "../core/config.h"
#include "../core/storage.h"
#include "game.h"

typedef enum
{
    NH_START_PLAY, /* `gs` est prêt : partie chargée ou nouvelle partie commencée */
    NH_START_QUIT, /* le joueur a choisi Quitter */
    NH_START_EOF   /* entrée fermée : l'appelant l'annonce et quitte */
} NhStart;

/*
 * Affiche le menu et traite les choix jusqu'à ce qu'une partie soit prête ou que le joueur parte.
 * `cfg` (session en cours) et `settings` (valeurs enregistrées) évoluent ensemble ; `paths` peut
 * être inutilisable (p->ok faux) : le jeu marche alors sans sauvegarde. `*resumed` vaut vrai si la
 * partie vient d'être rechargée (pour que ECHO-7 salue le joueur).
 */
NhStart nh_menu_run(NhConfig *cfg, NhSettings *settings, const NhPaths *paths, GameState *gs,
                    bool *resumed);

/* « Nouvelle partie » sans menu (--new) : état neuf, prologue, première sauvegarde. */
NhStart nh_start_new_game(GameState *gs, const NhPaths *paths);

#endif /* NH_MENU_H */
