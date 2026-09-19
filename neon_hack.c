#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stdbool.h>
#include <unistd.h>
#include <ctype.h>

// Includes des nouveaux modules
#include "src/game/game_types.h"
#include "src/game/shop.h"
#include "src/game/alert_system.h"
#include "src/game/quest_system.h"
#include "src/game/contacts.h"
#include "src/game/advanced_hacking.h"

// Constantes
#define MAX_NAME_LENGTH 50
#define MAX_INPUT_LENGTH 100
// Codes couleur ANSI
#define COLOR_RESET "\033[0m"
#define COLOR_RED "\033[31m"
#define COLOR_GREEN "\033[32m"
#define COLOR_YELLOW "\033[33m"
#define COLOR_BLUE "\033[34m"
#define COLOR_MAGENTA "\033[35m"
#define COLOR_CYAN "\033[36m"
#define COLOR_WHITE "\033[37m"
#define COLOR_BRIGHT_GREEN "\033[92m"
#define COLOR_BRIGHT_CYAN "\033[96m"

// Variables globales
Player game_player;
NetworkNode network_nodes[5];
int discovered_nodes = 0;
bool game_running = true;

// Nouvelles variables globales pour mécaniques avancées
SecuritySystem global_security;
Virus available_viruses[10];
int virus_count = 0;
bool stealth_mode = false;
int global_heat_level = 0;

// Variables globales pour les nouveaux modules
CyberShop global_shop;
AlertSystem alert_system;
QuestSystem quest_system;
ContactSystem contact_system;

// Prototypes
void init_game(void);
void game_loop(void);
void display_intro(void);
void display_help(void);
void process_command(char *input);
void print_colored_text(char *text, char *color);
void print_cyberpunk_art(void);
void print_typing_effect(char *text, int delay_ms);
bool cmd_scan_network(void);
bool cmd_bruteforce(char *target);
bool cmd_decrypt(char *data);
bool parse_command(char *input, char *command, char *argument);
void gain_experience(int exp);
void increase_alert_level(int amount);

// Nouveaux prototypes pour mécaniques avancées
bool cmd_backdoor(char *target);
bool cmd_trace_route(char *target);
bool cmd_upload_virus(char *target);
bool cmd_stealth_mode(void);
bool cmd_ai_hack(char *target);
bool cmd_quantum_decrypt(char *data);
void init_virus_library(void);
void init_security_system(void);
void update_global_heat(void);
void display_advanced_status(void);
bool purchase_upgrade(char *upgrade_name);

// Prototypes pour les nouveaux modules
bool cmd_shop(void);
bool cmd_lay_low(void);
bool cmd_quests(void);
bool cmd_contacts(void);
bool cmd_interact_contact(char *argument);
bool cmd_messages(void);
bool cmd_read(char *argument);

// Prototypes pour les commandes de hacking avancé
bool cmd_advanced_hack(char *target_name);
bool cmd_stealth_mode_toggle(void);
bool cmd_quantum_decrypt_advanced(char *encrypted_data);
bool cmd_ai_assist_hack(char *target_name);
bool cmd_neural_sync(void);
bool cmd_analyze_defenses(char *target_name);
bool cmd_social_engineer(char *target_name);
bool cmd_temporal_hack(char *target_name);

// Implémentations
void print_colored_text(char *text, char *color)
{
    printf("%s%s%s", color, text, COLOR_RESET);
}

void print_cyberpunk_art(void)
{
    print_colored_text("    ███▄    █ ▓█████  ▒█████   ███▄    █     ██░ ██  ▄▄▄       ▄████▄   ██ ▄█▀\n", COLOR_BRIGHT_CYAN);
    print_colored_text("    ██ ▀█   █ ▓█   ▀ ▒██▒  ██▒ ██ ▀█   █    ▓██░ ██▒▒████▄    ▒██▀ ▀█   ██▄█▒ \n", COLOR_CYAN);
    print_colored_text("   ▓██  ▀█ ██▒▒███   ▒██░  ██▒▓██  ▀█ ██▒   ▒██▀▀██░▒██  ▀█▄  ▒▓█    ▄ ▓███▄░ \n", COLOR_BRIGHT_CYAN);
    print_colored_text("   ▓██▒  ▐▌██▒▒▓█  ▄ ▒██   ██░▓██▒  ▐▌██▒   ░▓█ ░██ ░██▄▄▄▄██ ▒▓▓▄ ▄██▒▓██ █▄ \n", COLOR_CYAN);
    print_colored_text("   ▒██░   ▓██░░▒████▒░ ████▓▒░▒██░   ▓██░   ░▓█▒░██▓ ▓█   ▓██▒▒ ▓███▀ ░▒██▒ █▄\n", COLOR_BRIGHT_CYAN);
    printf("\n");
    print_colored_text("                        [Cyberpunk Terminal RPG - 2087]\n", COLOR_MAGENTA);
}

void print_typing_effect(char *text, int delay_ms)
{
    for (int i = 0; text[i] != '\0'; i++)
    {
        printf("%c", text[i]);
        fflush(stdout);
        usleep(delay_ms * 1000);
    }
}

