#ifndef ADVANCED_HACKING_H
#define ADVANCED_HACKING_H

#include "alert.h"
#include "game_types.h"
#include <stdbool.h>
#include <time.h>

// Énumérations pour mécaniques avancées
typedef enum
{
    HACK_TYPE_BASIC,
    HACK_TYPE_QUANTUM,
    HACK_TYPE_AI_ASSISTED,
    HACK_TYPE_NEURAL,
    HACK_TYPE_BACKDOOR,
    HACK_TYPE_VIRUS,
    HACK_TYPE_SOCIAL_ENGINEER,
    HACK_TYPE_STEALTH
} HackType;

typedef enum
{
    DEFENSE_TYPE_FIREWALL,
    DEFENSE_TYPE_INTRUSION_DETECTION,
    DEFENSE_TYPE_AI_GUARDIAN,
    DEFENSE_TYPE_QUANTUM_ENCRYPTION,
    DEFENSE_TYPE_HONEYPOT,
    DEFENSE_TYPE_BEHAVIORAL_ANALYSIS
} DefenseType;

typedef enum
{
    TOOL_QUANTUM_COMPUTER,
    TOOL_AI_ASSISTANT,
    TOOL_NEURAL_INTERFACE,
    TOOL_STEALTH_CLOAK,
    TOOL_VIRUS_LABORATORY,
    TOOL_SOCIAL_PROFILE_DB,
    TOOL_ZERO_DAY_EXPLOIT,
    TOOL_GHOST_PROTOCOL
} HackingTool;

// Structures pour mécaniques avancées
typedef struct
{
    HackType type;
    int success_rate;
    int detection_risk;
    int time_required;
    int energy_cost;
    bool requires_tool;
    HackingTool required_tool;
    char description[200];
} HackingMethod;

typedef struct
{
    DefenseType type;
    int strength;
    int adaptive_level;
    bool is_learning;
    time_t last_attack_time;
    int attack_count;
    char name[50];
} DefenseSystem;

typedef struct
{
    char name[50];
    int security_rating;
    DefenseSystem defenses[3];
    int defense_count;
    bool has_ai_guardian;
    bool quantum_encrypted;
    int corporate_level; // 1-5, 5 étant les plus dangereux
} AdvancedTarget;

typedef struct
{
    HackingTool tool;
    char name[50];
    int power_level;
    int battery_life;
    bool is_active;
    int upgrade_level;
    char description[200];
} CyberTool;

typedef struct
{
    int stealth_level;
    int detection_meter;
    int detection_reduction;
    bool ghost_mode_active;
    bool is_active;
    time_t last_stealth_use;
    int stealth_duration;
    int stealth_cooldown;
} StealthSystem;

typedef struct
{
    int quantum_cores;
    int processing_power;
    bool entanglement_active;
    int decrypt_speed_multiplier;
    bool temporal_hack_unlocked;
} QuantumSystem;

typedef struct
{
    char name[50];
    int intelligence_level;
    int efficiency;
    bool is_loyal;
    int trust_rating;
    char personality[100];
    bool can_learn;
    int hack_assistance_bonus;
} AIAssistant;

// Structure principale pour toutes les mécaniques avancées
typedef struct
{
    HackingMethod methods[8];
    int method_count;
    AdvancedTarget targets[10];
    int target_count;
    CyberTool tools[8];
    int tool_count;
    StealthSystem stealth;
    QuantumSystem quantum;
    AIAssistant ai_assistant;
    int player_hacking_level;
    int neural_interface_sync;
    bool god_mode_unlocked;
} AdvancedHackingSystem;

// Prototypes des fonctions
void init_advanced_hacking_system(AdvancedHackingSystem *system);
void display_hacking_menu(AdvancedHackingSystem *system);
bool attempt_advanced_hack(AdvancedHackingSystem *system, int target_id, HackType method, Player *player, AlertSystem *alert, int chance_bonus);
void display_available_tools(AdvancedHackingSystem *system);
bool use_hacking_tool(AdvancedHackingSystem *system, HackingTool tool, Player *player);
void update_stealth_system(StealthSystem *stealth);
void train_ai_assistant(AIAssistant *ai, int experience_points);
bool install_backdoor_advanced(AdvancedHackingSystem *system, int target_id, Player *player);
bool upload_advanced_virus(AdvancedHackingSystem *system, int target_id, char *virus_type, Player *player);
void display_defense_analysis(AdvancedTarget *target);
int calculate_hack_success_rate(const AdvancedHackingSystem *system, HackingMethod *method, AdvancedTarget *target, Player *player);
void handle_detection(AdvancedTarget *target, AlertSystem *alert, int severity);
int social_engineering_chance(const Player *player, const AdvancedTarget *target);
bool social_engineering_attack(AdvancedHackingSystem *system, int target_id, Player *player, AlertSystem *alert);
void display_neural_interface_status(AdvancedHackingSystem *system);
bool temporal_hack_attempt(AdvancedHackingSystem *system, int target_id, Player *player, AlertSystem *alert);

#endif
