/*
 * Commandes du monde : boutique, alerte, contacts, messages (le journal de quêtes, `quests`, est
 * dans quest_system.c).
 *
 * Code d'origine (neon_hack.c, v2.087) déplacé tel quel et adapté à GameState :
 * les variables globales sont devenues des champs de `gs`. La logique sera
 * réécrite en Phase 3 ; d'ici là, seuls les accès à l'état ont changé.
 */
#include "game.h"

#include "../core/io.h"
#include "../core/platform.h"
#include "../i18n/i18n.h"
#include "legacy_colors.h"
#include "progression.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>


bool cmd_shop(GameState *gs, const char *arg)
{
    (void)arg;

    // Vérifier si la boutique est accessible
    if (nh_alert_shop_closed(&gs->alert))
    {
        printf(nh_tr(NH_STR_SHOP_CLOSED), gs->alert.level);
        printf("\n");
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
            nh_event(gs, NH_EV_ITEM_BOUGHT, (int)item_type);
            // Utiliser l'objet acheté immédiatement si applicable. Seul le boost de réputation a un
            // effet pour l'instant (une quête en dépend) ; les autres arrivent avec la phase 3.4.
            if (item_type == ITEM_REPUTATION_BOOST)
            {
                nh_grant_reputation(gs, 20);
                printf("\n");
            }
            else
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

    nh_alert_print_status(&gs->alert);
    nh_alert_print_menu(&gs->alert);

    char input[16];
    printf("\n%s", nh_tr(NH_STR_ALERT_PROMPT));
    if (nh_read_line(input, sizeof(input)) != NH_IO_OK)
        return false;

    int choice = 0;
    if (!nh_parse_int(input, 0, NH_REDUCTION_COUNT, &choice))
    {
        printf("%s\n", nh_tr(NH_STR_ALERT_INVALID_CHOICE));
        return false;
    }

    if (choice == 0)
    {
        printf("%s\n", nh_tr(NH_STR_ALERT_STAY_HIDDEN));
        return true;
    }
    static const NhStr k_done[NH_REDUCTION_COUNT] = {
        [NH_REDUCTION_TIME] = NH_STR_ALERT_DONE_TIME,     [NH_REDUCTION_VPN] = NH_STR_ALERT_DONE_VPN,
        [NH_REDUCTION_PROXY] = NH_STR_ALERT_DONE_PROXY,   [NH_REDUCTION_GHOST] = NH_STR_ALERT_DONE_GHOST,
        [NH_REDUCTION_LAYLOW] = NH_STR_ALERT_DONE_LAYLOW, [NH_REDUCTION_FRAME] = NH_STR_ALERT_DONE_FRAME,
    };

    NhReduction method = (NhReduction)(choice - 1);
    int before = gs->alert.level;
    switch (nh_alert_apply_reduction(&gs->alert, method, &gs->player.credits, NULL))
    {
    case NH_REDUCE_NO_CREDITS:
        printf(nh_tr(NH_STR_ALERT_NO_CREDITS), nh_alert_reduction_cost(method));
        printf("\n");
        return false;
    case NH_REDUCE_NO_GHOST:
        printf("%s\n", nh_tr(NH_STR_ALERT_NO_GHOST));
        return false;
    case NH_REDUCE_OK:
        break;
    }

    printf("%s\n", nh_tr(k_done[method]));
    if (method == NH_REDUCTION_FRAME)
    {
        nh_grant_reputation(gs, -5);
        printf("%s\n", nh_tr(NH_STR_ALERT_KARMA));
    }
    printf(nh_tr(NH_STR_ALERT_NOW), before, gs->alert.level);
    printf("\n");
    return true;
}

/* Parle à un contact ; une conversation qui a eu lieu est un événement (les quêtes le comptent). */
static bool talk_to(GameState *gs, ContactType id)
{
    bool ok = contact_npc(&gs->contacts, id, &gs->player);
    if (ok)
        nh_event(gs, NH_EV_CONTACT_MET, (int)id);
    return ok;
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
        talk_to(gs, id);
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
                    talk_to(gs, id);
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
                    talk_to(gs, id);
                    return true;
                }
            }
        }

        printf("Contact '%s' introuvable. Tapez 'contacts' pour voir la liste.\n", argument);
        return false;
    }
}
