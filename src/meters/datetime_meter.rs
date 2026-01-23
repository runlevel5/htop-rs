//! DateTime Meter
//!
//! Displays the current date and time in YYYY-MM-DD HH:MM:SS format.

use chrono::Local;

use super::{draw_led, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

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
        "Date & Time: "
    }

    fn supported_modes(&self) -> u32 {
        // DateTime only supports Text and LED modes (no Bar or Graph)
        (1 << MeterMode::Text as u32) | (1 << MeterMode::Led as u32)
    }

    fn update(&mut self, _machine: &Machine) {
        // Get current local date and time
        let now = Local::now();
        self.datetime_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
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
                draw_led(crt, x, y, width, self.caption(), &self.datetime_str);
            }
            _ => {
                // Text mode (default)
                // Draw caption first with MeterText color
                let caption_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::DateTime);

                mv(y, x);
                attrset(caption_attr);
                let _ = addstr(self.caption());
                attrset(value_attr);
                let _ = addstr(&self.datetime_str);
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
