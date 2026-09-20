#ifndef GAME_TYPES_H
#define GAME_TYPES_H

#include <stdbool.h>
#include <time.h>

// Constantes
#define MAX_NAME_LENGTH 50
#define MAX_INPUT_LENGTH 100
#define MAX_COMMANDS 12

// Nom du héros tant que le joueur n'en a pas choisi un autre (prologue, voir intro.c)
#define NH_DEFAULT_NAME "Case"

// Énumérations
typedef enum
{
    LEVEL_NOVICE = 1,
    LEVEL_APPRENTICE = 2,
    LEVEL_HACKER = 3,
    LEVEL_EXPERT = 4,
    LEVEL_MASTER = 5,
    LEVEL_LEGEND = 6
} HackerLevel;

typedef enum
{
    SECURITY_LOW = 1,
    SECURITY_MEDIUM = 2,
    SECURITY_HIGH = 3,
    SECURITY_CRITICAL = 4
} SecurityLevel;

typedef enum
{
    CMD_SCAN = 0,
    CMD_BRUTEFORCE = 1,
    CMD_DECRYPT = 2,
    CMD_EXPLOIT = 3,
    CMD_BACKDOOR = 4,
    CMD_TRACE_ROUTE = 5,
    CMD_UPLOAD_VIRUS = 6,
    CMD_AI_HACK = 7,
    CMD_QUANTUM_DECRYPT = 8,
    CMD_SHOP = 9,
    CMD_LAY_LOW = 10
} CommandType;

// Structures
typedef struct
{
    char filename[100];
    char content[500];
    int encryption_level;
    bool is_unlocked;
    int credits_value;
} DataFile;

typedef struct
{
    char name[50];
    char description[200];
    int damage;
    int stealth_rating;
    bool is_detected;
} Virus;

typedef struct
{
    char corporation[50];
    int security_response;
    int trace_progress;
    bool is_tracing;
    time_t last_alert_time;
} SecuritySystem;

typedef struct
{
    char name[MAX_NAME_LENGTH];
    HackerLevel level;
    int experience;
    int scans_done;      // scans déjà comptés pour l'expérience (voir progression.c)
    unsigned milestones; // récompenses uniques déjà obtenues (bits NhMilestone)
    int credits;
    int reputation;
    int stealth_rating;
    bool commands_unlocked[MAX_COMMANDS];
    bool game_over;

    // Nouvelles mécaniques avancées
    bool has_quantum_computer;
    bool has_ai_assistant;
    bool has_encryption_key; // clé de chiffrement (boutique) : les fichiers de bas niveau s'ouvrent à la compromission
    int xp_boost;            // gains d'expérience qu'il reste à majorer (boutique, voir nh_grant_xp)
    int virus_library_size;
    int backdoors_active;
    time_t last_hack_time;
} Player;

// Avancement de la mission-tutoriel (voir tutorial.c)
typedef struct
{
    int step;  // étape en cours (NhTutStep) ; 0 = pas de tutoriel en cours
    bool done; // terminé ou passé : il ne reviendra plus
} TutorialState;

typedef struct
{
    char name[MAX_NAME_LENGTH];
    SecurityLevel security;
    bool is_compromised;
    bool has_backdoor;
    bool has_virus;
    bool is_traced;
    bool is_discovered; // révélé par `scan` (niveau de découverte atteint)
    bool has_intel;     // accès internes obtenus par ingénierie sociale (+chance sur ce système)
    int data_value;
    int firewall_strength;
    char corporation[50];
    DataFile secret_files[5];
    int file_count;
} NetworkNode;

#endif // GAME_TYPES_H
