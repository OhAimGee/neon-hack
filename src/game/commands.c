#include "commands.h"

#include "../core/parse.h"
#include "../ui/hud.h"
#include "../ui/term.h"
#include "legacy_colors.h"
#include "progression.h"
#include "save.h"
#include "tutorial.h"

#include <stdio.h>
#include <string.h>

#define NH_CMD_NAME_MAX 32

/* ---- Commandes système -------------------------------------------------- */

static bool cmd_help(GameState *gs, const char *arg)
{
    (void)arg;
    nh_print_help(gs, stdout);
    return true;
}

static void status_line(const char *label, NhColor color, const char *fmt, int value)
{
    char text[32];
    snprintf(text, sizeof text, fmt, value);
    printf("%s: %s%s%s\n", label, nh_c(color), text, nh_c(NH_C_RESET));
}

static void status_text(const char *label, NhColor color, const char *text)
{
    printf("%s: %s%s%s\n", label, nh_c(color), text, nh_c(NH_C_RESET));
}

static void status_flag(const char *label, bool yes, const char *yes_text, const char *no_text,
                        NhColor yes_color, NhColor no_color)
{
    printf("%s: %s%s%s\n", label, nh_c(yes ? yes_color : no_color), yes ? yes_text : no_text,
           nh_c(NH_C_RESET));
}

