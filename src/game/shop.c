#include "shop.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

// Codes couleur pour l'interface cyberpunk
#define COLOR_RESET "\033[0m"
#define COLOR_CYAN "\033[36m"
#define COLOR_MAGENTA "\033[35m"
#define COLOR_YELLOW "\033[33m"
#define COLOR_GREEN "\033[32m"
#define COLOR_RED "\033[31m"
#define COLOR_BRIGHT_CYAN "\033[96m"
#define COLOR_WHITE "\033[37m"

void init_shop(CyberShop *shop)
{
    strcpy(shop->vendor_name, "R4Z0R");
    strcpy(shop->shop_location, "Underground Market - Sector 7");
    shop->is_open = true;
    shop->item_count = ITEM_COUNT;

    // Amélioration furtivité
    shop->items[ITEM_STEALTH_UPGRADE].type = ITEM_STEALTH_UPGRADE;
    strcpy(shop->items[ITEM_STEALTH_UPGRADE].name, "Stealth Module v2.0");
    strcpy(shop->items[ITEM_STEALTH_UPGRADE].description, "Augmente votre furtivité de +2 points définitivement");
    shop->items[ITEM_STEALTH_UPGRADE].price = 150;
    shop->items[ITEM_STEALTH_UPGRADE].level_required = 2;
    shop->items[ITEM_STEALTH_UPGRADE].is_consumable = false;
    shop->items[ITEM_STEALTH_UPGRADE].is_available = true;

    // Réducteur d'alerte
    shop->items[ITEM_ALERT_REDUCER].type = ITEM_ALERT_REDUCER;
    strcpy(shop->items[ITEM_ALERT_REDUCER].name, "Ghost Protocol");
    strcpy(shop->items[ITEM_ALERT_REDUCER].description, "Réduit immédiatement le niveau d'alerte de 2 points");
    shop->items[ITEM_ALERT_REDUCER].price = 80;
    shop->items[ITEM_ALERT_REDUCER].level_required = 1;
    shop->items[ITEM_ALERT_REDUCER].is_consumable = true;
    shop->items[ITEM_ALERT_REDUCER].is_available = true;

    // Pack de virus
    shop->items[ITEM_VIRUS_PACK].type = ITEM_VIRUS_PACK;
    strcpy(shop->items[ITEM_VIRUS_PACK].name, "Malware Arsenal");
    strcpy(shop->items[ITEM_VIRUS_PACK].description, "Pack de 3 virus puissants pour vos intrusions");
    shop->items[ITEM_VIRUS_PACK].price = 120;
    shop->items[ITEM_VIRUS_PACK].level_required = 3;
    shop->items[ITEM_VIRUS_PACK].is_consumable = true;
    shop->items[ITEM_VIRUS_PACK].is_available = true;

    // Chaîne de proxies
    shop->items[ITEM_PROXY_CHAIN].type = ITEM_PROXY_CHAIN;
    strcpy(shop->items[ITEM_PROXY_CHAIN].name, "Proxy Chain Pro");
    strcpy(shop->items[ITEM_PROXY_CHAIN].description, "Protection anonymat +50% pendant 5 hacks");
    shop->items[ITEM_PROXY_CHAIN].price = 100;
    shop->items[ITEM_PROXY_CHAIN].level_required = 2;
    shop->items[ITEM_PROXY_CHAIN].is_consumable = true;
    shop->items[ITEM_PROXY_CHAIN].is_available = true;

    // Clé de chiffrement
    shop->items[ITEM_ENCRYPTION_KEY].type = ITEM_ENCRYPTION_KEY;
    strcpy(shop->items[ITEM_ENCRYPTION_KEY].name, "Quantum Encryption Key");
    strcpy(shop->items[ITEM_ENCRYPTION_KEY].description, "Déchiffre automatiquement les fichiers de niveau 1-2");
    shop->items[ITEM_ENCRYPTION_KEY].price = 200;
    shop->items[ITEM_ENCRYPTION_KEY].level_required = 3;
    shop->items[ITEM_ENCRYPTION_KEY].is_consumable = false;
    shop->items[ITEM_ENCRYPTION_KEY].is_available = true;

    // Module IA
    shop->items[ITEM_AI_MODULE].type = ITEM_AI_MODULE;
    strcpy(shop->items[ITEM_AI_MODULE].name, "Neural Assistant v3.1");
    strcpy(shop->items[ITEM_AI_MODULE].description, "Assistant IA avancé pour hacking automatisé");
    shop->items[ITEM_AI_MODULE].price = 500;
    shop->items[ITEM_AI_MODULE].level_required = 4;
    shop->items[ITEM_AI_MODULE].is_consumable = false;
    shop->items[ITEM_AI_MODULE].is_available = true;

    // Puce quantique
    shop->items[ITEM_QUANTUM_CHIP].type = ITEM_QUANTUM_CHIP;
    strcpy(shop->items[ITEM_QUANTUM_CHIP].name, "Quantum Processing Chip");
    strcpy(shop->items[ITEM_QUANTUM_CHIP].description, "Ordinateur quantique portable pour crypto-analyse");
    shop->items[ITEM_QUANTUM_CHIP].price = 800;
    shop->items[ITEM_QUANTUM_CHIP].level_required = 5;
    shop->items[ITEM_QUANTUM_CHIP].is_consumable = false;
    shop->items[ITEM_QUANTUM_CHIP].is_available = true;

    // Boost de réputation
    shop->items[ITEM_REPUTATION_BOOST].type = ITEM_REPUTATION_BOOST;
    strcpy(shop->items[ITEM_REPUTATION_BOOST].name, "Street Cred Booster");
    strcpy(shop->items[ITEM_REPUTATION_BOOST].description, "Augmente votre réputation de +20 points");
    shop->items[ITEM_REPUTATION_BOOST].price = 75;
    shop->items[ITEM_REPUTATION_BOOST].level_required = 1;
    shop->items[ITEM_REPUTATION_BOOST].is_consumable = true;
    shop->items[ITEM_REPUTATION_BOOST].is_available = true;

    // Boost d'XP
    shop->items[ITEM_XP_BOOST].type = ITEM_XP_BOOST;
    strcpy(shop->items[ITEM_XP_BOOST].name, "Neural Accelerator");
    strcpy(shop->items[ITEM_XP_BOOST].description, "Double l'XP gagné pendant 3 hacks");
    shop->items[ITEM_XP_BOOST].price = 90;
    shop->items[ITEM_XP_BOOST].level_required = 2;
    shop->items[ITEM_XP_BOOST].is_consumable = true;
    shop->items[ITEM_XP_BOOST].is_available = true;

    // Service VPN
    shop->items[ITEM_VPN_SERVICE].type = ITEM_VPN_SERVICE;
    strcpy(shop->items[ITEM_VPN_SERVICE].name, "Dark Web VPN");
    strcpy(shop->items[ITEM_VPN_SERVICE].description, "Service VPN premium - réduit l'alerte passivement");
    shop->items[ITEM_VPN_SERVICE].price = 60;
    shop->items[ITEM_VPN_SERVICE].level_required = 1;
    shop->items[ITEM_VPN_SERVICE].is_consumable = true;
    shop->items[ITEM_VPN_SERVICE].is_available = true;
}

