#ifndef NH_IO_H
#define NH_IO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>

typedef enum
{
    NH_IO_OK = 0,
    NH_IO_EOF,     /* entrée fermée : l'appelant doit s'arrêter ou annuler */
    NH_IO_ERROR,   /* erreur de lecture */
    NH_IO_INVALID  /* ligne lue mais ce n'est pas un entier valide / dans la plage */
} NhIoStatus;

/* Source d'entrée (tests). NULL restaure stdin. */
void nh_io_set_input(FILE *in);

/*
 * Lit une ligne dans buf (taille size >= 1), sans le "\r\n" final.
 * Une ligne trop longue est tronquée et le reste est ignoré : il ne
 * "fuit" jamais dans la lecture suivante.
 * Sur EOF ou erreur, buf est mis à "" et le statut le dit.
 */
NhIoStatus nh_read_line(char *buf, size_t size);

/* Analyse stricte d'un entier décimal (espaces autorisés autour), dans [min, max]. */
bool nh_parse_int(const char *text, int min, int max, int *out);

/* Lit une ligne puis l'analyse. *out n'est modifié que si le statut est NH_IO_OK. */
NhIoStatus nh_read_int(int *out, int min, int max);

#endif /* NH_IO_H */
