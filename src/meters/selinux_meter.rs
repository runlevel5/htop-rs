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
