#ifndef ALERT_SYSTEM_H
#define ALERT_SYSTEM_H

#include <stdbool.h>

// Niveaux d'alerte
typedef enum
{
    ALERT_NONE = 0,     // Invisible
    ALERT_LOW = 1,      // Surveillance légère
    ALERT_MEDIUM = 3,   // Recherche active
    ALERT_HIGH = 6,     // Traque intensive
    ALERT_CRITICAL = 8, // Lockdown total
    ALERT_MAXIMUM = 10  // Manhunt cybernétique
} AlertLevel;

// Méthodes de réduction d'alerte
typedef enum
{
    REDUCTION_TIME,   // Attendre que l'alerte diminue
    REDUCTION_VPN,    // Utiliser un VPN
    REDUCTION_PROXY,  // Chaîne de proxies
    REDUCTION_GHOST,  // Ghost Protocol (objet boutique)
    REDUCTION_LAYLOW, // Se faire discret
    REDUCTION_FRAME   // Faire accuser quelqu'un d'autre
} AlertReductionMethod;

// Structure du système d'alerte
typedef struct
{
    int current_level;
    int max_level_reached;
    int time_since_last_activity;
    bool vpn_active;
    bool proxy_active;
    int ghost_protocols_available;
    int heat_decay_rate; // Vitesse de réduction naturelle
} AlertSystem;

// Fonctions du système d'alerte
void init_alert_system(AlertSystem *alert);
void increase_alert(AlertSystem *alert, int amount, const char *reason);
void decrease_alert(AlertSystem *alert, int amount, const char *method);
void update_alert_passive(AlertSystem *alert); // Réduction passive avec le temps
void display_alert_status(const AlertSystem *alert);
void display_alert_reduction_menu(const AlertSystem *alert);
bool attempt_alert_reduction(AlertSystem *alert, AlertReductionMethod method, int *player_credits);
const char *get_alert_level_name(int level);
const char *get_alert_level_color(int level);
bool is_activity_risky(int alert_level, const char *activity);

#endif // ALERT_SYSTEM_H
