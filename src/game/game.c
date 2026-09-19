/*
 * État de partie, introduction, boucle de jeu et progression.
 *
 * init_game / display_intro viennent
 * du code d'origine (neon_hack.c v2.087), adaptés à GameState. La boucle de
 * jeu et le dispatch des commandes sont neufs (voir commands.c).
 */
#include "game.h"
#include "world.h"

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
    nh_alert_init(&gs->alert);
    init_quest_system(&gs->quests);
    init_contact_system(&gs->contacts);
    init_advanced_hacking_system(&gs->advanced);


    nh_world_init(gs->nodes);
    nh_world_discover(gs->nodes, gs->player.level); // au départ, seul votre propre poste est connu

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
        nh_refresh_hud(gs);
        printf("%s[%s@neon-terminal]%s $ ", nh_c(NH_C_BRIGHT_GREEN), gs->player.name, nh_c(NH_C_RESET));

        if (nh_read_line(input, sizeof(input)) != NH_IO_OK)
        {
            // Entrée fermée (Ctrl+D, fichier épuisé) : on quitte au lieu de boucler.
            printf("\n%s\n", nh_tr(NH_STR_INPUT_CLOSED));
            break;
        }

        nh_dispatch(gs, input);

        if (nh_alert_is_game_over(&gs->alert))
        {
            printf("\n%s%s%s\n", nh_c(NH_C_RED), nh_tr(NH_STR_GAME_OVER_TITLE), nh_c(NH_C_RESET));
            printf("%s\n", nh_tr(NH_STR_GAME_OVER_DETECTED));
            gs->player.game_over = true;
        }
    }
}
