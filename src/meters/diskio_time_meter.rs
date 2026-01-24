//! DiskIO Time Meter
//!
//! Displays disk percent time busy.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// DiskIO Time Meter - displays disk percent time busy
#[derive(Debug, Default)]
pub struct DiskIOTimeMeter {
    mode: MeterMode,
}

impl DiskIOTimeMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for DiskIOTimeMeter {
    fn name(&self) -> &'static str {
        "DiskIOTime"
    }

    fn caption(&self) -> &str {
        "Dsk: "
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
    fn test_diskio_time_meter_new() {
        let meter = DiskIOTimeMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_diskio_time_meter_default() {
        let meter = DiskIOTimeMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_diskio_time_meter_name() {
        let meter = DiskIOTimeMeter::new();
        assert_eq!(meter.name(), "DiskIOTime");
    }

    #[test]
    fn test_diskio_time_meter_caption() {
        let meter = DiskIOTimeMeter::new();
        assert_eq!(meter.caption(), "Dsk: ");
    }

    #[test]
    fn test_diskio_time_meter_mode() {
        let mut meter = DiskIOTimeMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    #[test]
    fn test_diskio_time_meter_update_does_nothing() {
        let mut meter = DiskIOTimeMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
    }
}
