use ratatui::prelude::*;

use super::TuiState;
use super::widgets::*;

const GRID_WIDTH: u16 = 79;

struct Grid {
    left: u16,
    right: u16,
    mid: u16,
    right_inner_w: u16,
    full_inner_w: u16,
    banner_y: u16,
    top: u16,
    div1: u16,
    div2: u16,
    div3: u16,
    bottom: u16,
    footer_y: u16,
    full_width: u16,
}

impl Grid {
    fn from_area(area: Rect) -> Self {
        let w = GRID_WIDTH.min(area.width);
        let left = area.x + (area.width.saturating_sub(w)) / 2;
        let right = left + w - 1;
        let mid = left + w * 55 / 100;
        let banner_y = area.y;
        let top = area.y + 1;
        let div1 = top + 3;
        let div2 = div1 + 4;
        let div3 = div2 + 4;
        let bottom = div3 + 3;
        let footer_y = bottom + 1;
        Self {
            left, right, mid,
            right_inner_w: right.saturating_sub(mid + 2),
            full_inner_w: right.saturating_sub(left + 2),
            banner_y, top, div1, div2, div3, bottom, footer_y,
            full_width: w,
        }
    }
}

struct AflGrid<'a> {
    state: &'a TuiState,
}

impl<'a> Widget for AflGrid<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 17 || area.width < 60 {
            buf.set_string(
                area.x, area.y,
                "Terminal too small (need 60x17)",
                Style::default().fg(Color::Red),
            );
            return;
        }
        let g = Grid::from_area(area);
        let bs = Style::default().fg(Color::White);
        let ts = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);

        draw_borders(buf, &g, bs);
        draw_titles(buf, &g, ts);
        render_banner(buf, &g, self.state);
        render_process_timing(buf, &g, self.state);
        render_overall_results(buf, &g, self.state);
        render_hash_performance(buf, &g, self.state);
        render_target_info(buf, &g, self.state);
        render_search_progress(buf, &g, self.state);
        render_engine_config(buf, &g, self.state);
        render_collisions(buf, &g, self.state);
        render_footer(buf, &g);
    }
}

pub fn render(frame: &mut Frame, state: &TuiState) {
    frame.render_widget(AflGrid { state }, frame.area());
}

fn draw_borders(buf: &mut Buffer, g: &Grid, s: Style) {
    set_cell(buf, g.left, g.top, '┌', s);
    draw_h_line(buf, g.left + 1, g.mid - 1, g.top, '─', s);
    set_cell(buf, g.mid, g.top, '┬', s);
    draw_h_line(buf, g.mid + 1, g.right - 1, g.top, '─', s);
    set_cell(buf, g.right, g.top, '┐', s);

    set_cell(buf, g.left, g.div1, '├', s);
    draw_h_line(buf, g.left + 1, g.mid - 1, g.div1, '─', s);
    set_cell(buf, g.mid, g.div1, '┼', s);
    draw_h_line(buf, g.mid + 1, g.right - 1, g.div1, '─', s);
    set_cell(buf, g.right, g.div1, '┤', s);

    set_cell(buf, g.left, g.div2, '├', s);
    draw_h_line(buf, g.left + 1, g.mid - 1, g.div2, '─', s);
    set_cell(buf, g.mid, g.div2, '┼', s);
    draw_h_line(buf, g.mid + 1, g.right - 1, g.div2, '─', s);
    set_cell(buf, g.right, g.div2, '┤', s);

    set_cell(buf, g.left, g.div3, '├', s);
    draw_h_line(buf, g.left + 1, g.mid - 1, g.div3, '─', s);
    set_cell(buf, g.mid, g.div3, '┴', s);
    draw_h_line(buf, g.mid + 1, g.right - 1, g.div3, '─', s);
    set_cell(buf, g.right, g.div3, '┤', s);

    set_cell(buf, g.left, g.bottom, '└', s);
    draw_h_line(buf, g.left + 1, g.right - 1, g.bottom, '─', s);
    set_cell(buf, g.right, g.bottom, '┘', s);

    for y in (g.top + 1)..g.bottom {
        if y == g.div1 || y == g.div2 || y == g.div3 { continue; }
        set_cell(buf, g.left, y, '│', s);
        set_cell(buf, g.right, y, '│', s);
    }

    for y in (g.top + 1)..g.div3 {
        if y == g.div1 || y == g.div2 { continue; }
        set_cell(buf, g.mid, y, '│', s);
    }
}

fn draw_titles(buf: &mut Buffer, g: &Grid, s: Style) {
    draw_title_on_border(buf, g.left + 2, g.top, " process timing ", s);
    draw_title_on_border(buf, g.mid + 2, g.top, " overall results ", s);
    draw_title_on_border(buf, g.left + 2, g.div1, " hash performance ", s);
    draw_title_on_border(buf, g.mid + 2, g.div1, " target info ", s);
    draw_title_on_border(buf, g.left + 2, g.div2, " search progress ", s);
    draw_title_on_border(buf, g.mid + 2, g.div2, " engine config ", s);
    draw_title_on_border(buf, g.left + 2, g.div3, " collisions found ", s);
}

