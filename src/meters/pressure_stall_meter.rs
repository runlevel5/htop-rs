//! Pressure Stall Information (PSI) Meters
//!
//! Linux-specific meters for displaying CPU, IO, and memory pressure.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// Helper function to draw "Not implemented" text
fn draw_not_implemented(crt: &Crt, x: i32, y: i32, caption: &str) {
    use ncurses::*;

    let caption_attr = crt.color(ColorElement::MeterText);
    let text_attr = crt.color(ColorElement::MeterValueError);

    mv(y, x);
    attrset(caption_attr);
    let _ = addstr(caption);
    attrset(text_attr);
    let _ = addstr("Not implemented");
    attrset(crt.color(ColorElement::ResetColor));
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
        crt: &Crt,
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
        crt: &Crt,
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
        crt: &Crt,
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
        crt: &Crt,
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
        crt: &Crt,
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
        crt: &Crt,
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
