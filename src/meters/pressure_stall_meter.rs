//! Pressure Stall Information (PSI) Meters
//!
//! Linux-specific meters for displaying CPU, IO, and memory pressure.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// Helper function to draw "Not implemented" text
fn draw_not_implemented(crt: &mut Crt, x: i32, y: i32, caption: &str) {
    let caption_attr = crt.color(ColorElement::MeterText);
    let text_attr = crt.color(ColorElement::MeterValueError);
    let reset_attr = crt.color(ColorElement::ResetColor);

    crt.with_window(|win| {
        let _ = win.mv(y, x);
        let _ = win.attrset(caption_attr);
        let _ = win.addstr(caption);
        let _ = win.attrset(text_attr);
        let _ = win.addstr("Not implemented");
        let _ = win.attrset(reset_attr);
    });
}

/// PressureStallCPUSome Meter - PSI some CPU
#[derive(Debug, Default)]
pub struct PressureStallCPUSomeMeter {
    mode: MeterMode,
}

impl PressureStallCPUSomeMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for PressureStallCPUSomeMeter {
    fn name(&self) -> &'static str {
        "PressureStallCPUSome"
    }

    fn caption(&self) -> &str {
        "PSI some CPU:    "
    }

    fn update(&mut self, _machine: &Machine) {}

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        draw_not_implemented(crt, x, y, self.caption());
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}

/// PressureStallIOSome Meter - PSI some IO
#[derive(Debug, Default)]
pub struct PressureStallIOSomeMeter {
    mode: MeterMode,
}

impl PressureStallIOSomeMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for PressureStallIOSomeMeter {
    fn name(&self) -> &'static str {
        "PressureStallIOSome"
    }

    fn caption(&self) -> &str {
        "PSI some IO:     "
    }

    fn update(&mut self, _machine: &Machine) {}

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        draw_not_implemented(crt, x, y, self.caption());
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}

/// PressureStallIOFull Meter - PSI full IO
#[derive(Debug, Default)]
pub struct PressureStallIOFullMeter {
    mode: MeterMode,
}

impl PressureStallIOFullMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for PressureStallIOFullMeter {
    fn name(&self) -> &'static str {
        "PressureStallIOFull"
    }

    fn caption(&self) -> &str {
        "PSI full IO:     "
    }

    fn update(&mut self, _machine: &Machine) {}

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        draw_not_implemented(crt, x, y, self.caption());
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}

/// PressureStallIRQFull Meter - PSI full IRQ
#[derive(Debug, Default)]
pub struct PressureStallIRQFullMeter {
    mode: MeterMode,
}

impl PressureStallIRQFullMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for PressureStallIRQFullMeter {
    fn name(&self) -> &'static str {
        "PressureStallIRQFull"
    }

    fn caption(&self) -> &str {
        "PSI full IRQ:    "
    }

    fn update(&mut self, _machine: &Machine) {}

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        draw_not_implemented(crt, x, y, self.caption());
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}

/// PressureStallMemorySome Meter - PSI some memory
#[derive(Debug, Default)]
pub struct PressureStallMemorySomeMeter {
    mode: MeterMode,
}

impl PressureStallMemorySomeMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for PressureStallMemorySomeMeter {
    fn name(&self) -> &'static str {
        "PressureStallMemorySome"
    }

    fn caption(&self) -> &str {
        "PSI some memory: "
    }

    fn update(&mut self, _machine: &Machine) {}

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        draw_not_implemented(crt, x, y, self.caption());
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}

/// PressureStallMemoryFull Meter - PSI full memory
#[derive(Debug, Default)]
pub struct PressureStallMemoryFullMeter {
    mode: MeterMode,
}

impl PressureStallMemoryFullMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
        }
    }
}

