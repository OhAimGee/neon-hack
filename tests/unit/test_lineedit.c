#include "nh_test.h"

#include "../../src/ui/lineedit.h"
#include "../../src/ui/term.h"

#include <stdlib.h>
#include <string.h>

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static int occurrences(const char *text, const char *needle)
{
    int n = 0;
    for (const char *p = text; (p = strstr(p, needle)) != NULL; p += strlen(needle))
        n++;
    return n;
}

/* ---- Bancs d'essai ---------------------------------------------------------------------------- */

#define PROMPT "> "

typedef struct
{
    NhLineEdit ed;
    char buf[100];
} Bench;

static void start(Bench *b, NhCompleteFn fn, void *ctx, int cols)
{
    nh_le_init(&b->ed, PROMPT, b->buf, sizeof b->buf, fn, ctx);
    b->ed.cols = cols;
    nh_le_start(&b->ed);
    nh_le_output_clear(&b->ed);
}

/* Envoie chaque octet de `keys` ; retourne le résultat du dernier. */
static NhLeResult keys(Bench *b, const char *text)
{
    NhLeResult r = NH_LE_CONTINUE;
    for (const char *p = text; *p != '\0' && r == NH_LE_CONTINUE; p++)
        r = nh_le_feed(&b->ed, (unsigned char)*p);
    return r;
}

#define LEFT "\033[D"
#define RIGHT "\033[C"
#define UP "\033[A"
#define DOWN "\033[B"
#define HOME "\033[H"
#define END "\033[F"
#define DELETE_KEY "\033[3~"

static void check_line(const Bench *b, const char *expected, size_t cursor)
{
    CHECK_STR(b->buf, expected);
    CHECK_INT(b->ed.len, strlen(expected));
    CHECK_INT(b->ed.cursor, cursor);
}

/* Un fournisseur de complétion de test : les mots de la liste, pour le dernier mot tapé. */
typedef struct
{
    const char *const *words;
    bool space;
} Words;

static void words_complete(void *ctx, const char *before, NhCompletions *out)
{
    const Words *w = ctx;
    const char *last = strrchr(before, ' ');
    out->start = last != NULL ? (size_t)(last - before) + 1 : 0;
    out->space = w->space;
    const char *typed = before + out->start;
    for (size_t i = 0; w->words[i] != NULL; i++)
    {
        size_t n = strlen(typed);
        bool match = strlen(w->words[i]) >= n;
        for (size_t k = 0; match && k < n; k++)
        {
            char a = typed[k], b = w->words[i][k];
            match = (a >= 'A' && a <= 'Z' ? a - 'A' + 'a' : a) == (b >= 'A' && b <= 'Z' ? b - 'A' + 'a' : b);
        }
        if (match && out->count < NH_LE_MAX_CANDIDATES)
            snprintf(out->items[out->count++], NH_LE_CANDIDATE_LEN, "%s", w->words[i]);
    }
}

/* ---- Saisie et affichage ---------------------------------------------------------------------- */

static void test_typing_and_enter(void)
{
    Bench b;
    nh_le_init(&b.ed, PROMPT, b.buf, sizeof b.buf, NULL, NULL);
    nh_le_start(&b.ed);
    CHECK(has(nh_le_output(&b.ed), PROMPT));
    nh_le_output_clear(&b.ed);

    CHECK_INT(keys(&b, "scan"), NH_LE_CONTINUE);
    check_line(&b, "scan", 4);
    CHECK(has(nh_le_output(&b.ed), "scan"));
    nh_le_output_clear(&b.ed);
    CHECK_INT(nh_le_feed(&b.ed, '\r'), NH_LE_DONE);
    CHECK_STR(b.buf, "scan");
    CHECK_STR(nh_le_output(&b.ed), "\r\n");

    start(&b, NULL, NULL, 80);
    CHECK_INT(keys(&b, "help\n"), NH_LE_DONE); /* Entrée peut arriver comme LF ou comme CR */
    CHECK_STR(b.buf, "help");
    start(&b, NULL, NULL, 80);
    CHECK_INT(nh_le_feed(&b.ed, '\r'), NH_LE_DONE);
    CHECK_STR(b.buf, ""); /* ligne vide : Entrée rend "" */
}

