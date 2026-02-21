const WORDS: &str = include_str!("../words.txt");

pub fn load_words() -> Vec<&'static str> {
    WORDS.split_whitespace().filter(|w| !w.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_words_loaded() {
        let words = load_words();
        assert!(words.len() > 50);
        assert!(words.contains(&"transfer"));
        assert!(words.contains(&"balance"));
    }

    #[test]
    fn test_all_words_ascii_lowercase() {
        let words = load_words();
        for word in &words {
            assert!(word.chars().all(|c| c.is_ascii_lowercase()),
                "Word '{}' contains non-lowercase ASCII", word);
        }
    }

    #[test]
    fn test_no_empty_words() {
        let words = load_words();
        for word in &words {
            assert!(!word.is_empty());
        }
    }
}
