#include "utf8.h"

#include <string.h>

void nh_utf8_trim_incomplete(char *s)
{
    size_t len = strlen(s);
    if (len == 0)
        return;

    /* Remonter jusqu'à l'octet de tête de la dernière séquence (au plus 3 octets de suite). */
    size_t start = len - 1;
    size_t back = 0;
    while (start > 0 && back < 3 && ((unsigned char)s[start] & 0xC0) == 0x80)
    {
        start--;
        back++;
    }

    unsigned char lead = (unsigned char)s[start];
    size_t expected = 1;
    if (lead >= 0xF0)
        expected = 4;
    else if (lead >= 0xE0)
        expected = 3;
    else if (lead >= 0xC0)
        expected = 2;

    if (len - start < expected)
        s[start] = '\0';
}