static void test_prompt_written_once(void)
{
    /* Les tests d'interface comptent les prompts à l'écran : les frappes ne doivent pas le réécrire. */
    Bench b;
    nh_le_init(&b.ed, PROMPT, b.buf, sizeof b.buf, NULL, NULL);
    nh_le_start(&b.ed);
    keys(&b, "advhack nexus" LEFT LEFT "x" DELETE_KEY HOME END "\x7f");
    CHECK_INT(occurrences(nh_le_output(&b.ed), PROMPT), 1);
}

static void test_editing_keys(void)
{
    Bench b;
    start(&b, NULL, NULL, 80);

    keys(&b, "abcd");
    keys(&b, LEFT LEFT);
    check_line(&b, "abcd", 2);
    keys(&b, "\x7f"); /* Retour arrière */
    check_line(&b, "acd", 1);
    keys(&b, DELETE_KEY);
    check_line(&b, "ad", 1);
    keys(&b, "XY");
    check_line(&b, "aXYd", 3);
    keys(&b, HOME);
    check_line(&b, "aXYd", 0);
    keys(&b, "\x7f" LEFT); /* rien avant le début */
    check_line(&b, "aXYd", 0);
    keys(&b, END);
    check_line(&b, "aXYd", 4);
    keys(&b, DELETE_KEY RIGHT); /* rien après la fin */
    check_line(&b, "aXYd", 4);
    keys(&b, "\x01");
    CHECK_INT(b.ed.cursor, 0); /* Ctrl+A */
    keys(&b, "\x05");
    CHECK_INT(b.ed.cursor, 4); /* Ctrl+E */
    keys(&b, "\x02\x02");
    CHECK_INT(b.ed.cursor, 2); /* Ctrl+B */
    keys(&b, "\x06");
    CHECK_INT(b.ed.cursor, 3); /* Ctrl+F */
    keys(&b, "\x08"); /* Ctrl+H = retour arrière */
    check_line(&b, "aXd", 2);

    /* Variantes de séquences envoyées selon les terminaux. */
    start(&b, NULL, NULL, 80);
    keys(&b, "abc\033OD\033OD");
    CHECK_INT(b.ed.cursor, 1);
    keys(&b, "\033OC");
    CHECK_INT(b.ed.cursor, 2);
    keys(&b, "\033[1~");
    CHECK_INT(b.ed.cursor, 0);
    keys(&b, "\033[4~");
    CHECK_INT(b.ed.cursor, 3);
    keys(&b, "\033[7~");
    CHECK_INT(b.ed.cursor, 0);
    keys(&b, "\033[8~");
    CHECK_INT(b.ed.cursor, 3);
    keys(&b, "\033OH");
    CHECK_INT(b.ed.cursor, 0);
    keys(&b, "\033OF");
    CHECK_INT(b.ed.cursor, 3);
}

static void test_kill_keys(void)
{
    Bench b;
    start(&b, NULL, NULL, 80);

    keys(&b, "hello world");
    keys(&b, "\x17"); /* Ctrl+W : le dernier mot */
    check_line(&b, "hello ", 6);
    keys(&b, "\x17");
    check_line(&b, "", 0);
    keys(&b, "\x17"); /* rien à effacer */
    check_line(&b, "", 0);

    keys(&b, "abc def ghi" LEFT LEFT LEFT LEFT);
    check_line(&b, "abc def ghi", 7);
    keys(&b, "\x0b"); /* Ctrl+K : jusqu'à la fin */
    check_line(&b, "abc def", 7);
    keys(&b, LEFT LEFT "\x15"); /* Ctrl+U : jusqu'au début */
    check_line(&b, "ef", 0);
    keys(&b, "\x15"); /* déjà au début : rien */
    check_line(&b, "ef", 0);
    keys(&b, END "\x0b"); /* déjà à la fin : rien */
    check_line(&b, "ef", 2);

    start(&b, NULL, NULL, 80);
    keys(&b, "ab  cd  " "\x17");
    check_line(&b, "ab  ", 4); /* les espaces de queue partent avec le mot */
}

