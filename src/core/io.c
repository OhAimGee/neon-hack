#include "io.h"

#include "utf8.h"

#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>

static FILE *g_in = NULL;

static FILE *input(void) { return g_in ? g_in : stdin; }

void nh_io_set_input(FILE *in) { g_in = in; }

bool nh_io_uses_stdin(void) { return g_in == NULL || g_in == stdin; }

NhIoStatus nh_read_line(char *buf, size_t size)
{
    if (size == 0)
        return NH_IO_ERROR;

    FILE *in = input();
    buf[0] = '\0';

    if (fgets(buf, (int)(size > INT_MAX ? INT_MAX : size), in) == NULL)
    {
        buf[0] = '\0';
        return feof(in) ? NH_IO_EOF : NH_IO_ERROR;
    }

    size_t len = strlen(buf);
    if (len > 0 && buf[len - 1] == '\n')
    {
        buf[--len] = '\0';
    }
    else if (len == size - 1)
    {
        /* Ligne plus longue que le tampon : jeter la suite jusqu'au saut de ligne. */
        int c;
        while ((c = getc(in)) != EOF && c != '\n')
        {
        }
        /* La coupure a pu tomber au milieu d'un caractère accentué. */
        nh_utf8_trim_incomplete(buf);
        len = strlen(buf);
    }
    /* Sinon : dernière ligne sans "\n" avant EOF, on la garde telle quelle. */

    if (len > 0 && buf[len - 1] == '\r')
        buf[--len] = '\0';

    return NH_IO_OK;
}

bool nh_parse_int(const char *text, int min, int max, int *out)
{
    if (text == NULL)
        return false;

    while (isspace((unsigned char)*text))
        text++;
    if (*text == '\0')
        return false;

    char *end = NULL;
    errno = 0;
    long value = strtol(text, &end, 10);
    if (errno == ERANGE || end == text)
        return false;

    while (isspace((unsigned char)*end))
        end++;
    if (*end != '\0')
        return false;

    if (value < (long)min || value > (long)max)
        return false;

    *out = (int)value;
    return true;
}

NhIoStatus nh_read_int(int *out, int min, int max)
{
    char line[64];
    NhIoStatus status = nh_read_line(line, sizeof line);
    if (status != NH_IO_OK)
        return status;

    return nh_parse_int(line, min, max, out) ? NH_IO_OK : NH_IO_INVALID;
}
