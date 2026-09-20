#include "complete.h"

#include <stdio.h>
#include <string.h>

#include "../core/parse.h"
#include "commands.h"
#include "contacts.h"

#define NAME_MAX_LEN 32

static bool is_blank(char c)
{
    return c == ' ' || c == '\t';
}

static size_t skip_blanks(const char *text, size_t i)
{
    while (is_blank(text[i]))
        i++;
    return i;
}

static void add(NhCompletions *out, const char *word)
{
    if (out->count >= NH_LE_MAX_CANDIDATES)
        return;
    snprintf(out->items[out->count], NH_LE_CANDIDATE_LEN, "%s", word);
    out->count++;
}

static void complete_command(const GameState *gs, const char *prefix, NhCompletions *out)
{
    size_t count;
    const NhCommand *table = nh_commands(&count);

    for (size_t i = 0; i < count; i++)
        if (!table[i].hidden && nh_command_available(gs, &table[i]) &&
            nh_str_has_prefix_nocase(table[i].name, prefix))
            add(out, table[i].name);
    if (out->count > 0)
        return;

    for (size_t i = 0; i < count; i++)
        if (!table[i].hidden && table[i].alias != NULL && nh_command_available(gs, &table[i]) &&
            nh_str_has_prefix_nocase(table[i].alias, prefix))
            add(out, table[i].alias);
}

static void complete_argument(const GameState *gs, NhArgKind kind, const char *prefix, NhCompletions *out)
{
    switch (kind)
    {
    case NH_ARG_SYSTEM:
        for (int i = 0; i < NH_MAX_NODES; i++)
            if (gs->nodes[i].is_discovered && nh_str_has_prefix_nocase(gs->nodes[i].name, prefix))
                add(out, gs->nodes[i].name);
        break;
    case NH_ARG_CONTACT:
        for (int i = 0; i < CONTACT_COUNT; i++)
            if (gs->contacts.contacts[i].is_unlocked &&
                nh_str_has_prefix_nocase(nh_contact_name((ContactType)i), prefix))
                add(out, nh_contact_name((ContactType)i));
        break;
    case NH_ARG_MESSAGE:
        for (int n = 1; n <= gs->contacts.inbox_count; n++)
        {
            char number[16];
            snprintf(number, sizeof number, "%d", n);
            if (nh_str_has_prefix_nocase(number, prefix))
                add(out, number);
        }
        break;
    case NH_ARG_NONE:
        break;
    }
}

void nh_complete_line(void *ctx, const char *before, NhCompletions *out)
{
    const GameState *gs = ctx;
    out->start = 0;
    out->count = 0;
    out->space = false;

    size_t first = skip_blanks(before, 0);
    size_t first_end = first;
    while (before[first_end] != '\0' && !is_blank(before[first_end]))
        first_end++;

    if (before[first_end] == '\0') /* on tape encore le nom de la commande */
    {
        out->start = first;
        out->space = true;
        complete_command(gs, before + first, out);
        return;
    }

    char name[NAME_MAX_LEN];
    snprintf(name, sizeof name, "%.*s", (int)(first_end - first), before + first);
    const NhCommand *cmd = nh_find_command(name);
    if (cmd == NULL || !nh_command_available(gs, cmd))
        return;

    /* L'argument est TOUT ce qui suit la commande : un contact peut avoir une espace dans son nom. */
    out->start = skip_blanks(before, first_end);
    complete_argument(gs, cmd->arg, before + out->start, out);
}
