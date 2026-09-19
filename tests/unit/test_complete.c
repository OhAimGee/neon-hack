#include "nh_test.h"

#include "nh_feed.h"

#include "../../src/game/commands.h"
#include "../../src/game/complete.h"
#include "../../src/game/contacts.h"
#include "../../src/game/world.h"
#include "../../src/i18n/i18n.h"

#include <stdlib.h>
#include <string.h>

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    nh_feed("");
    return gs;
}

static NhCompletions complete(GameState *gs, const char *before)
{
    NhCompletions c;
    memset(&c, 0, sizeof c);
    nh_complete_line(gs, before, &c);
    return c;
}

static bool offers(const NhCompletions *c, const char *word)
{
    for (size_t i = 0; i < c->count; i++)
        if (strcmp(c->items[i], word) == 0)
            return true;
    return false;
}

/* Nombre de commandes que le joueur peut taper maintenant et qui figurent dans l'aide. */
static size_t usable_commands(const GameState *gs)
{
    size_t total;
    const NhCommand *table = nh_commands(&total);
    size_t n = 0;
    for (size_t i = 0; i < total; i++)
        if (!table[i].hidden && nh_command_available(gs, &table[i]))
            n++;
    return n;
}

static void test_command_names(void)
{
    GameState *gs = new_game();

    /* Ligne vide : toutes les commandes utilisables, et rien d'autre. */
    NhCompletions c = complete(gs, "");
    CHECK_INT(c.start, 0);
    CHECK(c.space);
    CHECK_INT(c.count, usable_commands(gs));
    CHECK(offers(&c, "scan") && offers(&c, "help") && offers(&c, "shop"));
    CHECK(!offers(&c, "bruteforce")); /* pas encore débloquée */
    CHECK(!offers(&c, "advhack"));    /* niveau 3 requis */
    CHECK(!offers(&c, "stealth"));    /* commande cachée : absente de l'aide, donc de TAB */

    c = complete(gs, "sc");
    CHECK_INT(c.count, 1);
    CHECK(offers(&c, "scan"));
    c = complete(gs, "SC"); /* sans tenir compte de la casse */
    CHECK(offers(&c, "scan"));
    c = complete(gs, "sh");
    CHECK(offers(&c, "shop") && c.count == 1);
    c = complete(gs, "zz");
    CHECK_INT(c.count, 0);

    /* Les blancs de tête ne comptent pas, mais le point de départ les saute. */
    c = complete(gs, "  sc");
    CHECK_INT(c.start, 2);
    CHECK(offers(&c, "scan"));

    /* « stea » : stealthmode (avancée, niveau 0) mais pas la commande cachée stealth. */
    c = complete(gs, "stea");
    CHECK(offers(&c, "stealthmode") && !offers(&c, "stealth"));

    /* Une commande verrouillée apparaît dès qu'elle est débloquée (ou que le niveau le permet). */
    gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;
    c = complete(gs, "brute");
    CHECK(offers(&c, "bruteforce") && c.count == 1);
    gs->player.level = 3;
    c = complete(gs, "adv");
    CHECK(offers(&c, "advhack"));

    /* Les alias ne servent que s'aucun nom officiel ne convient. */
    c = complete(gs, "qu");
    CHECK(offers(&c, "quit"));
    CHECK(!offers(&c, "exit"));
    c = complete(gs, "exi");
    CHECK_INT(c.count, 1);
    CHECK(offers(&c, "exit"));
    c = complete(gs, "up"); /* uploadvirus est verrouillée : ni lui ni son alias */
    CHECK_INT(c.count, 0);
    gs->player.commands_unlocked[CMD_UPLOAD_VIRUS] = true;
    c = complete(gs, "upload_");
    CHECK(offers(&c, "upload_virus") && c.count == 1);
    c = complete(gs, "up");
    CHECK(offers(&c, "uploadvirus") && !offers(&c, "upload_virus"));

    free(gs);
}

