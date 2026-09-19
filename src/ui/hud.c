#include "hud.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../core/platform.h"
#include "../i18n/i18n.h"
#include "term.h"

#ifdef _WIN32
#include <io.h>
#define nh_write_fd(fd, buf, n) _write((fd), (buf), (unsigned)(n))
#else
#include <unistd.h>
#define nh_write_fd(fd, buf, n) write((fd), (buf), (n))
#endif

#define BAR_MAX 512

bool nh_hud_should_enable(bool disabled, bool is_tty, int cols, int rows)
{
    return !disabled && is_tty && cols >= NH_HUD_MIN_COLS && rows >= NH_HUD_MIN_ROWS;
}

/* ---- Composition -------------------------------------------------------- */

static size_t append(char *out, size_t size, size_t used, const char *text)
{
    size_t n = strlen(text);
    if (used + n + 1 > size)
        n = size > used + 1 ? size - used - 1 : 0;
    memcpy(out + used, text, n);
    out[used + n] = '\0';
    return used + n;
}

/* Complète `out` avec des espaces jusqu'à `cols` colonnes. */
static size_t pad_to(char *out, size_t size, size_t used, int cols)
{
    size_t width = nh_display_width(out);
    while (width < (size_t)cols && used + 1 < size)
    {
        out[used++] = ' ';
        width++;
    }
    out[used] = '\0';
    return used;
}

size_t nh_hud_compose_top(char *out, size_t size, const NhHudData *d, int cols)
{
    if (size == 0)
        return 0;
    out[0] = '\0';
    if (cols < 1)
        return 0;
    size_t total = (size_t)cols;

    /* Partie droite : « Alerte ████░░░░░░ 42 », qui rétrécit avant que la gauche ne soit coupée. */
    int alert = d->alert < 0 ? 0 : (d->alert > 100 ? 100 : d->alert);
    char right_plain[160];
    char right[224];
    char bar[48]; /* 10 cases de 3 octets au plus */
    right[0] = '\0';

    /* Partie gauche : nom, niveau, crédits, furtivité. */
    char left[192];
    char name[64];
    snprintf(name, sizeof name, "%s", d->name != NULL ? d->name : "");
    snprintf(left, sizeof left, " %s | %s %d | %d ¢", name, nh_tr(NH_STR_HUD_LEVEL), d->level,
             d->credits);
    if (d->stealth)
    {
        size_t l = strlen(left);
        snprintf(left + l, sizeof left - l, " | %s", nh_tr(NH_STR_HUD_STEALTH));
    }

    int bar_width = 10;
    for (;;)
    {
        if (bar_width > 0)
        {
            nh_gauge(bar, sizeof bar, alert, 100, bar_width);
            snprintf(right_plain, sizeof right_plain, "%s %s %d ", nh_tr(NH_STR_HUD_ALERT), bar,
                     alert);
        }
        else
        {
            snprintf(right_plain, sizeof right_plain, "%s %d ", nh_tr(NH_STR_HUD_ALERT), alert);
        }
        if (nh_display_width(left) + 1 + nh_display_width(right_plain) <= total)
            break;
        if (bar_width > 4)
            bar_width -= 3;
        else if (bar_width > 0)
            bar_width = 0;
        else
            break; /* même sans jauge ça ne tient pas : on coupera la gauche */
    }

    size_t right_width = nh_display_width(right_plain);
    if (nh_display_width(left) + 1 + right_width > total)
    {
        size_t room = total > right_width + 1 ? total - right_width - 1 : 0;
        nh_truncate_width(left, room);
    }

    /* La jauge prend la couleur de la bande d'alerte ; le reste de la barre garde son style. */
    snprintf(right, sizeof right, "%s%s%s", nh_c(d->alert_color), right_plain, nh_c(NH_C_CYAN));

    size_t used = append(out, size, 0, left);
    size_t gap = total > nh_display_width(left) + right_width ? total - nh_display_width(left) - right_width : 0;
    for (size_t i = 0; i < gap && used + 1 < size; i++)
        out[used++] = ' ';
    out[used] = '\0';
    used = append(out, size, used, right);
    return used;
}

