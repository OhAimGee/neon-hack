#ifndef NH_ALERT_H
#define NH_ALERT_H

/*
 * Niveau d'alerte du joueur : UNE seule échelle, 0 à 100, stockée ici et nulle
 * part ailleurs (l'ancien code en avait trois qui se réécrasaient).
 *
 * Les fonctions nh_alert_add / _reduce / _decay ne touchent qu'à la structure :
 * pas d'affichage, pas de hasard, donc testables telles quelles. L'affichage
 * est dans nh_alert_raise() et les nh_alert_print_*().
 */

#include <stdbool.h>
#include <stddef.h>

#include "../i18n/i18n.h"
#include "../ui/term.h"

#define NH_ALERT_MAX 100      /* atteindre ce niveau = fin de partie */
#define NH_ALERT_WARNING 30   /* bande « ATTENTION » (jaune) */
#define NH_ALERT_ELEVATED 50  /* le message « niveau élevé » apparaît */
#define NH_ALERT_DANGER 70    /* bande « DANGER » (rouge) */
#define NH_ALERT_CRITICAL 80  /* boutique fermée, alerte « critique » */

typedef struct
{
    int level;     /* 0..NH_ALERT_MAX */
    int max_level; /* plus haut niveau atteint dans la partie */
    bool vpn_active;
    bool proxy_active;
    int ghost_protocols_available;
} AlertSystem;

typedef enum
{
    NH_ALERT_SAFE,
    NH_ALERT_WARN,
    NH_ALERT_DANGER_BAND
} NhAlertBand;

typedef enum
{
    NH_REDUCTION_TIME,
    NH_REDUCTION_VPN,
    NH_REDUCTION_PROXY,
    NH_REDUCTION_GHOST,
    NH_REDUCTION_LAYLOW,
    NH_REDUCTION_FRAME,
    NH_REDUCTION_COUNT
} NhReduction;

typedef enum
{
    NH_REDUCE_OK,
    NH_REDUCE_NO_CREDITS,
    NH_REDUCE_NO_GHOST
} NhReduceResult;

void nh_alert_init(AlertSystem *a);

/* Ajoute `amount` (les valeurs négatives sont ignorées). Retourne la hausse réellement appliquée. */
int nh_alert_add(AlertSystem *a, int amount);
/* Retire `amount` (les valeurs négatives sont ignorées). Retourne la baisse réellement appliquée. */
int nh_alert_reduce(AlertSystem *a, int amount);
/* Refroidissement naturel, une fois par action de hacking : 1, +1 avec VPN, +1 avec proxy. */
int nh_alert_decay(AlertSystem *a);

NhAlertBand nh_alert_band(int level);
bool nh_alert_is_game_over(const AlertSystem *a);
bool nh_alert_shop_closed(const AlertSystem *a);
/* Malus (en points de pourcentage) sur les chances de réussite d'un hack. */
int nh_alert_success_penalty(const AlertSystem *a);

/* Couleur et libellé de la bande (SÉCURISÉ / ATTENTION / DANGER). */
NhColor nh_alert_color(int level);
NhStr nh_alert_label(int level);

int nh_alert_reduction_cost(NhReduction method);
int nh_alert_reduction_amount(NhReduction method);
/* Applique une méthode : débite les crédits, baisse l'alerte, active VPN/proxy. */
NhReduceResult nh_alert_apply_reduction(AlertSystem *a, NhReduction method, int *credits,
                                        int *applied);

/* Jauge de `width` colonnes (« ██████░░░░ »), au moins une case pleine dès que level > 0. */
void nh_alert_bar(char *out, size_t out_size, int level, int width);

/* Ajoute de l'alerte ET l'annonce (« [ALERTE +5] », puis les avertissements de seuil). */
void nh_alert_raise(AlertSystem *a, int amount);
void nh_alert_print_status(const AlertSystem *a);
void nh_alert_print_menu(const AlertSystem *a);

#endif /* NH_ALERT_H */
