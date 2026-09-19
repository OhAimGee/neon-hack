#include "save.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../core/kv.h"
#include "../core/parse.h"
#include "../core/storage.h"
#include "progression.h"
#include "tutorial.h"

/* Bornes des valeurs lues : au-delà, le fichier est considéré comme corrompu. */
#define MAX_CREDITS 1000000000L
#define MAX_COUNTER 1000000L

/* ---- Clés indexées ("node.3.flags") ------------------------------------------------------- */

static const char *key(char *buf, size_t size, const char *format, ...)
{
    va_list args;
    va_start(args, format);
    vsnprintf(buf, size, format, args);
    va_end(args);
    return buf;
}

/* Bits d'un tableau de booléens (`count` <= 30) ; le masque de l'autre sens est bit(). */
static long mask_of(const bool *flags, int count)
{
    long mask = 0;
    for (int i = 0; i < count; i++)
        if (flags[i])
            mask |= 1L << i;
    return mask;
}

static long full_mask(int count) { return (1L << count) - 1; }

/* Indicateurs d'un système, dans l'ordre du format (ne pas réordonner sans changer la version). */
enum
{
    NODE_DISCOVERED = 1,
    NODE_COMPROMISED = 2,
    NODE_BACKDOOR = 4,
    NODE_VIRUS = 8,
    NODE_TRACED = 16,
    NODE_INTEL = 32,
    NODE_ALL = 63
};

static long node_flags(const NetworkNode *n)
{
    return (n->is_discovered ? NODE_DISCOVERED : 0) | (n->is_compromised ? NODE_COMPROMISED : 0) |
           (n->has_backdoor ? NODE_BACKDOOR : 0) | (n->has_virus ? NODE_VIRUS : 0) |
           (n->is_traced ? NODE_TRACED : 0) | (n->has_intel ? NODE_INTEL : 0);
}

/* ---- Écriture ------------------------------------------------------------------------------ */

