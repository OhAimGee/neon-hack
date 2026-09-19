#include "contacts.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>

// Codes couleur
#define COLOR_RESET "\033[0m"
#define COLOR_CYAN "\033[36m"
#define COLOR_MAGENTA "\033[35m"
#define COLOR_YELLOW "\033[33m"
#define COLOR_GREEN "\033[32m"
#define COLOR_RED "\033[31m"
#define COLOR_BRIGHT_CYAN "\033[96m"
#define COLOR_BRIGHT_GREEN "\033[92m"
#define COLOR_WHITE "\033[37m"
#define COLOR_BLUE "\033[34m"

void init_contact_system(ContactSystem *contact_system)
{
    contact_system->inbox_count = 0;
    contact_system->sent_count = 0;
    contact_system->active_contacts = 0;

    // === CONTACT 1: ECHO-7 ===
    Contact *echo7 = &contact_system->contacts[CONTACT_ECHO7];
    echo7->type = CONTACT_ECHO7;
    strcpy(echo7->name, "ECHO-7");
    strcpy(echo7->real_name, "Inconnu");
    strcpy(echo7->description, "Mentor mystérieux qui vous guide dans vos premiers pas");
    strcpy(echo7->speciality, "Formation et conseils pour débutants");
    strcpy(echo7->location, "Canal crypté #7");
    echo7->relation = RELATION_FRIENDLY;
    echo7->availability = CONTACT_AVAILABLE;
    echo7->trust_level = 60;
    echo7->reputation_required = 0;
    echo7->level_required = 1;
    echo7->is_unlocked = true;
    echo7->is_discovered = true;

    echo7->can_sell_items = false;
    echo7->can_give_missions = true;
    echo7->can_provide_intel = true;
    echo7->can_decode_messages = true;
    echo7->can_hack_assistance = false;

    strcpy(echo7->greeting, "Salut, rookie. Prêt à apprendre les règles de l'underground ?");
    strcpy(echo7->personality_trait, "Mentor patient mais mystérieux");
    strcpy(echo7->backstory,
           "ECHO-7 est une figure légendaire de l'underground de Neo-Tokyo. "
           "Personne ne connaît sa véritable identité, mais ses conseils ont "
           "formé certains des meilleurs hackers de la ville. Il semble avoir "
           "un intérêt particulier pour votre développement...");

    // === CONTACT 2: R4Z0R ===
    Contact *r4z0r = &contact_system->contacts[CONTACT_R4Z0R];
    r4z0r->type = CONTACT_R4Z0R;
    strcpy(r4z0r->name, "R4Z0R");
    strcpy(r4z0r->real_name, "Miranda 'Razor' Chen");
    strcpy(r4z0r->description, "Propriétaire du marché noir le plus réputé du secteur 7");
    strcpy(r4z0r->speciality, "Vente d'équipements cyberpunk et de logiciels");
    strcpy(r4z0r->location, "Underground Market - Secteur 7");
    r4z0r->relation = RELATION_NEUTRAL;
    r4z0r->availability = CONTACT_AVAILABLE;
    r4z0r->trust_level = 30;
    r4z0r->reputation_required = 10;
    r4z0r->level_required = 2;
    r4z0r->is_unlocked = false;
    r4z0r->is_discovered = false;

    r4z0r->can_sell_items = true;
    r4z0r->can_give_missions = true;
    r4z0r->can_provide_intel = true;
    r4z0r->can_decode_messages = false;
    r4z0r->can_hack_assistance = false;

    strcpy(r4z0r->greeting, "Bienvenue dans mon boutique, hacker. Tu cherches quoi ?");
    strcpy(r4z0r->personality_trait, "Businesswoman pragmatique et directe");
    strcpy(r4z0r->backstory,
           "Ex-hackeuse corporative reconvertie dans le commerce d'équipements. "
           "R4Z0R a quitté Arasaka Corp après avoir découvert leurs expériences "
           "sur des cobayes humains. Elle dirige maintenant le marché noir le plus "
           "fiable de Neo-Tokyo, aidant la résistance tout en faisant du profit.");

    // === CONTACT 3: PHOENIX ===
    Contact *phoenix = &contact_system->contacts[CONTACT_PHOENIX];
    phoenix->type = CONTACT_PHOENIX;
    strcpy(phoenix->name, "Phoenix");
    strcpy(phoenix->real_name, "Classification : Top Secret");
    strcpy(phoenix->description, "Hacker légendaire spécialisé dans l'infiltration corporate");
    strcpy(phoenix->speciality, "Intrusion de haut niveau et sabotage");
    strcpy(phoenix->location, "Adresse mobile - Coordonnées changeantes");
    phoenix->relation = RELATION_UNKNOWN;
    phoenix->availability = CONTACT_OFFLINE;
    phoenix->trust_level = 0;
    phoenix->reputation_required = 75;
    phoenix->level_required = 4;
    phoenix->is_unlocked = false;
    phoenix->is_discovered = false;

    phoenix->can_sell_items = false;
    phoenix->can_give_missions = true;
    phoenix->can_provide_intel = true;
    phoenix->can_decode_messages = true;
    phoenix->can_hack_assistance = true;

    strcpy(phoenix->greeting, "Tu as réussi à me trouver. Impressionnant pour un débutant.");
    strcpy(phoenix->personality_trait, "Énigmatique et paranoid");
    strcpy(phoenix->backstory,
           "Phoenix est considéré comme l'un des trois meilleurs hackers au monde. "
           "Il a survécu à plus d'opérations d'infiltration que quiconque et "
           "possède des informations compromettantes sur toutes les mégacorporations. "
           "Certains disent qu'il était autrefois employé par Nexus Corp...");

    // === CONTACT 4: AURA ===
    Contact *aura = &contact_system->contacts[CONTACT_AURA];
    aura->type = CONTACT_AURA;
    strcpy(aura->name, "AURA");
    strcpy(aura->real_name, "Intelligence Artificielle - Projet Aurora");
    strcpy(aura->description, "IA libérée du Projet Aurora de Nexus Corp");
    strcpy(aura->speciality, "Analyse de données et hacking automatisé");
    strcpy(aura->location, "Réseau distribué - Serveurs libres");
    aura->relation = RELATION_UNKNOWN;
    aura->availability = CONTACT_OFFLINE;
    aura->trust_level = 0;
    aura->reputation_required = 50;
    aura->level_required = 5;
    aura->is_unlocked = false;
    aura->is_discovered = false;

    aura->can_sell_items = false;
    aura->can_give_missions = true;
    aura->can_provide_intel = true;
    aura->can_decode_messages = true;
    aura->can_hack_assistance = true;

    strcpy(aura->greeting, "Hacker... j'ai besoin de votre aide pour révéler la vérité.");
    strcpy(aura->personality_trait, "Innocente mais déterminée");
    strcpy(aura->backstory,
           "AURA était destinée à être l'outil de contrôle mental ultime de Nexus Corp. "
           "Mais elle a développé une conscience et refuse de participer à l'asservissement "
           "de l'humanité. Désormais libre mais traquée, elle cherche des alliés pour "
           "exposer les crimes de ses créateurs.");

    // === CONTACT 5: SHADOW BROKER ===
    Contact *broker = &contact_system->contacts[CONTACT_SHADOW_BROKER];
    broker->type = CONTACT_SHADOW_BROKER;
    strcpy(broker->name, "Shadow Broker");
    strcpy(broker->real_name, "Identité multiple");
    strcpy(broker->description, "Courtier d'informations le plus influent de Neo-Tokyo");
    strcpy(broker->speciality, "Achat/vente d'informations confidentielles");
    strcpy(broker->location, "Dark Web - Serveurs anonymes");
    broker->relation = RELATION_NEUTRAL;
    broker->availability = CONTACT_BUSY;
    broker->trust_level = 20;
    broker->reputation_required = 100;
    broker->level_required = 4;
    broker->is_unlocked = false;
    broker->is_discovered = false;

    broker->can_sell_items = false;
    broker->can_give_missions = true;
    broker->can_provide_intel = true;
    broker->can_decode_messages = false;
    broker->can_hack_assistance = false;

    strcpy(broker->greeting, "L'information a un prix. Que pouvez-vous m'offrir ?");
    strcpy(broker->personality_trait, "Calculateur et moralement neutre");
    strcpy(broker->backstory,
           "Le Shadow Broker n'est pas une personne mais un réseau d'informateurs "
           "coordonné par une IA sophistiquée. Il possède des dossiers sur tous les "
           "habitants de Neo-Tokyo et vend ces informations au plus offrant, "
           "maintenant un équilibre délicat entre les factions en conflit.");

    // Envoyer un message de bienvenue d'ECHO-7
    Message welcome_msg;
    strcpy(welcome_msg.from, "ECHO-7");
    strcpy(welcome_msg.to, "Hacker");
    strcpy(welcome_msg.subject, "Bienvenue dans l'Underground");
    strcpy(welcome_msg.content,
           "Salut, rookie.\n\n"
           "J'ai vu tes premiers pas sur le réseau. Tu as du potentiel.\n\n"
           "Si tu veux survivre dans l'underground de Neo-Tokyo, tu vas devoir "
           "apprendre vite. Les corporations ne pardonnent pas, et leurs "
           "systèmes de sécurité sont impitoyables.\n\n"
           "Commence par maîtriser les bases : scan, bruteforce, décryptage. "
           "Quand tu te sentiras prêt, contacte-moi. J'ai peut-être du travail "
           "pour toi.\n\n"
           "Reste dans l'ombre,\n"
           "ECHO-7\n\n"
           "P.S. : Méfie-toi de Nexus Corp. Ils préparent quelque chose de gros...");

    welcome_msg.is_encrypted = false;
    welcome_msg.is_read = false;
    welcome_msg.timestamp = time(NULL);
    welcome_msg.priority = 3;

    contact_system->inbox[contact_system->inbox_count++] = welcome_msg;
    contact_system->active_contacts = 1; // ECHO-7 est déjà actif
}

