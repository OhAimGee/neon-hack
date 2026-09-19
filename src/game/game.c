/*
 * État de partie, introduction, boucle de jeu et progression.
 *
 * init_game / display_intro / gain_experience / increase_alert_level viennent
 * du code d'origine (neon_hack.c v2.087), adaptés à GameState. La boucle de
 * jeu et le dispatch des commandes sont neufs (voir commands.c).
 */
#include "game.h"

#include "../core/io.h"
#include "../core/platform.h"
#include "../i18n/i18n.h"
#include "../ui/term.h"
#include "commands.h"
#include "legacy_colors.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static void init_virus_library(GameState *gs);

void print_colored_text(const char *text, const char *color)
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

void print_typing_effect(const char *text, int delay_ms)
{
    for (int i = 0; text[i] != '\0'; i++)
    {
        printf("%c", text[i]);
        fflush(stdout);
        nh_sleep_ms((unsigned)delay_ms);
    }
}

void init_game(GameState *gs)
{
    memset(gs, 0, sizeof(*gs));
    gs->running = true;

    strcpy(gs->player.name, "Anonymous");
    gs->player.level = LEVEL_NOVICE;
    gs->player.experience = 0;
    gs->player.alert_level = 0;
    gs->player.game_over = false;

    // Nouvelles propriétés du joueur
    gs->player.credits = 100;      // Crédits de départ
    gs->player.reputation = 0;     // Réputation neutre
    gs->player.stealth_rating = 3; // Capacité furtivité de base
    gs->player.has_quantum_computer = false;
    gs->player.has_ai_assistant = false;
    gs->player.virus_library_size = 0;
    gs->player.backdoors_active = 0;
    gs->player.last_hack_time = time(NULL);

    for (int i = 0; i < MAX_COMMANDS; i++)
    {
        gs->player.commands_unlocked[i] = false;
    }
    gs->player.commands_unlocked[CMD_SCAN] = true;

    // Initialiser les variables globales
    gs->stealth_mode = false;

    // Initialiser les nouveaux modules
    init_shop(&gs->shop);
    init_alert_system(&gs->alert);
    init_quest_system(&gs->quests);
    init_contact_system(&gs->contacts);
    init_advanced_hacking_system(&gs->advanced);


    // Initialiser le réseau avec nouvelles propriétés
    strcpy(gs->nodes[0].name, "localhost");
    gs->nodes[0].security = SECURITY_LOW;
    gs->nodes[0].is_compromised = false;
    gs->nodes[0].has_backdoor = false;
    gs->nodes[0].has_virus = false;
    gs->nodes[0].is_traced = false;
    gs->nodes[0].data_value = 10;
    gs->nodes[0].firewall_strength = 2;
    strcpy(gs->nodes[0].corporation, "Independent");
    gs->nodes[0].file_count = 1;
    strcpy(gs->nodes[0].secret_files[0].filename, "user_data.txt");
    strcpy(gs->nodes[0].secret_files[0].content, "Données utilisateur locales");
    gs->nodes[0].secret_files[0].encryption_level = 1;
    gs->nodes[0].secret_files[0].is_unlocked = false;
    gs->nodes[0].secret_files[0].credits_value = 50;

    strcpy(gs->nodes[1].name, "corp-server-01");
    gs->nodes[1].security = SECURITY_MEDIUM;
    gs->nodes[1].is_compromised = false;
    gs->nodes[1].has_backdoor = false;
    gs->nodes[1].has_virus = false;
    gs->nodes[1].is_traced = false;
    gs->nodes[1].data_value = 50;
    gs->nodes[1].firewall_strength = 5;
    strcpy(gs->nodes[1].corporation, "MegaCorp Industries");
    gs->nodes[1].file_count = 2;
    strcpy(gs->nodes[1].secret_files[0].filename, "employee_records.db");
    strcpy(gs->nodes[1].secret_files[0].content, "Registres des employés");
    gs->nodes[1].secret_files[0].encryption_level = 2;
    gs->nodes[1].secret_files[0].is_unlocked = false;
    gs->nodes[1].secret_files[0].credits_value = 200;
    strcpy(gs->nodes[1].secret_files[1].filename, "financial_data.xlsx");
    strcpy(gs->nodes[1].secret_files[1].content, "Données financières confidentielles");
    gs->nodes[1].secret_files[1].encryption_level = 3;
    gs->nodes[1].secret_files[1].is_unlocked = false;
    gs->nodes[1].secret_files[1].credits_value = 500;

    strcpy(gs->nodes[2].name, "nexus-mainframe");
    gs->nodes[2].security = SECURITY_HIGH;
    gs->nodes[2].is_compromised = false;
    gs->nodes[2].has_backdoor = false;
    gs->nodes[2].has_virus = false;
    gs->nodes[2].is_traced = false;
    gs->nodes[2].data_value = 200;
    gs->nodes[2].firewall_strength = 8;
    strcpy(gs->nodes[2].corporation, "Nexus Corp");
    gs->nodes[2].file_count = 3;
    strcpy(gs->nodes[2].secret_files[0].filename, "project_ghost.dat");
    strcpy(gs->nodes[2].secret_files[0].content, "CLASSIFIED");
    gs->nodes[2].secret_files[0].encryption_level = 4;
    gs->nodes[2].secret_files[0].is_unlocked = false;
    gs->nodes[2].secret_files[0].credits_value = 1000;
    strcpy(gs->nodes[2].secret_files[1].filename, "neural_maps.bin");
    strcpy(gs->nodes[2].secret_files[1].content, "Cartes neurales des citoyens");
    gs->nodes[2].secret_files[1].encryption_level = 5;
    gs->nodes[2].secret_files[1].is_unlocked = false;
    gs->nodes[2].secret_files[1].credits_value = 1500;
    strcpy(gs->nodes[2].secret_files[2].filename, "quantum_keys.qkey");
    strcpy(gs->nodes[2].secret_files[2].content, "Clés de chiffrement quantique");
    gs->nodes[2].secret_files[2].encryption_level = 6;
    gs->nodes[2].secret_files[2].is_unlocked = false;
    gs->nodes[2].secret_files[2].credits_value = 2000;

    gs->discovered_nodes = 3;

    // Initialiser la bibliothèque de virus
    init_virus_library(gs);

}

