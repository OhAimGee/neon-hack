/*
 * Commandes du monde : boutique, alerte, quêtes, contacts, messages.
 *
 * Code d'origine (neon_hack.c, v2.087) déplacé tel quel et adapté à GameState :
 * les variables globales sont devenues des champs de `gs`. La logique sera
 * réécrite en Phase 3 ; d'ici là, seuls les accès à l'état ont changé.
 */
#include "game.h"

#include "../core/io.h"
#include "../core/platform.h"
#include "legacy_colors.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>


bool cmd_shop(GameState *gs, const char *arg)
{
    (void)arg;

    // Vérifier si la boutique est accessible
    if (!is_shop_available(gs->player.level, gs->alert.current_level))
    {
        return false;
    }

    display_shop_welcome();
    display_shop(&gs->shop, gs->player.credits, gs->player.level);

    char input[10];
    printf("\nEntrez le numéro de l'objet à acheter (0 pour quitter): ");
    nh_read_line(input, sizeof(input));
    input[strcspn(input, "\n")] = 0;

    int choice = atoi(input);
    if (choice == 0)
    {
        printf("À bientôt dans l'ombre...\n");
        return true;
    }

    if (choice >= 1 && choice <= ITEM_COUNT)
    {
        ShopItemType item_type = (ShopItemType)(choice - 1);
        if (buy_item(&gs->shop, item_type, &gs->player.credits, gs->player.level))
        {
            // Utiliser l'objet acheté immédiatement si applicable
            use_item(item_type, &gs->player);
        }
    }
    else
    {
        printf("Numéro d'objet invalide.\n");
    }

    return true;
}

bool cmd_lay_low(GameState *gs, const char *arg)
{
    (void)arg;

    display_alert_status(&gs->alert);
    display_alert_reduction_menu(&gs->alert);

    char input[10];
    printf("\nChoisissez une méthode (0 pour annuler): ");
    nh_read_line(input, sizeof(input));
    input[strcspn(input, "\n")] = 0;

    int choice = atoi(input);
    if (choice == 0)
    {
        printf("Vous restez dans l'ombre pour le moment...\n");
        return true;
    }

    AlertReductionMethod method;
    switch (choice)
    {
    case 1:
        method = REDUCTION_TIME;
        break;
    case 2:
        method = REDUCTION_VPN;
        break;
    case 3:
        method = REDUCTION_PROXY;
        break;
    case 4:
        method = REDUCTION_GHOST;
        break;
    case 5:
        method = REDUCTION_LAYLOW;
        break;
    case 6:
        method = REDUCTION_FRAME;
        break;
    default:
        printf("Choix invalide.\n");
        return false;
    }

    if (attempt_alert_reduction(&gs->alert, method, &gs->player.credits))
    {
        // Synchroniser le niveau d'alerte du joueur avec le système
        gs->player.alert_level = gs->alert.current_level * 10;
        if (gs->player.alert_level > 100)
            gs->player.alert_level = 100;

        printf("\nNiveau d'alerte synchronisé: %d/100\n", gs->player.alert_level);
    }

    return true;
}

bool cmd_quests(GameState *gs, const char *arg)
{
    (void)arg;

    display_quest_log(&gs->quests);

    printf("\nVoulez-vous voir les détails d'une quête ? (tapez le numéro ou 0 pour sortir): ");
    char input[10];
    nh_read_line(input, sizeof(input));
    input[strcspn(input, "\n")] = 0;

    int quest_id = atoi(input);
    if (quest_id > 0 && quest_id <= gs->quests.active_quest_count)
    {
        display_quest_details(&gs->quests.quests[quest_id - 1]);
    }

    return true;
}

bool cmd_contacts(GameState *gs, const char *arg)
{
    (void)arg;

    display_contacts(&gs->contacts);

    printf("\nVoulez-vous parler à un contact ? (tapez le numéro ou 0 pour sortir): ");
    char input[10];
    nh_read_line(input, sizeof(input));
    input[strcspn(input, "\n")] = 0;

    int contact_id = atoi(input);
    if (contact_id > 0 && contact_id <= gs->contacts.active_contacts)
    {
        ContactType id = (ContactType)(contact_id - 1);
        contact_npc(&gs->contacts, id, &gs->player);
    }

    return true;
}

bool cmd_messages(GameState *gs, const char *arg)
{
    (void)arg;

    display_inbox(&gs->contacts);
    return true;
}

bool cmd_read(GameState *gs, const char *argument)
{
    if (strlen(argument) == 0)
    {
        printf("Usage: read <numéro_message>\n");
        printf("Exemple: read 1\n");
        return false;
    }

    int message_id = atoi(argument);
    if (message_id <= 0 || message_id > gs->contacts.inbox_count)
    {
        printf("Numéro de message invalide.\n");
        return false;
    }

    // Marquer le message comme lu et l'afficher
    Message *msg = &gs->contacts.inbox[message_id - 1];
    msg->is_read = true;

    printf("\n" COLOR_CYAN "═══════════════════════════════════════════════════════════════════════════\n");
    printf("De: " COLOR_WHITE "%s" COLOR_RESET "\n", msg->from);
    printf("Sujet: " COLOR_YELLOW "%s" COLOR_RESET "\n", msg->subject);
    printf("═══════════════════════════════════════════════════════════════════════════\n" COLOR_RESET);
    printf("\n%s\n", msg->content);
    printf("\n" COLOR_CYAN "═══════════════════════════════════════════════════════════════════════════\n" COLOR_RESET);

    return true;
}

bool cmd_interact_contact(GameState *gs, const char *argument)
{
    if (strlen(argument) == 0)
    {
        printf("Usage: contact <numéro> ou contact <nom>\n");
        printf("Exemple: contact 1\n");
        printf("Exemple: contact ECHO-7\n");
        printf("Tapez 'contacts' pour voir la liste des contacts disponibles.\n");
        return false;
    }

    // Vérifier si c'est un numéro
    if (isdigit(argument[0]))
    {
        int contact_number = atoi(argument);
        if (contact_number <= 0)
        {
            printf("Numéro de contact invalide. Tapez 'contacts' pour voir la liste.\n");
            return false;
        }

        // Trouver le contact par numéro (basé sur les contacts débloqués)
        int current_number = 1;
        for (int i = 0; i < CONTACT_COUNT; i++)
        {
            if (gs->contacts.contacts[i].is_unlocked)
            {
                if (current_number == contact_number)
                {
                    ContactType id = (ContactType)i;
                    contact_npc(&gs->contacts, id, &gs->player);
                    return true;
                }
                current_number++;
            }
        }

        printf("Contact #%d introuvable. Tapez 'contacts' pour voir la liste.\n", contact_number);
        return false;
    }
    else
    {
        // Rechercher par nom
        for (int i = 0; i < CONTACT_COUNT; i++)
        {
            if (gs->contacts.contacts[i].is_unlocked)
            {
                if (strcmp(gs->contacts.contacts[i].name, argument) == 0)
                {
                    ContactType id = (ContactType)i;
                    contact_npc(&gs->contacts, id, &gs->player);
                    return true;
                }
            }
        }

        printf("Contact '%s' introuvable. Tapez 'contacts' pour voir la liste.\n", argument);
        return false;
    }
}
