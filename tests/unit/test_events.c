#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"

#include "../../src/core/platform.h"
#include "../../src/game/commands.h"
#include "../../src/game/events.h"
#include "../../src/game/progression.h"
#include "../../src/game/quest_system.h"
#include "../../src/game/shop.h"
#include "../../src/game/world.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    nh_feed(""); /* jamais d'attente sur le vrai clavier */
    return gs;
}

static NhDispatch run(GameState *gs, const char *line, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    NhDispatch r = nh_dispatch(gs, line);
    nh_capture_end(&cap, out, size);
    return r;
}

/* Un espion : ce que le bus livre, dans l'ordre. */
typedef struct
{
    GameState *gs;
    NhEventRecord seen[1024];
    int count;
    int depth;     /* imbrication des appels à l'espion */
    int max_depth;
    /* comportement de l'espion */
    NhEvent on_type;   /* réagit à ce type… */
    NhEvent then_emit; /* …en émettant celui-ci */
    int then_value;
    bool reflush;      /* … et en demandant une livraison (doit rester sans effet) */
    bool forever;      /* réémet ce qu'il reçoit, sans fin */
} Spy;

static void spy_observer(void *ctx, const NhEventRecord *e)
{
    Spy *s = ctx;
    s->depth++;
    if (s->depth > s->max_depth)
        s->max_depth = s->depth;
    if (s->count < (int)(sizeof s->seen / sizeof s->seen[0]))
        s->seen[s->count] = *e;
    s->count++;
    if (s->forever)
        nh_event(s->gs, e->type, e->value);
    else if (e->type == s->on_type && s->then_value >= 0)
    {
        nh_event(s->gs, s->then_emit, s->then_value);
        if (s->reflush)
            nh_events_flush(s->gs);
    }
    s->depth--;
}

static Spy *spy_on(GameState *gs)
{
    Spy *s = calloc(1, sizeof *s);
    s->gs = gs;
    s->then_value = -1; /* rien à émettre */
    gs->events.observer = spy_observer;
    gs->events.observer_ctx = s;
    return s;
}

static void test_record_and_peek(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);

    CHECK_INT(nh_events_pending(gs), 0);
    nh_event(gs, NH_EV_ITEM_BOUGHT, 3);
    nh_event(gs, NH_EV_CONTACT_MET, 4);

    /* Différé : rien n'est livré tant que personne n'a demandé la livraison. */
    CHECK_INT(spy->count, 0);
    CHECK_INT(nh_events_pending(gs), 2);

    NhEventRecord rec;
    CHECK(nh_events_peek(gs, 0, &rec));
    CHECK_INT(rec.type, NH_EV_ITEM_BOUGHT);
    CHECK_INT(rec.value, 3);
    CHECK(nh_events_peek(gs, 1, &rec));
    CHECK_INT(rec.type, NH_EV_CONTACT_MET);
    CHECK_INT(rec.value, 4);
    CHECK(!nh_events_peek(gs, 2, &rec));
    CHECK(!nh_events_peek(gs, -1, &rec));

    CHECK_INT(nh_events_emitted(gs, NH_EV_ITEM_BOUGHT), 1);
    CHECK_INT(nh_events_emitted(gs, NH_EV_CONTACT_MET), 1);
    CHECK_INT(nh_events_emitted(gs, NH_EV_LEVEL_UP), 0);

    /* Type inconnu : ignoré, sans rien casser. */
    nh_event(gs, NH_EV_COUNT, 1);
    nh_event(gs, (NhEvent)-1, 1);
    nh_event(gs, (NhEvent)9999, 1);
    CHECK_INT(nh_events_pending(gs), 2);
    CHECK_INT(nh_events_emitted(gs, NH_EV_COUNT), 0);
    CHECK_INT(nh_events_emitted(gs, (NhEvent)-1), 0);

    free(spy);
    free(gs);
}

static void test_flush_is_fifo_and_empties(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);

    nh_event(gs, NH_EV_MILESTONE, 10);
    nh_event(gs, NH_EV_REPUTATION, 20);
    nh_event(gs, NH_EV_MILESTONE, 30);
    nh_events_flush(gs);

    CHECK_INT(spy->count, 3);
    CHECK_INT(spy->seen[0].type, NH_EV_MILESTONE);
    CHECK_INT(spy->seen[0].value, 10);
    CHECK_INT(spy->seen[1].type, NH_EV_REPUTATION);
    CHECK_INT(spy->seen[1].value, 20);
    CHECK_INT(spy->seen[2].value, 30);
    CHECK_INT(nh_events_pending(gs), 0);
    CHECK_INT(gs->events.dropped, 0);

    /* Une seconde livraison ne rejoue rien. */
    nh_events_flush(gs);
    CHECK_INT(spy->count, 3);
    /* Le total émis survit à la livraison. */
    CHECK_INT(nh_events_emitted(gs, NH_EV_MILESTONE), 2);

    free(spy);
    free(gs);
}

