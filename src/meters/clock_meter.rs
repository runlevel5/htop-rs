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
        ClockMeter {
            mode: MeterMode::Text,
            time_str: String::new(),
        }
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

    fn default_mode(&self) -> MeterMode {
        MeterMode::Text
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_meter_new() {
        let meter = ClockMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert!(meter.time_str.is_empty());
    }

    #[test]
    fn test_clock_meter_default() {
        let meter = ClockMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_clock_meter_name() {
        let meter = ClockMeter::new();
        assert_eq!(meter.name(), "Clock");
    }

    #[test]
    fn test_clock_meter_caption() {
        let meter = ClockMeter::new();
        assert_eq!(meter.caption(), "Time: ");
    }

    #[test]
    fn test_clock_meter_default_mode() {
        let meter = ClockMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }

    #[test]
    fn test_clock_meter_supported_modes() {
        let meter = ClockMeter::new();
        let modes = meter.supported_modes();
        // Should support Text and Led modes
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        assert!(modes & (1 << MeterMode::Led as u32) != 0);
        // Should not support Bar or Graph modes
        assert!(modes & (1 << MeterMode::Bar as u32) == 0);
        assert!(modes & (1 << MeterMode::Graph as u32) == 0);
    }

    #[test]
    fn test_clock_meter_mode() {
        let mut meter = ClockMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_clock_meter_update() {
        let mut meter = ClockMeter::new();
        let machine = Machine::default();

        assert!(meter.time_str.is_empty());
        meter.update(&machine);

        // After update, time_str should be in HH:MM:SS format
        assert!(!meter.time_str.is_empty());
        assert_eq!(meter.time_str.len(), 8); // "HH:MM:SS" = 8 chars
        assert!(meter.time_str.chars().nth(2) == Some(':'));
        assert!(meter.time_str.chars().nth(5) == Some(':'));
    }

    #[test]
    fn test_clock_meter_time_format() {
        let mut meter = ClockMeter::new();
        let machine = Machine::default();
        meter.update(&machine);

        // Parse the time to validate format
        let parts: Vec<&str> = meter.time_str.split(':').collect();
        assert_eq!(parts.len(), 3);

        let hour: u32 = parts[0].parse().expect("Invalid hour");
        let minute: u32 = parts[1].parse().expect("Invalid minute");
        let second: u32 = parts[2].parse().expect("Invalid second");

        assert!(hour < 24, "Hour should be less than 24");
        assert!(minute < 60, "Minute should be less than 60");
        assert!(second < 60, "Second should be less than 60");
    }
}
