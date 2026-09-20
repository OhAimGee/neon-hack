#include "contacts.h"

#include <stdio.h>
#include <string.h>

#include "../core/io.h"
#include "../core/parse.h"
#include "../i18n/i18n.h"
#include "../ui/term.h"
#include "game.h"
#include "quest_system.h"
#include "tutorial.h"

/* ---- Les contacts ---------------------------------------------------------------------------- */

typedef enum
{
    NH_TOPIC_TALK,    /* une réplique */
    NH_TOPIC_STATUS,  /* une réplique qui commente le niveau du joueur (le seul « %d » de la réplique) */
    NH_TOPIC_MISSION, /* la mission en cours confiée par ce contact ; sans mission, la réplique */
    NH_TOPIC_SHOP,    /* la réplique, puis la boutique s'ouvre */
    NH_TOPIC_PROFILE, /* son dossier : identité, lieu, spécialité, trait, histoire */
    NH_TOPIC_END      /* la réplique d'adieu, et la conversation se termine */
} NhTopicKind;

typedef struct
{
    NhTopicKind kind;
    NhStr label; /* ce que le joueur choisit dans le menu */
    NhStr reply; /* ce que répond le contact (inutilisé pour NH_TOPIC_PROFILE) */
} NhTopic;

#define NH_MAX_TOPICS 8

typedef struct
{
    const char *name; /* nom propre : jamais traduit ; vide = pas de fiche */
    NhColor color;    /* couleur de son nom devant ses répliques */

    /* Conditions de déblocage : toutes à la fois. */
    int level;
    int reputation;
    int quest; /* quête à avoir terminée, ou -1 */

    int trust; /* confiance de départ (0..100) */

    NhStr real_name;
    NhStr description;
    NhStr speciality;
    NhStr location;
    NhStr scene;    /* mise en scène, affichée à la connexion */
    NhStr greeting; /* première réplique */
    NhStr personality;
    NhStr story;

    int topic_count; /* 0 : fiche pas encore écrite (phase 4.2) */
    NhTopic topics[NH_MAX_TOPICS];
} NhContactDef;

/*
 * Les niveaux et réputations sont ceux de la fiche d'origine ; les quêtes rattachent Phoenix et
 * AURA à l'histoire : Phoenix apparaît quand « Réseaux d'Information » est terminée (c'est lui qui
 * confie « L'œil du Cyclone »), AURA quand Nexus a été percé (« L'œil du Cyclone »). Les réputations
 * sont celles que ces quêtes garantissent : aucune impasse pour un joueur qui suit l'histoire.
 */
