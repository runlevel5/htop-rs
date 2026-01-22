//! Uptime Meter

use std::time::Duration;

use super::{Meter, MeterMode};
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
        UptimeMeter::default()
    }

    /// Format uptime exactly like C htop:
    /// - "1 day, HH:MM:SS" for 1 day
    /// - "N days, HH:MM:SS" for N days
    /// - "N days(!), HH:MM:SS" for N > 100 days
    /// - "HH:MM:SS" for less than 1 day
    fn format_uptime(&self) -> String {
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
}

impl Meter for UptimeMeter {
    fn name(&self) -> &'static str {
        "Uptime"
    }

    fn caption(&self) -> &str {
        "Uptime: "
    }

    fn update(&mut self, machine: &Machine) {
        self.uptime = machine.uptime;
    }

    fn draw(
        &self,
        crt: &Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        use ncurses::*;

        let caption_attr = crt.color(ColorElement::MeterText);
        let value_attr = crt.color(ColorElement::Uptime);

        mv(y, x);
        attrset(caption_attr);
        let _ = addstr("Uptime: ");

        attrset(value_attr);
        let _ = addstr(&self.format_uptime());
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}
