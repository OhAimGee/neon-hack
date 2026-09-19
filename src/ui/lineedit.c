#include "lineedit.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../core/platform.h"
#include "term.h"

#ifndef _WIN32
#include <errno.h>
#include <poll.h>
#include <signal.h>
#include <termios.h>
#include <unistd.h>
#endif

#define DEFAULT_COLS 80
#define KEY_ESC 0x1B
#define ESCAPE_WAIT_MS 40 /* au-delà, un ESC est la touche seule et non le début d'une flèche */

/* ---- Historique ------------------------------------------------------------------------------ */

static char g_history[NH_LE_HISTORY][NH_LE_LINE_MAX]; /* de la plus ancienne à la plus récente */
static size_t g_history_count = 0;

void nh_le_history_add(const char *line)
{
    if (line == NULL || line[strspn(line, " \t")] == '\0' || strlen(line) >= NH_LE_LINE_MAX)
        return;
    if (g_history_count > 0 && strcmp(g_history[g_history_count - 1], line) == 0)
        return;
    if (g_history_count == NH_LE_HISTORY)
    {
        memmove(g_history[0], g_history[1], sizeof g_history[0] * (NH_LE_HISTORY - 1));
        g_history_count--;
    }
    snprintf(g_history[g_history_count++], NH_LE_LINE_MAX, "%s", line);
}

void nh_le_history_clear(void)
{
    g_history_count = 0;
}

size_t nh_le_history_count(void)
{
    return g_history_count;
}

/* ---- UTF-8 ----------------------------------------------------------------------------------- */

static bool is_continuation(char c)
{
    return ((unsigned char)c & 0xC0) == 0x80;
}

/* Début du caractère qui précède l'octet `i` (i > 0). */
static size_t prev_index(const char *b, size_t i)
{
    i--;
    while (i > 0 && is_continuation(b[i]))
        i--;
    return i;
}

/* Début du caractère qui suit celui qui commence en `i` (i < len). */
static size_t next_index(const char *b, size_t len, size_t i)
{
    i++;
    while (i < len && is_continuation(b[i]))
        i++;
    return i;
}

/* Largeur d'affichage de buf[from, to). */
static size_t span_width(const char *buf, size_t from, size_t to)
{
    char tmp[NH_LE_LINE_MAX];
    size_t n = to - from;
    if (n >= sizeof tmp)
        n = sizeof tmp - 1;
    memcpy(tmp, buf + from, n);
    tmp[n] = '\0';
    return nh_display_width(tmp);
}

/* ---- Sortie ---------------------------------------------------------------------------------- */

static void out_add_n(NhLineEdit *ed, const char *text, size_t n)
{
    size_t room = sizeof ed->out - 1 - ed->out_len;
    if (n > room)
        n = room;
    memcpy(ed->out + ed->out_len, text, n);
    ed->out_len += n;
    ed->out[ed->out_len] = '\0';
}

static void out_add(NhLineEdit *ed, const char *text)
{
    out_add_n(ed, text, strlen(text));
}

static void out_spaces(NhLineEdit *ed, size_t n)
{
    for (size_t i = 0; i < n; i++)
        out_add(ed, " ");
}

/* Avance le curseur de n colonnes depuis la colonne 0 (la séquence CSI « C » ne défile jamais). */
static void out_forward(NhLineEdit *ed, size_t n)
{
    if (n == 0)
        return;
    char seq[32];
    snprintf(seq, sizeof seq, "\033[%zuC", n);
    out_add(ed, seq);
}

static size_t screen_cols(const NhLineEdit *ed)
{
    return ed->cols > 0 ? (size_t)ed->cols : DEFAULT_COLS;
}

/*
 * Redessine le texte de la ligne (pas le prompt, déjà à l'écran : le réécrire à chaque touche
 * ferait apparaître le prompt des dizaines de fois dans la sortie) puis replace le curseur. La
 * fenêtre visible défile pour que le curseur reste dans les colonnes libres ; la dernière colonne
 * de l'écran n'est jamais écrite, donc jamais de retour à la ligne automatique.
 */