static void test_system_arguments(void)
{
    GameState *gs = new_game();
    gs->player.commands_unlocked[CMD_BRUTEFORCE] = true;

    /* Rien n'est proposé tant qu'aucun système n'est découvert. */
    for (int i = 0; i < NH_MAX_NODES; i++)
        gs->nodes[i].is_discovered = false;
    NhCompletions c = complete(gs, "bruteforce ");
    CHECK_INT(c.count, 0);

    nh_world_discover(gs->nodes, 1);
    int discovered = 0;
    for (int i = 0; i < NH_MAX_NODES; i++)
        discovered += gs->nodes[i].is_discovered ? 1 : 0;
    CHECK(discovered >= 1);

    c = complete(gs, "bruteforce ");
    CHECK_INT(c.start, 11);
    CHECK(!c.space); /* un argument final : pas d'espace derrière */
    CHECK_INT(c.count, discovered);
    CHECK(offers(&c, "localhost"));
    CHECK(!offers(&c, "nexus-mainframe")); /* pas encore découvert : on ne le révèle pas */
    c = complete(gs, "bruteforce LO");
    CHECK_INT(c.start, 11);
    CHECK(offers(&c, "localhost") && c.count == 1);
    c = complete(gs, "bruteforce   lo"); /* plusieurs espaces */
    CHECK_INT(c.start, 13);
    CHECK(offers(&c, "localhost"));

    /* Les autres commandes qui visent un système. */
    static const char *const targets[] = {"backdoor", "traceroute", "exploit", "uploadvirus", "aihack"};
    for (size_t i = 0; i < sizeof targets / sizeof targets[0]; i++)
    {
        const NhCommand *cmd = nh_find_command(targets[i]);
        CHECK(cmd != NULL && cmd->arg == NH_ARG_SYSTEM);
    }
    static const char *const advanced[] = {"advhack", "aiassist", "socialeng", "analyzedefenses", "temporalhack"};
    for (size_t i = 0; i < sizeof advanced / sizeof advanced[0]; i++)
    {
        const NhCommand *cmd = nh_find_command(advanced[i]);
        CHECK(cmd != NULL && cmd->arg == NH_ARG_SYSTEM);
    }

    /* Une commande verrouillée ne propose pas ses arguments : on ne devine pas ce qu'elle ferait. */
    c = complete(gs, "uploadvirus lo");
    CHECK_INT(c.count, 0);
    /* Ni une commande inconnue, ni une commande sans argument. */
    c = complete(gs, "frobnicate lo");
    CHECK_INT(c.count, 0);
    c = complete(gs, "scan lo");
    CHECK_INT(c.count, 0);
    c = complete(gs, "shop lo");
    CHECK_INT(c.count, 0);

    /* Le texte libre de decrypt n'est jamais complété. */
    gs->player.commands_unlocked[CMD_DECRYPT] = true;
    c = complete(gs, "decrypt lo");
    CHECK_INT(c.count, 0);
    c = complete(gs, "quantumdecrypt lo");
    CHECK_INT(c.count, 0);

    free(gs);
}

static void test_contacts_and_messages(void)
{
    GameState *gs = new_game();

    /* Au départ, seul ECHO-7 est débloqué. */
    NhCompletions c = complete(gs, "contact ");
    CHECK_INT(c.start, 8);
    CHECK(offers(&c, "ECHO-7") && c.count == 1);
    c = complete(gs, "contact e");
    CHECK(offers(&c, "ECHO-7"));
    c = complete(gs, "contact r");
    CHECK_INT(c.count, 0); /* R4Z0R n'est pas encore rencontrable */

    gs->contacts.contacts[CONTACT_R4Z0R].is_unlocked = true;
    gs->contacts.contacts[CONTACT_SHADOW_BROKER].is_unlocked = true;
    c = complete(gs, "contact ");
    CHECK_INT(c.count, 3);
    c = complete(gs, "contact r4");
    CHECK(offers(&c, "R4Z0R") && c.count == 1);

    /* Un nom avec une espace : l'argument est tout le reste de la ligne. */
    c = complete(gs, "contact sha");
    CHECK(offers(&c, "Shadow Broker") && c.count == 1);
    CHECK_INT(c.start, 8);
    c = complete(gs, "contact Shadow B");
    CHECK(offers(&c, "Shadow Broker"));
    CHECK_INT(c.start, 8);

    /* `contacts` (le pluriel) n'a pas d'argument. */
    c = complete(gs, "contacts ");
    CHECK_INT(c.count, 0);
    c = complete(gs, "contact");
    CHECK(offers(&c, "contact") && offers(&c, "contacts")); /* c'est encore le nom de la commande */
    CHECK(c.space);

    /* Messages : les numéros de la boîte de réception. */
    gs->contacts.inbox_count = 0;
    c = complete(gs, "read ");
    CHECK_INT(c.count, 0);
    gs->contacts.inbox_count = 12;
    c = complete(gs, "read ");
    CHECK_INT(c.count, 12);
    CHECK(offers(&c, "1") && offers(&c, "12") && !offers(&c, "13"));
    c = complete(gs, "read 1");
    CHECK_INT(c.count, 4); /* 1, 10, 11, 12 */
    CHECK(offers(&c, "1") && offers(&c, "10") && offers(&c, "11") && offers(&c, "12"));
    c = complete(gs, "read 2");
    CHECK_INT(c.count, 1);

    free(gs);
}

static void test_english_uses_same_commands(void)
{
    /* Les noms de commandes ne sont pas traduits : la complétion ne dépend pas de la langue. */
    GameState *gs = new_game();
    nh_set_lang(NH_LANG_EN);
    NhCompletions c = complete(gs, "sc");
    CHECK(offers(&c, "scan"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

int main(void)
{
    test_command_names();
    test_system_arguments();
    test_contacts_and_messages();
    test_english_uses_same_commands();
    return NH_TEST_REPORT("complete");
}
