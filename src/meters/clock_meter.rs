//! Clock Meter
//!
//! Displays the current time in HH:MM:SS format.

use chrono::Local;

use super::{draw_led, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

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

    fn supported_modes(&self) -> u32 {
        // Clock only supports Text and LED modes (no Bar or Graph)
        (1 << MeterMode::Text as u32) | (1 << MeterMode::Led as u32)
    }

    fn update(&mut self, _machine: &Machine) {
        // Get current local time
        let now = Local::now();
        self.time_str = now.format("%H:%M:%S").to_string();
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
                draw_led(crt, x, y, width, self.caption(), &self.time_str);
            }
            _ => {
                // Text mode (default)
                let caption_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::Clock);

                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("Time: ");

                attrset(value_attr);
                let _ = addstr(&self.time_str);
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