char *nh_save_to_text(const GameState *gs)
{
    NhKvWriter w;
    nh_kvw_init(&w);
    char k[64];

    nh_kvw_int(&w, "neon-hack-save", NH_SAVE_VERSION);

    const Player *p = &gs->player;
    nh_kvw_str(&w, "player.name", p->name);
    nh_kvw_int(&w, "player.level", (long)p->level);
    nh_kvw_int(&w, "player.experience", p->experience);
    nh_kvw_int(&w, "player.scans_done", p->scans_done);
    nh_kvw_int(&w, "player.milestones", (long)p->milestones);
    nh_kvw_int(&w, "player.credits", p->credits);
    nh_kvw_int(&w, "player.reputation", p->reputation);
    nh_kvw_int(&w, "player.stealth_rating", p->stealth_rating);
    nh_kvw_int(&w, "player.unlocked", mask_of(p->commands_unlocked, MAX_COMMANDS));
    nh_kvw_int(&w, "player.quantum", p->has_quantum_computer);
    nh_kvw_int(&w, "player.ai", p->has_ai_assistant);
    nh_kvw_int(&w, "player.virus_library", p->virus_library_size);
    nh_kvw_int(&w, "player.backdoors", p->backdoors_active);

    nh_kvw_int(&w, "game.stealth_mode", gs->stealth_mode);

    long detected = 0;
    for (int i = 0; i < gs->virus_count && i < 30; i++)
        if (gs->viruses[i].is_detected)
            detected |= 1L << i;
    nh_kvw_int(&w, "virus.detected", detected);

    for (int i = 0; i < NH_MAX_NODES; i++)
    {
        const NetworkNode *n = &gs->nodes[i];
        long files = 0;
        for (int f = 0; f < n->file_count && f < 30; f++)
            if (n->secret_files[f].is_unlocked)
                files |= 1L << f;
        nh_kvw_int(&w, key(k, sizeof k, "node.%d.flags", i), node_flags(n));
        nh_kvw_int(&w, key(k, sizeof k, "node.%d.firewall", i), n->firewall_strength);
        nh_kvw_int(&w, key(k, sizeof k, "node.%d.files", i), files);
    }

    long sold_out = 0;
    for (int i = 0; i < ITEM_COUNT; i++)
        if (!gs->shop.items[i].is_available)
            sold_out |= 1L << i;
    nh_kvw_int(&w, "shop.sold_out", sold_out);
    nh_kvw_int(&w, "shop.bought", (long)gs->shop.bought);

    nh_kvw_int(&w, "alert.level", gs->alert.level);
    nh_kvw_int(&w, "alert.max", gs->alert.max_level);
    nh_kvw_int(&w, "alert.vpn", gs->alert.vpn_active);
    nh_kvw_int(&w, "alert.proxy", gs->alert.proxy_active);
    nh_kvw_int(&w, "alert.ghost", gs->alert.ghost_protocols_available);
    nh_kvw_int(&w, "alert.reductions", gs->alert.reductions_done);

    nh_kvw_int(&w, "advanced.neural_sync", gs->advanced.neural_interface_sync);

    long read = 0;
    for (int i = 0; i < gs->contacts.inbox_count && i < 30; i++)
        if (gs->contacts.inbox[i].is_read)
            read |= 1L << i;
    nh_kvw_int(&w, "inbox.read", read);
    nh_kvw_int(&w, "contacts.active", gs->contacts.active_contacts);
    for (int i = 0; i < CONTACT_COUNT; i++)
    {
        const Contact *c = &gs->contacts.contacts[i];
        nh_kvw_int(&w, key(k, sizeof k, "contact.%d.flags", i), (c->is_unlocked ? 1 : 0) | (c->is_discovered ? 2 : 0));
        nh_kvw_int(&w, key(k, sizeof k, "contact.%d.interactions", i), c->interactions_count);
    }

    /* Les compteurs (actives, terminées, progression) se déduisent des statuts : on ne les écrit plus. */
    for (int i = 0; i < QUEST_COUNT; i++)
    {
        const QuestState *q = &gs->quests.quests[i];
        const NhQuestDef *def = nh_quest_def((QuestType)i);
        nh_kvw_int(&w, key(k, sizeof k, "quest.%d.status", i), (long)q->status);
        for (int o = 0; o < def->objective_count; o++)
            nh_kvw_int(&w, key(k, sizeof k, "quest.%d.obj.%d", i, o), q->progress[o]);
    }

    nh_kvw_int(&w, "tutorial.step", gs->tutorial.step);
    nh_kvw_int(&w, "tutorial.done", gs->tutorial.done);

    nh_kvw_int(&w, "end", 1); /* prouve que le fichier n'a pas été tronqué */

    char *text = NULL;
    if (!w.failed)
    {
        size_t n = strlen(nh_kvw_text(&w)) + 1;
        text = malloc(n);
        if (text != NULL)
            memcpy(text, nh_kvw_text(&w), n);
    }
    nh_kvw_free(&w);
    return text;
}

NhSaveStatus nh_save_game(const GameState *gs)
{
    if (gs->save_path[0] == '\0')
        return NH_SAVE_NO_PATH;

    char *text = nh_save_to_text(gs);
    if (text == NULL)
        return NH_SAVE_IO;
    bool ok = nh_storage_write_atomic(gs->save_path, text, strlen(text));
    free(text);
    return ok ? NH_SAVE_OK : NH_SAVE_IO;
}

/* ---- Lecture ------------------------------------------------------------------------------- */

typedef struct
{
    const NhKv *kv;
    bool bad; /* une valeur présente est illisible ou hors bornes */
} Reader;

/* Valeur de `name` dans [min, max] ; `fallback` (la valeur d'un état neuf) si la clé est absente. */
static long rd(Reader *r, const char *name, long min, long max, long fallback)
{
    long value;
    switch (nh_kv_int(r->kv, name, min, max, &value))
    {
    case NH_KV_OK:
        return value;
    case NH_KV_MISSING:
        return fallback;
    case NH_KV_INVALID:
        break;
    }
    r->bad = true;
    return fallback;
}

