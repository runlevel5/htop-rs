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
        DateMeter {
            mode: MeterMode::Text,
            date_str: String::new(),
        }
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

    fn default_mode(&self) -> MeterMode {
        MeterMode::Text
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
                draw_led(crt, x, y, width, self.caption(), &self.date_str);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_meter_new() {
        let meter = DateMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert!(meter.date_str.is_empty());
    }

    #[test]
    fn test_date_meter_default() {
        let meter = DateMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_date_meter_name() {
        let meter = DateMeter::new();
        assert_eq!(meter.name(), "Date");
    }

    #[test]
    fn test_date_meter_caption() {
        let meter = DateMeter::new();
        assert_eq!(meter.caption(), "Date: ");
    }

    #[test]
    fn test_date_meter_default_mode() {
        let meter = DateMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }

    #[test]
    fn test_date_meter_supported_modes() {
        let meter = DateMeter::new();
        let modes = meter.supported_modes();
        // Should support Text and Led modes
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        assert!(modes & (1 << MeterMode::Led as u32) != 0);
        // Should not support Bar or Graph modes
        assert!(modes & (1 << MeterMode::Bar as u32) == 0);
        assert!(modes & (1 << MeterMode::Graph as u32) == 0);
    }

    #[test]
    fn test_date_meter_mode() {
        let mut meter = DateMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_date_meter_update() {
        let mut meter = DateMeter::new();
        let machine = Machine::default();

        assert!(meter.date_str.is_empty());
        meter.update(&machine);

        // After update, date_str should be in YYYY-MM-DD format
        assert!(!meter.date_str.is_empty());
        assert_eq!(meter.date_str.len(), 10); // "YYYY-MM-DD" = 10 chars
        assert!(meter.date_str.chars().nth(4) == Some('-'));
        assert!(meter.date_str.chars().nth(7) == Some('-'));
    }

    #[test]
    fn test_date_meter_date_format() {
        let mut meter = DateMeter::new();
        let machine = Machine::default();
        meter.update(&machine);

        // Parse the date to validate format
        let parts: Vec<&str> = meter.date_str.split('-').collect();
        assert_eq!(parts.len(), 3);

        let year: u32 = parts[0].parse().expect("Invalid year");
        let month: u32 = parts[1].parse().expect("Invalid month");
        let day: u32 = parts[2].parse().expect("Invalid day");

        assert!(year >= 2020 && year <= 2100, "Year should be reasonable");
        assert!(month >= 1 && month <= 12, "Month should be 1-12");
        assert!(day >= 1 && day <= 31, "Day should be 1-31");
    }
}
