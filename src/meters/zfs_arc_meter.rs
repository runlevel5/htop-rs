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
