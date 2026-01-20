//! Date Meter
//!
//! Displays the current date in YYYY-MM-DD format.

use chrono::Local;

use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;
use super::{Meter, MeterMode};

/// Date Meter - displays the current date
#[derive(Debug, Default)]
pub struct DateMeter {
    mode: MeterMode,
    date_str: String,
}

impl DateMeter {
    pub fn new() -> Self {
        DateMeter::default()
    }
}

impl Meter for DateMeter {
    fn name(&self) -> &'static str {
        "Date"
    }

    fn caption(&self) -> &str {
        "Date: "
    }

    fn update(&mut self, _machine: &Machine) {
        // Get current local date
        let now = Local::now();
        self.date_str = now.format("%Y-%m-%d").to_string();
    }

    fn draw(&self, crt: &Crt, _machine: &Machine, _settings: &Settings, x: i32, y: i32, _width: i32) {
        use ncurses::*;

        let caption_attr = crt.color(ColorElement::MeterText);
        let value_attr = crt.color(ColorElement::Date);

        mv(y, x);
        attron(caption_attr);
        let _ = addstr("Date: ");
        attroff(caption_attr);

        attron(value_attr);
        let _ = addstr(&self.date_str);
        attroff(value_attr);
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}
