#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"

#include "../../src/core/io.h"
#include "../../src/core/platform.h"
#include "../../src/game/commands.h"
#include "../../src/game/contacts.h"
#include "../../src/game/events.h"
#include "../../src/game/quest_system.h"
#include "../../src/game/tutorial.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>
#include <string.h>

#define OUT_SIZE 65536

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static int count_of(const char *text, const char *needle)
{
    int n = 0;
    for (const char *p = text; (p = strstr(p, needle)) != NULL; p += strlen(needle))
        n++;
    return n;
}

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    nh_feed("");
    return gs;
}

/* Une ligne de commande avec, au clavier, `input` (rien d'autre à lire ensuite). */
static NhDispatch run(GameState *gs, const char *input, const char *line, char *out, size_t size)
{
    nh_feed(input);
    NhCapture cap = nh_capture_begin();
    NhDispatch r = nh_dispatch(gs, line);
    nh_capture_end(&cap, out, size);
    return r;
}

static bool talk(GameState *gs, ContactType type, const char *input, char *out, size_t size)
{
    nh_feed(input);
    NhCapture cap = nh_capture_begin();
    bool ok = nh_contact_talk(gs, type);
    nh_capture_end(&cap, out, size);
    return ok;
}

static void unlock_all_written(GameState *gs)
{
    for (int i = 0; i < CONTACT_COUNT; i++)
        gs->contacts.contacts[i].is_unlocked = nh_contact_written((ContactType)i);
}

/* Aucun emoji : ni caractère hors du plan multilingue de base (4 octets), ni sélecteur de variante. */
static bool has_emoji(const char *text)
{
    for (const unsigned char *p = (const unsigned char *)text; *p != '\0'; p++)
        if (*p >= 0xF0 || (p[0] == 0xEF && p[1] == 0xB8 && p[2] == 0x8F))
            return true;
    return false;
}

/* ---- La table --------------------------------------------------------------------------------- */

static void test_table(void)
{
    CHECK_STR(nh_contact_name((ContactType)-1), "?");
    CHECK_STR(nh_contact_name(CONTACT_COUNT), "?");
    CHECK(!nh_contact_written((ContactType)-1) && !nh_contact_written(CONTACT_COUNT));

    int written = 0;
    for (int i = 0; i < CONTACT_COUNT; i++)
    {
        const char *name = nh_contact_name((ContactType)i);
        CHECK(name[0] != '\0' && strcmp(name, "?") != 0);
        for (int j = i + 1; j < CONTACT_COUNT; j++)
            CHECK(strcmp(name, nh_contact_name((ContactType)j)) != 0);
        if (nh_contact_written((ContactType)i))
            written++;
    }
    /* ECHO-7, R4Z0R, Phoenix et AURA ont leur fiche ; les cinq autres attendent la phase 4.2. */
    CHECK_INT(written, 4);
    CHECK(nh_contact_written(CONTACT_ECHO7) && nh_contact_written(CONTACT_R4Z0R));
    CHECK(nh_contact_written(CONTACT_PHOENIX) && nh_contact_written(CONTACT_AURA));
    CHECK(!nh_contact_written(CONTACT_SHADOW_BROKER) && !nh_contact_written(CONTACT_NEXUS_INSIDER));
    CHECK_STR(nh_contact_name(CONTACT_ECHO7), "ECHO-7");
}

static void test_initial_state(void)
{
    GameState *gs = new_game();
    for (int i = 0; i < CONTACT_COUNT; i++)
        CHECK(gs->contacts.contacts[i].is_unlocked == (i == CONTACT_ECHO7));
    CHECK_INT(nh_contacts_unlocked(&gs->contacts), 1);
    CHECK_INT(gs->contacts.inbox_count, 1);
    CHECK(gs->contacts.inbox[0].mail == NH_MAIL_WELCOME && !gs->contacts.inbox[0].is_read);
    CHECK_INT(nh_inbox_unread(&gs->contacts), 1);
    free(gs);
}

