#include "rng.h"

uint32_t nh_rng_u32(NhRng *rng)
{
    uint64_t old = rng->state;
    rng->state = old * 6364136223846793005ULL + (rng->inc | 1u);

    uint32_t xorshifted = (uint32_t)(((old >> 18u) ^ old) >> 27u);
    uint32_t rot = (uint32_t)(old >> 59u);
    return (xorshifted >> rot) | (xorshifted << ((32u - rot) & 31u));
}

void nh_rng_seed(NhRng *rng, uint64_t seed, uint64_t stream)
{
    rng->state = 0u;
    rng->inc = (stream << 1u) | 1u;
    nh_rng_u32(rng);
    rng->state += seed;
    nh_rng_u32(rng);
}

int nh_rng_range(NhRng *rng, int lo, int hi)
{
    if (hi <= lo)
        return lo;

    int64_t span = (int64_t)hi - (int64_t)lo + 1;
    if (span > (int64_t)UINT32_MAX)
        return (int)((int64_t)lo + (int64_t)nh_rng_u32(rng)); /* plage complète de 32 bits */

    uint32_t bound = (uint32_t)span;
    uint32_t threshold = (uint32_t)(-bound) % bound;
    uint32_t r;
    do
    {
        r = nh_rng_u32(rng);
    } while (r < threshold);

    return (int)((int64_t)lo + (int64_t)(r % bound));
}

bool nh_rng_chance(NhRng *rng, int percent)
{
    if (percent <= 0)
        return false;
    if (percent >= 100)
        return true;
    return nh_rng_range(rng, 0, 99) < percent;
}
