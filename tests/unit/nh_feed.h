#ifndef NH_FEED_H
#define NH_FEED_H

/* Fournit du texte comme entrée du joueur (io.c) : un test ne doit jamais attendre le vrai clavier. */

#include <stdio.h>

#include "../../src/core/io.h"

static FILE *nh_fed = NULL;

/* Ferme le flux précédent ; `text` sera lu par nh_read_line. "" = fin d'entrée immédiate. */
static inline void nh_feed(const char *text)
{
    FILE *f = tmpfile();
    fputs(text, f);
    rewind(f);
    nh_io_set_input(f);
    if (nh_fed != NULL)
        fclose(nh_fed);
    nh_fed = f;
}

static inline void nh_unfeed(void)
{
    nh_io_set_input(NULL);
    if (nh_fed != NULL)
        fclose(nh_fed);
    nh_fed = NULL;
}

#endif /* NH_FEED_H */