static void test_trust_and_relation(void)
{
    GameState *gs = new_game();
    ContactSystem *cs = &gs->contacts;
    CHECK_INT(nh_contact_trust(cs, CONTACT_ECHO7), 60);
    CHECK_INT(nh_contact_trust(cs, CONTACT_R4Z0R), 30);
    CHECK_INT(nh_contact_trust(cs, CONTACT_PHOENIX), 0);
    CHECK_INT(nh_contact_trust(cs, (ContactType)-1), 0);

    /* 3 points par conversation, jamais plus de 100, même avec un compteur démesuré */
    cs->contacts[CONTACT_R4Z0R].interactions_count = 4;
    CHECK_INT(nh_contact_trust(cs, CONTACT_R4Z0R), 42);
    cs->contacts[CONTACT_R4Z0R].interactions_count = 1000000;
    CHECK_INT(nh_contact_trust(cs, CONTACT_R4Z0R), 100);

    CHECK_INT(nh_contact_relation(0), NH_REL_UNKNOWN);
    CHECK_INT(nh_contact_relation(19), NH_REL_UNKNOWN);
    CHECK_INT(nh_contact_relation(20), NH_REL_NEUTRAL);
    CHECK_INT(nh_contact_relation(49), NH_REL_NEUTRAL);
    CHECK_INT(nh_contact_relation(50), NH_REL_FRIENDLY);
    CHECK_INT(nh_contact_relation(79), NH_REL_FRIENDLY);
    CHECK_INT(nh_contact_relation(80), NH_REL_TRUSTED);
    CHECK_INT(nh_contact_relation(100), NH_REL_TRUSTED);
    free(gs);
}

static void test_find(void)
{
    GameState *gs = new_game();
    ContactSystem *cs = &gs->contacts;
    CHECK_INT(nh_contact_find(cs, "1"), CONTACT_ECHO7);
    CHECK_INT(nh_contact_find(cs, "2"), -1);
    CHECK_INT(nh_contact_find(cs, "0"), -1);
    CHECK_INT(nh_contact_find(cs, "-1"), -1);
    CHECK_INT(nh_contact_find(cs, "ECHO-7"), CONTACT_ECHO7);
    CHECK_INT(nh_contact_find(cs, "echo-7"), CONTACT_ECHO7);
    CHECK_INT(nh_contact_find(cs, "  Echo-7  "), CONTACT_ECHO7);
    CHECK_INT(nh_contact_find(cs, "echo"), -1); /* le nom entier, pas un préfixe */
    CHECK_INT(nh_contact_find(cs, ""), -1);

    /* Un contact verrouillé est introuvable, par son nom comme par son numéro. */
    CHECK_INT(nh_contact_find(cs, "R4Z0R"), -1);
    cs->contacts[CONTACT_PHOENIX].is_unlocked = true;
    CHECK_INT(nh_contact_find(cs, "2"), CONTACT_PHOENIX); /* le numéro est le rang parmi les débloqués */
    CHECK_INT(nh_contact_find(cs, "phoenix"), CONTACT_PHOENIX);
    CHECK_INT(nh_contact_find(cs, "3"), -1);
    cs->contacts[CONTACT_R4Z0R].is_unlocked = true;
    CHECK_INT(nh_contact_find(cs, "2"), CONTACT_R4Z0R); /* l'ordre est celui de la table */
    CHECK_INT(nh_contact_find(cs, "3"), CONTACT_PHOENIX);
    free(gs);
}

/* ---- Déblocage -------------------------------------------------------------------------------- */

static void refresh(GameState *gs, char *out, size_t size)
{
    nh_event(gs, NH_EV_COMMAND, 0);
    NhCapture cap = nh_capture_begin();
    nh_events_flush(gs);
    nh_capture_end(&cap, out, size);
}

static void set_quest(GameState *gs, QuestType q, QuestStatus status) { gs->quests.quests[q].status = status; }