static bool cmd_status(GameState *gs, const char *arg)
{
    (void)arg;
    const Player *p = &gs->player;

    printf("\n%s%s%s\n", nh_c(NH_C_BRIGHT_CYAN), nh_tr(NH_STR_STATUS_TITLE), nh_c(NH_C_RESET));
    printf("%s: %s%s%s\n", nh_tr(NH_STR_STATUS_NAME), nh_c(NH_C_BRIGHT_GREEN), p->name,
           nh_c(NH_C_RESET));
    char text[64];
    snprintf(text, sizeof text, "%d (%s)", (int)p->level, nh_level_name((int)p->level));
    status_text(nh_tr(NH_STR_STATUS_LEVEL), NH_C_YELLOW, text);
    int next_xp = nh_level_xp_required((int)p->level + 1);
    if (next_xp < 0)
        snprintf(text, sizeof text, "%d (%s)", p->experience, nh_tr(NH_STR_STATUS_XP_MAX));
    else
        snprintf(text, sizeof text, "%d/%d", p->experience, next_xp);
    status_text(nh_tr(NH_STR_STATUS_XP), NH_C_CYAN, text);
    status_line(nh_tr(NH_STR_STATUS_CREDITS), NH_C_BRIGHT_GREEN, "%d", p->credits);
    status_line(nh_tr(NH_STR_STATUS_REPUTATION), NH_C_MAGENTA, "%d", p->reputation);
    snprintf(text, sizeof text, "%d/%d", p->stealth_rating, NH_STEALTH_MAX);
    status_text(nh_tr(NH_STR_STATUS_STEALTH), NH_C_BLUE, text);

    printf("\n%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_STATUS_EQUIPMENT_TITLE), nh_c(NH_C_RESET));
    status_flag(nh_tr(NH_STR_STATUS_AI), p->has_ai_assistant, nh_tr(NH_STR_VALUE_AVAILABLE),
                nh_tr(NH_STR_VALUE_UNAVAILABLE), NH_C_GREEN, NH_C_RED);
    status_flag(nh_tr(NH_STR_STATUS_QUANTUM), p->has_quantum_computer,
                nh_tr(NH_STR_VALUE_AVAILABLE), nh_tr(NH_STR_VALUE_UNAVAILABLE), NH_C_GREEN,
                NH_C_RED);
    printf("%s: %s%d %s%s\n", nh_tr(NH_STR_STATUS_VIRUS_LIBRARY), nh_c(NH_C_CYAN),
           p->virus_library_size, nh_tr(NH_STR_STATUS_VIRUS_UNIT), nh_c(NH_C_RESET));
    status_line(nh_tr(NH_STR_STATUS_BACKDOORS), NH_C_YELLOW, "%d", p->backdoors_active);
    /* Ce qui vient de la boutique n'apparaît qu'une fois acheté : l'écran reste court en début de partie. */
    if (p->has_encryption_key)
        status_text(nh_tr(NH_STR_STATUS_KEY), NH_C_GREEN, nh_tr(NH_STR_VALUE_AVAILABLE));
    if (gs->alert.ghost_protocols_available > 0)
        status_line(nh_tr(NH_STR_STATUS_GHOST), NH_C_CYAN, "%d", gs->alert.ghost_protocols_available);
    if (p->xp_boost > 0)
        status_line(nh_tr(NH_STR_STATUS_XP_BOOST), NH_C_CYAN, "%d", p->xp_boost);

    printf("\n%s%s%s\n", nh_c(NH_C_RED), nh_tr(NH_STR_STATUS_SECURITY_TITLE), nh_c(NH_C_RESET));
    status_flag(nh_tr(NH_STR_STATUS_STEALTH_MODE), gs->stealth_mode, nh_tr(NH_STR_VALUE_ON),
                nh_tr(NH_STR_VALUE_OFF), NH_C_GREEN, NH_C_YELLOW);
    if (gs->alert.vpn_active)
        status_text(nh_tr(NH_STR_STATUS_VPN), NH_C_GREEN, nh_tr(NH_STR_VALUE_ON));
    if (gs->alert.proxy_hacks_left > 0)
        status_line(nh_tr(NH_STR_STATUS_PROXY), NH_C_GREEN, "%d", gs->alert.proxy_hacks_left);

    char bar[128];
    nh_alert_bar(bar, sizeof bar, gs->alert.level, 20);
    printf("%s: %s%s %d/100%s [%s]\n", nh_tr(NH_STR_STATUS_ALERT), nh_c(nh_alert_color(gs->alert.level)),
           bar, gs->alert.level, nh_c(NH_C_RESET), nh_tr(nh_alert_label(gs->alert.level)));
    return true;
}

/* Sauvegarde explicite : annonce toujours le résultat (la sauvegarde automatique, elle, se tait). */
static bool cmd_save(GameState *gs, const char *arg)
{
    (void)arg;
    switch (nh_save_game(gs))
    {
    case NH_SAVE_OK:
        printf("%s\n", nh_tr(NH_STR_SAVE_OK));
        return true;
    case NH_SAVE_NO_PATH:
        printf("%s\n", nh_tr(NH_STR_SAVE_UNAVAILABLE));
        return false;
    case NH_SAVE_IO:
    case NH_SAVE_MISSING:
    case NH_SAVE_CORRUPT:
    case NH_SAVE_TOO_NEW:
        break;
    }
    printf(nh_tr(NH_STR_SAVE_FAILED), gs->save_path);
    printf("\n");
    return false;
}

static bool cmd_quit(GameState *gs, const char *arg)
{
    (void)arg;
    printf("%s\n", nh_tr(NH_STR_QUIT_MESSAGE));
    if (gs->save_path[0] != '\0')
        (void)cmd_save(gs, "");
    gs->running = false;
    return true;
}

static bool cmd_clear(GameState *gs, const char *arg)
{
    (void)gs;
    (void)arg;
    if (nh_hud_active())
        nh_hud_clear_body(); /* n'efface que la zone de texte : les barres restent */
    else
        printf("\033[2J\033[H");
    print_colored_text("╔══════════════════════════════════════════════════════════════════╗\n", COLOR_CYAN);
    print_colored_text("║                           NEON HACK                             ║\n", COLOR_BRIGHT_CYAN);
    print_colored_text("╚══════════════════════════════════════════════════════════════════╝\n", COLOR_CYAN);
    return true;
}

/* ---- Table des commandes ------------------------------------------------ */

#define NO NH_NO_UNLOCK

static const NhCommand k_commands[] = {
    /* nom             alias              catégorie        déblocage         niv. caché aide                            gestionnaire  [argument] */
    {"scan",           NULL,              NH_CAT_HACK,     CMD_SCAN,         0, false, NH_STR_HELP_SCAN,           cmd_scan_network, NH_ARG_NONE},
    {"bruteforce",     NULL,              NH_CAT_HACK,     CMD_BRUTEFORCE,   0, false, NH_STR_HELP_BRUTEFORCE,     cmd_bruteforce, NH_ARG_SYSTEM},
    {"decrypt",        NULL,              NH_CAT_HACK,     CMD_DECRYPT,      0, false, NH_STR_HELP_DECRYPT,        cmd_decrypt, NH_ARG_NONE},
    {"backdoor",       NULL,              NH_CAT_HACK,     CMD_BACKDOOR,     0, false, NH_STR_HELP_BACKDOOR,       cmd_backdoor, NH_ARG_SYSTEM},
    {"traceroute",     NULL,              NH_CAT_HACK,     CMD_TRACE_ROUTE,  0, false, NH_STR_HELP_TRACEROUTE,     cmd_trace_route, NH_ARG_SYSTEM},
    {"exploit",        NULL,              NH_CAT_HACK,     CMD_EXPLOIT,      0, false, NH_STR_HELP_EXPLOIT,        cmd_exploit, NH_ARG_SYSTEM},
    {"uploadvirus",    "upload_virus",    NH_CAT_HACK,     CMD_UPLOAD_VIRUS, 0, false, NH_STR_HELP_UPLOADVIRUS,    cmd_upload_virus, NH_ARG_SYSTEM},
    {"stealth",        NULL,              NH_CAT_HACK,     NO,               0, true,  NH_STR_HELP_STEALTH,        cmd_stealth_mode, NH_ARG_NONE},
    {"aihack",         "ai_hack",         NH_CAT_HACK,     CMD_AI_HACK,      0, false, NH_STR_HELP_AIHACK,         cmd_ai_hack, NH_ARG_SYSTEM},
    {"quantumdecrypt", "quantum_decrypt", NH_CAT_HACK,     CMD_QUANTUM_DECRYPT, 0, false, NH_STR_HELP_QUANTUMDECRYPT, cmd_quantum_decrypt, NH_ARG_NONE},

    {"shop",           NULL,              NH_CAT_WORLD,    NO,               0, false, NH_STR_HELP_SHOP,           cmd_shop, NH_ARG_NONE},
    {"laylow",         NULL,              NH_CAT_WORLD,    NO,               0, false, NH_STR_HELP_LAYLOW,         cmd_lay_low, NH_ARG_NONE},
    {"quests",         NULL,              NH_CAT_WORLD,    NO,               0, false, NH_STR_HELP_QUESTS,         cmd_quests, NH_ARG_NONE},
    {"contacts",       NULL,              NH_CAT_WORLD,    NO,               0, false, NH_STR_HELP_CONTACTS,       cmd_contacts, NH_ARG_NONE},
    {"contact",        NULL,              NH_CAT_WORLD,    NO,               0, false, NH_STR_HELP_CONTACT,        cmd_interact_contact, NH_ARG_CONTACT},
    {"messages",       NULL,              NH_CAT_WORLD,    NO,               0, false, NH_STR_HELP_MESSAGES,       cmd_messages, NH_ARG_NONE},
    {"read",           NULL,              NH_CAT_WORLD,    NO,               0, false, NH_STR_HELP_READ,           cmd_read, NH_ARG_MESSAGE},

    {"advhack",        NULL,              NH_CAT_ADVANCED, NO,               3, false, NH_STR_HELP_ADVHACK,        cmd_advanced_hack, NH_ARG_SYSTEM},
    {"aiassist",       NULL,              NH_CAT_ADVANCED, NO,               3, false, NH_STR_HELP_AIASSIST,       cmd_ai_assist_hack, NH_ARG_SYSTEM},
    {"socialeng",      NULL,              NH_CAT_ADVANCED, NO,               2, false, NH_STR_HELP_SOCIALENG,      cmd_social_engineer, NH_ARG_SYSTEM},
    {"stealthmode",    NULL,              NH_CAT_ADVANCED, NO,               0, false, NH_STR_HELP_STEALTHMODE,    cmd_stealth_mode_toggle, NH_ARG_NONE},
    {"analyzedefenses", NULL,             NH_CAT_ADVANCED, NO,               0, false, NH_STR_HELP_ANALYZEDEFENSES, cmd_analyze_defenses, NH_ARG_SYSTEM},
    {"neuralsync",     NULL,              NH_CAT_ADVANCED, NO,               5, false, NH_STR_HELP_NEURALSYNC,     cmd_neural_sync, NH_ARG_NONE},
    {"temporalhack",   NULL,              NH_CAT_ADVANCED, NO,               6, false, NH_STR_HELP_TEMPORALHACK,   cmd_temporal_hack, NH_ARG_SYSTEM},

    {"status",         NULL,              NH_CAT_SYSTEM,   NO,               0, false, NH_STR_HELP_STATUS,         cmd_status, NH_ARG_NONE},
    {"help",           NULL,              NH_CAT_SYSTEM,   NO,               0, false, NH_STR_HELP_HELP,           cmd_help, NH_ARG_NONE},
    {"save",           NULL,              NH_CAT_SYSTEM,   NO,               0, false, NH_STR_HELP_SAVE,           cmd_save, NH_ARG_NONE},
    {"quit",           "exit",            NH_CAT_SYSTEM,   NO,               0, false, NH_STR_HELP_QUIT,           cmd_quit, NH_ARG_NONE},
    {"clear",          NULL,              NH_CAT_SYSTEM,   NO,               0, false, NH_STR_HELP_CLEAR,          cmd_clear, NH_ARG_NONE},
};

#undef NO

const NhCommand *nh_commands(size_t *count)
{
    if (count != NULL)
        *count = sizeof k_commands / sizeof k_commands[0];
    return k_commands;
}

const NhCommand *nh_find_command(const char *name)
{
    size_t count;
    const NhCommand *table = nh_commands(&count);

    for (size_t i = 0; i < count; i++)
    {
        if (nh_str_eq_nocase(name, table[i].name) ||
            (table[i].alias != NULL && nh_str_eq_nocase(name, table[i].alias)))
        {
            return &table[i];
        }
    }
    return NULL;
}

bool nh_command_available(const GameState *gs, const NhCommand *cmd)
{
    if ((int)gs->player.level < cmd->min_level)
        return false;
    if (cmd->unlock != NH_NO_UNLOCK && !gs->player.commands_unlocked[cmd->unlock])
        return false;
    return true;
}

NhDispatch nh_dispatch(GameState *gs, const char *line)
{
    char name[NH_CMD_NAME_MAX];
    char arg[MAX_INPUT_LENGTH];

    if (!nh_split_command(line, name, sizeof name, arg, sizeof arg))
        return NH_DISPATCH_EMPTY;

    const NhCommand *cmd = nh_find_command(name);
    if (cmd == NULL)
    {
        printf(nh_tr(NH_STR_UNKNOWN_COMMAND), name);
        printf("\n%s\n", nh_tr(NH_STR_HINT_HELP));
        nh_tutorial_on_command(gs, NULL, NH_DISPATCH_UNKNOWN);
        return NH_DISPATCH_UNKNOWN;
    }

    if (!nh_command_available(gs, cmd))
    {
        if ((int)gs->player.level < cmd->min_level)
            printf(nh_tr(NH_STR_CMD_NEEDS_LEVEL), cmd->min_level);
        else
            printf("%s", nh_tr(NH_STR_CMD_LOCKED));
        printf("\n");
        nh_tutorial_on_command(gs, cmd->name, NH_DISPATCH_LOCKED);
        return NH_DISPATCH_LOCKED;
    }

    /* Le temps passe à chaque action de hacking : l'alerte se refroidit un peu avant qu'elle
     * n'ajoute la sienne, et un proxy acheté en boutique s'use d'un hack. Consulter la boutique ou
     * l'aide ne fait pas baisser l'alerte. */
    bool hacking = cmd->category == NH_CAT_HACK || cmd->category == NH_CAT_ADVANCED;
    if (hacking)
        nh_alert_decay(&gs->alert);

    NhDispatch result = cmd->fn(gs, arg) ? NH_DISPATCH_OK : NH_DISPATCH_FAILED;
    if (hacking)
        nh_alert_end_action(&gs->alert);
    nh_tutorial_on_command(gs, cmd->name, result); /* la mission d'ECHO-7 suit ce que le joueur vient de faire */

    /* Puis les événements que la commande a provoqués (et le temps qui passe) sont livrés aux quêtes
     * et aux contacts : leurs annonces s'affichent après le résultat, pas au milieu. Une partie
     * finie (quit, game over) n'a plus personne à prévenir. */
    nh_event(gs, NH_EV_COMMAND, (int)result);
    if (gs->running && !gs->player.game_over && !nh_alert_is_game_over(&gs->alert))
        nh_events_flush(gs);
    else
        nh_events_clear(gs);
    return result;
}

/* ---- Aide --------------------------------------------------------------- */

static const struct
{
    NhStr title;
    NhColor color;
} k_categories[NH_CAT_COUNT] = {
    [NH_CAT_HACK] = {NH_STR_HELP_CAT_HACK, NH_C_YELLOW},
    [NH_CAT_WORLD] = {NH_STR_HELP_CAT_WORLD, NH_C_MAGENTA},
    [NH_CAT_ADVANCED] = {NH_STR_HELP_CAT_ADVANCED, NH_C_BRIGHT_CYAN},
    [NH_CAT_SYSTEM] = {NH_STR_HELP_CAT_SYSTEM, NH_C_YELLOW},
};

void nh_print_help(const GameState *gs, FILE *out)
{
    size_t count;
    const NhCommand *table = nh_commands(&count);

    fprintf(out, "\n%s%s%s\n", nh_c(NH_C_BRIGHT_CYAN), nh_tr(NH_STR_HELP_TITLE), nh_c(NH_C_RESET));

    for (int cat = 0; cat < NH_CAT_COUNT; cat++)
    {
        bool header_printed = false;

        for (size_t i = 0; i < count; i++)
        {
            const NhCommand *cmd = &table[i];
            if ((int)cmd->category != cat || cmd->hidden || !nh_command_available(gs, cmd))
                continue;

            if (!header_printed)
            {
                fprintf(out, "\n%s%s%s\n", nh_c(k_categories[cat].color),
                        nh_tr(k_categories[cat].title), nh_c(NH_C_RESET));
                header_printed = true;
            }
            fprintf(out, "  %-16s- %s\n", cmd->name, nh_tr(cmd->help));
        }
    }
}

void nh_refresh_hud(const GameState *gs)
{
    if (!nh_hud_active())
        return;

    /* `help` d'abord (jamais coupée), puis les commandes utilisables, dans l'ordre de la table. */
    enum { MAX_NAMES = 64 };
    const char *names[MAX_NAMES];
    size_t n = 0;
    names[n++] = "help";

    size_t count;
    const NhCommand *table = nh_commands(&count);
    for (size_t i = 0; i < count && n < MAX_NAMES; i++)
    {
        if (table[i].hidden || table[i].category == NH_CAT_SYSTEM ||
            !nh_command_available(gs, &table[i]))
            continue;
        names[n++] = table[i].name;
    }

    NhHudData data = {
        .name = gs->player.name,
        .level = (int)gs->player.level,
        .credits = gs->player.credits,
        .alert = gs->alert.level,
        .alert_color = nh_alert_color(gs->alert.level),
        .stealth = gs->stealth_mode,
        .commands = names,
        .command_count = n,
    };
    nh_hud_refresh(&data);
}
