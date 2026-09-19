/*
 * État de partie, introduction, boucle de jeu et progression.
 *
 * init_game vient du code d'origine (neon_hack.c v2.087), adapté à GameState. La boucle de
 * jeu et le dispatch des commandes sont neufs (voir commands.c) ; l'introduction est dans
 * intro.c, le menu dans menu.c, la sauvegarde dans save.c.
 */
#include "game.h"
#include "world.h"

#include "../core/io.h"
#include "../core/platform.h"
#include "../i18n/i18n.h"
#include "../ui/lineedit.h"
#include "../ui/term.h"
#include "commands.h"
#include "complete.h"
#include "legacy_colors.h"
#include "save.h"

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

    strcpy(gs->player.name, NH_DEFAULT_NAME);
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
    nh_quests_init(&gs->quests);
    init_contact_system(&gs->contacts);
    init_advanced_hacking_system(&gs->advanced);


    nh_world_init(gs->nodes);
    nh_world_discover(gs->nodes, gs->player.level); // au départ, seul votre propre poste est connu

    // Initialiser la bibliothèque de virus
    init_virus_library(gs);

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
    bool save_warned = false;

    while (gs->running && !gs->player.game_over)
    {
        nh_refresh_hud(gs);
        char prompt[192];
        snprintf(prompt, sizeof prompt, "%s[%s@neon-terminal]%s $ ", nh_c(NH_C_BRIGHT_GREEN), gs->player.name,
                 nh_c(NH_C_RESET));

        /* Sur un vrai terminal : édition, historique et complétion par TAB ; sinon, une simple lecture de ligne. */
        if (nh_lineedit_read(prompt, input, sizeof(input), nh_complete_line, gs) != NH_IO_OK)
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
        else if (gs->running && gs->save_path[0] != '\0' && nh_save_game(gs) != NH_SAVE_OK && !save_warned)
        {
            /* Sauvegarde automatique après chaque commande ; une seule alerte si elle échoue. Une
             * partie perdue n'est pas enregistrée : « Continuer » reprend la dernière sauvegarde. */
            printf(nh_tr(NH_STR_SAVE_FAILED), gs->save_path);
            printf("\n");
            save_warned = true;
        }
    }
}
