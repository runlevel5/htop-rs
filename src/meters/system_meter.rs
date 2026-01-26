//! System Meter
//!
//! Displays system information including kernel version, architecture, and OS name.
//! Format similar to C htop: "Linux 6.18.5-200.fc43.ppc64le [ppc64le] @ Fedora Linux 43"

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// System Meter - displays system information
#[derive(Debug)]
pub struct SystemMeter {
    mode: MeterMode,
    /// Static OS info (distro name, architecture) - doesn't change at runtime
    os_info: String,
}

impl Default for SystemMeter {
    fn default() -> Self {
        Self {
            mode: MeterMode::Bar, // MeterMode::default() is Bar
            os_info: String::new(),
        }
    }
}

impl SystemMeter {
    pub fn new() -> Self {
        let os_info = Self::build_os_info();
        Self {
            mode: MeterMode::Text,
            os_info,
        }
    }

    /// Build the static OS info string (distro and architecture)
    /// This is combined with kernel_version from Machine at draw time
    fn build_os_info() -> String {
        let info = os_info::get();

        let mut parts = Vec::new();

        // Add architecture if available
        if let Some(arch) = info.architecture() {
            parts.push(format!("[{}]", arch));
        }

        // OS type and version (e.g., "Fedora Linux 43")
        let os_string = info.to_string();
        if !os_string.is_empty() && os_string != "Unknown" {
            parts.push("@".to_string());
            parts.push(os_string);
        }

        parts.join(" ")
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
        // OS info is static, initialize if empty
        if self.os_info.is_empty() {
            self.os_info = Self::build_os_info();
        }
    }

    fn draw(
        &self,
        crt: &mut Crt,
        machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        let caption_attr = crt.color(ColorElement::MeterText);
        let value_attr = crt.color(ColorElement::Hostname); // Same color as hostname

        crt.with_window(|win| {
            let _ = win.mv(y, x);
            let _ = win.attrset(caption_attr);
            let _ = win.addstr("System: ");

            let _ = win.attrset(value_attr);
            // Format: "Linux 6.18.5-200.fc43.ppc64le [ppc64le] @ Fedora Linux 43"
            // kernel_version already includes "Linux " or "Darwin " prefix
            let _ = win.addstr(&machine.kernel_version);
            let _ = win.addstr(" ");
            let _ = win.addstr(&self.os_info);
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
    fn test_system_meter_new() {
        let meter = SystemMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        // os_info should be populated
        assert!(!meter.os_info.is_empty());
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
    fn test_system_meter_update() {
        let mut meter = SystemMeter::new();
        let machine = Machine::default();
        // Update should not panic and os_info should remain populated
        meter.update(&machine);
        assert!(!meter.os_info.is_empty());
    }

    #[test]
    fn test_os_info_contains_architecture() {
        let meter = SystemMeter::new();
        // os_info should contain architecture in brackets
        assert!(meter.os_info.contains('[') && meter.os_info.contains(']'));
    }

    #[test]
    fn test_os_info_contains_distro() {
        let meter = SystemMeter::new();
        // os_info should contain @ separator and distro name
        assert!(meter.os_info.contains('@'));
    }
}
