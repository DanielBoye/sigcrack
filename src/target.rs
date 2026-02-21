use tiny_keccak::{Hasher, Keccak};

pub struct Target {
    pub selector: [u8; 4],
    pub display: String,
}

impl Target {
    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
            Self::from_hex(trimmed)
        } else {
            Ok(Self::from_signature(trimmed))
        }
    }

    fn from_hex(hex: &str) -> Result<Self, String> {
        let hex = hex.strip_prefix("0x").or_else(|| hex.strip_prefix("0X")).unwrap_or(hex);
        if hex.len() != 8 {
            return Err(format!("Hex selector must be exactly 4 bytes (8 hex chars), got {}", hex.len()));
        }
        let bytes = (0..4)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Invalid hex: {}", e))?;
        Ok(Self {
            selector: [bytes[0], bytes[1], bytes[2], bytes[3]],
            display: format!("0x{}", hex.to_lowercase()),
        })
    }

    fn from_signature(sig: &str) -> Self {
        let selector = keccak256_selector(sig.as_bytes());
        Self { selector, display: sig.to_string() }
    }
}

pub fn keccak256_selector(input: &[u8]) -> [u8; 4] {
    let mut hasher = Keccak::v256();
    hasher.update(input);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);
    [output[0], output[1], output[2], output[3]]
}

pub fn selector_hex(sel: &[u8; 4]) -> String {
    format!("0x{:02x}{:02x}{:02x}{:02x}", sel[0], sel[1], sel[2], sel[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_erc20_selectors() {
        assert_eq!(keccak256_selector(b"transfer(address,uint256)"), [0xa9, 0x05, 0x9c, 0xbb]);
        assert_eq!(keccak256_selector(b"approve(address,uint256)"), [0x09, 0x5e, 0xa7, 0xb3]);
        assert_eq!(keccak256_selector(b"balanceOf(address)"), [0x70, 0xa0, 0x82, 0x31]);
        assert_eq!(keccak256_selector(b"totalSupply()"), [0x18, 0x16, 0x0d, 0xdd]);
        assert_eq!(keccak256_selector(b"transferFrom(address,address,uint256)"), [0x23, 0xb8, 0x72, 0xdd]);
    }

    #[test]
    fn test_parse_hex_selector() {
        let target = Target::parse("0xa9059cbb").unwrap();
        assert_eq!(target.selector, [0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn test_parse_signature() {
        let target = Target::parse("transfer(address,uint256)").unwrap();
        assert_eq!(target.selector, [0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn test_parse_hex_invalid_length() {
        assert!(Target::parse("0xaabb").is_err());
    }

    #[test]
    fn test_selector_hex_format() {
        assert_eq!(selector_hex(&[0xa9, 0x05, 0x9c, 0xbb]), "0xa9059cbb");
    }
}
