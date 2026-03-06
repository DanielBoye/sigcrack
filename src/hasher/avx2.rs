use super::{absorb, selector, Selector};
use std::arch::x86_64::*;

const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808A,
    0x8000000080008000,
    0x000000000000808B,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008A,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000A,
    0x000000008000808B,
    0x800000000000008B,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800A,
    0x800000008000000A,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

macro_rules! rol {
    ($x:expr, $n:literal) => {
        _mm256_or_si256(
            _mm256_slli_epi64::<$n>($x),
            _mm256_srli_epi64::<{ 64 - $n }>($x),
        )
    };
}

/// 4-way Keccak-256 truncated to the selector word: `rate[i * 4 + lane]` is
/// rate word `i` of instance `lane`, capacity words are zero. Only state word 0
/// is returned, so the last round computes just that lane.
/// Fully unrolled so every rotation is an immediate shift pair.
///
/// # Safety
/// The CPU must support AVX2.
#[target_feature(enable = "avx2")]
unsafe fn keccak256_selectors_x4(rate: &[u64; 68]) -> [u64; 4] {
    let p = rate.as_ptr();
    let mut a0 = _mm256_loadu_si256(p.add(0) as *const __m256i);
    let mut a1 = _mm256_loadu_si256(p.add(4) as *const __m256i);
    let mut a2 = _mm256_loadu_si256(p.add(8) as *const __m256i);
    let mut a3 = _mm256_loadu_si256(p.add(12) as *const __m256i);
    let mut a4 = _mm256_loadu_si256(p.add(16) as *const __m256i);
    let mut a5 = _mm256_loadu_si256(p.add(20) as *const __m256i);
    let mut a6 = _mm256_loadu_si256(p.add(24) as *const __m256i);
    let mut a7 = _mm256_loadu_si256(p.add(28) as *const __m256i);
    let mut a8 = _mm256_loadu_si256(p.add(32) as *const __m256i);
    let mut a9 = _mm256_loadu_si256(p.add(36) as *const __m256i);
    let mut a10 = _mm256_loadu_si256(p.add(40) as *const __m256i);
    let mut a11 = _mm256_loadu_si256(p.add(44) as *const __m256i);
    let mut a12 = _mm256_loadu_si256(p.add(48) as *const __m256i);
    let mut a13 = _mm256_loadu_si256(p.add(52) as *const __m256i);
    let mut a14 = _mm256_loadu_si256(p.add(56) as *const __m256i);
    let mut a15 = _mm256_loadu_si256(p.add(60) as *const __m256i);
    let mut a16 = _mm256_loadu_si256(p.add(64) as *const __m256i);
    let mut a17 = _mm256_setzero_si256();
    let mut a18 = _mm256_setzero_si256();
    let mut a19 = _mm256_setzero_si256();
    let mut a20 = _mm256_setzero_si256();
    let mut a21 = _mm256_setzero_si256();
    let mut a22 = _mm256_setzero_si256();
    let mut a23 = _mm256_setzero_si256();
    let mut a24 = _mm256_setzero_si256();
    for rc in &RC[..23] {
        let rc = _mm256_set1_epi64x(*rc as i64);
        let c0 = _mm256_xor_si256(
            _mm256_xor_si256(a0, a5),
            _mm256_xor_si256(a10, _mm256_xor_si256(a15, a20)),
        );
        let c1 = _mm256_xor_si256(
            _mm256_xor_si256(a1, a6),
            _mm256_xor_si256(a11, _mm256_xor_si256(a16, a21)),
        );
        let c2 = _mm256_xor_si256(
            _mm256_xor_si256(a2, a7),
            _mm256_xor_si256(a12, _mm256_xor_si256(a17, a22)),
        );
        let c3 = _mm256_xor_si256(
            _mm256_xor_si256(a3, a8),
            _mm256_xor_si256(a13, _mm256_xor_si256(a18, a23)),
        );
        let c4 = _mm256_xor_si256(
            _mm256_xor_si256(a4, a9),
            _mm256_xor_si256(a14, _mm256_xor_si256(a19, a24)),
        );
        let d0 = _mm256_xor_si256(c4, rol!(c1, 1));
        let d1 = _mm256_xor_si256(c0, rol!(c2, 1));
        let d2 = _mm256_xor_si256(c1, rol!(c3, 1));
        let d3 = _mm256_xor_si256(c2, rol!(c4, 1));
        let d4 = _mm256_xor_si256(c3, rol!(c0, 1));
        a0 = _mm256_xor_si256(a0, d0);
        a1 = _mm256_xor_si256(a1, d1);
        a2 = _mm256_xor_si256(a2, d2);
        a3 = _mm256_xor_si256(a3, d3);
        a4 = _mm256_xor_si256(a4, d4);
        a5 = _mm256_xor_si256(a5, d0);
        a6 = _mm256_xor_si256(a6, d1);
        a7 = _mm256_xor_si256(a7, d2);
        a8 = _mm256_xor_si256(a8, d3);
        a9 = _mm256_xor_si256(a9, d4);
        a10 = _mm256_xor_si256(a10, d0);
        a11 = _mm256_xor_si256(a11, d1);
        a12 = _mm256_xor_si256(a12, d2);
        a13 = _mm256_xor_si256(a13, d3);
        a14 = _mm256_xor_si256(a14, d4);
        a15 = _mm256_xor_si256(a15, d0);
        a16 = _mm256_xor_si256(a16, d1);
        a17 = _mm256_xor_si256(a17, d2);
        a18 = _mm256_xor_si256(a18, d3);
        a19 = _mm256_xor_si256(a19, d4);
        a20 = _mm256_xor_si256(a20, d0);
        a21 = _mm256_xor_si256(a21, d1);
        a22 = _mm256_xor_si256(a22, d2);
        a23 = _mm256_xor_si256(a23, d3);
        a24 = _mm256_xor_si256(a24, d4);
        let b0 = a0;
        let b10 = rol!(a1, 1);
        let b20 = rol!(a2, 62);
        let b5 = rol!(a3, 28);
        let b15 = rol!(a4, 27);
        let b16 = rol!(a5, 36);
        let b1 = rol!(a6, 44);
        let b11 = rol!(a7, 6);
        let b21 = rol!(a8, 55);
        let b6 = rol!(a9, 20);
        let b7 = rol!(a10, 3);
        let b17 = rol!(a11, 10);
        let b2 = rol!(a12, 43);
        let b12 = rol!(a13, 25);
        let b22 = rol!(a14, 39);
        let b23 = rol!(a15, 41);
        let b8 = rol!(a16, 45);
        let b18 = rol!(a17, 15);
        let b3 = rol!(a18, 21);
        let b13 = rol!(a19, 8);
        let b14 = rol!(a20, 18);
        let b24 = rol!(a21, 2);
        let b9 = rol!(a22, 61);
        let b19 = rol!(a23, 56);
        let b4 = rol!(a24, 14);
        a0 = _mm256_xor_si256(b0, _mm256_andnot_si256(b1, b2));
        a1 = _mm256_xor_si256(b1, _mm256_andnot_si256(b2, b3));
        a2 = _mm256_xor_si256(b2, _mm256_andnot_si256(b3, b4));
        a3 = _mm256_xor_si256(b3, _mm256_andnot_si256(b4, b0));
        a4 = _mm256_xor_si256(b4, _mm256_andnot_si256(b0, b1));
        a5 = _mm256_xor_si256(b5, _mm256_andnot_si256(b6, b7));
        a6 = _mm256_xor_si256(b6, _mm256_andnot_si256(b7, b8));
        a7 = _mm256_xor_si256(b7, _mm256_andnot_si256(b8, b9));
        a8 = _mm256_xor_si256(b8, _mm256_andnot_si256(b9, b5));
        a9 = _mm256_xor_si256(b9, _mm256_andnot_si256(b5, b6));
        a10 = _mm256_xor_si256(b10, _mm256_andnot_si256(b11, b12));
        a11 = _mm256_xor_si256(b11, _mm256_andnot_si256(b12, b13));
        a12 = _mm256_xor_si256(b12, _mm256_andnot_si256(b13, b14));
        a13 = _mm256_xor_si256(b13, _mm256_andnot_si256(b14, b10));
        a14 = _mm256_xor_si256(b14, _mm256_andnot_si256(b10, b11));
        a15 = _mm256_xor_si256(b15, _mm256_andnot_si256(b16, b17));
        a16 = _mm256_xor_si256(b16, _mm256_andnot_si256(b17, b18));
        a17 = _mm256_xor_si256(b17, _mm256_andnot_si256(b18, b19));
        a18 = _mm256_xor_si256(b18, _mm256_andnot_si256(b19, b15));
        a19 = _mm256_xor_si256(b19, _mm256_andnot_si256(b15, b16));
        a20 = _mm256_xor_si256(b20, _mm256_andnot_si256(b21, b22));
        a21 = _mm256_xor_si256(b21, _mm256_andnot_si256(b22, b23));
        a22 = _mm256_xor_si256(b22, _mm256_andnot_si256(b23, b24));
        a23 = _mm256_xor_si256(b23, _mm256_andnot_si256(b24, b20));
        a24 = _mm256_xor_si256(b24, _mm256_andnot_si256(b20, b21));
        a0 = _mm256_xor_si256(a0, rc);
    }
    let c0 = _mm256_xor_si256(
        _mm256_xor_si256(a0, a5),
        _mm256_xor_si256(a10, _mm256_xor_si256(a15, a20)),
    );
    let c1 = _mm256_xor_si256(
        _mm256_xor_si256(a1, a6),
        _mm256_xor_si256(a11, _mm256_xor_si256(a16, a21)),
    );
    let c2 = _mm256_xor_si256(
        _mm256_xor_si256(a2, a7),
        _mm256_xor_si256(a12, _mm256_xor_si256(a17, a22)),
    );
    let c3 = _mm256_xor_si256(
        _mm256_xor_si256(a3, a8),
        _mm256_xor_si256(a13, _mm256_xor_si256(a18, a23)),
    );
    let c4 = _mm256_xor_si256(
        _mm256_xor_si256(a4, a9),
        _mm256_xor_si256(a14, _mm256_xor_si256(a19, a24)),
    );
    let d0 = _mm256_xor_si256(c4, rol!(c1, 1));
    let d1 = _mm256_xor_si256(c0, rol!(c2, 1));
    let d2 = _mm256_xor_si256(c1, rol!(c3, 1));
    let b1 = rol!(_mm256_xor_si256(a6, d1), 44);
    let b2 = rol!(_mm256_xor_si256(a12, d2), 43);
    let a0 = _mm256_xor_si256(
        _mm256_xor_si256(_mm256_xor_si256(a0, d0), _mm256_andnot_si256(b1, b2)),
        _mm256_set1_epi64x(RC[23] as i64),
    );
    let mut out = [0u64; 4];
    _mm256_storeu_si256(out.as_mut_ptr() as *mut __m256i, a0);
    out
}

/// # Safety
/// The CPU must support AVX2.
pub unsafe fn keccak256_selector_x4(inputs: [&[u8]; 4]) -> [Selector; 4] {
    let mut rate = [0u64; 17 * 4];
    for (lane, input) in inputs.iter().enumerate() {
        absorb(&mut rate, 4, lane, input);
    }
    let word0 = keccak256_selectors_x4(&rate);
    std::array::from_fn(|lane| selector(&word0, lane))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hasher::scalar::keccak256_selector;

    #[test]
    fn matches_scalar() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }
        let long = [b'z'; 135];
        let batches: [[&[u8]; 4]; 2] = [
            [
                b"transfer(address,uint256)",
                b"approve(address,uint256)",
                b"balanceOf(address)",
                b"totalSupply()",
            ],
            [b"", &long, b"a_Z9(uint256,address)", b"a()"],
        ];
        for inputs in batches {
            let got = unsafe { keccak256_selector_x4(inputs) };
            for (sel, input) in got.iter().zip(inputs) {
                assert_eq!(*sel, keccak256_selector(input));
            }
        }
    }
}
