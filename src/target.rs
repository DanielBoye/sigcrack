use crate::hasher::scalar::keccak256_selector;
use crate::hasher::Selector;

pub struct Target {
    pub selector: Selector,
    pub display: String,
}

impl Target {
    /// Accepts a 4-byte hex selector ("0xa9059cbb") or a signature to hash.
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        let Some(hex) = input
            .strip_prefix("0x")
            .or_else(|| input.strip_prefix("0X"))
        else {
            return Ok(Self {
                selector: keccak256_selector(input.as_bytes()),
                display: input.to_string(),
            });
        };
        if hex.len() != 8 || !hex.is_ascii() {
            return Err(format!("expected 8 hex digits after 0x, got {input:?}"));
        }
        let mut selector = [0u8; 4];
        for (i, byte) in selector.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)
                .map_err(|e| format!("bad selector {input:?}: {e}"))?;
        }
        Ok(Self {
            selector,
            display: format!("0x{}", hex.to_lowercase()),
        })
    }
}

pub fn selector_hex(sel: &Selector) -> String {
    format!("0x{:02x}{:02x}{:02x}{:02x}", sel[0], sel[1], sel[2], sel[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex() {
        let t = Target::parse(" 0XA9059CBB ").unwrap();
        assert_eq!(t.selector, [0xa9, 0x05, 0x9c, 0xbb]);
        assert_eq!(t.display, "0xa9059cbb");
    }

    #[test]
    fn parse_signature() {
        let t = Target::parse("transfer(address,uint256)").unwrap();
        assert_eq!(selector_hex(&t.selector), "0xa9059cbb");
    }

    #[test]
    fn reject_bad_hex() {
        assert!(Target::parse("0xaabb").is_err());
        assert!(Target::parse("0xzzzzzzzz").is_err());
        assert!(Target::parse("0xa9059cbbé").is_err());
    }
}
