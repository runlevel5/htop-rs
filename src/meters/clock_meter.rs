//! Clock Meter
//!
//! Displays the current time in HH:MM:SS format.

use chrono::Local;

use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;
use super::{Meter, MeterMode};

/// Clock Meter - displays the current time
#[derive(Debug, Default)]
pub struct ClockMeter {
    mode: MeterMode,
    time_str: String,
}

impl ClockMeter {
    pub fn new() -> Self {
        ClockMeter::default()
    }
}

impl Meter for ClockMeter {
    fn name(&self) -> &'static str {
        "Clock"
    }

    fn caption(&self) -> &str {
        "Time: "
    }

    fn update(&mut self, _machine: &Machine) {
        // Get current local time
        let now = Local::now();
        self.time_str = now.format("%H:%M:%S").to_string();
    }

    fn draw(&self, crt: &Crt, _machine: &Machine, _settings: &Settings, x: i32, y: i32, _width: i32) {
        use ncurses::*;

        let caption_attr = crt.color(ColorElement::MeterText);
        let value_attr = crt.color(ColorElement::Clock);

        mv(y, x);
        attron(caption_attr);
        let _ = addstr("Time: ");
        attroff(caption_attr);

        attron(value_attr);
        let _ = addstr(&self.time_str);
        attroff(value_attr);
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}
