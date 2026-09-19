#include "alert_system.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>

// Codes couleur
#define COLOR_RESET "\033[0m"
#define COLOR_GREEN "\033[32m"
#define COLOR_YELLOW "\033[33m"
#define COLOR_RED "\033[31m"
#define COLOR_CYAN "\033[36m"
#define COLOR_MAGENTA "\033[35m"
#define COLOR_BRIGHT_RED "\033[91m"

void init_alert_system(AlertSystem *alert)
{
    alert->current_level = 0;
    alert->max_level_reached = 0;
    alert->time_since_last_activity = 0;
    alert->vpn_active = false;
    alert->proxy_active = false;
    alert->ghost_protocols_available = 0;
    alert->heat_decay_rate = 1; // 1 point par "tour"
}

void increase_alert(AlertSystem *alert, int amount, const char *reason)
{
    int old_level = alert->current_level;
    alert->current_level += amount;

    // Plafonner à 10
    if (alert->current_level > 10)
    {
        alert->current_level = 10;
    }

    // Mettre à jour le maximum
    if (alert->current_level > alert->max_level_reached)
    {
        alert->max_level_reached = alert->current_level;
    }

    // Reset du temps d'inactivité
    alert->time_since_last_activity = 0;

    // Afficher l'augmentation
    if (alert->current_level > old_level)
    {
        printf("%s🚨 ALERTE AUGMENTÉE: %s%s\n",
               get_alert_level_color(alert->current_level),
               reason, COLOR_RESET);
        printf("   Niveau: %d → %s%d%s\n",
               old_level, get_alert_level_color(alert->current_level),
               alert->current_level, COLOR_RESET);
    }
}

void decrease_alert(AlertSystem *alert, int amount, const char *method)
{
    int old_level = alert->current_level;
    alert->current_level -= amount;

    // Ne pas descendre en dessous de 0
    if (alert->current_level < 0)
    {
        alert->current_level = 0;
    }

    if (alert->current_level < old_level)
    {
        printf("%s✅ ALERTE RÉDUITE: %s%s\n",
               COLOR_GREEN, method, COLOR_RESET);
        printf("   Niveau: %d → %s%d%s\n",
               old_level, get_alert_level_color(alert->current_level),
               alert->current_level, COLOR_RESET);
    }
}

void update_alert_passive(AlertSystem *alert)
{
    alert->time_since_last_activity++;

    // Réduction passive après inactivité
    if (alert->time_since_last_activity >= 3 && alert->current_level > 0)
    {
        int reduction = alert->heat_decay_rate;

        // Bonus de réduction avec VPN/Proxy
        if (alert->vpn_active)
            reduction++;
        if (alert->proxy_active)
            reduction++;

        decrease_alert(alert, reduction, "Refroidissement naturel");
        alert->time_since_last_activity = 0;
    }
}

void display_alert_status(const AlertSystem *alert)
{
    printf("\n%s╔═══════════════════════════════════════════════╗\n", COLOR_CYAN);
    printf("║              🚨 STATUT D'ALERTE 🚨            ║\n");
    printf("╠═══════════════════════════════════════════════╣\n");

    const char *color = get_alert_level_color(alert->current_level);
    const char *level_name = get_alert_level_name(alert->current_level);

    printf("║ Niveau actuel: %s%-8s [%2d/10]%s            ║\n",
           color, level_name, alert->current_level, COLOR_CYAN);
    printf("║ Maximum atteint: %-3d                        ║\n", alert->max_level_reached);
    printf("║ Inactivité: %-2d tours                       ║\n", alert->time_since_last_activity);

    printf("╠═══════════════════════════════════════════════╣\n");
    printf("║ Protection active:                            ║\n");
    printf("║ VPN: %s%-3s%s  Proxy: %s%-3s%s  Ghost: %s%-3d%s         ║\n",
           alert->vpn_active ? COLOR_GREEN : COLOR_RED,
           alert->vpn_active ? "ON" : "OFF", COLOR_CYAN,
           alert->proxy_active ? COLOR_GREEN : COLOR_RED,
           alert->proxy_active ? "ON" : "OFF", COLOR_CYAN,
           alert->ghost_protocols_available > 0 ? COLOR_GREEN : COLOR_RED,
           alert->ghost_protocols_available, COLOR_CYAN);
    printf("╚═══════════════════════════════════════════════╝%s\n", COLOR_RESET);

    // Affichage des conséquences du niveau d'alerte
    if (alert->current_level >= 8)
    {
        printf(COLOR_BRIGHT_RED "⚠️  CONSÉQUENCES: Boutique fermée, hacks très risqués!" COLOR_RESET "\n");
    }
    else if (alert->current_level >= 6)
    {
        printf(COLOR_RED "⚠️  CONSÉQUENCES: Difficultés accrues, traçage actif!" COLOR_RESET "\n");
    }
    else if (alert->current_level >= 3)
    {
        printf(COLOR_YELLOW "⚠️  CONSÉQUENCES: Surveillance renforcée!" COLOR_RESET "\n");
    }
}