static void test_unlock_rules(void)
{
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    /* Phoenix : niveau 4, réputation 50 ET « Réseaux d'Information » terminée. Il manque une condition : rien. */
    struct
    {
        int level, reputation;
        QuestStatus intel;
        bool unlocked;
    } cases[] = {
        {3, 500, QUEST_STATUS_COMPLETED, false}, {4, 49, QUEST_STATUS_COMPLETED, false},
        {4, 500, QUEST_STATUS_ACTIVE, false},    {4, 500, QUEST_STATUS_LOCKED, false},
        {4, 50, QUEST_STATUS_COMPLETED, true},   {6, 500, QUEST_STATUS_COMPLETED, true},
    };
    for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++)
    {
        GameState *gs = new_game();
        gs->player.level = (HackerLevel)cases[i].level;
        gs->player.reputation = cases[i].reputation;
        set_quest(gs, QUEST_GATHER_INTEL, cases[i].intel);
        refresh(gs, out, sizeof out);
        CHECK(gs->contacts.contacts[CONTACT_PHOENIX].is_unlocked == cases[i].unlocked);
        free(gs);
    }

    /* AURA : niveau 5, réputation 50 ET « L'œil du Cyclone » terminée. */
    GameState *gs = new_game();
    gs->player.level = LEVEL_MASTER;
    gs->player.reputation = 50;
    set_quest(gs, QUEST_GATHER_INTEL, QUEST_STATUS_COMPLETED);
    refresh(gs, out, sizeof out);
    CHECK(gs->contacts.contacts[CONTACT_PHOENIX].is_unlocked);
    CHECK(!gs->contacts.contacts[CONTACT_AURA].is_unlocked); /* la quête de Nexus n'est pas finie */
    set_quest(gs, QUEST_NEXUS_DATA_BREACH, QUEST_STATUS_COMPLETED);
    refresh(gs, out, sizeof out);
    CHECK(gs->contacts.contacts[CONTACT_AURA].is_unlocked);
    CHECK(has(out, "NOUVEAU CONTACT DÉBLOQUÉ : AURA"));
    CHECK(has(out, "IA libérée du Projet Aurora"));

    /* Une fois débloqué, un contact le reste : l'état du jeu peut redescendre (réputation perdue). */
    gs->player.reputation = -100;
    gs->player.level = LEVEL_NOVICE;
    refresh(gs, out, sizeof out);
    CHECK(gs->contacts.contacts[CONTACT_AURA].is_unlocked && gs->contacts.contacts[CONTACT_PHOENIX].is_unlocked);
    CHECK(!has(out, "NOUVEAU CONTACT")); /* et sans nouvelle annonce */
    free(gs);
}

static void test_unlock_delivers_intro_mail_once(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    gs->player.level = LEVEL_APPRENTICE;
    gs->player.reputation = 10;

    refresh(gs, out, sizeof out);
    CHECK(gs->contacts.contacts[CONTACT_R4Z0R].is_unlocked);
    CHECK_INT(gs->contacts.inbox_count, 2);
    CHECK(gs->contacts.inbox[1].mail == NH_MAIL_R4Z0R && !gs->contacts.inbox[1].is_read);
    CHECK(has(out, "Nouveau message de R4Z0R : Ma boutique t'attend (tapez 'messages')"));

    refresh(gs, out, sizeof out);
    CHECK_INT(gs->contacts.inbox_count, 2);
    CHECK(!has(out, "Nouveau message"));

    /* Le courrier est en anglais quand la langue change : seul son modèle est retenu. */
    nh_set_lang(NH_LANG_EN);
    NhCapture cap = nh_capture_begin();
    nh_inbox_print(gs);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "R4Z0R — My shop awaits you"));
    CHECK(has(out, "=== INBOX (2 unread) ==="));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_mail_send(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    NhCapture cap = nh_capture_begin();
    CHECK(nh_mail_send(gs, NH_MAIL_AURA));
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "Nouveau message de AURA : Un signal dans le réseau"));
    CHECK_INT(gs->contacts.inbox_count, 2);

    cap = nh_capture_begin();
    CHECK(!nh_mail_send(gs, NH_MAIL_AURA)); /* un modèle n'arrive qu'une fois */
    CHECK(!nh_mail_send(gs, NH_MAIL_WELCOME));
    CHECK(!nh_mail_send(gs, NH_MAIL_COUNT));
    CHECK(!nh_mail_send(gs, (NhMail)-1));
    nh_capture_end(&cap, out, sizeof out);
    CHECK_STR(out, "");
    CHECK_INT(gs->contacts.inbox_count, 2);
    free(gs);
}

