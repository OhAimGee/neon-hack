#include "advanced_hacking.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "../core/platform.h"
#include "../i18n/i18n.h"

// Codes couleur ANSI
#define COLOR_RESET "\033[0m"
#define COLOR_RED "\033[31m"
#define COLOR_GREEN "\033[32m"
#define COLOR_YELLOW "\033[33m"
#define COLOR_BLUE "\033[34m"
#define COLOR_MAGENTA "\033[35m"
#define COLOR_CYAN "\033[36m"
#define COLOR_BRIGHT_GREEN "\033[92m"
#define COLOR_BRIGHT_CYAN "\033[96m"

void init_advanced_hacking_system(AdvancedHackingSystem *system)
{
    // Initialiser les méthodes de hacking
    system->method_count = 8;

    // Méthode 1: Quantum Decrypt
    system->methods[0] = (HackingMethod){
        .type = HACK_TYPE_QUANTUM,
        .success_rate = 85,
        .detection_risk = 15,
        .time_required = 30,
        .energy_cost = 40,
        .requires_tool = true,
        .required_tool = TOOL_QUANTUM_COMPUTER,
        .description = "Utilise l'informatique quantique pour casser instantanément les encryptions"};

    // Méthode 2: AI-Assisted Hack
    system->methods[1] = (HackingMethod){
        .type = HACK_TYPE_AI_ASSISTED,
        .success_rate = 75,
        .detection_risk = 25,
        .time_required = 45,
        .energy_cost = 30,
        .requires_tool = true,
        .required_tool = TOOL_AI_ASSISTANT,
        .description = "L'IA analyse les patterns de sécurité et optimise l'attaque"};

    // Méthode 3: Neural Interface Hack
    system->methods[2] = (HackingMethod){
        .type = HACK_TYPE_NEURAL,
        .success_rate = 90,
        .detection_risk = 35,
        .time_required = 20,
        .energy_cost = 50,
        .requires_tool = true,
        .required_tool = TOOL_NEURAL_INTERFACE,
        .description = "Interface directe cerveau-machine pour un contrôle parfait"};

    // Méthode 4: Stealth Hack
    system->methods[3] = (HackingMethod){
        .type = HACK_TYPE_STEALTH,
        .success_rate = 65,
        .detection_risk = 5,
        .time_required = 90,
        .energy_cost = 25,
        .requires_tool = true,
        .required_tool = TOOL_STEALTH_CLOAK,
        .description = "Infiltration invisible, presque indétectable"};

    // Méthode 5: Virus Deployment
    system->methods[4] = (HackingMethod){
        .type = HACK_TYPE_VIRUS,
        .success_rate = 70,
        .detection_risk = 45,
        .time_required = 60,
        .energy_cost = 35,
        .requires_tool = true,
        .required_tool = TOOL_VIRUS_LABORATORY,
        .description = "Déploie des virus auto-réplicants et adaptatifs"};

    // Méthode 6: Social Engineering
    system->methods[5] = (HackingMethod){
        .type = HACK_TYPE_SOCIAL_ENGINEER,
        .success_rate = 80,
        .detection_risk = 20,
        .time_required = 120,
        .energy_cost = 20,
        .requires_tool = true,
        .required_tool = TOOL_SOCIAL_PROFILE_DB,
        .description = "Manipulation psychologique des employés cibles"};

    // Méthode 7: Zero-Day Exploit
    system->methods[6] = (HackingMethod){
        .type = HACK_TYPE_BACKDOOR,
        .success_rate = 95,
        .detection_risk = 60,
        .time_required = 15,
        .energy_cost = 60,
        .requires_tool = true,
        .required_tool = TOOL_ZERO_DAY_EXPLOIT,
        .description = "Exploit inconnu des systèmes de défense"};

    // Méthode 8: Ghost Protocol
    system->methods[7] = (HackingMethod){
        .type = HACK_TYPE_BASIC,
        .success_rate = 50,
        .detection_risk = 0,
        .time_required = 180,
        .energy_cost = 10,
        .requires_tool = true,
        .required_tool = TOOL_GHOST_PROTOCOL,
        .description = "Hack fantôme qui ne laisse aucune trace"};

    // Initialiser les cibles avancées
    system->target_count = 5;

    // Cible 1: Nexus Corp MainFrame
    system->targets[0] = (AdvancedTarget){
        .name = "Nexus Corp MainFrame",
        .security_rating = 95,
        .defense_count = 3,
        .has_ai_guardian = true,
        .quantum_encrypted = true,
        .corporate_level = 5};
    system->targets[0].defenses[0] = (DefenseSystem){
        .type = DEFENSE_TYPE_AI_GUARDIAN,
        .strength = 90,
        .adaptive_level = 85,
        .is_learning = true,
        .attack_count = 0,
        .name = "AURA Defense Grid"};
    system->targets[0].defenses[1] = (DefenseSystem){
        .type = DEFENSE_TYPE_QUANTUM_ENCRYPTION,
        .strength = 95,
        .adaptive_level = 70,
        .is_learning = false,
        .attack_count = 0,
        .name = "Quantum Firewall"};
    system->targets[0].defenses[2] = (DefenseSystem){
        .type = DEFENSE_TYPE_BEHAVIORAL_ANALYSIS,
        .strength = 80,
        .adaptive_level = 90,
        .is_learning = true,
        .attack_count = 0,
        .name = "BehaviorScan Pro"};

    // Cible 2: Corporate Banking Network
    system->targets[1] = (AdvancedTarget){
        .name = "Corporate Banking Network",
        .security_rating = 85,
        .defense_count = 2,
        .has_ai_guardian = false,
        .quantum_encrypted = true,
        .corporate_level = 4};
    system->targets[1].defenses[0] = (DefenseSystem){
        .type = DEFENSE_TYPE_FIREWALL,
        .strength = 85,
        .adaptive_level = 60,
        .is_learning = false,
        .attack_count = 0,
        .name = "BankGuard Firewall"};
    system->targets[1].defenses[1] = (DefenseSystem){
        .type = DEFENSE_TYPE_INTRUSION_DETECTION,
        .strength = 80,
        .adaptive_level = 75,
        .is_learning = true,
        .attack_count = 0,
        .name = "SIEM Advanced"};

    // Cible 3: Research Lab Network
    system->targets[2] = (AdvancedTarget){
        .name = "Research Lab Network",
        .security_rating = 70,
        .defense_count = 2,
        .has_ai_guardian = true,
        .quantum_encrypted = false,
        .corporate_level = 3};
    system->targets[2].defenses[0] = (DefenseSystem){
        .type = DEFENSE_TYPE_AI_GUARDIAN,
        .strength = 70,
        .adaptive_level = 80,
        .is_learning = true,
        .attack_count = 0,
        .name = "LabWatch AI"};
    system->targets[2].defenses[1] = (DefenseSystem){
        .type = DEFENSE_TYPE_HONEYPOT,
        .strength = 60,
        .adaptive_level = 50,
        .is_learning = false,
        .attack_count = 0,
        .name = "DecoyNet"};

    // Cible 4: Government Database
    system->targets[3] = (AdvancedTarget){
        .name = "Government Database",
        .security_rating = 90,
        .defense_count = 3,
        .has_ai_guardian = true,
        .quantum_encrypted = true,
        .corporate_level = 5};
    system->targets[3].defenses[0] = (DefenseSystem){
        .type = DEFENSE_TYPE_AI_GUARDIAN,
        .strength = 88,
        .adaptive_level = 90,
        .is_learning = true,
        .attack_count = 0,
        .name = "SentinelAI MK-VII"};
    system->targets[3].defenses[1] = (DefenseSystem){
        .type = DEFENSE_TYPE_QUANTUM_ENCRYPTION,
        .strength = 92,
        .adaptive_level = 75,
        .is_learning = false,
        .attack_count = 0,
        .name = "QuantumShield Gov"};
    system->targets[3].defenses[2] = (DefenseSystem){
        .type = DEFENSE_TYPE_BEHAVIORAL_ANALYSIS,
        .strength = 85,
        .adaptive_level = 85,
        .is_learning = true,
        .attack_count = 0,
        .name = "GovWatch Behavioral"};

    // Cible 5: Underground Market Server
    system->targets[4] = (AdvancedTarget){
        .name = "Underground Market Server",
        .security_rating = 55,
        .defense_count = 1,
        .has_ai_guardian = false,
        .quantum_encrypted = false,
        .corporate_level = 2};
    system->targets[4].defenses[0] = (DefenseSystem){
        .type = DEFENSE_TYPE_FIREWALL,
        .strength = 55,
        .adaptive_level = 40,
        .is_learning = false,
        .attack_count = 0,
        .name = "BasicWall"};

    // Initialiser les outils cyber
    system->tool_count = 8;

    system->tools[0] = (CyberTool){
        .tool = TOOL_QUANTUM_COMPUTER,
        .name = "Quantum Processing Unit",
        .power_level = 95,
        .battery_life = 100,
        .is_active = false,
        .upgrade_level = 1,
        .description = "Processeur quantique pour calculs complexes"};

    system->tools[1] = (CyberTool){
        .tool = TOOL_AI_ASSISTANT,
        .name = "ECHO-Assistant v3.1",
        .power_level = 75,
        .battery_life = 100,
        .is_active = false,
        .upgrade_level = 1,
        .description = "Assistant IA spécialisé en hacking"};

    system->tools[2] = (CyberTool){
        .tool = TOOL_NEURAL_INTERFACE,
        .name = "BrainLink Neural Interface",
        .power_level = 90,
        .battery_life = 100,
        .is_active = false,
        .upgrade_level = 1,
        .description = "Interface neurale directe"};

    system->tools[3] = (CyberTool){
        .tool = TOOL_STEALTH_CLOAK,
        .name = "Ghost Cloak v2.0",
        .power_level = 65,
        .battery_life = 100,
        .is_active = false,
        .upgrade_level = 1,
        .description = "Système de furtivité avancé"};

    system->tools[4] = (CyberTool){
        .tool = TOOL_VIRUS_LABORATORY,
        .name = "VirLab Arsenal",
        .power_level = 70,
        .battery_life = 100,
        .is_active = false,
        .upgrade_level = 1,
        .description = "Laboratoire de création de virus"};

    system->tools[5] = (CyberTool){
        .tool = TOOL_SOCIAL_PROFILE_DB,
        .name = "SocialNet Database",
        .power_level = 80,
        .battery_life = 100,
        .is_active = false,
        .upgrade_level = 1,
        .description = "Base de données de profils sociaux"};

    system->tools[6] = (CyberTool){
        .tool = TOOL_ZERO_DAY_EXPLOIT,
        .name = "0-Day Exploit Kit",
        .power_level = 95,
        .battery_life = 100,
        .is_active = false,
        .upgrade_level = 1,
        .description = "Collection d'exploits zero-day"};

    system->tools[7] = (CyberTool){
        .tool = TOOL_GHOST_PROTOCOL,
        .name = "Ghost Protocol Suite",
        .power_level = 50,
        .battery_life = 100,
        .is_active = false,
        .upgrade_level = 1,
        .description = "Protocol d'invisibilité totale"};

    // Initialiser le système furtivité
    system->stealth = (StealthSystem){
        .stealth_level = 1,
        .detection_meter = 0,
        .ghost_mode_active = false,
        .last_stealth_use = 0,
        .stealth_duration = 300, // 5 minutes
        .stealth_cooldown = 600  // 10 minutes
    };

    // Initialiser le système quantique
    system->quantum = (QuantumSystem){
        .quantum_cores = 0,
        .processing_power = 0,
        .entanglement_active = false,
        .decrypt_speed_multiplier = 1,
        .temporal_hack_unlocked = false};

    // Initialiser l'assistant IA
    system->ai_assistant = (AIAssistant){
        .name = "ECHO",
        .intelligence_level = 1,
        .is_loyal = true,
        .trust_rating = 50,
        .personality = "Analytique et méthodique",
        .can_learn = true,
        .hack_assistance_bonus = 10};

    // Statistiques du joueur
    system->player_hacking_level = 1;
    system->neural_interface_sync = 0;
    system->god_mode_unlocked = false;
}