static void test_eof_and_delete(void)
{
    Bench b;
    start(&b, NULL, NULL, 80);
    CHECK_INT(nh_le_feed(&b.ed, 0x04), NH_LE_EOF); /* Ctrl+D sur une ligne vide */

    start(&b, NULL, NULL, 80);
    keys(&b, "abc" LEFT LEFT);
    CHECK_INT(nh_le_feed(&b.ed, 0x04), NH_LE_CONTINUE); /* sinon : Suppr */
    check_line(&b, "ac", 1);
}

static void test_utf8(void)
{
    Bench b;
    start(&b, NULL, NULL, 80);

    keys(&b, "a\xC3\xA9" "b"); /* « aéb » */
    check_line(&b, "a\xC3\xA9" "b", 4);
    keys(&b, LEFT LEFT);
    CHECK_INT(b.ed.cursor, 1); /* un é = un déplacement, pas deux */
    keys(&b, DELETE_KEY);
    check_line(&b, "ab", 1);

    start(&b, NULL, NULL, 80);
    keys(&b, "\xC3\xA9\xC3\xA9");
    keys(&b, "\x7f");
    check_line(&b, "\xC3\xA9", 2); /* le retour arrière retire tout le caractère */

    /* Emoji (4 octets) et séquence coupée : l'octet de tête seul ne s'insère pas, il est abandonné. */
    start(&b, NULL, NULL, 80);
    keys(&b, "\xF0\x9F\xA7\xA0");
    check_line(&b, "\xF0\x9F\xA7\xA0", 4);
    start(&b, NULL, NULL, 80);
    keys(&b, "\xC3" "a");
    check_line(&b, "a", 1);
    start(&b, NULL, NULL, 80);
    keys(&b, "\xFF\xC0z"); /* octets qui ne peuvent pas commencer un caractère UTF-8 */
    check_line(&b, "z", 1);
}

static void test_capacity(void)
{
    char buf[8];
    NhLineEdit ed;
    nh_le_init(&ed, PROMPT, buf, sizeof buf, NULL, NULL);
    ed.cols = 80;
    for (const char *p = "0123456789"; *p != '\0'; p++)
        nh_le_feed(&ed, (unsigned char)*p);
    CHECK_STR(buf, "0123456"); /* 7 caractères + la fin de chaîne */
    CHECK(has(nh_le_output(&ed), "\a"));

    /* Un caractère de 2 octets qui ne rentre pas n'est pas coupé en deux. */
    nh_le_init(&ed, PROMPT, buf, sizeof buf, NULL, NULL);
    for (const char *p = "012345"; *p != '\0'; p++)
        nh_le_feed(&ed, (unsigned char)*p);
    nh_le_feed(&ed, 0xC3);
    nh_le_feed(&ed, 0xA9);
    CHECK_STR(buf, "012345");
    CHECK_INT(nh_le_feed(&ed, '\n'), NH_LE_DONE);

    /* Tampon minimal : rien ne s'insère, rien ne déborde. */
    char tiny[1];
    nh_le_init(&ed, PROMPT, tiny, sizeof tiny, NULL, NULL);
    nh_le_feed(&ed, 'a');
    CHECK_STR(tiny, "");
    CHECK_INT(ed.len, 0);

    /* Un tampon plus grand que le maximum du cœur est ramené au maximum. */
    char big[NH_LE_LINE_MAX * 2];
    nh_le_init(&ed, PROMPT, big, sizeof big, NULL, NULL);
    CHECK_INT(ed.size, NH_LE_LINE_MAX);
}