void display_intro(GameState *gs)
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
    nh_read_line(gs->player.name, MAX_NAME_LENGTH);
    if (gs->player.name[0] == '\0')
    {
        strcpy(gs->player.name, "Anonymous");
    }

    printf("\nBienvenue dans l'ombre, %s%s%s.\n", COLOR_BRIGHT_CYAN, gs->player.name, COLOR_RESET);
    printf("\n");
    print_colored_text("Tapez 'help' pour voir les commandes disponibles.\n", COLOR_CYAN);
    printf("\n");
}

void gain_experience(GameState *gs, int exp)
{
    gs->player.experience += exp;
    printf("%s[+%d EXP]%s ", COLOR_GREEN, exp, COLOR_RESET);

    // Système de progression plus équilibré
    int required_exp = gs->player.level * 25; // Réduit de 100 à 25
    if (gs->player.experience >= required_exp)
    {
        gs->player.level++;
        print_colored_text("\n*** NIVEAU SUPÉRIEUR ! ***\n", COLOR_BRIGHT_GREEN);
        printf("Vous êtes maintenant niveau %d !\n", gs->player.level);

        // Déblocage des commandes par niveau
        if (gs->player.level == LEVEL_APPRENTICE)
        {
            gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;
            printf("Nouvelle commande débloquée: %sbruteforce%s\n", COLOR_YELLOW, COLOR_RESET);
        }
        else if (gs->player.level == LEVEL_HACKER)
        {
            gs->player.commands_unlocked[CMD_DECRYPT] = true;
            gs->player.commands_unlocked[CMD_BACKDOOR] = true;
            printf("Nouvelles commandes débloquées: %sdecrypt, backdoor%s\n", COLOR_YELLOW, COLOR_RESET);
            gs->player.virus_library_size = 1; // Accès aux virus de base
        }
        else if (gs->player.level == LEVEL_EXPERT)
        {
            gs->player.commands_unlocked[CMD_EXPLOIT] = true;
            gs->player.commands_unlocked[CMD_TRACE_ROUTE] = true;
            gs->player.commands_unlocked[CMD_UPLOAD_VIRUS] = true;
            printf("Nouvelles commandes débloquées: %sexploit, traceroute, uploadvirus%s\n", COLOR_YELLOW, COLOR_RESET);
            gs->player.virus_library_size = 2; // Plus de virus disponibles
            gs->player.stealth_rating += 2;    // Amélioration furtivité
        }
        else if (gs->player.level == LEVEL_MASTER)
        {
            gs->player.commands_unlocked[CMD_AI_HACK] = true;
            gs->player.commands_unlocked[CMD_QUANTUM_DECRYPT] = true;
            printf("Nouvelles commandes MAÎTRE débloquées: %saihack, quantumdecrypt%s\n", COLOR_BRIGHT_GREEN, COLOR_RESET);
            gs->player.has_ai_assistant = true;
            gs->player.has_quantum_computer = true;
            gs->player.virus_library_size = 3; // Arsenal complet
            gs->player.stealth_rating += 3;    // Furtivité maximale
            gs->player.credits += 5000;        // Bonus crédits maître
            printf("FÉLICITATIONS ! Vous avez atteint le niveau MAÎTRE !\n");
            printf("Équipement débloqué: IA Assistante + Ordinateur Quantique\n");
        }
    }
}

