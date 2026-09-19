#include "shop_view.h"

#include <stdbool.h>
#include <stdio.h>
#include <string.h>

#include "../core/platform.h"
#include "../core/utf8.h"
#include "../i18n/i18n.h"
#include "../ui/hud.h"
#include "../ui/term.h"

#define DEFAULT_COLS 80
#define MAX_WIDTH 100             /* au-delà, les deux colonnes s'éloignent trop l'une de l'autre */
#define TWO_COLUMNS_MIN_WIDTH 71  /* deux cases de 34 (« [10] » + le nom le plus long + son prix) + l'écart */
#define ROOMY_MIN_WIDTH 50        /* le bandeau fait 48 colonnes */
#define COLUMN_GAP 3
#define CELL_INDENT 5             /* la largeur de « [01] » : résumé et statut s'alignent sous le nom */
#define CELL_LINES 3              /* nom et prix, résumé, statut */
#define LIST_PRICE_WIDTH 6
#define TEXT_MAX 160

static const NhStr k_blurbs[ITEM_COUNT] = {
    [ITEM_STEALTH_UPGRADE] = NH_STR_SHOP_BLURB_STEALTH,
    [ITEM_ALERT_REDUCER] = NH_STR_SHOP_BLURB_ALERT,
    [ITEM_VIRUS_PACK] = NH_STR_SHOP_BLURB_VIRUS,
    [ITEM_PROXY_CHAIN] = NH_STR_SHOP_BLURB_PROXY,
    [ITEM_ENCRYPTION_KEY] = NH_STR_SHOP_BLURB_KEY,
    [ITEM_AI_MODULE] = NH_STR_SHOP_BLURB_AI,
    [ITEM_QUANTUM_CHIP] = NH_STR_SHOP_BLURB_QUANTUM,
    [ITEM_REPUTATION_BOOST] = NH_STR_SHOP_BLURB_REPUTATION,
    [ITEM_XP_BOOST] = NH_STR_SHOP_BLURB_XP,
    [ITEM_VPN_SERVICE] = NH_STR_SHOP_BLURB_VPN,
};

/* Le bandeau est du dessin, pas du texte : pas de traduction. */
static const char *const k_banner[2] = {
    "█▀▀ █▄█ █▄▄ █▀▀ █▀█   █▀▄▀█ ▄▀█ █▀█ █▄▀ █▀▀ ▀█▀",
    "█▄▄  █  █▄█ █▄▄ █▀▄   █ ▀ █ █▀█ █▀▄ █ █ █▄▄  █",
};

NhShopState nh_shop_state(const ShopItem *item, int credits, int level)
{
    if (!item->is_available)
        return NH_SHOP_SOLD_OUT;
    if (level < item->level_required)
        return NH_SHOP_LOW_LEVEL;
    if (credits < item->price)
        return NH_SHOP_LOW_CREDITS;
    return NH_SHOP_OK;
}

const char *nh_shop_blurb(ShopItemType type)
{
    if ((int)type < 0 || (int)type >= ITEM_COUNT)
        return "";
    return nh_tr(k_blurbs[type]);
}

/* ---- Écriture bornée ------------------------------------------------------------------------- */

typedef struct
{
    char *out;
    size_t size;
    size_t len;
} Sink;

static void add(Sink *s, const char *text)
{
    size_t room = s->size - 1 - s->len;
    size_t n = strlen(text);
    if (n > room)
        n = room;
    memcpy(s->out + s->len, text, n);
    s->len += n;
    s->out[s->len] = '\0';
}

static void add_spaces(Sink *s, int n)
{
    for (int i = 0; i < n; i++)
        add(s, " ");
}

static int width_of(const char *text)
{
    return (int)nh_display_width(text);
}

/* Un morceau de texte brut dans sa couleur ; `used` cumule les colonnes réellement affichées. */
static void piece(Sink *s, NhColor color, const char *text, int *used)
{
    add(s, nh_c(color));
    add(s, text);
    add(s, nh_c(NH_C_RESET));
    *used += width_of(text);
}

