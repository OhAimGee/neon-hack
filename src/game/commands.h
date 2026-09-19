#ifndef NH_COMMANDS_H
#define NH_COMMANDS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>

#include "../i18n/i18n.h"
#include "game.h"

/* Valeur de `unlock` pour une commande sans drapeau de déblocage. */
#define NH_NO_UNLOCK (-1)

/* Ordre d'affichage dans l'aide. */
typedef enum
{
    NH_CAT_HACK,
    NH_CAT_WORLD,
    NH_CAT_ADVANCED,
    NH_CAT_SYSTEM,
    NH_CAT_COUNT
} NhCmdCategory;

typedef bool (*NhCommandFn)(GameState *gs, const char *arg);

/* Ce que la commande attend comme argument : c'est ce que la touche TAB propose (voir complete.h). */
typedef enum
{
    NH_ARG_NONE,    /* aucun, ou un texte libre (decrypt) : rien à proposer */
    NH_ARG_SYSTEM,  /* un système découvert par `scan` */
    NH_ARG_CONTACT, /* un contact débloqué */
    NH_ARG_MESSAGE  /* le numéro d'un message de la boîte de réception */
} NhArgKind;

/*
 * Une commande du jeu. La table (commands.c) est l'unique source de vérité :
 * l'aide, le déblocage par niveau et le dispatch en sont tous dérivés, si bien
 * que le nom affiché dans l'aide est forcément celui qui fonctionne.
 */
typedef struct
{
    const char *name;  /* nom canonique, en minuscules ASCII */
    const char *alias; /* nom alternatif accepté, ou NULL */
    NhCmdCategory category;
    int unlock;        /* indice CMD_* dans player.commands_unlocked, ou NH_NO_UNLOCK */
    int min_level;     /* niveau minimum, 0 = aucun */
    bool hidden;       /* accepté mais absent de l'aide */
    NhStr help;        /* clé de la description (i18n) */
    NhCommandFn fn;
    NhArgKind arg;     /* nature de l'argument (NH_ARG_NONE si omis dans la table) */
} NhCommand;

typedef enum
{
    NH_DISPATCH_OK,      /* commande exécutée, gestionnaire a réussi */
    NH_DISPATCH_FAILED,  /* commande exécutée, gestionnaire a échoué (usage, cible invalide…) */
    NH_DISPATCH_EMPTY,   /* ligne vide */
    NH_DISPATCH_UNKNOWN, /* commande inconnue */
    NH_DISPATCH_LOCKED   /* commande connue mais pas encore débloquée */
} NhDispatch;

/* La table complète (pour l'aide et pour les tests). */
const NhCommand *nh_commands(size_t *count);

/* Recherche par nom ou alias, sans tenir compte de la casse. NULL si inconnue. */
const NhCommand *nh_find_command(const char *name);

/* Le joueur peut-il utiliser cette commande maintenant ? */
bool nh_command_available(const GameState *gs, const NhCommand *cmd);

/* Analyse la ligne, vérifie le déblocage, exécute. Affiche les erreurs elle-même. */
NhDispatch nh_dispatch(GameState *gs, const char *line);

/* Aide : uniquement les commandes disponibles, groupées par catégorie. */
void nh_print_help(const GameState *gs, FILE *out);

/*
 * Met à jour l'interface fixe (barres haut/bas) d'après l'état du jeu : joueur, alerte,
 * commandes disponibles. Sans effet si le HUD n'est pas actif.
 */
void nh_refresh_hud(const GameState *gs);

#endif /* NH_COMMANDS_H */
