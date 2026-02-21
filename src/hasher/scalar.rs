use super::Selector;

#[inline]
pub fn keccak256_selector(input: &[u8]) -> Selector {
    debug_assert!(input.len() < 136, "input must fit in single keccak block");
    let mut state = [0u64; 25];

    let full_words = input.len() / 8;
    for i in 0..full_words {
        let word = u64::from_le_bytes([
            input[i*8], input[i*8+1], input[i*8+2], input[i*8+3],
            input[i*8+4], input[i*8+5], input[i*8+6], input[i*8+7],
        ]);
        state[i] ^= word;
    }

    let remaining = input.len() % 8;
    if remaining > 0 {
        let mut last_word = [0u8; 8];
        last_word[..remaining].copy_from_slice(&input[full_words * 8..]);
        state[full_words] ^= u64::from_le_bytes(last_word);
    }

    let pad_word_idx = input.len() / 8;
    let pad_byte_pos = input.len() % 8;
    state[pad_word_idx] ^= 0x01u64 << (pad_byte_pos * 8);
    state[16] ^= 0x80u64 << 56;

    keccak::f1600(&mut state);

    let out = state[0].to_le_bytes();
    [out[0], out[1], out[2], out[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_tiny_keccak() {
        use tiny_keccak::{Hasher, Keccak};
        let inputs: &[&[u8]] = &[
            b"transfer(address,uint256)", b"approve(address,uint256)",
            b"balanceOf(address)", b"totalSupply()",
            b"transferFrom(address,address,uint256)",
            b"hello()", b"a(uint8)",
            b"test_function_with_long_name(uint256,address,bool,bytes32)",
        ];
        for input in inputs {
            let mut hasher = Keccak::v256();
            hasher.update(input);
            let mut expected = [0u8; 32];
            hasher.finalize(&mut expected);
            let got = keccak256_selector(input);
            assert_eq!(got, [expected[0], expected[1], expected[2], expected[3]],
                "mismatch for: {}", std::str::from_utf8(input).unwrap());
        }
    }

    #[test]
    fn test_known_selectors() {
        assert_eq!(keccak256_selector(b"transfer(address,uint256)"), [0xa9, 0x05, 0x9c, 0xbb]);
        assert_eq!(keccak256_selector(b"approve(address,uint256)"), [0x09, 0x5e, 0xa7, 0xb3]);
        assert_eq!(keccak256_selector(b"balanceOf(address)"), [0x70, 0xa0, 0x82, 0x31]);
        assert_eq!(keccak256_selector(b"totalSupply()"), [0x18, 0x16, 0x0d, 0xdd]);
    }

    #[test]
    fn test_empty_parens() {
        let sel = keccak256_selector(b"x()");
        assert_eq!(sel.len(), 4);
    }

    #[test]
    fn test_max_length_input() {
        let input = vec![b'a'; 135];
        let sel = keccak256_selector(&input);
        assert_eq!(sel.len(), 4);
    }
}
