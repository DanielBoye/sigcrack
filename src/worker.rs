use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::Sender;

use crate::cli::NameMode;
use crate::hasher::scalar;
use crate::hasher::SimdWidth;
use crate::namegen::{RandomNameIter, ReadableNameIter};
use crate::wordlist::load_words;

#[derive(Debug, Clone)]
pub struct Match {
    pub signature: String,
    pub selector: [u8; 4],
}

pub struct SharedState {
    pub total_hashes: AtomicU64,
    pub found_count: AtomicU32,
    pub stop: AtomicBool,
    pub current_name_len: AtomicU64,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            total_hashes: AtomicU64::new(0),
            found_count: AtomicU32::new(0),
            stop: AtomicBool::new(false),
            current_name_len: AtomicU64::new(1),
        }
    }
}

enum NameIter {
    Random(RandomNameIter),
    Readable(ReadableNameIter),
}

impl NameIter {
    #[inline(always)]
    fn current(&self) -> &[u8] {
        match self {
            NameIter::Random(r) => r.current(),
            NameIter::Readable(r) => r.current(),
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        match self {
            NameIter::Random(r) => r.len(),
            NameIter::Readable(r) => r.len(),
        }
    }

    #[inline(always)]
    fn is_exhausted(&self) -> bool {
        match self {
            NameIter::Random(r) => r.is_exhausted(),
            NameIter::Readable(r) => r.is_exhausted(),
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        match self {
            NameIter::Random(r) => r.reset(),
            NameIter::Readable(r) => r.reset(),
        }
    }

    #[inline]
    fn advance(&mut self) -> bool {
        match self {
            NameIter::Random(r) => r.advance(),
            NameIter::Readable(r) => r.advance(),
        }
    }
}

fn make_name_iter(mode: &NameMode, thread_id: usize, num_threads: usize) -> NameIter {
    match mode {
        NameMode::Random => NameIter::Random(RandomNameIter::for_thread(thread_id, num_threads)),
        NameMode::Readable => NameIter::Readable(ReadableNameIter::new(load_words())),
    }
}

fn thread_type_combos(
    mode: &NameMode,
    thread_id: usize,
    num_threads: usize,
    all_combos: &[Vec<u8>],
) -> Vec<Vec<u8>> {
    match mode {
        NameMode::Random => all_combos.to_vec(),
        NameMode::Readable => {
            all_combos
                .iter()
                .enumerate()
                .filter(|(i, _)| i % num_threads == thread_id)
                .map(|(_, c)| c.clone())
                .collect()
        }
    }
}

pub fn spawn_workers(
    num_threads: usize,
    target: [u8; 4],
    max_count: u32,
    type_combos: Arc<Vec<Vec<u8>>>,
    state: Arc<SharedState>,
    tx: Sender<Match>,
    simd_width: SimdWidth,
    name_mode: NameMode,
) -> Vec<thread::JoinHandle<()>> {
    let mut handles = Vec::with_capacity(num_threads);
    for thread_id in 0..num_threads {
        let type_combos = Arc::clone(&type_combos);
        let state = Arc::clone(&state);
        let tx = tx.clone();
        let mode = name_mode.clone();
        let handle = thread::spawn(move || {
            let combos = thread_type_combos(&mode, thread_id, num_threads, &type_combos);
            match simd_width {
                #[cfg(target_arch = "x86_64")]
                SimdWidth::Avx2 => {
                    if is_x86_feature_detected!("avx2") {
                        avx2_worker_loop(&mode, thread_id, num_threads, target, max_count, &combos, &state, &tx);
                    } else {
                        scalar_worker_loop(&mode, thread_id, num_threads, target, max_count, &combos, &state, &tx);
                    }
                }
                #[cfg(target_arch = "x86_64")]
                SimdWidth::Avx512 => {
                    if is_x86_feature_detected!("avx512f") {
                        avx512_worker_loop(&mode, thread_id, num_threads, target, max_count, &combos, &state, &tx);
                    } else if is_x86_feature_detected!("avx2") {
                        avx2_worker_loop(&mode, thread_id, num_threads, target, max_count, &combos, &state, &tx);
                    } else {
                        scalar_worker_loop(&mode, thread_id, num_threads, target, max_count, &combos, &state, &tx);
                    }
                }
                _ => {
                    scalar_worker_loop(&mode, thread_id, num_threads, target, max_count, &combos, &state, &tx);
                }
            }
        });
        handles.push(handle);
    }
    handles
}

fn scalar_worker_loop(
    mode: &NameMode,
    thread_id: usize,
    num_threads: usize,
    target: [u8; 4],
    max_count: u32,
    type_combos: &[Vec<u8>],
    state: &SharedState,
    tx: &Sender<Match>,
) {
    let mut name_iter = make_name_iter(mode, thread_id, num_threads);
    let mut buf = [0u8; 256];
    let mut local_hash_count: u64 = 0;
    let batch_report_interval: u64 = 65536;

    for type_combo in type_combos {
        let suffix = type_combo.as_slice();
        let suffix_len = suffix.len();
        name_iter.reset();
        if name_iter.is_exhausted() { continue; }

        let mut prev_name_len: usize = 0;

        loop {
            if local_hash_count & (batch_report_interval - 1) == 0 {
                state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
                local_hash_count = 0;
                if state.stop.load(Ordering::Relaxed)
                    || state.found_count.load(Ordering::Relaxed) >= max_count
                { return; }
                state.current_name_len.store(name_iter.len() as u64, Ordering::Relaxed);
            }

            let name = name_iter.current();
            let name_len = name.len();
            let total_len = name_len + suffix_len;

            buf[..name_len].copy_from_slice(name);
            if name_len != prev_name_len {
                buf[name_len..total_len].copy_from_slice(suffix);
                prev_name_len = name_len;
            }

            let selector = scalar::keccak256_selector(&buf[..total_len]);
            local_hash_count += 1;

            if selector == target {
                let sig = String::from_utf8_lossy(&buf[..total_len]).to_string();
                let _ = tx.send(Match { signature: sig, selector });
                state.found_count.fetch_add(1, Ordering::Relaxed);
                if state.found_count.load(Ordering::Relaxed) >= max_count {
                    state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
                    return;
                }
            }

            if !name_iter.advance() { break; }
        }
    }
    state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
}

#[cfg(target_arch = "x86_64")]
fn avx2_worker_loop(
    mode: &NameMode,
    thread_id: usize,
    num_threads: usize,
    target: [u8; 4],
    max_count: u32,
    type_combos: &[Vec<u8>],
    state: &SharedState,
    tx: &Sender<Match>,
) {
    use crate::hasher::avx2::keccak256_selector_x4;

    let mut name_iter = make_name_iter(mode, thread_id, num_threads);
    let mut bufs = [[0u8; 256]; 4];
    let mut lens = [0usize; 4];
    let mut local_hash_count: u64 = 0;
    let batch_report_interval: u64 = 65536;

    for type_combo in type_combos {
        let suffix = type_combo.as_slice();
        let suffix_len = suffix.len();
        name_iter.reset();
        if name_iter.is_exhausted() { continue; }

        let mut prev_name_lens = [0usize; 4];

        loop {
            if local_hash_count & (batch_report_interval - 1) == 0 {
                state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
                local_hash_count = 0;
                if state.stop.load(Ordering::Relaxed)
                    || state.found_count.load(Ordering::Relaxed) >= max_count
                { return; }
                state.current_name_len.store(name_iter.len() as u64, Ordering::Relaxed);
            }

            let mut batch_count = 0;
            for slot in 0..4 {
                if name_iter.is_exhausted() { break; }
                let name = name_iter.current();
                let name_len = name.len();
                let total_len = name_len + suffix_len;

                bufs[slot][..name_len].copy_from_slice(name);
                if name_len != prev_name_lens[slot] {
                    bufs[slot][name_len..total_len].copy_from_slice(suffix);
                    prev_name_lens[slot] = name_len;
                }
                lens[slot] = total_len;
                batch_count += 1;

                if slot < 3 {
                    if !name_iter.advance() {
                        batch_count = slot + 1;
                        break;
                    }
                }
            }

            if batch_count == 0 { break; }

            if batch_count < 4 {
                let mut tmp = [0u8; 256];
                let tmp_len = lens[0];
                tmp[..tmp_len].copy_from_slice(&bufs[0][..tmp_len]);
                for slot in batch_count..4 {
                    bufs[slot][..tmp_len].copy_from_slice(&tmp[..tmp_len]);
                    lens[slot] = tmp_len;
                }
            }

            let inputs: [&[u8]; 4] = [
                &bufs[0][..lens[0]],
                &bufs[1][..lens[1]],
                &bufs[2][..lens[2]],
                &bufs[3][..lens[3]],
            ];

            let results = unsafe { keccak256_selector_x4(inputs) };
            local_hash_count += batch_count as u64;

            for slot in 0..batch_count {
                if results[slot] == target {
                    let sig = String::from_utf8_lossy(&bufs[slot][..lens[slot]]).to_string();
                    let _ = tx.send(Match { signature: sig, selector: results[slot] });
                    state.found_count.fetch_add(1, Ordering::Relaxed);
                    if state.found_count.load(Ordering::Relaxed) >= max_count {
                        state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
                        return;
                    }
                }
            }

            if !name_iter.advance() { break; }
        }
    }
    state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
}

#[cfg(target_arch = "x86_64")]
fn avx512_worker_loop(
    mode: &NameMode,
    thread_id: usize,
    num_threads: usize,
    target: [u8; 4],
    max_count: u32,
    type_combos: &[Vec<u8>],
    state: &SharedState,
    tx: &Sender<Match>,
) {
    use crate::hasher::avx512::keccak256_selector_x8;

    let mut name_iter = make_name_iter(mode, thread_id, num_threads);
    let mut bufs = [[0u8; 256]; 8];
    let mut lens = [0usize; 8];
    let mut local_hash_count: u64 = 0;
    let batch_report_interval: u64 = 65536;

    for type_combo in type_combos {
        let suffix = type_combo.as_slice();
        let suffix_len = suffix.len();
        name_iter.reset();
        if name_iter.is_exhausted() { continue; }

        let mut prev_name_lens = [0usize; 8];

        loop {
            if local_hash_count & (batch_report_interval - 1) == 0 {
                state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
                local_hash_count = 0;
                if state.stop.load(Ordering::Relaxed)
                    || state.found_count.load(Ordering::Relaxed) >= max_count
                { return; }
                state.current_name_len.store(name_iter.len() as u64, Ordering::Relaxed);
            }

            let mut batch_count = 0;
            for slot in 0..8 {
                if name_iter.is_exhausted() { break; }
                let name = name_iter.current();
                let name_len = name.len();
                let total_len = name_len + suffix_len;

                bufs[slot][..name_len].copy_from_slice(name);
                if name_len != prev_name_lens[slot] {
                    bufs[slot][name_len..total_len].copy_from_slice(suffix);
                    prev_name_lens[slot] = name_len;
                }
                lens[slot] = total_len;
                batch_count += 1;

                if slot < 7 {
                    if !name_iter.advance() {
                        batch_count = slot + 1;
                        break;
                    }
                }
            }

            if batch_count == 0 { break; }

            if batch_count < 8 {
                let mut tmp = [0u8; 256];
                let tmp_len = lens[0];
                tmp[..tmp_len].copy_from_slice(&bufs[0][..tmp_len]);
                for slot in batch_count..8 {
                    bufs[slot][..tmp_len].copy_from_slice(&tmp[..tmp_len]);
                    lens[slot] = tmp_len;
                }
            }

            let inputs: [&[u8]; 8] = [
                &bufs[0][..lens[0]],
                &bufs[1][..lens[1]],
                &bufs[2][..lens[2]],
                &bufs[3][..lens[3]],
                &bufs[4][..lens[4]],
                &bufs[5][..lens[5]],
                &bufs[6][..lens[6]],
                &bufs[7][..lens[7]],
            ];

            let results = unsafe { keccak256_selector_x8(inputs) };
            local_hash_count += batch_count as u64;

            for slot in 0..batch_count {
                if results[slot] == target {
                    let sig = String::from_utf8_lossy(&bufs[slot][..lens[slot]]).to_string();
                    let _ = tx.send(Match { signature: sig, selector: results[slot] });
                    state.found_count.fetch_add(1, Ordering::Relaxed);
                    if state.found_count.load(Ordering::Relaxed) >= max_count {
                        state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
                        return;
                    }
                }
            }

            if !name_iter.advance() { break; }
        }
    }
    state.total_hashes.fetch_add(local_hash_count, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn test_finds_known_collision() {
        let target_sel = scalar::keccak256_selector(b"a()");
        let type_combos = Arc::new(vec![b"()".to_vec()]);
        let state = Arc::new(SharedState::new());
        let (tx, rx) = unbounded();
        let handles = spawn_workers(1, target_sel, 1, type_combos, Arc::clone(&state), tx, SimdWidth::Scalar, NameMode::Random);
        let result = rx.recv_timeout(std::time::Duration::from_secs(5));
        state.stop.store(true, Ordering::Relaxed);
        for h in handles { h.join().unwrap(); }
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.selector, target_sel);
        assert!(m.signature.ends_with("()"));
    }

    #[test]
    fn test_multi_thread_finds_result() {
        let target_sel = scalar::keccak256_selector(b"z()");
        let type_combos = Arc::new(vec![b"()".to_vec()]);
        let state = Arc::new(SharedState::new());
        let (tx, rx) = unbounded();
        let handles = spawn_workers(4, target_sel, 1, type_combos, Arc::clone(&state), tx, SimdWidth::Scalar, NameMode::Random);
        let result = rx.recv_timeout(std::time::Duration::from_secs(10));
        state.stop.store(true, Ordering::Relaxed);
        for h in handles { h.join().unwrap(); }
        assert!(result.is_ok());
        assert_eq!(result.unwrap().selector, target_sel);
    }

    #[test]
    fn test_respects_stop_signal() {
        let target_sel = [0xff, 0xff, 0xff, 0xff];
        let type_combos = Arc::new(vec![b"()".to_vec()]);
        let state = Arc::new(SharedState::new());
        let (tx, _rx) = unbounded();
        let handles = spawn_workers(2, target_sel, 1, type_combos, Arc::clone(&state), tx, SimdWidth::Scalar, NameMode::Random);
        thread::sleep(std::time::Duration::from_millis(100));
        state.stop.store(true, Ordering::Relaxed);
        for h in handles { h.join().unwrap(); }
        assert!(state.total_hashes.load(Ordering::Relaxed) > 0);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_avx2_worker_finds_collision() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("Skipping: no AVX2"); return;
        }
        let target_sel = scalar::keccak256_selector(b"a()");
        let type_combos = Arc::new(vec![b"()".to_vec()]);
        let state = Arc::new(SharedState::new());
        let (tx, rx) = unbounded();
        let handles = spawn_workers(1, target_sel, 1, type_combos, Arc::clone(&state), tx, SimdWidth::Avx2, NameMode::Random);
        let result = rx.recv_timeout(std::time::Duration::from_secs(5));
        state.stop.store(true, Ordering::Relaxed);
        for h in handles { h.join().unwrap(); }
        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.selector, target_sel);
        assert!(m.signature.ends_with("()"));
    }

