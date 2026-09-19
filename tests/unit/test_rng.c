#include "nh_test.h"

#include "../../src/core/rng.h"

#include <limits.h>

static void test_reference_vectors(void)
{
    /* Sortie de référence de l'implémentation officielle pcg32 (seed 42, séquence 54). */
    static const uint32_t expected[] = {0xa15c02b7u, 0x7b47f409u, 0xba1d3330u,
                                        0x83d2f293u, 0xbfa4784bu, 0xcbed606eu};
    NhRng rng;
    nh_rng_seed(&rng, 42u, 54u);
    for (size_t i = 0; i < sizeof expected / sizeof expected[0]; i++)
    {
        CHECK_INT(nh_rng_u32(&rng), expected[i]);
    }
}

static void test_determinism(void)
{
    NhRng a, b, c;
    nh_rng_seed(&a, 1234u, 1u);
    nh_rng_seed(&b, 1234u, 1u);
    nh_rng_seed(&c, 1235u, 1u);

    bool same = true, differs = false;
    for (int i = 0; i < 100; i++)
    {
        uint32_t x = nh_rng_u32(&a), y = nh_rng_u32(&b), z = nh_rng_u32(&c);
        if (x != y)
            same = false;
        if (x != z)
            differs = true;
    }
    CHECK(same);
    CHECK(differs);
}

static void test_range(void)
{
    NhRng rng;
    nh_rng_seed(&rng, 7u, 1u);

    int counts[7] = {0};
    for (int i = 0; i < 6000; i++)
    {
        int v = nh_rng_range(&rng, 1, 6);
        CHECK(v >= 1 && v <= 6);
        if (v >= 1 && v <= 6)
            counts[v]++;
    }
    for (int face = 1; face <= 6; face++)
    {
        CHECK(counts[face] > 800 && counts[face] < 1200); /* ~1000 attendu */
    }

    CHECK_INT(nh_rng_range(&rng, 5, 5), 5);
    CHECK_INT(nh_rng_range(&rng, 9, 3), 9); /* plage vide : renvoie lo */

    for (int i = 0; i < 1000; i++)
    {
        int v = nh_rng_range(&rng, -3, 3);
        CHECK(v >= -3 && v <= 3);
    }

    /* Plage complète : ne doit ni planter (division par zéro) ni sortir des bornes. */
    (void)nh_rng_range(&rng, INT_MIN, INT_MAX);
    int wide = nh_rng_range(&rng, INT_MIN, INT_MAX - 1);
    CHECK(wide >= INT_MIN && wide <= INT_MAX - 1);
}

static void test_no_modulo_bias(void)
{
    /*
     * Intervalle de 3 * 2^30 valeurs : un simple `% bound` ferait tomber
     * 1/2 des tirages dans le premier tiers de la plage au lieu de 1/3.
     * (Sur un intervalle de 6 valeurs le biais serait invisible : 1e-9.)
     */
    NhRng rng;
    nh_rng_seed(&rng, 2024u, 1u);

    const int lo = INT_MIN;
    const int hi = INT_MIN + 1610612735 + 1610612736; /* lo + 3*2^30 - 1 */
    const int first_third_end = INT_MIN + 1073741824;

    int in_first_third = 0;
    for (int i = 0; i < 9000; i++)
    {
        int v = nh_rng_range(&rng, lo, hi);
        CHECK(v >= lo && v <= hi);
        if (v < first_third_end)
            in_first_third++;
    }
    CHECK(in_first_third > 2700 && in_first_third < 3300); /* ~3000 (1/3) ; biaisé : ~4500 */
}

static void test_chance(void)
{
    NhRng rng;
    nh_rng_seed(&rng, 99u, 1u);

    for (int i = 0; i < 1000; i++)
    {
        CHECK(!nh_rng_chance(&rng, 0));
        CHECK(!nh_rng_chance(&rng, -10));
        CHECK(nh_rng_chance(&rng, 100));
        CHECK(nh_rng_chance(&rng, 250));
    }

    int hits = 0;
    for (int i = 0; i < 10000; i++)
    {
        if (nh_rng_chance(&rng, 30))
            hits++;
    }
    CHECK(hits > 2700 && hits < 3300); /* ~3000 attendu */
}

int main(void)
{
    test_reference_vectors();
    test_determinism();
    test_range();
    test_no_modulo_bias();
    test_chance();
    return NH_TEST_REPORT("rng");
}
