//! Hostname Meter
//!
//! Displays the system hostname.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Hostname Meter - displays the system hostname
#[derive(Debug, Default)]
pub struct HostnameMeter {
    mode: MeterMode,
    hostname: String,
}

impl HostnameMeter {
    pub fn new() -> Self {
        HostnameMeter {
            mode: MeterMode::Text,
            hostname: String::new(),
        }
    }
}

impl Meter for HostnameMeter {
    fn name(&self) -> &'static str {
        "Hostname"
    }

    fn caption(&self) -> &str {
        "Hostname: "
    }

    fn supported_modes(&self) -> u32 {
        // Hostname only supports Text mode
        1 << MeterMode::Text as u32
    }

    fn default_mode(&self) -> MeterMode {
        MeterMode::Text
    }

    fn update(&mut self, machine: &Machine) {
        self.hostname = machine.hostname.clone();
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
        use crate::ncurses_compat::*;

        let caption_attr = crt.color(ColorElement::MeterText);
        let value_attr = crt.color(ColorElement::Hostname);

        mv(y, x);
        attrset(caption_attr);
        let _ = addstr("Hostname: ");

        attrset(value_attr);
        let _ = addstr(&self.hostname);
    }

    fn mode(&self) -> MeterMode {
        // Hostname only supports text mode
        MeterMode::Text
    }

    fn set_mode(&mut self, mode: MeterMode) {
        // Hostname always uses text mode
        self.mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hostname_meter_new() {
        let meter = HostnameMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert!(meter.hostname.is_empty());
    }

    #[test]
    fn test_hostname_meter_default() {
        let meter = HostnameMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_hostname_meter_name() {
        let meter = HostnameMeter::new();
        assert_eq!(meter.name(), "Hostname");
    }

    #[test]
    fn test_hostname_meter_caption() {
        let meter = HostnameMeter::new();
        assert_eq!(meter.caption(), "Hostname: ");
    }

    #[test]
    fn test_hostname_meter_default_mode() {
        let meter = HostnameMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }

    #[test]
    fn test_hostname_meter_supported_modes() {
        let meter = HostnameMeter::new();
        let modes = meter.supported_modes();
        // Should support only Text mode
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        // Should not support Led, Bar or Graph modes
        assert!(modes & (1 << MeterMode::Led as u32) == 0);
        assert!(modes & (1 << MeterMode::Bar as u32) == 0);
        assert!(modes & (1 << MeterMode::Graph as u32) == 0);
    }

    #[test]
    fn test_hostname_meter_mode_always_text() {
        let meter = HostnameMeter::new();
        // mode() always returns Text for HostnameMeter
        assert_eq!(meter.mode(), MeterMode::Text);
    }

    #[test]
    fn test_hostname_meter_set_mode() {
        let mut meter = HostnameMeter::new();
        // Even if we set a different mode, the internal mode field is changed
        // but mode() always returns Text
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Text);
    }

    #[test]
    fn test_hostname_meter_update() {
        let mut meter = HostnameMeter::new();
        let mut machine = Machine::default();
        machine.hostname = "test-host.local".to_string();

        assert!(meter.hostname.is_empty());
        meter.update(&machine);

        assert_eq!(meter.hostname, "test-host.local");
    }

    #[test]
    fn test_hostname_meter_update_empty_hostname() {
        let mut meter = HostnameMeter::new();
        let machine = Machine::default();

        meter.update(&machine);
        // Default Machine should have empty hostname
        assert!(meter.hostname.is_empty());
    }

    #[test]
    fn test_hostname_meter_update_replaces_hostname() {
        let mut meter = HostnameMeter::new();
        let mut machine = Machine::default();

        machine.hostname = "first-host".to_string();
        meter.update(&machine);
        assert_eq!(meter.hostname, "first-host");

        machine.hostname = "second-host".to_string();
        meter.update(&machine);
        assert_eq!(meter.hostname, "second-host");
    }
}
