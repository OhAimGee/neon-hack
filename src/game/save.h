#ifndef NH_SAVE_H
#define NH_SAVE_H

/*
 * Sauvegarde de la partie : un fichier texte « clé=valeur » (core/kv.h), lisible et versionné.
 *
 * Pourquoi pas un simple vidage de la structure ? Parce qu'il dépendrait du compilateur, du
 * système et de la moindre modification d'un champ, et qu'un fichier abîmé pourrait y glisser
 * des indices hors tableau. Ici :
 *   - on n'enregistre que ce que le jeu MODIFIE (les tables fixes — systèmes, quêtes, contacts,
 *     boutique — sont reconstruites par init_game) : une mise à jour du contenu ne casse rien ;
 *   - le chargement part d'un état neuf, valide chaque valeur (bornes) et n'écrase la partie en
 *     cours que si TOUT est bon : un fichier corrompu ne laisse jamais un état à moitié chargé ;
 *   - une clé absente garde sa valeur par défaut, une clé inconnue est ignorée : les sauvegardes
 *     restent lisibles quand on ajoute des champs (NH_SAVE_VERSION ne bouge que pour un changement
 *     incompatible) ;
 *   - la première ligne porte la version, la dernière (« end=1 ») prouve que rien n'est tronqué.
 *
 * L'écriture est atomique (core/storage.h). Les parties perdues ne sont pas enregistrées : après
 * un game over, « Continuer » reprend la dernière sauvegarde, celle d'avant la commande fatale.
 */

#include <stdbool.h>
#include <stddef.h>

#include "game.h"

#define NH_SAVE_VERSION 1
#define NH_SAVE_MAX_BYTES (256 * 1024)

typedef enum
{
    NH_SAVE_OK,
    NH_SAVE_NO_PATH, /* aucun chemin de sauvegarde : fonction désactivée */
    NH_SAVE_IO,      /* fichier inaccessible ou écriture impossible */
    NH_SAVE_MISSING, /* pas de sauvegarde */
    NH_SAVE_CORRUPT, /* contenu illisible, tronqué ou incohérent */
    NH_SAVE_TOO_NEW  /* écrite par une version plus récente du jeu */
} NhSaveStatus;

/* Ce que le menu affiche pour « Continuer », sans charger toute la partie. */
typedef struct
{
    char name[MAX_NAME_LENGTH];
    int level;
    int credits;
    bool tutorial_active;
} NhSaveInfo;

/* Écrit la partie dans gs->save_path. NH_SAVE_NO_PATH si le chemin est vide. */
NhSaveStatus nh_save_game(const GameState *gs);

/*
 * Charge `path` dans `gs` (gs->save_path est conservé). En cas d'échec `gs` n'est PAS modifié.
 * Sur succès la partie est prête : elle n'est ni terminée ni en pause.
 */
NhSaveStatus nh_load_game(GameState *gs, const char *path);

/* Lit l'essentiel d'une sauvegarde (nom, niveau, crédits) pour le menu. */
NhSaveStatus nh_save_peek(const char *path, NhSaveInfo *info);

/* Versions « en mémoire » (tests et outils). Le texte de nh_save_to_text est à libérer avec free ; NULL si la mémoire manque. */
char *nh_save_to_text(const GameState *gs);
NhSaveStatus nh_save_from_text(GameState *gs, const char *text, size_t len);
NhSaveStatus nh_save_peek_text(const char *text, size_t len, NhSaveInfo *info);

#endif /* NH_SAVE_H */
