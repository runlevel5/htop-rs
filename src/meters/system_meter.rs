//! System Meter
//!
//! Displays system information.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// System Meter - displays system information
#[derive(Debug, Default)]
pub struct SystemMeter {
    mode: MeterMode,
}

impl SystemMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for SystemMeter {
    fn name(&self) -> &'static str {
        "System"
    }

    fn caption(&self) -> &str {
        "System: "
    }

    fn supported_modes(&self) -> u32 {
        1 << MeterMode::Text as u32
    }

    fn default_mode(&self) -> MeterMode {
        MeterMode::Text
    }

    fn update(&mut self, _machine: &Machine) {
        // Not yet implemented
    }

    fn draw(
        &self,
        crt: &Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        use ncurses::*;

        let caption_attr = crt.color(ColorElement::MeterText);
        let text_attr = crt.color(ColorElement::MeterValueError);

        mv(y, x);
        attrset(caption_attr);
        let _ = addstr(self.caption());
        attrset(text_attr);
        let _ = addstr("Not implemented");
        attrset(crt.color(ColorElement::ResetColor));
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
    fn test_system_meter_new() {
        let meter = SystemMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_system_meter_default() {
        let meter = SystemMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_system_meter_name() {
        let meter = SystemMeter::new();
        assert_eq!(meter.name(), "System");
    }

    #[test]
    fn test_system_meter_caption() {
        let meter = SystemMeter::new();
        assert_eq!(meter.caption(), "System: ");
    }

    #[test]
    fn test_system_meter_default_mode() {
        let meter = SystemMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }

    #[test]
    fn test_system_meter_supported_modes() {
        let meter = SystemMeter::new();
        let modes = meter.supported_modes();
        // Should support only Text mode
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        // Should not support Led, Bar or Graph modes
        assert!(modes & (1 << MeterMode::Led as u32) == 0);
        assert!(modes & (1 << MeterMode::Bar as u32) == 0);
        assert!(modes & (1 << MeterMode::Graph as u32) == 0);
    }

    #[test]
    fn test_system_meter_mode() {
        let mut meter = SystemMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    #[test]
    fn test_system_meter_update_does_nothing() {
        let mut meter = SystemMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
        assert_eq!(meter.mode(), MeterMode::Text);
    }
}
