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
        DateTimeMeter {
            mode: MeterMode::Text,
            datetime_str: String::new(),
        }
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

    fn default_mode(&self) -> MeterMode {
        MeterMode::Text
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_meter_new() {
        let meter = DateTimeMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert!(meter.datetime_str.is_empty());
    }

    #[test]
    fn test_datetime_meter_default() {
        let meter = DateTimeMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_datetime_meter_name() {
        let meter = DateTimeMeter::new();
        assert_eq!(meter.name(), "DateTime");
    }

    #[test]
    fn test_datetime_meter_caption() {
        let meter = DateTimeMeter::new();
        assert_eq!(meter.caption(), "Date & Time: ");
    }

    #[test]
    fn test_datetime_meter_default_mode() {
        let meter = DateTimeMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }

    #[test]
    fn test_datetime_meter_supported_modes() {
        let meter = DateTimeMeter::new();
        let modes = meter.supported_modes();
        // Should support Text and Led modes
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        assert!(modes & (1 << MeterMode::Led as u32) != 0);
        // Should not support Bar or Graph modes
        assert!(modes & (1 << MeterMode::Bar as u32) == 0);
        assert!(modes & (1 << MeterMode::Graph as u32) == 0);
    }

    #[test]
    fn test_datetime_meter_mode() {
        let mut meter = DateTimeMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_datetime_meter_update() {
        let mut meter = DateTimeMeter::new();
        let machine = Machine::default();

        assert!(meter.datetime_str.is_empty());
        meter.update(&machine);

        // After update, datetime_str should be in "YYYY-MM-DD HH:MM:SS" format
        assert!(!meter.datetime_str.is_empty());
        assert_eq!(meter.datetime_str.len(), 19); // "YYYY-MM-DD HH:MM:SS" = 19 chars
    }

    #[test]
    fn test_datetime_meter_format() {
        let mut meter = DateTimeMeter::new();
        let machine = Machine::default();
        meter.update(&machine);

        // Split into date and time parts
        let parts: Vec<&str> = meter.datetime_str.split(' ').collect();
        assert_eq!(parts.len(), 2);

        // Validate date part (YYYY-MM-DD)
        let date_parts: Vec<&str> = parts[0].split('-').collect();
        assert_eq!(date_parts.len(), 3);

        let year: u32 = date_parts[0].parse().expect("Invalid year");
        let month: u32 = date_parts[1].parse().expect("Invalid month");
        let day: u32 = date_parts[2].parse().expect("Invalid day");

        assert!(year >= 2020 && year <= 2100, "Year should be reasonable");
        assert!(month >= 1 && month <= 12, "Month should be 1-12");
        assert!(day >= 1 && day <= 31, "Day should be 1-31");

        // Validate time part (HH:MM:SS)
        let time_parts: Vec<&str> = parts[1].split(':').collect();
        assert_eq!(time_parts.len(), 3);

        let hour: u32 = time_parts[0].parse().expect("Invalid hour");
        let minute: u32 = time_parts[1].parse().expect("Invalid minute");
        let second: u32 = time_parts[2].parse().expect("Invalid second");

        assert!(hour < 24, "Hour should be less than 24");
        assert!(minute < 60, "Minute should be less than 60");
        assert!(second < 60, "Second should be less than 60");
    }
}
