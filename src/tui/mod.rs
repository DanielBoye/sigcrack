pub mod layout;
pub mod widgets;

use std::io;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::Terminal;

use crate::target::selector_hex;
use crate::worker::{Match, SharedState};

pub struct TuiState {
    pub target_display: String,
    pub target_hex: String,
    pub simd_mode: String,
    pub simd_lanes: usize,
    pub num_threads: usize,
    pub name_mode: String,
    pub num_type_combos: usize,
    pub max_count: u32,
    pub start_time: Instant,
    pub last_finding_time: Option<Instant>,
    pub total_hashes: u64,
    pub peak_rate: f64,
    pub current_name_len: u64,
    pub collisions: Vec<Match>,
    pub device_label: String,
    pub use_gpu: bool,
}

pub fn run_tui(
    mut tui_state: TuiState,
    shared: Arc<SharedState>,
    rx: Receiver<Match>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();
    let mut prev_hashes: u64 = 0;
    let mut prev_time = Instant::now();

    loop {
        while let Ok(m) = rx.try_recv() {
            tui_state.last_finding_time = Some(Instant::now());
            tui_state.collisions.push(m);
        }

        tui_state.total_hashes = shared.total_hashes.load(Ordering::Relaxed);
        tui_state.current_name_len = shared.current_name_len.load(Ordering::Relaxed);

        let now = Instant::now();
        let dt = now.duration_since(prev_time).as_secs_f64();
        if dt >= 0.5 {
            let dh = tui_state.total_hashes.saturating_sub(prev_hashes);
            let instant_rate = dh as f64 / dt;
            if instant_rate > tui_state.peak_rate {
                tui_state.peak_rate = instant_rate;
            }
            prev_hashes = tui_state.total_hashes;
            prev_time = now;
        }

        terminal.draw(|frame| {
            layout::render(frame, &tui_state);
        })?;

        let found = tui_state.collisions.len() as u32;
        if found >= tui_state.max_count && !shared.stop.load(Ordering::Relaxed) {
            shared.stop.store(true, Ordering::Relaxed);
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Esc => break,
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    terminal.show_cursor()?;

    let elapsed = tui_state.start_time.elapsed();
    let total = tui_state.total_hashes;
    let rate = if elapsed.as_secs_f64() > 0.0 {
        total as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };

    if tui_state.collisions.is_empty() {
        println!("[*] No collisions found.");
    } else {
        println!();
        println!("[*] Results:");
        for (i, m) in tui_state.collisions.iter().enumerate() {
            println!("  {}. {} \u{2192} {}", i + 1, m.signature, selector_hex(&m.selector));
        }
    }
    println!();
    println!(
        "[*] Done. {} collision(s) in {:.2}s ({} total hashes, {}/sec)",
        tui_state.collisions.len(),
        elapsed.as_secs_f64(),
        widgets::format_count(total),
        widgets::format_rate(rate),
    );

    Ok(())
}
