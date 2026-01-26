//! Uptime Meter

use std::time::Duration;

use super::{draw_led, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Uptime Meter
#[derive(Debug, Default)]
pub struct UptimeMeter {
    mode: MeterMode,
    uptime: Duration,
}

impl UptimeMeter {
    pub fn new() -> Self {
        UptimeMeter {
            mode: MeterMode::Text,
            uptime: Duration::default(),
        }
    }

    /// Format uptime exactly like C htop:
    /// - "1 day, HH:MM:SS" for 1 day
    /// - "N days, HH:MM:SS" for N days
    /// - "N days(!), HH:MM:SS" for N > 100 days
    /// - "HH:MM:SS" for less than 1 day
    pub(crate) fn format_uptime(&self) -> String {
        let total_secs = self.uptime.as_secs();
        if total_secs == 0 {
            return "(unknown)".to_string();
        }

        let seconds = total_secs % 60;
        let minutes = (total_secs / 60) % 60;
        let hours = (total_secs / 3600) % 24;
        let days = total_secs / 86400;

        let days_str = if days > 100 {
            format!("{} days(!), ", days)
        } else if days > 1 {
            format!("{} days, ", days)
        } else if days == 1 {
            "1 day, ".to_string()
        } else {
            String::new()
        };

        format!("{}{:02}:{:02}:{:02}", days_str, hours, minutes, seconds)
    }

    /// Format uptime for LED mode (compact format for LED digits)
    /// Shows days as "N days, " prefix if > 0, then HH:MM:SS
    pub(crate) fn format_uptime_led(&self) -> String {
        let total_secs = self.uptime.as_secs();
        if total_secs == 0 {
            return "00:00:00".to_string();
        }

        let seconds = total_secs % 60;
        let minutes = (total_secs / 60) % 60;
        let hours = (total_secs / 3600) % 24;
        let days = total_secs / 86400;

        if days > 1 {
            format!("{} days, {:02}:{:02}:{:02}", days, hours, minutes, seconds)
        } else if days == 1 {
            format!("1 day, {:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        }
    }
}

impl Meter for UptimeMeter {
    fn name(&self) -> &'static str {
        "Uptime"
    }

    fn caption(&self) -> &str {
        "Uptime: "
    }

    fn supported_modes(&self) -> u32 {
        // Uptime only supports Text and LED modes (no Bar or Graph)
        (1 << MeterMode::Text as u32) | (1 << MeterMode::Led as u32)
    }

    fn default_mode(&self) -> MeterMode {
        MeterMode::Text
    }

    fn update(&mut self, machine: &Machine) {
        self.uptime = machine.uptime;
    }

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        match self.mode {
            MeterMode::Led => {
                let text = self.format_uptime_led();
                draw_led(crt, x, y, width, "Uptime: ", &text);
            }
            _ => {
                // Text mode (default) - using native ncurses-rs Window methods
                let caption_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::Uptime);

                crt.with_window(|win| {
                    let _ = win.mv(y, x);
                    let _ = win.attrset(caption_attr);
                    let _ = win.addstr("Uptime: ");

                    let _ = win.attrset(value_attr);
                    let _ = win.addstr(&self.format_uptime());
                });
            }
        }
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Machine;

    // ==================== Constructor Tests ====================

    #[test]
    fn test_uptime_meter_new() {
        let meter = UptimeMeter::new();
        assert_eq!(meter.mode, MeterMode::Text); // Uptime defaults to Text
        assert_eq!(meter.uptime, Duration::default());
    }

    #[test]
    fn test_uptime_meter_default() {
        let meter = UptimeMeter::default();
        // Default trait uses MeterMode::default() which is Bar
        // but UptimeMeter::new() sets Text
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    // ==================== format_uptime Tests ====================

    #[test]
    fn test_format_uptime_zero() {
        let meter = UptimeMeter::new();
        assert_eq!(meter.format_uptime(), "(unknown)");
    }

    #[test]
    fn test_format_uptime_seconds_only() {
        let mut meter = UptimeMeter::new();
        meter.uptime = Duration::from_secs(45);
        assert_eq!(meter.format_uptime(), "00:00:45");
    }

    #[test]
    fn test_format_uptime_minutes_seconds() {
        let mut meter = UptimeMeter::new();
        meter.uptime = Duration::from_secs(5 * 60 + 30); // 5:30
        assert_eq!(meter.format_uptime(), "00:05:30");
    }

    #[test]
    fn test_format_uptime_hours_minutes_seconds() {
        let mut meter = UptimeMeter::new();
        meter.uptime = Duration::from_secs(3 * 3600 + 25 * 60 + 10); // 3:25:10
        assert_eq!(meter.format_uptime(), "03:25:10");
    }

    #[test]
    fn test_format_uptime_one_day() {
        let mut meter = UptimeMeter::new();
        // 1 day + 2 hours + 30 minutes + 15 seconds
        meter.uptime = Duration::from_secs(86400 + 2 * 3600 + 30 * 60 + 15);
        assert_eq!(meter.format_uptime(), "1 day, 02:30:15");
    }

    #[test]
    fn test_format_uptime_multiple_days() {
        let mut meter = UptimeMeter::new();
        // 5 days + 10 hours + 45 minutes + 30 seconds
        meter.uptime = Duration::from_secs(5 * 86400 + 10 * 3600 + 45 * 60 + 30);
        assert_eq!(meter.format_uptime(), "5 days, 10:45:30");
    }

    #[test]
    fn test_format_uptime_100_days_boundary() {
        let mut meter = UptimeMeter::new();
        // Exactly 100 days
        meter.uptime = Duration::from_secs(100 * 86400);
        assert_eq!(meter.format_uptime(), "100 days, 00:00:00");
    }

    #[test]
    fn test_format_uptime_over_100_days() {
        let mut meter = UptimeMeter::new();
        // 101 days + 5 hours (should show "!")
        meter.uptime = Duration::from_secs(101 * 86400 + 5 * 3600);
        assert_eq!(meter.format_uptime(), "101 days(!), 05:00:00");
    }

    #[test]
    fn test_format_uptime_365_days() {
        let mut meter = UptimeMeter::new();
        meter.uptime = Duration::from_secs(365 * 86400);
        assert_eq!(meter.format_uptime(), "365 days(!), 00:00:00");
    }

    // ==================== format_uptime_led Tests ====================

    #[test]
    fn test_format_uptime_led_zero() {
        let meter = UptimeMeter::new();
        assert_eq!(meter.format_uptime_led(), "00:00:00");
    }

    #[test]
    fn test_format_uptime_led_hours_minutes_seconds() {
        let mut meter = UptimeMeter::new();
        meter.uptime = Duration::from_secs(12 * 3600 + 34 * 60 + 56);
        assert_eq!(meter.format_uptime_led(), "12:34:56");
    }

    #[test]
    fn test_format_uptime_led_one_day() {
        let mut meter = UptimeMeter::new();
        meter.uptime = Duration::from_secs(86400 + 5 * 3600 + 30 * 60);
        assert_eq!(meter.format_uptime_led(), "1 day, 05:30:00");
    }

    #[test]
    fn test_format_uptime_led_multiple_days() {
        let mut meter = UptimeMeter::new();
        meter.uptime = Duration::from_secs(7 * 86400 + 23 * 3600 + 59 * 60 + 59);
        assert_eq!(meter.format_uptime_led(), "7 days, 23:59:59");
    }

    #[test]
    fn test_format_uptime_led_no_exclamation_for_long_uptime() {
        // LED format doesn't use "!" for > 100 days
        let mut meter = UptimeMeter::new();
        meter.uptime = Duration::from_secs(200 * 86400);
        assert_eq!(meter.format_uptime_led(), "200 days, 00:00:00");
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_uptime_meter_update() {
        let mut meter = UptimeMeter::new();
        let mut machine = Machine::default();

        machine.uptime = Duration::from_secs(123456);

        meter.update(&machine);

        assert_eq!(meter.uptime, Duration::from_secs(123456));
    }

    // ==================== Meter Trait Tests ====================

    #[test]
    fn test_uptime_meter_name() {
        let meter = UptimeMeter::new();
        assert_eq!(meter.name(), "Uptime");
    }

    #[test]
    fn test_uptime_meter_caption() {
        let meter = UptimeMeter::new();
        assert_eq!(meter.caption(), "Uptime: ");
    }

    #[test]
    fn test_uptime_meter_mode() {
        let mut meter = UptimeMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_uptime_meter_supported_modes() {
        let meter = UptimeMeter::new();
        let modes = meter.supported_modes();

        // Uptime only supports Text and LED modes
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        assert!(modes & (1 << MeterMode::Led as u32) != 0);
        assert!(modes & (1 << MeterMode::Bar as u32) == 0);
        assert!(modes & (1 << MeterMode::Graph as u32) == 0);
    }

    #[test]
    fn test_uptime_meter_default_mode() {
        let meter = UptimeMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }
}