    #[test]
    fn test_readable_mode_finds_collision() {
        // Target = selector of "get()" — "get" is the first word in the word list
        // so it should be found almost immediately even in debug mode.
        let target_sel = scalar::keccak256_selector(b"get()");
        let type_combos = Arc::new(vec![b"()".to_vec()]);
        let state = Arc::new(SharedState::new());
        let (tx, rx) = unbounded();
        let handles = spawn_workers(1, target_sel, 1, type_combos, Arc::clone(&state), tx, SimdWidth::Scalar, NameMode::Readable);
        let result = rx.recv_timeout(std::time::Duration::from_secs(10));
        state.stop.store(true, Ordering::Relaxed);
        for h in handles { h.join().unwrap(); }
        assert!(result.is_ok(), "Readable mode should find a collision for get()");
        let m = result.unwrap();
        assert_eq!(m.selector, target_sel);
        // The name should contain only ASCII letters (readable words)
        let name_part: &str = m.signature.split('(').next().unwrap();
        assert!(name_part.chars().all(|c| c.is_ascii_alphabetic()),
            "Readable name should be alphabetic, got: {}", name_part);
    }

    #[test]
    fn test_readable_mode_multi_thread_partitions_combos() {
        // Verify that readable mode partitions type_combos across threads.
        // "deposit(uint256)" — "deposit" is in the word list.
        let target_sel = scalar::keccak256_selector(b"deposit(uint256)");
        let type_combos = Arc::new(vec![
            b"()".to_vec(),
            b"(uint256)".to_vec(),
        ]);
        let state = Arc::new(SharedState::new());
        let (tx, rx) = unbounded();
        let handles = spawn_workers(2, target_sel, 1, type_combos, Arc::clone(&state), tx, SimdWidth::Scalar, NameMode::Readable);
        let result = rx.recv_timeout(std::time::Duration::from_secs(10));
        state.stop.store(true, Ordering::Relaxed);
        for h in handles { h.join().unwrap(); }
        assert!(result.is_ok(), "Readable mode with 2 threads should find deposit(uint256)");
        let m = result.unwrap();
        assert_eq!(m.selector, target_sel);
        assert!(m.signature.starts_with("deposit("));
    }
}