impl Meter for PressureStallMemoryFullMeter {
    fn name(&self) -> &'static str {
        "PressureStallMemoryFull"
    }

    fn caption(&self) -> &str {
        "PSI full memory: "
    }

    fn update(&mut self, _machine: &Machine) {}

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        draw_not_implemented(crt, x, y, self.caption());
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

    // PressureStallCPUSomeMeter tests
    #[test]
    fn test_psi_cpu_some_meter_new() {
        let meter = PressureStallCPUSomeMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_psi_cpu_some_meter_default() {
        let meter = PressureStallCPUSomeMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_psi_cpu_some_meter_name() {
        let meter = PressureStallCPUSomeMeter::new();
        assert_eq!(meter.name(), "PressureStallCPUSome");
    }

    #[test]
    fn test_psi_cpu_some_meter_caption() {
        let meter = PressureStallCPUSomeMeter::new();
        assert_eq!(meter.caption(), "PSI some CPU:    ");
    }

    #[test]
    fn test_psi_cpu_some_meter_mode() {
        let mut meter = PressureStallCPUSomeMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    // PressureStallIOSomeMeter tests
    #[test]
    fn test_psi_io_some_meter_new() {
        let meter = PressureStallIOSomeMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_psi_io_some_meter_default() {
        let meter = PressureStallIOSomeMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_psi_io_some_meter_name() {
        let meter = PressureStallIOSomeMeter::new();
        assert_eq!(meter.name(), "PressureStallIOSome");
    }

    #[test]
    fn test_psi_io_some_meter_caption() {
        let meter = PressureStallIOSomeMeter::new();
        assert_eq!(meter.caption(), "PSI some IO:     ");
    }

    #[test]
    fn test_psi_io_some_meter_mode() {
        let mut meter = PressureStallIOSomeMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    // PressureStallIOFullMeter tests
    #[test]
    fn test_psi_io_full_meter_new() {
        let meter = PressureStallIOFullMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_psi_io_full_meter_default() {
        let meter = PressureStallIOFullMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_psi_io_full_meter_name() {
        let meter = PressureStallIOFullMeter::new();
        assert_eq!(meter.name(), "PressureStallIOFull");
    }

    #[test]
    fn test_psi_io_full_meter_caption() {
        let meter = PressureStallIOFullMeter::new();
        assert_eq!(meter.caption(), "PSI full IO:     ");
    }

    #[test]
    fn test_psi_io_full_meter_mode() {
        let mut meter = PressureStallIOFullMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    // PressureStallIRQFullMeter tests
    #[test]
    fn test_psi_irq_full_meter_new() {
        let meter = PressureStallIRQFullMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_psi_irq_full_meter_default() {
        let meter = PressureStallIRQFullMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_psi_irq_full_meter_name() {
        let meter = PressureStallIRQFullMeter::new();
        assert_eq!(meter.name(), "PressureStallIRQFull");
    }

    #[test]
    fn test_psi_irq_full_meter_caption() {
        let meter = PressureStallIRQFullMeter::new();
        assert_eq!(meter.caption(), "PSI full IRQ:    ");
    }

    #[test]
    fn test_psi_irq_full_meter_mode() {
        let mut meter = PressureStallIRQFullMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    // PressureStallMemorySomeMeter tests
    #[test]
    fn test_psi_memory_some_meter_new() {
        let meter = PressureStallMemorySomeMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_psi_memory_some_meter_default() {
        let meter = PressureStallMemorySomeMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_psi_memory_some_meter_name() {
        let meter = PressureStallMemorySomeMeter::new();
        assert_eq!(meter.name(), "PressureStallMemorySome");
    }

    #[test]
    fn test_psi_memory_some_meter_caption() {
        let meter = PressureStallMemorySomeMeter::new();
        assert_eq!(meter.caption(), "PSI some memory: ");
    }

    #[test]
    fn test_psi_memory_some_meter_mode() {
        let mut meter = PressureStallMemorySomeMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    // PressureStallMemoryFullMeter tests
    #[test]
    fn test_psi_memory_full_meter_new() {
        let meter = PressureStallMemoryFullMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_psi_memory_full_meter_default() {
        let meter = PressureStallMemoryFullMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_psi_memory_full_meter_name() {
        let meter = PressureStallMemoryFullMeter::new();
        assert_eq!(meter.name(), "PressureStallMemoryFull");
    }

    #[test]
    fn test_psi_memory_full_meter_caption() {
        let meter = PressureStallMemoryFullMeter::new();
        assert_eq!(meter.caption(), "PSI full memory: ");
    }

    #[test]
    fn test_psi_memory_full_meter_mode() {
        let mut meter = PressureStallMemoryFullMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    // Test update functions don't panic
    #[test]
    fn test_psi_meters_update_does_nothing() {
        let machine = Machine::default();
        let mut cpu = PressureStallCPUSomeMeter::new();
        let mut io_some = PressureStallIOSomeMeter::new();
        let mut io_full = PressureStallIOFullMeter::new();
        let mut irq = PressureStallIRQFullMeter::new();
        let mut mem_some = PressureStallMemorySomeMeter::new();
        let mut mem_full = PressureStallMemoryFullMeter::new();

        cpu.update(&machine);
        io_some.update(&machine);
        io_full.update(&machine);
        irq.update(&machine);
        mem_some.update(&machine);
        mem_full.update(&machine);
    }
}
