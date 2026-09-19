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
#include "../i18n/i18n.h"
#include "legacy_colors.h"
#include "progression.h"
#include "world.h"

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

    bool was_known[NH_MAX_NODES];
    for (int i = 0; i < nh_world_count(); i++)
        was_known[i] = gs->nodes[i].is_discovered;
    int fresh = nh_world_discover(gs->nodes, gs->player.level);

    for (int i = 0; i < nh_world_count(); i++)
    {
        const NetworkNode *node = &gs->nodes[i];
        if (!node->is_discovered)
            continue;

        printf("  %s[%d]%s %s", COLOR_YELLOW, i + 1, COLOR_RESET, node->name);
        if (!was_known[i])
            printf(" %s%s%s", COLOR_BRIGHT_CYAN, nh_tr(NH_STR_WORLD_TAG_NEW), COLOR_RESET);
        if (node->is_compromised)
            printf(" %s%s%s", COLOR_GREEN, nh_tr(NH_STR_WORLD_TAG_COMPROMISED), COLOR_RESET);
        else if (!nh_world_reachable(gs->nodes, i))
            printf(" %s%s%s", COLOR_RED, nh_tr(NH_STR_WORLD_TAG_ROUTE_CLOSED), COLOR_RESET);

        const char *color = node->security == SECURITY_LOW ? COLOR_GREEN
                            : node->security == SECURITY_MEDIUM ? COLOR_YELLOW
                                                                : COLOR_RED;
        printf(" %s", color);
        printf(nh_tr(NH_STR_WORLD_SECURITY), nh_security_label(node->security));
        printf("%s\n", COLOR_RESET);
    }

    int scan_xp = nh_scan_xp(gs->player.scans_done);
    if (gs->player.scans_done < 1000000)
        gs->player.scans_done++;
    if (scan_xp > 0)
        nh_grant_xp(gs, scan_xp);
    else if (fresh == 0) // un scan qui révèle un nouveau système apprend quelque chose
        printf("\n%s\n", nh_tr(NH_STR_PROG_SCAN_DONE));
    nh_alert_raise(&gs->alert, 1);

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

    int target_index = nh_world_resolve(gs, target, true);
    if (target_index < 0)
        return false;

    NetworkNode *node = &gs->nodes[target_index];

    if (node->is_compromised)
    {
        printf("%s\n", nh_tr(NH_STR_WORLD_ALREADY_COMPROMISED));
        return false;
    }

    print_colored_text("\n=== ATTAQUE BRUTE FORCE ===\n", COLOR_RED);
    printf("Cible: %s\n", target);
    printf("Tentative de craquage du mot de passe...\n");

    char *passwords[] = {"admin", "123456", "password", "root", "guest"};
    int success_chance = nh_world_chance(NH_HACK_BRUTE, node, gs->player.level,
                                         -nh_alert_success_penalty(&gs->alert));

    for (int i = 0; i < 5; i++)
    {
        printf("Essai: %s", passwords[i]);
        for (int j = 0; j < 3; j++)
        {
            printf(".");
            fflush(stdout);
            nh_sleep_ms(300);
        }

        if (rand() % 100 < success_chance)
        {
            printf(" %sSUCCÈS !%s\n", COLOR_GREEN, COLOR_RESET);
            nh_world_compromise(gs, target_index, false);
            nh_alert_raise(&gs->alert, (int)node->security * 5);
            return true;
        }
        else
        {
            printf(" %sÉCHEC%s\n", COLOR_RED, COLOR_RESET);
        }
    }

    printf("\nAttaque brute force échouée.\n");
    nh_alert_raise(&gs->alert, (int)node->security * 8);
    return false;
}