/* Copie `text` dans `buf`, coupé pour tenir dans `room` colonnes ; « … » signale ce qui manque. */
static void fit(char *buf, size_t size, const char *text, int room)
{
    snprintf(buf, size, "%s", text);
    if (room <= 0)
        buf[0] = '\0';
    else if (width_of(buf) > room)
    {
        nh_truncate_width(buf, (size_t)room - 1);
        size_t len = strlen(buf);
        if (len + sizeof "…" <= size)
            memcpy(buf + len, "…", sizeof "…");
    }
}

/* ---- États et couleurs ----------------------------------------------------------------------- */

static NhColor state_color(NhShopState state)
{
    switch (state)
    {
    case NH_SHOP_OK:
        return NH_C_GREEN;
    case NH_SHOP_LOW_CREDITS:
        return NH_C_YELLOW;
    case NH_SHOP_LOW_LEVEL:
    case NH_SHOP_SOLD_OUT:
        return NH_C_RED;
    }
    return NH_C_WHITE;
}

/* « unique · niveau 4 requis » : la couleur seule ne suffit pas (--no-color, daltonisme). */
static void status_text(char *buf, size_t size, const ShopItem *item, NhShopState state, int credits)
{
    char detail[TEXT_MAX] = "";
    switch (state)
    {
    case NH_SHOP_LOW_LEVEL:
        snprintf(detail, sizeof detail, nh_tr(NH_STR_SHOP_STATE_LEVEL), item->level_required);
        break;
    case NH_SHOP_LOW_CREDITS:
        snprintf(detail, sizeof detail, nh_tr(NH_STR_SHOP_STATE_CREDITS), item->price - credits);
        break;
    case NH_SHOP_SOLD_OUT:
        snprintf(detail, sizeof detail, "%s", nh_tr(NH_STR_SHOP_STATE_SOLD));
        break;
    case NH_SHOP_OK:
        snprintf(detail, sizeof detail, "%s", nh_tr(NH_STR_SHOP_STATE_OK));
        break;
    }
    const char *kind = nh_tr(item->is_consumable ? NH_STR_SHOP_KIND_CONSUMABLE : NH_STR_SHOP_KIND_UNIQUE);
    snprintf(buf, size, "%s · %s", kind, detail);
}

static void put_tag(Sink *s, int index, NhShopState state, int *used)
{
    char tag[TEXT_MAX];
    snprintf(tag, sizeof tag, "[%02d]", index + 1);
    piece(s, state == NH_SHOP_SOLD_OUT ? NH_C_RED : NH_C_BRIGHT_CYAN, tag, used);
}

/* ---- Choix de la mise en page ---------------------------------------------------------------- */

typedef struct
{
    bool cards;   /* cases de 3 lignes ; sinon une ligne par objet */
    bool roomy;   /* cartes aérées : bandeau, mot de bienvenue, ligne vide entre les rangées */
    bool stacked; /* le portefeuille passe sous le titre : ils ne tiennent pas côte à côte */
    int columns;  /* colonnes de cases : 1 ou 2 */
    int width;    /* colonnes utiles */
    int count;    /* objets à afficher */
} Plan;

static int rows_of(const Plan *plan)
{
    return (plan->count + plan->columns - 1) / plan->columns;
}

static int lines_needed(const Plan *plan)
{
    int header = plan->stacked ? 2 : 1;
    int body = plan->count;
    if (plan->cards)
    {
        int rows = rows_of(plan);
        header += 2; /* mot d'ambiance, filet */
        if (plan->roomy)
            header += 4; /* bandeau (2), ligne vide, mot de bienvenue */
        body = rows * CELL_LINES + (plan->roomy && rows > 0 ? rows - 1 : 0);
    }
    return 2 + header + body; /* + la ligne vide avant et celle après */
}

static bool fits(const Plan *plan, int max_lines)
{
    return max_lines <= 0 || lines_needed(plan) <= max_lines;
}

/* La plus confortable des trois mises en page qui tient ; à défaut, la liste (la plus courte). */
static Plan choose_plan(Plan base, int max_lines)
{
    Plan roomy = base;
    roomy.cards = true;
    roomy.roomy = base.width >= ROOMY_MIN_WIDTH;
    if (roomy.roomy && fits(&roomy, max_lines))
        return roomy;

    Plan tight = base;
    tight.cards = true;
    if (fits(&tight, max_lines))
        return tight;

    Plan list = base;
    list.columns = 1;
    return list;
}

