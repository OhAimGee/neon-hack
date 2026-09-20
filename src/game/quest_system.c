#include "quest_system.h"

#include <stdio.h>
#include <string.h>

#include "../core/io.h"
#include "../ui/term.h"
#include "alert.h"
#include "game.h"
#include "progression.h"
#include "shop.h"
#include "tutorial.h"
#include "world.h"

/* ---- Les quêtes ------------------------------------------------------------------------------ */

/*
 * Les objectifs sont ceux du jeu d'origine, remis en accord avec le monde tel qu'il est : « extraire
 * 3 fichiers » au niveau 2 était impossible (aucune commande n'extrait avant le niveau 3) et
 * « TechDyne-Server » n'existe plus dans le monde unifié. Récompenses provisoires (phase 5) ; elles
 * sont à l'échelle de la courbe d'expérience (nh_level_xp_required), pas de celle d'origine.
 *
 * Attention aux impasses : la quête « Baptême du Feu » verse 25 XP parce qu'un joueur de niveau 2
 * n'a que 45 XP à gagner (scans 15 + localhost 5 + corp-server-01 25) pour 60 requis au niveau 3.
 * test_quests.c vérifie qu'à chaque niveau il reste de quoi atteindre le suivant.
 */
#define ITEM_MASK(item) (1 << (item))

static const NhQuestDef k_quests[QUEST_COUNT] = {
    [QUEST_INTRO_TUTORIAL] =
        {
            .title = NH_STR_TUT_MISSION_TITLE,
            .description = NH_STR_QT_TUTORIAL_DESC,
            .lore = NH_STR_QT_TUTORIAL_LORE,
            .location = NH_STR_QT_TUTORIAL_LOC,
            .contact = CONTACT_ECHO7,
            .level_required = 1,
            .prerequisites = {-1, -1},
            .chapter = 1,
            .objective_count = 1,
            .objectives = {{NH_OBJ_MANUAL, 1, -1, NH_STR_QT_TUTORIAL_OBJ, false}},
            .xp = 0,
            .credits = NH_TUT_REWARD_CREDITS,
            .reputation = NH_TUT_REWARD_REPUTATION,
        },
    [QUEST_FIRST_INFILTRATION] =
        {
            .title = NH_STR_QT_INFIL_TITLE,
            .description = NH_STR_QT_INFIL_DESC,
            .lore = NH_STR_QT_INFIL_LORE,
            .location = NH_STR_QT_INFIL_LOC,
            .contact = CONTACT_ECHO7,
            .level_required = 2,
            .prerequisites = {QUEST_INTRO_TUTORIAL, -1},
            .chapter = 1,
            .objective_count = 2,
            .objectives =
                {
                    {NH_OBJ_HACK_TARGET, 1, NH_NODE_CORP_SERVER, NH_STR_QT_INFIL_OBJ_HACK, false},
                    {NH_OBJ_KEEP_ALERT_BELOW, 1, NH_ALERT_ELEVATED, NH_STR_QT_INFIL_OBJ_ALERT, false},
                },
            .xp = 25,
            .credits = 200,
            .reputation = 25,
        },
    [QUEST_GATHER_INTEL] =
        {
            .title = NH_STR_QT_INTEL_TITLE,
            .description = NH_STR_QT_INTEL_DESC,
            .lore = NH_STR_QT_INTEL_LORE,
            .location = NH_STR_QT_INTEL_LOC,
            .contact = CONTACT_R4Z0R,
            .level_required = 3,
            .prerequisites = {QUEST_FIRST_INFILTRATION, -1},
            .chapter = 2,
            .objective_count = 5,
            .objectives =
                {
                    {NH_OBJ_MEET_CONTACT, 1, CONTACT_R4Z0R, NH_STR_QT_INTEL_OBJ_CONTACT, false},
                    {NH_OBJ_PURCHASE_ITEM, 1, ITEM_MASK(ITEM_STEALTH_UPGRADE), NH_STR_QT_INTEL_OBJ_ITEM, false},
                    {NH_OBJ_HACK_TARGET, 3, -1, NH_STR_QT_INTEL_OBJ_SYSTEMS, false},
                    {NH_OBJ_BUILD_REPUTATION, 50, -1, NH_STR_QT_INTEL_OBJ_REP, false},
                    {NH_OBJ_DECRYPT_MESSAGE, 1, NH_MS_DECRYPT_TEST, NH_STR_QT_INTEL_OBJ_DECRYPT, true},
                },
            .xp = 40,
            .credits = 300,
            .reputation = 40,
        },
    [QUEST_NEXUS_DATA_BREACH] =
        {
            .title = NH_STR_QT_NEXUS_TITLE,
            .description = NH_STR_QT_NEXUS_DESC,
            .lore = NH_STR_QT_NEXUS_LORE,
            .location = NH_STR_QT_NEXUS_LOC,
            .contact = CONTACT_PHOENIX,
            .level_required = 4,
            .prerequisites = {QUEST_GATHER_INTEL, -1},
            .chapter = 3,
            .objective_count = 3,
            .objectives =
                {
                    {NH_OBJ_PURCHASE_ITEM, 1,
                     ITEM_MASK(ITEM_ENCRYPTION_KEY) | ITEM_MASK(ITEM_AI_MODULE) | ITEM_MASK(ITEM_QUANTUM_CHIP),
                     NH_STR_QT_NEXUS_OBJ_ITEM, false},
                    {NH_OBJ_HACK_TARGET, 1, NH_NODE_NEXUS, NH_STR_QT_NEXUS_OBJ_BREACH, false},
                    {NH_OBJ_GATHER_DATA, 1, NH_NODE_NEXUS, NH_STR_QT_NEXUS_OBJ_DATA, false},
                },
            .xp = 80,
            .credits = 1000,
            .reputation = 100,
        },
};

