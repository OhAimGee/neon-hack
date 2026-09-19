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

/* `s` commence-t-il par `prefix`, sans tenir compte de la casse ? Un préfixe vide convient à tout. */
bool nh_str_has_prefix_nocase(const char *s, const char *prefix);

/* Nombre maximal de caractères (pas d'octets) du nom du héros. */
#define NH_NAME_MAX_CHARS 20

/*
 * Nettoie un nom saisi : espaces de tête et de queue retirés, espaces multiples réduits à un,
 * caractères de contrôle supprimés (dont ESC : un nom ne doit jamais pouvoir piloter le
 * terminal), séquences UTF-8 invalides supprimées, coupé à `max_chars` caractères sans jamais
 * casser un caractère accentué. `out` est toujours terminé par '\0'. Retourne la longueur en
 * octets ; 0 si rien de présentable ne reste (l'appelant prend alors le nom par défaut).
 */
size_t nh_clean_name(const char *in, char *out, size_t out_size, size_t max_chars);

/*
 * Réponse oui/non : « o », « oui », « y », « yes » = vrai ; « n », « non », « no » = faux (casse et
 * espaces ignorés). Ligne vide ou incompréhensible : `default_yes`.
 */
bool nh_parse_yes_no(const char *line, bool default_yes);

#endif /* NH_PARSE_H */