static const NhContactDef k_defs[CONTACT_COUNT] = {
    [CONTACT_ECHO7] =
        {
            .name = "ECHO-7",
            .color = NH_C_BRIGHT_CYAN,
            .level = 1,
            .reputation = 0,
            .quest = -1,
            .trust = 60,
            .real_name = NH_STR_CT_ECHO_REAL,
            .description = NH_STR_CT_ECHO_DESC,
            .speciality = NH_STR_CT_ECHO_SPEC,
            .location = NH_STR_CT_ECHO_LOC,
            .scene = NH_STR_CT_ECHO_SCENE,
            .greeting = NH_STR_CT_ECHO_HELLO,
            .personality = NH_STR_CT_ECHO_TRAIT,
            .story = NH_STR_CT_ECHO_STORY,
            .topic_count = 6,
            .topics =
                {
                    {NH_TOPIC_TALK, NH_STR_CT_ECHO_ADVICE_Q, NH_STR_CT_ECHO_ADVICE_A},
                    {NH_TOPIC_STATUS, NH_STR_CT_ECHO_PROGRESS_Q, NH_STR_CT_ECHO_PROGRESS_A},
                    {NH_TOPIC_MISSION, NH_STR_CT_ECHO_MISSION_Q, NH_STR_CT_ECHO_MISSION_A},
                    {NH_TOPIC_TALK, NH_STR_CT_ECHO_NEXUS_Q, NH_STR_CT_ECHO_NEXUS_A},
                    {NH_TOPIC_PROFILE, NH_STR_CT_DOSSIER, NH_STR_CT_DOSSIER},
                    {NH_TOPIC_END, NH_STR_CT_END, NH_STR_CT_ECHO_BYE},
                },
        },
    [CONTACT_R4Z0R] =
        {
            .name = "R4Z0R",
            .color = NH_C_MAGENTA,
            .level = 2,
            .reputation = 10,
            .quest = -1,
            .trust = 30,
            .real_name = NH_STR_CT_R4Z0R_REAL,
            .description = NH_STR_CT_R4Z0R_DESC,
            .speciality = NH_STR_CT_R4Z0R_SPEC,
            .location = NH_STR_CT_R4Z0R_LOC,
            .scene = NH_STR_CT_R4Z0R_SCENE,
            .greeting = NH_STR_CT_R4Z0R_HELLO,
            .personality = NH_STR_CT_R4Z0R_TRAIT,
            .story = NH_STR_CT_R4Z0R_STORY,
            .topic_count = 6,
            .topics =
                {
                    {NH_TOPIC_SHOP, NH_STR_CT_R4Z0R_SHOP_Q, NH_STR_CT_R4Z0R_SHOP_A},
                    {NH_TOPIC_TALK, NH_STR_CT_R4Z0R_GEAR_Q, NH_STR_CT_R4Z0R_GEAR_A},
                    {NH_TOPIC_TALK, NH_STR_CT_R4Z0R_MONEY_Q, NH_STR_CT_R4Z0R_MONEY_A},
                    {NH_TOPIC_TALK, NH_STR_CT_R4Z0R_NEWS_Q, NH_STR_CT_R4Z0R_NEWS_A},
                    {NH_TOPIC_PROFILE, NH_STR_CT_DOSSIER, NH_STR_CT_DOSSIER},
                    {NH_TOPIC_END, NH_STR_CT_END, NH_STR_CT_R4Z0R_BYE},
                },
        },
    [CONTACT_PHOENIX] =
        {
            .name = "Phoenix",
            .color = NH_C_RED,
            .level = 4,
            .reputation = 50,
            .quest = QUEST_GATHER_INTEL,
            .trust = 0,
            .real_name = NH_STR_CT_PHOENIX_REAL,
            .description = NH_STR_CT_PHOENIX_DESC,
            .speciality = NH_STR_CT_PHOENIX_SPEC,
            .location = NH_STR_CT_PHOENIX_LOC,
            .scene = NH_STR_CT_PHOENIX_SCENE,
            .greeting = NH_STR_CT_PHOENIX_HELLO,
            .personality = NH_STR_CT_PHOENIX_TRAIT,
            .story = NH_STR_CT_PHOENIX_STORY,
            .topic_count = 5,
            .topics =
                {
                    {NH_TOPIC_MISSION, NH_STR_CT_PHOENIX_MISSION_Q, NH_STR_CT_PHOENIX_MISSION_A},
                    {NH_TOPIC_TALK, NH_STR_CT_PHOENIX_NEXUS_Q, NH_STR_CT_PHOENIX_NEXUS_A},
                    {NH_TOPIC_TALK, NH_STR_CT_PHOENIX_TECH_Q, NH_STR_CT_PHOENIX_TECH_A},
                    {NH_TOPIC_PROFILE, NH_STR_CT_DOSSIER, NH_STR_CT_DOSSIER},
                    {NH_TOPIC_END, NH_STR_CT_END, NH_STR_CT_PHOENIX_BYE},
                },
        },
    [CONTACT_AURA] =
        {
            .name = "AURA",
            .color = NH_C_BRIGHT_CYAN,
            .level = 5,
            .reputation = 50,
            .quest = QUEST_NEXUS_DATA_BREACH,
            .trust = 0,
            .real_name = NH_STR_CT_AURA_REAL,
            .description = NH_STR_CT_AURA_DESC,
            .speciality = NH_STR_CT_AURA_SPEC,
            .location = NH_STR_CT_AURA_LOC,
            .scene = NH_STR_CT_AURA_SCENE,
            .greeting = NH_STR_CT_AURA_HELLO,
            .personality = NH_STR_CT_AURA_TRAIT,
            .story = NH_STR_CT_AURA_STORY,
            .topic_count = 5,
            .topics =
                {
                    {NH_TOPIC_TALK, NH_STR_CT_AURA_HELP_Q, NH_STR_CT_AURA_HELP_A},
                    {NH_TOPIC_TALK, NH_STR_CT_AURA_PROJECT_Q, NH_STR_CT_AURA_PROJECT_A},
                    {NH_TOPIC_MISSION, NH_STR_CT_AURA_FREE_Q, NH_STR_CT_AURA_FREE_A},
                    {NH_TOPIC_PROFILE, NH_STR_CT_DOSSIER, NH_STR_CT_DOSSIER},
                    {NH_TOPIC_END, NH_STR_CT_END, NH_STR_CT_AURA_BYE},
                },
        },
    /* Phase 4.2 : les cinq suivants n'ont qu'un nom ; sans sujet de conversation, ils ne se débloquent pas. */
    [CONTACT_SHADOW_BROKER] = {.name = "Shadow Broker", .quest = -1},
    [CONTACT_NEON_ANGEL] = {.name = "Neon Angel", .quest = -1},
    [CONTACT_GHOST_WALKER] = {.name = "Ghost Walker", .quest = -1},
    [CONTACT_DATA_MINER] = {.name = "Data Miner", .quest = -1},
    [CONTACT_NEXUS_INSIDER] = {.name = "Nexus Insider", .quest = -1},
};

