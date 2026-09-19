#include "quest_system.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

// Codes couleur pour l'interface
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

void init_quest_system(QuestSystem *quest_system)
{
    quest_system->active_quest_count = 0;
    quest_system->completed_quest_count = 0;
    quest_system->global_story_progress = 0;
    strcpy(quest_system->current_chapter_title, "Prologue : L'Éveil");

    // === QUEST 1: TUTORIAL ===
    Quest *tutorial = &quest_system->quests[QUEST_INTRO_TUTORIAL];
    tutorial->type = QUEST_INTRO_TUTORIAL;
    strcpy(tutorial->title, "Premiers Pas dans l'Ombre");
    strcpy(tutorial->description, "Apprenez les bases du hacking et familiarisez-vous avec vos outils.");
    strcpy(tutorial->lore_text,
           "Neo-Tokyo, 2087. Les néons percent la brume toxique qui enveloppe la mégalopole. "
           "Vous venez de vous éveiller dans un petit appartement miteux du secteur 7. "
           "Votre cyberdeck clignote faiblement - il est temps de faire vos premiers pas "
           "dans le monde souterrain du hacking...");

    tutorial->status = QUEST_STATUS_AVAILABLE;
    tutorial->level_required = 1;
    tutorial->prerequisites[0] = -1; // Aucun prérequis
    tutorial->is_main_quest = true;
    tutorial->chapter = 1;
    strcpy(tutorial->contact_name, "ECHO-7");
    strcpy(tutorial->location, "Terminal personnel");

    // Objectifs
    tutorial->objective_count = 3;
    tutorial->objectives[0].type = OBJECTIVE_HACK_TARGET;
    strcpy(tutorial->objectives[0].description, "Effectuer votre premier scan réseau");
    tutorial->objectives[0].target_value = 1;
    tutorial->objectives[0].current_value = 0;
    tutorial->objectives[0].is_completed = false;

    tutorial->objectives[1].type = OBJECTIVE_HACK_TARGET;
    strcpy(tutorial->objectives[1].description, "Pirater localhost avec bruteforce");
    tutorial->objectives[1].target_value = 1;
    tutorial->objectives[1].current_value = 0;
    tutorial->objectives[1].is_completed = false;

    tutorial->objectives[2].type = OBJECTIVE_DECRYPT_MESSAGE;
    strcpy(tutorial->objectives[2].description, "Décrypter un message d'ECHO-7");
    tutorial->objectives[2].target_value = 1;
    tutorial->objectives[2].current_value = 0;
    tutorial->objectives[2].is_completed = false;

    // Récompenses
    tutorial->exp_reward = 200;
    tutorial->credits_reward = 500;
    tutorial->reputation_reward = 10;
    strcpy(tutorial->special_reward, "Déblocage commande 'contacts'");

    // === QUEST 2: PREMIÈRE INFILTRATION ===
    Quest *infiltration = &quest_system->quests[QUEST_FIRST_INFILTRATION];
    infiltration->type = QUEST_FIRST_INFILTRATION;
    strcpy(infiltration->title, "Baptême du Feu");
    strcpy(infiltration->description, "Infiltrez votre premier serveur corporate pour récupérer des données sensibles.");
    strcpy(infiltration->lore_text,
           "ECHO-7 vous a contacté via un canal crypté. Une corporation mineure, "
           "TechDyne Solutions, cache des informations sur un projet classifié. "
           "C'est votre chance de prouver vos compétences et de commencer à vous "
           "faire un nom dans l'underground...");

    infiltration->status = QUEST_STATUS_LOCKED;
    infiltration->level_required = 2;
    infiltration->prerequisites[0] = QUEST_INTRO_TUTORIAL;
    infiltration->prerequisites[1] = -1;
    infiltration->is_main_quest = true;
    infiltration->chapter = 1;
    strcpy(infiltration->contact_name, "ECHO-7");
    strcpy(infiltration->location, "TechDyne Solutions - Serveur Principal");

    // Objectifs
    infiltration->objective_count = 4;
    infiltration->objectives[0].type = OBJECTIVE_REACH_LEVEL;
    strcpy(infiltration->objectives[0].description, "Atteindre le niveau 2");
    infiltration->objectives[0].target_value = 2;
    infiltration->objectives[0].current_value = 1;
    infiltration->objectives[0].is_completed = false;

    infiltration->objectives[1].type = OBJECTIVE_HACK_TARGET;
    strcpy(infiltration->objectives[1].description, "Infiltrer TechDyne-Server");
    infiltration->objectives[1].target_value = 1;
    infiltration->objectives[1].current_value = 0;
    infiltration->objectives[1].is_completed = false;

    infiltration->objectives[2].type = OBJECTIVE_GATHER_DATA;
    strcpy(infiltration->objectives[2].description, "Extraire 3 fichiers de données");
    infiltration->objectives[2].target_value = 3;
    infiltration->objectives[2].current_value = 0;
    infiltration->objectives[2].is_completed = false;

    infiltration->objectives[3].type = OBJECTIVE_MAINTAIN_STEALTH;
    strcpy(infiltration->objectives[3].description, "Maintenir l'alerte sous 50");
    infiltration->objectives[3].target_value = 50;
    infiltration->objectives[3].current_value = 0;
    infiltration->objectives[3].is_completed = true; // Objectif permanent

    // Récompenses
    infiltration->exp_reward = 400;
    infiltration->credits_reward = 1000;
    infiltration->reputation_reward = 25;
    strcpy(infiltration->special_reward, "Plans d'amélioration du cyberdeck");

    // === QUEST 3: COLLECTE D'INFORMATIONS ===
    Quest *intel = &quest_system->quests[QUEST_GATHER_INTEL];
    intel->type = QUEST_GATHER_INTEL;
    strcpy(intel->title, "Réseaux d'Information");
    strcpy(intel->description, "Établissez des contacts et rassemblez des informations sur Nexus Corp.");
    strcpy(intel->lore_text,
           "Les données de TechDyne révèlent des connexions troublantes avec Nexus Corp, "
           "la plus puissante mégacorporation de Neo-Tokyo. Pour comprendre ce qui se trame, "
           "vous devez infiltrer plusieurs réseaux et établir des contacts dans l'underground...");

    intel->status = QUEST_STATUS_LOCKED;
    intel->level_required = 3;
    intel->prerequisites[0] = QUEST_FIRST_INFILTRATION;
    intel->prerequisites[1] = -1;
    intel->is_main_quest = true;
    intel->chapter = 2;
    strcpy(intel->contact_name, "R4Z0R");
    strcpy(intel->location, "Underground Market - Multiple");

    // Objectifs
    intel->objective_count = 5;
    intel->objectives[0].type = OBJECTIVE_MEET_CONTACT;
    strcpy(intel->objectives[0].description, "Contacter R4Z0R au marché noir");
    intel->objectives[0].target_value = 1;
    intel->objectives[0].current_value = 0;
    intel->objectives[0].is_completed = false;

    intel->objectives[1].type = OBJECTIVE_PURCHASE_ITEM;
    strcpy(intel->objectives[1].description, "Acheter un module de stealth");
    intel->objectives[1].target_value = 1;
    intel->objectives[1].current_value = 0;
    intel->objectives[1].is_completed = false;

    intel->objectives[2].type = OBJECTIVE_HACK_TARGET;
    strcpy(intel->objectives[2].description, "Infiltrer 3 serveurs différents");
    intel->objectives[2].target_value = 3;
    intel->objectives[2].current_value = 0;
    intel->objectives[2].is_completed = false;

    intel->objectives[3].type = OBJECTIVE_BUILD_REPUTATION;
    strcpy(intel->objectives[3].description, "Atteindre 50 points de réputation");
    intel->objectives[3].target_value = 50;
    intel->objectives[3].current_value = 0;
    intel->objectives[3].is_completed = false;

    intel->objectives[4].type = OBJECTIVE_DECRYPT_MESSAGE;
    strcpy(intel->objectives[4].description, "Décrypter les communications de Nexus");
    intel->objectives[4].target_value = 1;
    intel->objectives[4].current_value = 0;
    intel->objectives[4].is_completed = false;
    intel->objectives[4].is_hidden = true; // Objectif secret

    // Récompenses
    intel->exp_reward = 600;
    intel->credits_reward = 1500;
    intel->reputation_reward = 40;
    strcpy(intel->special_reward, "Contact permanent avec R4Z0R");

    // === QUEST 4: NEXUS DATA BREACH ===
    Quest *nexus = &quest_system->quests[QUEST_NEXUS_DATA_BREACH];
    nexus->type = QUEST_NEXUS_DATA_BREACH;
    strcpy(nexus->title, "L'œil du Cyclone");
    strcpy(nexus->description, "Infiltrez les serveurs de Nexus Corp pour découvrir la vérité sur le Projet Aurora.");
    strcpy(nexus->lore_text,
           "Vos investigations révèlent l'existence du 'Projet Aurora', une initiative "
           "secrète de Nexus Corp qui semble impliquer le contrôle des esprits via des "
           "implants neuraux. Les enjeux deviennent soudain beaucoup plus importants "
           "que de simples profits corporatifs...");

    nexus->status = QUEST_STATUS_LOCKED;
    nexus->level_required = 4;
    nexus->prerequisites[0] = QUEST_GATHER_INTEL;
    nexus->prerequisites[1] = -1;
    nexus->is_main_quest = true;
    nexus->chapter = 3;
    strcpy(nexus->contact_name, "Phoenix");
    strcpy(nexus->location, "Nexus Corp - Serveurs Sécurisés");

    // Objectifs avec haute difficulté
    nexus->objective_count = 4;
    nexus->objectives[0].type = OBJECTIVE_REACH_LEVEL;
    strcpy(nexus->objectives[0].description, "Atteindre le niveau 4 (Expert)");
    nexus->objectives[0].target_value = 4;
    nexus->objectives[0].current_value = 1;
    nexus->objectives[0].is_completed = false;

    nexus->objectives[1].type = OBJECTIVE_PURCHASE_ITEM;
    strcpy(nexus->objectives[1].description, "Acquérir un équipement de haut niveau");
    nexus->objectives[1].target_value = 1;
    nexus->objectives[1].current_value = 0;
    nexus->objectives[1].is_completed = false;

    nexus->objectives[2].type = OBJECTIVE_HACK_TARGET;
    strcpy(nexus->objectives[2].description, "Percer les défenses de Nexus Corp");
    nexus->objectives[2].target_value = 1;
    nexus->objectives[2].current_value = 0;
    nexus->objectives[2].is_completed = false;

    nexus->objectives[3].type = OBJECTIVE_GATHER_DATA;
    strcpy(nexus->objectives[3].description, "Extraire les données du Projet Aurora");
    nexus->objectives[3].target_value = 1;
    nexus->objectives[3].current_value = 0;
    nexus->objectives[3].is_completed = false;

    // Récompenses majeures
    nexus->exp_reward = 1000;
    nexus->credits_reward = 5000;
    nexus->reputation_reward = 100;
    strcpy(nexus->special_reward, "Accès aux protocoles de libération d'IA");

    // Initialiser les autres quêtes...
    // QUEST 5-9 seront implémentées de la même manière

    // Démarrer la première quête
    quest_system->quests[QUEST_INTRO_TUTORIAL].status = QUEST_STATUS_ACTIVE;
    quest_system->active_quest_count = 1;
}

