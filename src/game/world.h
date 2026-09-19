#ifndef NH_WORLD_H
#define NH_WORLD_H

/*
 * Le monde : UN seul graphe de systèmes attaquables.
 *
 * Avant, les commandes classiques (bruteforce, backdoor…) visaient 3 nœuds et `advhack` 5 autres
 * cibles sans lien, sans état persistant (on pouvait re-pirater la même cible à l'infini). Tout
 * passe maintenant par `gs->nodes[]` :
 *   - chaque système a un niveau de découverte (`scan`) et un relais (« uplink ») : il faut avoir
 *     compromis le relais pour l'atteindre, sauf via `exploit` qui perce directement ;
 *   - l'état (compromis, backdoor, virus, tracé, accès internes, fichiers extraits) est
 *     persistant : la récompense d'un système n'est versée qu'une fois ;
 *   - les méthodes classiques ont un unique calcul de chance (nh_world_chance), avec bornes ;
 *   - les 5 systèmes qui ont un profil de défense avancé (AdvancedTarget) sont reliés par
 *     nh_world_adv_target(), et `advhack` et compagnie attaquent ces mêmes nœuds.
 *
 * Les données sont une table `static const` (world.c) ; rien ici ne lit ni n'écrit de fichier.
 */

#include <stdbool.h>

#include "game.h"

/* Méthodes d'attaque « classiques » : leurs formules de chance vivent toutes dans world.c. */
typedef enum
{
    NH_HACK_BRUTE,
    NH_HACK_BACKDOOR,
    NH_HACK_VIRUS,
    NH_HACK_AI,
    NH_HACK_EXPLOIT,
    NH_HACK_METHOD_COUNT
} NhHackMethod;

#define NH_CHANCE_MIN 5  /* jamais impossible… */
#define NH_CHANCE_MAX 95 /* …et jamais garanti */
#define NH_INTEL_BONUS 15 /* points de chance apportés par des accès internes (socialeng) */

/* Nombre de systèmes du monde (= NH_MAX_NODES). */
int nh_world_count(void);

/* Remplit `gs->nodes` depuis la table : tout est vierge, rien n'est découvert. */
void nh_world_init(NetworkNode nodes[NH_MAX_NODES]);

/* Nom lisible d'un niveau de sécurité (langue courante). */
const char *nh_security_label(SecurityLevel level);

/* --- Le graphe --------------------------------------------------------------------------- */

/* Relais nécessaire pour atteindre `idx` (-1 : accessible d'emblée ; -1 aussi si idx invalide). */
int nh_world_uplink(int idx);

/* Niveau du joueur à partir duquel `scan` révèle `idx` (0 si idx invalide). */
int nh_world_min_level(int idx);

/* Indice de la cible avancée (AdvancedHackingSystem.targets) de `idx`, ou -1 s'il n'en a pas. */
int nh_world_adv_target(int idx);

/* La route est ouverte : pas de relais, ou relais compromis. */
bool nh_world_reachable(const NetworkNode nodes[NH_MAX_NODES], int idx);

/* Marque découverts tous les systèmes visibles au niveau `level` ; renvoie le nombre de nouveaux. */
int nh_world_discover(NetworkNode nodes[NH_MAX_NODES], int level);

/* Indice du système DÉCOUVERT de ce nom (comparaison exacte), ou -1. */
int nh_world_find(const NetworkNode nodes[NH_MAX_NODES], const char *name);

/*
 * Résout l'argument d'une commande en indice de système, et explique à l'écran pourquoi pas :
 * inconnu / non découvert, ou (si `need_route`) route fermée. Renvoie -1 en cas d'échec.
 */
int nh_world_resolve(const GameState *gs, const char *name, bool need_route);

/* --- Attaques ---------------------------------------------------------------------------- */

/*
 * Chance de succès (en %) d'une méthode classique contre `node`, bornée à
 * [NH_CHANCE_MIN, NH_CHANCE_MAX]. `bonus` (négatif = malus) sert aux modificateurs du moment :
 * pénalité d'alerte, mode furtif, discrétion du virus… Les accès internes de `node` s'y ajoutent.
 */
int nh_world_chance(NhHackMethod method, const NetworkNode *node, int level, int bonus);

/* Bonus permanent propre au système (accès internes) : 0 ou NH_INTEL_BONUS. */
int nh_world_node_bonus(const NetworkNode *node);

/*
 * Met à jour l'état ACTIF/INACTIF des 8 outils du hacking avancé d'après ce que le joueur
 * possède réellement (avant, aucune commande ne pouvait les activer : toutes les méthodes de
 * `advhack` exigeaient un outil et étaient donc inutilisables). Règles :
 *   ordinateur quantique  → has_quantum_computer      assistant IA        → has_ai_assistant
 *   interface neurale     → synchronisation ≥ 50 %    cape de furtivité   → mode furtif actif
 *   labo de virus         → au moins un virus         base sociale        → accès internes quelque part
 *   exploits zero-day     → commande `exploit` débloquée   protocole fantôme → protocoles achetés
 */
void nh_world_sync_tools(GameState *gs);

/* Nombre de fichiers pas encore extraits. */
int nh_world_locked_files(const NetworkNode *node);

/*
 * Compromet le système `idx` : verse UNE fois ses crédits et son expérience (via nh_grant_xp).
 * Avec `deep`, extrait aussi tous ses fichiers. Renvoie false, sans rien verser, s'il l'était déjà.
 */
bool nh_world_compromise(GameState *gs, int idx, bool deep);

/* Extrait les fichiers restants d'un système ; verse leurs crédits ; renvoie combien. */
int nh_world_extract(GameState *gs, int idx);

#endif /* NH_WORLD_H */
