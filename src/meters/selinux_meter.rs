//! SELinux Meter
//!
//! Displays SELinux state (Linux-specific).

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// SELinux Meter - displays SELinux state overview
#[derive(Debug, Default)]
pub struct SELinuxMeter {
    mode: MeterMode,
}

impl SELinuxMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for SELinuxMeter {
    fn name(&self) -> &'static str {
        "SELinux"
    }

    fn caption(&self) -> &str {
        "SELinux: "
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
    fn test_selinux_meter_new() {
        let meter = SELinuxMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_selinux_meter_default() {
        let meter = SELinuxMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_selinux_meter_name() {
        let meter = SELinuxMeter::new();
        assert_eq!(meter.name(), "SELinux");
    }

    #[test]
    fn test_selinux_meter_caption() {
        let meter = SELinuxMeter::new();
        assert_eq!(meter.caption(), "SELinux: ");
    }

    #[test]
    fn test_selinux_meter_default_mode() {
        let meter = SELinuxMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }

    #[test]
    fn test_selinux_meter_supported_modes() {
        let meter = SELinuxMeter::new();
        let modes = meter.supported_modes();
        // Should support only Text mode
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        // Should not support Led, Bar or Graph modes
        assert!(modes & (1 << MeterMode::Led as u32) == 0);
        assert!(modes & (1 << MeterMode::Bar as u32) == 0);
        assert!(modes & (1 << MeterMode::Graph as u32) == 0);
    }

    #[test]
    fn test_selinux_meter_mode() {
        let mut meter = SELinuxMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    #[test]
    fn test_selinux_meter_update_does_nothing() {
        let mut meter = SELinuxMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
    }
}