static void test_escape_handling(void)
{
    Bench b;
    start(&b, NULL, NULL, 80);

    keys(&b, "ab\033[Z" "c"); /* touche inconnue : ignorée */
    check_line(&b, "abc", 3);
    keys(&b, "\033[1;5D"); /* Ctrl+Flèche : traitée comme une flèche */
    CHECK_INT(b.ed.cursor, 2);
    keys(&b, "\033x" "d"); /* Alt+x : ignorée */
    check_line(&b, "abdc", 3);
    keys(&b, "\001\002\003"); /* Ctrl+C ne parvient normalement pas jusqu'ici ; en tout cas rien n'est inséré */
    check_line(&b, "abdc", 0);

    /* ESC seul : en attente de sa suite, puis abandonné à l'expiration du délai. */
    start(&b, NULL, NULL, 80);
    keys(&b, "a\033");
    CHECK(nh_le_escape_pending(&b.ed));
    nh_le_escape_timeout(&b.ed);
    CHECK(!nh_le_escape_pending(&b.ed));
    keys(&b, "b");
    check_line(&b, "ab", 2);
    nh_le_escape_timeout(&b.ed); /* sans effet hors attente */
    CHECK(!nh_le_escape_pending(&b.ed));
}

/* ---- Historique -------------------------------------------------------------------------------- */

static void test_history(void)
{
    Bench b;
    nh_le_history_clear();
    nh_le_history_add("scan");
    nh_le_history_add("help");
    nh_le_history_add("");     /* ignorée */
    nh_le_history_add("   ");  /* ignorée */
    nh_le_history_add("help"); /* doublon consécutif ignoré */
    CHECK_INT(nh_le_history_count(), 2);

    start(&b, NULL, NULL, 80);
    keys(&b, "xy");
    keys(&b, UP);
    check_line(&b, "help", 4);
    keys(&b, UP);
    check_line(&b, "scan", 4);
    nh_le_output_clear(&b.ed);
    keys(&b, UP);
    check_line(&b, "scan", 4); /* plus rien de plus ancien */
    CHECK(has(nh_le_output(&b.ed), "\a"));
    keys(&b, DOWN);
    check_line(&b, "help", 4);
    keys(&b, DOWN);
    check_line(&b, "xy", 2); /* la ligne en cours est restituée */
    keys(&b, DOWN);
    check_line(&b, "xy", 2);

    /* Ctrl+P / Ctrl+N et les séquences « ESC O ». */
    keys(&b, "\x10");
    check_line(&b, "help", 4);
    keys(&b, "\x0e");
    check_line(&b, "xy", 2);
    keys(&b, "\033OA");
    check_line(&b, "help", 4);
    keys(&b, "\033OB");
    check_line(&b, "xy", 2);

    /* Une entrée d'historique se modifie comme n'importe quelle ligne. */
    keys(&b, UP "\x7f" "pp");
    check_line(&b, "helpp", 5);

    /* Historique vide, et ligne plus longue que le tampon. */
    nh_le_history_clear();
    start(&b, NULL, NULL, 80);
    keys(&b, UP);
    check_line(&b, "", 0);
    char small[6];
    NhLineEdit ed;
    nh_le_history_add("abcdefghij");
    nh_le_init(&ed, PROMPT, small, sizeof small, NULL, NULL);
    nh_le_feed(&ed, 0x10);
    CHECK_STR(small, "abcde");

    /* La mémoire est bornée : la plus ancienne disparaît. */
    nh_le_history_clear();
    for (int i = 0; i < NH_LE_HISTORY + 8; i++)
    {
        char line[16];
        snprintf(line, sizeof line, "cmd%d", i);
        nh_le_history_add(line);
    }
    CHECK_INT(nh_le_history_count(), NH_LE_HISTORY);
    start(&b, NULL, NULL, 80);
    for (int i = 0; i < NH_LE_HISTORY; i++)
        keys(&b, UP);
    CHECK_STR(b.buf, "cmd8"); /* les 8 premières sont parties */
    nh_le_history_clear();
}

/* ---- Complétion -------------------------------------------------------------------------------- */

static const char *const k_commands[] = {"scan", "shop", "status", "bruteforce", "backdoor", "traceroute", "traceback", NULL};
static const char *const k_names[] = {"R4Z0R", "ECHO-7", "Shadow Broker", NULL};