static void redraw(NhLineEdit *ed)
{
    size_t cols = screen_cols(ed);
    size_t prompt_w = nh_display_width(ed->prompt);
    size_t avail = cols > prompt_w + 1 ? cols - 1 - prompt_w : 1;

    size_t start = 0;
    while (start < ed->cursor && span_width(ed->buf, start, ed->cursor) > avail)
        start = next_index(ed->buf, ed->len, start);
    size_t end = start;
    while (end < ed->len)
    {
        size_t next = next_index(ed->buf, ed->len, end);
        if (span_width(ed->buf, start, next) > avail)
            break;
        end = next;
    }

    out_add(ed, "\r");
    out_forward(ed, prompt_w);
    out_add_n(ed, ed->buf + start, end - start);
    out_add(ed, "\033[K\r");
    out_forward(ed, prompt_w + span_width(ed->buf, start, ed->cursor));
}

static void bell(NhLineEdit *ed)
{
    out_add(ed, "\a");
}

/* ---- Édition --------------------------------------------------------------------------------- */

void nh_le_init(NhLineEdit *ed, const char *prompt, char *buf, size_t size, NhCompleteFn complete,
                void *ctx)
{
    memset(ed, 0, sizeof *ed);
    ed->prompt = prompt != NULL ? prompt : "";
    ed->buf = buf;
    ed->size = size > NH_LE_LINE_MAX ? NH_LE_LINE_MAX : size;
    ed->complete = complete;
    ed->ctx = ctx;
    ed->history_pos = -1;
    if (ed->size > 0)
        buf[0] = '\0';
}

void nh_le_start(NhLineEdit *ed)
{
    out_add(ed, ed->prompt);
    redraw(ed);
}

const char *nh_le_output(const NhLineEdit *ed)
{
    return ed->out;
}

void nh_le_output_clear(NhLineEdit *ed)
{
    ed->out_len = 0;
    ed->out[0] = '\0';
}

bool nh_le_escape_pending(const NhLineEdit *ed)
{
    return ed->esc == 1;
}

void nh_le_escape_timeout(NhLineEdit *ed)
{
    if (ed->esc == 1)
        ed->esc = 0;
}

static void insert_bytes(NhLineEdit *ed, const char *bytes, size_t n)
{
    if (ed->len + n >= ed->size) /* la place de la fin de chaîne compte */
    {
        bell(ed);
        return;
    }
    memmove(ed->buf + ed->cursor + n, ed->buf + ed->cursor, ed->len - ed->cursor + 1);
    memcpy(ed->buf + ed->cursor, bytes, n);
    ed->len += n;
    ed->cursor += n;
    redraw(ed);
}

/* Retire buf[from, to) ; le curseur suit. */
static void erase(NhLineEdit *ed, size_t from, size_t to)
{
    memmove(ed->buf + from, ed->buf + to, ed->len - to + 1);
    ed->len -= to - from;
    if (ed->cursor >= to)
        ed->cursor -= to - from;
    else if (ed->cursor > from)
        ed->cursor = from;
    redraw(ed);
}

static void move_to(NhLineEdit *ed, size_t position)
{
    ed->cursor = position;
    redraw(ed);
}

static void key_left(NhLineEdit *ed)
{
    if (ed->cursor > 0)
        move_to(ed, prev_index(ed->buf, ed->cursor));
}

static void key_right(NhLineEdit *ed)
{
    if (ed->cursor < ed->len)
        move_to(ed, next_index(ed->buf, ed->len, ed->cursor));
}

static void key_backspace(NhLineEdit *ed)
{
    if (ed->cursor > 0)
        erase(ed, prev_index(ed->buf, ed->cursor), ed->cursor);
}

