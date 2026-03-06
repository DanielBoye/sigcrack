use ratatui::prelude::*;

use super::widgets::*;
use super::View;
use crate::target::selector_hex;
use crate::Engine;

const MIN_WIDTH: u16 = 64;
const MIN_HEIGHT: u16 = 16;
const MAX_WIDTH: u16 = 100;

struct Grid {
    left: u16,
    mid: u16,
    right: u16,
    top: u16,
    div1: u16,
    div2: u16,
    div3: u16,
    bottom: u16,
    footer: u16,
}

impl Grid {
    fn new(area: Rect, collision_rows: u16) -> Self {
        let width = area.width.min(MAX_WIDTH);
        let left = area.x + (area.width - width) / 2;
        let top = area.y + 1;
        let div1 = top + 3;
        let div2 = div1 + 4;
        let div3 = div2 + 4;
        let bottom = div3 + 1 + collision_rows;
        Self {
            left,
            mid: left + width * 55 / 100,
            right: left + width - 1,
            top,
            div1,
            div2,
            div3,
            bottom,
            footer: bottom + 1,
        }
    }

    fn width(&self) -> u16 {
        self.right - self.left + 1
    }

    fn right_inner(&self) -> usize {
        (self.right - self.mid - 3) as usize
    }

    fn full_inner(&self) -> usize {
        (self.right - self.left - 3) as usize
    }
}

pub fn render(frame: &mut Frame, view: &View) {
    let area = frame.area();
    let buf = frame.buffer_mut();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = format!("terminal too small, need {MIN_WIDTH}x{MIN_HEIGHT}");
        buf.set_string(area.x, area.y, msg, Style::default().fg(Color::Red));
        return;
    }
    let max_rows = (area.height - MIN_HEIGHT + 1) as usize;
    let rows = view.collisions.len().clamp(1, max_rows);
    let g = Grid::new(area, rows as u16);

    borders(buf, &g);
    banner(buf, &g, view);
    timing(buf, &g, view);
    results(buf, &g, view);
    performance(buf, &g, view);
    target(buf, &g, view);
    search(buf, &g, view);
    engine(buf, &g, view);
    collisions(buf, &g, view, rows);

    let footer = if view.done {
        "done  [q] quit"
    } else {
        "[q] quit"
    };
    buf.set_string(
        g.right + 1 - footer.len() as u16,
        g.footer,
        footer,
        Style::default().fg(Color::DarkGray),
    );
}

fn borders(buf: &mut Buffer, g: &Grid) {
    let line = Style::default().fg(Color::White);
    let title = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    for y in g.top + 1..g.bottom {
        put(buf, g.left, y, '│', line);
        put(buf, g.right, y, '│', line);
        if y < g.div3 {
            put(buf, g.mid, y, '│', line);
        }
    }
    let rows = [
        (
            g.top,
            '┌',
            '┬',
            '┐',
            " process timing ",
            " overall results ",
        ),
        (g.div1, '├', '┼', '┤', " hash performance ", " target "),
        (g.div2, '├', '┼', '┤', " search progress ", " engine "),
        (g.div3, '├', '┴', '┤', " collisions ", ""),
        (g.bottom, '└', '─', '┘', "", ""),
    ];
    for (y, l, m, r, left_title, right_title) in rows {
        hline(buf, g.left + 1, g.right - 1, y, line);
        put(buf, g.left, y, l, line);
        put(buf, g.mid, y, m, line);
        put(buf, g.right, y, r, line);
        buf.set_string(g.left + 2, y, left_title, title);
        buf.set_string(g.mid + 2, y, right_title, title);
    }
}

