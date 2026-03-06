use super::{absorb, selector, Selector};

extern "C" {
    fn keccak_f1600_x8(states: *mut u64);
}

/// # Safety
/// The CPU must support AVX-512F.
pub unsafe fn keccak256_selector_x8(inputs: [&[u8]; 8]) -> [Selector; 8] {
    let mut state = [0u64; 25 * 8];
    for (lane, input) in inputs.iter().enumerate() {
        absorb(&mut state, 8, lane, input);
    }
    keccak_f1600_x8(state.as_mut_ptr());
    std::array::from_fn(|lane| selector(&state, lane))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hasher::scalar::keccak256_selector;

    #[test]
    fn matches_scalar() {
        if !is_x86_feature_detected!("avx512f") {
            return;
        }
        let inputs: [&[u8]; 8] = [
            b"transfer(address,uint256)",
            b"approve(address,uint256)",
            b"balanceOf(address)",
            b"totalSupply()",
            b"transferFrom(address,address,uint256)",
            b"name()",
            b"symbol()",
            b"decimals()",
        ];
        let got = unsafe { keccak256_selector_x8(inputs) };
        for (sel, input) in got.iter().zip(inputs) {
            assert_eq!(*sel, keccak256_selector(input));
        }
    }
}
