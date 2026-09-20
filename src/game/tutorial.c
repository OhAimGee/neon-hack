#include "tutorial.h"

#include <stdio.h>
#include <string.h>

#include "../i18n/i18n.h"
#include "../ui/term.h"
#include "world.h"

typedef struct
{
    NhStr objective; /* intitulé court, affiché dans le journal de mission */
    NhStr say;       /* consigne d'ECHO-7 quand l'étape commence */
} StepText;

static const StepText k_steps[NH_TUT_STEP_COUNT] = {
    [NH_TUT_QUESTS] = {NH_STR_TUT_OBJ_QUESTS, NH_STR_TUT_SAY_QUESTS},
    [NH_TUT_HELP] = {NH_STR_TUT_OBJ_HELP, NH_STR_TUT_SAY_HELP},
    [NH_TUT_SCAN] = {NH_STR_TUT_OBJ_SCAN, NH_STR_TUT_SAY_SCAN},
    [NH_TUT_STATUS] = {NH_STR_TUT_OBJ_STATUS, NH_STR_TUT_SAY_STATUS},
    [NH_TUT_LEVELUP] = {NH_STR_TUT_OBJ_LEVELUP, NH_STR_TUT_SAY_LEVELUP},
    [NH_TUT_BRUTEFORCE] = {NH_STR_TUT_OBJ_BRUTEFORCE, NH_STR_TUT_SAY_BRUTEFORCE},
    [NH_TUT_LAYLOW] = {NH_STR_TUT_OBJ_LAYLOW, NH_STR_TUT_SAY_LAYLOW},
};

void nh_echo_say(const char *text, bool newline, unsigned delay_ms)
{
    nh_speak(NH_ECHO7, NH_C_BRIGHT_CYAN, text, newline, delay_ms);
}

bool nh_tutorial_active(const GameState *gs) { return gs->tutorial.step != NH_TUT_NONE; }

/* ---- Cycle de vie ---------------------------------------------------------------------------- */

void nh_tutorial_start(GameState *gs)
{
    gs->tutorial.step = NH_TUT_QUESTS;
    gs->tutorial.done = false;
}

void nh_tutorial_skip(GameState *gs)
{
    gs->tutorial.step = NH_TUT_NONE;
    gs->tutorial.done = true;
    nh_quest_complete(gs, QUEST_INTRO_TUTORIAL, false); /* close en silence : sans récompense */
}

void nh_tutorial_announce(const GameState *gs, bool resumed)
{
    if (!nh_tutorial_active(gs))
        return;

    if (resumed)
    {
        char line[256];
        snprintf(line, sizeof line, nh_tr(NH_STR_TUT_RESUME), gs->player.name);
        printf("\n");
        nh_echo_say(line, true, 0);
    }
    printf("\n");
    nh_echo_say(nh_tr(k_steps[gs->tutorial.step].say), true, 0);
    printf("\n");
}

void nh_tutorial_repeat(const GameState *gs)
{
    if (!nh_tutorial_active(gs))
        return;
    nh_echo_say(nh_tr(k_steps[gs->tutorial.step].say), true, 0);
}