static void test_completion_unique(void)
{
    Bench b;
    Words w = {k_commands, true};
    start(&b, words_complete, &w, 80);

    keys(&b, "sca\t");
    check_line(&b, "scan ", 5); /* un seul candidat : complété, avec une espace */

    start(&b, words_complete, &w, 80);
    keys(&b, "scan\t"); /* déjà complet : on ajoute quand même l'espace, comme bash */
    check_line(&b, "scan ", 5);

    w.space = false; /* un argument final : pas d'espace derrière */
    start(&b, words_complete, &w, 80);
    keys(&b, "sca\t");
    check_line(&b, "scan", 4);
}

static void test_completion_common_prefix_and_list(void)
{
    Bench b;
    Words w = {k_commands, true};
    start(&b, words_complete, &w, 80);

    keys(&b, "t\t"); /* traceroute et traceback : on avance jusqu'à « trace » */
    check_line(&b, "trace", 5);
    CHECK(!has(nh_le_output(&b.ed), "\a"));
    CHECK(!has(nh_le_output(&b.ed), "traceback"));

    nh_le_output_clear(&b.ed);
    keys(&b, "\t"); /* plus d'avancée possible et TAB deux fois de suite : la liste */
    check_line(&b, "trace", 5);
    const char *listing = nh_le_output(&b.ed);
    CHECK(has(listing, "traceback") && has(listing, "traceroute"));
    CHECK(strstr(listing, "traceback") < strstr(listing, "traceroute")); /* triés */
    CHECK_INT(occurrences(listing, PROMPT), 1); /* le prompt est réécrit UNE fois, sous la liste */
    CHECK(strstr(listing, "traceroute") < strstr(listing, PROMPT));

    /* Ambiguïté sans avancée : le premier TAB sonne, le suivant liste. */
    start(&b, words_complete, &w, 80);
    keys(&b, "s\t");
    check_line(&b, "s", 1);
    CHECK(has(nh_le_output(&b.ed), "\a"));
    CHECK(!has(nh_le_output(&b.ed), "shop"));
    nh_le_output_clear(&b.ed);
    keys(&b, "\t");
    CHECK(has(nh_le_output(&b.ed), "scan") && has(nh_le_output(&b.ed), "shop") && has(nh_le_output(&b.ed), "status"));
    CHECK(!has(nh_le_output(&b.ed), "\a"));

    /* Une autre touche entre deux TAB : on repart de zéro (le prochain TAB sonne à nouveau). */
    start(&b, words_complete, &w, 80);
    keys(&b, "s\t" LEFT RIGHT "\t");
    CHECK(has(nh_le_output(&b.ed), "\a"));
    CHECK(!has(nh_le_output(&b.ed), "shop"));

    /* Ligne vide : tout est proposé (au deuxième TAB). */
    start(&b, words_complete, &w, 80);
    keys(&b, "\t\t");
    CHECK(has(nh_le_output(&b.ed), "bruteforce") && has(nh_le_output(&b.ed), "status"));
}

static void test_completion_case_and_arguments(void)
{
    Bench b;
    Words w = {k_names, false};
    start(&b, words_complete, &w, 80);

    keys(&b, "contact r4\t"); /* casse du candidat, pas celle du joueur */
    check_line(&b, "contact R4Z0R", 13);
    start(&b, words_complete, &w, 80);
    keys(&b, "contact e\t");
    check_line(&b, "contact ECHO-7", 14);

    /* Un nom avec une espace se complète en entier. */
    start(&b, words_complete, &w, 80);
    keys(&b, "contact sha\t");
    check_line(&b, "contact Shadow Broker", 21);
}

