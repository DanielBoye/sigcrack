use ratatui::prelude::*;

pub fn put(buf: &mut Buffer, x: u16, y: u16, ch: char, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_char(ch).set_style(style);
    }
}

pub fn hline(buf: &mut Buffer, x1: u16, x2: u16, y: u16, style: Style) {
    for x in x1..=x2 {
        put(buf, x, y, '─', style);
    }
}

pub fn kv(buf: &mut Buffer, x: u16, y: u16, label: &str, value: &str, color: Color) {
    let label = format!("{label:>12} : ");
    buf.set_string(x, y, &label, Style::default().fg(Color::Gray));
    buf.set_string(x + label.len() as u16, y, value, Style::default().fg(color));
}

/// Truncates to `max` characters with a trailing ellipsis.
pub fn fit(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

pub fn format_num(n: f64) -> String {
    if n >= 1e9 {
        format!("{:.2}B", n / 1e9)
    } else if n >= 1e6 {
        format!("{:.2}M", n / 1e6)
    } else if n >= 1e3 {
        format!("{:.2}K", n / 1e3)
    } else {
        format!("{n:.0}")
    }
}

pub fn format_duration(secs: u64) -> String {
    format!(
        "{} days, {} hrs, {} min, {} sec",
        secs / 86400,
        secs % 86400 / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

pub fn rate_color(rate: f64) -> Color {
    if rate > 100e6 {
        Color::Green
    } else if rate > 10e6 {
        Color::Yellow
    } else {
        Color::Red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(format_num(0.0), "0");
        assert_eq!(format_num(999.0), "999");
        assert_eq!(format_num(1_500.0), "1.50K");
        assert_eq!(format_num(412_830_000.0), "412.83M");
        assert_eq!(format_num(2_340_000_000.0), "2.34B");
    }

    #[test]
    fn durations() {
        assert_eq!(format_duration(7), "0 days, 0 hrs, 0 min, 7 sec");
        assert_eq!(
            format_duration(86400 + 7200 + 300 + 9),
            "1 days, 2 hrs, 5 min, 9 sec"
        );
    }

    #[test]
    fn fit_truncates_on_char_boundaries() {
        assert_eq!(fit("short", 10), "short");
        assert_eq!(fit("transfer(address,uint256)", 12), "transfer(...");
        assert_eq!(fit("ééééé", 4), "é...");
    }
}
