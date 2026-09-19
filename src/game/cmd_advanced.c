/*
 * Commandes de hacking avancé (advhack, aiassist, neuralsync…).
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


bool cmd_advanced_hack(GameState *gs, const char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("Usage: advhack <nom_cible>\n");
        printf("Cibles disponibles: nexus-mainframe, banking-network, research-lab, gov-database, underground-market\n");
        return false;
    }

    // Trouver la cible dans le système avancé
    int target_id = -1;
    for (int i = 0; i < gs->advanced.target_count; i++)
    {
        if (strcmp(gs->advanced.targets[i].name, target_name) == 0)
        {
            target_id = i;
            break;
        }
    }

    if (target_id == -1)
    {
        printf("Cible '%s' introuvable. Utilisez 'analyzedefenses' pour voir les cibles disponibles.\n", target_name);
        return false;
    }

    display_hacking_menu(&gs->advanced);

    printf("\nChoisissez une méthode de hack (1-8): ");
    char input[10];
    nh_read_line(input, sizeof(input));
    int method_choice = atoi(input) - 1;

    if (method_choice < 0 || method_choice >= 8)
    {
        printf("Méthode invalide.\n");
        return false;
    }

    HackType hack_type = (HackType)method_choice;
    return attempt_advanced_hack(&gs->advanced, target_id, hack_type, &gs->player, &gs->alert);
}

bool cmd_stealth_mode_toggle(GameState *gs, const char *arg)
{
    (void)arg;

    update_stealth_system(&gs->advanced.stealth);

    if (gs->advanced.stealth.is_active)
    {
        printf("\n" COLOR_GREEN "🔮 MODE FURTIF ACTIVÉ" COLOR_RESET "\n");
        printf("Détection réduite de %d%%\n", gs->advanced.stealth.detection_reduction);
    }
    else
    {
        printf("\n" COLOR_YELLOW "👁️ MODE FURTIF DÉSACTIVÉ" COLOR_RESET "\n");
        printf("Visibilité normale rétablie.\n");
    }

    return true;
}

bool cmd_ai_assist_hack(GameState *gs, const char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("Usage: aiassist <nom_cible>\n");
        return false;
    }

    AIAssistant *ai = &gs->advanced.ai_assistant;

    printf("\n" COLOR_BRIGHT_CYAN "🤖 ASSISTANT IA ACTIVÉ" COLOR_RESET "\n");
    printf("Niveau d'IA: %d | Efficacité: %d%%\n", ai->intelligence_level, ai->efficiency);

    if (ai->intelligence_level < 3)
    {
        printf("⚠️ L'IA a besoin d'entraînement. Utilisez 'neuralsync' pour l'améliorer.\n");
    }

    // Simuler l'assistance IA
    int ai_bonus = ai->intelligence_level * 10;
    printf("Bonus de réussite IA: +%d%%\n", ai_bonus);

    // Appliquer le bonus au prochain hack
    printf("✅ Assistant IA configuré pour la prochaine tentative de hack.\n");

    // Entraîner l'IA avec cette utilisation
    train_ai_assistant(ai, 10);

    return true;
}

bool cmd_neural_sync(GameState *gs, const char *arg)
{
    (void)arg;

    printf("\n" COLOR_MAGENTA "🧠 SYNCHRONISATION INTERFACE NEURALE" COLOR_RESET "\n");
    printf("Établissement de la connexion synaptique...\n");

    for (int i = 0; i < 3; i++)
    {
        printf("⚡");
        fflush(stdout);
        nh_sleep_ms(800);
    }

    gs->advanced.neural_interface_sync += 10;
    if (gs->advanced.neural_interface_sync > 100)
        gs->advanced.neural_interface_sync = 100;

    printf("\n✅ Synchronisation complétée: %d%%\n", gs->advanced.neural_interface_sync);

    if (gs->advanced.neural_interface_sync >= 90)
    {
        printf("🎉 " COLOR_BRIGHT_GREEN "SYNCHRONISATION PARFAITE ATTEINTE!" COLOR_RESET "\n");
        printf("Nouvelles capacités débloquées: Hack temporel\n");
    }

    // Améliorer l'IA en même temps
    train_ai_assistant(&gs->advanced.ai_assistant, 20);

    return true;
}

bool cmd_analyze_defenses(GameState *gs, const char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("\n" COLOR_YELLOW "🎯 CIBLES DISPONIBLES:" COLOR_RESET "\n");
        display_advanced_targets(&gs->advanced);
        return true;
    }

    // Trouver et analyser la cible spécifique
    for (int i = 0; i < gs->advanced.target_count; i++)
    {
        if (strcmp(gs->advanced.targets[i].name, target_name) == 0)
        {
            display_defense_analysis(&gs->advanced.targets[i]);
            return true;
        }
    }

    printf("Cible '%s' introuvable.\n", target_name);
    return false;
}

bool cmd_social_engineer(GameState *gs, const char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("Usage: socialeng <nom_cible>\n");
        return false;
    }

    // Trouver la cible
    int target_id = -1;
    for (int i = 0; i < gs->advanced.target_count; i++)
    {
        if (strcmp(gs->advanced.targets[i].name, target_name) == 0)
        {
            target_id = i;
            break;
        }
    }

    if (target_id == -1)
    {
        printf("Cible introuvable pour l'ingénierie sociale.\n");
        return false;
    }

    return social_engineering_attack(&gs->advanced, target_id, &gs->player, &gs->alert);
}

bool cmd_temporal_hack(GameState *gs, const char *target_name)
{
    if (gs->advanced.neural_interface_sync < 90)
    {
        printf("⚠️ Synchronisation neurale insuffisante pour le hack temporel.\n");
        printf("Synchronisation actuelle: %d%% (minimum requis: 90%%)\n",
               gs->advanced.neural_interface_sync);
        return false;
    }

    if (strlen(target_name) == 0)
    {
        printf("Usage: temporalhack <nom_cible>\n");
        printf("⚠️ ATTENTION: Technique expérimentale avec risques élevés.\n");
        return false;
    }

    // Trouver la cible
    int target_id = -1;
    for (int i = 0; i < gs->advanced.target_count; i++)
    {
        if (strcmp(gs->advanced.targets[i].name, target_name) == 0)
        {
            target_id = i;
            break;
        }
    }

    if (target_id == -1)
    {
        printf("Cible introuvable pour le hack temporel.\n");
        return false;
    }

    return temporal_hack_attempt(&gs->advanced, target_id, &gs->player, &gs->alert);
}

/*
 * Mort depuis l'origine : la commande « quantumdecrypt » du niveau 5
 * (cmd_quantum_decrypt) la masquait dans le dispatch. Volontairement non
 * enregistrée dans la table de commandes ; à fusionner avec
 * cmd_quantum_decrypt lors de la Phase 3 (unification du hacking).
 */

bool cmd_quantum_decrypt_advanced(GameState *gs, const char *encrypted_data)
{
    if (strlen(encrypted_data) == 0)
    {
        printf("Usage: quantumdecrypt <données_cryptées>\n");
        printf("Exemple: quantumdecrypt 'Q#X7#NEXUS#SECRET#DATA'\n");
        return false;
    }

    return activate_quantum_hack(&gs->advanced.quantum, encrypted_data);
}
