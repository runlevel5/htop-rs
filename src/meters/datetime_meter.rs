//! DateTime Meter
//!
//! Displays the current date and time in YYYY-MM-DD HH:MM:SS format.

use chrono::Local;

use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;
use super::{Meter, MeterMode};

/// DateTime Meter - displays the current date and time
#[derive(Debug, Default)]
pub struct DateTimeMeter {
    mode: MeterMode,
    datetime_str: String,
}

impl DateTimeMeter {
    pub fn new() -> Self {
        DateTimeMeter::default()
    }
}

impl Meter for DateTimeMeter {
    fn name(&self) -> &'static str {
        "DateTime"
    }

    fn caption(&self) -> &str {
        ""  // No caption, the datetime is self-explanatory
    }

    fn update(&mut self, _machine: &Machine) {
        // Get current local date and time
        let now = Local::now();
        self.datetime_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    }

    fn draw(&self, crt: &Crt, _machine: &Machine, _settings: &Settings, x: i32, y: i32, _width: i32) {
        use ncurses::*;

        let value_attr = crt.color(ColorElement::DateTime);

        mv(y, x);
        attron(value_attr);
        let _ = addstr(&self.datetime_str);
        attroff(value_attr);
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}
