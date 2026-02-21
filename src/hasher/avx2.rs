use super::Selector;

extern "C" {
    fn keccak_f1600_x4(states: *mut u64);
}

/// # Safety
/// Caller must ensure AVX2 is available on the current CPU.
#[target_feature(enable = "avx2")]
pub unsafe fn keccak256_selector_x4(inputs: [&[u8]; 4]) -> [Selector; 4] {
    let mut states = [0u64; 100]; // 25 lanes x 4 instances

    for (j, input) in inputs.iter().enumerate() {
        debug_assert!(input.len() < 136);
        let full_words = input.len() / 8;
        for i in 0..full_words {
            states[i * 4 + j] = u64::from_le_bytes([
                input[i*8], input[i*8+1], input[i*8+2], input[i*8+3],
                input[i*8+4], input[i*8+5], input[i*8+6], input[i*8+7],
            ]);
        }
        let remaining = input.len() % 8;
        if remaining > 0 {
            let mut last = [0u8; 8];
            last[..remaining].copy_from_slice(&input[full_words * 8..]);
            states[full_words * 4 + j] = u64::from_le_bytes(last);
        }
        let pad_word = input.len() / 8;
        let pad_byte = input.len() % 8;
        states[pad_word * 4 + j] ^= 0x01u64 << (pad_byte * 8);
        states[16 * 4 + j] ^= 0x80u64 << 56;
    }

    keccak_f1600_x4(states.as_mut_ptr());

    let mut results = [[0u8; 4]; 4];
    for j in 0..4 {
        let bytes = states[j].to_le_bytes();
        results[j] = [bytes[0], bytes[1], bytes[2], bytes[3]];
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avx2_matches_scalar() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("Skipping: no AVX2"); return;
        }
        let inputs: [&[u8]; 4] = [
            b"transfer(address,uint256)", b"approve(address,uint256)",
            b"balanceOf(address)", b"totalSupply()",
        ];
        let results = unsafe { keccak256_selector_x4(inputs) };
        for (i, input) in inputs.iter().enumerate() {
            let scalar = crate::hasher::scalar::keccak256_selector(input);
            assert_eq!(results[i], scalar, "AVX2 mismatch at {}", i);
        }
    }

    #[test]
    fn test_avx2_known_selectors() {
        if !is_x86_feature_detected!("avx2") { return; }
        let inputs: [&[u8]; 4] = [
            b"transfer(address,uint256)", b"approve(address,uint256)",
            b"balanceOf(address)", b"totalSupply()",
        ];
        let results = unsafe { keccak256_selector_x4(inputs) };
        assert_eq!(results[0], [0xa9, 0x05, 0x9c, 0xbb]);
        assert_eq!(results[1], [0x09, 0x5e, 0xa7, 0xb3]);
        assert_eq!(results[2], [0x70, 0xa0, 0x82, 0x31]);
        assert_eq!(results[3], [0x18, 0x16, 0x0d, 0xdd]);
    }
}
