const FIRST_CHARS: &[u8; 53] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_";
const REST_CHARS: &[u8; 63] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";
const MAX_NAME_LEN: usize = 16;

pub struct RandomNameIter {
    buf: [u8; MAX_NAME_LEN],
    indices: [u8; MAX_NAME_LEN],
    len: usize,
    first_char_start: usize,
    first_char_end: usize,
    exhausted: bool,
}

impl RandomNameIter {
    pub fn for_thread(thread_id: usize, num_threads: usize) -> Self {
        let total_first = FIRST_CHARS.len();
        let per_thread = (total_first + num_threads - 1) / num_threads;
        let start = thread_id * per_thread;
        let end = ((thread_id + 1) * per_thread).min(total_first);

        let mut iter = Self {
            buf: [0u8; MAX_NAME_LEN],
            indices: [0u8; MAX_NAME_LEN],
            len: 1,
            first_char_start: start,
            first_char_end: end,
            exhausted: start >= total_first,
        };
        if !iter.exhausted {
            iter.indices[0] = start as u8;
            iter.buf[0] = FIRST_CHARS[start];
        }
        iter
    }

    #[inline(always)]
    pub fn current(&self) -> &[u8] { &self.buf[..self.len] }

    #[inline(always)]
    pub fn len(&self) -> usize { self.len }

    #[inline(always)]
    pub fn is_exhausted(&self) -> bool { self.exhausted }

    pub fn reset(&mut self) {
        if self.first_char_start >= FIRST_CHARS.len() {
            self.exhausted = true;
            return;
        }
        self.len = 1;
        self.indices = [0u8; MAX_NAME_LEN];
        self.indices[0] = self.first_char_start as u8;
        self.buf[0] = FIRST_CHARS[self.first_char_start];
        self.exhausted = false;
    }

    #[inline]
    pub fn advance(&mut self) -> bool {
        if self.exhausted { return false; }
        let mut pos = self.len - 1;
        loop {
            self.indices[pos] += 1;
            if pos == 0 {
                if (self.indices[0] as usize) < self.first_char_end {
                    self.buf[0] = FIRST_CHARS[self.indices[0] as usize];
                    return true;
                }
                self.len += 1;
                if self.len > MAX_NAME_LEN {
                    self.exhausted = true;
                    return false;
                }
                self.indices = [0u8; MAX_NAME_LEN];
                self.indices[0] = self.first_char_start as u8;
                self.buf[0] = FIRST_CHARS[self.first_char_start];
                for i in 1..self.len {
                    self.indices[i] = 0;
                    self.buf[i] = REST_CHARS[0];
                }
                return true;
            } else {
                if (self.indices[pos] as usize) < REST_CHARS.len() {
                    self.buf[pos] = REST_CHARS[self.indices[pos] as usize];
                    return true;
                }
                self.indices[pos] = 0;
                self.buf[pos] = REST_CHARS[0];
                pos -= 1;
            }
        }
    }
}

pub struct ReadableNameIter {
    words: Vec<&'static str>,
    indices: [usize; 3],
    num_words: usize,
    buf: [u8; 128],
    buf_len: usize,
    exhausted: bool,
}

impl ReadableNameIter {
    pub fn new(words: Vec<&'static str>) -> Self {
        let mut iter = Self {
            words,
            indices: [0, 0, 0],
            num_words: 1,
            buf: [0u8; 128],
            buf_len: 0,
            exhausted: false,
        };
        if iter.words.is_empty() {
            iter.exhausted = true;
        } else {
            iter.build_current();
        }
        iter
    }

    fn build_current(&mut self) {
        let mut pos = 0;
        for i in 0..self.num_words {
            let word = self.words[self.indices[i]];
            let bytes = word.as_bytes();
            if i == 0 {
                self.buf[pos..pos + bytes.len()].copy_from_slice(bytes);
            } else {
                self.buf[pos] = bytes[0].to_ascii_uppercase();
                if bytes.len() > 1 {
                    self.buf[pos + 1..pos + bytes.len()].copy_from_slice(&bytes[1..]);
                }
            }
            pos += bytes.len();
        }
        self.buf_len = pos;
    }

    #[inline(always)]
    pub fn current(&self) -> &[u8] { &self.buf[..self.buf_len] }

    #[inline(always)]
    pub fn len(&self) -> usize { self.buf_len }

    #[inline(always)]
    pub fn is_exhausted(&self) -> bool { self.exhausted }

    pub fn reset(&mut self) {
        self.indices = [0, 0, 0];
        self.num_words = 1;
        self.exhausted = false;
        if self.words.is_empty() {
            self.exhausted = true;
        } else {
            self.build_current();
        }
    }

