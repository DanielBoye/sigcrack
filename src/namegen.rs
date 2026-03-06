use crate::cli::NameMode;

pub(crate) const FIRST_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_";
pub(crate) const REST_CHARS: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";
/// The GPU counter shader unrolls exactly this many positions.
pub(crate) const MAX_CHARS: usize = 16;
pub(crate) const MAX_WORDS: usize = 3;
/// Room for MAX_WORDS of the longest word; `word_list_is_lowercase_ascii`
/// checks the list fits.
pub(crate) const MAX_WORD_NAME_LEN: usize = 64;
const WORDS: &str = include_str!("words.txt");

pub fn words() -> Vec<&'static str> {
    WORDS.split_whitespace().collect()
}

/// Writes the words at `idx` into `out` as one camelCase identifier and
/// returns its length. The GPU reconstructs names the same way.
pub fn camel_case(words: &[&str], idx: &[usize], out: &mut [u8]) -> usize {
    let mut len = 0;
    for (i, &w) in idx.iter().enumerate() {
        let word = words[w].as_bytes();
        let dst = &mut out[len..len + word.len()];
        dst.copy_from_slice(word);
        if i > 0 {
            dst[0] = dst[0].to_ascii_uppercase();
        }
        len += word.len();
    }
    len
}

pub enum NameIter {
    Chars(CharNameIter),
    Words(WordNameIter),
}

impl NameIter {
    pub fn new(mode: NameMode, thread_id: usize, num_threads: usize) -> Self {
        match mode {
            NameMode::Random => NameIter::Chars(CharNameIter::new(thread_id, num_threads)),
            NameMode::Readable => NameIter::Words(WordNameIter::new(words())),
        }
    }

    #[inline(always)]
    pub fn current(&self) -> &[u8] {
        match self {
            NameIter::Chars(it) => it.current(),
            NameIter::Words(it) => it.current(),
        }
    }

    #[inline(always)]
    pub fn is_exhausted(&self) -> bool {
        match self {
            NameIter::Chars(it) => it.is_exhausted(),
            NameIter::Words(it) => it.is_exhausted(),
        }
    }

    pub fn reset(&mut self) {
        match self {
            NameIter::Chars(it) => it.reset(),
            NameIter::Words(it) => it.reset(),
        }
    }

    #[inline]
    pub fn advance(&mut self) -> bool {
        match self {
            NameIter::Chars(it) => it.advance(),
            NameIter::Words(it) => it.advance(),
        }
    }
}

/// Every identifier up to MAX_CHARS long, shortest first. Threads split the
/// space by first character, so each thread gets a disjoint slice of names.
pub struct CharNameIter {
    buf: [u8; MAX_CHARS],
    idx: [u8; MAX_CHARS],
    len: usize,
    first: std::ops::Range<usize>,
    exhausted: bool,
}

impl CharNameIter {
    pub fn new(thread_id: usize, num_threads: usize) -> Self {
        let per_thread = FIRST_CHARS.len().div_ceil(num_threads);
        let start = (thread_id * per_thread).min(FIRST_CHARS.len());
        let end = ((thread_id + 1) * per_thread).min(FIRST_CHARS.len());
        let mut it = Self {
            buf: [0; MAX_CHARS],
            idx: [0; MAX_CHARS],
            len: 1,
            first: start..end,
            exhausted: false,
        };
        it.reset();
        it
    }

    #[inline(always)]
    pub fn current(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    #[inline(always)]
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    pub fn reset(&mut self) {
        self.exhausted = self.first.is_empty();
        if !self.exhausted {
            self.start(1);
        }
    }

    fn start(&mut self, len: usize) {
        self.len = len;
        self.idx = [0; MAX_CHARS];
        self.idx[0] = self.first.start as u8;
        self.buf[0] = FIRST_CHARS[self.first.start];
        self.buf[1..len].fill(REST_CHARS[0]);
    }

    #[inline]
    pub fn advance(&mut self) -> bool {
        if self.exhausted {
            return false;
        }
        for pos in (1..self.len).rev() {
            self.idx[pos] += 1;
            if (self.idx[pos] as usize) < REST_CHARS.len() {
                self.buf[pos] = REST_CHARS[self.idx[pos] as usize];
                return true;
            }
            self.idx[pos] = 0;
            self.buf[pos] = REST_CHARS[0];
        }
        self.idx[0] += 1;
        if (self.idx[0] as usize) < self.first.end {
            self.buf[0] = FIRST_CHARS[self.idx[0] as usize];
            return true;
        }
        if self.len == MAX_CHARS {
            self.exhausted = true;
            return false;
        }
        self.start(self.len + 1);
        true
    }
}

/// camelCase names built from one to MAX_WORDS dictionary words.
pub struct WordNameIter {
    words: Vec<&'static str>,
    idx: [usize; MAX_WORDS],
    count: usize,
    buf: [u8; MAX_WORD_NAME_LEN],
    len: usize,
    exhausted: bool,
}

impl WordNameIter {
    pub fn new(words: Vec<&'static str>) -> Self {
        let mut it = Self {
            words,
            idx: [0; MAX_WORDS],
            count: 1,
            buf: [0; MAX_WORD_NAME_LEN],
            len: 0,
            exhausted: false,
        };
        it.reset();
        it
    }

