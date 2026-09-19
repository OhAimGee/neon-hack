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
#include "../i18n/i18n.h"
#include "legacy_colors.h"
#include "progression.h"
#include "world.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>


/*
 * Les attaques avancées visent les MÊMES systèmes que les classiques (gs->nodes) : seuls ceux qui
 * ont un profil de défense avancé (nh_world_adv_target) les acceptent. Résout l'argument en
 * (système, cible avancée) et explique à l'écran pourquoi pas.
 */
static bool resolve_advanced(const GameState *gs, const char *name, bool need_route, int *node_idx, int *target_id)
{
    int idx = nh_world_resolve(gs, name, need_route);
    if (idx < 0)
        return false;
    int tid = nh_world_adv_target(idx);
    if (tid < 0)
    {
        printf(nh_tr(NH_STR_WORLD_NO_PROFILE), gs->nodes[idx].name);
        printf("\n");
        return false;
    }
    *node_idx = idx;
    *target_id = tid;
    return true;
}

/* Liste les systèmes découverts qui ont un profil avancé. */
static void list_advanced_targets(const GameState *gs)
{
    bool any = false;
    for (int i = 0; i < nh_world_count(); i++)
    {
        int tid = nh_world_adv_target(i);
        if (tid < 0 || !gs->nodes[i].is_discovered)
            continue;
        if (!any)
            printf("%s\n", nh_tr(NH_STR_WORLD_ADV_TARGETS));
        any = true;
        const AdvancedTarget *t = &gs->advanced.targets[tid];
        printf("  %s%s%s (%s) - %d/100", COLOR_YELLOW, gs->nodes[i].name, COLOR_RESET, t->name, t->security_rating);
        if (gs->nodes[i].is_compromised)
            printf(" %s%s%s", COLOR_GREEN, nh_tr(NH_STR_WORLD_TAG_COMPROMISED), COLOR_RESET);
        else if (!nh_world_reachable(gs->nodes, i))
            printf(" %s%s%s", COLOR_RED, nh_tr(NH_STR_WORLD_TAG_ROUTE_CLOSED), COLOR_RESET);
        printf("\n");
    }
    if (!any)
        printf("%s\n", nh_tr(NH_STR_WORLD_ADV_NONE));
}

bool cmd_advanced_hack(GameState *gs, const char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("Usage: advhack <nom_cible>\n");
        list_advanced_targets(gs);
        return false;
    }

    int node_idx, target_id;
    if (!resolve_advanced(gs, target_name, true, &node_idx, &target_id))
        return false;

    NetworkNode *node = &gs->nodes[node_idx];
    if (node->is_compromised)
    {
        printf("%s\n", nh_tr(NH_STR_WORLD_ALREADY_COMPROMISED));
        return false;
    }

    nh_world_sync_tools(gs);
    display_hacking_menu(&gs->advanced);

    printf("\nChoisissez une méthode de hack (1-%d): ", gs->advanced.method_count);
    char input[10];
    nh_read_line(input, sizeof(input));
    int method_choice = atoi(input) - 1;

    if (method_choice < 0 || method_choice >= gs->advanced.method_count)
    {
        printf("Méthode invalide.\n");
        return false;
    }

    /* Le numéro choisi est celui du menu (ordre de la table des méthodes), pas celui de l'enum. */
    HackType hack_type = gs->advanced.methods[method_choice].type;
    int bonus = nh_world_node_bonus(node) - nh_alert_success_penalty(&gs->alert);
    bool ok = attempt_advanced_hack(&gs->advanced, target_id, hack_type, &gs->player, &gs->alert, bonus);
    if (ok)
        nh_world_compromise(gs, node_idx, true);
    return ok;
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
        list_advanced_targets(gs);
        return true;
    }

    // Une analyse est passive : elle n'exige pas que la route soit ouverte
    int node_idx, target_id;
    if (!resolve_advanced(gs, target_name, false, &node_idx, &target_id))
        return false;

    display_defense_analysis(&gs->advanced.targets[target_id]);
    return true;
}

bool cmd_social_engineer(GameState *gs, const char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("Usage: socialeng <nom_cible>\n");
        return false;
    }

    // Réussie, elle donne des accès internes : toutes les attaques sur ce système y gagnent
    // NH_INTEL_BONUS points, une fois pour toutes (et l'expérience n'est versée qu'une fois).
    int node_idx, target_id;
    if (!resolve_advanced(gs, target_name, true, &node_idx, &target_id))
        return false;

    NetworkNode *node = &gs->nodes[node_idx];
    if (node->has_intel)
    {
        printf(nh_tr(NH_STR_WORLD_INTEL_ALREADY), node->name);
        printf("\n");
        return false;
    }

    bool ok = social_engineering_attack(&gs->advanced, target_id, &gs->player, &gs->alert);
    if (ok)
    {
        node->has_intel = true;
        printf(nh_tr(NH_STR_WORLD_INTEL_GAINED), node->name, NH_INTEL_BONUS);
        printf("\n");
        nh_grant_xp(gs, 20);
    }
    return ok;
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

    // Le hack temporel contourne la route : le relais n'a pas besoin d'être compromis
    int node_idx, target_id;
    if (!resolve_advanced(gs, target_name, false, &node_idx, &target_id))
        return false;

    if (gs->nodes[node_idx].is_compromised)
    {
        printf("%s\n", nh_tr(NH_STR_WORLD_ALREADY_COMPROMISED));
        return false;
    }

    bool ok = temporal_hack_attempt(&gs->advanced, target_id, &gs->player, &gs->alert);
    if (ok)
        nh_world_compromise(gs, node_idx, true);
    return ok;
}
