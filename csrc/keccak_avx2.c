#include <immintrin.h>
#include <stdint.h>

/*
 * 4-way parallel Keccak-f[1600] using AVX2.
 * State layout: 25 lanes x 4 instances, interleaved.
 * states[i*4 + j] = lane i of instance j.
 */

static const uint64_t RC[24] = {
    0x0000000000000001ULL, 0x0000000000008082ULL,
    0x800000000000808AULL, 0x8000000080008000ULL,
    0x000000000000808BULL, 0x0000000080000001ULL,
    0x8000000080008081ULL, 0x8000000000008009ULL,
    0x000000000000008AULL, 0x0000000000000088ULL,
    0x0000000080008009ULL, 0x000000008000000AULL,
    0x000000008000808BULL, 0x800000000000008BULL,
    0x8000000000008089ULL, 0x8000000000008003ULL,
    0x8000000000008002ULL, 0x8000000000000080ULL,
    0x000000000000800AULL, 0x800000008000000AULL,
    0x8000000080008081ULL, 0x8000000000008080ULL,
    0x0000000080000001ULL, 0x8000000080008008ULL,
};

static const int RHO[25] = {
     0,  1, 62, 28, 27,
    36, 44,  6, 55, 20,
     3, 10, 43, 25, 39,
    41, 45, 15, 21,  8,
    18,  2, 61, 56, 14,
};

static const int PI[25] = {
     0, 10, 20,  5, 15,
    16,  1, 11, 21,  6,
     7, 17,  2, 12, 22,
    23,  8, 18,  3, 13,
    14, 24,  9, 19,  4,
};

static inline __m256i rotl64(__m256i x, int n) {
    if (n == 0) return x;
    return _mm256_or_si256(_mm256_slli_epi64(x, n), _mm256_srli_epi64(x, 64 - n));
}

void keccak_f1600_x4(uint64_t *states) {
    __m256i A[25], B[25], C[5], D[5];
    int i, round, x, y;

    for (i = 0; i < 25; i++)
        A[i] = _mm256_loadu_si256((__m256i*)(states + i * 4));

    for (round = 0; round < 24; round++) {
        /* Theta */
        for (x = 0; x < 5; x++)
            C[x] = _mm256_xor_si256(_mm256_xor_si256(A[x], A[x+5]),
                   _mm256_xor_si256(A[x+10], _mm256_xor_si256(A[x+15], A[x+20])));
        for (x = 0; x < 5; x++)
            D[x] = _mm256_xor_si256(C[(x+4)%5], rotl64(C[(x+1)%5], 1));
        for (i = 0; i < 25; i++)
            A[i] = _mm256_xor_si256(A[i], D[i%5]);

        /* Rho + Pi */
        for (i = 0; i < 25; i++)
            B[PI[i]] = rotl64(A[i], RHO[i]);

        /* Chi */
        for (y = 0; y < 5; y++)
            for (x = 0; x < 5; x++)
                A[y*5+x] = _mm256_xor_si256(B[y*5+x],
                    _mm256_andnot_si256(B[y*5+(x+1)%5], B[y*5+(x+2)%5]));

        /* Iota */
        A[0] = _mm256_xor_si256(A[0], _mm256_set1_epi64x(RC[round]));
    }

    for (i = 0; i < 25; i++)
        _mm256_storeu_si256((__m256i*)(states + i * 4), A[i]);
}
