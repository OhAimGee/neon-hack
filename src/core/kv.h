#ifndef NH_KV_H
#define NH_KV_H

/*
 * Format texte « clé=valeur », une paire par ligne : c'est celui des sauvegardes et des
 * réglages. Lisible et modifiable à la main, sans dépendance, et surtout *tolérant* :
 * une clé absente garde sa valeur par défaut, une clé inconnue est ignorée. C'est ce qui
 * permet d'ajouter des champs sans invalider les sauvegardes existantes.
 *
 *   # commentaire (ligne vide ignorée aussi)
 *   player.level=3
 *   player.name=Case
 *
 * Aucune connaissance du jeu ici : ce module ne fait que découper et convertir.
 */

#include <stdbool.h>
#include <stddef.h>

typedef struct
{
    char *key;
    char *value;
} NhKvPair;

typedef struct
{
    char *text; /* copie du texte analysé ; les paires pointent dedans */
    NhKvPair *pairs;
    size_t count;
} NhKv;

typedef enum
{
    NH_KV_OK,
    NH_KV_MISSING, /* clé absente : l'appelant garde sa valeur par défaut */
    NH_KV_INVALID  /* clé présente mais illisible ou hors bornes : le fichier est corrompu */
} NhKvStatus;

/*
 * Analyse `len` octets. Les fins de ligne "\n" et "\r\n" sont acceptées ; la clé est
 * débarrassée de ses espaces, la valeur est gardée telle quelle (après le premier '=').
 * Une ligne sans '=' est ignorée. Faux si la mémoire manque ou s'il y a trop de lignes.
 */
bool nh_kv_parse(NhKv *kv, const char *text, size_t len);
void nh_kv_free(NhKv *kv);

/* Valeur d'une clé (la dernière occurrence l'emporte), ou NULL. */
const char *nh_kv_get(const NhKv *kv, const char *key);

/* Entier décimal strict dans [min, max] (espaces autorisés autour). *out n'est écrit qu'en cas de succès. */
NhKvStatus nh_kv_int(const NhKv *kv, const char *key, long min, long max, long *out);

/* Écrivain : accumule le texte dans un tampon qui grandit tout seul. */
typedef struct
{
    char *data; /* NULL tant que rien n'est écrit : passer par nh_kvw_text() */
    size_t len;
    size_t cap;
    bool failed; /* mémoire épuisée : le contenu n'est plus fiable */
} NhKvWriter;

void nh_kvw_init(NhKvWriter *w);
void nh_kvw_free(NhKvWriter *w);
/* Le texte écrit (jamais NULL, toujours terminé par '\0'). */
const char *nh_kvw_text(const NhKvWriter *w);
void nh_kvw_int(NhKvWriter *w, const char *key, long value);
/* Les caractères de contrôle de la valeur (dont les fins de ligne) sont remplacés par des espaces. */
void nh_kvw_str(NhKvWriter *w, const char *key, const char *value);

#endif /* NH_KV_H */