static void key_delete(NhLineEdit *ed)
{
    if (ed->cursor < ed->len)
        erase(ed, ed->cursor, next_index(ed->buf, ed->len, ed->cursor));
}

/* Ctrl+W : le mot qui précède le curseur, avec les espaces entre lui et le curseur. */
static void key_kill_word(NhLineEdit *ed)
{
    size_t from = ed->cursor;
    while (from > 0 && ed->buf[from - 1] == ' ')
        from--;
    while (from > 0 && ed->buf[from - 1] != ' ')
        from--;
    if (from < ed->cursor)
        erase(ed, from, ed->cursor);
}

/* Remplace la ligne par `text` (tronqué à la taille du tampon), curseur à la fin. */
static void load_line(NhLineEdit *ed, const char *text)
{
    size_t n = strlen(text);
    if (n >= ed->size)
    {
        n = ed->size - 1;
        while (n > 0 && is_continuation(text[n]))
            n--;
    }
    memcpy(ed->buf, text, n);
    ed->buf[n] = '\0';
    ed->len = n;
    ed->cursor = n;
    redraw(ed);
}

static void history_older(NhLineEdit *ed)
{
    if (ed->history_pos + 1 >= (int)g_history_count)
    {
        bell(ed);
        return;
    }
    if (ed->history_pos < 0)
        snprintf(ed->draft, sizeof ed->draft, "%s", ed->buf);
    ed->history_pos++;
    load_line(ed, g_history[g_history_count - 1 - (size_t)ed->history_pos]);
}

static void history_newer(NhLineEdit *ed)
{
    if (ed->history_pos < 0)
        return;
    ed->history_pos--;
    load_line(ed, ed->history_pos < 0 ? ed->draft : g_history[g_history_count - 1 - (size_t)ed->history_pos]);
}

/* ---- Complétion ------------------------------------------------------------------------------ */

