#ifndef NH_LINEEDIT_H
#define NH_LINEEDIT_H

/*
 * Saisie d'une ligne « comme dans un vrai terminal » : édition (flèches, Début/Fin, Suppr,
 * Ctrl+A/E/U/K/W), historique (flèches haut/bas) et complétion par TAB à la manière de bash.
 *
 * Deux couches :
 *
 *  - Le CŒUR (nh_le_*) est pur : il reçoit des octets, tient la ligne à jour et « écrit » dans un
 *    tampon ce qu'il faudrait afficher. Aucune entrée-sortie, donc testable octet par octet, sans
 *    terminal.
 *
 *  - nh_lineedit_read() y branche le vrai terminal (mode brut termios, read/write). Quand l'entrée
 *    n'est pas un terminal — tubes, scripts, tests, Windows — elle retombe sur nh_read_line() :
 *    rien ne change pour eux, TAB compris (le caractère est alors simplement lu).
 *
 * La complétion ne connaît pas le jeu : l'appelant fournit une fonction qui dit, pour le texte
 * situé avant le curseur, quels mots conviennent (voir NhCompleteFn).
 *
 * Affichage : la ligne tient toujours sur UNE rangée de l'écran. Quand elle est plus longue, la
 * fenêtre visible défile pour garder le curseur en vue ; on ne passe jamais à la ligne. C'est ce
 * qui permet de redessiner sans jamais remonter le curseur, donc de cohabiter avec les barres fixes
 * du HUD (pas de « effacer jusqu'en bas d'écran » qui emporterait la barre du bas).
 */

#include <stdbool.h>
#include <stddef.h>

#include "../core/io.h"

#define NH_LE_LINE_MAX 256       /* taille maximale d'une ligne (octets, NUL compris) */
#define NH_LE_MAX_CANDIDATES 64
#define NH_LE_CANDIDATE_LEN 48
#define NH_LE_HISTORY 32
#define NH_LE_OUT_SIZE 8192

/* Les mots proposés pour le texte situé avant le curseur. */
typedef struct
{
    size_t start; /* octet de `before` où commence le mot à remplacer (jusqu'au curseur) */
    size_t count;
    bool space;   /* ajouter une espace quand un seul mot convient (un nom de commande, pas un argument final) */
    char items[NH_LE_MAX_CANDIDATES][NH_LE_CANDIDATE_LEN];
} NhCompletions;

/*
 * Remplit `out` pour la ligne `before` (le texte avant le curseur, sans ce qui le suit). Chaque
 * mot proposé commence par le texte à remplacer, sans tenir compte de la casse. `out->count` = 0
 * si rien ne convient. Le mot inséré prend la casse du candidat.
 */
typedef void (*NhCompleteFn)(void *ctx, const char *before, NhCompletions *out);

typedef enum
{
    NH_LE_CONTINUE, /* la ligne est en cours de saisie */
    NH_LE_DONE,     /* Entrée : la ligne est dans le tampon */
    NH_LE_EOF       /* Ctrl+D sur une ligne vide */
} NhLeResult;

typedef struct
{
    /* Configuration, fixée par nh_le_init(). */
    const char *prompt;
    char *buf; /* la ligne, toujours terminée par '\0' */
    size_t size;
    NhCompleteFn complete;
    void *ctx;
    int cols; /* largeur de l'écran ; l'appelant la met à jour (0 : 80) */

    /* État. */
    size_t len;
    size_t cursor; /* en octets, toujours sur une frontière de caractère */
    bool after_tab; /* la touche précédente était TAB (un second TAB liste les candidats) */
    int history_pos; /* -1 : la ligne en cours ; n : la n-ième plus récente */
    char draft[NH_LE_LINE_MAX]; /* la ligne en cours, mise de côté pendant qu'on parcourt l'historique */
    int esc;                    /* analyseur : 0 rien, 1 après ESC, 2 dans « ESC [ », 3 après « ESC O » */
    int csi_param;
    bool csi_more;
    unsigned char utf8[4];
    size_t utf8_have;
    size_t utf8_need;

    /* Ce qu'il faut afficher, à écrire tel quel sur le terminal puis à vider. */
    char out[NH_LE_OUT_SIZE];
    size_t out_len;
} NhLineEdit;

/* ---- Cœur ------------------------------------------------------------------------------------ */

/* `size` est ramené à NH_LE_LINE_MAX au plus. `complete` peut être NULL (TAB sonne alors). */
void nh_le_init(NhLineEdit *ed, const char *prompt, char *buf, size_t size, NhCompleteFn complete,
                void *ctx);

/* Écrit dans out le prompt puis la ligne (vide au départ). */
void nh_le_start(NhLineEdit *ed);

/* Traite un octet reçu du clavier. Après NH_LE_DONE ou NH_LE_EOF, ne plus rien envoyer. */
NhLeResult nh_le_feed(NhLineEdit *ed, unsigned char byte);

/* Un ESC est en attente de sa suite : à l'appelant de décider, après un court délai, que c'était la touche seule. */
bool nh_le_escape_pending(const NhLineEdit *ed);
void nh_le_escape_timeout(NhLineEdit *ed);

const char *nh_le_output(const NhLineEdit *ed);
void nh_le_output_clear(NhLineEdit *ed);

/* Historique de la session (partagé par toutes les saisies). Les lignes vides et les doublons consécutifs sont ignorés. */
void nh_le_history_add(const char *line);
void nh_le_history_clear(void);
size_t nh_le_history_count(void);

/* ---- Terminal -------------------------------------------------------------------------------- */

/* Vrai si nh_lineedit_read() éditera vraiment la ligne : entrée et sortie sont un terminal, non « dumb », pas de flux de test. */
bool nh_lineedit_interactive(void);

/*
 * Affiche `prompt` puis lit une ligne, avec édition et complétion si le terminal s'y prête, sinon
 * exactement comme nh_read_line(). Même contrat : ligne sans « \n », NH_IO_EOF sur fin d'entrée.
 */
NhIoStatus nh_lineedit_read(const char *prompt, char *buf, size_t size, NhCompleteFn complete, void *ctx);

#endif /* NH_LINEEDIT_H */
