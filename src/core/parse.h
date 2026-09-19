#ifndef NH_PARSE_H
#define NH_PARSE_H

#include <stdbool.h>
#include <stddef.h>

/*
 * Sépare une ligne saisie en commande + argument :
 *   "  bruteforce   localhost  "  ->  cmd="bruteforce", arg="localhost"
 * Les espaces autour sont ignorés ; l'argument garde ses espaces internes.
 * Un mot ou un argument trop long est tronqué (jamais de dépassement de
 * tampon, jamais au milieu d'un caractère UTF-8).
 * Retourne false si la ligne ne contient aucune commande.
 */
bool nh_split_command(const char *line, char *cmd, size_t cmd_size, char *arg, size_t arg_size);

/* Égalité sans tenir compte de la casse (ASCII uniquement, indépendant de la locale). */
bool nh_str_eq_nocase(const char *a, const char *b);

#endif /* NH_PARSE_H */