/* Les chapitres : un titre et deux lignes de présentation, annoncés quand une quête du chapitre démarre. */
static const struct
{
    NhStr title;
    NhStr text;
} k_chapters[NH_CHAPTER_COUNT] = {
    {NH_STR_CHAPTER_1_TITLE, NH_STR_CHAPTER_1_TEXT},
    {NH_STR_CHAPTER_2_TITLE, NH_STR_CHAPTER_2_TEXT},
    {NH_STR_CHAPTER_3_TITLE, NH_STR_CHAPTER_3_TEXT},
    {NH_STR_CHAPTER_4_TITLE, NH_STR_CHAPTER_4_TEXT},
};

const NhQuestDef *nh_quest_def(QuestType quest)
{
    if ((int)quest < 0 || quest >= QUEST_COUNT)
        return NULL;
    return &k_quests[quest];
}

static bool defined(QuestType quest) { return k_quests[quest].objective_count > 0; }

/* ---- État ------------------------------------------------------------------------------------ */

void nh_quests_init(QuestSystem *qs)
{
    memset(qs, 0, sizeof *qs);
    qs->quests[QUEST_INTRO_TUTORIAL].status = QUEST_STATUS_ACTIVE;
}

int nh_quests_count(const QuestSystem *qs, QuestStatus status)
{
    int n = 0;
    for (int q = 0; q < QUEST_COUNT; q++)
        if (qs->quests[q].status == status)
            n++;
    return n;
}

int nh_quests_percent(const QuestSystem *qs)
{
    return nh_quests_count(qs, QUEST_STATUS_COMPLETED) * 100 / QUEST_COUNT;
}

int nh_quests_chapter(const QuestSystem *qs)
{
    int chapter = 0;
    for (int q = 0; q < QUEST_COUNT; q++)
        if (defined((QuestType)q) && qs->quests[q].status != QUEST_STATUS_LOCKED &&
            k_quests[q].chapter > chapter)
            chapter = k_quests[q].chapter;
    return chapter;
}

bool nh_quest_objective_done(const QuestSystem *qs, QuestType quest, int objective)
{
    const NhQuestDef *def = nh_quest_def(quest);
    if (def == NULL || objective < 0 || objective >= def->objective_count)
        return false;
    return qs->quests[quest].progress[objective] >= def->objectives[objective].target;
}

/* ---- Lecture de l'état du jeu ---------------------------------------------------------------- */

static int compromised_count(const GameState *gs)
{
    int n = 0;
    for (int i = 0; i < NH_MAX_NODES; i++)
        if (gs->nodes[i].is_compromised)
            n++;
    return n;
}