void init_game(void)
{
    srand(time(NULL));

    strcpy(game_player.name, "Anonymous");
    game_player.level = LEVEL_NOVICE;
    game_player.experience = 0;
    game_player.alert_level = 0;
    game_player.game_over = false;

    // Nouvelles propriétés du joueur
    game_player.credits = 100;      // Crédits de départ
    game_player.reputation = 0;     // Réputation neutre
    game_player.stealth_rating = 3; // Capacité furtivité de base
    game_player.has_quantum_computer = false;
    game_player.has_ai_assistant = false;
    game_player.virus_library_size = 0;
    game_player.backdoors_active = 0;
    game_player.last_hack_time = time(NULL);

    for (int i = 0; i < MAX_COMMANDS; i++)
    {
        game_player.commands_unlocked[i] = false;
    }
    game_player.commands_unlocked[CMD_SCAN] = true;

    // Initialiser les variables globales
    stealth_mode = false;
    global_heat_level = 0;

    // Initialiser les nouveaux modules
    init_shop(&global_shop);
    init_alert_system(&alert_system);
    init_quest_system(&quest_system);
    init_contact_system(&contact_system);
    init_advanced_hacking_system(&global_advanced_system);

    // Initialiser le système de sécurité global
    strcpy(global_security.corporation, "Nexus Corp");
    global_security.security_response = 0;
    global_security.trace_progress = 0;
    global_security.is_tracing = false;
    global_security.last_alert_time = time(NULL);

    // Initialiser le réseau avec nouvelles propriétés
    strcpy(network_nodes[0].name, "localhost");
    network_nodes[0].security = SECURITY_LOW;
    network_nodes[0].is_compromised = false;
    network_nodes[0].has_backdoor = false;
    network_nodes[0].has_virus = false;
    network_nodes[0].is_traced = false;
    network_nodes[0].data_value = 10;
    network_nodes[0].firewall_strength = 2;
    strcpy(network_nodes[0].corporation, "Independent");
    network_nodes[0].file_count = 1;
    strcpy(network_nodes[0].secret_files[0].filename, "user_data.txt");
    strcpy(network_nodes[0].secret_files[0].content, "Données utilisateur locales");
    network_nodes[0].secret_files[0].encryption_level = 1;
    network_nodes[0].secret_files[0].is_unlocked = false;
    network_nodes[0].secret_files[0].credits_value = 50;

    strcpy(network_nodes[1].name, "corp-server-01");
    network_nodes[1].security = SECURITY_MEDIUM;
    network_nodes[1].is_compromised = false;
    network_nodes[1].has_backdoor = false;
    network_nodes[1].has_virus = false;
    network_nodes[1].is_traced = false;
    network_nodes[1].data_value = 50;
    network_nodes[1].firewall_strength = 5;
    strcpy(network_nodes[1].corporation, "MegaCorp Industries");
    network_nodes[1].file_count = 2;
    strcpy(network_nodes[1].secret_files[0].filename, "employee_records.db");
    strcpy(network_nodes[1].secret_files[0].content, "Registres des employés");
    network_nodes[1].secret_files[0].encryption_level = 2;
    network_nodes[1].secret_files[0].is_unlocked = false;
    network_nodes[1].secret_files[0].credits_value = 200;
    strcpy(network_nodes[1].secret_files[1].filename, "financial_data.xlsx");
    strcpy(network_nodes[1].secret_files[1].content, "Données financières confidentielles");
    network_nodes[1].secret_files[1].encryption_level = 3;
    network_nodes[1].secret_files[1].is_unlocked = false;
    network_nodes[1].secret_files[1].credits_value = 500;

    strcpy(network_nodes[2].name, "nexus-mainframe");
    network_nodes[2].security = SECURITY_HIGH;
    network_nodes[2].is_compromised = false;
    network_nodes[2].has_backdoor = false;
    network_nodes[2].has_virus = false;
    network_nodes[2].is_traced = false;
    network_nodes[2].data_value = 200;
    network_nodes[2].firewall_strength = 8;
    strcpy(network_nodes[2].corporation, "Nexus Corp");
    network_nodes[2].file_count = 3;
    strcpy(network_nodes[2].secret_files[0].filename, "project_ghost.dat");
    strcpy(network_nodes[2].secret_files[0].content, "CLASSIFIED");
    network_nodes[2].secret_files[0].encryption_level = 4;
    network_nodes[2].secret_files[0].is_unlocked = false;
    network_nodes[2].secret_files[0].credits_value = 1000;
    strcpy(network_nodes[2].secret_files[1].filename, "neural_maps.bin");
    strcpy(network_nodes[2].secret_files[1].content, "Cartes neurales des citoyens");
    network_nodes[2].secret_files[1].encryption_level = 5;
    network_nodes[2].secret_files[1].is_unlocked = false;
    network_nodes[2].secret_files[1].credits_value = 1500;
    strcpy(network_nodes[2].secret_files[2].filename, "quantum_keys.qkey");
    strcpy(network_nodes[2].secret_files[2].content, "Clés de chiffrement quantique");
    network_nodes[2].secret_files[2].encryption_level = 6;
    network_nodes[2].secret_files[2].is_unlocked = false;
    network_nodes[2].secret_files[2].credits_value = 2000;

    discovered_nodes = 3;

    // Initialiser la bibliothèque de virus
    init_virus_library();

    display_intro();
}

void display_intro(void)
{
    printf("\n");
    print_colored_text("╔══════════════════════════════════════════════════════════════════╗\n", COLOR_CYAN);
    print_colored_text("║                           NEON HACK                             ║\n", COLOR_BRIGHT_CYAN);
    print_colored_text("║                    Terminal de Hacking v2.087                   ║\n", COLOR_CYAN);
    print_colored_text("╚══════════════════════════════════════════════════════════════════╝\n", COLOR_CYAN);

    print_typing_effect("\n=== NEON HACK - ANNÉE 2087 ===\n", 50);
    print_typing_effect("Neo-Tokyo brille sous les néons, mais l'obscurité règne dans les réseaux...\n", 30);
    print_typing_effect("Vous êtes un hacker novice avec un vieux terminal et de grands rêves.\n", 30);
    print_typing_effect("Votre mission : infiltrer Nexus Corp et découvrir leurs secrets.\n", 30);

    printf("\n");
    print_colored_text("Entrez votre nom de hacker : ", COLOR_YELLOW);
    fgets(game_player.name, MAX_NAME_LENGTH, stdin);
    game_player.name[strcspn(game_player.name, "\n")] = 0;

    printf("\nBienvenue dans l'ombre, %s%s%s.\n", COLOR_BRIGHT_CYAN, game_player.name, COLOR_RESET);
    printf("\n");
    print_colored_text("Tapez 'help' pour voir les commandes disponibles.\n", COLOR_CYAN);
    printf("\n");
}

void game_loop(void)
{
    char input[MAX_INPUT_LENGTH];

    while (game_running && !game_player.game_over)
    {
        printf("%s[%s@neon-terminal]%s $ ", COLOR_BRIGHT_GREEN, game_player.name, COLOR_RESET);

        if (fgets(input, sizeof(input), stdin) != NULL)
        {
            input[strcspn(input, "\n")] = 0;

            if (strlen(input) > 0)
            {
                process_command(input);
            }
        }

        if (game_player.alert_level >= 100)
        {
            print_colored_text("\n=== GAME OVER ===\n", COLOR_RED);
            printf("Vous avez été détecté par les systèmes de sécurité corporate !\n");
            game_player.game_over = true;
        }
    }
}