/* Un abonné peut émettre : c'est livré dans la foulée, sans jamais rappeler l'abonné en cours. */
static void test_emission_during_flush(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);
    spy->on_type = NH_EV_LEVEL_UP;
    spy->then_emit = NH_EV_REPUTATION;
    spy->then_value = 7;
    spy->reflush = true;

    nh_event(gs, NH_EV_LEVEL_UP, 2);
    nh_event(gs, NH_EV_MILESTONE, 1);
    nh_events_flush(gs);

    /* ordre : ceux qui attendaient déjà, puis ce qui est né pendant la livraison */
    CHECK_INT(spy->count, 3);
    CHECK_INT(spy->seen[0].type, NH_EV_LEVEL_UP);
    CHECK_INT(spy->seen[1].type, NH_EV_MILESTONE);
    CHECK_INT(spy->seen[2].type, NH_EV_REPUTATION);
    CHECK_INT(spy->seen[2].value, 7);
    CHECK_INT(spy->max_depth, 1); /* pas de récursion, malgré la demande de livraison de l'abonné */
    CHECK_INT(nh_events_pending(gs), 0);
    CHECK(!gs->events.flushing);

    free(spy);
    free(gs);
}

static void test_queue_overflow_drops_newest(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);

    for (int i = 0; i < NH_EVENT_QUEUE + 5; i++)
        nh_event(gs, NH_EV_REPUTATION, i);

    CHECK_INT(nh_events_pending(gs), NH_EVENT_QUEUE);
    CHECK_INT(gs->events.dropped, 5);
    CHECK_INT(nh_events_emitted(gs, NH_EV_REPUTATION), NH_EVENT_QUEUE + 5); /* les perdus sont comptés */

    nh_events_flush(gs);
    CHECK_INT(spy->count, NH_EVENT_QUEUE);
    CHECK_INT(spy->seen[0].value, 0); /* on garde les plus anciens */
    CHECK_INT(spy->seen[NH_EVENT_QUEUE - 1].value, NH_EVENT_QUEUE - 1);

    /* Après la livraison, la file est de nouveau utilisable. */
    nh_event(gs, NH_EV_ITEM_BOUGHT, 1);
    CHECK_INT(nh_events_pending(gs), 1);

    free(spy);
    free(gs);
}

/* La file est circulaire : l'ordre est conservé quand elle tourne. */
static void test_ring_wraps_around(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);

    for (int round = 0; round < 7; round++)
    {
        for (int i = 0; i < 20; i++)
            nh_event(gs, NH_EV_REPUTATION, round * 100 + i);
        NhEventRecord rec;
        CHECK(nh_events_peek(gs, 0, &rec));
        CHECK_INT(rec.value, round * 100);
        CHECK(nh_events_peek(gs, 19, &rec));
        CHECK_INT(rec.value, round * 100 + 19);
        nh_events_flush(gs);
    }
    CHECK_INT(spy->count, 140);
    bool ordered = true;
    for (int n = 0; n < 140; n++)
        if (spy->seen[n].value != (n / 20) * 100 + n % 20)
            ordered = false;
    CHECK(ordered);
    CHECK_INT(gs->events.dropped, 0);

    free(spy);
    free(gs);
}

/* Un abonné qui réémet sans fin ne fige pas le jeu. */
static void test_runaway_subscriber_is_cut(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);
    spy->forever = true;

    nh_event(gs, NH_EV_REPUTATION, 1);
    nh_events_flush(gs); /* doit se terminer */

    CHECK(spy->count > 0);
    CHECK(spy->count <= NH_EVENT_QUEUE * 8);
    CHECK_INT(nh_events_pending(gs), 0);
    CHECK(gs->events.dropped > 0);
    CHECK(!gs->events.flushing);

    /* et le bus fonctionne encore ensuite */
    gs->events.observer = NULL;
    nh_event(gs, NH_EV_ITEM_BOUGHT, 2);
    nh_events_flush(gs);
    CHECK_INT(nh_events_pending(gs), 0);

    free(spy);
    free(gs);
}