/* Vérifie l'en-tête et la marque de fin. */
static NhSaveStatus check_frame(const NhKv *kv)
{
    long version = 0;
    switch (nh_kv_int(kv, "neon-hack-save", 1, 1000000, &version))
    {
    case NH_KV_MISSING:
    case NH_KV_INVALID:
        return NH_SAVE_CORRUPT;
    case NH_KV_OK:
        break;
    }
    if (version > NH_SAVE_VERSION)
        return NH_SAVE_TOO_NEW;

    long end = 0;
    if (nh_kv_int(kv, "end", 1, 1, &end) != NH_KV_OK)
        return NH_SAVE_CORRUPT;
    return NH_SAVE_OK;
}

static void apply(GameState *gs, Reader *r)
{
    char k[64];
    Player *p = &gs->player;

    const char *name = nh_kv_get(r->kv, "player.name");
    if (name != NULL)
    {
        char clean[MAX_NAME_LENGTH];
        if (nh_clean_name(name, clean, sizeof clean, NH_NAME_MAX_CHARS) > 0)
            snprintf(p->name, sizeof p->name, "%s", clean);
    }
    p->level = (HackerLevel)rd(r, "player.level", 1, NH_LEVEL_MAX, (long)p->level);
    p->experience = (int)rd(r, "player.experience", 0, NH_XP_CAP, p->experience);
    p->scans_done = (int)rd(r, "player.scans_done", 0, MAX_COUNTER, p->scans_done);
    p->milestones = (unsigned)rd(r, "player.milestones", 0, full_mask(NH_MS_COUNT), (long)p->milestones);
    p->credits = (int)rd(r, "player.credits", 0, MAX_CREDITS, p->credits);
    p->reputation = (int)rd(r, "player.reputation", -NH_REPUTATION_CAP, NH_REPUTATION_CAP, p->reputation);
    p->stealth_rating = (int)rd(r, "player.stealth_rating", 0, 100, p->stealth_rating);
    long unlocked = rd(r, "player.unlocked", 0, full_mask(MAX_COMMANDS), mask_of(p->commands_unlocked, MAX_COMMANDS));
    for (int i = 0; i < MAX_COMMANDS; i++)
        p->commands_unlocked[i] = (unlocked >> i) & 1;
    p->has_quantum_computer = rd(r, "player.quantum", 0, 1, p->has_quantum_computer) != 0;
    p->has_ai_assistant = rd(r, "player.ai", 0, 1, p->has_ai_assistant) != 0;
    p->virus_library_size = (int)rd(r, "player.virus_library", 0, gs->virus_count, p->virus_library_size);
    p->backdoors_active = (int)rd(r, "player.backdoors", 0, MAX_COUNTER, p->backdoors_active);

    gs->stealth_mode = rd(r, "game.stealth_mode", 0, 1, gs->stealth_mode) != 0;

    long detected = rd(r, "virus.detected", 0, full_mask(gs->virus_count), 0);
    for (int i = 0; i < gs->virus_count; i++)
        gs->viruses[i].is_detected = (detected >> i) & 1;

    for (int i = 0; i < NH_MAX_NODES; i++)
    {
        NetworkNode *n = &gs->nodes[i];
        long flags = rd(r, key(k, sizeof k, "node.%d.flags", i), 0, NODE_ALL, node_flags(n));
        n->is_discovered = (flags & NODE_DISCOVERED) != 0;
        n->is_compromised = (flags & NODE_COMPROMISED) != 0;
        n->has_backdoor = (flags & NODE_BACKDOOR) != 0;
        n->has_virus = (flags & NODE_VIRUS) != 0;
        n->is_traced = (flags & NODE_TRACED) != 0;
        n->has_intel = (flags & NODE_INTEL) != 0;
        n->firewall_strength = (int)rd(r, key(k, sizeof k, "node.%d.firewall", i), 0, 100, n->firewall_strength);
        long files = rd(r, key(k, sizeof k, "node.%d.files", i), 0, full_mask(n->file_count), 0);
        for (int f = 0; f < n->file_count; f++)
            n->secret_files[f].is_unlocked = (files >> f) & 1;
    }

    long sold_out = rd(r, "shop.sold_out", 0, full_mask(ITEM_COUNT), 0);
    for (int i = 0; i < ITEM_COUNT; i++)
        gs->shop.items[i].is_available = !((sold_out >> i) & 1);
    /* Un objet non consommable épuisé a forcément été acheté, même dans une sauvegarde qui n'a pas encore ce masque. */
    gs->shop.bought = (unsigned)rd(r, "shop.bought", 0, full_mask(ITEM_COUNT), 0) | (unsigned)sold_out;

    gs->alert.level = (int)rd(r, "alert.level", 0, NH_ALERT_MAX, gs->alert.level);
    gs->alert.max_level = (int)rd(r, "alert.max", 0, NH_ALERT_MAX, gs->alert.max_level);
    gs->alert.vpn_active = rd(r, "alert.vpn", 0, 1, gs->alert.vpn_active) != 0;
    gs->alert.proxy_active = rd(r, "alert.proxy", 0, 1, gs->alert.proxy_active) != 0;
    gs->alert.ghost_protocols_available = (int)rd(r, "alert.ghost", 0, MAX_COUNTER, gs->alert.ghost_protocols_available);
    gs->alert.reductions_done = (int)rd(r, "alert.reductions", 0, MAX_COUNTER, gs->alert.reductions_done);

    gs->advanced.neural_interface_sync = (int)rd(r, "advanced.neural_sync", 0, 100, gs->advanced.neural_interface_sync);

    long read = rd(r, "inbox.read", 0, full_mask(gs->contacts.inbox_count), 0);
    for (int i = 0; i < gs->contacts.inbox_count; i++)
        gs->contacts.inbox[i].is_read = (read >> i) & 1;
    gs->contacts.active_contacts = (int)rd(r, "contacts.active", 0, CONTACT_COUNT, gs->contacts.active_contacts);
    for (int i = 0; i < CONTACT_COUNT; i++)
    {
        Contact *c = &gs->contacts.contacts[i];
        long flags = rd(r, key(k, sizeof k, "contact.%d.flags", i), 0, 3, (c->is_unlocked ? 1 : 0) | (c->is_discovered ? 2 : 0));
        c->is_unlocked = (flags & 1) != 0;
        c->is_discovered = (flags & 2) != 0;
        c->interactions_count = (int)rd(r, key(k, sizeof k, "contact.%d.interactions", i), 0, MAX_COUNTER, c->interactions_count);
    }

    /* Les clés « quests.* » et « quest.N.done » des versions précédentes sont ignorées : tout se déduit
     * des statuts et de l'avancement. Une quête terminée est complète, quoi qu'en disent ses compteurs. */
    for (int i = 0; i < QUEST_COUNT; i++)
    {
        QuestState *q = &gs->quests.quests[i];
        const NhQuestDef *def = nh_quest_def((QuestType)i);
        q->status = (QuestStatus)rd(r, key(k, sizeof k, "quest.%d.status", i), QUEST_STATUS_LOCKED, QUEST_STATUS_FAILED, (long)q->status);
        for (int o = 0; o < def->objective_count; o++)
        {
            int value = (int)rd(r, key(k, sizeof k, "quest.%d.obj.%d", i, o), 0, MAX_COUNTER, q->progress[o]);
            q->progress[o] = value < def->objectives[o].target ? value : def->objectives[o].target;
            if (q->status == QUEST_STATUS_COMPLETED)
                q->progress[o] = def->objectives[o].target;
        }
    }

    gs->tutorial.step = (int)rd(r, "tutorial.step", 0, NH_TUT_STEP_COUNT - 1, gs->tutorial.step);
    gs->tutorial.done = rd(r, "tutorial.done", 0, 1, gs->tutorial.done) != 0;
    if (gs->tutorial.done)
        gs->tutorial.step = NH_TUT_NONE; /* un tutoriel terminé n'a plus d'étape */
}

