//! Blank Meter (placeholder/spacer)

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::Crt;

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

    fn draw(
        &self,
        _crt: &Crt,
        _machine: &Machine,
        _settings: &Settings,
        _x: i32,
        _y: i32,
        _width: i32,
    ) {
        // Draw nothing
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
    fn test_blank_meter_new() {
        let meter = BlankMeter::new();
        // new() uses default(), which defaults to Bar mode
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_blank_meter_default() {
        let meter = BlankMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_blank_meter_name() {
        let meter = BlankMeter::new();
        assert_eq!(meter.name(), "Blank");
    }

    #[test]
    fn test_blank_meter_caption() {
        let meter = BlankMeter::new();
        assert_eq!(meter.caption(), "");
    }

    #[test]
    fn test_blank_meter_mode() {
        let mut meter = BlankMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);
    }

    #[test]
    fn test_blank_meter_update_does_nothing() {
        let mut meter = BlankMeter::new();
        let machine = Machine::default();
        // Update should not panic and should not change state
        meter.update(&machine);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }
}