static void test_clear(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);

    nh_event(gs, NH_EV_LEVEL_UP, 2);
    nh_event(gs, NH_EV_LEVEL_UP, 3);
    nh_events_clear(gs);
    CHECK_INT(nh_events_pending(gs), 0);
    nh_events_flush(gs);
    CHECK_INT(spy->count, 0);
    CHECK_INT(nh_events_emitted(gs, NH_EV_LEVEL_UP), 2); /* clear oublie l'attente, pas l'historique */

    /* Après un clear, l'ordre reste bon. */
    nh_event(gs, NH_EV_REPUTATION, 5);
    nh_event(gs, NH_EV_REPUTATION, 6);
    nh_events_flush(gs);
    CHECK_INT(spy->count, 2);
    CHECK_INT(spy->seen[0].value, 5);
    CHECK_INT(spy->seen[1].value, 6);

    free(spy);
    free(gs);
}

/* ---- Le bus dans la boucle de jeu (nh_dispatch) ------------------------------------------------- */

static void test_dispatch_flushes_after_command(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);
    char out[8192];

    CHECK(run(gs, "status", out, sizeof out) == NH_DISPATCH_OK);
    CHECK_INT(nh_events_pending(gs), 0);
    CHECK_INT(spy->count, 1);
    CHECK_INT(spy->seen[0].type, NH_EV_COMMAND);
    CHECK_INT(spy->seen[0].value, NH_DISPATCH_OK);

    /* une commande qui échoue est un événement aussi : le temps a passé */
    CHECK(run(gs, "read", out, sizeof out) == NH_DISPATCH_FAILED);
    CHECK_INT(spy->seen[1].type, NH_EV_COMMAND);
    CHECK_INT(spy->seen[1].value, NH_DISPATCH_FAILED);

    /* ni une ligne vide, ni une commande inconnue ou verrouillée : rien ne s'est passé */
    int before = spy->count;
    CHECK(run(gs, "", out, sizeof out) == NH_DISPATCH_EMPTY);
    CHECK(run(gs, "xyzzy", out, sizeof out) == NH_DISPATCH_UNKNOWN);
    CHECK(run(gs, "bruteforce localhost", out, sizeof out) == NH_DISPATCH_LOCKED);
    CHECK_INT(spy->count, before);

    free(spy);
    free(gs);
}

/* Les événements d'une commande passent AVANT le bilan COMMAND, dans l'ordre où ils sont nés. */
static void test_dispatch_delivers_command_effects(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);
    char out[16384];

    run(gs, "scan", out, sizeof out);
    /* cinq scans = niveau 2 : la montée de niveau précède le bilan de la commande */
    for (int i = 0; i < 4; i++)
        run(gs, "scan", out, sizeof out);
    CHECK_INT(gs->player.level, LEVEL_APPRENTICE);

    int level_up_at = -1, last_command_at = -1;
    for (int i = 0; i < spy->count; i++)
    {
        if (spy->seen[i].type == NH_EV_LEVEL_UP && level_up_at < 0)
            level_up_at = i;
        if (spy->seen[i].type == NH_EV_COMMAND)
            last_command_at = i;
    }
    CHECK(level_up_at >= 0);
    CHECK(level_up_at < last_command_at);
    CHECK_INT(spy->seen[level_up_at].value, 2);

    free(spy);
    free(gs);
}