/* Fichiers extraits : ceux du système `node`, ou de tous s'il est négatif. */
static int extracted_files(const GameState *gs, int node)
{
    int n = 0;
    for (int i = 0; i < NH_MAX_NODES; i++)
    {
        if (node >= 0 && i != node)
            continue;
        for (int f = 0; f < gs->nodes[i].file_count; f++)
            if (gs->nodes[i].secret_files[f].is_unlocked)
                n++;
    }
    return n;
}

/* La valeur actuelle de l'objectif d'après l'état du jeu ; faux s'il ne se mesure pas. */
static bool measure(const GameState *gs, const NhObjectiveDef *o, int *out)
{
    switch (o->kind)
    {
    case NH_OBJ_MANUAL:
        return false;
    case NH_OBJ_REACH_LEVEL:
        *out = (int)gs->player.level;
        return true;
    case NH_OBJ_BUILD_REPUTATION:
        *out = gs->player.reputation > 0 ? gs->player.reputation : 0;
        return true;
    case NH_OBJ_HACK_TARGET:
        if (o->arg < 0)
            *out = compromised_count(gs);
        else
            *out = o->arg < NH_MAX_NODES && gs->nodes[o->arg].is_compromised ? 1 : 0;
        return true;
    case NH_OBJ_GATHER_DATA:
        *out = extracted_files(gs, o->arg);
        return true;
    case NH_OBJ_MEET_CONTACT:
        *out = o->arg >= 0 && o->arg < CONTACT_COUNT && gs->contacts.contacts[o->arg].interactions_count > 0 ? 1 : 0;
        return true;
    case NH_OBJ_PURCHASE_ITEM:
        *out = (gs->shop.bought & (unsigned)o->arg) != 0 ? 1 : 0;
        return true;
    case NH_OBJ_DECRYPT_MESSAGE:
        *out = (gs->player.milestones & (1u << (unsigned)o->arg)) != 0 ? 1 : 0;
        return true;
    case NH_OBJ_KEEP_ALERT_BELOW:
        *out = gs->alert.level < o->arg ? 1 : 0;
        return true;
    }
    return false;
}

/* ---- Affichage ------------------------------------------------------------------------------- */

/* Le texte de l'objectif ; seule la condition d'alerte a un nombre à y insérer. */
static void objective_text(char *out, size_t size, const NhObjectiveDef *o)
{
    if (o->kind == NH_OBJ_KEEP_ALERT_BELOW)
        snprintf(out, size, nh_tr(o->text), o->arg);
    else
        snprintf(out, size, "%s", nh_tr(o->text));
}

/* Un paragraphe, coupé à la largeur du terminal, décalé de `indent` colonnes. */
static void print_wrapped(const char *text, size_t indent)
{
    char wrapped[2048];
    nh_wrap_text(wrapped, sizeof wrapped, text, indent, indent, nh_wrap_width());
    printf("%*s%s\n", (int)indent, "", wrapped);
}

static void print_objective(const QuestState *state, const NhQuestDef *def, int i, const char *indent)
{
    const NhObjectiveDef *o = &def->objectives[i];
    bool done = state->progress[i] >= o->target;

    if (o->hidden && !done)
    {
        printf("%s[ ] %s\n", indent, nh_tr(NH_STR_QUEST_SECRET));
        return;
    }
    char text[256];
    objective_text(text, sizeof text, o);
    printf("%s%s%s %s", indent, nh_c(done ? NH_C_GREEN : NH_C_RESET), done ? "[x]" : "[ ]", text);
    if (o->target > 1)
        printf(" (%d/%d)", state->progress[i], o->target);
    printf("%s\n", nh_c(NH_C_RESET));
}

static void print_chapter_header(int chapter)
{
    printf(nh_tr(NH_STR_CHAPTER_HEADER), chapter, nh_tr(k_chapters[chapter - 1].title));
}

static void announce_chapter(int chapter)
{
    printf("\n%s═══ ", nh_c(NH_C_MAGENTA));
    print_chapter_header(chapter);
    printf(" ═══%s\n", nh_c(NH_C_RESET));
    print_wrapped(nh_tr(k_chapters[chapter - 1].text), 0);
}