void increase_alert_level(GameState *gs, int amount)
{
    // Synchroniser avec le nouveau AlertSystem
    increase_alert(&gs->alert, amount / 10, "Action du joueur");

    // Maintenir l'ancien système pour compatibilité
    gs->player.alert_level += amount;
    if (gs->player.alert_level > 100)
    {
        gs->player.alert_level = 100;
    }

    // Synchroniser bidirectionnellement
    gs->player.alert_level = gs->alert.current_level * 10;
    if (gs->player.alert_level > 100)
        gs->player.alert_level = 100;

    if (amount > 0)
    {
        printf("%s[ALERTE +%d]%s ", COLOR_RED, amount, COLOR_RESET);

        if (gs->player.alert_level >= 80)
        {
            printf("%s[DANGER CRITIQUE !]%s ", COLOR_RED, COLOR_RESET);
        }
        else if (gs->player.alert_level >= 50)
        {
            printf("%s[NIVEAU D'ALERTE ÉLEVÉ]%s ", COLOR_YELLOW, COLOR_RESET);
        }
    }
}

static void init_virus_library(GameState *gs)
{
    // Initialisation de la bibliothèque de virus
    strcpy(gs->viruses[0].name, "Trojan.Stealth");
    strcpy(gs->viruses[0].description, "Virus furtif pour accès discret");
    gs->viruses[0].damage = 25;
    gs->viruses[0].stealth_rating = 8;
    gs->viruses[0].is_detected = false;

    strcpy(gs->viruses[1].name, "Worm.DataMiner");
    strcpy(gs->viruses[1].description, "Ver extracteur de données");
    gs->viruses[1].damage = 40;
    gs->viruses[1].stealth_rating = 5;
    gs->viruses[1].is_detected = false;

    strcpy(gs->viruses[2].name, "Ransomware.CryptoLock");
    strcpy(gs->viruses[2].description, "Chiffrement malveillant de systèmes");
    gs->viruses[2].damage = 70;
    gs->viruses[2].stealth_rating = 3;
    gs->viruses[2].is_detected = false;

    gs->virus_count = 3;
}

void game_loop(GameState *gs)
{
    char input[MAX_INPUT_LENGTH];

    while (gs->running && !gs->player.game_over)
    {
        printf("%s[%s@neon-terminal]%s $ ", nh_c(NH_C_BRIGHT_GREEN), gs->player.name, nh_c(NH_C_RESET));

        if (nh_read_line(input, sizeof(input)) != NH_IO_OK)
        {
            // Entrée fermée (Ctrl+D, fichier épuisé) : on quitte au lieu de boucler.
            printf("\n%s\n", nh_tr(NH_STR_INPUT_CLOSED));
            break;
        }

        nh_dispatch(gs, input);

        if (gs->player.alert_level >= 100)
        {
            print_colored_text("\n=== GAME OVER ===\n", COLOR_RED);
            printf("Vous avez été détecté par les systèmes de sécurité corporate !\n");
            gs->player.game_over = true;
        }
    }
}
