#include "world.h"

#include <stdio.h>
#include <string.h>

#include "../i18n/i18n.h"
#include "../ui/term.h"
#include "progression.h"

typedef struct
{
    const char *filename;
    const char *content;
    int encryption_level;
    int credits;
} FileDef;

typedef struct
{
    const char *name;
    const char *corporation;
    SecurityLevel security;
    int data_value;  /* crédits versés à la première compromission */
    int firewall;
    int xp;          /* expérience versée à la première compromission */
    int min_level;   /* niveau du joueur à partir duquel `scan` le révèle */
    int uplink;      /* système à compromettre d'abord, ou -1 */
    int adv_target;  /* indice dans AdvancedHackingSystem.targets, ou -1 */
    int file_count;
    FileDef files[3];
} NodeDef;

/*
 * Le graphe (relais entre parenthèses) :
 *
 *   localhost ─┬─ corp-server-01 ─┬─ nexus-mainframe ─ gov-database
 *              │                  └─ research-lab
 *              └─ underground-market ─ banking-network
 *
 * Valeurs de gains et de sécurité provisoires : l'équilibrage se fait en phase 5.
 */
static const NodeDef k_nodes[NH_MAX_NODES] = {
    {"localhost", "Independent", SECURITY_LOW, 10, 2, 5, 1, -1, -1, 1,
     {{"user_data.txt", "Données utilisateur locales", 1, 50}}},
    {"corp-server-01", "MegaCorp Industries", SECURITY_MEDIUM, 50, 5, 25, 2, 0, -1, 2,
     {{"employee_records.db", "Registres des employés", 2, 200},
      {"financial_data.xlsx", "Données financières confidentielles", 3, 500}}},
    {"nexus-mainframe", "Nexus Corp", SECURITY_HIGH, 200, 8, 100, 3, 1, 0, 3,
     {{"project_ghost.dat", "CLASSIFIED", 4, 1000},
      {"neural_maps.bin", "Cartes neurales des citoyens", 5, 1500},
      {"quantum_keys.qkey", "Clés de chiffrement quantique", 6, 2000}}},
    {"underground-market", "Underground", SECURITY_MEDIUM, 150, 4, 30, 3, 0, 4, 1,
     {{"black_ledger.dat", "Registre des ventes du marché noir", 2, 300}}},
    {"research-lab", "TechDyne Research", SECURITY_HIGH, 350, 6, 60, 4, 1, 2, 1,
     {{"prototype_specs.cad", "Plans de prototypes militaires", 4, 600}}},
    {"banking-network", "MegaCorp Financial", SECURITY_HIGH, 600, 7, 80, 4, 3, 1, 1,
     {{"vault_keys.enc", "Clés des chambres fortes", 5, 900}}},
    {"gov-database", "Gouvernement de Neo-Tokyo", SECURITY_CRITICAL, 800, 9, 110, 5, 2, 3, 1,
     {{"citizen_registry.db", "Registre de tous les citoyens", 6, 1200}}},
};

_Static_assert(sizeof k_nodes / sizeof k_nodes[0] == NH_MAX_NODES, "NH_MAX_NODES != table du monde");

int nh_world_count(void)
{
    return NH_MAX_NODES;
}

static bool valid(int idx)
{
    return idx >= 0 && idx < NH_MAX_NODES;
}

static void copy_str(char *dst, size_t size, const char *src)
{
    size_t n = strlen(src);
    if (n >= size)
        n = size - 1;
    memcpy(dst, src, n);
    dst[n] = '\0';
}

void nh_world_init(NetworkNode nodes[NH_MAX_NODES])
{
    memset(nodes, 0, NH_MAX_NODES * sizeof nodes[0]);
    for (int i = 0; i < NH_MAX_NODES; i++)
    {
        const NodeDef *d = &k_nodes[i];
        NetworkNode *n = &nodes[i];
        copy_str(n->name, sizeof n->name, d->name);
        copy_str(n->corporation, sizeof n->corporation, d->corporation);
        n->security = d->security;
        n->data_value = d->data_value;
        n->firewall_strength = d->firewall;
        n->file_count = d->file_count;
        for (int f = 0; f < d->file_count; f++)
        {
            DataFile *file = &n->secret_files[f];
            copy_str(file->filename, sizeof file->filename, d->files[f].filename);
            copy_str(file->content, sizeof file->content, d->files[f].content);
            file->encryption_level = d->files[f].encryption_level;
            file->credits_value = d->files[f].credits;
        }
    }
}

const char *nh_security_label(SecurityLevel level)
{
    switch (level)
    {
    case SECURITY_LOW:
        return nh_tr(NH_STR_WORLD_SEC_LOW);
    case SECURITY_MEDIUM:
        return nh_tr(NH_STR_WORLD_SEC_MEDIUM);
    case SECURITY_HIGH:
        return nh_tr(NH_STR_WORLD_SEC_HIGH);
    case SECURITY_CRITICAL:
        return nh_tr(NH_STR_WORLD_SEC_CRITICAL);
    }
    return "";
}

int nh_world_uplink(int idx)
{
    return valid(idx) ? k_nodes[idx].uplink : -1;
}

int nh_world_min_level(int idx)
{
    return valid(idx) ? k_nodes[idx].min_level : 0;
}

int nh_world_adv_target(int idx)
{
    return valid(idx) ? k_nodes[idx].adv_target : -1;
}

bool nh_world_reachable(const NetworkNode nodes[NH_MAX_NODES], int idx)
{
    if (!valid(idx))
        return false;
    int up = k_nodes[idx].uplink;
    return up < 0 || nodes[up].is_compromised;
}

