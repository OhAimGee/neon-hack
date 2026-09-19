#include "events.h"

#include "game.h"
#include "quest_system.h"

/* Un abonné qui réémettrait sans fin ne doit pas figer le jeu : au-delà, on jette le reste. */
#define MAX_DELIVERIES (NH_EVENT_QUEUE * 8)

void nh_event(GameState *gs, NhEvent event, int value)
{
    if ((int)event < 0 || event >= NH_EV_COUNT)
        return;

    EventBus *bus = &gs->events;
    bus->emitted[event]++;
    if (bus->count >= NH_EVENT_QUEUE)
    {
        bus->dropped++;
        return;
    }
    NhEventRecord *slot = &bus->queue[(bus->head + bus->count) % NH_EVENT_QUEUE];
    slot->type = event;
    slot->value = value;
    bus->count++;
}

/* ---- Abonnés --------------------------------------------------------------------------------- */

/*
 * Les contacts se débloquent quand le niveau et la réputation de la fiche sont atteints. Les
 * contacts encore à écrire (fiche vide) et ceux hors ligne (Phoenix, AURA : rien ne les rend
 * joignables avant la phase 3.3) restent verrouillés plutôt que d'apparaître inutilisables.
 */
static void unlock_contacts(GameState *gs)
{
    for (int i = 0; i < CONTACT_COUNT; i++)
    {
        const Contact *c = &gs->contacts.contacts[i];
        if (c->is_unlocked || c->name[0] == '\0' || c->availability == CONTACT_OFFLINE)
            continue;
        (void)unlock_contact(&gs->contacts, (ContactType)i, &gs->player);
    }
}

static void deliver(GameState *gs, const NhEventRecord *event)
{
    if (gs->events.observer != NULL)
        gs->events.observer(gs->events.observer_ctx, event);

    switch (event->type)
    {
    case NH_EV_COMMAND:
    case NH_EV_LEVEL_UP:
    case NH_EV_REPUTATION:
    case NH_EV_QUEST_COMPLETED:
        unlock_contacts(gs);
        break;
    case NH_EV_NODE_COMPROMISED:
    case NH_EV_FILES_EXTRACTED:
    case NH_EV_ITEM_BOUGHT:
    case NH_EV_CONTACT_MET:
    case NH_EV_MILESTONE:
    case NH_EV_COUNT:
        break;
    }

    nh_quests_on_event(gs, event->type, event->value);
}

void nh_events_flush(GameState *gs)
{
    EventBus *bus = &gs->events;
    if (bus->flushing)
        return;

    bus->flushing = true;
    int budget = MAX_DELIVERIES;
    while (bus->count > 0 && budget-- > 0)
    {
        NhEventRecord event = bus->queue[bus->head];
        bus->head = (bus->head + 1) % NH_EVENT_QUEUE;
        bus->count--;
        deliver(gs, &event);
    }
    if (bus->count > 0)
    {
        bus->dropped += bus->count;
        bus->count = 0;
        bus->head = 0;
    }
    bus->flushing = false;
}

void nh_events_clear(GameState *gs)
{
    gs->events.head = 0;
    gs->events.count = 0;
}

int nh_events_pending(const GameState *gs) { return gs->events.count; }

bool nh_events_peek(const GameState *gs, int index, NhEventRecord *out)
{
    const EventBus *bus = &gs->events;
    if (index < 0 || index >= bus->count)
        return false;
    *out = bus->queue[(bus->head + index) % NH_EVENT_QUEUE];
    return true;
}

unsigned nh_events_emitted(const GameState *gs, NhEvent event)
{
    if ((int)event < 0 || event >= NH_EV_COUNT)
        return 0;
    return gs->events.emitted[event];
}