void display_shop(const CyberShop *shop, int player_credits, int player_level)
{
    // Header cyberpunk amélioré
    printf("\n" COLOR_MAGENTA);
    printf("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    printf("║                           🔮 CYBER MARKET 🔮                            ║\n");
    printf("║                          ▓▓▓ R4Z0R'S SHOP ▓▓▓                           ║\n");
    printf("╠═══════════════════════════════════════════════════════════════════════════╣\n");
    printf("║ " COLOR_YELLOW "Vendeur: " COLOR_CYAN "%s" COLOR_MAGENTA "                  " COLOR_GREEN "Crédits: " COLOR_BRIGHT_CYAN "%d ¢" COLOR_MAGENTA "               ║\n",
           shop->vendor_name, player_credits);
    printf("║ " COLOR_YELLOW "Secteur: " COLOR_CYAN "Underground-7" COLOR_MAGENTA "        " COLOR_YELLOW "Niveau: " COLOR_BRIGHT_CYAN "%d" COLOR_MAGENTA "                    ║\n",
           player_level);
    printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    printf(COLOR_RESET);

    // Interface moderne en cartes
    printf("\n" COLOR_BRIGHT_CYAN "▓▓▓ CATALOGUE CYBERPUNK ▓▓▓" COLOR_RESET "\n\n");

    int displayed_items = 0;
    for (int i = 0; i < shop->item_count; i++)
    {
        if (!shop->items[i].is_available)
            continue;

        displayed_items++;

        // Déterminer la couleur et le statut
        const char *status_color;
        const char *status_icon;
        const char *status_text;
        bool can_buy = (player_credits >= shop->items[i].price && player_level >= shop->items[i].level_required);

        if (can_buy)
        {
            status_color = COLOR_GREEN;
            status_icon = "✅";
            status_text = "DISPONIBLE";
        }
        else if (player_level < shop->items[i].level_required)
        {
            status_color = COLOR_RED;
            status_icon = "🔒";
            status_text = "NIVEAU REQUIS";
        }
        else
        {
            status_color = COLOR_YELLOW;
            status_icon = "💰";
            status_text = "CREDITS REQUIS";
        }

        // Affichage en carte cyberpunk moderne
        printf(COLOR_CYAN "╔═══════════════════════════════════════════════════════════════════════════╗\n");
        printf("║ " COLOR_BRIGHT_CYAN "[%02d] " COLOR_YELLOW "%-25s" COLOR_CYAN " ║ %s%s %-12s" COLOR_CYAN " ║\n",
               i + 1, shop->items[i].name, status_color, status_icon, status_text);
        printf("╠═══════════════════════════════════════════════════════════════════════════╣\n");
        printf("║ " COLOR_MAGENTA "💰 Prix: " COLOR_BRIGHT_CYAN "%3d ¢" COLOR_CYAN "                ║ " COLOR_MAGENTA "🎯 Niveau: " COLOR_BRIGHT_CYAN "%d" COLOR_CYAN "                  ║\n",
               shop->items[i].price, shop->items[i].level_required);
        printf("╠═══════════════════════════════════════════════════════════════════════════╣\n");

        // Description avec formatage amélioré
        int desc_len = strlen(shop->items[i].description);
        if (desc_len > 65)
        {
            // Diviser en deux lignes pour les descriptions longues
            char desc_line1[66];
            char desc_line2[66];
            strncpy(desc_line1, shop->items[i].description, 65);
            desc_line1[65] = '\0';
            strcpy(desc_line2, shop->items[i].description + 65);

            printf("║ " COLOR_WHITE "%-65s" COLOR_CYAN " ║\n", desc_line1);
            printf("║ " COLOR_WHITE "%-65s" COLOR_CYAN " ║\n", desc_line2);
        }
        else
        {
            printf("║ " COLOR_WHITE "%-65s" COLOR_CYAN " ║\n", shop->items[i].description);
        }

        printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");

        // Espacement entre les objets
        if (displayed_items % 2 == 0 && i < shop->item_count - 1)
        {
            printf("\n");
        }
    }

    // Footer avec légende améliorée
    printf("\n" COLOR_BRIGHT_CYAN "▓▓▓ INSTRUCTIONS ▓▓▓" COLOR_RESET "\n");
    printf(COLOR_GREEN "✅ Disponible" COLOR_RESET " │ ");
    printf(COLOR_YELLOW "💰 Crédits insuffisants" COLOR_RESET " │ ");
    printf(COLOR_RED "🔒 Niveau trop bas" COLOR_RESET "\n");
    printf(COLOR_MAGENTA "➤ Entrez le numéro de l'objet à acheter (0 pour quitter)" COLOR_RESET "\n\n");
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

void display_shop_welcome()
{
    printf("\n" COLOR_MAGENTA);
    printf("█▀▀ █▄█ █▄▄ █▀▀ █▀█   █▀▄▀█ ▄▀█ █▀█ █▄▀ █▀▀ ▀█▀\n");
    printf("█▄▄  █  █▄█ █▄▄ █▀▄   █ ▀ █ █▀█ █▀▄ █ █ █▄▄  █ \n");
    printf(COLOR_RESET "\n");
    printf(COLOR_CYAN "Bienvenue dans le marché noir du deep web...\n");
    printf("Ici, les crédits parlent et l'anonymat règne.\n" COLOR_RESET "\n");
}
