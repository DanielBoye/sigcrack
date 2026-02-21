use super::Selector;

extern "C" {
    fn keccak_f1600_x8(states: *mut u64);
}

/// # Safety
/// Caller must ensure AVX-512F is available on the current CPU.
pub unsafe fn keccak256_selector_x8(inputs: [&[u8]; 8]) -> [Selector; 8] {
    let mut states = [0u64; 200]; // 25 lanes x 8 instances

    for (j, input) in inputs.iter().enumerate() {
        debug_assert!(input.len() < 136);
        let full_words = input.len() / 8;
        for i in 0..full_words {
            states[i * 8 + j] = u64::from_le_bytes([
                input[i*8], input[i*8+1], input[i*8+2], input[i*8+3],
                input[i*8+4], input[i*8+5], input[i*8+6], input[i*8+7],
            ]);
        }
        let remaining = input.len() % 8;
        if remaining > 0 {
            let mut last = [0u8; 8];
            last[..remaining].copy_from_slice(&input[full_words * 8..]);
            states[full_words * 8 + j] = u64::from_le_bytes(last);
        }
        let pad_word = input.len() / 8;
        let pad_byte = input.len() % 8;
        states[pad_word * 8 + j] ^= 0x01u64 << (pad_byte * 8);
        states[16 * 8 + j] ^= 0x80u64 << 56;
    }

    keccak_f1600_x8(states.as_mut_ptr());

    let mut results = [[0u8; 4]; 8];
    for j in 0..8 {
        let bytes = states[j].to_le_bytes();
        results[j] = [bytes[0], bytes[1], bytes[2], bytes[3]];
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avx512_matches_scalar() {
        if !is_x86_feature_detected!("avx512f") {
            eprintln!("Skipping: no AVX-512"); return;
        }
        let inputs: [&[u8]; 8] = [
            b"transfer(address,uint256)", b"approve(address,uint256)",
            b"balanceOf(address)", b"totalSupply()",
            b"transferFrom(address,address,uint256)", b"name()", b"symbol()", b"decimals()",
        ];
        let results = unsafe { keccak256_selector_x8(inputs) };
        for (i, input) in inputs.iter().enumerate() {
            let scalar = crate::hasher::scalar::keccak256_selector(input);
            assert_eq!(results[i], scalar, "AVX-512 mismatch at {}", i);
        }
    }
}
