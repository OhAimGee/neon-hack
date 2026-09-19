#ifndef NH_STORAGE_H
#define NH_STORAGE_H

/*
 * Fichiers du joueur (sauvegarde, réglages) : où ils vivent et comment les écrire sans
 * jamais laisser un fichier à moitié écrit.
 *
 * Écriture ATOMIQUE : le contenu est d'abord écrit dans « fichier.tmp », forcé sur le disque,
 * puis renomme par-dessus l'ancien. Une coupure de courant, un Ctrl+C ou un disque plein en
 * plein milieu laissent donc l'ancienne sauvegarde intacte, jamais une sauvegarde tronquée.
 *
 * Le code Windows (MoveFileEx, _mkdir) n'a jamais été compilé, comme celui de platform.c.
 */

#include <stdbool.h>
#include <stddef.h>

#define NH_PATH_MAX 512
#define NH_SAVE_FILE "savegame.sav"
#define NH_SETTINGS_FILE "settings.cfg"

typedef struct
{
    bool ok; /* un dossier de données a pu être déterminé (sinon : jeu sans sauvegarde) */
    char dir[NH_PATH_MAX];
    char save[NH_PATH_MAX + 32];
    char settings[NH_PATH_MAX + 32];
} NhPaths;

/*
 * Dossier de données. Ordre : `override_dir` (--data-dir) > %APPDATA%/neon-hack >
 * $XDG_DATA_HOME/neon-hack > $HOME/.local/share/neon-hack. Fonction pure : l'environnement est
 * passé en paramètre (NULL ou "" = absent), pour rester testable. Faux si rien n'est utilisable.
 */
bool nh_storage_dir(char *out, size_t size, const char *override_dir, const char *xdg_data_home,
                    const char *home, const char *appdata);

/* Remplit `p` d'après un dossier ; p->ok est faux si `dir` est vide ou trop long. */
void nh_paths_from_dir(NhPaths *p, const char *dir);

/* Idem, en lisant l'environnement réel. */
void nh_paths_init(NhPaths *p, const char *override_dir);

/* Crée le dossier et ses parents (droits 0700). Vrai s'il existe à la fin. */
bool nh_storage_mkdirs(const char *dir);

bool nh_storage_exists(const char *path);
bool nh_storage_remove(const char *path);

typedef enum
{
    NH_STORAGE_OK,
    NH_STORAGE_MISSING, /* le fichier n'existe pas */
    NH_STORAGE_TOO_BIG, /* plus de `max_len` octets : pas un fichier de ce jeu */
    NH_STORAGE_ERROR    /* lecture impossible, mémoire épuisée */
} NhStorageStatus;

/* Lit tout le fichier dans un tampon alloué (à libérer avec free), terminé par '\0'. */
NhStorageStatus nh_storage_read(const char *path, size_t max_len, char **data, size_t *len);

/* Écrit `data` de façon atomique (crée les dossiers manquants). Faux en cas d'échec ; l'ancien fichier reste alors intact. */
bool nh_storage_write_atomic(const char *path, const char *data, size_t len);

#endif /* NH_STORAGE_H */