static void announce_start(const NhQuestDef *def, bool new_chapter)
{
    if (new_chapter)
        announce_chapter(def->chapter);

    printf("\n%s", nh_c(NH_C_BRIGHT_CYAN));
    printf(nh_tr(NH_STR_QUEST_NEW_BANNER), nh_tr(def->title));
    printf("%s\n", nh_c(NH_C_RESET));
    print_wrapped(nh_tr(def->description), 0);
    printf("%s\n", nh_tr(NH_STR_QUEST_NEW_HINT));
}

static void announce_objective(const NhObjectiveDef *o)
{
    char text[256];
    objective_text(text, sizeof text, o);
    printf("\n%s", nh_c(NH_C_GREEN));
    printf(nh_tr(NH_STR_QUEST_OBJ_DONE), text);
    printf("%s\n", nh_c(NH_C_RESET));
}

/* ---- Moteur ---------------------------------------------------------------------------------- */

static bool unlock_ready(const GameState *gs, const NhQuestDef *def)
{
    if ((int)gs->player.level < def->level_required)
        return false;
    for (int p = 0; p < NH_QUEST_MAX_PREREQ; p++)
        if (def->prerequisites[p] >= 0 &&
            gs->quests.quests[def->prerequisites[p]].status != QUEST_STATUS_COMPLETED)
            return false;
    return true;
}

/*
 * Met l'avancement d'une quête active à jour d'après l'état du jeu. Renvoie vrai si tous ses
 * objectifs sont accomplis. `announce` : dire ce qui vient de s'accomplir (pas pour ce qui était
 * déjà fait quand la quête démarre : le journal le montre coché).
 */
static bool sync_objectives(GameState *gs, QuestType quest, bool announce)
{
    QuestState *state = &gs->quests.quests[quest];
    const NhQuestDef *def = &k_quests[quest];
    bool all_done = true;

    for (int i = 0; i < def->objective_count; i++)
    {
        const NhObjectiveDef *o = &def->objectives[i];
        bool was_done = state->progress[i] >= o->target;

        int value;
        if (measure(gs, o, &value))
        {
            if (value > o->target)
                value = o->target;
            if (o->kind == NH_OBJ_KEEP_ALERT_BELOW || value > state->progress[i])
                state->progress[i] = value;
        }

        bool done = state->progress[i] >= o->target;
        /* Une condition va et vient avec l'alerte : elle ne s'annonce pas, le journal la montre. */
        if (announce && done && !was_done && o->kind != NH_OBJ_KEEP_ALERT_BELOW)
            announce_objective(o);
        all_done = all_done && done;
    }
    return all_done;
}

bool nh_quest_complete(GameState *gs, QuestType quest, bool reward)
{
    if ((int)quest < 0 || quest >= QUEST_COUNT || !defined(quest))
        return false;
    QuestState *state = &gs->quests.quests[quest];
    if (state->status != QUEST_STATUS_ACTIVE)
        return false;

    /* Terminée AVANT de payer : rien de ce que le paiement déclenche ne peut la payer une seconde fois. */
    const NhQuestDef *def = &k_quests[quest];
    state->status = QUEST_STATUS_COMPLETED;
    for (int i = 0; i < def->objective_count; i++)
        state->progress[i] = def->objectives[i].target;

    if (reward)
    {
        printf("\n%s", nh_c(NH_C_BRIGHT_GREEN));
        printf(nh_tr(NH_STR_QUEST_DONE_BANNER), nh_tr(def->title));
        printf("%s\n", nh_c(NH_C_RESET));

        if (def->credits > 0 || def->reputation > 0 || def->xp > 0)
        {
            printf("  ");
            if (def->credits > 0)
            {
                gs->player.credits += def->credits;
                printf("%s", nh_c(NH_C_BRIGHT_GREEN));
                printf(nh_tr(NH_STR_PROG_CREDITS), def->credits);
                printf("%s ", nh_c(NH_C_RESET));
            }
            nh_grant_reputation(gs, def->reputation);
            nh_grant_xp_flat(gs, def->xp);
            printf("\n");
        }
    }
    nh_event(gs, NH_EV_QUEST_COMPLETED, (int)quest);
    return true;
}