static void test_dispatch_clears_when_game_ends(void)
{
    char out[8192];

    /* quit : plus personne à prévenir */
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);
    nh_event(gs, NH_EV_REPUTATION, 1); /* un événement resté en attente */
    CHECK(run(gs, "quit", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(!gs->running);
    CHECK_INT(nh_events_pending(gs), 0);
    CHECK_INT(spy->count, 0);
    free(spy);
    free(gs);

    /* partie perdue : idem, aucune annonce de quête sur un écran de game over */
    gs = new_game();
    spy = spy_on(gs);
    gs->player.game_over = true;
    run(gs, "status", out, sizeof out);
    CHECK_INT(nh_events_pending(gs), 0);
    CHECK_INT(spy->count, 0);
    free(spy);
    free(gs);
}

/* ---- Les émetteurs ---------------------------------------------------------------------------- */

static int count_type(const Spy *s, NhEvent type)
{
    int n = 0;
    for (int i = 0; i < s->count && i < (int)(sizeof s->seen / sizeof s->seen[0]); i++)
        if (s->seen[i].type == type)
            n++;
    return n;
}

static void test_emitter_world(void)
{
    GameState *gs = new_game();
    char out[8192];
    nh_set_fast(true);
    nh_term_set_color(false);

    NhCapture cap = nh_capture_begin();
    CHECK(nh_world_compromise(gs, NH_NODE_LOCALHOST, false));
    nh_capture_end(&cap, out, sizeof out);
    NhEventRecord rec;
    bool found = false;
    for (int i = 0; nh_events_peek(gs, i, &rec); i++)
        if (rec.type == NH_EV_NODE_COMPROMISED)
        {
            found = true;
            CHECK_INT(rec.value, NH_NODE_LOCALHOST);
        }
    CHECK(found);
    CHECK_INT(nh_events_emitted(gs, NH_EV_NODE_COMPROMISED), 1);
    CHECK_INT(nh_events_emitted(gs, NH_EV_FILES_EXTRACTED), 0);

    /* déjà compromis : rien de plus, ni récompense ni événement */
    cap = nh_capture_begin();
    CHECK(!nh_world_compromise(gs, NH_NODE_LOCALHOST, false));
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(nh_events_emitted(gs, NH_EV_NODE_COMPROMISED), 1);

    /* en profondeur : les fichiers sont extraits et annoncés, avec leur nombre */
    cap = nh_capture_begin();
    CHECK(nh_world_compromise(gs, NH_NODE_CORP_SERVER, true));
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(nh_events_emitted(gs, NH_EV_NODE_COMPROMISED), 2);
    CHECK_INT(nh_events_emitted(gs, NH_EV_FILES_EXTRACTED), 1);
    int files = gs->nodes[NH_NODE_CORP_SERVER].file_count;
    found = false;
    for (int i = 0; nh_events_peek(gs, i, &rec); i++)
        if (rec.type == NH_EV_FILES_EXTRACTED)
        {
            found = true;
            CHECK_INT(rec.value, files);
        }
    CHECK(found);

    /* extraire ce qui est déjà extrait ne dit rien */
    cap = nh_capture_begin();
    CHECK_INT(nh_world_extract(gs, NH_NODE_CORP_SERVER), 0);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(nh_events_emitted(gs, NH_EV_FILES_EXTRACTED), 1);

    /* extraction seule sur un autre système */
    cap = nh_capture_begin();
    int n = nh_world_extract(gs, NH_NODE_NEXUS);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(n > 0);
    CHECK_INT(nh_events_emitted(gs, NH_EV_FILES_EXTRACTED), 2);

    free(gs);
}

static void test_emitter_shop(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);
    char out[16384];
    nh_set_fast(true);
    nh_term_set_color(false);
    gs->player.level = LEVEL_LEGEND;
    gs->player.credits = 100000;

    nh_feed("1\n"); /* le premier objet */
    CHECK(run(gs, "shop", out, sizeof out) == NH_DISPATCH_OK);
    CHECK_INT(count_type(spy, NH_EV_ITEM_BOUGHT), 1);
    int first = -1;
    for (int i = 0; i < spy->count; i++)
        if (spy->seen[i].type == NH_EV_ITEM_BOUGHT)
            first = spy->seen[i].value;
    CHECK_INT(first, ITEM_STEALTH_UPGRADE);
    CHECK(gs->shop.bought & (1u << ITEM_STEALTH_UPGRADE));

    /* quitter la boutique, un numéro invalide, ou un achat refusé : aucun événement d'achat */
    nh_feed("0\n");
    run(gs, "shop", out, sizeof out);
    nh_feed("99\n");
    run(gs, "shop", out, sizeof out);
    gs->player.credits = 0;
    nh_feed("5\n");
    run(gs, "shop", out, sizeof out);
    CHECK_INT(count_type(spy, NH_EV_ITEM_BOUGHT), 1);
    CHECK(!(gs->shop.bought & (1u << ITEM_ENCRYPTION_KEY)));

    free(spy);
    free(gs);
}

