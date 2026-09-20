#ifndef SHOP_H
#define SHOP_H

/*
 * La boutique de R4Z0R : le catalogue (prix, niveau, unique ou consommable) et l'achat. Chaque
 * objet a un EFFET réel sur la partie, décrit par nh_shop_buy() : plus de « Activation de
 * l'objet... » sans conséquence. L'affichage du catalogue est dans shop_view.[ch].
 *
 * Valeurs (prix, plafonds, durées) provisoires : l'équilibrage se fait en phase 5.
 */

#include <stdbool.h>

struct GameState;

// Types d'objets dans la boutique
typedef enum
{
    ITEM_STEALTH_UPGRADE,
    ITEM_ALERT_REDUCER,
    ITEM_VIRUS_PACK,
    ITEM_PROXY_CHAIN,
    ITEM_ENCRYPTION_KEY,
    ITEM_AI_MODULE,
    ITEM_QUANTUM_CHIP,
    ITEM_REPUTATION_BOOST,
    ITEM_XP_BOOST,
    ITEM_VPN_SERVICE,
    ITEM_COUNT
} ShopItemType;

typedef struct
{
    ShopItemType type;
    char name[50]; // nom propre (marque) : identique dans les deux langues ; le résumé est dans shop_view.c
    int price;
    int level_required;
    bool is_consumable;
    bool is_available;
} ShopItem;

typedef struct
{
    ShopItem items[ITEM_COUNT];
    int item_count;
    bool is_open;
    unsigned bought; // bit i : l'objet i a déjà été acheté au moins une fois (sauvegardé ; lu par les quêtes)
    char vendor_name[50];
    char shop_location[100];
} CyberShop;

/* Effets des objets (les plafonds évitent d'acheter ce qui ne servirait plus à rien). */
#define NH_STEALTH_MAX 10          /* affiché « n/10 » par `status` */
#define NH_STEALTH_MODULE_BONUS 2  /* Stealth Module v2.0 */
#define NH_GHOST_MAX 5             /* Ghost Protocols en réserve (le menu de laylow en consomme un pour -30 d'alerte) */
#define NH_PROXY_HACKS 5           /* hacks couverts par une Proxy Chain */
#define NH_PROXY_HACKS_MAX 15      /* réserve maximale de hacks sous proxy */
#define NH_REPUTATION_BOOST_POINTS 20

typedef enum
{
    NH_BUY_OK,
    NH_BUY_INVALID,     /* numéro d'objet hors catalogue */
    NH_BUY_SOLD_OUT,    /* objet unique déjà acheté */
    NH_BUY_LOW_LEVEL,
    NH_BUY_LOW_CREDITS,
    NH_BUY_MAXED        /* l'objet ne changerait plus rien (réserve pleine, équipement déjà possédé) */
} NhBuyResult;

/* Remplit le catalogue : tout est en vente, rien n'est acheté. */
void init_shop(CyberShop *shop);

/*
 * Achète `type` : vérifie (dans l'ordre épuisé, niveau, crédits, plafond), débite, applique l'effet,
 * émet NH_EV_ITEM_BOUGHT et annonce tout à l'écran (FR/EN). Un refus est expliqué et ne coûte rien.
 */
NhBuyResult nh_shop_buy(struct GameState *gs, ShopItemType type);

#endif // SHOP_H
