pub mod layout;
pub mod widgets;

use std::io;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{cursor, execute};
use ratatui::prelude::*;

use crate::target::{selector_hex, Target};
use crate::worker::{Match, SharedState};
use crate::Engine;
use widgets::format_num;

pub struct View {
    pub target: String,
    pub selector: String,
    pub engine: Engine,
    pub name_mode: &'static str,
    pub type_combos: usize,
    pub max_count: u32,
    pub start: Instant,
    pub last_find: Option<Instant>,
    pub total_hashes: u64,
    pub rate: f64,
    pub done: bool,
    pub peak_rate: f64,
    pub name_len: usize,
    pub collisions: Vec<Match>,
}

impl View {
    pub fn new(
        target: &Target,
        engine: Engine,
        name_mode: &'static str,
        type_combos: usize,
        max_count: u32,
    ) -> Self {
        Self {
            target: target.display.clone(),
            selector: selector_hex(&target.selector),
            engine,
            name_mode,
            type_combos,
            max_count,
            start: Instant::now(),
            last_find: None,
            total_hashes: 0,
            rate: 0.0,
            done: false,
            peak_rate: 0.0,
            name_len: 1,
            collisions: Vec::new(),
        }
    }
}

pub fn run(mut view: View, shared: Arc<SharedState>, rx: Receiver<Match>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = event_loop(&mut terminal, &mut view, &shared, &rx);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show)?;
    result?;

    let secs = view.start.elapsed().as_secs_f64();
    for m in &view.collisions {
        println!("{} {}", m.signature, selector_hex(&m.selector));
    }
    println!(
        "{} collision(s), {secs:.1}s, {} hashes, {}/s",
        view.collisions.len(),
        format_num(view.total_hashes as f64),
        format_num(view.total_hashes as f64 / secs.max(1e-9)),
    );
    Ok(())
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    view: &mut View,
    shared: &SharedState,
    rx: &Receiver<Match>,
) -> io::Result<()> {
    let mut prev = (Instant::now(), 0u64);
    loop {
        // Workers exit once the count is reached or the space is exhausted;
        // the screen stays up until the user quits.
        view.done |= drain_matches(view, rx);
        view.total_hashes = shared.total_hashes.load(Relaxed);
        view.name_len = shared.name_len.load(Relaxed);

        let now = Instant::now();
        let dt = now.duration_since(prev.0).as_secs_f64();
        if dt >= 0.5 {
            view.rate = view.total_hashes.saturating_sub(prev.1) as f64 / dt;
            view.peak_rate = view.peak_rate.max(view.rate);
            prev = (now, view.total_hashes);
        }

        terminal.draw(|frame| layout::render(frame, view))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let ctrl_c =
                    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
                if ctrl_c
                    || matches!(
                        key.code,
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc
                    )
                {
                    return Ok(());
                }
            }
        }
    }
}

/// Collects new matches. Returns true once every worker has exited, which
/// drops the last sender.
fn drain_matches(view: &mut View, rx: &Receiver<Match>) -> bool {
    loop {
        match rx.try_recv() {
            Ok(m) => {
                view.last_find = Some(Instant::now());
                view.collisions.push(m);
            }
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => return true,
        }
    }
}