    #[inline(always)]
    pub fn current(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    #[inline(always)]
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    pub fn reset(&mut self) {
        self.exhausted = self.words.is_empty();
        if !self.exhausted {
            self.idx = [0; MAX_WORDS];
            self.count = 1;
            self.build();
        }
    }

    fn build(&mut self) {
        self.len = camel_case(&self.words, &self.idx[..self.count], &mut self.buf);
    }

    #[inline]
    pub fn advance(&mut self) -> bool {
        if self.exhausted {
            return false;
        }
        for pos in (0..self.count).rev() {
            self.idx[pos] += 1;
            if self.idx[pos] < self.words.len() {
                self.build();
                return true;
            }
            self.idx[pos] = 0;
        }
        if self.count == MAX_WORDS {
            self.exhausted = true;
            return false;
        }
        self.count += 1;
        self.build();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(it: &mut NameIter) -> Vec<Vec<u8>> {
        let mut out = vec![it.current().to_vec()];
        while it.advance() {
            out.push(it.current().to_vec());
        }
        out
    }

    #[test]
    fn chars_count_by_length() {
        let mut it = CharNameIter::new(0, 1);
        let mut n = 1;
        while it.advance() && it.current().len() == 1 {
            n += 1;
        }
        assert_eq!(n, FIRST_CHARS.len());
        assert_eq!(it.current(), b"aa");
    }

    #[test]
    fn chars_threads_are_disjoint_and_complete() {
        let mut seen = std::collections::HashSet::new();
        for t in 0..4 {
            let mut it = CharNameIter::new(t, 4);
            while !it.is_exhausted() && it.current().len() == 1 {
                assert!(seen.insert(it.current().to_vec()));
                it.advance();
            }
        }
        assert_eq!(seen.len(), FIRST_CHARS.len());
    }

    #[test]
    fn chars_more_threads_than_first_chars() {
        let it = CharNameIter::new(60, 64);
        assert!(it.is_exhausted());
    }

    #[test]
    fn chars_are_valid_identifiers() {
        let mut it = CharNameIter::new(0, 1);
        for _ in 0..5000 {
            let name = it.current();
            assert!(name[0].is_ascii_alphabetic() || name[0] == b'_');
            assert!(name[1..]
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || *b == b'_'));
            it.advance();
        }
    }

    #[test]
    fn words_sequence() {
        let mut it = NameIter::Words(WordNameIter::new(vec!["get", "set"]));
        let names = collect(&mut it);
        assert_eq!(names.len(), 2 + 4 + 8);
        assert_eq!(
            &names[..6],
            &[
                &b"get"[..],
                b"set",
                b"getGet",
                b"getSet",
                b"setGet",
                b"setSet"
            ]
        );
        assert_eq!(names[6], b"getGetGet");
        assert!(it.is_exhausted());
        it.reset();
        assert_eq!(it.current(), b"get");
    }

    #[test]
    fn words_empty_list() {
        assert!(WordNameIter::new(vec![]).is_exhausted());
    }

    #[test]
    fn word_list_is_lowercase_ascii() {
        let words = words();
        assert!(words.len() > 50);
        assert!(words
            .iter()
            .all(|w| w.bytes().all(|b| b.is_ascii_lowercase())));
        assert!(MAX_WORDS * words.iter().map(|w| w.len()).max().unwrap() <= MAX_WORD_NAME_LEN);
        // The GPU word table holds 12 bytes per entry.
        assert!(words.iter().all(|w| w.len() <= 12));
    }
}
