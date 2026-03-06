use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::cli::NameMode;
use crate::hasher::gpu::{CharWork, GpuHasher, Pending, WordWork, Work, DISPATCH, SLOTS};
use crate::hasher::{scalar, Selector, SimdWidth, RATE};
use crate::namegen::{words, NameIter, MAX_CHARS};

/// Hashes between updates of the shared counters.
const REPORT_INTERVAL: u64 = 1 << 16;

pub struct Match {
    pub signature: String,
    pub selector: Selector,
}

#[derive(Default)]
pub struct SharedState {
    pub total_hashes: AtomicU64,
    pub found: AtomicU32,
    pub stop: AtomicBool,
    pub name_len: AtomicUsize,
}

#[derive(Clone)]
pub struct Job {
    pub target: Selector,
    pub max_count: u32,
    pub mode: NameMode,
    pub type_combos: Arc<Vec<Vec<u8>>>,
    pub state: Arc<SharedState>,
    pub tx: Sender<Match>,
}

impl Job {
    fn should_stop(&self) -> bool {
        self.state.stop.load(Relaxed) || self.state.found.load(Relaxed) >= self.max_count
    }

    /// Sends a match. Returns false once the requested count is reached.
    fn report(&self, signature: &[u8], selector: Selector) -> bool {
        let signature = String::from_utf8_lossy(signature).into_owned();
        let _ = self.tx.send(Match {
            signature,
            selector,
        });
        self.state.found.fetch_add(1, Relaxed) + 1 < self.max_count
    }

    /// Random names are split across threads by first character, so every
    /// thread takes all type combos. Readable names can't be split, so the
    /// combos are dealt out instead.
    fn combos_for(&self, thread_id: usize, num_threads: usize) -> Vec<Vec<u8>> {
        match self.mode {
            NameMode::Random => self.type_combos.to_vec(),
            NameMode::Readable => self
                .type_combos
                .iter()
                .skip(thread_id)
                .step_by(num_threads)
                .cloned()
                .collect(),
        }
    }
}

/// The caller must have checked that the CPU supports `simd`.
pub fn spawn_cpu_workers(job: Job, num_threads: usize, simd: SimdWidth) -> Vec<JoinHandle<()>> {
    (0..num_threads)
        .map(|thread_id| {
            let job = job.clone();
            thread::spawn(move || {
                let combos = job.combos_for(thread_id, num_threads);
                let mut names = NameIter::new(job.mode, thread_id, num_threads);
                match simd {
                    #[cfg(target_arch = "x86_64")]
                    SimdWidth::Avx512 => {
                        cpu_loop::<8>(&job, &combos, &mut names, |inputs| unsafe {
                            crate::hasher::avx512::keccak256_selector_x8(inputs)
                        })
                    }
                    #[cfg(target_arch = "x86_64")]
                    SimdWidth::Avx2 => cpu_loop::<4>(&job, &combos, &mut names, |inputs| unsafe {
                        crate::hasher::avx2::keccak256_selector_x4(inputs)
                    }),
                    _ => cpu_loop::<1>(&job, &combos, &mut names, |[input]| {
                        [scalar::keccak256_selector(input)]
                    }),
                }
            })
        })
        .collect()
}

fn cpu_loop<const N: usize>(
    job: &Job,
    combos: &[Vec<u8>],
    names: &mut NameIter,
    hash: impl Fn([&[u8]; N]) -> [Selector; N],
) {
    let mut bufs = [[0u8; RATE]; N];
    let mut lens = [0usize; N];
    let mut local: u64 = 0;

    for suffix in combos {
        names.reset();
        while !names.is_exhausted() {
            if local >= REPORT_INTERVAL {
                job.state.total_hashes.fetch_add(local, Relaxed);
                local = 0;
                if job.should_stop() {
                    return;
                }
                job.state.name_len.store(names.current().len(), Relaxed);
            }

            let mut n = 0;
            while n < N {
                let name = names.current();
                let len = name.len() + suffix.len();
                bufs[n][..name.len()].copy_from_slice(name);
                bufs[n][name.len()..len].copy_from_slice(suffix);
                lens[n] = len;
                n += 1;
                if !names.advance() {
                    break;
                }
            }
            // Unused SIMD lanes get a copy of lane 0 so they hash valid input.
            for i in n..N {
                bufs[i] = bufs[0];
                lens[i] = lens[0];
            }

            let selectors = hash(std::array::from_fn(|i| &bufs[i][..lens[i]]));
            local += n as u64;

            for i in 0..n {
                if selectors[i] == job.target && !job.report(&bufs[i][..lens[i]], selectors[i]) {
                    job.state.total_hashes.fetch_add(local, Relaxed);
                    return;
                }
            }
        }
    }
    job.state.total_hashes.fetch_add(local, Relaxed);
}

pub fn spawn_gpu_worker(job: Job, hasher: GpuHasher) -> JoinHandle<()> {
    thread::spawn(move || {
        if let Err(e) = gpu_loop(&job, &hasher) {
            eprintln!("gpu worker failed: {e}");
            job.state.stop.store(true, Relaxed);
        }
    })
}

/// Names are generated on the GPU; only matching indices come back. Two
/// dispatches stay in flight so the readback of one overlaps the other's
/// compute.
fn gpu_loop(job: &Job, hasher: &GpuHasher) -> Result<(), String> {
    let mut pending: [Option<Pending>; SLOTS] = Default::default();
    search(job, hasher, &mut pending)?;
    // An early exit leaves dispatches in flight; the GPU hashes them anyway,
    // so they belong in the total.
    for (slot, p) in pending.iter().enumerate() {
        if let Some(p) = p {
            complete(job, hasher, slot, p)?;
        }
    }
    Ok(())
}

