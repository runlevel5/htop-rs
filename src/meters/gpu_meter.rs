//! GPU Meter
//!
//! Displays GPU usage statistics.
//! Available on Linux and macOS.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// GPU Meter - displays GPU usage
#[derive(Debug, Default)]
pub struct GpuMeter {
    mode: MeterMode,
}

impl GpuMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for GpuMeter {
    fn name(&self) -> &'static str {
        "GPU"
    }

    fn caption(&self) -> &str {
        "GPU: "
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_meter_new() {
        let meter = GpuMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_gpu_meter_default() {
        let meter = GpuMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_gpu_meter_name() {
        let meter = GpuMeter::new();
        assert_eq!(meter.name(), "GPU");
    }

    #[test]
    fn test_gpu_meter_caption() {
        let meter = GpuMeter::new();
        assert_eq!(meter.caption(), "GPU: ");
    }

    #[test]
    fn test_gpu_meter_default_mode() {
        let meter = GpuMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }

    #[test]
    fn test_gpu_meter_supported_modes() {
        let meter = GpuMeter::new();
        let modes = meter.supported_modes();
        // Should support only Text mode
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        // Should not support Led, Bar or Graph modes
        assert!(modes & (1 << MeterMode::Led as u32) == 0);
        assert!(modes & (1 << MeterMode::Bar as u32) == 0);
        assert!(modes & (1 << MeterMode::Graph as u32) == 0);
    }

    #[test]
    fn test_gpu_meter_mode() {
        let mut meter = GpuMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    #[test]
    fn test_gpu_meter_update_does_nothing() {
        let mut meter = GpuMeter::new();
        let machine = Machine::default();
        // Update is not yet implemented but should not panic
        meter.update(&machine);
    }
}