void display_help(void)
{
    print_colored_text("\n=== COMMANDES DISPONIBLES ===\n", COLOR_BRIGHT_CYAN);

    printf("\n%sCommandes de hacking :%s\n", COLOR_YELLOW, COLOR_RESET);
    if (game_player.commands_unlocked[CMD_SCAN])
    {
        printf("  scan          - Scanner le réseau pour trouver des cibles\n");
    }
    if (game_player.commands_unlocked[CMD_BRUTEFORCE])
    {
        printf("  bruteforce    - Attaque par force brute sur un système\n");
    }
    if (game_player.commands_unlocked[CMD_DECRYPT])
    {
        printf("  decrypt       - Décrypter des messages codés\n");
    }
    if (game_player.commands_unlocked[CMD_EXPLOIT])
    {
        printf("  exploit       - Exploiter une vulnérabilité système\n");
    }
    if (game_player.commands_unlocked[CMD_BACKDOOR])
    {
        printf("  backdoor      - Installer un accès clandestin sur un système\n");
    }
    if (game_player.commands_unlocked[CMD_TRACE_ROUTE])
    {
        printf("  traceroute    - Suivre le chemin des paquets vers une cible\n");
    }
    if (game_player.commands_unlocked[CMD_UPLOAD_VIRUS])
    {
        printf("  upload_virus  - Télécharger un virus sur un système distant\n");
    }
    if (game_player.commands_unlocked[CMD_AI_HACK])
    {
        printf("  ai_hack       - Lancer une attaque avancée avec l'IA\n");
    }
    if (game_player.commands_unlocked[CMD_QUANTUM_DECRYPT])
    {
        printf("  quantum_decrypt- Décrypter des données avec un décryptage quantique\n");
    }

    printf("\n%sCommandes spéciales :%s\n", COLOR_MAGENTA, COLOR_RESET);
    printf("  shop        - Accéder au marché noir cyberpunk\n");
    printf("  laylow      - Réduire votre niveau d'alerte\n");
    printf("  quests      - Afficher le journal de quêtes\n");
    printf("  contacts    - Gérer vos contacts dans l'ombre\n");
    printf("  contact <n> - Parler directement à un contact (numéro ou nom)\n");
    printf("  messages    - Consulter vos messages cryptés\n");
    printf("  read        - Lire un message spécifique\n");

    printf("\n%sCommandes de hacking avancé :%s\n", COLOR_BRIGHT_CYAN, COLOR_RESET);
    if (game_player.level >= 3)
    {
        printf("  advhack     - Lancer un hack avancé avec méthodes spécialisées\n");
        printf("  aiassist    - Utiliser l'assistant IA pour un hack\n");
    }
    if (game_player.level >= 2)
    {
        printf("  socialeng   - Attaque par ingénierie sociale\n");
    }
    printf("  stealthmode - Basculer le mode furtif\n");
    printf("  analyzedefenses - Analyser les défenses d'une cible\n");
    if (game_player.level >= 4)
    {
        printf("  quantumdecrypt - Décryptage quantique avancé\n");
    }
    if (game_player.level >= 5)
    {
        printf("  neuralsync  - Synchroniser l'interface neurale\n");
    }
    if (game_player.level >= 6)
    {
        printf("  temporalhack - Hack temporel (expérimental)\n");
    }

    printf("\n%sCommandes système :%s\n", COLOR_YELLOW, COLOR_RESET);
    printf("  status      - Affiche votre statut actuel\n");
    printf("  help        - Affiche cette aide\n");
    printf("  quit        - Quitte le jeu\n");
    printf("  clear       - Efface l'écran\n");
}

void process_command(char *input)
{
    char command[MAX_INPUT_LENGTH];
    char argument[MAX_INPUT_LENGTH];

    if (!parse_command(input, command, argument))
    {
        printf("Commande non reconnue. Tapez 'help' pour l'aide.\n");
        return;
    }

    if (strcmp(command, "help") == 0)
    {
        display_help();
    }
    else if (strcmp(command, "status") == 0)
    {
        print_colored_text("\n=== STATUT DU HACKER ===\n", COLOR_BRIGHT_CYAN);
        printf("Nom: %s%s%s\n", COLOR_BRIGHT_GREEN, game_player.name, COLOR_RESET);
        printf("Niveau: %s%d%s\n", COLOR_YELLOW, game_player.level, COLOR_RESET);
        printf("Expérience: %s%d%s\n", COLOR_CYAN, game_player.experience, COLOR_RESET);
        printf("Crédits: %s%d%s\n", COLOR_BRIGHT_GREEN, game_player.credits, COLOR_RESET);
        printf("Réputation: %s%d%s\n", COLOR_MAGENTA, game_player.reputation, COLOR_RESET);
        printf("Furtivité: %s%d/10%s\n", COLOR_BLUE, game_player.stealth_rating, COLOR_RESET);

        printf("\n%s=== ÉQUIPEMENT ===%s\n", COLOR_YELLOW, COLOR_RESET);
        printf("IA Assistante: %s%s%s\n",
               game_player.has_ai_assistant ? COLOR_GREEN : COLOR_RED,
               game_player.has_ai_assistant ? "DISPONIBLE" : "NON DISPONIBLE",
               COLOR_RESET);
        printf("Ordinateur Quantique: %s%s%s\n",
               game_player.has_quantum_computer ? COLOR_GREEN : COLOR_RED,
               game_player.has_quantum_computer ? "DISPONIBLE" : "NON DISPONIBLE",
               COLOR_RESET);
        printf("Bibliothèque Virus: %s%d virus%s\n", COLOR_CYAN, game_player.virus_library_size, COLOR_RESET);
        printf("Backdoors Actives: %s%d%s\n", COLOR_YELLOW, game_player.backdoors_active, COLOR_RESET);

        printf("\n%s=== SÉCURITÉ ===%s\n", COLOR_RED, COLOR_RESET);
        printf("Mode Furtif: %s%s%s\n",
               stealth_mode ? COLOR_GREEN : COLOR_YELLOW,
               stealth_mode ? "ACTIVÉ" : "DÉSACTIVÉ",
               COLOR_RESET);
        printf("Niveau d'alerte: ");
        if (game_player.alert_level < 30)
        {
            printf("%s%d/100%s [SÉCURISÉ]\n", COLOR_GREEN, game_player.alert_level, COLOR_RESET);
        }
        else if (game_player.alert_level < 70)
        {
            printf("%s%d/100%s [ATTENTION]\n", COLOR_YELLOW, game_player.alert_level, COLOR_RESET);
        }
        else
        {
            printf("%s%d/100%s [DANGER]\n", COLOR_RED, game_player.alert_level, COLOR_RESET);
        }
    }
    else if (strcmp(command, "quit") == 0 || strcmp(command, "exit") == 0)
    {
        printf("Au revoir, hacker...\n");
        game_running = false;
    }
    else if (strcmp(command, "clear") == 0)
    {
        printf("\033[2J\033[H");
        print_colored_text("╔══════════════════════════════════════════════════════════════════╗\n", COLOR_CYAN);
        print_colored_text("║                           NEON HACK                             ║\n", COLOR_BRIGHT_CYAN);
        print_colored_text("╚══════════════════════════════════════════════════════════════════╝\n", COLOR_CYAN);
    }
    else if (strcmp(command, "scan") == 0)
    {
        if (game_player.commands_unlocked[CMD_SCAN])
        {
            cmd_scan_network();
        }
        else
        {
            printf("Commande non disponible à votre niveau.\n");
        }
    }
    else if (strcmp(command, "bruteforce") == 0)
    {
        if (game_player.commands_unlocked[CMD_BRUTEFORCE])
        {
            cmd_bruteforce(argument);
        }
        else
        {
            printf("Commande non disponible à votre niveau.\n");
        }
    }
    else if (strcmp(command, "decrypt") == 0)
    {
        if (game_player.commands_unlocked[CMD_DECRYPT])
        {
            cmd_decrypt(argument);
        }
        else
        {
            printf("Commande non disponible à votre niveau.\n");
        }
    }
    else if (strcmp(command, "backdoor") == 0)
    {
        if (game_player.commands_unlocked[CMD_BACKDOOR])
        {
            cmd_backdoor(argument);
        }
        else
        {
            printf("Commande non disponible à votre niveau.\n");
        }
    }
    else if (strcmp(command, "traceroute") == 0)
    {
        if (game_player.commands_unlocked[CMD_TRACE_ROUTE])
        {
            cmd_trace_route(argument);
        }
        else
        {
            printf("Commande non disponible à votre niveau.\n");
        }
    }
    else if (strcmp(command, "uploadvirus") == 0)
    {
        if (game_player.commands_unlocked[CMD_UPLOAD_VIRUS])
        {
            cmd_upload_virus(argument);
        }
        else
        {
            printf("Commande non disponible à votre niveau.\n");
        }
    }
    else if (strcmp(command, "stealth") == 0)
    {
        cmd_stealth_mode();
    }
    else if (strcmp(command, "aihack") == 0)
    {
        if (game_player.commands_unlocked[CMD_AI_HACK])
        {
            cmd_ai_hack(argument);
        }
        else
        {
            printf("Commande non disponible à votre niveau.\n");
        }
    }
    else if (strcmp(command, "quantumdecrypt") == 0)
    {
        if (game_player.commands_unlocked[CMD_QUANTUM_DECRYPT])
        {
            cmd_quantum_decrypt(argument);
        }
        else
        {
            printf("Commande non disponible à votre niveau.\n");
        }
    }
    else if (strcmp(command, "shop") == 0)
    {
        cmd_shop();
    }
    else if (strcmp(command, "laylow") == 0)
    {
        cmd_lay_low();
    }
    else if (strcmp(command, "quests") == 0)
    {
        cmd_quests();
    }
    else if (strcmp(command, "contacts") == 0)
    {
        cmd_contacts();
    }
    else if (strcmp(command, "contact") == 0)
    {
        cmd_interact_contact(argument);
    }
    else if (strcmp(command, "messages") == 0)
    {
        cmd_messages();
    }
    else if (strcmp(command, "read") == 0)
    {
        cmd_read(argument);
    }
    // Nouvelles commandes de hacking avancé
    else if (strcmp(command, "advhack") == 0)
    {
        if (game_player.level >= 3)
        {
            cmd_advanced_hack(argument);
        }
        else
        {
            printf("Commande de hacking avancé non disponible. Niveau minimum: 3\n");
        }
    }
    else if (strcmp(command, "stealthmode") == 0)
    {
        cmd_stealth_mode_toggle();
    }
    else if (strcmp(command, "quantumdecrypt") == 0)
    {
        if (game_player.level >= 4)
        {
            cmd_quantum_decrypt_advanced(argument);
        }
        else
        {
            printf("Décryptage quantique non disponible. Niveau minimum: 4\n");
        }
    }
    else if (strcmp(command, "aiassist") == 0)
    {
        if (game_player.level >= 3)
        {
            cmd_ai_assist_hack(argument);
        }
        else
        {
            printf("Assistant IA non disponible. Niveau minimum: 3\n");
        }
    }
    else if (strcmp(command, "neuralsync") == 0)
    {
        if (game_player.level >= 5)
        {
            cmd_neural_sync();
        }
        else
        {
            printf("Interface neurale non disponible. Niveau minimum: 5\n");
        }
    }
    else if (strcmp(command, "analyzedefenses") == 0)
    {
        cmd_analyze_defenses(argument);
    }
    else if (strcmp(command, "socialeng") == 0)
    {
        if (game_player.level >= 2)
        {
            cmd_social_engineer(argument);
        }
        else
        {
            printf("Ingénierie sociale non disponible. Niveau minimum: 2\n");
        }
    }
    else if (strcmp(command, "temporalhack") == 0)
    {
        if (game_player.level >= 6)
        {
            cmd_temporal_hack(argument);
        }
        else
        {
            printf("Hack temporel non disponible. Niveau minimum: 6\n");
        }
    }
    else
    {
        printf("Commande inconnue: %s\n", command);
        printf("Tapez 'help' pour voir les commandes disponibles.\n");
    }
}

