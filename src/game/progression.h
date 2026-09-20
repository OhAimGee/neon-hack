#ifndef NH_PROGRESSION_H
#define NH_PROGRESSION_H

/*
 * Progression du joueur : une courbe d'expérience explicite, un plafond de niveau et UN seul
 * point d'entrée pour gagner de l'expérience (nh_grant_xp). Tout gain passe par là : c'est ce
 * qui déclenche les montées de niveau et leurs déblocages (avant, plusieurs commandes
 * ajoutaient l'expérience directement et contournaient les niveaux).
 *
 * Les fonctions de courbe (nh_level_*, nh_scan_xp) sont pures et testables seules.
 */

#include <stdbool.h>

#include "game.h"

#define NH_LEVEL_MAX 6        /* Légende : niveau maximal */
#define NH_XP_CAP 1000000     /* l'expérience cumulée ne dépasse jamais cette valeur */

/*
 * Expérience CUMULÉE nécessaire pour atteindre `level` (0 pour le niveau 1).
 * -1 pour un niveau hors de 1..NH_LEVEL_MAX.
 */
int nh_level_xp_required(int level);

/* Niveau atteint avec `xp` points cumulés (1 si xp <= 0, NH_LEVEL_MAX au plus). */
int nh_level_for_xp(int xp);

/* Nom du niveau (« Novice », « Apprenti »…) dans la langue courante ; "" hors bornes. */
const char *nh_level_name(int level);

/*
 * Expérience rapportée par le (scans_done+1)-ième scan. Elle décroît puis s'éteint : sans
 * cela, `scan` en boucle rapportait de l'expérience à l'infini.
 */
int nh_scan_xp(int scans_done);

/* Récompenses uniques (une seule fois par partie). */
typedef enum
{
    NH_MS_DECRYPT_TEST,     /* premier message de test décrypté */
    NH_MS_QUANTUM_FIRST,    /* premier décryptage quantique */
    NH_MS_CLASSIFIED_DOC,   /* document ultra-secret lu */
    NH_MS_COUNT
} NhMilestone;

/* Vrai la première fois seulement (et enregistre le jalon, puis émet NH_EV_MILESTONE). */
bool nh_milestone_claim(GameState *gs, NhMilestone milestone);

/*
 * Modifie la réputation de `amount` (négatif : elle baisse) et émet NH_EV_REPUTATION. Un gain
 * est annoncé (« [+20 réputation] », ligne laissée ouverte comme nh_grant_xp) ; une perte est
 * silencieuse : à l'appelant de la raconter. Bornée à ±NH_REPUTATION_CAP. 0 : sans effet.
 */
#define NH_REPUTATION_CAP 1000000
void nh_grant_reputation(GameState *gs, int amount);

/*
 * Accélérateur neuronal (boutique) : les NH_XP_BOOST_CHARGES gains d'expérience suivants sont
 * majorés de leur propre montant, sans dépasser NH_XP_BOOST_BONUS_CAP points de plus. Le boost ne
 * crée pas d'expérience : il double des gains que l'état du jeu limite déjà (règle anti-farm,
 * voir ARCHITECTURE.md), et il ne se cumule pas au-delà de NH_XP_BOOST_CHARGES.
 */
#define NH_XP_BOOST_CHARGES 3
#define NH_XP_BOOST_BONUS_CAP 30

/*
 * Ajoute `amount` (> 0) d'expérience, annonce le gain, puis applique autant de montées de
 * niveau que la courbe le permet (un gros gain peut en enchaîner plusieurs), chacune avec
 * ses déblocages. Retourne le nombre de niveaux gagnés. amount <= 0 : sans effet.
 * Le gain consomme une charge de boost s'il en reste (c'est le cas des piratages).
 */
int nh_grant_xp(GameState *gs, int amount);

/* Comme nh_grant_xp, sans boost : les récompenses de quête ne sont pas des « hacks ». */
int nh_grant_xp_flat(GameState *gs, int amount);

#endif /* NH_PROGRESSION_H */
