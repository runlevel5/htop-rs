//! Systemd Meters
//!
//! Displays Systemd state (Linux-specific).

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// Systemd Meter - displays Systemd system state and unit overview
#[derive(Debug, Default)]
pub struct SystemdMeter {
    mode: MeterMode,
}

impl SystemdMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for SystemdMeter {
    fn name(&self) -> &'static str {
        "Systemd"
    }

    fn caption(&self) -> &str {
        "Systemd: "
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

/// SystemdUser Meter - displays Systemd user state and unit overview
#[derive(Debug, Default)]
pub struct SystemdUserMeter {
    mode: MeterMode,
}

impl SystemdUserMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for SystemdUserMeter {
    fn name(&self) -> &'static str {
        "SystemdUser"
    }

    fn caption(&self) -> &str {
        "Systemd User: "
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