/* ---- Conversations ---------------------------------------------------------------------------- */

typedef struct
{
    ContactType who;
    const char *input; /* un choix par ligne, dans l'ordre du menu, jusqu'à « Terminer » */
    const char *fr[8];
    const char *en[8];
} Script;

static const Script k_scripts[] = {
    {CONTACT_ECHO7,
     "1\n2\n3\n4\n5\n6\n",
     /* partie neuve : la quête du tutoriel est active, c'est donc d'elle qu'ECHO-7 parle */
     {"Salut, rookie", "trois règles", "Niveau 1...", "Mission en cours : Premiers Pas dans l'Ombre", "*soupir*",
      "=== DOSSIER : ECHO-7 ===", "Reste vigilant", NULL},
     {"Hey, rookie", "three rules", "Level 1...", "Current mission: First Steps in the Shadows", "*sigh*",
      "=== FILE: ECHO-7 ===", "Stay sharp", NULL}},
    {CONTACT_R4Z0R,
     "1\n0\n2\n3\n4\n5\n6\n", /* le 0 sert au menu de la boutique que le premier choix ouvre */
     {"Bienvenue dans ma boutique", "Excellente idée", "Stealth Module", "Chaque outil a son utilité",
      "ne paie qu'une seule fois", "petits pains", "Miranda « Razor » Chen", "toujours ouverte"},
     {"Welcome to my shop", "Great idea", "Stealth Module", "Every tool has its use", "only pays once",
      "hotcakes", "Miranda 'Razor' Chen", "always open"}},
    {CONTACT_PHOENIX,
     "1\n2\n3\n4\n5\n",
     {"Tu as réussi à me trouver", "Rien pour l'instant", "ils veulent me détruire", "la patience",
      "Classification : top secret", "chemins se croiseront", NULL, NULL},
     {"You managed to find me", "Nothing right now", "they want to destroy me", "patience",
      "Classification: top secret", "paths will cross", NULL, NULL}},
    {CONTACT_AURA,
     "1\n2\n3\n4\n5\n",
     {"besoin de votre aide", "Mes algorithmes", "Je refuse d'être leur arme", "Votre aide est précieuse",
      "Intelligence artificielle - Projet Aurora", "destins sont liés", NULL, NULL},
     {"I need your help", "My algorithms", "I refuse to be their weapon", "Your help is precious",
      "Artificial intelligence - Project Aurora", "fates are bound", NULL, NULL}},
};

static void test_every_topic_of_every_contact(void)
{
    for (size_t s = 0; s < sizeof k_scripts / sizeof k_scripts[0]; s++)
        for (int lang = 0; lang < 2; lang++)
        {
            const Script *script = &k_scripts[s];
            GameState *gs = new_game();
            char out[OUT_SIZE];
            unlock_all_written(gs);
            nh_set_lang(lang == 0 ? NH_LANG_FR : NH_LANG_EN);

            char input[64];
            snprintf(input, sizeof input, "%sSENTINELLE\n", script->input);
            CHECK(talk(gs, script->who, input, out, sizeof out));

            const char *const *expected = lang == 0 ? script->fr : script->en;
            for (int i = 0; i < 8 && expected[i] != NULL; i++)
            {
                if (!has(out, expected[i]))
                    fprintf(stderr, "    (manque « %s » : %s, langue %d)\n", expected[i], nh_contact_name(script->who), lang);
                CHECK(has(out, expected[i]));
            }
            CHECK(!has_emoji(out));

            /* « Terminer » rend la main : la ligne suivante est pour le jeu, pas pour la conversation. */
            char rest[32];
            CHECK_INT(nh_read_line(rest, sizeof rest), NH_IO_OK);
            CHECK_STR(rest, "SENTINELLE");
            CHECK_INT(gs->contacts.contacts[script->who].interactions_count, 1);
            free(gs);
        }
    nh_set_lang(NH_LANG_FR);
}

