use ratatui::prelude::*;

pub fn draw_h_line(buf: &mut Buffer, x1: u16, x2: u16, y: u16, ch: char, style: Style) {
    let mut b = [0u8; 4];
    let s = ch.encode_utf8(&mut b);
    for x in x1..=x2 {
        buf.set_string(x, y, &*s, style);
    }
}

pub fn set_cell(buf: &mut Buffer, x: u16, y: u16, ch: char, style: Style) {
    let mut b = [0u8; 4];
    buf.set_string(x, y, ch.encode_utf8(&mut b), style);
}

pub fn draw_title_on_border(buf: &mut Buffer, x: u16, y: u16, title: &str, style: Style) {
    buf.set_string(x, y, title, style);
}

pub fn draw_kv(buf: &mut Buffer, x: u16, y: u16, label: &str, value: &str, value_color: Color) {
    let label_str = format!("{} : ", label);
    buf.set_string(x, y, &label_str, Style::default().fg(Color::Gray));
    buf.set_string(x + label_str.len() as u16, y, value, Style::default().fg(value_color));
}

pub fn format_count(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1e9)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1e6)
    } else if n >= 1_000 {
        format!("{:.2}K", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}

pub fn format_rate(rate: f64) -> String {
    if rate >= 1e9 {
        format!("{:.2}B", rate / 1e9)
    } else if rate >= 1e6 {
        format!("{:.2}M", rate / 1e6)
    } else if rate >= 1e3 {
        format!("{:.2}K", rate / 1e3)
    } else {
        format!("{:.0}", rate)
    }
}

pub fn rate_color(rate: f64) -> Color {
    if rate > 100_000_000.0 {
        Color::Green
    } else if rate > 10_000_000.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

pub fn format_duration(secs: u64) -> String {
    let days = secs / 86400;
    let hrs = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    format!("{} days, {} hrs, {} min", days, hrs, mins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1.00K");
        assert_eq!(format_count(1_500_000), "1.50M");
        assert_eq!(format_count(2_340_000_000), "2.34B");
    }

    #[test]
    fn test_format_rate() {
        assert_eq!(format_rate(500.0), "500");
        assert_eq!(format_rate(1_500.0), "1.50K");
        assert_eq!(format_rate(412_830_000.0), "412.83M");
        assert_eq!(format_rate(1_200_000_000.0), "1.20B");
    }

    #[test]
    fn test_rate_color() {
        assert_eq!(rate_color(200_000_000.0), Color::Green);
        assert_eq!(rate_color(50_000_000.0), Color::Yellow);
        assert_eq!(rate_color(5_000_000.0), Color::Red);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0 days, 0 hrs, 0 min");
        assert_eq!(format_duration(183), "0 days, 0 hrs, 3 min");
        assert_eq!(format_duration(86400 + 7200 + 300), "1 days, 2 hrs, 5 min");
    }
}
