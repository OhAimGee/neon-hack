#include "term.h"

#include "../core/platform.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

static bool g_color = true;

void nh_term_set_color(bool on) { g_color = on; }
bool nh_term_color_enabled(void) { return g_color; }

static const char *const k_ansi[NH_C_COUNT] = {
    [NH_C_RESET] = "\033[0m",
    [NH_C_RED] = "\033[31m",
    [NH_C_GREEN] = "\033[32m",
    [NH_C_YELLOW] = "\033[33m",
    [NH_C_BLUE] = "\033[34m",
    [NH_C_MAGENTA] = "\033[35m",
    [NH_C_CYAN] = "\033[36m",
    [NH_C_WHITE] = "\033[37m",
    [NH_C_BRIGHT_GREEN] = "\033[92m",
    [NH_C_BRIGHT_CYAN] = "\033[96m",
};

const char *nh_c(NhColor color)
{
    if (!g_color || color < 0 || color >= NH_C_COUNT)
        return "";
    return k_ansi[color];
}

/* Décode un point de code UTF-8. Octet invalide : retourne cet octet, avance de 1. */
static uint32_t decode_utf8(const unsigned char *s, size_t *len)
{
    unsigned char c = s[0];

    if (c < 0x80)
    {
        *len = 1;
        return c;
    }
    if ((c & 0xE0) == 0xC0 && (s[1] & 0xC0) == 0x80)
    {
        *len = 2;
        return ((uint32_t)(c & 0x1F) << 6) | (s[1] & 0x3F);
    }
    if ((c & 0xF0) == 0xE0 && (s[1] & 0xC0) == 0x80 && (s[2] & 0xC0) == 0x80)
    {
        *len = 3;
        return ((uint32_t)(c & 0x0F) << 12) | ((uint32_t)(s[1] & 0x3F) << 6) | (s[2] & 0x3F);
    }
    if ((c & 0xF8) == 0xF0 && (s[1] & 0xC0) == 0x80 && (s[2] & 0xC0) == 0x80 &&
        (s[3] & 0xC0) == 0x80)
    {
        *len = 4;
        return ((uint32_t)(c & 0x07) << 18) | ((uint32_t)(s[1] & 0x3F) << 12) |
               ((uint32_t)(s[2] & 0x3F) << 6) | (s[3] & 0x3F);
    }
    *len = 1;
    return c;
}

typedef struct
{
    uint32_t lo, hi;
} Range;

static bool in_ranges(uint32_t cp, const Range *r, size_t n)
{
    for (size_t i = 0; i < n; i++)
    {
        if (cp >= r[i].lo && cp <= r[i].hi)
            return true;
    }
    return false;
}

/* Largeur 0 : accents combinants, sélecteurs de variation, caractères de liaison. */
static const Range k_zero[] = {
    {0x0300, 0x036F}, {0x200B, 0x200F}, {0x20D0, 0x20FF}, {0xFE00, 0xFE0F}, {0xFE20, 0xFE2F},
};

/* Largeur 2 : CJK, hangul, formes pleine largeur, emoji à présentation emoji. */
static const Range k_wide[] = {
    {0x1100, 0x115F},   {0x231A, 0x231B},   {0x23E9, 0x23EC},   {0x23F0, 0x23F0},
    {0x23F3, 0x23F3},   {0x25FD, 0x25FE},   {0x2614, 0x2615},   {0x2648, 0x2653},
    {0x267F, 0x267F},   {0x2693, 0x2693},   {0x26A1, 0x26A1},   {0x26AA, 0x26AB},
    {0x26BD, 0x26BE},   {0x26C4, 0x26C5},   {0x26CE, 0x26CE},   {0x26D4, 0x26D4},
    {0x26EA, 0x26EA},   {0x26F2, 0x26F3},   {0x26F5, 0x26F5},   {0x26FA, 0x26FA},
    {0x26FD, 0x26FD},   {0x2705, 0x2705},   {0x270A, 0x270B},   {0x2728, 0x2728},
    {0x274C, 0x274C},   {0x274E, 0x274E},   {0x2753, 0x2755},   {0x2757, 0x2757},
    {0x2795, 0x2797},   {0x27B0, 0x27B0},   {0x27BF, 0x27BF},   {0x2B1B, 0x2B1C},
    {0x2B50, 0x2B50},   {0x2B55, 0x2B55},   {0x2E80, 0xA4CF},   {0xAC00, 0xD7A3},
    {0xF900, 0xFAFF},   {0xFE30, 0xFE6F},   {0xFF00, 0xFF60},   {0xFFE0, 0xFFE6},
    {0x1F300, 0x1F64F}, {0x1F680, 0x1F6FF}, {0x1F900, 0x1F9FF}, {0x1FA70, 0x1FAFF},
    {0x20000, 0x3FFFD},
};

static int cp_width(uint32_t cp)
{
    if (cp == 0)
        return 0;
    if (cp < 0x20 || (cp >= 0x7F && cp < 0xA0))
        return 0; /* caractères de contrôle */
    if (in_ranges(cp, k_zero, sizeof k_zero / sizeof k_zero[0]))
        return 0;
    if (in_ranges(cp, k_wide, sizeof k_wide / sizeof k_wide[0]))
        return 2;
    return 1;
}

/* Si `s` commence par une séquence CSI (ESC [ ... lettre), retourne sa longueur, sinon 0. */
static size_t ansi_csi_length(const unsigned char *s)
{
    if (s[0] != 0x1B || s[1] != '[')
        return 0;
    size_t i = 2;
    while (s[i] >= 0x20 && s[i] <= 0x3F) /* paramètres et intermédiaires */
        i++;
    if (s[i] >= 0x40 && s[i] <= 0x7E)
        return i + 1;
    return 0; /* séquence tronquée : la traiter comme du texte */
}

size_t nh_display_width(const char *utf8)
{
    const unsigned char *s = (const unsigned char *)utf8;
    size_t width = 0;

    while (*s)
    {
        size_t skip = ansi_csi_length(s);
        if (skip)
        {
            s += skip;
            continue;
        }
        size_t len;
        uint32_t cp = decode_utf8(s, &len);
        width += (size_t)cp_width(cp);
        s += len;
    }
    return width;
}

size_t nh_pad(char *out, size_t out_size, const char *s, size_t width, NhAlign align)
{
    if (out_size == 0)
        return 0;

    size_t w = nh_display_width(s);
    size_t gap = width > w ? width - w : 0;
    size_t left = 0;
    size_t right = gap;
    if (align == NH_ALIGN_RIGHT)
    {
        left = gap;
        right = 0;
    }
    else if (align == NH_ALIGN_CENTER)
    {
        left = gap / 2;
        right = gap - left;
    }

    size_t pos = 0;
    for (size_t i = 0; i < left && pos + 1 < out_size; i++)
        out[pos++] = ' ';
    size_t slen = strlen(s);
    for (size_t i = 0; i < slen && pos + 1 < out_size; i++)
        out[pos++] = s[i];
    for (size_t i = 0; i < right && pos + 1 < out_size; i++)
        out[pos++] = ' ';
    out[pos] = '\0';
    return pos;
}

void nh_typewriter(const char *text, unsigned delay_ms)
{
    const unsigned char *s = (const unsigned char *)text;

    while (*s)
    {
        size_t len;
        decode_utf8(s, &len);
        fwrite(s, 1, len, stdout);
        fflush(stdout);
        nh_sleep_ms(delay_ms);
        s += len;
    }
}