/// Submits every dispatch and reports collisions in search order. Returns
/// on the stop flag or once the requested count is reached, leaving the
/// unread dispatches in `pending`.
fn search(
    job: &Job,
    hasher: &GpuHasher,
    pending: &mut [Option<Pending>; SLOTS],
) -> Result<(), String> {
    let mut total = 0;
    for (n, (name_len, work)) in dispatches(job).enumerate() {
        if job.should_stop() {
            return Ok(());
        }
        job.state.name_len.store(name_len, Relaxed);
        let slot = n % SLOTS;
        if let Some(p) = pending[slot].take() {
            if !finish_dispatch(job, hasher, slot, &p)? {
                return Ok(());
            }
        }
        pending[slot] = Some(hasher.submit(slot, work, job.target));
        total = n + 1;
    }
    // The oldest in-flight dispatch sits in slot total % SLOTS.
    for i in 0..SLOTS {
        let slot = (total + i) % SLOTS;
        if let Some(p) = pending[slot].take() {
            if !finish_dispatch(job, hasher, slot, &p)? {
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Every dispatch of the search with the name length it is working on.
/// Random mode walks each length class in DISPATCH-sized chunks; readable
/// mode covers the whole word space in one dispatch per type combo.
fn dispatches(job: &Job) -> Box<dyn Iterator<Item = (usize, Work)> + '_> {
    match job.mode {
        NameMode::Random => Box::new(job.type_combos.iter().flat_map(|suffix| {
            (1..=MAX_CHARS).flat_map(move |len| {
                let total = CharWork::class_size(len);
                (0..total).step_by(DISPATCH as usize).map(move |consumed| {
                    let count = (total - consumed).min(u128::from(DISPATCH)) as u32;
                    let work = CharWork::new(len, consumed, count, suffix);
                    (len, Work::Chars(work))
                })
            })
        })),
        NameMode::Readable => {
            let words = words();
            Box::new(job.type_combos.iter().map(move |suffix| {
                let work = WordWork::new(&words, suffix);
                (work.max_name_len(), Work::Words(work))
            }))
        }
    }
}

/// Waits for `pending`, counts its hashes and returns the matching indices.
fn complete(
    job: &Job,
    hasher: &GpuHasher,
    slot: usize,
    pending: &Pending,
) -> Result<Vec<u32>, String> {
    let matches = hasher.complete(slot, pending)?;
    job.state
        .total_hashes
        .fetch_add(u64::from(pending.work.count()), Relaxed);
    Ok(matches)
}

/// Reads back one dispatch and reports its collisions. Returns false once
/// the requested count is reached.
fn finish_dispatch(
    job: &Job,
    hasher: &GpuHasher,
    slot: usize,
    pending: &Pending,
) -> Result<bool, String> {
    for idx in complete(job, hasher, slot, pending)? {
        let Some(candidate) = pending.work.signature(idx) else {
            return Err(format!("gpu reported a false match for index {idx}"));
        };
        let selector = scalar::keccak256_selector(&candidate);
        if selector != job.target {
            return Err(format!(
                "gpu reported a false match for {:?}",
                String::from_utf8_lossy(&candidate)
            ));
        }
        if !job.report(&candidate, selector) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Receiver};
    use std::time::Duration;

    fn run(
        target: &[u8],
        combos: &[&[u8]],
        threads: usize,
        simd: SimdWidth,
        mode: NameMode,
    ) -> Match {
        let (tx, rx): (Sender<Match>, Receiver<Match>) = channel();
        let job = Job {
            target: scalar::keccak256_selector(target),
            max_count: 1,
            mode,
            type_combos: Arc::new(combos.iter().map(|c| c.to_vec()).collect()),
            state: Arc::new(SharedState::default()),
            tx,
        };
        let state = Arc::clone(&job.state);
        let handles = spawn_cpu_workers(job, threads, simd);
        let m = rx.recv_timeout(Duration::from_secs(10));
        state.stop.store(true, Relaxed);
        for h in handles {
            h.join().unwrap();
        }
        let m = m.expect("no collision found");
        assert_eq!(m.selector, scalar::keccak256_selector(target));
        m
    }

    #[test]
    fn scalar_finds_target() {
        assert_eq!(
            run(b"a()", &[b"()"], 1, SimdWidth::Scalar, NameMode::Random).signature,
            "a()"
        );
    }

    #[test]
    fn multi_thread_finds_target_in_another_threads_range() {
        assert_eq!(
            run(b"z()", &[b"()"], 4, SimdWidth::Scalar, NameMode::Random).signature,
            "z()"
        );
    }

    #[test]
    fn readable_mode_deals_combos_across_threads() {
        let m = run(
            b"deposit(uint256)",
            &[b"()", b"(uint256)"],
            2,
            SimdWidth::Scalar,
            NameMode::Readable,
        );
        assert_eq!(m.signature, "deposit(uint256)");
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn avx2_finds_target_past_first_batch() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }
        assert_eq!(
            run(b"ab()", &[b"()"], 1, SimdWidth::Avx2, NameMode::Random).signature,
            "ab()"
        );
    }

    #[test]
    fn stop_flag_ends_workers() {
        let (tx, _rx) = channel();
        let job = Job {
            target: [0xff; 4],
            max_count: 1,
            mode: NameMode::Random,
            type_combos: Arc::new(vec![b"()".to_vec()]),
            state: Arc::new(SharedState::default()),
            tx,
        };
        let state = Arc::clone(&job.state);
        let handles = spawn_cpu_workers(job, 2, SimdWidth::Scalar);
        thread::sleep(Duration::from_millis(100));
        state.stop.store(true, Relaxed);
        for h in handles {
            h.join().unwrap();
        }
        assert!(state.total_hashes.load(Relaxed) > 0);
    }
}
