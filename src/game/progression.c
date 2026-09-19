#include "progression.h"

#include <stdio.h>
#include <string.h>

#include "../i18n/i18n.h"
#include "../ui/term.h"
#include "commands.h"

/* Expérience cumulée pour atteindre chaque niveau (index = niveau). Valeurs provisoires :
 * l'équilibrage se fait en phase 5, avec une simulation. */
static const int k_xp_required[NH_LEVEL_MAX + 1] = {0, 0, 15, 60, 140, 260, 420};

int nh_level_xp_required(int level)
{
    if (level < 1 || level > NH_LEVEL_MAX)
        return -1;
    return k_xp_required[level];
}

int nh_level_for_xp(int xp)
{
    int level = 1;
    for (int l = 2; l <= NH_LEVEL_MAX; l++)
        if (xp >= k_xp_required[l])
            level = l;
    return level;
}

static const NhStr k_level_names[NH_LEVEL_MAX + 1] = {
    NH_STR_LEVEL_NAME_1, NH_STR_LEVEL_NAME_1, NH_STR_LEVEL_NAME_2, NH_STR_LEVEL_NAME_3,
    NH_STR_LEVEL_NAME_4, NH_STR_LEVEL_NAME_5, NH_STR_LEVEL_NAME_6,
};

const char *nh_level_name(int level)
{
    if (level < 1 || level > NH_LEVEL_MAX)
        return "";
    return nh_tr(k_level_names[level]);
}

/* 5 + 4 + 3 + 2 + 1 = 15 : exactement le niveau 2, atteint au cinquième scan. */
int nh_scan_xp(int scans_done)
{
    static const int k_scan_xp[] = {5, 4, 3, 2, 1};
    if (scans_done < 0 || scans_done >= (int)(sizeof k_scan_xp / sizeof k_scan_xp[0]))
        return 0;
    return k_scan_xp[scans_done];
}

bool nh_milestone_claim(Player *player, NhMilestone milestone)
{
    if (milestone < 0 || milestone >= NH_MS_COUNT)
        return false;
    unsigned bit = 1u << (unsigned)milestone;
    if (player->milestones & bit)
        return false;
    player->milestones |= bit;
    return true;
}

/* ---- Montée de niveau ----------------------------------------------------- */

typedef struct
{
    CommandType unlocks[3]; /* drapeaux CMD_* à lever */
    int unlock_count;
    int virus_library; /* taille de la bibliothèque de virus (0 = inchangée) */
    int stealth_bonus;
    int credits;
    bool ai_assistant;
    bool quantum_computer;
} LevelReward;

/* Ce que chaque niveau accorde en plus des commandes à niveau minimum (voir la table de commandes). */
static const LevelReward k_rewards[NH_LEVEL_MAX + 1] = {
    [2] = {.unlocks = {CMD_BRUTEFORCE}, .unlock_count = 1},
    [3] = {.unlocks = {CMD_DECRYPT, CMD_BACKDOOR}, .unlock_count = 2, .virus_library = 1},
    [4] = {.unlocks = {CMD_EXPLOIT, CMD_TRACE_ROUTE, CMD_UPLOAD_VIRUS},
           .unlock_count = 3,
           .virus_library = 2,
           .stealth_bonus = 2},
    [5] = {.unlocks = {CMD_AI_HACK, CMD_QUANTUM_DECRYPT},
           .unlock_count = 2,
           .virus_library = 3,
           .stealth_bonus = 3,
           .credits = 5000, /* provisoire : à revoir avec l'économie (phase 3.4 / 5) */
           .ai_assistant = true,
           .quantum_computer = true},
};

enum { MAX_TRACKED = 64 };

static void level_up(GameState *gs)
{
    Player *p = &gs->player;
    size_t count;
    const NhCommand *table = nh_commands(&count);
    if (count > MAX_TRACKED)
        count = MAX_TRACKED;

    bool before[MAX_TRACKED];
    for (size_t i = 0; i < count; i++)
        before[i] = nh_command_available(gs, &table[i]);

    p->level = (HackerLevel)((int)p->level + 1);
    int level = (int)p->level;
    const LevelReward *r = &k_rewards[level];
    for (int i = 0; i < r->unlock_count; i++)
        p->commands_unlocked[r->unlocks[i]] = true;
    if (r->virus_library > 0)
        p->virus_library_size = r->virus_library;
    p->stealth_rating += r->stealth_bonus;
    p->credits += r->credits;
    if (r->ai_assistant)
        p->has_ai_assistant = true;
    if (r->quantum_computer)
        p->has_quantum_computer = true;

    printf("\n%s%s%s\n", nh_c(NH_C_BRIGHT_GREEN), nh_tr(NH_STR_PROG_LEVEL_UP), nh_c(NH_C_RESET));
    printf(nh_tr(NH_STR_PROG_LEVEL_NOW), level, nh_level_name(level));
    printf("\n");

    /* Les commandes nouvellement utilisables sont déduites de la table : jamais d'écart avec l'aide. */
    char list[256] = "";
    size_t used = 0;
    for (size_t i = 0; i < count; i++)
    {
        if (before[i] || table[i].hidden || !nh_command_available(gs, &table[i]))
            continue;
        int n = snprintf(list + used, sizeof list - used, "%s%s", used > 0 ? ", " : "", table[i].name);
        if (n < 0 || (size_t)n >= sizeof list - used)
            break;
        used += (size_t)n;
    }
    if (used > 0)
    {
        printf(nh_tr(NH_STR_PROG_NEW_COMMANDS), nh_c(NH_C_YELLOW), list, nh_c(NH_C_RESET));
        printf("\n");
    }
    if (r->ai_assistant || r->quantum_computer)
        printf("%s\n", nh_tr(NH_STR_PROG_EQUIPMENT));
    if (r->credits > 0)
    {
        printf(nh_tr(NH_STR_PROG_CREDITS), r->credits);
        printf("\n");
    }
    if (level == NH_LEVEL_MAX)
        printf("%s\n", nh_tr(NH_STR_PROG_MAX_LEVEL));
}

int nh_grant_xp(GameState *gs, int amount)
{
    if (amount <= 0)
        return 0;

    Player *p = &gs->player;
    p->experience = (p->experience > NH_XP_CAP - amount) ? NH_XP_CAP : p->experience + amount;

    printf("%s", nh_c(NH_C_GREEN));
    printf(nh_tr(NH_STR_PROG_XP_GAIN), amount);
    printf("%s ", nh_c(NH_C_RESET));

    int gained = 0;
    int target = nh_level_for_xp(p->experience);
    while ((int)p->level < target)
    {
        level_up(gs);
        gained++;
    }
    return gained;
}
