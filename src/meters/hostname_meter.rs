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
        use ncurses::*;

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