void nh_quests_refresh(GameState *gs)
{
    QuestSystem *qs = &gs->quests;

    /* Terminer une quête peut en débloquer une autre : on relit jusqu'à ce que plus rien ne change. */
    for (int pass = 0; pass <= QUEST_COUNT; pass++)
    {
        bool changed = false;
        for (int q = 0; q < QUEST_COUNT; q++)
        {
            if (!defined((QuestType)q))
                continue;
            const NhQuestDef *def = &k_quests[q];
            QuestState *state = &qs->quests[q];

            if (state->status == QUEST_STATUS_LOCKED && unlock_ready(gs, def))
            {
                bool new_chapter = def->chapter > nh_quests_chapter(qs);
                state->status = QUEST_STATUS_ACTIVE;
                announce_start(def, new_chapter);
                sync_objectives(gs, (QuestType)q, false);
                changed = true;
            }
            if (state->status == QUEST_STATUS_ACTIVE && sync_objectives(gs, (QuestType)q, true))
            {
                nh_quest_complete(gs, (QuestType)q, true);
                changed = true;
            }
        }
        if (!changed)
            break;
    }
}

void nh_quests_on_event(GameState *gs, NhEvent event, int value)
{
    (void)event;
    (void)value;
    nh_quests_refresh(gs);
}

/* ---- Journal --------------------------------------------------------------------------------- */