static char lower_ascii(char c)
{
    return (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : c;
}

static int compare_nocase(const char *a, const char *b)
{
    while (*a != '\0' && lower_ascii(*a) == lower_ascii(*b))
    {
        a++;
        b++;
    }
    return (unsigned char)lower_ascii(*a) - (unsigned char)lower_ascii(*b);
}

/* Tri par insertion (au plus 64 mots) sans doublons. */
static void sort_unique(NhCompletions *c)
{
    for (size_t i = 1; i < c->count; i++)
    {
        char tmp[NH_LE_CANDIDATE_LEN];
        memcpy(tmp, c->items[i], sizeof tmp);
        size_t j = i;
        while (j > 0 && compare_nocase(c->items[j - 1], tmp) > 0)
        {
            memcpy(c->items[j], c->items[j - 1], sizeof tmp);
            j--;
        }
        memcpy(c->items[j], tmp, sizeof tmp);
    }
    size_t kept = 0;
    for (size_t i = 0; i < c->count; i++)
        if (kept == 0 || strcmp(c->items[kept - 1], c->items[i]) != 0)
        {
            if (kept != i)
                memcpy(c->items[kept], c->items[i], sizeof c->items[0]);
            kept++;
        }
    c->count = kept;
}

/* Longueur du préfixe commun à tous les mots, sans tenir compte de la casse, sur une frontière de caractère. */
static size_t common_prefix(const NhCompletions *c)
{
    size_t n = strlen(c->items[0]);
    for (size_t k = 1; k < c->count; k++)
    {
        size_t i = 0;
        while (i < n && c->items[k][i] != '\0' && lower_ascii(c->items[0][i]) == lower_ascii(c->items[k][i]))
            i++;
        n = i;
    }
    while (n > 0 && is_continuation(c->items[0][n]))
        n--;
    return n;
}

/* Remplace buf[from, to) par `text` ; le curseur se place après. */
static void replace_range(NhLineEdit *ed, size_t from, size_t to, const char *text, size_t n)
{
    if (ed->len - (to - from) + n >= ed->size)
    {
        bell(ed);
        return;
    }
    memmove(ed->buf + from + n, ed->buf + to, ed->len - to + 1);
    memcpy(ed->buf + from, text, n);
    ed->len = ed->len - (to - from) + n;
    ed->cursor = from + n;
    redraw(ed);
}

/* Les candidats en colonnes, comme `ls` (on lit de haut en bas), puis la ligne réaffichée dessous. */
static void list_candidates(NhLineEdit *ed, const NhCompletions *c)
{
    size_t widest = 0;
    for (size_t k = 0; k < c->count; k++)
        if (nh_display_width(c->items[k]) > widest)
            widest = nh_display_width(c->items[k]);
    size_t column_w = widest + 2;
    size_t per_row = (screen_cols(ed) - 1) / column_w;
    if (per_row < 1)
        per_row = 1;
    size_t rows = (c->count + per_row - 1) / per_row;

    out_add(ed, "\r\n");
    for (size_t r = 0; r < rows; r++)
    {
        for (size_t col = 0; col < per_row; col++)
        {
            size_t k = col * rows + r;
            if (k >= c->count)
                break;
            out_add(ed, c->items[k]);
            if ((col + 1) * rows + r < c->count)
                out_spaces(ed, column_w - nh_display_width(c->items[k]));
        }
        out_add(ed, "\r\n");
    }
    out_add(ed, ed->prompt);
    redraw(ed);
}

/*
 * TAB à la manière de bash : un seul mot convient → on le complète ; plusieurs → on avance jusqu'à
 * leur préfixe commun ; si on ne peut plus avancer, le premier TAB sonne, le suivant liste.
 */
static void complete_word(NhLineEdit *ed, bool again)
{
    if (ed->complete == NULL)
    {
        bell(ed);
        return;
    }

    char before[NH_LE_LINE_MAX];
    memcpy(before, ed->buf, ed->cursor);
    before[ed->cursor] = '\0';

    NhCompletions c;
    memset(&c, 0, sizeof c);
    ed->complete(ed->ctx, before, &c);
    if (c.count > NH_LE_MAX_CANDIDATES)
        c.count = NH_LE_MAX_CANDIDATES;
    if (c.start > ed->cursor)
        c.start = ed->cursor;
    for (size_t k = 0; k < c.count; k++)
        c.items[k][NH_LE_CANDIDATE_LEN - 1] = '\0';
    if (c.count == 0)
    {
        bell(ed);
        return;
    }
    sort_unique(&c);

    size_t typed = ed->cursor - c.start;
    if (c.count == 1)
    {
        char word[NH_LE_CANDIDATE_LEN + 1];
        snprintf(word, sizeof word, "%s", c.items[0]);
        if (c.space && ed->buf[ed->cursor] != ' ')
            strcat(word, " ");
        replace_range(ed, c.start, ed->cursor, word, strlen(word));
        return;
    }

    size_t common = common_prefix(&c);
    if (common > typed)
        replace_range(ed, c.start, ed->cursor, c.items[0], common);
    else if (again)
        list_candidates(ed, &c);
    else
        bell(ed);
}

/* ---- Touches --------------------------------------------------------------------------------- */

/* Séquence « ESC [ param final » ou « ESC O final ». Les touches inconnues sont ignorées. */
static void key_sequence(NhLineEdit *ed, unsigned char final, int param)
{
    switch (final)
    {
    case 'A':
        history_older(ed);
        break;
    case 'B':
        history_newer(ed);
        break;
    case 'C':
        key_right(ed);
        break;
    case 'D':
        key_left(ed);
        break;
    case 'H':
        move_to(ed, 0);
        break;
    case 'F':
        move_to(ed, ed->len);
        break;
    case '~':
        if (param == 1 || param == 7)
            move_to(ed, 0);
        else if (param == 4 || param == 8)
            move_to(ed, ed->len);
        else if (param == 3)
            key_delete(ed);
        break;
    default:
        break;
    }
}

static void escape_byte(NhLineEdit *ed, unsigned char b)
{
    if (ed->esc == 1)
    {
        if (b == '[')
        {
            ed->esc = 2;
            ed->csi_param = 0;
            ed->csi_more = false;
        }
        else if (b == 'O')
            ed->esc = 3;
        else
            ed->esc = 0; /* Alt+touche : ignorée */
        return;
    }
    if (ed->esc == 3)
    {
        ed->esc = 0;
        key_sequence(ed, b, 0);
        return;
    }
    /* Dans « ESC [ » : paramètres numériques, séparateurs, puis l'octet final. */
    if (b >= '0' && b <= '9')
    {
        if (!ed->csi_more && ed->csi_param < 1000)
            ed->csi_param = ed->csi_param * 10 + (b - '0');
    }
    else if (b == ';')
        ed->csi_more = true;
    else if (b >= 0x20 && b <= 0x3F)
    {
        /* autre paramètre ou intermédiaire : sans effet sur nous */
    }
    else
    {
        ed->esc = 0;
        if (b >= 0x40 && b <= 0x7E)
            key_sequence(ed, b, ed->csi_param);
    }
}

static NhLeResult plain_byte(NhLineEdit *ed, unsigned char b, bool after_tab)
{
    switch (b)
    {
    case KEY_ESC:
        ed->esc = 1;
        return NH_LE_CONTINUE;
    case '\t':
        complete_word(ed, after_tab);
        ed->after_tab = true;
        return NH_LE_CONTINUE;
    case '\r':
    case '\n':
        out_add(ed, "\r\n");
        return NH_LE_DONE;
    case 0x04: /* Ctrl+D : fin de saisie sur une ligne vide, sinon Suppr */
        if (ed->len == 0)
            return NH_LE_EOF;
        key_delete(ed);
        return NH_LE_CONTINUE;
    case 0x01: /* Ctrl+A */
        move_to(ed, 0);
        return NH_LE_CONTINUE;
    case 0x05: /* Ctrl+E */
        move_to(ed, ed->len);
        return NH_LE_CONTINUE;
    case 0x02: /* Ctrl+B */
        key_left(ed);
        return NH_LE_CONTINUE;
    case 0x06: /* Ctrl+F */
        key_right(ed);
        return NH_LE_CONTINUE;
    case 0x10: /* Ctrl+P */
        history_older(ed);
        return NH_LE_CONTINUE;
    case 0x0E: /* Ctrl+N */
        history_newer(ed);
        return NH_LE_CONTINUE;
    case 0x08:
    case 0x7F:
        key_backspace(ed);
        return NH_LE_CONTINUE;
    case 0x0B: /* Ctrl+K : jusqu'à la fin */
        if (ed->cursor < ed->len)
            erase(ed, ed->cursor, ed->len);
        return NH_LE_CONTINUE;
    case 0x15: /* Ctrl+U : jusqu'au début */
        if (ed->cursor > 0)
            erase(ed, 0, ed->cursor);
        return NH_LE_CONTINUE;
    case 0x17: /* Ctrl+W */
        key_kill_word(ed);
        return NH_LE_CONTINUE;
    default:
        break;
    }

    if (b < 0x20 || b == 0x7F)
        return NH_LE_CONTINUE; /* autres commandes de contrôle : ignorées */
    if (b < 0x80)
    {
        char c = (char)b;
        insert_bytes(ed, &c, 1);
        return NH_LE_CONTINUE;
    }
    /* Début d'un caractère sur plusieurs octets : on attend la suite avant de l'insérer. */
    size_t need = b >= 0xF0 && b <= 0xF4 ? 4 : b >= 0xE0 && b < 0xF0 ? 3 : b >= 0xC2 && b < 0xE0 ? 2 : 0;
    if (need > 0)
    {
        ed->utf8[0] = b;
        ed->utf8_have = 1;
        ed->utf8_need = need;
    }
    return NH_LE_CONTINUE;
}

NhLeResult nh_le_feed(NhLineEdit *ed, unsigned char byte)
{
    bool after_tab = ed->after_tab;
    ed->after_tab = false;

    if (ed->esc != 0)
    {
        escape_byte(ed, byte);
        return NH_LE_CONTINUE;
    }
    if (ed->utf8_need > 0)
    {
        if ((byte & 0xC0) == 0x80)
        {
            ed->utf8[ed->utf8_have++] = byte;
            if (ed->utf8_have == ed->utf8_need)
            {
                insert_bytes(ed, (const char *)ed->utf8, ed->utf8_need);
                ed->utf8_have = 0;
                ed->utf8_need = 0;
            }
            return NH_LE_CONTINUE;
        }
        ed->utf8_have = 0; /* séquence interrompue : abandonnée, cet octet est traité normalement */
        ed->utf8_need = 0;
    }
    return plain_byte(ed, byte, after_tab);
}

/* ---- Terminal -------------------------------------------------------------------------------- */

#ifdef _WIN32

bool nh_lineedit_interactive(void)
{
    return false; /* la console Windows garde sa propre édition de ligne */
}

NhIoStatus nh_lineedit_read(const char *prompt, char *buf, size_t size, NhCompleteFn complete, void *ctx)
{
    (void)complete;
    (void)ctx;
    fputs(prompt, stdout);
    fflush(stdout);
    return nh_read_line(buf, size);
}

#else

bool nh_lineedit_interactive(void)
{
    const char *term = getenv("TERM");
    return nh_io_uses_stdin() && isatty(STDIN_FILENO) && nh_stdout_is_tty() &&
           (term == NULL || strcmp(term, "dumb") != 0);
}

/* -- Mode brut : rendu au terminal quoi qu'il arrive, y compris sur Ctrl+C ou kill -- */

static struct termios g_saved;
static volatile sig_atomic_t g_raw = 0;
static struct sigaction g_old_action[3];
static const int k_signals[3] = {SIGINT, SIGTERM, SIGHUP};

static void restore_terminal(void)
{
    if (g_raw)
    {
        g_raw = 0;
        tcsetattr(STDIN_FILENO, TCSANOW, &g_saved);
    }
}

/* Rend d'abord le terminal, puis laisse le signal suivre son cours (le HUD remet sa région, puis le processus s'arrête). */
static void on_signal(int sig)
{
    for (size_t i = 0; i < 3; i++)
        if (k_signals[i] == sig)
        {
            if (g_old_action[i].sa_handler == SIG_IGN)
                return;
            restore_terminal();
            sigaction(sig, &g_old_action[i], NULL);
            raise(sig);
            return;
        }
}

static bool raw_enter(void)
{
    if (tcgetattr(STDIN_FILENO, &g_saved) != 0)
        return false;
    struct termios raw = g_saved;
    raw.c_lflag &= ~(tcflag_t)(ICANON | ECHO | IEXTEN); /* ISIG reste : Ctrl+C et Ctrl+Z gardent leur effet */
    raw.c_cc[VMIN] = 1;
    raw.c_cc[VTIME] = 0;
    if (tcsetattr(STDIN_FILENO, TCSADRAIN, &raw) != 0)
        return false;
    g_raw = 1;

    struct sigaction action;
    memset(&action, 0, sizeof action);
    action.sa_handler = on_signal;
    sigemptyset(&action.sa_mask);
    for (size_t i = 0; i < 3; i++)
        sigaction(k_signals[i], &action, &g_old_action[i]);
    return true;
}

static void raw_leave(void)
{
    for (size_t i = 0; i < 3; i++)
        sigaction(k_signals[i], &g_old_action[i], NULL);
    restore_terminal();
}

/* Un octet du clavier ; -1 : fin d'entrée ou erreur ; -2 : rien reçu dans le délai (timeout_ms >= 0). */
static int read_byte(int timeout_ms)
{
    if (timeout_ms >= 0)
    {
        struct pollfd p = {.fd = STDIN_FILENO, .events = POLLIN, .revents = 0};
        int ready = poll(&p, 1, timeout_ms);
        if (ready == 0 || (ready < 0 && errno == EINTR))
            return -2;
        if (ready < 0)
            return -1;
    }
    for (;;)
    {
        unsigned char c;
        ssize_t n = read(STDIN_FILENO, &c, 1);
        if (n == 1)
            return c;
        if (n == 0 || errno != EINTR)
            return -1;
    }
}

static void write_all(const char *data, size_t left)
{
    while (left > 0)
    {
        ssize_t n = write(STDOUT_FILENO, data, left);
        if (n < 0)
        {
            if (errno == EINTR)
                continue;
            break;
        }
        data += n;
        left -= (size_t)n;
    }
}

static void flush_output(NhLineEdit *ed)
{
    write_all(ed->out, ed->out_len);
    nh_le_output_clear(ed);
}

/*
 * Le prompt doit commencer en colonne 0 : le cœur redessine la ligne avec « \r » et des déplacements
 * absolus. Or certaines sorties du jeu se terminent au milieu d'une ligne (« [ALERTE +1] » est suivi
 * d'une espace, pas d'un saut de ligne). Astuce de zsh : écrire (largeur - 1) espaces puis « \r ».
 * En colonne 0, le curseur revient où il était, rien ne change à l'écran ; au milieu d'une ligne, les
 * espaces débordent, le terminal passe à la ligne suivante, et « \r » nous y met en colonne 0.
 */
static void start_on_fresh_line(int cols)
{
    char blanks[64];
    memset(blanks, ' ', sizeof blanks);
    for (int left = cols - 1; left > 0;)
    {
        size_t n = left > (int)sizeof blanks ? sizeof blanks : (size_t)left;
        write_all(blanks, n);
        left -= (int)n;
    }
    write_all("\r", 1);
}

NhIoStatus nh_lineedit_read(const char *prompt, char *buf, size_t size, NhCompleteFn complete, void *ctx)
{
    if (size == 0)
        return NH_IO_ERROR;

    fflush(stdout); /* tout ce que le jeu a déjà écrit passe avant notre prompt */
    if (!nh_lineedit_interactive() || !raw_enter())
    {
        fputs(prompt, stdout);
        fflush(stdout);
        return nh_read_line(buf, size);
    }

    NhLineEdit *ed = malloc(sizeof *ed);
    if (ed == NULL)
    {
        raw_leave();
        fputs(prompt, stdout);
        fflush(stdout);
        return nh_read_line(buf, size);
    }
    nh_le_init(ed, prompt, buf, size, complete, ctx);
    int cols = 0;
    int rows = 0;
    ed->cols = nh_term_size(&cols, &rows) ? cols : 0;
    if (ed->cols > 1)
        start_on_fresh_line(ed->cols);
    nh_le_start(ed);
    flush_output(ed);

    NhLeResult result = NH_LE_CONTINUE;
    NhIoStatus status = NH_IO_OK;
    while (result == NH_LE_CONTINUE)
    {
        int b = read_byte(nh_le_escape_pending(ed) ? ESCAPE_WAIT_MS : -1);
        if (b == -2)
        {
            nh_le_escape_timeout(ed);
            continue;
        }
        if (b < 0)
        {
            status = NH_IO_EOF;
            break;
        }
        ed->cols = nh_term_size(&cols, &rows) ? cols : 0; /* la fenêtre a pu être redimensionnée */
        result = nh_le_feed(ed, (unsigned char)b);
        flush_output(ed);
    }
    raw_leave();

    if (status == NH_IO_OK && result == NH_LE_EOF)
        status = NH_IO_EOF;
    if (status == NH_IO_OK)
        nh_le_history_add(buf);
    else
        buf[0] = '\0';
    free(ed);
    return status;
}

#endif