/* ---- Les courriers --------------------------------------------------------------------------- */

static const struct
{
    ContactType from;
    ContactType on_unlock; /* le courrier arrive quand ce contact est débloqué */
    NhStr subject;
    NhStr body;
} k_mails[NH_MAIL_COUNT] = {
    [NH_MAIL_WELCOME] = {CONTACT_ECHO7, CONTACT_ECHO7, NH_STR_MAIL_WELCOME_SUBJECT, NH_STR_MAIL_WELCOME_BODY},
    [NH_MAIL_R4Z0R] = {CONTACT_R4Z0R, CONTACT_R4Z0R, NH_STR_MAIL_R4Z0R_SUBJECT, NH_STR_MAIL_R4Z0R_BODY},
    [NH_MAIL_PHOENIX] = {CONTACT_PHOENIX, CONTACT_PHOENIX, NH_STR_MAIL_PHOENIX_SUBJECT, NH_STR_MAIL_PHOENIX_BODY},
    [NH_MAIL_AURA] = {CONTACT_AURA, CONTACT_AURA, NH_STR_MAIL_AURA_SUBJECT, NH_STR_MAIL_AURA_BODY},
};

/* ---- Données --------------------------------------------------------------------------------- */

#define MAX_TALKS 1000000 /* les compteurs sont bornés, comme partout ailleurs */
#define TRUST_PER_TALK 3

static bool valid(ContactType type)
{
    return (int)type >= 0 && type < CONTACT_COUNT;
}

const char *nh_contact_name(ContactType type)
{
    return valid(type) ? k_defs[type].name : "?";
}

bool nh_contact_written(ContactType type)
{
    return valid(type) && k_defs[type].topic_count > 0;
}

int nh_contact_trust(const ContactSystem *contact_system, ContactType type)
{
    if (!valid(type))
        return 0;
    long trust = k_defs[type].trust + (long)contact_system->contacts[type].interactions_count * TRUST_PER_TALK;
    return trust > 100 ? 100 : (int)trust;
}

