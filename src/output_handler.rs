use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicUsize;
use std::time::{Instant, Duration};
use std::thread;

pub fn start_output_thread(
    guess_counter: Arc<AtomicUsize>,
    latest_function_guess: Arc<Mutex<String>>,
    _start_time: Instant,
) {
    thread::spawn(move || {
        let mut last_print = Instant::now();
        let mut last_count = 0;
        loop {
            thread::sleep(Duration::from_secs(2));
            let current_count = guess_counter.load(std::sync::atomic::Ordering::Relaxed);
            let elapsed = last_print.elapsed();
            let rate = (current_count - last_count) as f64 / elapsed.as_secs_f64();
            let latest_guess = latest_function_guess.lock().unwrap().clone();

            let rate_display = if rate >= 1_000_000.0 {
                format!("{:.2} MH/s", rate / 1_000_000.0)
            } else if rate >= 1_000.0 {
                format!("{:.2} kH/s", rate / 1_000.0)
            } else {
                format!("{:.2} H/s", rate)
            };

            println!(
                "hash rate: {}, total hashes: {}, latest function guess: {}",
                rate_display, current_count, latest_guess
            );

            last_print = Instant::now();
            last_count = current_count;
        }
    });
}

pub fn print_found_signature(candidate: &str, start_time: Instant, guess_counter: &Arc<AtomicUsize>) {
    println!(
        "found matching signature: {}. total time: {:?}, total guesses: {}",
        candidate, start_time.elapsed(), guess_counter.load(std::sync::atomic::Ordering::Relaxed)
    );
}