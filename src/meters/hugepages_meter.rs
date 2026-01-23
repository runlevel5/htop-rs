//! HugePages Meter
//!
//! Displays HugePages usage (Linux-specific).

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// HugePages Meter - displays HugePages usage
#[derive(Debug, Default)]
pub struct HugePagesMeter {
    mode: MeterMode,
}

impl HugePagesMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Bar,
        }
    }
}

impl Meter for HugePagesMeter {
    fn name(&self) -> &'static str {
        "HugePages"
    }

    fn caption(&self) -> &str {
        "HP: "
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