fn banner(buf: &mut Buffer, g: &Grid, view: &View) {
    let text = fit(
        &format!(
            "sigcrack ({}) [{}] {{{}}}",
            view.selector, view.name_mode, view.engine
        ),
        g.width() as usize,
    );
    let x = g.left + (g.width() - text.len() as u16) / 2;
    buf.set_string(
        x,
        g.top - 1,
        text,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
}

fn timing(buf: &mut Buffer, g: &Grid, view: &View) {
    let (x, y) = (g.left + 2, g.top + 1);
    kv(
        buf,
        x,
        y,
        "run time",
        &format_duration(view.start.elapsed().as_secs()),
        Color::Yellow,
    );
    match view.last_find {
        Some(t) => kv(
            buf,
            x,
            y + 1,
            "last find",
            &format_duration(t.elapsed().as_secs()),
            Color::Green,
        ),
        None => kv(buf, x, y + 1, "last find", "none yet", Color::DarkGray),
    }
}

fn results(buf: &mut Buffer, g: &Grid, view: &View) {
    let (x, y) = (g.mid + 2, g.top + 1);
    let found = view.collisions.len();
    let color = if found > 0 {
        Color::Green
    } else {
        Color::Yellow
    };
    kv(
        buf,
        x,
        y,
        "collisions",
        &format!("{found} / {}", view.max_count),
        color,
    );
    kv(
        buf,
        x,
        y + 1,
        "candidates",
        &format_num(view.total_hashes as f64),
        Color::Yellow,
    );
}

fn performance(buf: &mut Buffer, g: &Grid, view: &View) {
    let (x, y) = (g.left + 2, g.div1 + 1);
    let avg = view.total_hashes as f64 / view.start.elapsed().as_secs_f64().max(1e-9);
    for (i, (label, rate)) in [
        ("rate", view.rate),
        ("avg rate", avg),
        ("peak rate", view.peak_rate),
    ]
    .iter()
    .enumerate()
    {
        kv(
            buf,
            x,
            y + i as u16,
            label,
            &format!("{}/s", format_num(*rate)),
            rate_color(*rate),
        );
    }
}

fn target(buf: &mut Buffer, g: &Grid, view: &View) {
    let (x, y) = (g.mid + 2, g.div1 + 1);
    kv(buf, x, y, "selector", &view.selector, Color::Cyan);
    kv(
        buf,
        x,
        y + 1,
        "signature",
        &fit(&view.target, g.right_inner() - 13),
        Color::Yellow,
    );
}

fn search(buf: &mut Buffer, g: &Grid, view: &View) {
    let (x, y) = (g.left + 2, g.div2 + 1);
    kv(
        buf,
        x,
        y,
        "name len",
        &format!("{} chars", view.name_len),
        Color::Yellow,
    );
    kv(
        buf,
        x,
        y + 1,
        "combos",
        &view.type_combos.to_string(),
        Color::Yellow,
    );
    kv(buf, x, y + 2, "name mode", view.name_mode, Color::Yellow);
}

fn engine(buf: &mut Buffer, g: &Grid, view: &View) {
    let (x, y) = (g.mid + 2, g.div2 + 1);
    let max = g.right_inner() - 13;
    match &view.engine {
        Engine::Cpu { threads, simd } => {
            kv(buf, x, y, "device", "cpu", Color::Cyan);
            kv(
                buf,
                x,
                y + 1,
                "threads",
                &threads.to_string(),
                Color::Yellow,
            );
            kv(
                buf,
                x,
                y + 2,
                "simd",
                &format!("{} ({} lanes)", simd.name(), simd.lanes()),
                Color::Cyan,
            );
        }
        Engine::Gpu { name, backend } => {
            kv(buf, x, y, "device", "gpu", Color::Cyan);
            kv(buf, x, y + 1, "name", &fit(name, max), Color::Yellow);
            kv(buf, x, y + 2, "backend", backend, Color::Cyan);
        }
    }
}

fn collisions(buf: &mut Buffer, g: &Grid, view: &View, rows: usize) {
    let (x, y) = (g.left + 2, g.div3 + 1);
    if view.collisions.is_empty() {
        let style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);
        let text = if view.done {
            "nothing found"
        } else {
            "searching..."
        };
        buf.set_string(x, y, text, style);
        return;
    }
    let skip = view.collisions.len().saturating_sub(rows);
    for (i, m) in view.collisions.iter().enumerate().skip(skip) {
        let y = y + (i - skip) as u16;
        let prefix = format!("{}. ", i + 1);
        let hex = selector_hex(&m.selector);
        let sig = fit(
            &m.signature,
            g.full_inner().saturating_sub(prefix.len() + hex.len() + 1),
        );
        buf.set_string(x, y, &prefix, Style::default().fg(Color::Gray));
        let x = x + prefix.len() as u16;
        buf.set_string(
            x,
            y,
            &sig,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
        buf.set_string(
            x + sig.len() as u16 + 1,
            y,
            &hex,
            Style::default().fg(Color::Cyan),
        );
    }
}
