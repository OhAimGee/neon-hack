#ifndef NH_UTF8_H
#define NH_UTF8_H

/*
 * Retire de `s` une séquence UTF-8 incomplète en fin de chaîne (un caractère
 * accentué coupé en deux par une troncature). Ne touche pas à une chaîne
 * qui se termine proprement.
 */
void nh_utf8_trim_incomplete(char *s);

#endif /* NH_UTF8_H */
