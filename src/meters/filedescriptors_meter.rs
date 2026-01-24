//! FileDescriptors Meter
//!
//! Displays number of allocated/available file descriptors.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// FileDescriptors Meter - displays allocated/available file descriptors
#[derive(Debug, Default)]
pub struct FileDescriptorsMeter {
    mode: MeterMode,
}

impl FileDescriptorsMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for FileDescriptorsMeter {
    fn name(&self) -> &'static str {
        "FileDescriptors"
    }

    fn caption(&self) -> &str {
        "FDs: "
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
        use crate::ncurses_compat::*;

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
    fn test_filedescriptors_meter_new() {
        let meter = FileDescriptorsMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_filedescriptors_meter_default() {
        let meter = FileDescriptorsMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_filedescriptors_meter_name() {
        let meter = FileDescriptorsMeter::new();
        assert_eq!(meter.name(), "FileDescriptors");
    }

    #[test]
    fn test_filedescriptors_meter_caption() {
        let meter = FileDescriptorsMeter::new();
        assert_eq!(meter.caption(), "FDs: ");
    }

    #[test]
    fn test_filedescriptors_meter_mode() {
        let mut meter = FileDescriptorsMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    #[test]
    fn test_filedescriptors_meter_update_does_nothing() {
        let mut meter = FileDescriptorsMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
    }
}