NhRelation nh_contact_relation(int trust)
{
    if (trust >= 80)
        return NH_REL_TRUSTED;
    if (trust >= 50)
        return NH_REL_FRIENDLY;
    if (trust >= 20)
        return NH_REL_NEUTRAL;
    return NH_REL_UNKNOWN;
}

int nh_contacts_unlocked(const ContactSystem *contact_system)
{
    int n = 0;
    for (int i = 0; i < CONTACT_COUNT; i++)
        if (contact_system->contacts[i].is_unlocked)
            n++;
    return n;
}

/* Le n-ième contact débloqué (n à partir de 1), ou -1. */
static int nth_unlocked(const ContactSystem *contact_system, int n)
{
    int seen = 0;
    for (int i = 0; i < CONTACT_COUNT; i++)
        if (contact_system->contacts[i].is_unlocked && ++seen == n)
            return i;
    return -1;
}

int nh_contact_find(const ContactSystem *contact_system, const char *text)
{
    char word[64];
    snprintf(word, sizeof word, "%s", text);
    size_t start = strspn(word, " \t");
    size_t end = strlen(word);
    while (end > start && (word[end - 1] == ' ' || word[end - 1] == '\t'))
        end--;
    word[end] = '\0';

    int number;
    if (nh_parse_int(word + start, 0, MAX_TALKS, &number))
        return nth_unlocked(contact_system, number);

    for (int i = 0; i < CONTACT_COUNT; i++)
        if (contact_system->contacts[i].is_unlocked && nh_str_eq_nocase(k_defs[i].name, word + start))
            return i;
    return -1;
}

/* ---- Courriers : état ------------------------------------------------------------------------ */

static bool add_mail(ContactSystem *contact_system, NhMail mail)
{
    if ((int)mail < 0 || mail >= NH_MAIL_COUNT || contact_system->inbox_count >= NH_INBOX_MAX)
        return false;
    for (int i = 0; i < contact_system->inbox_count; i++)
        if (contact_system->inbox[i].mail == mail)
            return false;
    contact_system->inbox[contact_system->inbox_count].mail = mail;
    contact_system->inbox[contact_system->inbox_count].is_read = false;
    contact_system->inbox_count++;
    return true;
}

int nh_inbox_unread(const ContactSystem *contact_system)
{
    int n = 0;
    for (int i = 0; i < contact_system->inbox_count; i++)
        if (!contact_system->inbox[i].is_read)
            n++;
    return n;
}

void init_contact_system(ContactSystem *contact_system)
{
    memset(contact_system, 0, sizeof *contact_system);
    contact_system->contacts[CONTACT_ECHO7].is_unlocked = true;
    for (int m = 0; m < NH_MAIL_COUNT; m++)
        if (k_mails[m].on_unlock == CONTACT_ECHO7)
            (void)add_mail(contact_system, (NhMail)m);
}

bool nh_mail_send(GameState *gs, NhMail mail)
{
    if (!add_mail(&gs->contacts, mail))
        return false;
    printf("%s", nh_c(NH_C_CYAN));
    printf(nh_tr(NH_STR_MAIL_ARRIVED), nh_contact_name(k_mails[mail].from), nh_tr(k_mails[mail].subject));
    printf("%s\n", nh_c(NH_C_RESET));
    return true;
}

/* ---- Déblocage ------------------------------------------------------------------------------- */

static bool conditions_met(const GameState *gs, const NhContactDef *def)
{
    if ((int)gs->player.level < def->level || gs->player.reputation < def->reputation)
        return false;
    return def->quest < 0 || gs->quests.quests[def->quest].status == QUEST_STATUS_COMPLETED;
}

static void unlock(GameState *gs, ContactType type)
{
    const NhContactDef *def = &k_defs[type];
    gs->contacts.contacts[type].is_unlocked = true;

    printf("\n%s", nh_c(NH_C_BRIGHT_GREEN));
    printf(nh_tr(NH_STR_CT_NEW), def->name);
    printf("%s\n%s\n", nh_c(NH_C_RESET), nh_tr(def->description));

    for (int m = 0; m < NH_MAIL_COUNT; m++)
        if (k_mails[m].on_unlock == type)
            (void)nh_mail_send(gs, (NhMail)m);
}

