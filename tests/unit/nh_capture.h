#ifndef NH_CAPTURE_H
#define NH_CAPTURE_H

/* Capture de stdout dans un tampon, pour tester ce qu'une commande affiche (POSIX). */

#include <stdio.h>
#include <unistd.h>

typedef struct
{
    int saved_fd;
    FILE *tmp;
} NhCapture;

static NhCapture nh_capture_begin(void)
{
    NhCapture c;
    fflush(stdout);
    c.tmp = tmpfile();
    c.saved_fd = dup(fileno(stdout));
    dup2(fileno(c.tmp), fileno(stdout));
    return c;
}

static void nh_capture_end(NhCapture *c, char *buf, size_t size)
{
    fflush(stdout);
    dup2(c->saved_fd, fileno(stdout));
    close(c->saved_fd);
    rewind(c->tmp);
    size_t n = fread(buf, 1, size - 1, c->tmp);
    buf[n] = '\0';
    fclose(c->tmp);
}

#endif /* NH_CAPTURE_H */