size_t nh_hud_compose_bottom(char *out, size_t size, const NhHudData *d, int cols)
{
    if (size == 0)
        return 0;
    out[0] = '\0';
    if (cols < 1)
        return 0;

    size_t used = append(out, size, 0, " ");
    used = append(out, size, used, nh_tr(NH_STR_HUD_COMMANDS));
    used = append(out, size, used, " ");

    bool dropped = false;
    for (size_t i = 0; i < d->command_count; i++)
    {
        char piece[80];
        snprintf(piece, sizeof piece, "%s%s", i > 0 ? " · " : "", d->commands[i]);
        /* On garde de la place pour l'ellipse, sauf pour la dernière commande. */
        size_t reserve = (i + 1 < d->command_count) ? 4 : 1;
        if (nh_display_width(out) + nh_display_width(piece) + reserve > (size_t)cols)
        {
            dropped = true;
            break;
        }
        used = append(out, size, used, piece);
    }
    if (dropped)
        used = append(out, size, used, " …");

    /* Sécurité : jamais plus large que l'écran. */
    if (nh_display_width(out) > (size_t)cols)
    {
        nh_truncate_width(out, (size_t)cols);
        used = strlen(out);
    }
    return pad_to(out, size, used, cols);
}

/* ---- Affichage ---------------------------------------------------------- */

static bool g_active = false;
static bool g_atexit_done = false;
static int g_cols = 0;
static int g_rows = 0;

bool nh_hud_active(void) { return g_active; }

/* Style des barres : vidéo inverse (un attribut, pas une couleur : reste valable avec NO_COLOR). */
static const char *bar_style(void)
{
    return nh_term_color_enabled() ? "\033[36;7m" : "\033[7m";
}

static void set_region(int rows)
{
    printf("\033[2;%dr", rows - 1);
}

static void on_terminate(int sig)
{
    /* Rend le terminal utilisable : sans cela il resterait figé dans la région de défilement. */
    static const char reset[] = "\033[r\033[0m\033[999;1H\n";
    if (nh_write_fd(1, reset, sizeof reset - 1) < 0)
    {
        /* rien de plus à faire dans un gestionnaire de signal */
    }
    signal(sig, SIG_DFL);
    raise(sig);
}

bool nh_hud_start(bool disabled)
{
    int cols = 0;
    int rows = 0;
    const char *term = getenv("TERM");
    bool have_size = nh_term_size(&cols, &rows);
    bool dumb = term != NULL && strcmp(term, "dumb") == 0;

    if (g_active || dumb || !have_size ||
        !nh_hud_should_enable(disabled, nh_stdout_is_tty(), cols, rows))
        return false;

    g_cols = cols;
    g_rows = rows;
    g_active = true;

    if (!g_atexit_done)
    {
        atexit(nh_hud_stop);
        signal(SIGINT, on_terminate);
        signal(SIGTERM, on_terminate);
        g_atexit_done = true;
    }

    /* On repousse le contenu déjà affiché dans l'historique du terminal plutôt que de l'effacer. */
    for (int i = 0; i < rows; i++)
        putchar('\n');
    set_region(rows);
    printf("\033[%d;1H", rows - 1);
    fflush(stdout);
    return true;
}

void nh_hud_refresh(const NhHudData *data)
{
    if (!g_active)
        return;

    int cols = 0;
    int rows = 0;
    if (!nh_term_size(&cols, &rows) || cols < NH_HUD_MIN_COLS || rows < NH_HUD_MIN_ROWS)
    {
        nh_hud_stop(); /* fenêtre devenue trop petite : retour à l'affichage classique */
        return;
    }
    if (cols != g_cols || rows != g_rows)
    {
        g_cols = cols;
        g_rows = rows;
        set_region(rows);
    }

    char top[BAR_MAX];
    char bottom[BAR_MAX];
    nh_hud_compose_top(top, sizeof top, data, cols);
    nh_hud_compose_bottom(bottom, sizeof bottom, data, cols);

    /* Enregistre le curseur, dessine les deux barres sans retour à la ligne automatique, restaure. */
    printf("\0337\033[?7l");
    printf("\033[1;1H%s%s\033[0m", bar_style(), top);
    printf("\033[%d;1H%s%s\033[0m", rows, bar_style(), bottom);
    printf("\033[?7h\0338");
    fflush(stdout);
}

void nh_hud_stop(void)
{
    if (!g_active)
        return;
    g_active = false;
    /* Région rétablie, barres effacées, curseur en bas : le shell reprend proprement. */
    printf("\033[r\033[1;1H\033[2K\033[%d;1H\033[2K\033[0m\n", g_rows);
    fflush(stdout);
}

void nh_hud_clear_body(void)
{
    if (!g_active)
        return;
    for (int row = 2; row < g_rows; row++)
        printf("\033[%d;1H\033[2K", row);
    printf("\033[2;1H");
}