static void test_completion_midline_and_limits(void)
{
    Bench b;
    Words w = {k_commands, true};

    /* Curseur au milieu : ce qui le suit est conservé, et l'espace existante n'est pas doublée. */
    start(&b, words_complete, &w, 80);
    keys(&b, "sca extra" LEFT LEFT LEFT LEFT LEFT LEFT "\t");
    check_line(&b, "scan extra", 4);
    keys(&b, "X");
    check_line(&b, "scanX extra", 5);

    /* Aucun candidat, ou pas de fournisseur : la sonnerie, rien d'autre. */
    start(&b, words_complete, &w, 80);
    keys(&b, "zz\t");
    check_line(&b, "zz", 2);
    CHECK(has(nh_le_output(&b.ed), "\a"));
    start(&b, NULL, NULL, 80);
    keys(&b, "sc\t");
    check_line(&b, "sc", 2);
    CHECK(has(nh_le_output(&b.ed), "\a"));

    /* Le mot complété ne tient pas dans le tampon : la ligne reste telle quelle. */
    char small[6];
    NhLineEdit ed;
    nh_le_init(&ed, PROMPT, small, sizeof small, words_complete, &w);
    ed.cols = 80;
    for (const char *p = "sca"; *p != '\0'; p++)
        nh_le_feed(&ed, (unsigned char)*p);
    nh_le_feed(&ed, '\t'); /* « scan » + espace = 5 caractères + fin de chaîne : 6 octets, ça tient */
    CHECK_STR(small, "scan ");
    nh_le_init(&ed, PROMPT, small, sizeof small, words_complete, &w);
    for (const char *p = "bru"; *p != '\0'; p++)
        nh_le_feed(&ed, (unsigned char)*p);
    nh_le_feed(&ed, '\t'); /* « bruteforce » ne tient pas */
    CHECK_STR(small, "bru");
}

/* Le mot de départ d'une complétion peut être n'importe où dans la ligne (les arguments). */
static void offset_complete(void *ctx, const char *before, NhCompletions *out)
{
    (void)ctx;
    out->start = strlen(before) >= 9 ? 9 : 0; /* après « backdoor » et son espace */
    out->count = 1;
    snprintf(out->items[0], NH_LE_CANDIDATE_LEN, "localhost");
}

static void test_completion_start_offset(void)
{
    Bench b;
    start(&b, offset_complete, NULL, 80);
    keys(&b, "backdoor lo\t");
    check_line(&b, "backdoor localhost", 18);
}

static void test_list_layout(void)
{
    static const char *const many[] = {"alpha-001", "alpha-002", "alpha-003", "alpha-004", "alpha-005", "alpha-006",
                                       "alpha-007", "alpha-008", "alpha-009", "alpha-010", "alpha-011", "alpha-012",
                                       "alpha-013", "alpha-014", "alpha-015", "alpha-016", NULL};
    Bench b;
    Words w = {many, false};
    for (int cols = 20; cols <= 100; cols += 20)
    {
        start(&b, words_complete, &w, cols);
        keys(&b, "alpha-0\t\t");
        const char *out = nh_le_output(&b.ed);
        /* Aucune ligne de la liste n'est plus large que l'écran, tous les mots y sont. */
        const char *p = strstr(out, "\r\n");
        bool ok = p != NULL;
        int rows = 0;
        while (ok && p != NULL)
        {
            const char *next = strstr(p + 2, "\r\n");
            if (next == NULL)
                break;
            char line[256];
            size_t n = (size_t)(next - (p + 2));
            if (n >= sizeof line)
                n = sizeof line - 1;
            memcpy(line, p + 2, n);
            line[n] = '\0';
            ok = nh_display_width(line) <= (size_t)(cols - 1);
            rows++;
            p = next;
        }
        CHECK(ok);
        CHECK(rows >= 1);
        for (int i = 0; many[i] != NULL; i++)
            CHECK_INT(occurrences(out, many[i]), 1);
        if (cols == 100)
            CHECK(rows <= 3); /* 100 colonnes : 9 mots de 11 colonnes par rangée */
    }

    /* En colonnes, comme ls : on lit de haut en bas, donc le 2e mot est sous le 1er. */
    start(&b, words_complete, &w, 30);
    keys(&b, "alpha-0\t\t");
    const char *out = nh_le_output(&b.ed);
    CHECK(strstr(out, "alpha-001") < strstr(out, "alpha-002"));
    const char *row1 = strstr(out, "alpha-001");
    const char *eol = strstr(row1, "\r\n");
    CHECK(eol != NULL && strstr(row1, "alpha-002") > eol); /* alpha-002 est sur la rangée suivante */
}

