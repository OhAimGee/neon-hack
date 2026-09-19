#ifndef NH_TEST_H
#define NH_TEST_H

/* Mini-framework de tests : un exécutable par fichier, code retour != 0 si un CHECK échoue. */

#include <stdio.h>
#include <string.h>

static int nh_t_total = 0;
static int nh_t_failed = 0;

#define CHECK(cond)                                                              \
    do                                                                           \
    {                                                                            \
        nh_t_total++;                                                            \
        if (!(cond))                                                             \
        {                                                                        \
            nh_t_failed++;                                                       \
            fprintf(stderr, "  FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);    \
        }                                                                        \
    } while (0)

#define CHECK_INT(actual, expected)                                              \
    do                                                                           \
    {                                                                            \
        long long a_ = (long long)(actual), e_ = (long long)(expected);          \
        nh_t_total++;                                                            \
        if (a_ != e_)                                                            \
        {                                                                        \
            nh_t_failed++;                                                       \
            fprintf(stderr, "  FAIL %s:%d: %s = %lld, attendu %lld\n", __FILE__, \
                    __LINE__, #actual, a_, e_);                                  \
        }                                                                        \
    } while (0)

#define CHECK_STR(actual, expected)                                              \
    do                                                                           \
    {                                                                            \
        const char *a_ = (actual), *e_ = (expected);                             \
        nh_t_total++;                                                            \
        if (a_ == NULL || strcmp(a_, e_) != 0)                                   \
        {                                                                        \
            nh_t_failed++;                                                       \
            fprintf(stderr, "  FAIL %s:%d: %s = \"%s\", attendu \"%s\"\n",       \
                    __FILE__, __LINE__, #actual, a_ ? a_ : "(null)", e_);        \
        }                                                                        \
    } while (0)

#define NH_TEST_REPORT(name)                                                     \
    (printf("%-14s %3d checks, %d échec(s)\n", (name), nh_t_total, nh_t_failed), \
     nh_t_failed == 0 ? 0 : 1)

#endif /* NH_TEST_H */