bool cmd_scan_network(void)
{
    print_colored_text("\n=== SCAN RÉSEAU ===\n", COLOR_BRIGHT_CYAN);
    printf("Recherche de systèmes connectés...\n");

    for (int i = 0; i < 3; i++)
    {
        printf(".");
        fflush(stdout);
        usleep(500000);
    }
    printf("\n\nSystèmes détectés:\n");

    int max_nodes = (game_player.level >= LEVEL_HACKER) ? 3 : (game_player.level >= LEVEL_APPRENTICE) ? 2
                                                                                                      : 1;

    for (int i = 0; i < max_nodes; i++)
    {
        printf("  %s[%d]%s %s", COLOR_YELLOW, i + 1, COLOR_RESET, network_nodes[i].name);

        if (network_nodes[i].is_compromised)
        {
            printf(" %s[COMPROMIS]%s", COLOR_GREEN, COLOR_RESET);
        }

        switch (network_nodes[i].security)
        {
        case SECURITY_LOW:
            printf(" %s[SÉCURITÉ: FAIBLE]%s", COLOR_GREEN, COLOR_RESET);
            break;
        case SECURITY_MEDIUM:
            printf(" %s[SÉCURITÉ: MOYENNE]%s", COLOR_YELLOW, COLOR_RESET);
            break;
        case SECURITY_HIGH:
            printf(" %s[SÉCURITÉ: ÉLEVÉE]%s", COLOR_RED, COLOR_RESET);
            break;
        case SECURITY_CRITICAL:
            printf(" %s[SÉCURITÉ: CRITIQUE]%s", COLOR_RED, COLOR_RESET);
            break;
        }
        printf("\n");
    }

    discovered_nodes = max_nodes;
    gain_experience(5);
    increase_alert_level(1);

    return true;
}

bool cmd_bruteforce(char *target)
{
    if (strlen(target) == 0)
    {
        printf("Usage: bruteforce <target>\n");
        printf("Exemple: bruteforce localhost\n");
        return false;
    }

    int target_index = -1;
    for (int i = 0; i < discovered_nodes; i++)
    {
        if (strcmp(network_nodes[i].name, target) == 0)
        {
            target_index = i;
            break;
        }
    }

    if (target_index == -1)
    {
        printf("Cible non trouvée. Utilisez 'scan' d'abord.\n");
        return false;
    }

    NetworkNode *node = &network_nodes[target_index];

    if (node->is_compromised)
    {
        printf("Système déjà compromis.\n");
        return false;
    }

    print_colored_text("\n=== ATTAQUE BRUTE FORCE ===\n", COLOR_RED);
    printf("Cible: %s\n", target);
    printf("Tentative de craquage du mot de passe...\n");

    char *passwords[] = {"admin", "123456", "password", "root", "guest"};

    for (int i = 0; i < 5; i++)
    {
        printf("Essai: %s", passwords[i]);
        for (int j = 0; j < 3; j++)
        {
            printf(".");
            fflush(stdout);
            usleep(300000);
        }

        int success_chance = 80 - (node->security * 15);
        if (rand() % 100 < success_chance)
        {
            printf(" %sSUCCÈS !%s\n", COLOR_GREEN, COLOR_RESET);
            node->is_compromised = true;

            printf("\nAccès obtenu à %s !\n", target);
            printf("Données récupérées: %d credits\n", node->data_value);

            gain_experience(node->data_value / 2);
            increase_alert_level(node->security * 5);

            // Débloquer bruteforce après le premier hack réussi
            if (!game_player.commands_unlocked[CMD_BRUTEFORCE] && target_index == 0)
            {
                game_player.commands_unlocked[CMD_BRUTEFORCE] = true;
                game_player.commands_unlocked[CMD_DECRYPT] = true;
                print_colored_text("\n*** NOUVELLES COMMANDES DÉBLOQUÉES ***\n", COLOR_BRIGHT_GREEN);
                printf("Vous pouvez maintenant utiliser: bruteforce, decrypt\n");
            }

            return true;
        }
        else
        {
            printf(" %sÉCHEC%s\n", COLOR_RED, COLOR_RESET);
        }
    }

    printf("\nAttaque brute force échouée.\n");
    increase_alert_level(node->security * 8);
    return false;
}