/* ---- Fenêtre de saisie -------------------------------------------------------------------------- */

/* Dernier redessin du texte : ce qui est écrit après le « \r » et le saut du prompt, jusqu'à « effacer la fin de ligne », puis la colonne où va le curseur. */
static void last_redraw(const char *out, char *text, size_t size, int *cursor_col)
{
    const char *erase = NULL;
    for (const char *p = out; (p = strstr(p, "\033[K")) != NULL; p += 3)
        erase = p;
    text[0] = '\0';
    *cursor_col = -1;
    if (erase == NULL)
        return;

    const char *begin = erase;
    while (begin > out && begin[-1] != '\r')
        begin--;
    if (strncmp(begin, "\033[", 2) == 0) /* le saut au-dessus du prompt */
        begin = strchr(begin, 'C') + 1;
    size_t n = (size_t)(erase - begin);
    if (n >= size)
        n = size - 1;
    memcpy(text, begin, n);
    text[n] = '\0';

    const char *forward = strstr(erase + 3, "\033[");
    *cursor_col = forward != NULL ? atoi(forward + 2) : 0;
}

static void test_window_scrolls_with_the_cursor(void)
{
    /* Écran de 20 colonnes, prompt de 2 : 17 colonnes pour le texte, jamais d'écriture dans la dernière colonne. */
    Bench b;
    start(&b, NULL, NULL, 20);
    for (int i = 0; i < 40; i++)
    {
        nh_le_output_clear(&b.ed);
        nh_le_feed(&b.ed, (unsigned char)('a' + i % 26));
        char text[64];
        int col;
        last_redraw(nh_le_output(&b.ed), text, sizeof text, &col);
        CHECK((int)nh_display_width(text) <= 17);
        CHECK(col <= 19); /* index de colonne du curseur : jamais au-delà de la dernière colonne */
        CHECK(col >= 2);
    }
    CHECK_INT(b.ed.len, 40); /* la ligne, elle, est complète */

    /* Le curseur qui remonte fait revenir le début de la ligne dans la fenêtre. */
    nh_le_output_clear(&b.ed);
    nh_le_feed(&b.ed, 0x01);
    CHECK(has(nh_le_output(&b.ed), "abcdefghijklmnopq")); /* les 17 premiers caractères */
    CHECK(has(nh_le_output(&b.ed), "\r\033[2C") || has(nh_le_output(&b.ed), "\033[2C"));

    /* Des caractères larges (2 colonnes) : la fenêtre ne coupe jamais l'un d'eux. */
    start(&b, NULL, NULL, 20);
    for (int i = 0; i < 12; i++)
    {
        keys(&b, "\xE6\x97\xA5"); /* 日 */
        nh_le_output_clear(&b.ed);
        keys(&b, "\xE6\x9C\xAC"); /* 本 */
        char text[128];
        int col;
        last_redraw(nh_le_output(&b.ed), text, sizeof text, &col);
        CHECK((int)nh_display_width(text) <= 17);
        CHECK(col <= 19);
    }

    /* Prompt plus large que l'écran : pas de plantage. */
    start(&b, NULL, NULL, 2);
    keys(&b, "abc" LEFT HOME END);
    check_line(&b, "abc", 3);
}

int main(void)
{
    test_typing_and_enter();
    test_prompt_written_once();
    test_editing_keys();
    test_kill_keys();
    test_eof_and_delete();
    test_utf8();
    test_capacity();
    test_escape_handling();
    test_history();
    test_completion_unique();
    test_completion_common_prefix_and_list();
    test_completion_case_and_arguments();
    test_completion_midline_and_limits();
    test_completion_start_offset();
    test_list_layout();
    test_window_scrolls_with_the_cursor();
    return NH_TEST_REPORT("lineedit");
}