static void test_emitter_contact_and_reputation_booster(void)
{
    GameState *gs = new_game();
    Spy *spy = spy_on(gs);
    char out[16384];
    nh_set_fast(true);
    nh_term_set_color(false);

    nh_feed("0\n");
    CHECK(run(gs, "contact ECHO-7", out, sizeof out) == NH_DISPATCH_OK);
    CHECK_INT(count_type(spy, NH_EV_CONTACT_MET), 1);
    CHECK_INT(gs->contacts.contacts[CONTACT_ECHO7].interactions_count, 1);

    /* un contact inconnu ou pas débloqué : pas de conversation, pas d'événement */
    run(gs, "contact Personne", out, sizeof out);
    run(gs, "contact AURA", out, sizeof out);
    CHECK_INT(count_type(spy, NH_EV_CONTACT_MET), 1);

    /* le boost de réputation de la boutique a un effet immédiat, et le dit sur le bus */
    gs->player.level = LEVEL_LEGEND;
    gs->player.credits = 100000;
    int rep0 = gs->player.reputation;
    nh_feed("8\n");
    run(gs, "shop", out, sizeof out);
    CHECK_INT(gs->player.reputation, rep0 + 20);
    CHECK(count_type(spy, NH_EV_REPUTATION) >= 1);
    CHECK(strstr(out, "[+20 réputation]") != NULL);

    free(spy);
    free(gs);
}

/* ---- Abonné : déblocage des contacts ------------------------------------------------------------ */

static void test_contacts_unlock_on_events(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_set_lang(NH_LANG_FR);
    nh_term_set_color(false);

    CHECK(gs->contacts.contacts[CONTACT_ECHO7].is_unlocked);
    CHECK(!gs->contacts.contacts[CONTACT_R4Z0R].is_unlocked);
    int mails0 = gs->contacts.inbox_count;

    /* trop tôt : niveau atteint mais pas la réputation */
    gs->player.level = LEVEL_APPRENTICE;
    nh_event(gs, NH_EV_LEVEL_UP, 2);
    NhCapture cap = nh_capture_begin();
    nh_events_flush(gs);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(!gs->contacts.contacts[CONTACT_R4Z0R].is_unlocked);

    /* niveau ET réputation : R4Z0R apparaît, une seule fois, avec une annonce et son courrier */
    gs->player.reputation = 10;
    nh_event(gs, NH_EV_REPUTATION, 10);
    cap = nh_capture_begin();
    nh_events_flush(gs);
    nh_capture_end(&cap, out, sizeof out);
    CHECK(gs->contacts.contacts[CONTACT_R4Z0R].is_unlocked);
    CHECK_INT(gs->contacts.inbox_count, mails0 + 1);
    CHECK(strstr(out, "NOUVEAU CONTACT DÉBLOQUÉ : R4Z0R") != NULL);
    CHECK(strstr(out, "Nouveau message de R4Z0R") != NULL);

    nh_event(gs, NH_EV_REPUTATION, 10);
    cap = nh_capture_begin();
    nh_events_flush(gs);
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(gs->contacts.inbox_count, mails0 + 1);
    CHECK(strstr(out, "R4Z0R") == NULL); /* pas de seconde annonce */

    free(gs);
}

/* Sans fiche : jamais débloqué, quels que soient niveau, réputation et quêtes. Avec fiche : dès que tout est réuni. */
static void test_contacts_unwritten_stay_locked(void)
{
    GameState *gs = new_game();
    char out[16384];
    nh_term_set_color(false);
    gs->player.level = LEVEL_LEGEND;
    gs->player.reputation = 100000;
    for (int q = 0; q < QUEST_COUNT; q++)
        gs->quests.quests[q].status = QUEST_STATUS_COMPLETED;

    nh_event(gs, NH_EV_COMMAND, 0);
    NhCapture cap = nh_capture_begin();
    nh_events_flush(gs);
    nh_capture_end(&cap, out, sizeof out);

    for (int i = 0; i < CONTACT_COUNT; i++)
        CHECK(gs->contacts.contacts[i].is_unlocked == nh_contact_written((ContactType)i));
    CHECK(!gs->contacts.contacts[CONTACT_SHADOW_BROKER].is_unlocked);
    free(gs);
}

int main(void)
{
    nh_set_fast(true);
    nh_term_set_color(false);
    nh_set_lang(NH_LANG_FR);

    test_record_and_peek();
    test_flush_is_fifo_and_empties();
    test_emission_during_flush();
    test_queue_overflow_drops_newest();
    test_ring_wraps_around();
    test_runaway_subscriber_is_cut();
    test_clear();
    test_dispatch_flushes_after_command();
    test_dispatch_delivers_command_effects();
    test_dispatch_clears_when_game_ends();
    test_emitter_world();
    test_emitter_shop();
    test_emitter_contact_and_reputation_booster();
    test_contacts_unlock_on_events();
    test_contacts_unwritten_stay_locked();
    nh_unfeed();
    return NH_TEST_REPORT("events");
}