bool cmd_decrypt(char *encrypted_data)
{
    if (strlen(encrypted_data) == 0)
    {
        printf("Usage: decrypt <message_crypté>\n");
        printf("Essayez: decrypt WKLV#IS#D#TEZT\n");
        return false;
    }

    print_colored_text("\n=== DÉCRYPTAGE ===\n", COLOR_MAGENTA);
    printf("Données cryptées: %s\n", encrypted_data);
    printf("Analyse en cours...\n");

    char decrypted[MAX_INPUT_LENGTH];
    for (int i = 0; encrypted_data[i] != '\0'; i++)
    {
        if (encrypted_data[i] >= 'A' && encrypted_data[i] <= 'Z')
        {
            decrypted[i] = ((encrypted_data[i] - 'A' - 3 + 26) % 26) + 'A';
        }
        else if (encrypted_data[i] >= 'a' && encrypted_data[i] <= 'z')
        {
            decrypted[i] = ((encrypted_data[i] - 'a' - 3 + 26) % 26) + 'a';
        }
        else if (encrypted_data[i] == '#')
        {
            decrypted[i] = ' '; // Remplacer # par espace
        }
        else
        {
            decrypted[i] = encrypted_data[i];
        }
    }
    decrypted[strlen(encrypted_data)] = '\0';

    printf("Message décrypté: %s%s%s\n", COLOR_GREEN, decrypted, COLOR_RESET);

    if (strstr(decrypted, "THIS IS A TEST") != NULL)
    {
        printf("\nMessage de test décodé avec succès !\n");
        gain_experience(20);
    }

    return true;
}

bool parse_command(char *input, char *command, char *argument)
{
    command[0] = '\0';
    argument[0] = '\0';

    char *space_pos = strchr(input, ' ');

    if (space_pos == NULL)
    {
        strcpy(command, input);
    }
    else
    {
        int cmd_length = space_pos - input;
        strncpy(command, input, cmd_length);
        command[cmd_length] = '\0';

        while (*space_pos == ' ')
            space_pos++;
        strcpy(argument, space_pos);
    }

    return strlen(command) > 0;
}

void gain_experience(int exp)
{
    game_player.experience += exp;
    printf("%s[+%d EXP]%s ", COLOR_GREEN, exp, COLOR_RESET);

    // Système de progression plus équilibré
    int required_exp = game_player.level * 25; // Réduit de 100 à 25
    if (game_player.experience >= required_exp)
    {
        game_player.level++;
        print_colored_text("\n*** NIVEAU SUPÉRIEUR ! ***\n", COLOR_BRIGHT_GREEN);
        printf("Vous êtes maintenant niveau %d !\n", game_player.level);

        // Déblocage des commandes par niveau
        if (game_player.level == LEVEL_APPRENTICE)
        {
            game_player.commands_unlocked[CMD_BRUTEFORCE] = true;
            printf("Nouvelle commande débloquée: %sbruteforce%s\n", COLOR_YELLOW, COLOR_RESET);
        }
        else if (game_player.level == LEVEL_HACKER)
        {
            game_player.commands_unlocked[CMD_DECRYPT] = true;
            game_player.commands_unlocked[CMD_BACKDOOR] = true;
            printf("Nouvelles commandes débloquées: %sdecrypt, backdoor%s\n", COLOR_YELLOW, COLOR_RESET);
            game_player.virus_library_size = 1; // Accès aux virus de base
        }
        else if (game_player.level == LEVEL_EXPERT)
        {
            game_player.commands_unlocked[CMD_EXPLOIT] = true;
            game_player.commands_unlocked[CMD_TRACE_ROUTE] = true;
            game_player.commands_unlocked[CMD_UPLOAD_VIRUS] = true;
            printf("Nouvelles commandes débloquées: %sexploit, traceroute, uploadvirus%s\n", COLOR_YELLOW, COLOR_RESET);
            game_player.virus_library_size = 2; // Plus de virus disponibles
            game_player.stealth_rating += 2;    // Amélioration furtivité
        }
        else if (game_player.level == LEVEL_MASTER)
        {
            game_player.commands_unlocked[CMD_AI_HACK] = true;
            game_player.commands_unlocked[CMD_QUANTUM_DECRYPT] = true;
            printf("Nouvelles commandes MAÎTRE débloquées: %saihack, quantumdecrypt%s\n", COLOR_BRIGHT_GREEN, COLOR_RESET);
            game_player.has_ai_assistant = true;
            game_player.has_quantum_computer = true;
            game_player.virus_library_size = 3; // Arsenal complet
            game_player.stealth_rating += 3;    // Furtivité maximale
            game_player.credits += 5000;        // Bonus crédits maître
            printf("FÉLICITATIONS ! Vous avez atteint le niveau MAÎTRE !\n");
            printf("Équipement débloqué: IA Assistante + Ordinateur Quantique\n");
        }
    }
}

void increase_alert_level(int amount)
{
    // Synchroniser avec le nouveau AlertSystem
    increase_alert(&alert_system, amount / 10, "Action du joueur");

    // Maintenir l'ancien système pour compatibilité
    game_player.alert_level += amount;
    if (game_player.alert_level > 100)
    {
        game_player.alert_level = 100;
    }

    // Synchroniser bidirectionnellement
    game_player.alert_level = alert_system.current_level * 10;
    if (game_player.alert_level > 100)
        game_player.alert_level = 100;

    if (amount > 0)
    {
        printf("%s[ALERTE +%d]%s ", COLOR_RED, amount, COLOR_RESET);

        if (game_player.alert_level >= 80)
        {
            printf("%s[DANGER CRITIQUE !]%s ", COLOR_RED, COLOR_RESET);
        }
        else if (game_player.alert_level >= 50)
        {
            printf("%s[NIVEAU D'ALERTE ÉLEVÉ]%s ", COLOR_YELLOW, COLOR_RESET);
        }
    }
}

// ================================
// NOUVELLES COMMANDES AVANCÉES
// ================================

void init_virus_library(void)
{
    // Initialisation de la bibliothèque de virus
    strcpy(available_viruses[0].name, "Trojan.Stealth");
    strcpy(available_viruses[0].description, "Virus furtif pour accès discret");
    available_viruses[0].damage = 25;
    available_viruses[0].stealth_rating = 8;
    available_viruses[0].is_detected = false;

    strcpy(available_viruses[1].name, "Worm.DataMiner");
    strcpy(available_viruses[1].description, "Ver extracteur de données");
    available_viruses[1].damage = 40;
    available_viruses[1].stealth_rating = 5;
    available_viruses[1].is_detected = false;

    strcpy(available_viruses[2].name, "Ransomware.CryptoLock");
    strcpy(available_viruses[2].description, "Chiffrement malveillant de systèmes");
    available_viruses[2].damage = 70;
    available_viruses[2].stealth_rating = 3;
    available_viruses[2].is_detected = false;

    virus_count = 3;
}

