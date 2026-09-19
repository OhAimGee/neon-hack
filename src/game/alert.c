#include "alert.h"

#include <stdio.h>
#include <string.h>

#include "../i18n/i18n.h"
#include "../ui/term.h"

static int clamp_level(int v)
{
    if (v < 0)
        return 0;
    if (v > NH_ALERT_MAX)
        return NH_ALERT_MAX;
    return v;
}

void nh_alert_init(AlertSystem *a)
{
    memset(a, 0, sizeof *a);
}

int nh_alert_add(AlertSystem *a, int amount)
{
    if (amount <= 0)
        return 0;
    int before = a->level;
    /* Comparaison avant l'addition : pas de dépassement d'entier sur un gros montant. */
    a->level = (amount >= NH_ALERT_MAX - before) ? NH_ALERT_MAX : clamp_level(before + amount);
    if (a->level > a->max_level)
        a->max_level = a->level;
    return a->level - before;
}

int nh_alert_reduce(AlertSystem *a, int amount)
{
    if (amount <= 0)
        return 0;
    int before = a->level;
    a->level = (amount >= before) ? 0 : before - amount;
    return before - a->level;
}

int nh_alert_decay(AlertSystem *a)
{
    return nh_alert_reduce(a, 1 + (a->vpn_active ? 1 : 0) + (a->proxy_active ? 1 : 0));
}

NhAlertBand nh_alert_band(int level)
{
    if (level < NH_ALERT_WARNING)
        return NH_ALERT_SAFE;
    if (level < NH_ALERT_DANGER)
        return NH_ALERT_WARN;
    return NH_ALERT_DANGER_BAND;
}

bool nh_alert_is_game_over(const AlertSystem *a)
{
    return a->level >= NH_ALERT_MAX;
}

bool nh_alert_shop_closed(const AlertSystem *a)
{
    return a->level >= NH_ALERT_CRITICAL;
}

int nh_alert_success_penalty(const AlertSystem *a)
{
    if (a->level >= NH_ALERT_CRITICAL)
        return 20;
    if (a->level >= NH_ALERT_ELEVATED)
        return 10;
    if (a->level >= NH_ALERT_WARNING)
        return 5;
    return 0;
}

/* Coûts en crédits et baisses d'alerte (à équilibrer en phase 5, avec une simulation). */
static const struct
{
    int cost;
    int amount;
} k_reductions[NH_REDUCTION_COUNT] = {
    [NH_REDUCTION_TIME] = {0, 8},   [NH_REDUCTION_VPN] = {20, 10},
    [NH_REDUCTION_PROXY] = {30, 15}, [NH_REDUCTION_GHOST] = {0, 30},
    [NH_REDUCTION_LAYLOW] = {40, 25}, [NH_REDUCTION_FRAME] = {60, 35},
};

int nh_alert_reduction_cost(NhReduction method)
{
    return k_reductions[method].cost;
}

int nh_alert_reduction_amount(NhReduction method)
{
    return k_reductions[method].amount;
}

NhReduceResult nh_alert_apply_reduction(AlertSystem *a, NhReduction method, int *credits,
                                        int *applied)
{
    int cost = k_reductions[method].cost;

    if (method == NH_REDUCTION_GHOST && a->ghost_protocols_available <= 0)
        return NH_REDUCE_NO_GHOST;
    if (*credits < cost)
        return NH_REDUCE_NO_CREDITS;

    *credits -= cost;
    if (method == NH_REDUCTION_GHOST)
        a->ghost_protocols_available--;
    else if (method == NH_REDUCTION_VPN)
        a->vpn_active = true;
    else if (method == NH_REDUCTION_PROXY)
        a->proxy_active = true;

    int done = nh_alert_reduce(a, k_reductions[method].amount);
    if (applied != NULL)
        *applied = done;
    return NH_REDUCE_OK;
}

void nh_alert_bar(char *out, size_t out_size, int level, int width)
{
    if (out_size == 0)
        return;
    out[0] = '\0';
    if (width < 1)
        return;

    int lvl = clamp_level(level);
    int filled = (lvl * width + NH_ALERT_MAX - 1) / NH_ALERT_MAX;
    size_t used = 0;
    for (int i = 0; i < width; i++)
    {
        const char *cell = (i < filled) ? "█" : "░";
        size_t n = strlen(cell);
        if (used + n + 1 > out_size)
            break;
        memcpy(out + used, cell, n);
        used += n;
    }
    out[used] = '\0';
}

NhColor nh_alert_color(int level)
{
    switch (nh_alert_band(level))
    {
    case NH_ALERT_SAFE:
        return NH_C_GREEN;
    case NH_ALERT_WARN:
        return NH_C_YELLOW;
    case NH_ALERT_DANGER_BAND:
        break;
    }
    return NH_C_RED;
}