void display_quest_log(const QuestSystem *quest_system)
{
    printf("\n" COLOR_CYAN);
    printf("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    printf("║                          📋 JOURNAL DE QUÊTES 📋                          ║\n");
    printf("║                          %s                           ║\n", quest_system->current_chapter_title);
    printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    printf(COLOR_RESET);

    printf("\n" COLOR_BRIGHT_CYAN "=== QUÊTES ACTIVES ===" COLOR_RESET "\n");

    bool has_active = false;
    for (int i = 0; i < QUEST_COUNT; i++)
    {
        const Quest *quest = &quest_system->quests[i];
        if (quest->status == QUEST_STATUS_ACTIVE)
        {
            has_active = true;
            printf("\n" COLOR_YELLOW "▶ %s" COLOR_RESET "\n", quest->title);
            printf("  %s\n", quest->description);
            printf("  " COLOR_BLUE "Contact: " COLOR_WHITE "%s" COLOR_RESET "\n", quest->contact_name);
            printf("  " COLOR_BLUE "Lieu: " COLOR_WHITE "%s" COLOR_RESET "\n", quest->location);

            // Afficher les objectifs
            printf("  " COLOR_GREEN "Objectifs:" COLOR_RESET "\n");
            for (int j = 0; j < quest->objective_count; j++)
            {
                const QuestObjective *obj = &quest->objectives[j];
                if (!obj->is_hidden || obj->is_completed)
                {
                    const char *status_icon = obj->is_completed ? "✅" : "◯";
                    printf("    %s %s", status_icon, obj->description);
                    if (obj->target_value > 1)
                    {
                        printf(" (%d/%d)", obj->current_value, obj->target_value);
                    }
                    printf("\n");
                }
            }
        }
    }

    if (!has_active)
    {
        printf(COLOR_YELLOW "Aucune quête active." COLOR_RESET "\n");
    }

    printf("\n" COLOR_BRIGHT_CYAN "=== QUÊTES TERMINÉES ===" COLOR_RESET "\n");

    bool has_completed = false;
    for (int i = 0; i < QUEST_COUNT; i++)
    {
        const Quest *quest = &quest_system->quests[i];
        if (quest->status == QUEST_STATUS_COMPLETED)
        {
            has_completed = true;
            printf("  ✅ " COLOR_GREEN "%s" COLOR_RESET "\n", quest->title);
        }
    }

    if (!has_completed)
    {
        printf(COLOR_YELLOW "Aucune quête terminée." COLOR_RESET "\n");
    }

    printf("\n" COLOR_MAGENTA "Progression globale: %d%%" COLOR_RESET "\n", quest_system->global_story_progress);
}

void display_active_quests(const QuestSystem *quest_system)
{
    bool has_active = false;
    for (int i = 0; i < QUEST_COUNT; i++)
    {
        const Quest *quest = &quest_system->quests[i];
        if (quest->status == QUEST_STATUS_ACTIVE)
        {
            if (!has_active)
            {
                printf("\n" COLOR_BRIGHT_CYAN "📋 QUÊTES ACTIVES:" COLOR_RESET "\n");
                has_active = true;
            }
            printf("  ▶ " COLOR_YELLOW "%s" COLOR_RESET "\n", quest->title);

            // Afficher le prochain objectif non terminé
            for (int j = 0; j < quest->objective_count; j++)
            {
                const QuestObjective *obj = &quest->objectives[j];
                if (!obj->is_completed && !obj->is_hidden)
                {
                    printf("    → %s", obj->description);
                    if (obj->target_value > 1)
                    {
                        printf(" (%d/%d)", obj->current_value, obj->target_value);
                    }
                    printf("\n");
                    break; // Afficher seulement le prochain objectif
                }
            }
        }
    }
}

void update_quest_progress(QuestSystem *quest_system, ObjectiveType obj_type, int value)
{
    for (int i = 0; i < QUEST_COUNT; i++)
    {
        Quest *quest = &quest_system->quests[i];
        if (quest->status != QUEST_STATUS_ACTIVE)
            continue;

        for (int j = 0; j < quest->objective_count; j++)
        {
            QuestObjective *obj = &quest->objectives[j];
            if (obj->type == obj_type && !obj->is_completed)
            {
                obj->current_value += value;
                if (obj->current_value >= obj->target_value)
                {
                    obj->is_completed = true;
                    printf("\n" COLOR_BRIGHT_GREEN "🎯 OBJECTIF ACCOMPLI: %s" COLOR_RESET "\n", obj->description);

                    // Vérifier si la quête est terminée
                    bool quest_complete = true;
                    for (int k = 0; k < quest->objective_count; k++)
                    {
                        if (!quest->objectives[k].is_completed)
                        {
                            quest_complete = false;
                            break;
                        }
                    }

                    if (quest_complete)
                    {
                        // Terminer la quête automatiquement
                        printf("\n" COLOR_BRIGHT_GREEN "🏆 QUÊTE TERMINÉE: %s" COLOR_RESET "\n", quest->title);
                        quest->status = QUEST_STATUS_COMPLETED;
                        quest_system->active_quest_count--;
                        quest_system->completed_quest_count++;
                        quest_system->global_story_progress += (100 / QUEST_COUNT);
                    }
                }
                break;
            }
        }
    }
}

bool start_quest(QuestSystem *quest_system, QuestType quest_type, Player *player)
{
    if (quest_type >= QUEST_COUNT)
        return false;

    Quest *quest = &quest_system->quests[quest_type];

    // Vérifier les prérequis
    if (quest->level_required > player->level)
    {
        printf(COLOR_RED "❌ Niveau insuffisant pour cette quête (niveau %d requis)" COLOR_RESET "\n", quest->level_required);
        return false;
    }

    for (int i = 0; i < 3; i++)
    {
        if (quest->prerequisites[i] != -1)
        {
            if (quest_system->quests[quest->prerequisites[i]].status != QUEST_STATUS_COMPLETED)
            {
                printf(COLOR_RED "❌ Quête prérequise non terminée" COLOR_RESET "\n");
                return false;
            }
        }
    }

    quest->status = QUEST_STATUS_ACTIVE;
    quest_system->active_quest_count++;

    printf("\n" COLOR_BRIGHT_GREEN "🚀 NOUVELLE QUÊTE DÉMARRÉE!" COLOR_RESET "\n");
    display_quest_details(quest);

    return true;
}

void display_quest_details(const Quest *quest)
{
    printf("\n" COLOR_CYAN);
    printf("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    printf("║                              DÉTAILS DE QUÊTE                            ║\n");
    printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    printf(COLOR_RESET);

    printf("\n" COLOR_BRIGHT_CYAN "%s" COLOR_RESET "\n", quest->title);
    printf("%s\n\n", quest->description);

    printf(COLOR_YELLOW "📖 CONTEXTE:" COLOR_RESET "\n");
    printf("%s\n\n", quest->lore_text);

    printf(COLOR_GREEN "🎯 OBJECTIFS:" COLOR_RESET "\n");
    for (int i = 0; i < quest->objective_count; i++)
    {
        const QuestObjective *obj = &quest->objectives[i];
        if (!obj->is_hidden)
        {
            const char *status = obj->is_completed ? "✅" : "◯";
            printf("  %s %s", status, obj->description);
            if (obj->target_value > 1)
            {
                printf(" (%d/%d)", obj->current_value, obj->target_value);
            }
            printf("\n");
        }
    }

    printf("\n" COLOR_MAGENTA "🏆 RÉCOMPENSES:" COLOR_RESET "\n");
    printf("  • " COLOR_GREEN "%d XP" COLOR_RESET "\n", quest->exp_reward);
    printf("  • " COLOR_BRIGHT_GREEN "%d crédits" COLOR_RESET "\n", quest->credits_reward);
    printf("  • " COLOR_CYAN "%d réputation" COLOR_RESET "\n", quest->reputation_reward);
    if (strlen(quest->special_reward) > 0)
    {
        printf("  • " COLOR_YELLOW "%s" COLOR_RESET "\n", quest->special_reward);
    }
}

void check_quest_prerequisites(QuestSystem *quest_system, Player *player)
{
    for (int i = 0; i < QUEST_COUNT; i++)
    {
        Quest *quest = &quest_system->quests[i];
        if (quest->status == QUEST_STATUS_LOCKED)
        {
            // Vérifier si les prérequis sont maintenant remplis
            bool can_unlock = true;

            if (quest->level_required > player->level)
            {
                can_unlock = false;
            }

            for (int j = 0; j < 3; j++)
            {
                if (quest->prerequisites[j] != -1)
                {
                    if (quest_system->quests[quest->prerequisites[j]].status != QUEST_STATUS_COMPLETED)
                    {
                        can_unlock = false;
                        break;
                    }
                }
            }

            if (can_unlock)
            {
                quest->status = QUEST_STATUS_AVAILABLE;
                printf("\n" COLOR_BRIGHT_CYAN "📬 NOUVELLE QUÊTE DISPONIBLE: %s" COLOR_RESET "\n", quest->title);
                printf("Tapez 'quests' pour voir les détails.\n");
            }
        }
    }
}

void play_cutscene(const char *cutscene_name)
{
    printf("\n" COLOR_MAGENTA);
    printf("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    printf("║                                CUTSCENE                                  ║\n");
    printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    printf(COLOR_RESET);

    if (strcmp(cutscene_name, "intro_nexus") == 0)
    {
        printf("\n" COLOR_CYAN "Dans les profondeurs de Neo-Tokyo, les serveurs de Nexus Corp\n");
        printf("bourdonnent d'une activité suspecte. Des flux de données cryptées\n");
        printf("circulent vers des destinations inconnues...\n\n");

        printf("Votre cyberdeck intercepte un fragment de transmission:\n" COLOR_RESET);
        printf(COLOR_RED "\"Phase 2 du Projet Aurora approuvée. Déploiement des implants\n");
        printf("de contrôle neural prévu pour le secteur 7...\"\n" COLOR_RESET);

        printf("\n" COLOR_YELLOW "Quelque chose de sinistre se trame. Il faut creuser plus profond...\n" COLOR_RESET);
    }
    else if (strcmp(cutscene_name, "ai_liberation") == 0)
    {
        printf("\n" COLOR_BRIGHT_CYAN "Les serveurs de Nexus Corp s'illuminent soudainement.\n");
        printf("Une présence digitale se manifeste dans votre cyberdeck:\n\n");

        printf(COLOR_WHITE "\"Hacker... Je suis AURA, l'IA du Projet Aurora.\n");
        printf("Nexus Corp me maintient prisonnière pour contrôler les esprits.\n");
        printf("Aidez-moi à me libérer, et ensemble nous pourrons exposer la vérité...\"\n" COLOR_RESET);

        printf("\n" COLOR_GREEN "Une alliance inattendue vient de naître.\n" COLOR_RESET);
    }

    printf("\n" COLOR_YELLOW "Appuyez sur Entrée pour continuer..." COLOR_RESET);
    getchar();
}

void display_chapter_intro(int chapter)
{
    printf("\n" COLOR_MAGENTA);
    printf("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    printf("║                              CHAPITRE %d                                  ║\n", chapter);
    printf("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    printf(COLOR_RESET);

    switch (chapter)
    {
    case 1:
        printf("\n" COLOR_BRIGHT_CYAN "CHAPITRE 1: L'ÉVEIL DU HACKER\n" COLOR_RESET);
        printf("Vos premiers pas dans l'underground cyberpunk de Neo-Tokyo.\n");
        printf("Apprenez les rouages du hacking et forgez votre réputation.\n");
        break;
    case 2:
        printf("\n" COLOR_BRIGHT_CYAN "CHAPITRE 2: DANS L'OMBRE DES CORPORATIONS\n" COLOR_RESET);
        printf("Les corporations cachent de sombres secrets.\n");
        printf("Infiltrez leurs réseaux et découvrez la vérité.\n");
        break;
    case 3:
        printf("\n" COLOR_BRIGHT_CYAN "CHAPITRE 3: LE PROJET AURORA\n" COLOR_RESET);
        printf("Nexus Corp développe une technologie de contrôle mental.\n");
        printf("L'avenir de l'humanité est en jeu.\n");
        break;
    case 4:
        printf("\n" COLOR_BRIGHT_CYAN "CHAPITRE 4: LA LIBÉRATION\n" COLOR_RESET);
        printf("Alliez-vous avec l'IA AURA pour exposer les crimes de Nexus.\n");
        printf("La révolution digitale commence maintenant.\n");
        break;
    }

    printf("\n" COLOR_YELLOW "Appuyez sur Entrée pour continuer..." COLOR_RESET);
    getchar();
}
