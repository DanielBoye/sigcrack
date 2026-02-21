const COMMON_TYPES: &[&str] = &[
    "uint256", "uint128", "uint64", "uint32", "uint16", "uint8",
    "int256", "address", "bool", "bytes32", "bytes", "string",
];

pub fn all_base_types() -> Vec<String> {
    let mut types = Vec::new();
    for n in (8..=256).step_by(8) { types.push(format!("uint{}", n)); }
    for n in (8..=256).step_by(8) { types.push(format!("int{}", n)); }
    for n in 1..=32 { types.push(format!("bytes{}", n)); }
    types.push("address".to_string());
    types.push("bool".to_string());
    types.push("bytes".to_string());
    types.push("string".to_string());
    types
}

pub fn generate_type_combos(max_params: usize) -> Vec<Vec<u8>> {
    let mut combos = Vec::new();
    combos.push(b"()".to_vec());
    if max_params == 0 { return combos; }

    let all_types = all_base_types();
    for t in &all_types {
        combos.push(format!("({})", t).into_bytes());
    }

    if max_params >= 2 {
        let common: Vec<&str> = COMMON_TYPES.to_vec();
        generate_combos_recursive(&common, max_params, 2, Vec::new(), &mut combos);
    }
    combos
}

fn generate_combos_recursive(
    types: &[&str], max_arity: usize, current_arity: usize,
    current: Vec<&str>, result: &mut Vec<Vec<u8>>,
) {
    if current.len() == current_arity {
        let inner = current.join(",");
        result.push(format!("({})", inner).into_bytes());
        return;
    }
    for t in types {
        let mut next = current.clone();
        next.push(t);
        generate_combos_recursive(types, max_arity, current_arity, next, result);
    }
    if current.is_empty() && current_arity < max_arity {
        generate_combos_recursive(types, max_arity, current_arity + 1, Vec::new(), result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_base_types_count() {
        let types = all_base_types();
        assert_eq!(types.len(), 100);
    }

    #[test]
    fn test_arity_zero() {
        let combos = generate_type_combos(0);
        assert_eq!(combos.len(), 1);
        assert_eq!(&combos[0], b"()");
    }

    #[test]
    fn test_arity_one_includes_all_base() {
        let combos = generate_type_combos(1);
        assert_eq!(combos.len(), 101);
        assert!(combos.iter().any(|c| c == b"(uint256)"));
        assert!(combos.iter().any(|c| c == b"(address)"));
    }

    #[test]
    fn test_arity_two_uses_common_types() {
        let combos = generate_type_combos(2);
        assert_eq!(combos.len(), 245);
        assert!(combos.iter().any(|c| c == b"(uint256,address)"));
    }

    #[test]
    fn test_combo_format() {
        let combos = generate_type_combos(1);
        for combo in &combos {
            assert_eq!(combo[0], b'(');
            assert_eq!(*combo.last().unwrap(), b')');
        }
    }

    #[test]
    fn test_max_params_4_manageable_count() {
        let combos = generate_type_combos(4);
        assert!(combos.len() < 25_000);
        assert!(combos.len() > 200);
    }
}
