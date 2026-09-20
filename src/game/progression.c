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

bool nh_milestone_claim(GameState *gs, NhMilestone milestone)
{
    if (milestone < 0 || milestone >= NH_MS_COUNT)
        return false;
    unsigned bit = 1u << (unsigned)milestone;
    if (gs->player.milestones & bit)
        return false;
    gs->player.milestones |= bit;
    nh_event(gs, NH_EV_MILESTONE, (int)milestone);
    return true;
}

void nh_grant_reputation(GameState *gs, int amount)
{
    if (amount == 0)
        return;

    Player *p = &gs->player;
    long total = (long)p->reputation + amount;
    if (total > NH_REPUTATION_CAP)
        total = NH_REPUTATION_CAP;
    if (total < -NH_REPUTATION_CAP)
        total = -NH_REPUTATION_CAP;
    p->reputation = (int)total;

    if (amount > 0)
    {
        printf("%s", nh_c(NH_C_MAGENTA));
        printf(nh_tr(NH_STR_PROG_REPUTATION), amount);
        printf("%s ", nh_c(NH_C_RESET));
    }
    nh_event(gs, NH_EV_REPUTATION, p->reputation);
}

/* ---- Montée de niveau ----------------------------------------------------- */

typedef struct
{
    CommandType unlocks[3]; /* drapeaux CMD_* à lever */
    int unlock_count;
    int virus_library; /* taille de la bibliothèque de virus (0 = inchangée) */
    int stealth_bonus;
} LevelReward;

/*
 * Ce que chaque niveau accorde en plus des commandes à niveau minimum (voir la table de commandes).
 * Plus de crédits ni d'équipement offerts : l'IA et la puce quantique s'achètent chez R4Z0R, dont le
 * catalogue s'élargit avec le niveau (le gain de niveau l'annonce).
 */
static const LevelReward k_rewards[NH_LEVEL_MAX + 1] = {
    [2] = {.unlocks = {CMD_BRUTEFORCE}, .unlock_count = 1},
    [3] = {.unlocks = {CMD_DECRYPT, CMD_BACKDOOR}, .unlock_count = 2, .virus_library = 1},
    [4] = {.unlocks = {CMD_EXPLOIT, CMD_TRACE_ROUTE, CMD_UPLOAD_VIRUS},
           .unlock_count = 3,
           .virus_library = 2,
           .stealth_bonus = 2},
    [5] = {.unlocks = {CMD_AI_HACK, CMD_QUANTUM_DECRYPT}, .unlock_count = 2, .virus_library = 3, .stealth_bonus = 3},
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

    nh_event(gs, NH_EV_LEVEL_UP, level);

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

    /* Ce que R4Z0R met en vente à ce niveau. */
    char wares[256] = "";
    size_t shown = 0;
    for (int i = 0; i < gs->shop.item_count && i < ITEM_COUNT; i++)
    {
        const ShopItem *item = &gs->shop.items[i];
        if (item->level_required != level || !item->is_available)
            continue;
        int n = snprintf(wares + shown, sizeof wares - shown, "%s%s", shown > 0 ? ", " : "", item->name);
        if (n < 0 || (size_t)n >= sizeof wares - shown)
            break;
        shown += (size_t)n;
    }
    if (shown > 0)
    {
        printf(nh_tr(NH_STR_PROG_SHOP_NEW), wares);
        printf("\n");
    }
    if (level == NH_LEVEL_MAX)
        printf("%s\n", nh_tr(NH_STR_PROG_MAX_LEVEL));
}

static int grant_xp(GameState *gs, int amount, bool boosted)
{
    if (amount <= 0)
        return 0;
    if (amount > NH_XP_CAP)
        amount = NH_XP_CAP; /* pas de dépassement d'entier en y ajoutant le bonus */

    Player *p = &gs->player;
    int bonus = 0;
    if (boosted && p->xp_boost > 0)
    {
        p->xp_boost--;
        bonus = amount < NH_XP_BOOST_BONUS_CAP ? amount : NH_XP_BOOST_BONUS_CAP;
    }
    int total = amount + bonus;
    p->experience = (p->experience > NH_XP_CAP - total) ? NH_XP_CAP : p->experience + total;

    printf("%s", nh_c(NH_C_GREEN));
    if (bonus > 0)
        printf(nh_tr(NH_STR_PROG_XP_BOOSTED), total, bonus);
    else
        printf(nh_tr(NH_STR_PROG_XP_GAIN), total);
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

int nh_grant_xp(GameState *gs, int amount)
{
    return grant_xp(gs, amount, true);
}

int nh_grant_xp_flat(GameState *gs, int amount)
{
    return grant_xp(gs, amount, false);
}