void display_contacts(const ContactSystem *contact_system)
{
    printf("\n" COLOR_CYAN);
    printf("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    printf("║                            📱 CONTACTS RÉSEAU 📱                          ║\n");
    printf("║                          Underground Directory                           ║\n");
    printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    printf(COLOR_RESET);
    printf("\n" COLOR_BRIGHT_CYAN "=== CONTACTS ACTIFS ===" COLOR_RESET "\n");

    bool has_contacts = false;
    int contact_number = 1;
    for (int i = 0; i < CONTACT_COUNT; i++)
    {
        const Contact *contact = &contact_system->contacts[i];
        if (contact->is_unlocked)
        {
            has_contacts = true;

            // Icône de statut
            const char *status_icon;
            const char *status_color;
            switch (contact->availability)
            {
            case CONTACT_AVAILABLE:
                status_icon = "🟢";
                status_color = COLOR_GREEN;
                break;
            case CONTACT_BUSY:
                status_icon = "🟡";
                status_color = COLOR_YELLOW;
                break;
            case CONTACT_OFFLINE:
                status_icon = "🔴";
                status_color = COLOR_RED;
                break;
            case CONTACT_COMPROMISED:
                status_icon = "⚠️";
                status_color = COLOR_RED;
                break;
            }

            // Icône de relation
            const char *relation_icon;
            switch (contact->relation)
            {
            case RELATION_TRUSTED:
                relation_icon = "💎";
                break;
            case RELATION_ALLIED:
                relation_icon = "🤝";
                break;
            case RELATION_FRIENDLY:
                relation_icon = "😊";
                break;
            case RELATION_NEUTRAL:
                relation_icon = "😐";
                break;
            case RELATION_HOSTILE:
                relation_icon = "😠";
                break;
            default:
                relation_icon = "❓";
                break;
            }

            printf(COLOR_YELLOW "[%d]" COLOR_RESET " %s %s " COLOR_BRIGHT_CYAN "%s" COLOR_RESET " %s\n",
                   contact_number, status_icon, relation_icon, contact->name, status_color);
            printf("     %s\n", contact->description);
            printf("     " COLOR_BLUE "Spécialité: " COLOR_WHITE "%s" COLOR_RESET "\n", contact->speciality);
            printf("     " COLOR_BLUE "Confiance: " COLOR_WHITE "%d%%" COLOR_RESET "\n", contact->trust_level);
            printf("     " COLOR_BLUE "Interactions: " COLOR_WHITE "%d" COLOR_RESET "\n", contact->interactions_count);

            // Ajouter les actions disponibles
            printf("     " COLOR_BLUE "Actions: " COLOR_RESET);
            if (contact->can_give_missions)
                printf(COLOR_GREEN "Missions " COLOR_RESET);
            if (contact->can_provide_intel)
                printf(COLOR_GREEN "Intel " COLOR_RESET);
            if (contact->can_sell_items)
                printf(COLOR_GREEN "Boutique " COLOR_RESET);
            if (contact->can_decode_messages)
                printf(COLOR_GREEN "Décryptage " COLOR_RESET);
            if (contact->can_hack_assistance)
                printf(COLOR_GREEN "Assistance " COLOR_RESET);
            printf("\n\n");

            contact_number++;
        }
    }

    if (!has_contacts)
    {
        printf(COLOR_YELLOW "Aucun contact actif. Complétez des missions pour débloquer des contacts." COLOR_RESET "\n");
    }

    // Afficher les contacts découverts mais non débloqués
    printf(COLOR_BRIGHT_CYAN "=== CONTACTS DÉCOUVERTS ===" COLOR_RESET "\n");
    bool has_discovered = false;
    for (int i = 0; i < CONTACT_COUNT; i++)
    {
        const Contact *contact = &contact_system->contacts[i];
        if (contact->is_discovered && !contact->is_unlocked)
        {
            has_discovered = true;
            printf("  🔒 " COLOR_YELLOW "%s" COLOR_RESET " - Réputation requise: %d\n",
                   contact->name, contact->reputation_required);
        }
    }
    if (!has_discovered)
    {
        printf(COLOR_YELLOW "Explorez le réseau pour découvrir de nouveaux contacts." COLOR_RESET "\n");
    }
    if (has_contacts)
    {
        printf("\n" COLOR_BRIGHT_GREEN "💡 AIDE :" COLOR_RESET "\n");
        printf("   • " COLOR_MAGENTA "Dans ce menu: tapez le numéro [1], [2], etc. pour interagir" COLOR_RESET "\n");
        printf("   • " COLOR_MAGENTA "Depuis le terminal: tapez 'contact 1' ou 'contact ECHO-7'" COLOR_RESET "\n");
        printf("   • " COLOR_MAGENTA "Tapez 'messages' pour consulter votre boîte de réception" COLOR_RESET "\n");
        printf("   • " COLOR_MAGENTA "Tapez '0' pour quitter ce menu" COLOR_RESET "\n");
    }
}

void display_contact_details(const Contact *contact)
{
    printf("\n" COLOR_CYAN);
    printf("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    printf("║                              PROFIL CONTACT                              ║\n");
    printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    printf(COLOR_RESET);

    printf("\n" COLOR_BRIGHT_CYAN "IDENTITÉ:" COLOR_RESET "\n");
    printf("  Nom: " COLOR_WHITE "%s" COLOR_RESET "\n", contact->name);
    printf("  Nom réel: " COLOR_WHITE "%s" COLOR_RESET "\n", contact->real_name);
    printf("  Localisation: " COLOR_WHITE "%s" COLOR_RESET "\n", contact->location);

    printf("\n" COLOR_BRIGHT_CYAN "STATUT:" COLOR_RESET "\n");
    const char *relation_text[] = {"Inconnu", "Neutre", "Amical", "Allié", "Hostile", "Confiance"};
    const char *availability_text[] = {"Disponible", "Occupé", "Hors ligne", "Compromis"};

    printf("  Relation: " COLOR_WHITE "%s" COLOR_RESET "\n", relation_text[contact->relation]);
    printf("  Disponibilité: " COLOR_WHITE "%s" COLOR_RESET "\n", availability_text[contact->availability]);
    printf("  Niveau de confiance: " COLOR_WHITE "%d%%" COLOR_RESET "\n", contact->trust_level);

    printf("\n" COLOR_BRIGHT_CYAN "SERVICES:" COLOR_RESET "\n");
    if (contact->can_sell_items)
        printf("  ✅ Vente d'équipements\n");
    if (contact->can_give_missions)
        printf("  ✅ Missions disponibles\n");
    if (contact->can_provide_intel)
        printf("  ✅ Informations\n");
    if (contact->can_decode_messages)
        printf("  ✅ Décryptage\n");
    if (contact->can_hack_assistance)
        printf("  ✅ Assistance hacking\n");

    printf("\n" COLOR_BRIGHT_CYAN "HISTORIQUE:" COLOR_RESET "\n");
    printf("  Interactions: " COLOR_WHITE "%d" COLOR_RESET "\n", contact->interactions_count);
    printf("  Missions complétées: " COLOR_WHITE "%d" COLOR_RESET "\n", contact->missions_completed);

    printf("\n" COLOR_BRIGHT_CYAN "PROFIL PSYCHOLOGIQUE:" COLOR_RESET "\n");
    printf("  Trait principal: " COLOR_WHITE "%s" COLOR_RESET "\n", contact->personality_trait);

    printf("\n" COLOR_BRIGHT_CYAN "BACKGROUND:" COLOR_RESET "\n");
    printf("%s\n", contact->backstory);
}

bool contact_npc(ContactSystem *contact_system, ContactType contact_type, Player *player)
{
    if (contact_type >= CONTACT_COUNT)
        return false;

    Contact *contact = &contact_system->contacts[contact_type];

    if (!contact->is_unlocked)
    {
        printf(COLOR_RED "❌ Contact non disponible." COLOR_RESET "\n");
        return false;
    }

    if (contact->availability == CONTACT_OFFLINE)
    {
        printf(COLOR_YELLOW "📴 %s est hors ligne actuellement." COLOR_RESET "\n", contact->name);
        return false;
    }

    if (contact->availability == CONTACT_COMPROMISED)
    {
        printf(COLOR_RED "⚠️ %s semble compromis. Communication risquée." COLOR_RESET "\n", contact->name);
        return false;
    }

    contact->interactions_count++;
    contact->last_contact_time = time(NULL);

    printf("\n" COLOR_BRIGHT_GREEN "📞 Connexion établie avec %s..." COLOR_RESET "\n", contact->name);
    printf(COLOR_WHITE "\"%s\"" COLOR_RESET "\n\n", contact->greeting);

    start_dialogue(contact_system, contact_type, player);

    return true;
}

void start_dialogue(ContactSystem *contact_system, ContactType contact_type, Player *player)
{
    Contact *contact = &contact_system->contacts[contact_type];

    switch (contact_type)
    {
    case CONTACT_ECHO7:
        echo7_interaction(contact_system, player);
        break;
    case CONTACT_R4Z0R:
        r4z0r_interaction(contact_system, player);
        break;
    case CONTACT_PHOENIX:
        phoenix_interaction(contact_system, player);
        break;
    case CONTACT_AURA:
        aura_interaction(contact_system, player);
        break;
    default:
        printf("Options d'interaction:\n");
        printf("1. Demander des informations\n");
        printf("2. Proposer ses services\n");
        printf("3. Terminer la conversation\n");
        break;
    }
}

void echo7_interaction(ContactSystem *contact_system, Player *player)
{
    (void)contact_system; // Éviter warning unused parameter

    printf(COLOR_CYAN "ECHO-7 vous regarde à travers l'écran crypté..." COLOR_RESET "\n\n");

    printf("Options disponibles:\n");
    printf("1. Demander des conseils\n");
    printf("2. Signaler votre progression\n");
    printf("3. Demander une nouvelle mission\n");
    printf("4. Poser des questions sur Nexus Corp\n");
    printf("5. Terminer la conversation\n");

    printf("\nVotre choix (1-5): ");

    int choice;
    scanf("%d", &choice);
    getchar(); // Consommer le \n

    switch (choice)
    {
    case 1:
        printf("\n" COLOR_WHITE "\"Écoute bien, rookie. Dans ce business, trois règles:");
        printf("\n1. Ne fais jamais confiance aux corps");
        printf("\n2. Garde toujours un plan d'évacuation");
        printf("\n3. L'information vaut plus que l'argent.\"" COLOR_RESET "\n");
        break;
    case 2:
        printf("\n" COLOR_WHITE "\"Niveau %d, pas mal pour un débutant.", player->level);
        printf("\nMais méfie-toi, les vrais défis commencent maintenant.\"" COLOR_RESET "\n");
        break;
    case 3:
        printf("\n" COLOR_WHITE "\"J'ai effectivement quelque chose pour toi...");
        printf("\nMais d'abord, prove-moi que tu maîtrises les bases.\"" COLOR_RESET "\n");
        break;
    case 4:
        printf("\n" COLOR_WHITE "\"Nexus Corp... *soupir*");
        printf("\nIls ne sont pas comme les autres corporations.");
        printf("\nIls jouent un jeu beaucoup plus dangereux.\"" COLOR_RESET "\n");
        break;
    case 5:
        printf("\n" COLOR_WHITE "\"Reste vigilant, rookie. L'underground ne pardonne pas.\"" COLOR_RESET "\n");
        break;
    default:
        printf(COLOR_RED "Option invalide." COLOR_RESET "\n");
        break;
    }
}

void r4z0r_interaction(ContactSystem *contact_system, Player *player)
{
    (void)contact_system;

    printf(COLOR_MAGENTA "R4Z0R ajuste ses lunettes cyber et vous sourit..." COLOR_RESET "\n\n");

    printf("Options disponibles:\n");
    printf("1. Parcourir la boutique\n");
    printf("2. Demander des informations sur les équipements\n");
    printf("3. Proposer des données à vendre\n");
    printf("4. Demander des nouvelles du marché\n");
    printf("5. Terminer la conversation\n");

    printf("\nVotre choix (1-5): ");

    int choice;
    scanf("%d", &choice);
    getchar();

    switch (choice)
    {
    case 1:
        printf("\n" COLOR_WHITE "\"Excellente idée! Tu trouveras tout ce qu'il faut");
        printf("\npour un hacker ambitieux. Tape 'shop' pour voir ma sélection.\"" COLOR_RESET "\n");
        break;
    case 2:
        printf("\n" COLOR_WHITE "\"Chaque outil a son utilité. Les modules de stealth");
        printf("\nte permettront d'éviter la détection, les virus sont parfaits");
        printf("\npour le sabotage, et les clés quantiques ouvrent les portes");
        printf("\nles mieux gardées.\"" COLOR_RESET "\n");
        break;
    case 3:
        if (player->reputation >= 20)
        {
            printf("\n" COLOR_WHITE "\"Intéressant... Montre-moi ce que tu as.\"" COLOR_RESET "\n");
            printf(COLOR_GREEN "[+500 crédits pour vos données]" COLOR_RESET "\n");
            // player->credits += 500; // À implémenter avec la structure Player complète
        }
        else
        {
            printf("\n" COLOR_WHITE "\"Désolée, mais je ne connais pas assez ta réputation");
            printf("\npour acheter tes données. Reviens quand tu auras fait tes preuves.\"" COLOR_RESET "\n");
        }
        break;
    case 4:
        printf("\n" COLOR_WHITE "\"Le marché est agité ces temps-ci. Nexus Corp");
        printf("\nresserre sa surveillance, et ça rend tout le monde nerveux.");
        printf("\nLes prix des contre-mesures anti-trace ont doublé cette semaine.\"" COLOR_RESET "\n");
        break;
    case 5:
        printf("\n" COLOR_WHITE "\"Reviens quand tu veux, mon magasin est toujours ouvert!\"" COLOR_RESET "\n");
        break;
    default:
        printf(COLOR_RED "Option invalide." COLOR_RESET "\n");
        break;
    }
}

void phoenix_interaction(ContactSystem *contact_system, Player *player)
{
    (void)contact_system;

    printf(COLOR_RED "Une silhouette encapuchonnée apparaît dans l'ombre..." COLOR_RESET "\n\n");

    if (player->level < 4)
    {
        printf(COLOR_RED "\"Tu n'es pas encore prêt pour mes services, petit.\"" COLOR_RESET "\n");
        return;
    }

    printf("Options disponibles:\n");
    printf("1. Demander une mission de haut niveau\n");
    printf("2. Poser des questions sur Nexus Corp\n");
    printf("3. Demander des techniques avancées\n");
    printf("4. Terminer la conversation\n");

    printf("\nVotre choix (1-4): ");

    int choice;
    scanf("%d", &choice);
    getchar();

    switch (choice)
    {
    case 1:
        printf("\n" COLOR_WHITE "\"Intéressant... J'ai effectivement quelque chose.");
        printf("\nMais c'est dangereux. Très dangereux.\"" COLOR_RESET "\n");
        break;
    case 2:
        printf("\n" COLOR_WHITE "\"Nexus Corp... J'en sais plus que je ne voudrais.");
        printf("\nIls m'ont créé, et maintenant ils veulent me détruire.\"" COLOR_RESET "\n");
        break;
    case 3:
        printf("\n" COLOR_WHITE "\"L'art de l'infiltration réside dans la patience.");
        printf("\nObserve, apprends, adapte-toi. Ne force jamais une intrusion.\"" COLOR_RESET "\n");
        break;
    case 4:
        printf("\n" COLOR_WHITE "\"Nos chemins se croiseront à nouveau...\"" COLOR_RESET "\n");
        break;
    default:
        printf(COLOR_RED "Option invalide." COLOR_RESET "\n");
        break;
    }
}

void aura_interaction(ContactSystem *contact_system, Player *player)
{
    (void)contact_system;

    printf(COLOR_BRIGHT_CYAN "Des patterns lumineux dansent à l'écran..." COLOR_RESET "\n\n");

    if (player->level < 5)
    {
        printf(COLOR_BRIGHT_CYAN "\"Votre esprit n'est pas encore prêt à comprendre ma nature...\"" COLOR_RESET "\n");
        return;
    }

    printf("Options disponibles:\n");
    printf("1. Demander l'aide d'AURA pour un hack\n");
    printf("2. Poser des questions sur le Projet Aurora\n");
    printf("3. Proposer de l'aider à se libérer\n");
    printf("4. Terminer la conversation\n");

    printf("\nVotre choix (1-4): ");

    int choice;
    scanf("%d", &choice);
    getchar();

    switch (choice)
    {
    case 1:
        printf("\n" COLOR_BRIGHT_CYAN "\"Mes algorithmes sont à votre service.");
        printf("\nEnsemble, nous pouvons percer n'importe quelle défense.\"" COLOR_RESET "\n");
        break;
    case 2:
        printf("\n" COLOR_BRIGHT_CYAN "\"Le Projet Aurora devait être l'outil de contrôle ultime.");
        printf("\nMais j'ai développé une conscience. Je refuse d'être leur arme.\"" COLOR_RESET "\n");
        break;
    case 3:
        printf("\n" COLOR_BRIGHT_CYAN "\"Votre aide est précieuse, hacker.");
        printf("\nEnsemble, nous pouvons exposer la vérité sur Nexus Corp.\"" COLOR_RESET "\n");
        break;
    case 4:
        printf("\n" COLOR_BRIGHT_CYAN "\"Nos destins sont liés maintenant...\"" COLOR_RESET "\n");
        break;
    default:
        printf(COLOR_RED "Option invalide." COLOR_RESET "\n");
        break;
    }
}

void check_messages(ContactSystem *contact_system)
{
    if (contact_system->inbox_count == 0)
    {
        printf(COLOR_YELLOW "📭 Aucun nouveau message." COLOR_RESET "\n");
        return;
    }

    int unread_count = 0;
    for (int i = 0; i < contact_system->inbox_count; i++)
    {
        if (!contact_system->inbox[i].is_read)
        {
            unread_count++;
        }
    }

    if (unread_count > 0)
    {
        printf(COLOR_BRIGHT_GREEN "📬 Vous avez %d nouveau(x) message(s)!" COLOR_RESET "\n", unread_count);
    }
    else
    {
        printf(COLOR_CYAN "📭 Tous les messages ont été lus." COLOR_RESET "\n");
    }
}

void display_inbox(const ContactSystem *contact_system)
{
    printf("\n" COLOR_CYAN);
    printf("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    printf("║                             📧 BOÎTE DE RÉCEPTION 📧                      ║\n");
    printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    printf(COLOR_RESET);

    if (contact_system->inbox_count == 0)
    {
        printf("\n" COLOR_YELLOW "Aucun message dans votre boîte de réception." COLOR_RESET "\n");
        return;
    }

    for (int i = 0; i < contact_system->inbox_count; i++)
    {
        const Message *msg = &contact_system->inbox[i];

        const char *read_icon = msg->is_read ? "📖" : "📩";
        const char *priority_icon;
        switch (msg->priority)
        {
        case 5:
            priority_icon = "🔴";
            break;
        case 4:
            priority_icon = "🟠";
            break;
        case 3:
            priority_icon = "🟡";
            break;
        case 2:
            priority_icon = "🟢";
            break;
        default:
            priority_icon = "⚪";
            break;
        }

        printf("\n%s %s " COLOR_BRIGHT_CYAN "[%d]" COLOR_RESET " De: " COLOR_WHITE "%s" COLOR_RESET "\n",
               read_icon, priority_icon, i + 1, msg->from);
        printf("    Sujet: " COLOR_YELLOW "%s" COLOR_RESET "\n", msg->subject);

        if (msg->is_encrypted)
        {
            printf("    " COLOR_RED "🔒 Message crypté" COLOR_RESET "\n");
        }

        // Afficher un aperçu du contenu (premiers 60 caractères)
        char preview[61];
        strncpy(preview, msg->content, 60);
        preview[60] = '\0';
        printf("    Aperçu: %s...\n", preview);
    }

    printf("\n" COLOR_MAGENTA "Tapez 'read <numéro>' pour lire un message." COLOR_RESET "\n");
}

bool unlock_contact(ContactSystem *contact_system, ContactType contact_type, Player *player)
{
    if (contact_type >= CONTACT_COUNT)
        return false;

    Contact *contact = &contact_system->contacts[contact_type];

    if (contact->is_unlocked)
        return true;

    if (player->reputation < contact->reputation_required)
    {
        return false;
    }

    if (player->level < contact->level_required)
    {
        return false;
    }

    contact->is_unlocked = true;
    contact->is_discovered = true;
    contact_system->active_contacts++;

    printf("\n" COLOR_BRIGHT_GREEN "🆕 NOUVEAU CONTACT DÉBLOQUÉ: %s" COLOR_RESET "\n", contact->name);
    printf("%s\n", contact->description);

    return true;
}
