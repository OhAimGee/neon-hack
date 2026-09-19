#ifndef NH_GAME_H
#define NH_GAME_H

#include <stdbool.h>

#include "../core/storage.h"
#include "advanced_hacking.h"
#include "alert.h"
#include "contacts.h"
#include "events.h"
#include "game_types.h"
#include "quest_system.h"
#include "shop.h"

#define NH_MAX_NODES 7 /* = nh_world_count() (world.c le vérifie à la compilation) */

/*
 * Tout l'état d'une partie. Aucune variable globale : chaque fonction de jeu
 * reçoit le GameState qu'elle doit lire ou modifier (et les tests peuvent en
 * fabriquer un à volonté). Les sous-systèmes d'origine y sont regroupés en
 * attendant leur réécriture (Phase 3).
 */
typedef struct GameState
{
    Player player;
    bool running; /* false : la boucle de jeu s'arrête */

    /* Réseau */
    NetworkNode nodes[NH_MAX_NODES];

    /* Mécaniques d'origine */
    Virus viruses[10];
    int virus_count;
    bool stealth_mode;

    /* Sous-systèmes d'origine */
    CyberShop shop;
    AlertSystem alert;
    QuestSystem quests;
    ContactSystem contacts;
    AdvancedHackingSystem advanced;

    /* Ce qui vient de se passer (voir events.h) : vide entre deux commandes, jamais sauvegardé */
    EventBus events;

    /* Tutoriel et sauvegarde */
    TutorialState tutorial;
    char save_path[NH_PATH_MAX + 32]; /* fichier de sauvegarde ; "" = aucune sauvegarde (tests, pas de dossier) */
} GameState;

/* Cycle de vie */
void init_game(GameState *gs); /* état initial (héros « Case », sans tutoriel), sans aucune entrée/sortie */
void game_loop(GameState *gs);

/* Progression */

/* Affichage (couleurs d'origine) */
void print_colored_text(const char *text, const char *color);
void print_cyberpunk_art(void);
void print_typing_effect(const char *text, int delay_ms);

/*
 * Gestionnaires de commandes : (état, argument) -> succès.
 * Ils sont branchés sur le nom saisi par la table de commandes (commands.c).
 */
bool cmd_scan_network(GameState *gs, const char *arg);
bool cmd_bruteforce(GameState *gs, const char *target);
bool cmd_decrypt(GameState *gs, const char *encrypted_data);
bool cmd_backdoor(GameState *gs, const char *target);
bool cmd_trace_route(GameState *gs, const char *target);
bool cmd_upload_virus(GameState *gs, const char *target);
bool cmd_stealth_mode(GameState *gs, const char *arg);
bool cmd_exploit(GameState *gs, const char *target);
bool cmd_ai_hack(GameState *gs, const char *target);
bool cmd_quantum_decrypt(GameState *gs, const char *data);

bool cmd_shop(GameState *gs, const char *arg);
bool cmd_lay_low(GameState *gs, const char *arg);
bool cmd_quests(GameState *gs, const char *arg);
bool cmd_contacts(GameState *gs, const char *arg);
bool cmd_messages(GameState *gs, const char *arg);
bool cmd_read(GameState *gs, const char *argument);
bool cmd_interact_contact(GameState *gs, const char *argument);

bool cmd_advanced_hack(GameState *gs, const char *target_name);
bool cmd_stealth_mode_toggle(GameState *gs, const char *arg);
bool cmd_ai_assist_hack(GameState *gs, const char *target_name);
bool cmd_neural_sync(GameState *gs, const char *arg);
bool cmd_analyze_defenses(GameState *gs, const char *target_name);
bool cmd_social_engineer(GameState *gs, const char *target_name);
bool cmd_temporal_hack(GameState *gs, const char *target_name);

#endif /* NH_GAME_H */
