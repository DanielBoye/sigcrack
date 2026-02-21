#include <immintrin.h>
#include <stdint.h>

/* Same constants as AVX2 version */
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
     0,  1, 62, 28, 27,  36, 44,  6, 55, 20,
     3, 10, 43, 25, 39,  41, 45, 15, 21,  8,
    18,  2, 61, 56, 14,
};

static const int PI[25] = {
     0, 10, 20,  5, 15,  16,  1, 11, 21,  6,
     7, 17,  2, 12, 22,  23,  8, 18,  3, 13,
    14, 24,  9, 19,  4,
};

static inline __m512i rotl64_512(__m512i x, int n) {
    if (n == 0) return x;
    /* Use variable rotate: broadcast n into all 8 lanes */
    return _mm512_rolv_epi64(x, _mm512_set1_epi64(n));
}

void keccak_f1600_x8(uint64_t *states) {
    __m512i A[25], B[25], C[5], D[5];
    int i, round, x, y;

    for (i = 0; i < 25; i++)
        A[i] = _mm512_loadu_si512((__m512i*)(states + i * 8));

    for (round = 0; round < 24; round++) {
        for (x = 0; x < 5; x++)
            C[x] = _mm512_xor_si512(_mm512_xor_si512(A[x], A[x+5]),
                   _mm512_xor_si512(A[x+10], _mm512_xor_si512(A[x+15], A[x+20])));
        for (x = 0; x < 5; x++)
            D[x] = _mm512_xor_si512(C[(x+4)%5], rotl64_512(C[(x+1)%5], 1));
        for (i = 0; i < 25; i++)
            A[i] = _mm512_xor_si512(A[i], D[i%5]);

        for (i = 0; i < 25; i++)
            B[PI[i]] = rotl64_512(A[i], RHO[i]);

        /* Chi: a ^ (~b & c) = ternarylogic(a, b, c, 0xD2) */
        for (y = 0; y < 5; y++) {
            __m512i t[5];
            for (x = 0; x < 5; x++) t[x] = B[y*5+x];
            for (x = 0; x < 5; x++)
                A[y*5+x] = _mm512_ternarylogic_epi64(t[x], t[(x+1)%5], t[(x+2)%5], 0xD2);
        }

        A[0] = _mm512_xor_si512(A[0], _mm512_set1_epi64(RC[round]));
    }

    for (i = 0; i < 25; i++)
        _mm512_storeu_si512((__m512i*)(states + i * 8), A[i]);
}
