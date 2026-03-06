use super::{absorb, selector, Selector};

#[inline]
pub fn keccak256_selector(input: &[u8]) -> Selector {
    let mut state = [0u64; 25];
    absorb(&mut state, 1, 0, input);
    keccak::f1600(&mut state);
    selector(&state, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_keccak::{Hasher, Keccak};

    #[test]
    fn matches_reference() {
        let long = [b'a'; 135];
        let inputs: &[&[u8]] = &[
            b"",
            b"a()",
            b"transfer(address,uint256)",
            b"transferFrom(address,address,uint256)",
            b"x(uint256,uint256,uint256,uint256,uint256,uint256,uint256)",
            &long,
        ];
        for input in inputs {
            let mut h = Keccak::v256();
            h.update(input);
            let mut out = [0u8; 32];
            h.finalize(&mut out);
            assert_eq!(keccak256_selector(input)[..], out[..4]);
        }
    }

    #[test]
    fn known_selectors() {
        assert_eq!(
            keccak256_selector(b"transfer(address,uint256)"),
            [0xa9, 0x05, 0x9c, 0xbb]
        );
        assert_eq!(
            keccak256_selector(b"balanceOf(address)"),
            [0x70, 0xa0, 0x82, 0x31]
        );
        assert_eq!(
            keccak256_selector(b"totalSupply()"),
            [0x18, 0x16, 0x0d, 0xdd]
        );
    }
}