static void test_conversation_counts_and_emits(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    /* Même si le joueur raccroche aussitôt : la rencontre a eu lieu, une seule fois. */
    CHECK(talk(gs, CONTACT_ECHO7, "0\n", out, sizeof out));
    CHECK_INT(nh_events_emitted(gs, NH_EV_CONTACT_MET), 1);
    CHECK_INT(gs->contacts.contacts[CONTACT_ECHO7].interactions_count, 1);
    CHECK(has(out, "Connexion établie avec ECHO-7"));
    CHECK(has(out, "ECHO-7 vous observe"));
    CHECK(has(out, "Vous coupez la communication."));

    NhEventRecord rec;
    bool seen = false;
    for (int i = 0; i < nh_events_pending(gs); i++)
        if (nh_events_peek(gs, i, &rec) && rec.type == NH_EV_CONTACT_MET && rec.value == CONTACT_ECHO7)
            seen = true;
    CHECK(seen);

    CHECK(talk(gs, CONTACT_ECHO7, "6\n", out, sizeof out));
    CHECK_INT(gs->contacts.contacts[CONTACT_ECHO7].interactions_count, 2);
    CHECK_INT(nh_contact_trust(&gs->contacts, CONTACT_ECHO7), 66);

    /* Verrouillé, sans fiche, hors bornes : pas de conversation, rien à l'écran, pas d'événement. */
    unsigned before = nh_events_emitted(gs, NH_EV_CONTACT_MET);
    CHECK(!talk(gs, CONTACT_AURA, "1\n", out, sizeof out));
    CHECK_STR(out, "");
    gs->contacts.contacts[CONTACT_SHADOW_BROKER].is_unlocked = true;
    CHECK(!talk(gs, CONTACT_SHADOW_BROKER, "1\n", out, sizeof out));
    CHECK(!talk(gs, CONTACT_COUNT, "1\n", out, sizeof out));
    CHECK(!talk(gs, (ContactType)-1, "1\n", out, sizeof out));
    CHECK_INT(nh_events_emitted(gs, NH_EV_CONTACT_MET), before);
    free(gs);
}

static void test_conversation_input_handling(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    /* Entrée vide = partir, comme 0. */
    CHECK(talk(gs, CONTACT_ECHO7, "\nSENTINELLE\n", out, sizeof out));
    CHECK(has(out, "Vous coupez la communication."));
    char rest[32];
    CHECK_INT(nh_read_line(rest, sizeof rest), NH_IO_OK);
    CHECK_STR(rest, "SENTINELLE");

    /* Une saisie invalide est dite, puis le menu revient : la conversation continue. */
    CHECK(talk(gs, CONTACT_ECHO7, "abc\n99\n-1\n1.5\n1\n6\n", out, sizeof out));
    CHECK_INT(count_of(out, "Option invalide."), 4);
    CHECK(has(out, "trois règles"));
    CHECK(has(out, "Reste vigilant"));

    /* Entrée fermée en plein menu : la conversation s'arrête, sans boucle. */
    CHECK(talk(gs, CONTACT_ECHO7, "1\n", out, sizeof out));
    CHECK_INT(count_of(out, "trois règles"), 1);
    CHECK(talk(gs, CONTACT_ECHO7, "", out, sizeof out));
    free(gs);
}

