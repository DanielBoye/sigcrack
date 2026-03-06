mod cli;
mod gpu;
mod hardware;
mod hasher;
mod namegen;
mod target;
mod tui;
mod types;
mod worker;

use std::fmt::Display;
use std::process::exit;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;

use cli::{Args, SimdMode};
use hardware::Hardware;
use hasher::gpu::GpuHasher;
use hasher::SimdWidth;
use target::{selector_hex, Target};
use tui::widgets::format_num;
use worker::{Job, Match, SharedState};

pub enum Engine {
    Cpu { threads: usize, simd: SimdWidth },
    Gpu { name: String, backend: String },
}

impl Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Engine::Cpu { threads, simd } => write!(f, "cpu, {threads} threads, {}", simd.name()),
            Engine::Gpu { name, backend } => write!(f, "gpu, {name} ({backend})"),
        }
    }
}

fn fail(msg: impl Display) -> ! {
    eprintln!("error: {msg}");
    exit(1)
}

fn main() {
    let args = Args::parse();
    let hw = Hardware::detect();
    if args.list_devices {
        hw.print_devices();
        return;
    }

    let Some(target) = &args.target else {
        fail("missing target")
    };
    let target = Target::parse(target).unwrap_or_else(|e| fail(e));

    let type_combos = Arc::new(types::type_combos(args.max_params.into()));
    let gpu = select_gpu(&args, &hw);
    let (tx, rx) = mpsc::channel();
    let job = Job {
        target: target.selector,
        max_count: args.count,
        mode: args.mode,
        type_combos: Arc::clone(&type_combos),
        state: Arc::new(SharedState::default()),
        tx,
    };
    let state = Arc::clone(&job.state);

    let (engine, handles) = match gpu {
        Some((info, hasher)) => {
            let engine = Engine::Gpu {
                name: info.name.clone(),
                backend: info.backend.clone(),
            };
            (engine, vec![worker::spawn_gpu_worker(job, hasher)])
        }
        None => {
            let simd = simd_width(&hw, args.simd);
            let threads = if args.threads == 0 {
                hw.physical_cores
            } else {
                args.threads
            };
            (
                Engine::Cpu { threads, simd },
                worker::spawn_cpu_workers(job, threads, simd),
            )
        }
    };

    if args.no_tui {
        run_plain(&args, &hw, &target, &engine, type_combos.len(), &state, &rx);
    } else {
        let view = tui::View::new(
            &target,
            engine,
            args.mode.name(),
            type_combos.len(),
            args.count,
        );
        if let Err(e) = tui::run(view, Arc::clone(&state), rx) {
            eprintln!("tui error: {e}");
        }
    }

    state.stop.store(true, Relaxed);
    for h in handles {
        let _ = h.join();
    }
}

fn select_gpu<'a>(args: &Args, hw: &'a Hardware) -> Option<(&'a gpu::GpuInfo, GpuHasher)> {
    if args.cpu {
        return None;
    }
    let Some(info) = hw.gpus.get(args.gpu_device) else {
        if args.gpu || args.gpu_device != 0 {
            fail(format!("no gpu with index {}", args.gpu_device));
        }
        return None;
    };
    match GpuHasher::new(&info.adapter) {
        Ok(hasher) => Some((info, hasher)),
        Err(e) if args.gpu => fail(format!("gpu init failed: {e}")),
        Err(e) => {
            eprintln!("gpu init failed ({e}), using cpu");
            None
        }
    }
}

fn simd_width(hw: &Hardware, requested: SimdMode) -> SimdWidth {
    let best = hw.best_simd();
    let wanted = match requested {
        SimdMode::Auto => return best,
        SimdMode::Avx512 => SimdWidth::Avx512,
        SimdMode::Avx2 => SimdWidth::Avx2,
        SimdMode::Scalar => SimdWidth::Scalar,
    };
    if wanted.lanes() > best.lanes() {
        eprintln!("{} not supported, using {}", wanted.name(), best.name());
        return best;
    }
    wanted
}

fn status(state: &SharedState, start: Instant) -> String {
    let secs = start.elapsed().as_secs_f64();
    let total = state.total_hashes.load(Relaxed) as f64;
    format!(
        "{secs:.1}s  {} hashes  {}/s",
        format_num(total),
        format_num(total / secs.max(1e-9))
    )
}

fn run_plain(
    args: &Args,
    hw: &Hardware,
    target: &Target,
    engine: &Engine,
    num_combos: usize,
    state: &SharedState,
    rx: &Receiver<Match>,
) {
    let start = Instant::now();
    eprintln!("cpu: {hw}");
    for gpu in &hw.gpus {
        eprintln!("gpu: {} ({})", gpu.name, gpu.backend);
    }
    eprintln!("engine: {engine}");
    eprintln!(
        "target: {} ({})",
        target.display,
        selector_hex(&target.selector)
    );
    eprintln!(
        "names: {}, up to {} params, {num_combos} type combos",
        args.mode.name(),
        args.max_params
    );
    eprintln!();

    let mut found = 0;
    while found < args.count {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(m) => {
                found += 1;
                println!("{} {}", m.signature, selector_hex(&m.selector));
                eprintln!(
                    "found {found}/{} after {}",
                    args.count,
                    status(state, start)
                );
            }
            Err(RecvTimeoutError::Timeout) => eprint!("\r{}   ", status(state, start)),
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    // Workers count their in-flight work on the way out, so the summary
    // waits for the last sender to drop.
    state.stop.store(true, Relaxed);
    for _ in rx.iter() {}
    eprintln!("\r{found} collision(s), {}", status(state, start));
}
