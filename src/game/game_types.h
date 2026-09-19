#ifndef GAME_TYPES_H
#define GAME_TYPES_H

#include <stdbool.h>
#include <time.h>

// Constantes
#define MAX_NAME_LENGTH 50
#define MAX_INPUT_LENGTH 100
#define MAX_COMMANDS 12

// Énumérations
typedef enum
{
    LEVEL_NOVICE = 1,
    LEVEL_APPRENTICE = 2,
    LEVEL_HACKER = 3,
    LEVEL_EXPERT = 4,
    LEVEL_MASTER = 5
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
    int alert_level;
    int credits;
    int reputation;
    int stealth_rating;
    bool commands_unlocked[MAX_COMMANDS];
    bool game_over;

    // Nouvelles mécaniques avancées
    bool has_quantum_computer;
    bool has_ai_assistant;
    int virus_library_size;
    int backdoors_active;
    time_t last_hack_time;
} Player;

typedef struct
{
    char name[MAX_NAME_LENGTH];
    SecurityLevel security;
    bool is_compromised;
    bool has_backdoor;
    bool has_virus;
    bool is_traced;
    int data_value;
    int firewall_strength;
    char corporation[50];
    DataFile secret_files[5];
    int file_count;
} NetworkNode;

#endif // GAME_TYPES_H
