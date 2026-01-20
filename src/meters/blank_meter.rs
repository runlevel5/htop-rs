//! Blank Meter (placeholder/spacer)

use crate::core::{Machine, Settings};
use crate::ui::Crt;
use super::{Meter, MeterMode};

/// Blank Meter - just empty space
#[derive(Debug, Default)]
pub struct BlankMeter {
    mode: MeterMode,
}

impl BlankMeter {
    pub fn new() -> Self {
        BlankMeter::default()
    }
}

impl Meter for BlankMeter {
    fn name(&self) -> &'static str {
        "Blank"
    }

    fn caption(&self) -> &str {
        ""
    }

    fn update(&mut self, _machine: &Machine) {
        // Nothing to update
    }

    fn draw(&self, _crt: &Crt, _machine: &Machine, _settings: &Settings, _x: i32, _y: i32, _width: i32) {
        // Draw nothing
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}