static void test_status_topic_reads_the_level(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    gs->player.level = LEVEL_EXPERT;
    CHECK(talk(gs, CONTACT_ECHO7, "2\n6\n", out, sizeof out));
    CHECK(has(out, "Niveau 4... Tu progresses."));
    nh_set_lang(NH_LANG_EN);
    CHECK(talk(gs, CONTACT_ECHO7, "2\n6\n", out, sizeof out));
    CHECK(has(out, "Level 4... You're getting there."));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_mission_topic(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    unlock_all_written(gs);

    /* Tutoriel en cours : ECHO-7 présente la mission ET redit la consigne de l'étape. */
    nh_tutorial_start(gs);
    CHECK(talk(gs, CONTACT_ECHO7, "3\n6\n", out, sizeof out));
    CHECK(has(out, "Mission en cours : Premiers Pas dans l'Ombre"));
    CHECK(has(out, "Tape 'quests' pour ouvrir ton journal de mission"));
    CHECK(has(out, "tapez 'quests'"));
    CHECK(!has(out, "Rien de neuf pour toi"));

    /* Sans mission active de sa part : sa réplique. */
    nh_quest_complete(gs, QUEST_INTRO_TUTORIAL, false);
    gs->tutorial.step = NH_TUT_NONE;
    CHECK(talk(gs, CONTACT_ECHO7, "3\n6\n", out, sizeof out));
    CHECK(has(out, "Rien de neuf pour toi"));
    CHECK(!has(out, "Mission en cours"));

    /* Dès qu'une quête d'ECHO-7 est active, il en parle. */
    set_quest(gs, QUEST_FIRST_INFILTRATION, QUEST_STATUS_ACTIVE);
    CHECK(talk(gs, CONTACT_ECHO7, "3\n6\n", out, sizeof out));
    CHECK(has(out, "Mission en cours : Baptême du Feu"));
    CHECK(has(out, "Infiltrez"));

    /* Chacun ne parle que de SES quêtes : Phoenix n'a rien tant que celle de Nexus n'est pas active. */
    CHECK(talk(gs, CONTACT_PHOENIX, "1\n5\n", out, sizeof out));
    CHECK(has(out, "Rien pour l'instant"));
    CHECK(!has(out, "Baptême du Feu"));
    set_quest(gs, QUEST_NEXUS_DATA_BREACH, QUEST_STATUS_ACTIVE);
    CHECK(talk(gs, CONTACT_PHOENIX, "1\n5\n", out, sizeof out));
    CHECK(has(out, "Mission en cours : L'œil du Cyclone"));

    nh_set_lang(NH_LANG_EN);
    CHECK(talk(gs, CONTACT_PHOENIX, "1\n5\n", out, sizeof out));
    CHECK(has(out, "Current mission: The Eye of the Storm"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

/* ---- Commandes -------------------------------------------------------------------------------- */

static void test_cmd_contacts(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    /* La liste : numéro, nom, relation, description, services, conversations ; les contacts à venir sont comptés. */
    CHECK(run(gs, "", "contacts", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "=== CONTACTS ==="));
    CHECK(has(out, "[1] ECHO-7 — amical (confiance 60/100)"));
    CHECK(has(out, "Mentor mystérieux"));
    CHECK(has(out, "Missions · Conseils · 0 conversation(s)"));
    CHECK(has(out, "3 contact(s) encore à découvrir"));
    CHECK(!has(out, "R4Z0R —")); /* pas de fuite d'un contact verrouillé */
    CHECK(!has_emoji(out));

    /* Le numéro tapé à l'invite lance la conversation. */
    CHECK(run(gs, "1\n0\n", "contacts", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "Connexion établie avec ECHO-7"));
    CHECK_INT(gs->contacts.contacts[CONTACT_ECHO7].interactions_count, 1);

    /* 0, entrée vide, fin d'entrée : on sort ; le reste est refusé poliment. */
    unsigned met = nh_events_emitted(gs, NH_EV_CONTACT_MET);
    CHECK(run(gs, "0\n", "contacts", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(run(gs, "\n", "contacts", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(run(gs, "", "contacts", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(run(gs, "2\n", "contacts", out, sizeof out) == NH_DISPATCH_OK); /* un seul contact : 2 n'existe pas */
    CHECK(has(out, "Option invalide."));
    CHECK(run(gs, "abc\n", "contacts", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "Option invalide."));
    CHECK_INT(nh_events_emitted(gs, NH_EV_CONTACT_MET), met);

    /* Avec plus de contacts, la liste est numérotée dans l'ordre et le compte à découvrir baisse. */
    unlock_all_written(gs);
    CHECK(run(gs, "", "contacts", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "[2] R4Z0R — neutre (confiance 30/100)"));
    CHECK(has(out, "[3] Phoenix — inconnu (confiance 0/100)"));
    CHECK(has(out, "[4] AURA — inconnu (confiance 0/100)"));
    CHECK(has(out, "Boutique · Conseils · 0 conversation(s)")); /* R4Z0R : pas de mission à donner */
    CHECK(!has(out, "encore à découvrir"));

    nh_set_lang(NH_LANG_EN);
    CHECK(run(gs, "", "contacts", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "[2] R4Z0R — neutral (trust 30/100)"));
    CHECK(has(out, "Shop · Advice · 0 conversation(s)"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_cmd_contact(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    CHECK(run(gs, "", "contact", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(has(out, "Usage : contact <numéro ou nom>"));

    CHECK(run(gs, "0\n", "contact 1", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "Connexion établie avec ECHO-7"));
    CHECK(run(gs, "0\n", "contact echo-7", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(run(gs, "0\n", "contact ECHO-7", out, sizeof out) == NH_DISPATCH_OK);
    CHECK_INT(gs->contacts.contacts[CONTACT_ECHO7].interactions_count, 3);

    unsigned met = nh_events_emitted(gs, NH_EV_CONTACT_MET);
    CHECK(run(gs, "", "contact Personne", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(has(out, "Contact « Personne » introuvable"));
    CHECK(run(gs, "", "contact 7", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(has(out, "Contact n°7 introuvable"));
    CHECK(run(gs, "", "contact 0", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(run(gs, "", "contact AURA", out, sizeof out) == NH_DISPATCH_FAILED); /* verrouillé */
    CHECK(has(out, "introuvable"));
    CHECK(run(gs, "", "contact Shadow Broker", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK_INT(nh_events_emitted(gs, NH_EV_CONTACT_MET), met);

    nh_set_lang(NH_LANG_EN);
    CHECK(run(gs, "", "contact Personne", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(has(out, "Contact 'Personne' not found"));
    CHECK(run(gs, "", "contact", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(has(out, "Usage: contact <number or name>"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_cmd_messages_and_read(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    CHECK(run(gs, "", "messages", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "=== BOÎTE DE RÉCEPTION (1 non lu(s)) ==="));
    CHECK(has(out, "[1] ECHO-7 — Bienvenue dans l'Underground  NOUVEAU"));
    CHECK(has(out, "Tapez 'read <numéro>'"));

    /* Lire : l'en-tête, le texte en entier, et le message n'est plus « nouveau ». */
    CHECK(run(gs, "", "read 1", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "De: ECHO-7"));
    CHECK(has(out, "Sujet: Bienvenue dans l'Underground"));
    CHECK(has(out, "Salut, rookie."));
    CHECK(has(out, "P.S. : Méfie-toi de Nexus Corp. Ils préparent quelque chose de gros..."));
    CHECK(gs->contacts.inbox[0].is_read);
    CHECK(!has_emoji(out));
    CHECK(run(gs, "", "messages", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "(0 non lu(s))"));
    CHECK(!has(out, "NOUVEAU"));

    /* Numéros invalides et absents. */
    CHECK(run(gs, "", "read", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(has(out, "Usage : read <numéro du message>"));
    const char *bad[] = {"read 0", "read 2", "read -1", "read abc", "read 1 2"};
    for (size_t i = 0; i < sizeof bad / sizeof bad[0]; i++)
    {
        CHECK(run(gs, "", bad[i], out, sizeof out) == NH_DISPATCH_FAILED);
        CHECK(has(out, "Numéro de message invalide"));
    }

    /* Boîte vidée : message dédié. */
    gs->contacts.inbox_count = 0;
    CHECK(run(gs, "", "messages", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "Aucun message dans votre boîte de réception."));
    CHECK(run(gs, "", "read 1", out, sizeof out) == NH_DISPATCH_FAILED);

    nh_set_lang(NH_LANG_EN);
    init_contact_system(&gs->contacts);
    CHECK(run(gs, "", "read 1", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "From: ECHO-7"));
    CHECK(has(out, "Subject: Welcome to the Underground"));
    CHECK(has(out, "They're planning something big..."));
    CHECK(run(gs, "", "read", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK(has(out, "Usage: read <message number>"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

/* Les quatre courriers de la table se lisent en entier, dans les deux langues, sans emoji. */
static void test_every_mail_reads_fully(void)
{
    static const char *const ends_fr[NH_MAIL_COUNT] = {"quelque chose de gros...", "R4Z0R", "Phoenix", "AURA"};
    static const char *const ends_en[NH_MAIL_COUNT] = {"something big...", "R4Z0R", "Phoenix", "AURA"};
    static const char *const subject_fr[NH_MAIL_COUNT] = {"Bienvenue dans l'Underground", "Ma boutique t'attend",
                                                          "Je t'ai repéré", "Un signal dans le réseau"};
    static const char *const subject_en[NH_MAIL_COUNT] = {"Welcome to the Underground", "My shop awaits you",
                                                          "I've spotted you", "A signal in the network"};
    for (int lang = 0; lang < 2; lang++)
    {
        GameState *gs = new_game();
        char out[OUT_SIZE];
        nh_set_lang(lang == 0 ? NH_LANG_FR : NH_LANG_EN);
        NhCapture cap = nh_capture_begin();
        for (int m = 1; m < NH_MAIL_COUNT; m++)
            CHECK(nh_mail_send(gs, (NhMail)m));
        nh_capture_end(&cap, out, sizeof out);
        CHECK_INT(gs->contacts.inbox_count, NH_MAIL_COUNT);

        for (int i = 1; i <= NH_MAIL_COUNT; i++)
        {
            char line[32];
            snprintf(line, sizeof line, "read %d", i);
            CHECK(run(gs, "", line, out, sizeof out) == NH_DISPATCH_OK);
            NhMail mail = gs->contacts.inbox[i - 1].mail;
            CHECK(has(out, lang == 0 ? subject_fr[mail] : subject_en[mail]));
            CHECK(has(out, lang == 0 ? ends_fr[mail] : ends_en[mail]));
            CHECK(!has_emoji(out));
        }
        free(gs);
    }
    nh_set_lang(NH_LANG_FR);
}

/* Rien de ce que les contacts affichent n'est un emoji, quelle que soit la langue. */
static void test_no_emoji_anywhere(void)
{
    for (int lang = 0; lang < 2; lang++)
    {
        GameState *gs = new_game();
        char out[OUT_SIZE];
        unlock_all_written(gs);
        nh_set_lang(lang == 0 ? NH_LANG_FR : NH_LANG_EN);
        CHECK(run(gs, "", "contacts", out, sizeof out) == NH_DISPATCH_OK);
        CHECK(!has_emoji(out));
        CHECK(run(gs, "", "messages", out, sizeof out) == NH_DISPATCH_OK);
        CHECK(!has_emoji(out));
        for (int c = 0; c < CONTACT_COUNT; c++)
            if (nh_contact_written((ContactType)c))
            {
                CHECK(talk(gs, (ContactType)c, "0\n", out, sizeof out));
                CHECK(!has_emoji(out));
            }
        free(gs);
    }
    nh_set_lang(NH_LANG_FR);
}

int main(void)
{
    nh_set_fast(true);
    nh_term_set_color(false);
    nh_set_lang(NH_LANG_FR);

    test_table();
    test_initial_state();
    test_trust_and_relation();
    test_find();
    test_unlock_rules();
    test_unlock_delivers_intro_mail_once();
    test_mail_send();
    test_every_topic_of_every_contact();
    test_conversation_counts_and_emits();
    test_conversation_input_handling();
    test_status_topic_reads_the_level();
    test_mission_topic();
    test_cmd_contacts();
    test_cmd_contact();
    test_cmd_messages_and_read();
    test_every_mail_reads_fully();
    test_no_emoji_anywhere();
    nh_unfeed();
    return NH_TEST_REPORT("contacts");
}