void nh_contacts_on_event(GameState *gs, NhEvent event, int value)
{
    (void)value;
    switch (event)
    {
    case NH_EV_COMMAND:         /* l'état a pu changer de mille façons (sauvegarde rechargée…) */
    case NH_EV_LEVEL_UP:
    case NH_EV_REPUTATION:
    case NH_EV_QUEST_COMPLETED:
        break;
    case NH_EV_NODE_COMPROMISED:
    case NH_EV_FILES_EXTRACTED:
    case NH_EV_ITEM_BOUGHT:
    case NH_EV_CONTACT_MET:
    case NH_EV_MILESTONE:
    case NH_EV_COUNT:
        return;
    }

    for (int i = 0; i < CONTACT_COUNT; i++)
    {
        const NhContactDef *def = &k_defs[i];
        if (gs->contacts.contacts[i].is_unlocked || def->topic_count == 0 || !conditions_met(gs, def))
            continue;
        unlock(gs, (ContactType)i);
    }
}

/* ---- Liste des contacts ---------------------------------------------------------------------- */

static const NhStr k_relation[] = {
    [NH_REL_UNKNOWN] = NH_STR_CT_REL_UNKNOWN,
    [NH_REL_NEUTRAL] = NH_STR_CT_REL_NEUTRAL,
    [NH_REL_FRIENDLY] = NH_STR_CT_REL_FRIENDLY,
    [NH_REL_TRUSTED] = NH_STR_CT_REL_TRUSTED,
};

/* Ce que le contact sait faire, déduit de ses sujets : la liste ne peut pas promettre plus. */
static void print_services(const NhContactDef *def)
{
    bool missions = false;
    bool shop = false;
    bool advice = false;
    for (int i = 0; i < def->topic_count; i++)
        switch (def->topics[i].kind)
        {
        case NH_TOPIC_MISSION:
            missions = true;
            break;
        case NH_TOPIC_SHOP:
            shop = true;
            break;
        case NH_TOPIC_TALK:
        case NH_TOPIC_STATUS:
            advice = true;
            break;
        case NH_TOPIC_PROFILE:
        case NH_TOPIC_END:
            break;
        }

    const char *separator = "";
    printf("%s", nh_c(NH_C_GREEN));
    if (missions)
    {
        printf("%s%s", separator, nh_tr(NH_STR_CT_SVC_MISSIONS));
        separator = " · ";
    }
    if (shop)
    {
        printf("%s%s", separator, nh_tr(NH_STR_CT_SVC_SHOP));
        separator = " · ";
    }
    if (advice)
        printf("%s%s", separator, nh_tr(NH_STR_CT_SVC_ADVICE));
    printf("%s", nh_c(NH_C_RESET));
}

static void print_entry(const ContactSystem *contact_system, int number, ContactType type)
{
    const NhContactDef *def = &k_defs[type];
    int trust = nh_contact_trust(contact_system, type);

    printf("%s[%d]%s %s%s%s", nh_c(NH_C_YELLOW), number, nh_c(NH_C_RESET), nh_c(def->color), def->name,
           nh_c(NH_C_RESET));
    printf(nh_tr(NH_STR_CT_LINE_REL), nh_tr(k_relation[nh_contact_relation(trust)]), trust);
    printf("\n    %s\n    ", nh_tr(def->description));
    print_services(def);
    printf(" · ");
    printf(nh_tr(NH_STR_CT_TALKS), contact_system->contacts[type].interactions_count);
    printf("\n");
}

