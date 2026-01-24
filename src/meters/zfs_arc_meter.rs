//! ZFS ARC Meters
//!
//! Displays ZFS ARC statistics (available where ZFS is supported).

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// ZFS ARC Meter - displays ZFS Adaptive Replacement Cache statistics
#[derive(Debug, Default)]
pub struct ZfsArcMeter {
    mode: MeterMode,
}

impl ZfsArcMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Bar,
        }
    }
}

impl Meter for ZfsArcMeter {
    fn name(&self) -> &'static str {
        "ZFSARC"
    }

    fn caption(&self) -> &str {
        "ARC: "
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

/// ZFS Compressed ARC Meter - displays ZFS Compressed ARC statistics
#[derive(Debug, Default)]
pub struct ZfsCompressedArcMeter {
    mode: MeterMode,
}

impl ZfsCompressedArcMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Bar,
        }
    }
}

impl Meter for ZfsCompressedArcMeter {
    fn name(&self) -> &'static str {
        "ZFSCARC"
    }

    fn caption(&self) -> &str {
        "ARC: "
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

    // ZfsArcMeter tests
    #[test]
    fn test_zfs_arc_meter_new() {
        let meter = ZfsArcMeter::new();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_zfs_arc_meter_default() {
        let meter = ZfsArcMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_zfs_arc_meter_name() {
        let meter = ZfsArcMeter::new();
        assert_eq!(meter.name(), "ZFSARC");
    }

    #[test]
    fn test_zfs_arc_meter_caption() {
        let meter = ZfsArcMeter::new();
        assert_eq!(meter.caption(), "ARC: ");
    }

    #[test]
    fn test_zfs_arc_meter_mode() {
        let mut meter = ZfsArcMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);
    }

    #[test]
    fn test_zfs_arc_meter_update_does_nothing() {
        let mut meter = ZfsArcMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
    }

    // ZfsCompressedArcMeter tests
    #[test]
    fn test_zfs_compressed_arc_meter_new() {
        let meter = ZfsCompressedArcMeter::new();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_zfs_compressed_arc_meter_default() {
        let meter = ZfsCompressedArcMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_zfs_compressed_arc_meter_name() {
        let meter = ZfsCompressedArcMeter::new();
        assert_eq!(meter.name(), "ZFSCARC");
    }

    #[test]
    fn test_zfs_compressed_arc_meter_caption() {
        let meter = ZfsCompressedArcMeter::new();
        assert_eq!(meter.caption(), "ARC: ");
    }

    #[test]
    fn test_zfs_compressed_arc_meter_mode() {
        let mut meter = ZfsCompressedArcMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);
    }

    #[test]
    fn test_zfs_compressed_arc_meter_update_does_nothing() {
        let mut meter = ZfsCompressedArcMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
    }
}