bool cmd_backdoor(char *target)
{
    if (strlen(target) == 0)
    {
        printf("Usage: backdoor <nom_système>\n");
        return false;
    }

    // Chercher le nœud cible
    int target_index = -1;
    for (int i = 0; i < discovered_nodes; i++)
    {
        if (strcmp(network_nodes[i].name, target) == 0)
        {
            target_index = i;
            break;
        }
    }

    if (target_index == -1)
    {
        printf("Système '%s' non trouvé. Utilisez 'scan' d'abord.\n", target);
        return false;
    }

    NetworkNode *node = &network_nodes[target_index];

    if (node->has_backdoor)
    {
        printf("Backdoor déjà installée sur %s\n", target);
        return false;
    }

    printf("Installation de backdoor sur %s...\n", target);
    print_typing_effect("Injection du payload...", 500);
    print_typing_effect("Création des hooks système...", 500);
    print_typing_effect("Masquage des traces...", 500);

    // Calcul de succès basé sur le niveau et la sécurité
    int success_chance = 70 + (game_player.level * 10) - (node->security * 15);
    if (stealth_mode)
        success_chance += 20;

    if (rand() % 100 < success_chance)
    {
        node->has_backdoor = true;
        game_player.backdoors_active++;
        print_colored_text("✓ Backdoor installée avec succès !\n", COLOR_GREEN);
        gain_experience(15);

        // Bonus de crédits pour backdoor réussie
        game_player.credits += 500;
        printf("[+500 crédits]\n");

        // Risque d'alerte réduit si en mode furtif
        increase_alert_level(stealth_mode ? 5 : 10);
        return true;
    }
    else
    {
        print_colored_text("✗ Échec de l'installation - Système protégé\n", COLOR_RED);
        increase_alert_level(20);
        return false;
    }
}

bool cmd_trace_route(char *target)
{
    if (strlen(target) == 0)
    {
        printf("Usage: traceroute <nom_système>\n");
        return false;
    }

    printf("Traçage de route vers %s...\n", target);
    print_typing_effect("Envoi des paquets ICMP...", 300);

    // Simulation de traceroute
    int hops = rand() % 8 + 3;
    for (int i = 1; i <= hops; i++)
    {
        printf("%d   192.168.%d.%d   %dms\n", i, rand() % 255, rand() % 255, rand() % 100 + 10);
        usleep(200000); // 200ms delay
    }

    // Chercher le nœud cible
    int target_index = -1;
    for (int i = 0; i < discovered_nodes; i++)
    {
        if (strcmp(network_nodes[i].name, target) == 0)
        {
            target_index = i;
            break;
        }
    }

    if (target_index != -1)
    {
        NetworkNode *node = &network_nodes[target_index];
        node->is_traced = true;

        printf("Route trouvée ! Informations système révélées :\n");
        printf("  Corporation: %s\n", node->corporation);
        printf("  Force firewall: %d/10\n", node->firewall_strength);
        printf("  Fichiers secrets: %d détectés\n", node->file_count);

        gain_experience(8);
        increase_alert_level(3);
        return true;
    }
    else
    {
        printf("Hôte inaccessible\n");
        return false;
    }
}

bool cmd_upload_virus(char *target)
{
    if (game_player.virus_library_size == 0)
    {
        printf("Aucun virus disponible. Développez d'abord votre arsenal.\n");
        return false;
    }

    if (strlen(target) == 0)
    {
        printf("Usage: uploadvirus <nom_système>\n");
        printf("Virus disponibles :\n");
        for (int i = 0; i < virus_count && i < game_player.virus_library_size; i++)
        {
            printf("  %s - %s (DMG:%d, STEALTH:%d)\n",
                   available_viruses[i].name,
                   available_viruses[i].description,
                   available_viruses[i].damage,
                   available_viruses[i].stealth_rating);
        }
        return false;
    }

    // Chercher le nœud cible
    int target_index = -1;
    for (int i = 0; i < discovered_nodes; i++)
    {
        if (strcmp(network_nodes[i].name, target) == 0)
        {
            target_index = i;
            break;
        }
    }

    if (target_index == -1)
    {
        printf("Système '%s' non trouvé.\n", target);
        return false;
    }

    NetworkNode *node = &network_nodes[target_index];

    if (node->has_virus)
    {
        printf("Ce système est déjà infecté.\n");
        return false;
    }

    // Choisir un virus aléatoire disponible
    int virus_choice = rand() % (game_player.virus_library_size < virus_count ? game_player.virus_library_size : virus_count);
    Virus *chosen_virus = &available_viruses[virus_choice];

    printf("Upload de %s vers %s...\n", chosen_virus->name, target);
    print_typing_effect("Contournement antivirus...", 400);
    print_typing_effect("Injection du code malveillant...", 400);
    print_typing_effect("Activation du payload...", 400);

    // Calcul de succès
    int success_chance = 60 + chosen_virus->stealth_rating - (node->security * 10);
    if (node->has_backdoor)
        success_chance += 30;

    if (rand() % 100 < success_chance)
    {
        node->has_virus = true;
        chosen_virus->is_detected = false;

        print_colored_text("✓ Virus uploadé avec succès !\n", COLOR_GREEN);
        printf("Dégâts infligés: %d\n", chosen_virus->damage);

        // Réduction de la sécurité du système
        if (node->firewall_strength > 0)
        {
            node->firewall_strength -= (chosen_virus->damage / 10);
            if (node->firewall_strength < 0)
                node->firewall_strength = 0;
        }

        gain_experience(20);
        game_player.credits += chosen_virus->damage * 10;
        printf("[+%d crédits]\n", chosen_virus->damage * 10);

        increase_alert_level(15 - chosen_virus->stealth_rating);
        return true;
    }
    else
    {
        print_colored_text("✗ Upload échoué - Antivirus détecté\n", COLOR_RED);
        chosen_virus->is_detected = true;
        increase_alert_level(25);
        return false;
    }
}

bool cmd_stealth_mode(void)
{
    if (game_player.stealth_rating < 5)
    {
        printf("Capacité de furtivité insuffisante (requis: 5, actuel: %d)\n", game_player.stealth_rating);
        return false;
    }

    stealth_mode = !stealth_mode;

    if (stealth_mode)
    {
        print_colored_text(">>> MODE FURTIF ACTIVÉ <<<\n", COLOR_BLUE);
        printf("Toutes les opérations génèrent moins d'alertes.\n");
        printf("Coût: -1 point de furtivité par minute\n");
    }
    else
    {
        print_colored_text(">>> MODE FURTIF DÉSACTIVÉ <<<\n", COLOR_YELLOW);
    }

    return true;
}

