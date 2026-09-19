#ifndef SHOP_H
#define SHOP_H

#include <stdbool.h>

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

// Fonctions de la boutique (l'affichage est dans shop_view.[ch])
void init_shop(CyberShop *shop);
bool buy_item(CyberShop *shop, ShopItemType item_type, int *player_credits, int player_level);
void use_item(ShopItemType item_type, void *player_data);

#endif // SHOP_H
