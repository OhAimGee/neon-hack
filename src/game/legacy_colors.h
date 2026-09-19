#ifndef NH_LEGACY_COLORS_H
#define NH_LEGACY_COLORS_H

/*
 * Couleurs ANSI du code d'origine : des constantes de compilation, donc non
 * désactivables. Les écrans portés au nouveau socle utilisent nh_c() (ui/term.h)
 * qui respecte --no-color. Ce fichier disparaîtra avec le dernier écran d'origine.
 */
#define COLOR_RESET "\033[0m"
#define COLOR_RED "\033[31m"
#define COLOR_GREEN "\033[32m"
#define COLOR_YELLOW "\033[33m"
#define COLOR_BLUE "\033[34m"
#define COLOR_MAGENTA "\033[35m"
#define COLOR_CYAN "\033[36m"
#define COLOR_WHITE "\033[37m"
#define COLOR_BRIGHT_GREEN "\033[92m"
#define COLOR_BRIGHT_CYAN "\033[96m"

#endif /* NH_LEGACY_COLORS_H */