bool cmd_ai_hack(char *target)
{
    if (!game_player.has_ai_assistant)
    {
        printf("IA assistante non disponible. Améliorer d'abord votre équipement.\n");
        return false;
    }

    if (strlen(target) == 0)
    {
        printf("Usage: aihack <nom_système>\n");
        return false;
    }

    // Chercher le nœud cible
    int target_index = -1;
    for (int i = 0; i < discovered_nodes; i++)
    {
        if (strcmp(network_nodes[i].name, target) == 0)
        {
            target_index = i;
            break;
        }
    }

    if (target_index == -1)
    {
        printf("Système '%s' non trouvé.\n", target);
        return false;
    }

    NetworkNode *node = &network_nodes[target_index];

    printf("Lancement de l'attaque IA sur %s...\n", target);
    print_colored_text(">>> IA NOVA EN LIGNE <<<\n", COLOR_MAGENTA);
    print_typing_effect("Analyse des patterns de sécurité...", 300);
    print_typing_effect("Génération d'exploits adaptatifs...", 300);
    print_typing_effect("Exécution de l'attaque neuromorphe...", 300);

    // L'IA a un taux de succès très élevé
    int success_chance = 85 + (game_player.level * 5);
    if (node->security == SECURITY_CRITICAL)
        success_chance -= 20;

    if (rand() % 100 < success_chance)
    {
        node->is_compromised = true;
        print_colored_text("✓ SYSTÈME COMPROMIS PAR L'IA !\n", COLOR_BRIGHT_GREEN);

        // L'IA révèle tous les fichiers secrets
        for (int i = 0; i < node->file_count; i++)
        {
            node->secret_files[i].is_unlocked = true;
            printf("Fichier déchiffré: %s\n", node->secret_files[i].filename);
            game_player.credits += node->secret_files[i].credits_value;
        }

        gain_experience(35);
        increase_alert_level(5); // L'IA est très discrète
        return true;
    }
    else
    {
        print_colored_text("✗ IA repoussée par des contre-mesures adaptatives\n", COLOR_RED);
        increase_alert_level(30);
        return false;
    }
}

bool cmd_quantum_decrypt(char *data)
{
    if (!game_player.has_quantum_computer)
    {
        printf("Ordinateur quantique non disponible.\n");
        printf("Requis pour décrypter les chiffrements de niveau quantique.\n");
        return false;
    }

    if (strlen(data) == 0)
    {
        printf("Usage: quantumdecrypt <données_chiffrées>\n");
        return false;
    }

    printf("Initialisation du processeur quantique...\n");
    print_colored_text(">>> QUANTUM CORE ONLINE <<<\n", COLOR_BRIGHT_CYAN);
    print_typing_effect("Superposition des qubits...", 400);
    print_typing_effect("Algorithme de Shor en cours...", 400);
    print_typing_effect("Factorisation quantique...", 400);
    print_typing_effect("Effondrement de la fonction d'onde...", 400);

    // Le décryptage quantique ne peut échouer
    print_colored_text("✓ DÉCRYPTAGE QUANTIQUE RÉUSSI !\n", COLOR_BRIGHT_GREEN);

    // Messages spéciaux déchiffrés par quantum
    if (strstr(data, "CLASSIFIED") != NULL)
    {
        printf("\n");
        print_colored_text("╔══════════════════════════════════════════════════════════════════╗\n", COLOR_RED);
        print_colored_text("║                    DOCUMENT ULTRA-SECRET                        ║\n", COLOR_RED);
        print_colored_text("║                                                                  ║\n", COLOR_RED);
        print_colored_text("║  PROJET GHOST PROTOCOL - PHASE 3                                ║\n", COLOR_RED);
        print_colored_text("║  Neo-Tokyo sera sous contrôle total d'ici 2088                  ║\n", COLOR_RED);
        print_colored_text("║  Coordinateur: Agent Smith                                       ║\n", COLOR_RED);
        print_colored_text("║  Budget: 50 000 000 crédits                                     ║\n", COLOR_RED);
        print_colored_text("╚══════════════════════════════════════════════════════════════════╝\n", COLOR_RED);

        game_player.credits += 10000;
        printf("[+10000 crédits bonus !]\n");
    }
    else
    {
        // Décryptage standard amélioré
        printf("Message déchiffré: ");
        for (int i = 0; data[i]; i++)
        {
            if (data[i] >= 'A' && data[i] <= 'Z')
            {
                printf("%c", ((data[i] - 'A' - 3 + 26) % 26) + 'A');
            }
            else if (data[i] >= 'a' && data[i] <= 'z')
            {
                printf("%c", ((data[i] - 'a' - 3 + 26) % 26) + 'a');
            }
            else if (data[i] == '#')
            {
                printf(" ");
            }
            else
            {
                printf("%c", data[i]);
            }
        }
        printf("\n");
    }

    gain_experience(50);
    game_player.credits += 2000;
    printf("[+2000 crédits]\n");

    // Le quantique génère moins d'alertes mais consomme beaucoup d'énergie
    increase_alert_level(2);

    return true;
}

// ================================
// NOUVELLES COMMANDES INTÉGRÉES
// ================================

bool cmd_shop(void)
{
    // Vérifier si la boutique est accessible
    if (!is_shop_available(game_player.level, alert_system.current_level))
    {
        return false;
    }

    display_shop_welcome();
    display_shop(&global_shop, game_player.credits, game_player.level);

    char input[10];
    printf("\nEntrez le numéro de l'objet à acheter (0 pour quitter): ");
    fgets(input, sizeof(input), stdin);
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
        if (buy_item(&global_shop, item_type, &game_player.credits, game_player.level))
        {
            // Utiliser l'objet acheté immédiatement si applicable
            use_item(item_type, &game_player);
        }
    }
    else
    {
        printf("Numéro d'objet invalide.\n");
    }

    return true;
}

bool cmd_lay_low(void)
{
    display_alert_status(&alert_system);
    display_alert_reduction_menu(&alert_system);

    char input[10];
    printf("\nChoisissez une méthode (0 pour annuler): ");
    fgets(input, sizeof(input), stdin);
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

    if (attempt_alert_reduction(&alert_system, method, &game_player.credits))
    {
        // Synchroniser le niveau d'alerte du joueur avec le système
        game_player.alert_level = alert_system.current_level * 10;
        if (game_player.alert_level > 100)
            game_player.alert_level = 100;

        printf("\nNiveau d'alerte synchronisé: %d/100\n", game_player.alert_level);
    }

    return true;
}

bool cmd_quests(void)
{
    display_quest_log(&quest_system);

    printf("\nVoulez-vous voir les détails d'une quête ? (tapez le numéro ou 0 pour sortir): ");
    char input[10];
    fgets(input, sizeof(input), stdin);
    input[strcspn(input, "\n")] = 0;

    int quest_id = atoi(input);
    if (quest_id > 0 && quest_id <= quest_system.active_quest_count)
    {
        display_quest_details(&quest_system.quests[quest_id - 1]);
    }

    return true;
}

bool cmd_contacts(void)
{
    display_contacts(&contact_system);

    printf("\nVoulez-vous parler à un contact ? (tapez le numéro ou 0 pour sortir): ");
    char input[10];
    fgets(input, sizeof(input), stdin);
    input[strcspn(input, "\n")] = 0;

    int contact_id = atoi(input);
    if (contact_id > 0 && contact_id <= contact_system.active_contacts)
    {
        ContactType id = (ContactType)(contact_id - 1);
        contact_npc(&contact_system, id, &game_player);
    }

    return true;
}

