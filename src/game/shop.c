#include "shop.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

// Codes couleur pour l'interface cyberpunk
#define COLOR_RESET "\033[0m"
#define COLOR_CYAN "\033[36m"
#define COLOR_YELLOW "\033[33m"
#define COLOR_GREEN "\033[32m"
#define COLOR_RED "\033[31m"

void init_shop(CyberShop *shop)
{
    strcpy(shop->vendor_name, "R4Z0R");
    strcpy(shop->shop_location, "Underground Market - Sector 7");
    shop->is_open = true;
    shop->bought = 0;
    shop->item_count = ITEM_COUNT;

    // Amélioration furtivité
    shop->items[ITEM_STEALTH_UPGRADE].type = ITEM_STEALTH_UPGRADE;
    strcpy(shop->items[ITEM_STEALTH_UPGRADE].name, "Stealth Module v2.0");
    shop->items[ITEM_STEALTH_UPGRADE].price = 150;
    shop->items[ITEM_STEALTH_UPGRADE].level_required = 2;
    shop->items[ITEM_STEALTH_UPGRADE].is_consumable = false;
    shop->items[ITEM_STEALTH_UPGRADE].is_available = true;

    // Réducteur d'alerte
    shop->items[ITEM_ALERT_REDUCER].type = ITEM_ALERT_REDUCER;
    strcpy(shop->items[ITEM_ALERT_REDUCER].name, "Ghost Protocol");
    shop->items[ITEM_ALERT_REDUCER].price = 80;
    shop->items[ITEM_ALERT_REDUCER].level_required = 1;
    shop->items[ITEM_ALERT_REDUCER].is_consumable = true;
    shop->items[ITEM_ALERT_REDUCER].is_available = true;

    // Pack de virus
    shop->items[ITEM_VIRUS_PACK].type = ITEM_VIRUS_PACK;
    strcpy(shop->items[ITEM_VIRUS_PACK].name, "Malware Arsenal");
    shop->items[ITEM_VIRUS_PACK].price = 120;
    shop->items[ITEM_VIRUS_PACK].level_required = 3;
    shop->items[ITEM_VIRUS_PACK].is_consumable = true;
    shop->items[ITEM_VIRUS_PACK].is_available = true;

    // Chaîne de proxies
    shop->items[ITEM_PROXY_CHAIN].type = ITEM_PROXY_CHAIN;
    strcpy(shop->items[ITEM_PROXY_CHAIN].name, "Proxy Chain Pro");
    shop->items[ITEM_PROXY_CHAIN].price = 100;
    shop->items[ITEM_PROXY_CHAIN].level_required = 2;
    shop->items[ITEM_PROXY_CHAIN].is_consumable = true;
    shop->items[ITEM_PROXY_CHAIN].is_available = true;

    // Clé de chiffrement
    shop->items[ITEM_ENCRYPTION_KEY].type = ITEM_ENCRYPTION_KEY;
    strcpy(shop->items[ITEM_ENCRYPTION_KEY].name, "Quantum Encryption Key");
    shop->items[ITEM_ENCRYPTION_KEY].price = 200;
    shop->items[ITEM_ENCRYPTION_KEY].level_required = 3;
    shop->items[ITEM_ENCRYPTION_KEY].is_consumable = false;
    shop->items[ITEM_ENCRYPTION_KEY].is_available = true;

    // Module IA
    shop->items[ITEM_AI_MODULE].type = ITEM_AI_MODULE;
    strcpy(shop->items[ITEM_AI_MODULE].name, "Neural Assistant v3.1");
    shop->items[ITEM_AI_MODULE].price = 500;
    shop->items[ITEM_AI_MODULE].level_required = 4;
    shop->items[ITEM_AI_MODULE].is_consumable = false;
    shop->items[ITEM_AI_MODULE].is_available = true;

    // Puce quantique
    shop->items[ITEM_QUANTUM_CHIP].type = ITEM_QUANTUM_CHIP;
    strcpy(shop->items[ITEM_QUANTUM_CHIP].name, "Quantum Processing Chip");
    shop->items[ITEM_QUANTUM_CHIP].price = 800;
    shop->items[ITEM_QUANTUM_CHIP].level_required = 5;
    shop->items[ITEM_QUANTUM_CHIP].is_consumable = false;
    shop->items[ITEM_QUANTUM_CHIP].is_available = true;

    // Boost de réputation
    shop->items[ITEM_REPUTATION_BOOST].type = ITEM_REPUTATION_BOOST;
    strcpy(shop->items[ITEM_REPUTATION_BOOST].name, "Street Cred Booster");
    shop->items[ITEM_REPUTATION_BOOST].price = 75;
    shop->items[ITEM_REPUTATION_BOOST].level_required = 1;
    shop->items[ITEM_REPUTATION_BOOST].is_consumable = true;
    shop->items[ITEM_REPUTATION_BOOST].is_available = true;

    // Boost d'XP
    shop->items[ITEM_XP_BOOST].type = ITEM_XP_BOOST;
    strcpy(shop->items[ITEM_XP_BOOST].name, "Neural Accelerator");
    shop->items[ITEM_XP_BOOST].price = 90;
    shop->items[ITEM_XP_BOOST].level_required = 2;
    shop->items[ITEM_XP_BOOST].is_consumable = true;
    shop->items[ITEM_XP_BOOST].is_available = true;

    // Service VPN
    shop->items[ITEM_VPN_SERVICE].type = ITEM_VPN_SERVICE;
    strcpy(shop->items[ITEM_VPN_SERVICE].name, "Dark Web VPN");
    shop->items[ITEM_VPN_SERVICE].price = 60;
    shop->items[ITEM_VPN_SERVICE].level_required = 1;
    shop->items[ITEM_VPN_SERVICE].is_consumable = true;
    shop->items[ITEM_VPN_SERVICE].is_available = true;
}