NhStr nh_alert_label(int level)
{
    switch (nh_alert_band(level))
    {
    case NH_ALERT_SAFE:
        return NH_STR_ALERT_SAFE;
    case NH_ALERT_WARN:
        return NH_STR_ALERT_WARNING;
    case NH_ALERT_DANGER_BAND:
        break;
    }
    return NH_STR_ALERT_DANGER;
}

void nh_alert_raise(AlertSystem *a, int amount)
{
    int applied = nh_alert_add(a, amount);
    if (applied <= 0)
        return;

    printf("%s", nh_c(NH_C_RED));
    printf(nh_tr(NH_STR_ALERT_RAISED), applied);
    printf("%s ", nh_c(NH_C_RESET));

    if (a->level >= NH_ALERT_CRITICAL)
        printf("%s%s%s ", nh_c(NH_C_RED), nh_tr(NH_STR_ALERT_TAG_CRITICAL), nh_c(NH_C_RESET));
    else if (a->level >= NH_ALERT_ELEVATED)
        printf("%s%s%s ", nh_c(NH_C_YELLOW), nh_tr(NH_STR_ALERT_TAG_ELEVATED), nh_c(NH_C_RESET));
}

static void print_protection(const char *label, bool on)
{
    printf("  %s: %s%s%s\n", label, nh_c(on ? NH_C_GREEN : NH_C_RED),
           nh_tr(on ? NH_STR_VALUE_ON : NH_STR_VALUE_OFF), nh_c(NH_C_RESET));
}

void nh_alert_print_status(const AlertSystem *a)
{
    char bar[128];
    nh_alert_bar(bar, sizeof bar, a->level, 20);
    NhColor color = nh_alert_color(a->level);

    printf("\n%s=== %s ===%s\n", nh_c(NH_C_CYAN), nh_tr(NH_STR_ALERT_TITLE), nh_c(NH_C_RESET));
    printf("%s%s %d/100%s [%s]\n", nh_c(color), bar, a->level, nh_c(NH_C_RESET),
           nh_tr(nh_alert_label(a->level)));
    printf(nh_tr(NH_STR_ALERT_MAX_REACHED), a->max_level);
    printf("\n%s\n", nh_tr(NH_STR_ALERT_PROTECTION));
    print_protection("VPN", a->vpn_active);
    print_protection("Proxy", a->proxy_active);
    printf("  Ghost Protocol: %d\n", a->ghost_protocols_available);

    if (a->level >= NH_ALERT_WARNING)
    {
        NhStr consequence = NH_STR_ALERT_CONSEQUENCE_WARNING;
        if (a->level >= NH_ALERT_CRITICAL)
            consequence = NH_STR_ALERT_CONSEQUENCE_CRITICAL;
        else if (a->level >= NH_ALERT_DANGER)
            consequence = NH_STR_ALERT_CONSEQUENCE_DANGER;
        printf("%s%s%s\n", nh_c(color), nh_tr(consequence), nh_c(NH_C_RESET));
    }
}

static const NhStr k_method_names[NH_REDUCTION_COUNT] = {
    [NH_REDUCTION_TIME] = NH_STR_ALERT_METHOD_TIME,   [NH_REDUCTION_VPN] = NH_STR_ALERT_METHOD_VPN,
    [NH_REDUCTION_PROXY] = NH_STR_ALERT_METHOD_PROXY, [NH_REDUCTION_GHOST] = NH_STR_ALERT_METHOD_GHOST,
    [NH_REDUCTION_LAYLOW] = NH_STR_ALERT_METHOD_LAYLOW, [NH_REDUCTION_FRAME] = NH_STR_ALERT_METHOD_FRAME,
};

void nh_alert_print_menu(const AlertSystem *a)
{
    printf("\n%s=== %s ===%s\n\n", nh_c(NH_C_MAGENTA), nh_tr(NH_STR_ALERT_MENU_TITLE),
           nh_c(NH_C_RESET));

    for (int m = 0; m < NH_REDUCTION_COUNT; m++)
    {
        const char *name = nh_tr(k_method_names[m]);
        int cost = nh_alert_reduction_cost((NhReduction)m);
        int amount = nh_alert_reduction_amount((NhReduction)m);
        bool unavailable = (m == NH_REDUCTION_GHOST && a->ghost_protocols_available <= 0);

        printf("%s%d.%s ", nh_c(unavailable ? NH_C_RED : NH_C_CYAN), m + 1, nh_c(NH_C_RESET));
        if (unavailable)
            printf(nh_tr(NH_STR_ALERT_MENU_UNAVAILABLE), name);
        else if (cost == 0)
            printf(nh_tr(NH_STR_ALERT_MENU_FREE), name, amount);
        else
            printf(nh_tr(NH_STR_ALERT_MENU_PAID), name, cost, amount);
        printf("\n");
    }
    printf("%s0.%s %s\n", nh_c(NH_C_YELLOW), nh_c(NH_C_RESET), nh_tr(NH_STR_ALERT_MENU_BACK));
}
