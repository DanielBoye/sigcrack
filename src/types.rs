const COMMON_TYPES: &[&str] = &[
    "uint256", "uint128", "uint64", "uint32", "uint16", "uint8", "int256", "address", "bool",
    "bytes32", "bytes", "string",
];

fn base_types() -> Vec<String> {
    let mut types = Vec::new();
    types.extend((8..=256).step_by(8).map(|n| format!("uint{n}")));
    types.extend((8..=256).step_by(8).map(|n| format!("int{n}")));
    types.extend((1..=32).map(|n| format!("bytes{n}")));
    types.extend(["address", "bool", "bytes", "string"].map(String::from));
    types
}

/// Parameter lists appended to each name: "()", every single base type, then
/// every combination of COMMON_TYPES for arities 2..=max_params.
pub fn type_combos(max_params: usize) -> Vec<Vec<u8>> {
    let mut combos = vec![b"()".to_vec()];
    if max_params >= 1 {
        combos.extend(base_types().iter().map(|t| format!("({t})").into_bytes()));
    }
    for arity in 2..=max_params {
        let mut idx = vec![0usize; arity];
        loop {
            let list: Vec<&str> = idx.iter().map(|&i| COMMON_TYPES[i]).collect();
            combos.push(format!("({})", list.join(",")).into_bytes());
            match idx.iter().rposition(|&i| i + 1 < COMMON_TYPES.len()) {
                Some(pos) => {
                    idx[pos] += 1;
                    idx[pos + 1..].fill(0);
                }
                None => break,
            }
        }
    }
    combos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        assert_eq!(type_combos(0).len(), 1);
        assert_eq!(type_combos(1).len(), 1 + 100);
        assert_eq!(type_combos(2).len(), 1 + 100 + 144);
        assert_eq!(type_combos(3).len(), 1 + 100 + 144 + 1728);
    }

    #[test]
    fn contents() {
        let combos = type_combos(2);
        assert_eq!(combos[0], b"()");
        assert!(combos.contains(&b"(bytes7)".to_vec()));
        assert!(combos.contains(&b"(uint256,address)".to_vec()));
        assert!(combos.contains(&b"(string,string)".to_vec()));
        assert!(combos
            .iter()
            .all(|c| c.starts_with(b"(") && c.ends_with(b")")));
    }
}
