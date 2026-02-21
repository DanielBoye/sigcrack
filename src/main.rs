mod cli;
mod gpu;
mod hardware;
mod target;
mod types;
mod namegen;
mod hasher;
mod worker;
mod tui;
mod wordlist;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use clap::Parser;
use crossbeam_channel::{unbounded, Receiver};

use cli::{Args, NameMode, SimdMode};
use hardware::HardwareInfo;
use hasher::SimdWidth;
use hasher::gpu::GpuHasher;
use namegen::{RandomNameIter, ReadableNameIter};
use target::{selector_hex, Target};
use types::generate_type_combos;
use worker::{spawn_workers, Match, SharedState};
use wordlist::load_words;

fn determine_simd_width(hw: &HardwareInfo, requested: &SimdMode) -> SimdWidth {
    match requested {
        SimdMode::Auto => {
            if hw.has_avx512f { SimdWidth::Avx512 }
            else if hw.has_avx2 { SimdWidth::Avx2 }
            else { SimdWidth::Scalar }
        }
        SimdMode::Avx512 => {
            if hw.has_avx512f { SimdWidth::Avx512 }
            else {
                eprintln!("[!] AVX-512 not available, falling back");
                if hw.has_avx2 { SimdWidth::Avx2 } else { SimdWidth::Scalar }
            }
        }
        SimdMode::Avx2 => {
            if hw.has_avx2 { SimdWidth::Avx2 }
            else {
                eprintln!("[!] AVX2 not available, falling back to scalar");
                SimdWidth::Scalar
            }
        }
        SimdMode::Scalar => SimdWidth::Scalar,
    }
}

fn main() {
    let args = Args::parse();
    let mut hw = HardwareInfo::detect();
    if let Some(gpu_info) = gpu::detect_gpu() {
        hw.gpu_name = Some(gpu_info.name);
        hw.gpu_backend = Some(gpu_info.backend);
    }

    if args.list_devices {
        hw.print_devices();
        return;
    }

    let target_str = match &args.target {
        Some(t) => t.clone(),
        None => {
            eprintln!("Error: <TARGET> is required (use --list-devices to see hardware)");
            std::process::exit(1);
        }
    };

    let target = match Target::parse(&target_str) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if args.gpu && !hw.has_gpu() {
        eprintln!("Error: --gpu requested but no GPU detected");
        std::process::exit(1);
    }

    let mut use_gpu = !args.cpu && hw.has_gpu();

    let type_combos = Arc::new(generate_type_combos(args.max_params));
    let num_combos = type_combos.len();
    let simd_width = determine_simd_width(&hw, &args.simd);

    let gpu_hasher = if use_gpu {
        match GpuHasher::new(args.gpu_device) {
            Ok(h) => Some(h),
            Err(e) => {
                eprintln!("[!] GPU init failed: {} — falling back to CPU", e);
                use_gpu = false;
                None
            }
        }
    } else {
        None
    };

    let num_threads = if use_gpu {
        0
    } else if args.threads != 0 {
        args.threads
    } else {
        hw.physical_cores
    };

    let (tx, rx) = unbounded();
    let state = Arc::new(SharedState::new());

    let handles = if let Some(hasher) = gpu_hasher {
        let gpu_state = Arc::clone(&state);
        let gpu_combos = Arc::clone(&type_combos);
        let gpu_mode = args.mode.clone();
        let gpu_target = target.selector;
        let gpu_count = args.count;
        let handle = thread::spawn(move || {
            run_gpu_worker(gpu_target, gpu_count, gpu_combos, gpu_state, tx, gpu_mode, hasher);
        });
        vec![handle]
    } else {
        spawn_workers(
            num_threads, target.selector, args.count,
            Arc::clone(&type_combos), Arc::clone(&state),
            tx, simd_width, args.mode.clone(),
        )
    };

    let device_label = if use_gpu {
        match (&hw.gpu_name, &hw.gpu_backend) {
            (Some(name), Some(backend)) => format!("{}/{}", name, backend),
            _ => "GPU".to_string(),
        }
    } else {
        let simd_short = if simd_width.name().contains("AVX-512") { "AVX512" }
            else if simd_width.name().contains("AVX2") { "AVX2" }
            else { "scalar" };
        format!("{}T/{}", num_threads, simd_short)
    };

    if args.no_tui {
        run_no_tui(&args, &hw, &target, num_threads, num_combos, &state, rx, handles, simd_width, use_gpu);
    } else {
        let tui_state = tui::TuiState {
            target_display: target.display.clone(),
            target_hex: selector_hex(&target.selector),
            simd_mode: simd_width.name().to_string(),
            simd_lanes: simd_width.lanes(),
            num_threads,
            name_mode: format!("{:?}", args.mode).to_lowercase(),
            num_type_combos: num_combos,
            max_count: args.count,
            start_time: Instant::now(),
            last_finding_time: None,
            total_hashes: 0,
            peak_rate: 0.0,
            current_name_len: 1,
            collisions: Vec::new(),
            device_label,
            use_gpu,
        };

        if let Err(e) = tui::run_tui(tui_state, Arc::clone(&state), rx) {
            eprintln!("TUI error: {}", e);
        }

        state.stop.store(true, Ordering::Relaxed);
        for h in handles { h.join().ok(); }
    }
}