/* ---- Composition ----------------------------------------------------------------------------- */

static void end_line(Sink *s)
{
    add(s, "\n");
}

static void put_header(Sink *s, const Plan *plan, const char *title, const char *wallet)
{
    char text[TEXT_MAX];
    int used = 0;

    if (plan->roomy)
    {
        for (int i = 0; i < 2; i++)
        {
            piece(s, NH_C_MAGENTA, k_banner[i], &used);
            end_line(s);
        }
        end_line(s);
    }

    used = 0;
    fit(text, sizeof text, title, plan->width);
    piece(s, NH_C_MAGENTA, text, &used);
    if (plan->stacked)
    {
        end_line(s);
        used = 0;
    }
    else
        add_spaces(s, plan->width - used - width_of(wallet));
    fit(text, sizeof text, wallet, plan->width);
    piece(s, NH_C_BRIGHT_CYAN, text, &used);
    end_line(s);

    if (!plan->cards)
        return;
    if (plan->roomy)
    {
        used = 0;
        fit(text, sizeof text, nh_tr(NH_STR_SHOP_WELCOME), plan->width);
        piece(s, NH_C_CYAN, text, &used);
        end_line(s);
    }
    used = 0;
    fit(text, sizeof text, nh_tr(NH_STR_SHOP_TAGLINE), plan->width);
    piece(s, NH_C_CYAN, text, &used);
    end_line(s);

    add(s, nh_c(NH_C_CYAN));
    for (int i = 0; i < plan->width; i++)
        add(s, "─");
    add(s, nh_c(NH_C_RESET));
    end_line(s);
}

/*
 * Une ligne d'une case de `width` colonnes :
 *   [01] Stealth Module v2.0        150 ¢
 *        +2 furtivité, définitivement
 *        unique · niveau 2 requis
 * `pad` : compléter par des espaces (une autre case suit sur la ligne).
 */
static void cell_line(Sink *s, const ShopItem *item, int index, NhShopState state, int credits,
                      int line, int width, bool pad)
{
    char text[TEXT_MAX];
    int used = 0;

    if (line == 0)
    {
        char price[32];
        char name[64];
        snprintf(price, sizeof price, "%d ¢", item->price);
        int price_w = width_of(price);

        put_tag(s, index, state, &used);
        fit(name, sizeof name, item->name, width - used - 1 - price_w - 1);
        snprintf(text, sizeof text, " %s", name);
        piece(s, NH_C_WHITE, text, &used);
        int gap = width - used - price_w;
        if (gap < 1)
            gap = 1;
        add_spaces(s, gap);
        used += gap;
        piece(s, state_color(state), price, &used);
    }
    else
    {
        add_spaces(s, CELL_INDENT);
        used = CELL_INDENT;
        if (line == 1)
        {
            fit(text, sizeof text, nh_shop_blurb((ShopItemType)index), width - CELL_INDENT);
            piece(s, NH_C_WHITE, text, &used);
        }
        else
        {
            char status[TEXT_MAX];
            status_text(status, sizeof status, item, state, credits);
            fit(text, sizeof text, status, width - CELL_INDENT);
            piece(s, state_color(state), text, &used);
        }
    }
    if (pad)
        add_spaces(s, width - used);
}

/* Les objets sont rangés par colonne (1 à 5 à gauche, 6 à 10 à droite) : on lit de haut en bas. */
static void put_cards(Sink *s, const Plan *plan, const CyberShop *shop, int credits, int level)
{
    int rows = rows_of(plan);
    int width = (plan->width - COLUMN_GAP * (plan->columns - 1)) / plan->columns;

    for (int r = 0; r < rows; r++)
    {
        for (int line = 0; line < CELL_LINES; line++)
        {
            for (int c = 0; c < plan->columns; c++)
            {
                int i = c * rows + r;
                if (i >= plan->count)
                    break;
                if (c > 0)
                    add_spaces(s, COLUMN_GAP);
                bool followed = c + 1 < plan->columns && (c + 1) * rows + r < plan->count;
                cell_line(s, &shop->items[i], i, nh_shop_state(&shop->items[i], credits, level),
                          credits, line, width, followed);
            }
            end_line(s);
        }
        if (plan->roomy && r + 1 < rows)
            end_line(s);
    }
}