void nh_contacts_print(const GameState *gs)
{
    const ContactSystem *contacts = &gs->contacts;

    printf("\n%s%s%s\n", nh_c(NH_C_BRIGHT_CYAN), nh_tr(NH_STR_CT_TITLE), nh_c(NH_C_RESET));

    int number = 0;
    for (int i = 0; i < CONTACT_COUNT; i++)
        if (contacts->contacts[i].is_unlocked && k_defs[i].topic_count > 0)
            print_entry(contacts, ++number, (ContactType)i);
    if (number == 0)
        printf("%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_CT_NONE), nh_c(NH_C_RESET));

    int hidden = 0;
    for (int i = 0; i < CONTACT_COUNT; i++)
        if (!contacts->contacts[i].is_unlocked && k_defs[i].topic_count > 0)
            hidden++;
    if (hidden > 0)
    {
        printf("%s", nh_c(NH_C_YELLOW));
        printf(nh_tr(NH_STR_CT_MORE), hidden);
        printf("%s\n", nh_c(NH_C_RESET));
    }
}

/* ---- Conversation ---------------------------------------------------------------------------- */

static void say(const NhContactDef *def, const char *text)
{
    nh_speak(def->name, def->color, text, true, 0);
}

static void print_field(NhStr label, NhStr value)
{
    printf("  %s%s:%s %s\n", nh_c(NH_C_YELLOW), nh_tr(label), nh_c(NH_C_RESET), nh_tr(value));
}

static void print_profile(const NhContactDef *def)
{
    printf("%s", nh_c(NH_C_BRIGHT_CYAN));
    printf(nh_tr(NH_STR_CT_FILE_TITLE), def->name);
    printf("%s\n", nh_c(NH_C_RESET));

    print_field(NH_STR_CT_F_REAL, def->real_name);
    print_field(NH_STR_CT_F_PLACE, def->location);
    print_field(NH_STR_CT_F_SPECIALITY, def->speciality);
    print_field(NH_STR_CT_F_TRAIT, def->personality);
    printf("  %s%s:%s\n  ", nh_c(NH_C_YELLOW), nh_tr(NH_STR_CT_F_BACKGROUND), nh_c(NH_C_RESET));

    char story[1024];
    nh_wrap_text(story, sizeof story, nh_tr(def->story), 2, 2, nh_wrap_width());
    printf("%s\n", story);
}

/* La mission active que ce contact a confiée : le titre, sa description, et où trouver le détail. */
static void give_mission(const GameState *gs, ContactType type, const NhContactDef *def, const NhTopic *topic)
{
    for (int q = 0; q < QUEST_COUNT; q++)
    {
        const NhQuestDef *quest = nh_quest_def((QuestType)q);
        if (quest->contact != type || gs->quests.quests[q].status != QUEST_STATUS_ACTIVE)
            continue;

        printf("%s", nh_c(NH_C_YELLOW));
        printf(nh_tr(NH_STR_CT_MISSION_NOW), nh_tr(quest->title));
        printf("%s\n", nh_c(NH_C_RESET));

        char text[1024];
        nh_wrap_text(text, sizeof text, nh_tr(quest->description), 0, 0, nh_wrap_width());
        printf("%s\n", text);
        /* Le tutoriel se déroule pas à pas : ECHO-7 redit la consigne de l'étape où l'on en est. */
        if (q == QUEST_INTRO_TUTORIAL)
            nh_tutorial_repeat(gs);
        printf("%s%s%s\n", nh_c(NH_C_CYAN), nh_tr(NH_STR_CT_MISSION_HINT), nh_c(NH_C_RESET));
        return;
    }
    say(def, nh_tr(topic->reply));
}

static void run_topic(GameState *gs, ContactType type, const NhContactDef *def, const NhTopic *topic)
{
    switch (topic->kind)
    {
    case NH_TOPIC_TALK:
    case NH_TOPIC_END:
        say(def, nh_tr(topic->reply));
        break;
    case NH_TOPIC_STATUS:
    {
        char line[512];
        snprintf(line, sizeof line, nh_tr(topic->reply), (int)gs->player.level);
        say(def, line);
        break;
    }
    case NH_TOPIC_MISSION:
        give_mission(gs, type, def, topic);
        break;
    case NH_TOPIC_SHOP:
        say(def, nh_tr(topic->reply));
        (void)cmd_shop(gs, "");
        break;
    case NH_TOPIC_PROFILE:
        print_profile(def);
        break;
    }
}

enum
{
    CHOICE_EOF = -1,
    CHOICE_INVALID = -2
};

/* Numéro de sujet choisi : 0 pour partir (ou entrée vide), CHOICE_EOF si l'entrée est fermée. */
static int read_choice(int count)
{
    printf("%s", nh_c(NH_C_MAGENTA));
    printf(nh_tr(NH_STR_CT_CHOICE), count);
    printf("%s", nh_c(NH_C_RESET));

    char line[16];
    if (nh_read_line(line, sizeof line) != NH_IO_OK)
        return CHOICE_EOF;
    if (line[strspn(line, " \t")] == '\0')
        return 0;
    int choice;
    return nh_parse_int(line, 0, count, &choice) ? choice : CHOICE_INVALID;
}

bool nh_contact_talk(GameState *gs, ContactType type)
{
    if (!nh_contact_written(type) || !gs->contacts.contacts[type].is_unlocked)
        return false;

    const NhContactDef *def = &k_defs[type];
    Contact *contact = &gs->contacts.contacts[type];
    if (contact->interactions_count < MAX_TALKS)
        contact->interactions_count++;
    nh_event(gs, NH_EV_CONTACT_MET, (int)type);

    printf("\n%s", nh_c(NH_C_BRIGHT_GREEN));
    printf(nh_tr(NH_STR_CT_CONNECTED), def->name);
    printf("%s\n%s%s%s\n\n", nh_c(NH_C_RESET), nh_c(NH_C_CYAN), nh_tr(def->scene), nh_c(NH_C_RESET));
    say(def, nh_tr(def->greeting));

    for (;;)
    {
        printf("\n");
        for (int i = 0; i < def->topic_count; i++)
            printf("  %s[%d]%s %s\n", nh_c(NH_C_YELLOW), i + 1, nh_c(NH_C_RESET), nh_tr(def->topics[i].label));

        int choice = read_choice(def->topic_count);
        if (choice == CHOICE_EOF)
            break;
        if (choice == CHOICE_INVALID)
        {
            printf("%s%s%s\n", nh_c(NH_C_RED), nh_tr(NH_STR_INVALID_OPTION), nh_c(NH_C_RESET));
            continue;
        }
        if (choice == 0)
        {
            printf("%s\n", nh_tr(NH_STR_CT_LEFT));
            break;
        }

        const NhTopic *topic = &def->topics[choice - 1];
        printf("\n");
        run_topic(gs, type, def, topic);
        if (topic->kind == NH_TOPIC_END)
            break;
    }
    return true;
}

/* ---- Boîte de réception ---------------------------------------------------------------------- */

void nh_inbox_print(const GameState *gs)
{
    const ContactSystem *contacts = &gs->contacts;

    printf("\n%s", nh_c(NH_C_BRIGHT_CYAN));
    printf(nh_tr(NH_STR_MAIL_TITLE), nh_inbox_unread(contacts));
    printf("%s\n", nh_c(NH_C_RESET));

    if (contacts->inbox_count == 0)
    {
        printf("%s%s%s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_MAIL_EMPTY), nh_c(NH_C_RESET));
        return;
    }

    for (int i = 0; i < contacts->inbox_count; i++)
    {
        const Mail *mail = &contacts->inbox[i];
        if ((int)mail->mail < 0 || mail->mail >= NH_MAIL_COUNT)
            continue;
        printf(" %s[%d]%s %s — %s", nh_c(NH_C_YELLOW), i + 1, nh_c(NH_C_RESET), nh_contact_name(k_mails[mail->mail].from),
               nh_tr(k_mails[mail->mail].subject));
        if (!mail->is_read)
            printf("  %s%s%s", nh_c(NH_C_BRIGHT_GREEN), nh_tr(NH_STR_MAIL_NEW_TAG), nh_c(NH_C_RESET));
        printf("\n");
    }
    printf("\n%s%s%s\n", nh_c(NH_C_MAGENTA), nh_tr(NH_STR_MAIL_HOWTO), nh_c(NH_C_RESET));
}

static void print_rule(void)
{
    printf("%s", nh_c(NH_C_CYAN));
    for (int i = 0; i < 40; i++)
        printf("─");
    printf("%s\n", nh_c(NH_C_RESET));
}

/* ---- Commandes ------------------------------------------------------------------------------- */

bool cmd_contacts(GameState *gs, const char *arg)
{
    (void)arg;

    nh_contacts_print(gs);
    int count = nh_contacts_unlocked(&gs->contacts);
    if (count == 0)
        return true;

    printf("\n%s%s%s\n", nh_c(NH_C_MAGENTA), nh_tr(NH_STR_CT_HOWTO), nh_c(NH_C_RESET));
    printf("%s", nh_tr(NH_STR_CT_PROMPT));

    char input[16];
    if (nh_read_line(input, sizeof input) != NH_IO_OK || input[strspn(input, " \t")] == '\0')
        return true;

    int number;
    if (!nh_parse_int(input, 0, count, &number))
    {
        printf("%s%s%s\n", nh_c(NH_C_RED), nh_tr(NH_STR_INVALID_OPTION), nh_c(NH_C_RESET));
        return true;
    }
    if (number > 0)
        (void)nh_contact_talk(gs, (ContactType)nth_unlocked(&gs->contacts, number));
    return true;
}

bool cmd_interact_contact(GameState *gs, const char *argument)
{
    if (argument[strspn(argument, " \t")] == '\0')
    {
        printf("%s\n", nh_tr(NH_STR_CT_USAGE));
        return false;
    }

    int found = nh_contact_find(&gs->contacts, argument);
    if (found < 0 || !nh_contact_talk(gs, (ContactType)found))
    {
        int number;
        if (nh_parse_int(argument, 0, MAX_TALKS, &number))
            printf(nh_tr(NH_STR_CT_UNKNOWN_NUMBER), number);
        else
            printf(nh_tr(NH_STR_CT_UNKNOWN), argument);
        printf("\n");
        return false;
    }
    return true;
}

bool cmd_messages(GameState *gs, const char *arg)
{
    (void)arg;
    nh_inbox_print(gs);
    return true;
}

bool cmd_read(GameState *gs, const char *argument)
{
    if (argument[strspn(argument, " \t")] == '\0')
    {
        printf("%s\n", nh_tr(NH_STR_MAIL_USAGE));
        return false;
    }

    int number;
    if (!nh_parse_int(argument, 1, gs->contacts.inbox_count, &number))
    {
        printf("%s\n", nh_tr(NH_STR_MAIL_BAD_NUMBER));
        return false;
    }

    Mail *mail = &gs->contacts.inbox[number - 1];
    mail->is_read = true;
    if ((int)mail->mail < 0 || mail->mail >= NH_MAIL_COUNT)
        return true;

    printf("\n");
    print_rule();
    printf("%s%s:%s %s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_MAIL_FROM), nh_c(NH_C_RESET),
           nh_contact_name(k_mails[mail->mail].from));
    printf("%s%s:%s %s\n", nh_c(NH_C_YELLOW), nh_tr(NH_STR_MAIL_SUBJECT), nh_c(NH_C_RESET),
           nh_tr(k_mails[mail->mail].subject));
    print_rule();

    char body[2048];
    nh_wrap_text(body, sizeof body, nh_tr(k_mails[mail->mail].body), 0, 0, nh_wrap_width());
    printf("\n%s\n\n", body);
    print_rule();
    return true;
}