fn run_gpu_worker(
    target: [u8; 4],
    max_count: u32,
    type_combos: Arc<Vec<Vec<u8>>>,
    state: Arc<SharedState>,
    tx: crossbeam_channel::Sender<Match>,
    mode: NameMode,
    hasher: GpuHasher,
) {
    let batch_size = hasher.batch_size() as usize;

    enum NameIterEnum {
        Random(RandomNameIter),
        Readable(ReadableNameIter),
    }

    let mut name_iter = match &mode {
        NameMode::Random => NameIterEnum::Random(RandomNameIter::for_thread(0, 1)),
        NameMode::Readable => NameIterEnum::Readable(ReadableNameIter::new(load_words())),
    };

    for type_combo in type_combos.iter() {
        if state.stop.load(Ordering::Relaxed)
            || state.found_count.load(Ordering::Relaxed) >= max_count
        {
            return;
        }

        let suffix = type_combo.as_slice();
        match &mut name_iter {
            NameIterEnum::Random(r) => r.reset(),
            NameIterEnum::Readable(r) => r.reset(),
        }

        let is_exhausted = match &name_iter {
            NameIterEnum::Random(r) => r.is_exhausted(),
            NameIterEnum::Readable(r) => r.is_exhausted(),
        };
        if is_exhausted {
            continue;
        }

        loop {
            if state.stop.load(Ordering::Relaxed)
                || state.found_count.load(Ordering::Relaxed) >= max_count
            {
                return;
            }

            let mut batch_bufs: Vec<Vec<u8>> = Vec::with_capacity(batch_size);
            let mut done = false;

            for _ in 0..batch_size {
                let exhausted = match &name_iter {
                    NameIterEnum::Random(r) => r.is_exhausted(),
                    NameIterEnum::Readable(r) => r.is_exhausted(),
                };
                if exhausted {
                    done = true;
                    break;
                }

                let name = match &name_iter {
                    NameIterEnum::Random(r) => r.current(),
                    NameIterEnum::Readable(r) => r.current(),
                };
                let mut sig = Vec::with_capacity(name.len() + suffix.len());
                sig.extend_from_slice(name);
                sig.extend_from_slice(suffix);
                batch_bufs.push(sig);

                let advanced = match &mut name_iter {
                    NameIterEnum::Random(r) => r.advance(),
                    NameIterEnum::Readable(r) => r.advance(),
                };
                if !advanced {
                    done = true;
                    break;
                }
            }

            if batch_bufs.is_empty() {
                break;
            }

            let refs: Vec<&[u8]> = batch_bufs.iter().map(|v| v.as_slice()).collect();
            let matched = hasher.hash_batch(&refs, target);

            state
                .total_hashes
                .fetch_add(batch_bufs.len() as u64, Ordering::Relaxed);

            for idx in matched {
                if idx < batch_bufs.len() {
                    let sig = String::from_utf8_lossy(&batch_bufs[idx]).to_string();
                    let selector = crate::hasher::scalar::keccak256_selector(&batch_bufs[idx]);
                    let _ = tx.send(Match {
                        signature: sig,
                        selector,
                    });
                    state.found_count.fetch_add(1, Ordering::Relaxed);
                    if state.found_count.load(Ordering::Relaxed) >= max_count {
                        return;
                    }
                }
            }

            if done {
                break;
            }
        }
    }
}

fn run_no_tui(
    args: &Args,
    hw: &HardwareInfo,
    target: &Target,
    num_threads: usize,
    num_combos: usize,
    state: &Arc<SharedState>,
    rx: Receiver<Match>,
    handles: Vec<thread::JoinHandle<()>>,
    simd_width: SimdWidth,
    use_gpu: bool,
) {
    let start_time = Instant::now();

    eprintln!("[*] CPU: {}", hw);
    if let (Some(name), Some(backend)) = (&hw.gpu_name, &hw.gpu_backend) {
        eprintln!("[*] GPU: {} ({})", name, backend);
    } else {
        eprintln!("[*] GPU: none detected");
    }
    if use_gpu {
        eprintln!("[*] Device: gpu");
    } else {
        eprintln!("[*] Device: cpu — {} threads — {}", num_threads, simd_width.name());
    }
    eprintln!("[*] Target: {} ({})", target.display, selector_hex(&target.selector));
    eprintln!(
        "[*] Mode: {:?} names, max {} params ({} type combos)",
        args.mode, args.max_params, num_combos
    );
    eprintln!("[*] Searching for {} collision(s)...", args.count);
    eprintln!();

    let mut found = 0u32;
    while found < args.count {
        match rx.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(m) => {
                found += 1;
                let elapsed = start_time.elapsed();
                let total = state.total_hashes.load(Ordering::Relaxed);
                let rate = total as f64 / elapsed.as_secs_f64();
                println!("  {}. {} \u{2192} {}", found, m.signature, selector_hex(&m.selector));
                eprintln!(
                    "[*] Found #{} after {:.2}s ({:.2}M hashes, {:.2}M/sec)",
                    found, elapsed.as_secs_f64(),
                    total as f64 / 1_000_000.0, rate / 1_000_000.0
                );
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if handles.iter().all(|h| h.is_finished()) { break; }
                let elapsed = start_time.elapsed();
                let total = state.total_hashes.load(Ordering::Relaxed);
                let rate = total as f64 / elapsed.as_secs_f64();
                eprint!(
                    "\r[*] {:.2}s | {:.2}M hashes | {:.2}M/sec   ",
                    elapsed.as_secs_f64(), total as f64 / 1_000_000.0, rate / 1_000_000.0
                );
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }

    state.stop.store(true, Ordering::Relaxed);
    for h in handles { h.join().ok(); }

    let elapsed = start_time.elapsed();
    let total = state.total_hashes.load(Ordering::Relaxed);
    eprintln!();
    eprintln!(
        "[*] Done. {} collision(s) in {:.2}s ({:.2}M total hashes)",
        found, elapsed.as_secs_f64(), total as f64 / 1_000_000.0
    );
}