NhSaveStatus nh_save_from_text(GameState *gs, const char *text, size_t len)
{
    NhKv kv;
    if (!nh_kv_parse(&kv, text, len))
        return NH_SAVE_CORRUPT;

    NhSaveStatus status = check_frame(&kv);
    if (status != NH_SAVE_OK)
    {
        nh_kv_free(&kv);
        return status;
    }

    /* Les 170 Ko d'un GameState n'ont rien à faire sur la pile ; et on ne touche à `gs` qu'à la fin. */
    GameState *fresh = calloc(1, sizeof *fresh);
    if (fresh == NULL)
    {
        nh_kv_free(&kv);
        return NH_SAVE_IO;
    }
    init_game(fresh);

    Reader r = {.kv = &kv, .bad = false};
    apply(fresh, &r);
    nh_kv_free(&kv);

    if (r.bad)
    {
        free(fresh);
        return NH_SAVE_CORRUPT;
    }

    memcpy(fresh->save_path, gs->save_path, sizeof fresh->save_path);
    *gs = *fresh;
    free(fresh);
    return NH_SAVE_OK;
}

NhSaveStatus nh_load_game(GameState *gs, const char *path)
{
    char *text = NULL;
    size_t len = 0;
    switch (nh_storage_read(path, NH_SAVE_MAX_BYTES, &text, &len))
    {
    case NH_STORAGE_OK:
        break;
    case NH_STORAGE_MISSING:
        return NH_SAVE_MISSING;
    case NH_STORAGE_TOO_BIG:
        return NH_SAVE_CORRUPT;
    case NH_STORAGE_ERROR:
        return NH_SAVE_IO;
    }
    NhSaveStatus status = nh_save_from_text(gs, text, len);
    free(text);
    return status;
}