bool buy_item(CyberShop *shop, ShopItemType item_type, int *player_credits, int player_level)
{
    if (item_type < 0 || item_type >= ITEM_COUNT)
    {
        printf(COLOR_RED "❌ Objet invalide!" COLOR_RESET "\n");
        return false;
    }

    ShopItem *item = &shop->items[item_type];

    if (!item->is_available)
    {
        printf(COLOR_RED "❌ Cet objet n'est pas disponible!" COLOR_RESET "\n");
        return false;
    }

    if (player_level < item->level_required)
    {
        printf(COLOR_RED "❌ Niveau insuffisant! Niveau %d requis." COLOR_RESET "\n", item->level_required);
        return false;
    }

    if (*player_credits < item->price)
    {
        printf(COLOR_RED "❌ Crédits insuffisants! Prix: %d ¢" COLOR_RESET "\n", item->price);
        return false;
    }

    // Effectuer l'achat
    *player_credits -= item->price;
    shop->bought |= 1u << (unsigned)item_type;

    printf(COLOR_GREEN "✅ Achat réussi: %s pour %d ¢" COLOR_RESET "\n", item->name, item->price);
    printf(COLOR_CYAN "💰 Crédits restants: %d ¢" COLOR_RESET "\n", *player_credits);

    // Si l'objet n'est pas consommable, le marquer comme indisponible
    if (!item->is_consumable)
    {
        item->is_available = false;
        printf(COLOR_YELLOW "⚠️  Cet objet unique n'est plus disponible à l'achat." COLOR_RESET "\n");
    }

    return true;
}

void use_item(ShopItemType item_type, void *player_data)
{
    // Supprimer le warning pour les paramètres non utilisés
    (void)item_type;
    (void)player_data;

    // Cette fonction sera implémentée dans le fichier principal
    // car elle nécessite accès à la structure Player complète
    printf(COLOR_CYAN "🔧 Activation de l'objet..." COLOR_RESET "\n");
}