void nh_tutorial_print_mission(const GameState *gs)
{
    int current = gs->tutorial.step;

    printf("\n%s%s%s\n", nh_c(NH_C_BRIGHT_CYAN), nh_tr(NH_STR_TUT_LOG_TITLE), nh_c(NH_C_RESET));
    printf("%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_TUT_MISSION_TITLE), nh_c(NH_C_RESET));
    printf("%s: %s\n\n", nh_tr(NH_STR_TUT_LOG_CONTACT), NH_ECHO7);

    for (int step = NH_TUT_QUESTS; step < NH_TUT_STEP_COUNT; step++)
    {
        const char *mark = step < current ? "[x]" : (step == current ? "[>]" : "[ ]");
        NhColor color = step < current ? NH_C_GREEN : (step == current ? NH_C_BRIGHT_CYAN : NH_C_RESET);
        printf("  %s%s %s%s\n", nh_c(color), mark, nh_tr(k_steps[step].objective), nh_c(NH_C_RESET));
    }

    printf("\n");
    printf(nh_tr(NH_STR_TUT_LOG_REWARD), NH_TUT_REWARD_CREDITS, NH_TUT_REWARD_REPUTATION);
    printf("\n");
}

/* ---- Étapes ---------------------------------------------------------------------------------- */

static bool is_command(const char *command, const char *name)
{
    return command != NULL && strcmp(command, name) == 0;
}

/*
 * L'étape `step` est-elle accomplie ? Les étapes « lance telle commande » ne se valident que par
 * la commande qui vient de tourner (`command`, réussie) ; les autres se lisent dans l'état du jeu,
 * ce qui les rend indifférentes à l'ordre dans lequel le joueur a exploré.
 */
static bool step_satisfied(const GameState *gs, int step, const char *command, bool ok)
{
    switch (step)
    {
    case NH_TUT_QUESTS:
        return ok && is_command(command, "quests");
    case NH_TUT_HELP:
        return ok && is_command(command, "help");
    case NH_TUT_SCAN:
        return ok && is_command(command, "scan");
    case NH_TUT_STATUS:
        return ok && is_command(command, "status");
    case NH_TUT_LEVELUP:
        return (int)gs->player.level >= (int)LEVEL_APPRENTICE;
    case NH_TUT_BRUTEFORCE:
    {
        int idx = nh_world_find(gs->nodes, "localhost");
        return idx >= 0 && gs->nodes[idx].is_compromised;
    }
    case NH_TUT_LAYLOW:
        /* Une méthode a réellement été appliquée : ouvrir le menu puis annuler ne suffit pas. */
        return gs->alert.reductions_done > 0;
    default:
        return false;
    }
}

static void say_step(int step)
{
    printf("\n");
    nh_echo_say(nh_tr(k_steps[step].say), true, 0);
}

static void finish(GameState *gs)
{
    gs->tutorial.step = NH_TUT_NONE;
    gs->tutorial.done = true;

    char line[256];
    printf("\n");
    snprintf(line, sizeof line, nh_tr(NH_STR_TUT_DONE_1), gs->player.name);
    nh_echo_say(line, true, 0);
    nh_echo_say(nh_tr(NH_STR_TUT_DONE_2), true, 0);
    nh_echo_say(nh_tr(NH_STR_TUT_DONE_3), true, 0);

    /* La mission est la première quête du journal : la terminer l'annonce, verse la récompense
     * (NH_TUT_REWARD_*, une seule fois) et débloque la suite. */
    nh_quest_complete(gs, QUEST_INTRO_TUTORIAL, true);
}

/* Valide l'étape en cours, puis annonce la suivante (ou termine la mission). */
static void complete_step(GameState *gs)
{
    int step = gs->tutorial.step;

    printf("\n%s", nh_c(NH_C_GREEN));
    printf(nh_tr(NH_STR_TUT_STEP_DONE), nh_tr(k_steps[step].objective));
    printf("%s\n", nh_c(NH_C_RESET));

    if (step + 1 >= NH_TUT_STEP_COUNT)
    {
        finish(gs);
        return;
    }
    gs->tutorial.step = step + 1;
    /* Une étape déjà remplie par l'état du jeu se valide toute seule : pas de consigne inutile. */
    if (!step_satisfied(gs, gs->tutorial.step, NULL, false))
        say_step(gs->tutorial.step);
}

static void remind(const GameState *gs)
{
    char line[320];
    snprintf(line, sizeof line, nh_tr(NH_STR_TUT_REMINDER), nh_tr(k_steps[gs->tutorial.step].objective));
    printf("\n");
    nh_echo_say(line, true, 0);
}

void nh_tutorial_on_command(GameState *gs, const char *command, NhDispatch result)
{
    if (!nh_tutorial_active(gs) || !gs->running || gs->player.game_over ||
        nh_alert_is_game_over(&gs->alert))
        return;

    bool progressed = false;
    while (nh_tutorial_active(gs) &&
           step_satisfied(gs, gs->tutorial.step, command, result == NH_DISPATCH_OK))
    {
        complete_step(gs);
        command = NULL; /* une commande ne valide qu'une seule étape */
        progressed = true;
    }

    bool confused = result == NH_DISPATCH_UNKNOWN || result == NH_DISPATCH_LOCKED ||
                    result == NH_DISPATCH_FAILED;
    if (!progressed && confused && nh_tutorial_active(gs))
        remind(gs);
}
