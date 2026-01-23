//! Date Meter
//!
//! Displays the current date in YYYY-MM-DD format.

use chrono::Local;

use super::{draw_led, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

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

    fn supported_modes(&self) -> u32 {
        // Date only supports Text and LED modes (no Bar or Graph)
        (1 << MeterMode::Text as u32) | (1 << MeterMode::Led as u32)
    }

    fn update(&mut self, _machine: &Machine) {
        // Get current local date
        let now = Local::now();
        self.date_str = now.format("%Y-%m-%d").to_string();
    }

    fn draw(
        &self,
        crt: &Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        use ncurses::*;

        match self.mode {
            MeterMode::Led => {
                draw_led(crt, x, y, width, "", &self.date_str);
            }
            _ => {
                // Text mode (default)
                let caption_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::Date);

                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("Date: ");

                attrset(value_attr);
                let _ = addstr(&self.date_str);
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