fn render_banner(buf: &mut Buffer, g: &Grid, state: &TuiState) {
    let banner = format!(
        "sigcrack ({}) [{}] {{{}}}",
        state.target_hex, state.name_mode, state.device_label
    );
    let x = g.left + g.full_width.saturating_sub(banner.len() as u16) / 2;
    buf.set_string(x, g.banner_y, &banner,
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
}

fn render_process_timing(buf: &mut Buffer, g: &Grid, state: &TuiState) {
    let x = g.left + 2;
    let y = g.top + 1;
    draw_kv(buf, x, y, "      run time", &format_duration(state.start_time.elapsed().as_secs()), Color::Yellow);
    let (lf, lf_c) = match state.last_finding_time {
        Some(t) => (format_duration(t.elapsed().as_secs()), Color::Green),
        None => ("n/a".to_string(), Color::DarkGray),
    };
    draw_kv(buf, x, y + 1, "  last finding", &lf, lf_c);
}

fn render_overall_results(buf: &mut Buffer, g: &Grid, state: &TuiState) {
    let x = g.mid + 2;
    let y = g.top + 1;
    let found = state.collisions.len() as u32;
    let col_color = if found > 0 { Color::Green } else { Color::Yellow };
    draw_kv(buf, x, y, "  collisions", &format!("{} / {}", found, state.max_count), col_color);
    draw_kv(buf, x, y + 1, "  candidates", &format_count(state.total_hashes), Color::Yellow);
}

fn render_hash_performance(buf: &mut Buffer, g: &Grid, state: &TuiState) {
    let x = g.left + 2;
    let y = g.div1 + 1;
    let elapsed = state.start_time.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 { state.total_hashes as f64 / elapsed } else { 0.0 };
    draw_kv(buf, x, y, "   hash rate", &format!("{}/sec", format_rate(rate)), rate_color(rate));
    draw_kv(buf, x, y + 1, "   peak rate", &format!("{}/sec", format_rate(state.peak_rate)), rate_color(state.peak_rate));
    draw_kv(buf, x, y + 2, "   simd mode", &state.simd_mode, Color::Cyan);
}

fn render_target_info(buf: &mut Buffer, g: &Grid, state: &TuiState) {
    let x = g.mid + 2;
    let w = g.right_inner_w;
    let y = g.div1 + 1;
    draw_kv(buf, x, y, "   selector", &state.target_hex, Color::Cyan);
    let max_val = (w as usize).saturating_sub(15);
    let display = if state.target_display.len() > max_val {
        format!("{}...", &state.target_display[..max_val.saturating_sub(3)])
    } else {
        state.target_display.clone()
    };
    draw_kv(buf, x, y + 1, "  input sig", &display, Color::Yellow);
    draw_kv(buf, x, y + 2, "  name mode", &state.name_mode, Color::Yellow);
}

fn render_search_progress(buf: &mut Buffer, g: &Grid, state: &TuiState) {
    let x = g.left + 2;
    let y = g.div2 + 1;
    draw_kv(buf, x, y, " name length", &format!("{} chars", state.current_name_len), Color::Yellow);
    draw_kv(buf, x, y + 1, " type combos", &format!("{}", state.num_type_combos), Color::Yellow);
    draw_kv(buf, x, y + 2, " total execs", &format_count(state.total_hashes), Color::Yellow);
}

fn render_engine_config(buf: &mut Buffer, g: &Grid, state: &TuiState) {
    let x = g.mid + 2;
    let y = g.div2 + 1;
    if state.use_gpu {
        draw_kv(buf, x, y, "      device", &state.device_label, Color::Cyan);
        draw_kv(buf, x, y + 1, "        mode", "gpu", Color::Cyan);
        let search_pct = if state.num_type_combos > 0 {
            let nl = state.current_name_len;
            let est = if nl <= 1 { 0.1 } else { ((nl as f64 - 1.0) * 5.0).min(99.9) };
            format!("{:.1}%", est)
        } else { "0.0%".to_string() };
        draw_kv(buf, x, y + 2, "search space", &search_pct, Color::Yellow);
    } else {
        draw_kv(buf, x, y, "     threads", &state.num_threads.to_string(), Color::Yellow);
        let simd_short = if state.simd_mode.contains("AVX-512") { "AVX512" }
            else if state.simd_mode.contains("AVX2") { "AVX2" }
            else { "scalar" };
        draw_kv(buf, x, y + 1, "  simd lanes", &format!("{} ({})", state.simd_lanes, simd_short), Color::Cyan);
        let search_pct = if state.num_type_combos > 0 {
            let nl = state.current_name_len;
            let est = if nl <= 1 { 0.1 } else { ((nl as f64 - 1.0) * 5.0).min(99.9) };
            format!("{:.1}%", est)
        } else { "0.0%".to_string() };
        draw_kv(buf, x, y + 2, "search space", &search_pct, Color::Yellow);
    }
}

fn render_collisions(buf: &mut Buffer, g: &Grid, state: &TuiState) {
    let x = g.left + 2;
    let w = g.full_inner_w;
    let y = g.div3 + 1;
    if state.collisions.is_empty() {
        buf.set_string(x, y, "searching...",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC));
    } else {
        for (i, m) in state.collisions.iter().take(2).enumerate() {
            let hex = crate::target::selector_hex(&m.selector);
            let prefix = format!("{}. ", i + 1);
            let arrow = " \u{2192} ";
            let max_sig = (w as usize).saturating_sub(prefix.len() + arrow.len() + hex.len());
            let sig = if m.signature.len() > max_sig {
                format!("{}...", &m.signature[..max_sig.saturating_sub(3)])
            } else { m.signature.clone() };

            let mut cx = x;
            buf.set_string(cx, y + i as u16, &prefix, Style::default().fg(Color::Gray));
            cx += prefix.len() as u16;
            buf.set_string(cx, y + i as u16, &sig,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
            cx += sig.len() as u16;
            buf.set_string(cx, y + i as u16, arrow, Style::default().fg(Color::DarkGray));
            cx += 3;
            buf.set_string(cx, y + i as u16, &hex, Style::default().fg(Color::Cyan));
        }
    }
}

fn render_footer(buf: &mut Buffer, g: &Grid) {
    let footer = "[q] quit";
    let x = g.right.saturating_sub(footer.len() as u16);
    buf.set_string(x, g.footer_y, footer, Style::default().fg(Color::DarkGray));
}