bool cmd_messages(void)
{
    display_inbox(&contact_system);
    return true;
}

bool cmd_read(char *argument)
{
    if (strlen(argument) == 0)
    {
        printf("Usage: read <numéro_message>\n");
        printf("Exemple: read 1\n");
        return false;
    }

    int message_id = atoi(argument);
    if (message_id <= 0 || message_id > contact_system.inbox_count)
    {
        printf("Numéro de message invalide.\n");
        return false;
    }

    // Marquer le message comme lu et l'afficher
    Message *msg = &contact_system.inbox[message_id - 1];
    msg->is_read = true;

    printf("\n" COLOR_CYAN "═══════════════════════════════════════════════════════════════════════════\n");
    printf("De: " COLOR_WHITE "%s" COLOR_RESET "\n", msg->from);
    printf("Sujet: " COLOR_YELLOW "%s" COLOR_RESET "\n", msg->subject);
    printf("═══════════════════════════════════════════════════════════════════════════\n" COLOR_RESET);
    printf("\n%s\n", msg->content);
    printf("\n" COLOR_CYAN "═══════════════════════════════════════════════════════════════════════════\n" COLOR_RESET);

    return true;
}

bool cmd_interact_contact(char *argument)
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
            if (contact_system.contacts[i].is_unlocked)
            {
                if (current_number == contact_number)
                {
                    ContactType id = (ContactType)i;
                    contact_npc(&contact_system, id, &game_player);
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
            if (contact_system.contacts[i].is_unlocked)
            {
                if (strcmp(contact_system.contacts[i].name, argument) == 0)
                {
                    ContactType id = (ContactType)i;
                    contact_npc(&contact_system, id, &game_player);
                    return true;
                }
            }
        }

        printf("Contact '%s' introuvable. Tapez 'contacts' pour voir la liste.\n", argument);
        return false;
    }
}

// ===== IMPLÉMENTATIONS DES COMMANDES DE HACKING AVANCÉ =====

bool cmd_advanced_hack(char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("Usage: advhack <nom_cible>\n");
        printf("Cibles disponibles: nexus-mainframe, banking-network, research-lab, gov-database, underground-market\n");
        return false;
    }

    // Trouver la cible dans le système avancé
    int target_id = -1;
    for (int i = 0; i < global_advanced_system.target_count; i++)
    {
        if (strcmp(global_advanced_system.targets[i].name, target_name) == 0)
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

    display_hacking_menu(&global_advanced_system);

    printf("\nChoisissez une méthode de hack (1-8): ");
    char input[10];
    fgets(input, sizeof(input), stdin);
    int method_choice = atoi(input) - 1;

    if (method_choice < 0 || method_choice >= 8)
    {
        printf("Méthode invalide.\n");
        return false;
    }

    HackType hack_type = (HackType)method_choice;
    return attempt_advanced_hack(&global_advanced_system, target_id, hack_type, &game_player);
}

bool cmd_stealth_mode_toggle(void)
{
    update_stealth_system(&global_advanced_system.stealth);

    if (global_advanced_system.stealth.is_active)
    {
        printf("\n" COLOR_GREEN "🔮 MODE FURTIF ACTIVÉ" COLOR_RESET "\n");
        printf("Détection réduite de %d%%\n", global_advanced_system.stealth.detection_reduction);
    }
    else
    {
        printf("\n" COLOR_YELLOW "👁️ MODE FURTIF DÉSACTIVÉ" COLOR_RESET "\n");
        printf("Visibilité normale rétablie.\n");
    }

    return true;
}

bool cmd_quantum_decrypt_advanced(char *encrypted_data)
{
    if (strlen(encrypted_data) == 0)
    {
        printf("Usage: quantumdecrypt <données_cryptées>\n");
        printf("Exemple: quantumdecrypt 'Q#X7#NEXUS#SECRET#DATA'\n");
        return false;
    }

    return activate_quantum_hack(&global_advanced_system.quantum, encrypted_data);
}

bool cmd_ai_assist_hack(char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("Usage: aiassist <nom_cible>\n");
        return false;
    }

    AIAssistant *ai = &global_advanced_system.ai_assistant;

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

bool cmd_neural_sync(void)
{
    printf("\n" COLOR_MAGENTA "🧠 SYNCHRONISATION INTERFACE NEURALE" COLOR_RESET "\n");
    printf("Établissement de la connexion synaptique...\n");

    for (int i = 0; i < 3; i++)
    {
        printf("⚡");
        fflush(stdout);
        usleep(800000);
    }

    global_advanced_system.neural_interface_sync += 10;
    if (global_advanced_system.neural_interface_sync > 100)
        global_advanced_system.neural_interface_sync = 100;

    printf("\n✅ Synchronisation complétée: %d%%\n", global_advanced_system.neural_interface_sync);

    if (global_advanced_system.neural_interface_sync >= 90)
    {
        printf("🎉 " COLOR_BRIGHT_GREEN "SYNCHRONISATION PARFAITE ATTEINTE!" COLOR_RESET "\n");
        printf("Nouvelles capacités débloquées: Hack temporel\n");
    }

    // Améliorer l'IA en même temps
    train_ai_assistant(&global_advanced_system.ai_assistant, 20);

    return true;
}

bool cmd_analyze_defenses(char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("\n" COLOR_YELLOW "🎯 CIBLES DISPONIBLES:" COLOR_RESET "\n");
        display_advanced_targets(&global_advanced_system);
        return true;
    }

    // Trouver et analyser la cible spécifique
    for (int i = 0; i < global_advanced_system.target_count; i++)
    {
        if (strcmp(global_advanced_system.targets[i].name, target_name) == 0)
        {
            display_defense_analysis(&global_advanced_system.targets[i]);
            return true;
        }
    }

    printf("Cible '%s' introuvable.\n", target_name);
    return false;
}

bool cmd_social_engineer(char *target_name)
{
    if (strlen(target_name) == 0)
    {
        printf("Usage: socialeng <nom_cible>\n");
        return false;
    }

    // Trouver la cible
    int target_id = -1;
    for (int i = 0; i < global_advanced_system.target_count; i++)
    {
        if (strcmp(global_advanced_system.targets[i].name, target_name) == 0)
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

    return social_engineering_attack(&global_advanced_system, target_id, &game_player);
}

bool cmd_temporal_hack(char *target_name)
{
    if (global_advanced_system.neural_interface_sync < 90)
    {
        printf("⚠️ Synchronisation neurale insuffisante pour le hack temporel.\n");
        printf("Synchronisation actuelle: %d%% (minimum requis: 90%%)\n",
               global_advanced_system.neural_interface_sync);
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
    for (int i = 0; i < global_advanced_system.target_count; i++)
    {
        if (strcmp(global_advanced_system.targets[i].name, target_name) == 0)
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

    return temporal_hack_attempt(&global_advanced_system, target_id, &game_player);
}

int main(void)
{
    printf("\n");
    print_cyberpunk_art();
    printf("\n");

    init_game();
    game_loop();

    printf("\nMerci d'avoir joué à Neon Hack !\n");
    printf("Gardez vos secrets... dans l'ombre.\n");

    return 0;
}