int nh_world_discover(NetworkNode nodes[NH_MAX_NODES], int level)
{
    int fresh = 0;
    for (int i = 0; i < NH_MAX_NODES; i++)
    {
        if (!nodes[i].is_discovered && level >= k_nodes[i].min_level)
        {
            nodes[i].is_discovered = true;
            fresh++;
        }
    }
    return fresh;
}

int nh_world_find(const NetworkNode nodes[NH_MAX_NODES], const char *name)
{
    for (int i = 0; i < NH_MAX_NODES; i++)
        if (nodes[i].is_discovered && strcmp(nodes[i].name, name) == 0)
            return i;
    return -1;
}

int nh_world_resolve(const GameState *gs, const char *name, bool need_route)
{
    int idx = nh_world_find(gs->nodes, name);
    if (idx < 0)
    {
        printf(nh_tr(NH_STR_WORLD_NOT_FOUND), name);
        printf("\n");
        return -1;
    }
    if (need_route && !nh_world_reachable(gs->nodes, idx))
    {
        printf(nh_tr(NH_STR_WORLD_ROUTE_CLOSED), gs->nodes[idx].name, gs->nodes[k_nodes[idx].uplink].name);
        printf("\n");
        return -1;
    }
    return idx;
}

/* ---- Attaques ------------------------------------------------------------------------------ */

typedef struct
{
    int base;
    int per_level;
    int per_security;
    int critical_malus; /* en plus, contre un système CRITIQUE */
} Formula;

/* chance = base + niveau × per_level − sécurité × per_security − (critique ? malus : 0) + bonus */
static const Formula k_formulas[NH_HACK_METHOD_COUNT] = {
    [NH_HACK_BRUTE] = {80, 0, 15, 0},
    [NH_HACK_BACKDOOR] = {70, 10, 15, 0},
    [NH_HACK_VIRUS] = {60, 0, 10, 0},
    [NH_HACK_AI] = {85, 5, 0, 20},
    [NH_HACK_EXPLOIT] = {60, 5, 12, 0},
};

int nh_world_node_bonus(const NetworkNode *node)
{
    return node->has_intel ? NH_INTEL_BONUS : 0;
}

int nh_world_chance(NhHackMethod method, const NetworkNode *node, int level, int bonus)
{
    if ((int)method < 0 || method >= NH_HACK_METHOD_COUNT)
        return NH_CHANCE_MIN;
    const Formula *f = &k_formulas[method];
    int chance = f->base + level * f->per_level - (int)node->security * f->per_security + bonus +
                 nh_world_node_bonus(node);
    if (node->security == SECURITY_CRITICAL)
        chance -= f->critical_malus;
    if (chance < NH_CHANCE_MIN)
        return NH_CHANCE_MIN;
    if (chance > NH_CHANCE_MAX)
        return NH_CHANCE_MAX;
    return chance;
}

void nh_world_sync_tools(GameState *gs)
{
    bool any_intel = false;
    for (int i = 0; i < NH_MAX_NODES; i++)
        any_intel = any_intel || gs->nodes[i].has_intel;

    AdvancedHackingSystem *adv = &gs->advanced;
    for (int i = 0; i < adv->tool_count; i++)
    {
        CyberTool *tool = &adv->tools[i];
        switch (tool->tool)
        {
        case TOOL_QUANTUM_COMPUTER:
            tool->is_active = gs->player.has_quantum_computer;
            break;
        case TOOL_AI_ASSISTANT:
            tool->is_active = gs->player.has_ai_assistant;
            break;
        case TOOL_NEURAL_INTERFACE:
            tool->is_active = adv->neural_interface_sync >= 50;
            break;
        case TOOL_STEALTH_CLOAK:
            tool->is_active = gs->stealth_mode;
            break;
        case TOOL_VIRUS_LABORATORY:
            tool->is_active = gs->player.virus_library_size > 0;
            break;
        case TOOL_SOCIAL_PROFILE_DB:
            tool->is_active = any_intel;
            break;
        case TOOL_ZERO_DAY_EXPLOIT:
            tool->is_active = gs->player.commands_unlocked[CMD_EXPLOIT];
            break;
        case TOOL_GHOST_PROTOCOL:
            tool->is_active = gs->alert.ghost_protocols_available > 0;
            break;
        }
    }
}

int nh_world_locked_files(const NetworkNode *node)
{
    int n = 0;
    for (int i = 0; i < node->file_count; i++)
        if (!node->secret_files[i].is_unlocked)
            n++;
    return n;
}

int nh_world_extract(GameState *gs, int idx)
{
    if (!valid(idx))
        return 0;
    NetworkNode *node = &gs->nodes[idx];
    int done = 0;
    for (int i = 0; i < node->file_count; i++)
    {
        DataFile *file = &node->secret_files[i];
        if (file->is_unlocked)
            continue;
        file->is_unlocked = true;
        printf(nh_tr(NH_STR_WORLD_FILE), file->filename);
        printf("  ");
        printf(nh_tr(NH_STR_PROG_CREDITS), file->credits_value);
        printf("\n");
        gs->player.credits += file->credits_value;
        done++;
    }
    return done;
}

bool nh_world_compromise(GameState *gs, int idx, bool deep)
{
    if (!valid(idx) || gs->nodes[idx].is_compromised)
        return false;
    NetworkNode *node = &gs->nodes[idx];
    node->is_compromised = true;

    printf("\n");
    printf(nh_tr(NH_STR_WORLD_ACCESS), node->name);
    printf("\n");
    printf(nh_tr(NH_STR_WORLD_DATA), node->data_value);
    printf("\n");
    gs->player.credits += node->data_value;
    if (deep)
        nh_world_extract(gs, idx);
    nh_grant_xp(gs, k_nodes[idx].xp);
    return true;
}