bool cmd_decrypt(GameState *gs, const char *encrypted_data)
{
    if (strlen(encrypted_data) == 0)
    {
        printf("Usage: decrypt <message_crypté>\n");
        printf("Essayez: decrypt WKLV#LV#D#WHVW\n");
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
        if (nh_milestone_claim(gs, NH_MS_DECRYPT_TEST))
            nh_grant_xp(gs, 20);
        else
            printf("%s\n", nh_tr(NH_STR_PROG_ALREADY_CLAIMED));
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

    int target_index = nh_world_resolve(gs, target, true);
    if (target_index < 0)
        return false;

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

    // Le mode furtif aide, l'alerte pénalise
    int bonus = (gs->stealth_mode ? 20 : 0) - nh_alert_success_penalty(&gs->alert);
    int success_chance = nh_world_chance(NH_HACK_BACKDOOR, node, gs->player.level, bonus);

    if (rand() % 100 < success_chance)
    {
        node->has_backdoor = true;
        gs->player.backdoors_active++;
        print_colored_text("✓ Backdoor installée avec succès !\n", COLOR_GREEN);
        nh_grant_xp(gs, 15);

        // Bonus de crédits pour backdoor réussie
        gs->player.credits += 500;
        printf("[+500 crédits]\n");

        // Risque d'alerte réduit si en mode furtif
        nh_alert_raise(&gs->alert, gs->stealth_mode ? 5 : 10);
        return true;
    }
    else
    {
        print_colored_text("✗ Échec de l'installation - Système protégé\n", COLOR_RED);
        nh_alert_raise(&gs->alert, 20);
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

    // Passif : il n'exige pas de route ouverte, seulement que le système soit connu
    int target_index = nh_world_resolve(gs, target, false);
    if (target_index < 0)
        return false;

    NetworkNode *node = &gs->nodes[target_index];
    bool first_trace = !node->is_traced;
    node->is_traced = true;

    printf("Route trouvée ! Informations système révélées :\n");
    printf("  Corporation: %s\n", node->corporation);
    printf("  Force firewall: %d/10\n", node->firewall_strength);
    printf("  Fichiers secrets: %d détectés\n", node->file_count);
    int uplink = nh_world_uplink(target_index);
    if (uplink >= 0)
        printf("  Relais: %s\n", gs->nodes[uplink].name);

    if (first_trace)
        nh_grant_xp(gs, 8);
    else
        printf("%s\n", nh_tr(NH_STR_PROG_ALREADY_TRACED));
    nh_alert_raise(&gs->alert, 3);
    return true;
}

bool cmd_exploit(GameState *gs, const char *target)
{
    if (strlen(target) == 0)
    {
        printf("%s\n", nh_tr(NH_STR_EXPLOIT_USAGE));
        return false;
    }

    // L'exploit perce directement : pas besoin que la route soit ouverte
    int target_index = nh_world_resolve(gs, target, false);
    if (target_index < 0)
        return false;

    NetworkNode *node = &gs->nodes[target_index];

    if (node->is_compromised)
    {
        printf("%s\n", nh_tr(NH_STR_WORLD_ALREADY_COMPROMISED));
        return false;
    }
    if (!node->is_traced)
    {
        printf(nh_tr(NH_STR_EXPLOIT_NO_FLAW), node->name, node->name);
        printf("\n");
        return false;
    }

    printf(nh_tr(NH_STR_EXPLOIT_START), node->name);
    printf("\n");
    print_typing_effect(nh_tr(NH_STR_EXPLOIT_STEP1), 400);
    print_typing_effect(nh_tr(NH_STR_EXPLOIT_STEP2), 400);

    int success_chance = nh_world_chance(NH_HACK_EXPLOIT, node, gs->player.level,
                                         -nh_alert_success_penalty(&gs->alert));
    if (rand() % 100 < success_chance)
    {
        print_colored_text(nh_tr(NH_STR_EXPLOIT_OK), COLOR_GREEN);
        printf("\n");
        nh_world_compromise(gs, target_index, false);
        nh_alert_raise(&gs->alert, 12);
        return true;
    }

    print_colored_text(nh_tr(NH_STR_EXPLOIT_FAIL), COLOR_RED);
    printf("\n");
    nh_alert_raise(&gs->alert, 25);
    return false;
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

    int target_index = nh_world_resolve(gs, target, true);
    if (target_index < 0)
        return false;

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

    // La discrétion du virus et une backdoor déjà en place aident, l'alerte pénalise
    int bonus = chosen_virus->stealth_rating + (node->has_backdoor ? 30 : 0) - nh_alert_success_penalty(&gs->alert);
    int success_chance = nh_world_chance(NH_HACK_VIRUS, node, gs->player.level, bonus);

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

        nh_grant_xp(gs, 20);
        gs->player.credits += chosen_virus->damage * 10;
        printf("[+%d crédits]\n", chosen_virus->damage * 10);

        nh_alert_raise(&gs->alert, 15 - chosen_virus->stealth_rating);
        return true;
    }
    else
    {
        print_colored_text("✗ Upload échoué - Antivirus détecté\n", COLOR_RED);
        chosen_virus->is_detected = true;
        nh_alert_raise(&gs->alert, 25);
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

    int target_index = nh_world_resolve(gs, target, true);
    if (target_index < 0)
        return false;

    NetworkNode *node = &gs->nodes[target_index];

    // Sur un système déjà compromis, l'IA sert à extraire ce qu'il reste (une seule fois par fichier)
    if (node->is_compromised)
    {
        if (nh_world_locked_files(node) == 0)
        {
            printf(nh_tr(NH_STR_WORLD_NOTHING_TO_EXTRACT), node->name);
            printf("\n");
            return false;
        }
        print_colored_text(">>> IA NOVA EN LIGNE <<<\n", COLOR_MAGENTA);
        printf(nh_tr(NH_STR_WORLD_EXTRACT), node->name);
        printf("\n");
        nh_world_extract(gs, target_index);
        nh_alert_raise(&gs->alert, 5);
        return true;
    }

    printf("Lancement de l'attaque IA sur %s...\n", target);
    print_colored_text(">>> IA NOVA EN LIGNE <<<\n", COLOR_MAGENTA);
    print_typing_effect("Analyse des patterns de sécurité...", 300);
    print_typing_effect("Génération d'exploits adaptatifs...", 300);
    print_typing_effect("Exécution de l'attaque neuromorphe...", 300);

    // L'IA a un taux de succès très élevé
    int success_chance = nh_world_chance(NH_HACK_AI, node, gs->player.level,
                                         -nh_alert_success_penalty(&gs->alert));

    if (rand() % 100 < success_chance)
    {
        print_colored_text("✓ SYSTÈME COMPROMIS PAR L'IA !\n", COLOR_BRIGHT_GREEN);
        // L'IA révèle tous les fichiers secrets
        nh_world_compromise(gs, target_index, true);
        nh_alert_raise(&gs->alert, 5); // L'IA est très discrète
        return true;
    }
    else
    {
        print_colored_text("✗ IA repoussée par des contre-mesures adaptatives\n", COLOR_RED);
        nh_alert_raise(&gs->alert, 30);
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

        if (nh_milestone_claim(gs, NH_MS_CLASSIFIED_DOC))
        {
            gs->player.credits += 10000;
            printf("[+10000 crédits bonus !]\n");
        }
        else
            printf("%s\n", nh_tr(NH_STR_PROG_ALREADY_CLAIMED));
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

    if (nh_milestone_claim(gs, NH_MS_QUANTUM_FIRST))
    {
        nh_grant_xp(gs, 50);
        gs->player.credits += 2000;
        printf("[+2000 crédits]\n");
    }
    else
        printf("%s\n", nh_tr(NH_STR_PROG_ALREADY_CLAIMED));

    // Le quantique génère moins d'alertes mais consomme beaucoup d'énergie
    nh_alert_raise(&gs->alert, 2);

    return true;
}