NhSaveStatus nh_save_peek_text(const char *text, size_t len, NhSaveInfo *info)
{
    NhKv kv;
    if (!nh_kv_parse(&kv, text, len))
        return NH_SAVE_CORRUPT;

    NhSaveStatus status = check_frame(&kv);
    if (status == NH_SAVE_OK)
    {
        Reader r = {.kv = &kv, .bad = false};
        memset(info, 0, sizeof *info);
        snprintf(info->name, sizeof info->name, "%s", NH_DEFAULT_NAME);

        const char *name = nh_kv_get(&kv, "player.name");
        if (name != NULL)
        {
            char clean[MAX_NAME_LENGTH];
            if (nh_clean_name(name, clean, sizeof clean, NH_NAME_MAX_CHARS) > 0)
                snprintf(info->name, sizeof info->name, "%s", clean);
        }
        info->level = (int)rd(&r, "player.level", 1, NH_LEVEL_MAX, 1);
        info->credits = (int)rd(&r, "player.credits", 0, MAX_CREDITS, 0);
        info->tutorial_active = rd(&r, "tutorial.step", 0, NH_TUT_STEP_COUNT - 1, 0) != 0;
        if (r.bad)
            status = NH_SAVE_CORRUPT;
    }
    nh_kv_free(&kv);
    return status;
}

NhSaveStatus nh_save_peek(const char *path, NhSaveInfo *info)
{
    char *text = NULL;
    size_t len = 0;
    switch (nh_storage_read(path, NH_SAVE_MAX_BYTES, &text, &len))
    {
    case NH_STORAGE_OK:
        break;
    case NH_STORAGE_MISSING:
        return NH_SAVE_MISSING;
    case NH_STORAGE_TOO_BIG:
        return NH_SAVE_CORRUPT;
    case NH_STORAGE_ERROR:
        return NH_SAVE_IO;
    }
    NhSaveStatus status = nh_save_peek_text(text, len, info);
    free(text);
    return status;
}