/* Une ligne par objet : le plus court, pour les terminaux bas. Le résumé est sacrifié. */
static void put_list(Sink *s, const Plan *plan, const CyberShop *shop, int credits, int level)
{
    int name_w = 0;
    for (int i = 0; i < plan->count; i++)
        if (width_of(shop->items[i].name) > name_w)
            name_w = width_of(shop->items[i].name);
    /* Le nom cède la place au statut si l'écran est étroit, mais garde au moins de quoi se lire. */
    int fixed = CELL_INDENT + 1 + LIST_PRICE_WIDTH + 2;
    if (name_w > plan->width - fixed - 20)
        name_w = plan->width - fixed - 20;
    if (name_w < 8)
        name_w = 8;

    for (int i = 0; i < plan->count; i++)
    {
        const ShopItem *item = &shop->items[i];
        NhShopState state = nh_shop_state(item, credits, level);
        char text[TEXT_MAX];
        char cell[TEXT_MAX];
        int used = 0;

        put_tag(s, i, state, &used);
        add_spaces(s, 1);
        used += 1;

        fit(text, sizeof text, item->name, name_w);
        nh_pad(cell, sizeof cell, text, (size_t)name_w, NH_ALIGN_LEFT);
        piece(s, NH_C_WHITE, cell, &used);
        add_spaces(s, 1);
        used += 1;

        snprintf(text, sizeof text, "%d ¢", item->price);
        nh_pad(cell, sizeof cell, text, LIST_PRICE_WIDTH, NH_ALIGN_RIGHT);
        piece(s, state_color(state), cell, &used);
        add_spaces(s, 2);
        used += 2;

        status_text(text, sizeof text, item, state, credits);
        fit(cell, sizeof cell, text, plan->width - used);
        piece(s, state_color(state), cell, &used);
        end_line(s);
    }
}

size_t nh_shop_compose(char *out, size_t size, const CyberShop *shop, int credits, int level,
                       int cols, int max_lines)
{
    if (out == NULL || size == 0)
        return 0;
    out[0] = '\0';

    Sink sink = {out, size, 0};
    char title[TEXT_MAX];
    char wallet[TEXT_MAX];
    snprintf(title, sizeof title, nh_tr(NH_STR_SHOP_TITLE), shop->vendor_name);
    snprintf(wallet, sizeof wallet, nh_tr(NH_STR_SHOP_WALLET), credits, level);

    int width = (cols > 0 ? cols : DEFAULT_COLS) - 1; /* la dernière colonne reste libre : pas de retour automatique */
    if (width > MAX_WIDTH)
        width = MAX_WIDTH;

    int count = shop->item_count;
    if (count < 0)
        count = 0;
    if (count > ITEM_COUNT)
        count = ITEM_COUNT;

    Plan base = {.width = width,
                 .columns = width >= TWO_COLUMNS_MIN_WIDTH ? 2 : 1,
                 .count = count,
                 .stacked = width_of(title) + 2 + width_of(wallet) > width};
    Plan plan = choose_plan(base, max_lines);

    end_line(&sink);
    put_header(&sink, &plan, title, wallet);
    if (plan.cards)
        put_cards(&sink, &plan, shop, credits, level);
    else
        put_list(&sink, &plan, shop, credits, level);
    end_line(&sink);

    nh_utf8_trim_incomplete(out);
    return strlen(out);
}

void nh_shop_show(const CyberShop *shop, int credits, int level)
{
    int cols = 0;
    int rows = 0;
    int max_lines = 0;

    if (nh_stdout_is_tty() && nh_term_size(&cols, &rows))
    {
        /* Les deux barres du HUD ne défilent pas ; la dernière ligne est celle de la question. */
        max_lines = (nh_hud_active() ? rows - 2 : rows) - 1;
    }
    else
        cols = 0;

    char buffer[NH_SHOP_BUFFER];
    nh_shop_compose(buffer, sizeof buffer, shop, credits, level, cols, max_lines);
    fputs(buffer, stdout);
}
