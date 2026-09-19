/*
 * Commandes de hacking classiques (scan, bruteforce, decrypt, backdoor…).
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


bool cmd_scan_network(GameState *gs, const char *arg)
{
    (void)arg;

    print_colored_text("\n=== SCAN RÉSEAU ===\n", COLOR_BRIGHT_CYAN);
    printf("Recherche de systèmes connectés...\n");

    for (int i = 0; i < 3; i++)
    {
        printf(".");
        fflush(stdout);
        nh_sleep_ms(500);
    }
    printf("\n\nSystèmes détectés:\n");

    int max_nodes = (gs->player.level >= LEVEL_HACKER) ? 3 : (gs->player.level >= LEVEL_APPRENTICE) ? 2
                                                                                                      : 1;

    for (int i = 0; i < max_nodes; i++)
    {
        printf("  %s[%d]%s %s", COLOR_YELLOW, i + 1, COLOR_RESET, gs->nodes[i].name);

        if (gs->nodes[i].is_compromised)
        {
            printf(" %s[COMPROMIS]%s", COLOR_GREEN, COLOR_RESET);
        }

        switch (gs->nodes[i].security)
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

    gs->discovered_nodes = max_nodes;
    gain_experience(gs, 5);
    increase_alert_level(gs, 1);

    return true;
}

bool cmd_bruteforce(GameState *gs, const char *target)
{
    if (strlen(target) == 0)
    {
        printf("Usage: bruteforce <target>\n");
        printf("Exemple: bruteforce localhost\n");
        return false;
    }

    int target_index = -1;
    for (int i = 0; i < gs->discovered_nodes; i++)
    {
        if (strcmp(gs->nodes[i].name, target) == 0)
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

    NetworkNode *node = &gs->nodes[target_index];

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
            nh_sleep_ms(300);
        }

        int success_chance = 80 - (node->security * 15);
        if (rand() % 100 < success_chance)
        {
            printf(" %sSUCCÈS !%s\n", COLOR_GREEN, COLOR_RESET);
            node->is_compromised = true;

            printf("\nAccès obtenu à %s !\n", target);
            printf("Données récupérées: %d credits\n", node->data_value);

            gain_experience(gs, node->data_value / 2);
            increase_alert_level(gs, node->security * 5);

            // Débloquer bruteforce après le premier hack réussi
            if (!gs->player.commands_unlocked[CMD_BRUTEFORCE] && target_index == 0)
            {
                gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;
                gs->player.commands_unlocked[CMD_DECRYPT] = true;
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
    increase_alert_level(gs, node->security * 8);
    return false;
}

bool cmd_decrypt(GameState *gs, const char *encrypted_data)
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
        gain_experience(gs, 20);
    }

    return true;
}

bool cmd_backdoor(GameState *gs, const char *target)
{
    if (strlen(target) == 0)
    {
        printf("Usage: backdoor <nom_système>\n");
        return false;
    }

    // Chercher le nœud cible
    int target_index = -1;
    for (int i = 0; i < gs->discovered_nodes; i++)
    {
        if (strcmp(gs->nodes[i].name, target) == 0)
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

    NetworkNode *node = &gs->nodes[target_index];

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
    int success_chance = 70 + (gs->player.level * 10) - (node->security * 15);
    if (gs->stealth_mode)
        success_chance += 20;

    if (rand() % 100 < success_chance)
    {
        node->has_backdoor = true;
        gs->player.backdoors_active++;
        print_colored_text("✓ Backdoor installée avec succès !\n", COLOR_GREEN);
        gain_experience(gs, 15);

        // Bonus de crédits pour backdoor réussie
        gs->player.credits += 500;
        printf("[+500 crédits]\n");

        // Risque d'alerte réduit si en mode furtif
        increase_alert_level(gs, gs->stealth_mode ? 5 : 10);
        return true;
    }
    else
    {
        print_colored_text("✗ Échec de l'installation - Système protégé\n", COLOR_RED);
        increase_alert_level(gs, 20);
        return false;
    }
}

bool cmd_trace_route(GameState *gs, const char *target)
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
        nh_sleep_ms(200); // 200ms delay
    }

    // Chercher le nœud cible
    int target_index = -1;
    for (int i = 0; i < gs->discovered_nodes; i++)
    {
        if (strcmp(gs->nodes[i].name, target) == 0)
        {
            target_index = i;
            break;
        }
    }

    if (target_index != -1)
    {
        NetworkNode *node = &gs->nodes[target_index];
        node->is_traced = true;

        printf("Route trouvée ! Informations système révélées :\n");
        printf("  Corporation: %s\n", node->corporation);
        printf("  Force firewall: %d/10\n", node->firewall_strength);
        printf("  Fichiers secrets: %d détectés\n", node->file_count);

        gain_experience(gs, 8);
        increase_alert_level(gs, 3);
        return true;
    }
    else
    {
        printf("Hôte inaccessible\n");
        return false;
    }
}

bool cmd_upload_virus(GameState *gs, const char *target)
{
    if (gs->player.virus_library_size == 0)
    {
        printf("Aucun virus disponible. Développez d'abord votre arsenal.\n");
        return false;
    }

    if (strlen(target) == 0)
    {
        printf("Usage: uploadvirus <nom_système>\n");
        printf("Virus disponibles :\n");
        for (int i = 0; i < gs->virus_count && i < gs->player.virus_library_size; i++)
        {
            printf("  %s - %s (DMG:%d, STEALTH:%d)\n",
                   gs->viruses[i].name,
                   gs->viruses[i].description,
                   gs->viruses[i].damage,
                   gs->viruses[i].stealth_rating);
        }
        return false;
    }

    // Chercher le nœud cible
    int target_index = -1;
    for (int i = 0; i < gs->discovered_nodes; i++)
    {
        if (strcmp(gs->nodes[i].name, target) == 0)
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

    NetworkNode *node = &gs->nodes[target_index];

    if (node->has_virus)
    {
        printf("Ce système est déjà infecté.\n");
        return false;
    }

    // Choisir un virus aléatoire disponible
    int virus_choice = rand() % (gs->player.virus_library_size < gs->virus_count ? gs->player.virus_library_size : gs->virus_count);
    Virus *chosen_virus = &gs->viruses[virus_choice];

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

        gain_experience(gs, 20);
        gs->player.credits += chosen_virus->damage * 10;
        printf("[+%d crédits]\n", chosen_virus->damage * 10);

        increase_alert_level(gs, 15 - chosen_virus->stealth_rating);
        return true;
    }
    else
    {
        print_colored_text("✗ Upload échoué - Antivirus détecté\n", COLOR_RED);
        chosen_virus->is_detected = true;
        increase_alert_level(gs, 25);
        return false;
    }
}

bool cmd_stealth_mode(GameState *gs, const char *arg)
{
    (void)arg;

    if (gs->player.stealth_rating < 5)
    {
        printf("Capacité de furtivité insuffisante (requis: 5, actuel: %d)\n", gs->player.stealth_rating);
        return false;
    }

    gs->stealth_mode = !gs->stealth_mode;

    if (gs->stealth_mode)
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

bool cmd_ai_hack(GameState *gs, const char *target)
{
    if (!gs->player.has_ai_assistant)
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
    for (int i = 0; i < gs->discovered_nodes; i++)
    {
        if (strcmp(gs->nodes[i].name, target) == 0)
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

    NetworkNode *node = &gs->nodes[target_index];

    printf("Lancement de l'attaque IA sur %s...\n", target);
    print_colored_text(">>> IA NOVA EN LIGNE <<<\n", COLOR_MAGENTA);
    print_typing_effect("Analyse des patterns de sécurité...", 300);
    print_typing_effect("Génération d'exploits adaptatifs...", 300);
    print_typing_effect("Exécution de l'attaque neuromorphe...", 300);

    // L'IA a un taux de succès très élevé
    int success_chance = 85 + (gs->player.level * 5);
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
            gs->player.credits += node->secret_files[i].credits_value;
        }

        gain_experience(gs, 35);
        increase_alert_level(gs, 5); // L'IA est très discrète
        return true;
    }
    else
    {
        print_colored_text("✗ IA repoussée par des contre-mesures adaptatives\n", COLOR_RED);
        increase_alert_level(gs, 30);
        return false;
    }
}

bool cmd_quantum_decrypt(GameState *gs, const char *data)
{
    if (!gs->player.has_quantum_computer)
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

        gs->player.credits += 10000;
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

    gain_experience(gs, 50);
    gs->player.credits += 2000;
    printf("[+2000 crédits]\n");

    // Le quantique génère moins d'alertes mais consomme beaucoup d'énergie
    increase_alert_level(gs, 2);

    return true;
}
