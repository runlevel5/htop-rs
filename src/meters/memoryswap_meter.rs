//! MemorySwap Meter
//!
//! Combined memory and swap usage display.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// MemorySwap Meter - displays combined memory and swap usage
#[derive(Debug, Default)]
pub struct MemorySwapMeter {
    mode: MeterMode,
}

impl MemorySwapMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Bar,
        }
    }
}

impl Meter for MemorySwapMeter {
    fn name(&self) -> &'static str {
        "MemorySwap"
    }

    fn caption(&self) -> &str {
        "M&S: "
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
    fn test_memoryswap_meter_new() {
        let meter = MemorySwapMeter::new();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_memoryswap_meter_default() {
        let meter = MemorySwapMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_memoryswap_meter_name() {
        let meter = MemorySwapMeter::new();
        assert_eq!(meter.name(), "MemorySwap");
    }

    #[test]
    fn test_memoryswap_meter_caption() {
        let meter = MemorySwapMeter::new();
        assert_eq!(meter.caption(), "M&S: ");
    }

    #[test]
    fn test_memoryswap_meter_mode() {
        let mut meter = MemorySwapMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);
    }

    #[test]
    fn test_memoryswap_meter_update_does_nothing() {
        let mut meter = MemorySwapMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
    }
}
