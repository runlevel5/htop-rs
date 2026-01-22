//! Load Average Meter

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Load Average Meter
///
/// Displays load averages exactly like C htop:
/// "Load average: X.XX Y.YY Z.ZZ"
/// with each value in a different color
#[derive(Debug, Default)]
pub struct LoadAverageMeter {
    mode: MeterMode,
    load1: f64,
    load5: f64,
    load15: f64,
}

impl LoadAverageMeter {
    pub fn new() -> Self {
        LoadAverageMeter::default()
    }
}

impl Meter for LoadAverageMeter {
    fn name(&self) -> &'static str {
        "LoadAverage"
    }

    fn caption(&self) -> &str {
        "Load average: "
    }

    fn update(&mut self, machine: &Machine) {
        self.load1 = machine.load_average[0];
        self.load5 = machine.load_average[1];
        self.load15 = machine.load_average[2];
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
        let load1_attr = crt.color(ColorElement::LoadAverageOne);
        let load5_attr = crt.color(ColorElement::LoadAverageFive);
        let load15_attr = crt.color(ColorElement::LoadAverageFifteen);

        mv(y, x);

        // "Load average: "
        attrset(caption_attr);
        let _ = addstr("Load average: ");

        // 1-minute load (with trailing space)
        attrset(load1_attr);
        let _ = addstr(&format!("{:.2} ", self.load1));

        // 5-minute load (with trailing space)
        attrset(load5_attr);
        let _ = addstr(&format!("{:.2} ", self.load5));

        // 15-minute load (with trailing space like C htop)
        attrset(load15_attr);
        let _ = addstr(&format!("{:.2} ", self.load15));
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}
