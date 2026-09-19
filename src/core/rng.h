#ifndef NH_RNG_H
#define NH_RNG_H

#include <stdbool.h>
#include <stdint.h>

/* Générateur PCG32 : petit, rapide, reproductible d'une plateforme à l'autre
 * (contrairement à rand(), dont la suite dépend de la libc). */
typedef struct
{
    uint64_t state;
    uint64_t inc;
} NhRng;

void nh_rng_seed(NhRng *rng, uint64_t seed, uint64_t stream);
uint32_t nh_rng_u32(NhRng *rng);

/* Entier uniforme dans [lo, hi] (bornes incluses, sans biais de modulo). */
int nh_rng_range(NhRng *rng, int lo, int hi);

/* Vrai avec une probabilité de `percent` % (0 = jamais, 100 = toujours). */
bool nh_rng_chance(NhRng *rng, int percent);

#endif /* NH_RNG_H */
