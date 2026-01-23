//! DiskIO Rate Meter
//!
//! Displays disk IO read/write bytes per second.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// DiskIO Rate Meter - displays disk IO read/write bytes per second
#[derive(Debug, Default)]
pub struct DiskIORateMeter {
    mode: MeterMode,
}

impl DiskIORateMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for DiskIORateMeter {
    fn name(&self) -> &'static str {
        "DiskIORate"
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