    #[inline]
    pub fn advance(&mut self) -> bool {
        if self.exhausted { return false; }

        let last = self.num_words - 1;
        self.indices[last] += 1;

        for i in (0..self.num_words).rev() {
            if self.indices[i] < self.words.len() {
                self.build_current();
                return true;
            }
            self.indices[i] = 0;
            if i == 0 {
                self.num_words += 1;
                if self.num_words > 3 {
                    self.exhausted = true;
                    return false;
                }
                self.indices = [0; 3];
                self.build_current();
                return true;
            }
            self.indices[i - 1] += 1;
        }
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_thread_starts_with_a() {
        let iter = RandomNameIter::for_thread(0, 1);
        assert_eq!(iter.current(), b"a");
    }

    #[test]
    fn test_first_advances() {
        let mut iter = RandomNameIter::for_thread(0, 1);
        assert_eq!(iter.current(), b"a");
        iter.advance();
        assert_eq!(iter.current(), b"b");
    }

    #[test]
    fn test_length_one_count() {
        let mut iter = RandomNameIter::for_thread(0, 1);
        let mut count = 1;
        while iter.advance() {
            if iter.len() > 1 { break; }
            count += 1;
        }
        assert_eq!(count, 53);
    }

    #[test]
    fn test_grows_to_length_2() {
        let mut iter = RandomNameIter::for_thread(0, 1);
        for _ in 0..53 { iter.advance(); }
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.current(), b"aa");
    }

    #[test]
    fn test_two_threads_no_overlap() {
        let mut names_t0 = Vec::new();
        let mut names_t1 = Vec::new();
        let mut iter0 = RandomNameIter::for_thread(0, 2);
        let mut iter1 = RandomNameIter::for_thread(1, 2);

        names_t0.push(iter0.current().to_vec());
        while iter0.advance() && iter0.len() == 1 { names_t0.push(iter0.current().to_vec()); }
        names_t1.push(iter1.current().to_vec());
        while iter1.advance() && iter1.len() == 1 { names_t1.push(iter1.current().to_vec()); }

        for n in &names_t0 { assert!(!names_t1.contains(n)); }
        assert_eq!(names_t0.len() + names_t1.len(), 53);
    }

    #[test]
    fn test_reset_restarts() {
        let mut iter = RandomNameIter::for_thread(0, 1);
        iter.advance(); iter.advance();
        iter.reset();
        assert_eq!(iter.current(), b"a");
        assert_eq!(iter.len(), 1);
    }

    #[test]
    fn test_valid_solidity_identifiers() {
        let mut iter = RandomNameIter::for_thread(0, 1);
        for _ in 0..1000 {
            let name = iter.current();
            assert!(name[0].is_ascii_alphabetic() || name[0] == b'_');
            for &b in &name[1..] { assert!(b.is_ascii_alphanumeric() || b == b'_'); }
            if !iter.advance() { break; }
        }
    }

    #[test]
    fn test_readable_first_name() {
        let words = vec!["get", "set", "add"];
        let iter = ReadableNameIter::new(words);
        assert_eq!(iter.current(), b"get");
    }

    #[test]
    fn test_readable_camel_case() {
        let words = vec!["get", "set"];
        let mut iter = ReadableNameIter::new(words);
        // Skip all 1-word names: get, set
        iter.advance(); // set
        iter.advance(); // should now be 2-word: getGet
        assert_eq!(iter.len(), 6); // "getGet"
        assert_eq!(iter.current()[0], b'g');
        assert_eq!(iter.current()[3], b'G'); // capitalized
        assert_eq!(iter.current(), b"getGet");
    }

    #[test]
    fn test_readable_two_word_sequence() {
        let words = vec!["get", "set"];
        let mut iter = ReadableNameIter::new(words);
        // 1-word: get, set
        assert_eq!(iter.current(), b"get");
        iter.advance();
        assert_eq!(iter.current(), b"set");
        // 2-word: getGet, getSet, setGet, setSet
        iter.advance();
        assert_eq!(iter.current(), b"getGet");
        iter.advance();
        assert_eq!(iter.current(), b"getSet");
        iter.advance();
        assert_eq!(iter.current(), b"setGet");
        iter.advance();
        assert_eq!(iter.current(), b"setSet");
    }

    #[test]
    fn test_readable_three_word() {
        let words = vec!["get", "set"];
        let mut iter = ReadableNameIter::new(words);
        // 1-word: 2 names, 2-word: 4 names = 6 total before 3-word
        for _ in 0..6 {
            iter.advance();
        }
        // Now at first 3-word: getGetGet
        assert_eq!(iter.current(), b"getGetGet");
    }

    #[test]
    fn test_readable_exhausts() {
        let words = vec!["a", "b"];
        let mut iter = ReadableNameIter::new(words);
        // 1-word: 2, 2-word: 4, 3-word: 8 = 14 total
        let mut count = 1;
        while iter.advance() {
            count += 1;
        }
        assert_eq!(count, 14);
        assert!(iter.is_exhausted());
    }

    #[test]
    fn test_readable_reset() {
        let words = vec!["get", "set"];
        let mut iter = ReadableNameIter::new(words);
        iter.advance();
        iter.advance();
        iter.reset();
        assert_eq!(iter.current(), b"get");
        assert!(!iter.is_exhausted());
    }

    #[test]
    fn test_readable_empty_words() {
        let words: Vec<&'static str> = vec![];
        let iter = ReadableNameIter::new(words);
        assert!(iter.is_exhausted());
    }
}