void display_alert_reduction_menu(const AlertSystem *alert)
{
    printf("\n%s╔═══════════════════════════════════════════════╗\n", COLOR_MAGENTA);
    printf("║          🛡️  RÉDUCTION D'ALERTE 🛡️           ║\n");
    printf("╚═══════════════════════════════════════════════╝%s\n", COLOR_RESET);

    printf("\n%s1.%s Attendre (Gratuit) - Réduction: 1-2 points\n", COLOR_CYAN, COLOR_RESET);
    printf("%s2.%s Activer VPN (20 ¢) - Réduction: 1 point + protection\n", COLOR_CYAN, COLOR_RESET);
    printf("%s3.%s Chaîne Proxy (30 ¢) - Réduction: 2 points + furtivité\n", COLOR_CYAN, COLOR_RESET);
    if (alert->ghost_protocols_available > 0)
    {
        printf("%s4.%s Ghost Protocol (0 ¢) - Réduction: 3 points\n", COLOR_GREEN, COLOR_RESET);
    }
    else
    {
        printf("%s4.%s Ghost Protocol %s(Indisponible)%s\n", COLOR_RED, COLOR_RESET, COLOR_RED, COLOR_RESET);
    }
    printf("%s5.%s Se faire discret (40 ¢) - Réduction: 2-4 points\n", COLOR_CYAN, COLOR_RESET);
    printf("%s6.%s Frame quelqu'un (60 ¢) - Réduction: 3-5 points\n", COLOR_CYAN, COLOR_RESET);
    printf("%s0.%s Retour\n", COLOR_YELLOW, COLOR_RESET);
}

bool attempt_alert_reduction(AlertSystem *alert, AlertReductionMethod method, int *player_credits)
{
    switch (method)
    {
    case REDUCTION_TIME:
        printf(COLOR_CYAN "⏰ Vous attendez dans l'ombre..." COLOR_RESET "\n");
        sleep(1);
        decrease_alert(alert, 1 + rand() % 2, "Attente stratégique");
        alert->time_since_last_activity += 2;
        return true;

    case REDUCTION_VPN:
        if (*player_credits < 20)
        {
            printf(COLOR_RED "❌ Crédits insuffisants (20 ¢ requis)" COLOR_RESET "\n");
            return false;
        }
        *player_credits -= 20;
        alert->vpn_active = true;
        decrease_alert(alert, 1, "Activation VPN");
        printf(COLOR_GREEN "🔒 VPN activé - Protection continue!" COLOR_RESET "\n");
        return true;

    case REDUCTION_PROXY:
        if (*player_credits < 30)
        {
            printf(COLOR_RED "❌ Crédits insuffisants (30 ¢ requis)" COLOR_RESET "\n");
            return false;
        }
        *player_credits -= 30;
        alert->proxy_active = true;
        decrease_alert(alert, 2, "Chaîne de proxies");
        printf(COLOR_GREEN "🔀 Proxies configurés - Anonymat renforcé!" COLOR_RESET "\n");
        return true;

    case REDUCTION_GHOST:
        if (alert->ghost_protocols_available <= 0)
        {
            printf(COLOR_RED "❌ Aucun Ghost Protocol disponible!" COLOR_RESET "\n");
            return false;
        }
        alert->ghost_protocols_available--;
        decrease_alert(alert, 3, "Ghost Protocol");
        printf(COLOR_GREEN "👻 Ghost Protocol activé - Disparition numérique!" COLOR_RESET "\n");
        return true;

    case REDUCTION_LAYLOW:
        if (*player_credits < 40)
        {
            printf(COLOR_RED "❌ Crédits insuffisants (40 ¢ requis)" COLOR_RESET "\n");
            return false;
        }
        *player_credits -= 40;
        int reduction = 2 + rand() % 3;
        decrease_alert(alert, reduction, "Profil bas");
        printf(COLOR_GREEN "🤫 Vous vous faites discret dans les bas-fonds..." COLOR_RESET "\n");
        return true;

    case REDUCTION_FRAME:
        if (*player_credits < 60)
        {
            printf(COLOR_RED "❌ Crédits insuffisants (60 ¢ requis)" COLOR_RESET "\n");
            return false;
        }
        *player_credits -= 60;
        int frame_reduction = 3 + rand() % 3;
        decrease_alert(alert, frame_reduction, "Bouc émissaire");
        printf(COLOR_GREEN "🎭 Vous faites porter le chapeau à un autre..." COLOR_RESET "\n");
        printf(COLOR_YELLOW "⚠️  Karma négatif acquis!" COLOR_RESET "\n");
        return true;
    }
    return false;
}

const char *get_alert_level_name(int level)
{
    if (level == 0)
        return "INVISIBLE";
    else if (level <= 2)
        return "DISCRET";
    else if (level <= 4)
        return "SURVEILLÉ";
    else if (level <= 6)
        return "TRAQUÉ";
    else if (level <= 8)
        return "RECHERCHÉ";
    else
        return "MANHUNT";
}

const char *get_alert_level_color(int level)
{
    if (level == 0)
        return COLOR_GREEN;
    else if (level <= 2)
        return COLOR_CYAN;
    else if (level <= 4)
        return COLOR_YELLOW;
    else if (level <= 6)
        return COLOR_RED;
    else
        return COLOR_BRIGHT_RED;
}

bool is_activity_risky(int alert_level, const char *activity)
{
    // Plus le niveau d'alerte est élevé, plus les activités sont risquées
    if (alert_level >= 8)
    {
        printf(COLOR_BRIGHT_RED "⚠️  TRÈS RISQUÉ: %s (Alerte: %d)" COLOR_RESET "\n", activity, alert_level);
        return true;
    }
    else if (alert_level >= 6)
    {
        printf(COLOR_RED "⚠️  RISQUÉ: %s (Alerte: %d)" COLOR_RESET "\n", activity, alert_level);
        return true;
    }
    else if (alert_level >= 3)
    {
        printf(COLOR_YELLOW "⚠️  Prudence: %s (Alerte: %d)" COLOR_RESET "\n", activity, alert_level);
    }
    return false;
}
