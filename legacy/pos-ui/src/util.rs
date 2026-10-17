//! Small UI helpers: toasts, the scan buffer and layout utilities.

use std::time::Instant;

use eframe::egui;
use pos_domain::Money;

/// A transient status message.
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    pub created: Instant,
}

impl Toast {
    pub fn new(message: String, is_error: bool) -> Self {
        Self {
            message,
            is_error,
            created: Instant::now(),
        }
    }

    pub fn expired(&self) -> bool {
        self.created.elapsed().as_secs_f32() > 4.5
    }
}

/// Accumulates rapid keyboard-wedge keystrokes into a barcode.
#[derive(Default)]
pub struct ScanBuffer {
    pub buf: String,
    last: Option<f64>,
    min_len: usize,
}

impl ScanBuffer {
    pub fn reset(&mut self) {
        self.buf.clear();
        self.last = None;
    }

    /// Append a character. If the gap since the previous key exceeds the
    /// threshold, the buffer is assumed to be slow human typing and is cleared.
    pub fn push_char(&mut self, ch: char, now_seconds: f64, threshold_ms: u64) {
        if let Some(last) = self.last {
            let gap_ms = (now_seconds - last) * 1000.0;
            if gap_ms > threshold_ms as f64 {
                self.buf.clear();
            }
        }
        self.buf.push(ch);
        self.last = Some(now_seconds);
    }

    /// Finish a scan: return the code if it is plausibly a barcode.
    pub fn take(&mut self) -> Option<String> {
        let s = std::mem::take(&mut self.buf).trim().to_string();
        self.last = None;
        let min = if self.min_len == 0 { 4 } else { self.min_len };
        if s.chars().count() >= min {
            Some(s)
        } else {
            None
        }
    }

    pub fn current(&self) -> &str {
        &self.buf
    }
}

/// Run `add` with an LTR or RTL horizontal layout.
pub fn hrow<R>(ui: &mut egui::Ui, rtl: bool, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let layout = if rtl {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    ui.with_layout(layout, add).inner
}

/// Format an amount as `1 234,56 DA`.
pub fn money(m: Money) -> String {
    m.format_dzd()
}

/// Format a Unix timestamp as a local date+time string.
pub fn fmt_ts(unix: i64) -> String {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(unix, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| unix.to_string())
}

/// Local-midnight timestamp for a date offset in days from today.
pub fn local_day_start(offset_days: i64) -> i64 {
    use chrono::{Duration, Local};
    let today = Local::now().date_naive();
    let date = today - Duration::days(offset_days);
    date.and_hms_opt(0, 0, 0)
        .and_then(|dt| dt.and_local_timezone(Local).single())
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

/// Local-midnight timestamp for the first day of the current month.
pub fn local_month_start() -> i64 {
    use chrono::{Datelike, Local};
    let now = Local::now();
    chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|dt| dt.and_local_timezone(Local).single())
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

/// now + 1 second, so `[from, to)` includes the current instant.
pub fn now_plus() -> i64 {
    pos_domain::now_unix() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_buffer_rejects_slow_typing() {
        let mut b = ScanBuffer::default();
        // Fast keys: 10ms apart.
        b.push_char('6', 0.000, 30);
        b.push_char('1', 0.010, 30);
        b.push_char('3', 0.020, 30);
        assert_eq!(b.current(), "613");
        assert_eq!(b.take().as_deref(), None); // too short

        // Slow key (200ms gap) clears the buffer.
        b.push_char('6', 1.000, 30);
        b.push_char('1', 1.010, 30);
        assert_eq!(b.current(), "61");
        b.push_char('x', 1.500, 30);
        assert_eq!(b.current(), "x");
    }

    #[test]
    fn scan_buffer_returns_long_codes() {
        let mut b = ScanBuffer::default();
        for (i, c) in "6130000000019".chars().enumerate() {
            b.push_char(c, i as f64 * 0.005, 30);
        }
        assert_eq!(b.take().as_deref(), Some("6130000000019"));
    }
}
