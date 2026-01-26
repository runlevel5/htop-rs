//! Zram Meter
//!
//! Displays Zram usage (Linux-specific).

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// Zram Meter - displays Zram compressed memory usage
#[derive(Debug, Default)]
pub struct ZramMeter {
    mode: MeterMode,
}

impl ZramMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Bar,
        }
    }
}

impl Meter for ZramMeter {
    fn name(&self) -> &'static str {
        "Zram"
    }

    fn caption(&self) -> &str {
        "zrm: "
    }

    fn update(&mut self, _machine: &Machine) {
        // Not yet implemented
    }

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        let caption_attr = crt.color(ColorElement::MeterText);
        let text_attr = crt.color(ColorElement::MeterValueError);
        let reset_attr = crt.color(ColorElement::ResetColor);
        let caption = self.caption();

        crt.with_window(|win| {
            let _ = win.mv(y, x);
            let _ = win.attrset(caption_attr);
            let _ = win.addstr(caption);
            let _ = win.attrset(text_attr);
            let _ = win.addstr("Not implemented");
            let _ = win.attrset(reset_attr);
        });
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
    fn test_zram_meter_new() {
        let meter = ZramMeter::new();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_zram_meter_default() {
        let meter = ZramMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_zram_meter_name() {
        let meter = ZramMeter::new();
        assert_eq!(meter.name(), "Zram");
    }

    #[test]
    fn test_zram_meter_caption() {
        let meter = ZramMeter::new();
        assert_eq!(meter.caption(), "zrm: ");
    }

    #[test]
    fn test_zram_meter_mode() {
        let mut meter = ZramMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);
    }

    #[test]
    fn test_zram_meter_update_does_nothing() {
        let mut meter = ZramMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
    }
}
