#[cfg(target_arch = "x86_64")]
pub mod avx2;
#[cfg(target_arch = "x86_64")]
pub mod avx512;
pub mod gpu;
pub mod scalar;

pub type Selector = [u8; 4];

/// Keccak-256 rate in bytes. Every input must fit in a single block.
pub const RATE: usize = 136;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdWidth {
    Scalar,
    Avx2,
    Avx512,
}

impl SimdWidth {
    pub fn lanes(self) -> usize {
        match self {
            SimdWidth::Scalar => 1,
            SimdWidth::Avx2 => 4,
            SimdWidth::Avx512 => 8,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SimdWidth::Scalar => "scalar",
            SimdWidth::Avx2 => "avx2",
            SimdWidth::Avx512 => "avx512",
        }
    }
}

/// XOR a padded single-block message into lane `lane` of a state that holds
/// `stride` interleaved instances: `state[i * stride + lane]` is word `i`.
#[inline(always)]
pub fn absorb(state: &mut [u64], stride: usize, lane: usize, input: &[u8]) {
    debug_assert!(input.len() < RATE);
    let mut words = input.chunks_exact(8);
    let mut i = 0;
    for word in &mut words {
        let mut w = [0u8; 8];
        w.copy_from_slice(word);
        state[i * stride + lane] ^= u64::from_le_bytes(w);
        i += 1;
    }
    let rest = words.remainder();
    let mut last = [0u8; 8];
    last[..rest.len()].copy_from_slice(rest);
    last[rest.len()] = 0x01;
    state[i * stride + lane] ^= u64::from_le_bytes(last);
    state[16 * stride + lane] ^= 0x80 << 56;
}

/// First four bytes of the hash held in lane `lane`.
#[inline(always)]
pub fn selector(state: &[u64], lane: usize) -> Selector {
    let b = state[lane].to_le_bytes();
    [b[0], b[1], b[2], b[3]]
}