void display_hacking_menu(AdvancedHackingSystem *system)
{
    printf("\n");
    printf("%s╔══════════════════════════════════════════════════════════════════╗%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);
    printf("%s║                      ADVANCED HACKING SUITE                     ║%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);
    printf("%s╚══════════════════════════════════════════════════════════════════╝%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);
    printf("\n");

    printf("%sNiveau de hacking:%s %d/10\n", COLOR_YELLOW, COLOR_RESET, system->player_hacking_level);
    printf("%sSynchronisation neurale:%s %d%%\n", COLOR_CYAN, COLOR_RESET, system->neural_interface_sync);

    if (system->stealth.ghost_mode_active)
    {
        printf("%s[MODE FANTÔME ACTIF]%s\n", COLOR_GREEN, COLOR_RESET);
    }

    if (system->quantum.entanglement_active)
    {
        printf("%s[INTRICATION QUANTIQUE ACTIVE]%s\n", COLOR_MAGENTA, COLOR_RESET);
    }

    printf("\n%sMéthodes de hacking disponibles:%s\n", COLOR_BRIGHT_GREEN, COLOR_RESET);
    for (int i = 0; i < system->method_count; i++)
    {
        HackingMethod *method = &system->methods[i];
        char *color = method->requires_tool ? COLOR_YELLOW : COLOR_GREEN;

        printf("  %s[%d]%s %s - Succès: %d%% | Détection: %d%% | Temps: %ds\n",
               color, i + 1, COLOR_RESET, method->description,
               method->success_rate, method->detection_risk, method->time_required);

        if (method->requires_tool)
        {
            const CyberTool *tool = &system->tools[method->required_tool];
            printf("      %sRequiert:%s %s %s%s%s\n", COLOR_RED, COLOR_RESET, tool->name,
                   tool->is_active ? COLOR_GREEN : COLOR_RED, tool->is_active ? "[ACTIF]" : "[INACTIF]",
                   COLOR_RESET);
        }
    }

    printf("\n%sCommandes spéciales:%s\n", COLOR_MAGENTA, COLOR_RESET);
    printf("  advhack <cible>            - Lancer un hack avancé (la méthode est demandée ensuite)\n");
    printf("  stealthmode                - Activer/désactiver mode furtif\n");
    printf("  quantumdecrypt <data>      - Décryptage quantique\n");
    printf("  aiassist <cible>           - Assistance IA pour hack\n");
    printf("  neuralsync                 - Synchroniser interface neurale\n");
    printf("  analyzedefenses <cible>    - Analyser les défenses\n");
    printf("  socialeng <cible>          - Ingénierie sociale\n");
    printf("  temporalhack <cible>       - Hack temporel (expérimental)\n");
}

bool attempt_advanced_hack(AdvancedHackingSystem *system, int target_id, HackType method, Player *player, AlertSystem *alert, int chance_bonus)
{
    if (target_id < 0 || target_id >= system->target_count)
    {
        printf("%sErreur: Cible invalide.%s\n", COLOR_RED, COLOR_RESET);
        return false;
    }
    AdvancedTarget *target = &system->targets[target_id];
    HackingMethod *selected_method = NULL;

    // Trouver la méthode correspondante
    for (int i = 0; i < system->method_count; i++)
    {
        if (system->methods[i].type == method)
        {
            selected_method = &system->methods[i];
            break;
        }
    }
    if (!selected_method)
    {
        printf("%sErreur: Méthode de hack invalide.%s\n", COLOR_RED, COLOR_RESET);
        return false;
    }

    printf("\n%s>>> LANCEMENT DU HACK AVANCÉ <<<-%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);
    printf("%sCible:%s %s\n", COLOR_YELLOW, COLOR_RESET, target->name);
    printf("%sMéthode:%s %s\n", COLOR_YELLOW, COLOR_RESET, selected_method->description);
    printf("\n"); // Vérifier si l'outil requis est disponible
    if (selected_method->requires_tool)
    {
        CyberTool *required_tool = &system->tools[selected_method->required_tool];
        if (!required_tool->is_active)
        {
            printf("%sErreur: %s requis mais non actif.%s\n",
                   COLOR_RED, required_tool->name, COLOR_RESET);
            return false;
        }

        if (required_tool->battery_life < selected_method->energy_cost)
        {
            printf("%sErreur: Batterie insuffisante (%d%% requis, %d%% disponible).%s\n",
                   COLOR_RED, selected_method->energy_cost, required_tool->battery_life, COLOR_RESET);
            return false;
        }
    }

    // Animation de hack
    printf("%sInitialisation de l'attaque...%s\n", COLOR_CYAN, COLOR_RESET);
    for (int i = 0; i < 20; i++)
    {
        printf("█");
        fflush(stdout);
        nh_sleep_ms((unsigned)selected_method->time_required);
    }
    printf("\n");

    // Calcul du taux de succès
    int final_success_rate = calculate_hack_success_rate(system, selected_method, target, player) + chance_bonus;
    if (final_success_rate < 1)
        final_success_rate = 1;
    if (final_success_rate > 99)
        final_success_rate = 99;
    int roll = rand() % 100;

    printf("\n%sAnalyse des défenses...%s\n", COLOR_YELLOW, COLOR_RESET);
    display_defense_analysis(target);

    printf("\n%sTaux de succès calculé: %d%%%s\n", COLOR_CYAN, final_success_rate, COLOR_RESET);

    if (roll < final_success_rate)
    {
        printf("\n%s🎯 HACK RÉUSSI ! 🎯%s\n", COLOR_BRIGHT_GREEN, COLOR_RESET);
        // Crédits, expérience et fichiers : versés une seule fois par le monde (nh_world_compromise)
        if (selected_method->requires_tool)
        {
            system->tools[selected_method->required_tool].battery_life -= selected_method->energy_cost;
        }

        // Augmenter le niveau de hack
        system->player_hacking_level += (target->corporate_level > 3) ? 1 : 0;

        return true;
    }
    else
    {
        printf("\n%s❌ HACK ÉCHOUÉ ❌%s\n", COLOR_RED, COLOR_RESET); // Gestion de la détection
        int detection_severity = (selected_method->detection_risk + (100 - final_success_rate)) / 20;
        handle_detection(target, alert, detection_severity);

        return false;
    }
}

int calculate_hack_success_rate(const AdvancedHackingSystem *system, HackingMethod *method, AdvancedTarget *target, Player *player)
{
    int base_rate = method->success_rate;

    // Bonus du niveau du joueur
    base_rate += player->level * 5;

    // Malus de la sécurité de la cible
    base_rate -= target->security_rating / 2;

    // Bonus d'expérience
    base_rate += player->experience / 100;

    // Bonus des outils
    if (method->requires_tool)
    {
        const CyberTool *tool = &system->tools[method->required_tool];
        base_rate += tool->power_level / 10;
        base_rate += tool->upgrade_level * 5;
    }

    // Bonus de furtivité
    if (system->stealth.ghost_mode_active)
    {
        base_rate += 15;
    }

    // Bonus quantique
    if (system->quantum.entanglement_active)
    {
        base_rate += 20;
    }

    // Bonus IA
    if (system->ai_assistant.is_loyal)
    {
        base_rate += system->ai_assistant.hack_assistance_bonus;
    }

    // Malus des défenses adaptatives
    for (int i = 0; i < target->defense_count; i++)
    {
        DefenseSystem *defense = &target->defenses[i];
        if (defense->is_learning && defense->attack_count > 0)
        {
            base_rate -= defense->adaptive_level / 10;
        }
    }

    // Limitation entre 1 et 99
    if (base_rate < 1)
        base_rate = 1;
    if (base_rate > 99)
        base_rate = 99;

    return base_rate;
}

// ===== FONCTIONS MANQUANTES =====

bool temporal_hack_attempt(AdvancedHackingSystem *system, int target_id, Player *player, AlertSystem *alert)
{
    if (player->level < 6)
    {
        printf("%sNiveau 6 requis pour le hack temporel!%s\n", COLOR_RED, COLOR_RESET);
        return false;
    }

    if (target_id < 0 || target_id >= system->target_count)
    {
        printf("%sCible invalide!%s\n", COLOR_RED, COLOR_RESET);
        return false;
    }

    AdvancedTarget *target = &system->targets[target_id];

    printf("%s=== INITIATION DU HACK TEMPOREL ===%s\n", COLOR_MAGENTA, COLOR_RESET);
    printf("Cible: %s%s%s\n", COLOR_CYAN, target->name, COLOR_RESET);
    printf("Manipulation du flux temporel en cours...\n");

    // Animation de hack temporel
    for (int i = 0; i < 5; i++)
    {
        printf(".");
        fflush(stdout);
        nh_sleep_ms(500);
    }

    int success_chance = 30 + (player->level * 5) + system->quantum.processing_power;
    int roll = rand() % 100;

    if (roll < success_chance)
    {
        printf("\n%sHACK TEMPOREL RÉUSSI!%s\n", COLOR_GREEN, COLOR_RESET);
        printf("Les défenses ont été contournées via manipulation temporelle!\n");
        return true;
    }
    else
    {
        printf("\n%sÉchec du hack temporel! Paradoxe temporel détecté!%s\n", COLOR_RED, COLOR_RESET);
        nh_alert_raise(alert, 20);
        return false;
    }
}

void display_defense_analysis(AdvancedTarget *target)
{
    printf("%s=== ANALYSE DES DÉFENSES ===%s\n", COLOR_CYAN, COLOR_RESET);
    printf("Cible: %s%s%s\n", COLOR_YELLOW, target->name, COLOR_RESET);
    printf("Niveau de sécurité: %s%d/10%s\n",
           target->security_rating > 7 ? COLOR_RED : COLOR_YELLOW,
           target->security_rating, COLOR_RESET);

    printf("\nDéfenses actives:\n");
    for (int i = 0; i < target->defense_count; i++)
    {
        DefenseSystem *def = &target->defenses[i];
        printf("- %s%s%s (Force: %d)\n", COLOR_BLUE, def->name, COLOR_RESET, def->strength);
    }

    if (target->has_ai_guardian)
    {
        printf("⚠️  %sIA Gardienne détectée!%s\n", COLOR_RED, COLOR_RESET);
    }

    if (target->quantum_encrypted)
    {
        printf("⚠️  %sChiffrement quantique actif!%s\n", COLOR_MAGENTA, COLOR_RESET);
    }
}

int social_engineering_chance(const Player *player, const AdvancedTarget *target)
{
    /* security_rating est sur 100 : l'ancienne formule (rating × 5) le lisait sur 10 et rendait
     * l'attaque impossible. Valeurs provisoires (équilibrage en phase 5). */
    int chance = 50 + player->level * 5 + player->reputation * 2 - target->security_rating / 2;
    if (chance < 5)
        return 5;
    if (chance > 95)
        return 95;
    return chance;
}

bool social_engineering_attack(AdvancedHackingSystem *system, int target_id, Player *player, AlertSystem *alert)
{
    if (target_id < 0 || target_id >= system->target_count)
    {
        printf("%sCible invalide!%s\n", COLOR_RED, COLOR_RESET);
        return false;
    }

    AdvancedTarget *target = &system->targets[target_id];

    printf("%s=== ATTAQUE D'INGÉNIERIE SOCIALE ===%s\n", COLOR_YELLOW, COLOR_RESET);
    printf("Cible: %s%s%s\n", COLOR_CYAN, target->name, COLOR_RESET);

    printf("Techniques disponibles:\n");
    printf("1. Phishing par email\n");
    printf("2. Appel téléphonique (vishing)\n");
    printf("3. Infiltration physique\n");
    printf("4. Manipulation des réseaux sociaux\n");

    int technique = (rand() % 4) + 1;
    printf("Utilisation de la technique %d...\n", technique);

    // Animation
    for (int i = 0; i < 3; i++)
    {
        printf(".");
        fflush(stdout);
        nh_sleep_ms(800);
    }
    int success_chance = social_engineering_chance(player, target);
    int roll = rand() % 100;

    if (roll < success_chance)
    {
        printf("\n%sINGÉNIERIE SOCIALE RÉUSSIE!%s\n", COLOR_GREEN, COLOR_RESET);
        printf("Informations d'accès obtenues via manipulation humaine!\n");
        return true;
    }
    else
    {
        printf("\n%sÉchec de l'ingénierie sociale! Suspicion éveillée!%s\n", COLOR_RED, COLOR_RESET);
        nh_alert_raise(alert, 10);
        return false;
    }
}

void display_neural_interface_status(AdvancedHackingSystem *system)
{
    printf("%s=== STATUT INTERFACE NEURALE ===%s\n", COLOR_MAGENTA, COLOR_RESET);
    printf("Niveau de synchronisation: %s%d%%%s\n",
           system->neural_interface_sync > 70 ? COLOR_GREEN : COLOR_YELLOW,
           system->neural_interface_sync, COLOR_RESET);

    if (system->neural_interface_sync >= 100)
    {
        printf("🧠 %sInterface parfaitement synchronisée!%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);
        printf("Capacités neurales maximales débloquées!\n");
    }
    else if (system->neural_interface_sync >= 50)
    {
        printf("⚡ %sSynchronisation partielle active%s\n", COLOR_YELLOW, COLOR_RESET);
    }
    else
    {
        printf("⚠️  %sSynchronisation faible - Performance réduite%s\n", COLOR_RED, COLOR_RESET);
    }

    printf("Mode Dieu: %s%s%s\n",
           system->god_mode_unlocked ? COLOR_GREEN : COLOR_RED,
           system->god_mode_unlocked ? "ACTIVÉ" : "DÉSACTIVÉ",
           COLOR_RESET);
}

void display_available_tools(AdvancedHackingSystem *system)
{
    printf("\n%s╔══════════════════════════════════════════════════════════════════╗%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);
    printf("%s║                        ARSENAL CYBER                            ║%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);
    printf("%s╚══════════════════════════════════════════════════════════════════╝%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);

    for (int i = 0; i < system->tool_count; i++)
    {
        CyberTool *tool = &system->tools[i];
        char *status_color = tool->is_active ? COLOR_GREEN : COLOR_RED;
        char *status_text = tool->is_active ? "ACTIF" : "INACTIF";

        printf("\n%s[%d] %s%s (%sNiv.%d%s)\n",
               COLOR_YELLOW, i + 1, tool->name, COLOR_RESET,
               COLOR_CYAN, tool->upgrade_level, COLOR_RESET);
        printf("    %s%s%s | Puissance: %d%% | Batterie: %d%%\n",
               status_color, status_text, COLOR_RESET,
               tool->power_level, tool->battery_life);
        printf("    %s%s%s\n", COLOR_MAGENTA, tool->description, COLOR_RESET);
    }

    printf("\n%sCommandes d'outils:%s\n", COLOR_BRIGHT_GREEN, COLOR_RESET);
    printf("  tools                    - Afficher cet arsenal\n");
    printf("  activate <numéro>        - Activer un outil\n");
    printf("  deactivate <numéro>      - Désactiver un outil\n");
    printf("  upgrade <numéro>         - Améliorer un outil (coûte des crédits)\n");
    printf("  recharge <numéro>        - Recharger la batterie d'un outil\n");
}

bool use_hacking_tool(AdvancedHackingSystem *system, HackingTool tool_type, Player *player)
{
    CyberTool *tool = NULL;

    // Trouver l'outil
    for (int i = 0; i < system->tool_count; i++)
    {
        if (system->tools[i].tool == tool_type)
        {
            tool = &system->tools[i];
            break;
        }
    }

    if (!tool)
    {
        printf("%sOutil non trouvé.%s\n", COLOR_RED, COLOR_RESET);
        return false;
    }

    if (!tool->is_active)
    {
        printf("%s%s n'est pas activé.%s\n", COLOR_RED, tool->name, COLOR_RESET);
        return false;
    }

    if (tool->battery_life < 20)
    {
        printf("%sBatterie de %s trop faible (%d%%).%s\n",
               COLOR_RED, tool->name, tool->battery_life, COLOR_RESET);
        return false;
    }

    printf("%s%s activé avec succès !%s\n", COLOR_GREEN, tool->name, COLOR_RESET);
    tool->battery_life -= 20;

    return true;
}

void update_stealth_system(StealthSystem *stealth)
{
    time_t current_time = time(NULL);

    // Vérifier si le mode fantôme doit être désactivé
    if (stealth->ghost_mode_active)
    {
        if (current_time - stealth->last_stealth_use > stealth->stealth_duration)
        {
            stealth->ghost_mode_active = false;
            stealth->detection_meter = 0;
            printf("%sMode fantôme expiré.%s\n", COLOR_YELLOW, COLOR_RESET);
        }
    }

    // Réduction naturelle du compteur de détection
    if (stealth->detection_meter > 0 && !stealth->ghost_mode_active)
    {
        stealth->detection_meter -= 1;
        if (stealth->detection_meter < 0)
        {
            stealth->detection_meter = 0;
        }
    }
}

void train_ai_assistant(AIAssistant *ai, int experience_points)
{
    ai->intelligence_level += experience_points / 50;
    ai->hack_assistance_bonus += experience_points / 100;

    if (ai->intelligence_level > 10)
    {
        ai->intelligence_level = 10;
    }

    if (ai->hack_assistance_bonus > 30)
    {
        ai->hack_assistance_bonus = 30;
    }

    printf("%s%s a gagné en intelligence ! Niveau: %d/10%s\n",
           COLOR_GREEN, ai->name, ai->intelligence_level, COLOR_RESET);
}

void handle_detection(AdvancedTarget *target, AlertSystem *alert, int severity)
{
    printf("\n%s⚠️  DÉTECTION DE SÉCURITÉ ⚠️%s\n", COLOR_RED, COLOR_RESET);
    printf("Niveau de sévérité: %d\n", severity);
    int old_alert = alert->level;

    if (severity >= 5)
    {
        printf("%s🚨 ALERTE MAXIMALE - CONTRE-ATTAQUE ACTIVÉE! 🚨%s\n", COLOR_RED, COLOR_RESET);
        nh_alert_raise(alert, 30);
        printf("Votre position a été compromise!\n");
    }
    else if (severity >= 3)
    {
        printf("%s⚠️  Intrusion détectée - Sécurité renforcée%s\n", COLOR_YELLOW, COLOR_RESET);
        nh_alert_raise(alert, 15);
        target->security_rating += 1;
    }
    else if (severity >= 1)
    {
        printf("%s👁️  Activité suspecte notée%s\n", COLOR_YELLOW, COLOR_RESET);
        nh_alert_raise(alert, 5);
    }

    printf("\n");
    printf(nh_tr(NH_STR_ALERT_NOW), old_alert, alert->level);
    printf("\n");
}