void nh_quests_print_log(const GameState *gs)
{
    const QuestSystem *qs = &gs->quests;
    int chapter = nh_quests_chapter(qs);

    printf("\n%s%s%s\n", nh_c(NH_C_BRIGHT_CYAN), nh_tr(NH_STR_QUEST_LOG_TITLE), nh_c(NH_C_RESET));
    if (chapter >= 1)
    {
        printf("%s", nh_c(NH_C_MAGENTA));
        print_chapter_header(chapter);
        printf("%s\n", nh_c(NH_C_RESET));
    }

    printf("\n%s%s%s\n", nh_c(NH_C_BRIGHT_CYAN), nh_tr(NH_STR_QUEST_SECTION_ACTIVE), nh_c(NH_C_RESET));
    bool any_active = false;
    for (int q = 0; q < QUEST_COUNT; q++)
    {
        const NhQuestDef *def = &k_quests[q];
        if (!defined((QuestType)q) || qs->quests[q].status != QUEST_STATUS_ACTIVE)
            continue;
        any_active = true;

        printf("\n%s[%d] %s%s\n", nh_c(NH_C_YELLOW), q + 1, nh_tr(def->title), nh_c(NH_C_RESET));
        print_wrapped(nh_tr(def->description), 4);
        printf("    %s: %s%s%s   %s: %s%s%s\n", nh_tr(NH_STR_QUEST_LABEL_CONTACT), nh_c(NH_C_WHITE),
               nh_contact_name(def->contact), nh_c(NH_C_RESET), nh_tr(NH_STR_QUEST_LABEL_LOCATION), nh_c(NH_C_WHITE),
               nh_tr(def->location), nh_c(NH_C_RESET));
        printf("    %s%s%s\n", nh_c(NH_C_GREEN), nh_tr(NH_STR_QUEST_LABEL_OBJECTIVES), nh_c(NH_C_RESET));
        for (int i = 0; i < def->objective_count; i++)
            print_objective(&qs->quests[q], def, i, "      ");
    }
    if (!any_active)
    {
        printf("%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_QUEST_NONE_ACTIVE), nh_c(NH_C_RESET));
        /* Une quête n'attend plus que le niveau : le dire évite de chercher ce qui manque. */
        for (int q = 0; q < QUEST_COUNT; q++)
        {
            const NhQuestDef *def = &k_quests[q];
            if (!defined((QuestType)q) || qs->quests[q].status != QUEST_STATUS_LOCKED ||
                (int)gs->player.level >= def->level_required)
                continue;
            bool blocked = false;
            for (int p = 0; p < NH_QUEST_MAX_PREREQ; p++)
                blocked = blocked || (def->prerequisites[p] >= 0 &&
                                      qs->quests[def->prerequisites[p]].status != QUEST_STATUS_COMPLETED);
            if (blocked)
                continue;
            printf(nh_tr(NH_STR_QUEST_NEXT_LEVEL), def->level_required);
            printf("\n");
            break;
        }
    }

    printf("\n%s%s%s\n", nh_c(NH_C_BRIGHT_CYAN), nh_tr(NH_STR_QUEST_SECTION_DONE), nh_c(NH_C_RESET));
    bool any_done = false;
    for (int q = 0; q < QUEST_COUNT; q++)
    {
        if (!defined((QuestType)q) || qs->quests[q].status != QUEST_STATUS_COMPLETED)
            continue;
        any_done = true;
        printf("  %s[%d] %s%s\n", nh_c(NH_C_GREEN), q + 1, nh_tr(k_quests[q].title), nh_c(NH_C_RESET));
    }
    if (!any_done)
        printf("%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_QUEST_NONE_DONE), nh_c(NH_C_RESET));

    printf("\n%s", nh_c(NH_C_MAGENTA));
    printf(nh_tr(NH_STR_QUEST_PROGRESS), nh_quests_percent(qs));
    printf("%s\n", nh_c(NH_C_RESET));
}

void nh_quest_print_details(const GameState *gs, QuestType quest)
{
    const NhQuestDef *def = nh_quest_def(quest);
    if (def == NULL || !defined(quest))
        return;
    const QuestState *state = &gs->quests.quests[quest];

    printf("\n%s%s%s\n", nh_c(NH_C_CYAN), nh_tr(NH_STR_QUEST_DETAILS_TITLE), nh_c(NH_C_RESET));
    printf("\n%s%s%s\n", nh_c(NH_C_BRIGHT_CYAN), nh_tr(def->title), nh_c(NH_C_RESET));
    print_wrapped(nh_tr(def->description), 0);

    printf("\n%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_QUEST_LABEL_CONTEXT), nh_c(NH_C_RESET));
    print_wrapped(nh_tr(def->lore), 0);

    printf("\n%s%s%s\n", nh_c(NH_C_GREEN), nh_tr(NH_STR_QUEST_LABEL_OBJECTIVES), nh_c(NH_C_RESET));
    for (int i = 0; i < def->objective_count; i++)
        print_objective(state, def, i, "  ");

    /* Une quête terminée a déjà payé (ou, tutoriel passé, ne paiera jamais) : on ne promet plus rien. */
    if (state->status == QUEST_STATUS_ACTIVE)
    {
        printf("\n%s%s%s\n", nh_c(NH_C_MAGENTA), nh_tr(NH_STR_QUEST_LABEL_REWARDS), nh_c(NH_C_RESET));
        if (def->xp > 0)
        {
            printf("  • ");
            printf(nh_tr(NH_STR_QUEST_REWARD_XP), def->xp);
            printf("\n");
        }
        if (def->credits > 0)
        {
            printf("  • ");
            printf(nh_tr(NH_STR_QUEST_REWARD_CREDITS), def->credits);
            printf("\n");
        }
        if (def->reputation > 0)
        {
            printf("  • ");
            printf(nh_tr(NH_STR_QUEST_REWARD_REPUTATION), def->reputation);
            printf("\n");
        }
    }
}

/* La commande `quests` : le journal, puis le détail d'une quête au choix. */
bool cmd_quests(GameState *gs, const char *arg)
{
    (void)arg;

    /* Pendant le tutoriel, le journal montre la mission d'ECHO-7 étape par étape. */
    if (nh_tutorial_active(gs))
    {
        nh_tutorial_print_mission(gs);
        return true;
    }

    nh_quests_print_log(gs);

    printf("\n%s%s%s", nh_c(NH_C_YELLOW), nh_tr(NH_STR_QUEST_PROMPT), nh_c(NH_C_RESET));
    fflush(stdout);
    char line[32];
    if (nh_read_line(line, sizeof line) != NH_IO_OK || line[0] == '\0')
        return true;

    int number;
    if (!nh_parse_int(line, 0, 99, &number))
    {
        printf("%s\n", nh_tr(NH_STR_INVALID_OPTION));
        return true;
    }
    if (number == 0)
        return true;

    int q = number - 1;
    if (q >= QUEST_COUNT || !defined((QuestType)q) ||
        (gs->quests.quests[q].status != QUEST_STATUS_ACTIVE &&
         gs->quests.quests[q].status != QUEST_STATUS_COMPLETED))
    {
        printf(nh_tr(NH_STR_QUEST_NO_SUCH), number);
        printf("\n");
        return true;
    }
    nh_quest_print_details(gs, (QuestType)q);
    return true;
}
