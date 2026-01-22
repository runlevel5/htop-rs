//! Meters module
//!
//! This module contains the meter system for displaying system statistics
//! at the top of the screen.

#![allow(dead_code)]

mod battery_meter;
mod blank_meter;
mod clock_meter;
mod cpu_meter;
mod date_meter;
mod datetime_meter;
mod hostname_meter;
mod load_meter;
mod memory_meter;
mod swap_meter;
mod tasks_meter;
mod uptime_meter;

use crate::core::{Machine, Settings};
use crate::ui::Crt;

pub use battery_meter::*;
pub use blank_meter::*;
pub use clock_meter::*;
pub use cpu_meter::*;
pub use date_meter::*;
pub use datetime_meter::*;
pub use hostname_meter::*;
pub use load_meter::*;
pub use memory_meter::*;
pub use swap_meter::*;
pub use tasks_meter::*;
pub use uptime_meter::*;

/// Meter display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeterMode {
    #[default]
    Bar,
    Text,
    Graph,
    Led,
}

/// Meter trait - all meters implement this
pub trait Meter: std::fmt::Debug {
    /// Get the meter name
    fn name(&self) -> &'static str;

    /// Get the caption (prefix in the header)
    fn caption(&self) -> &str;

    /// Initialize the meter
    fn init(&mut self) {}

    /// Update meter values from machine state
    fn update(&mut self, machine: &Machine);

    /// Get the height of the meter in lines
    fn height(&self) -> i32 {
        1
    }

    /// Draw the meter
    fn draw(&self, crt: &Crt, machine: &Machine, settings: &Settings, x: i32, y: i32, width: i32);

    /// Get the display mode
    fn mode(&self) -> MeterMode {
        MeterMode::Bar
    }

    /// Set the display mode
    fn set_mode(&mut self, mode: MeterMode);
}

/// Meter type enum for creating meters by name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterType {
    Cpu,
    AllCpus,
    LeftCpus,
    RightCpus,
    LeftCpus2,
    RightCpus2,
    LeftCpus4,
    RightCpus4,
    LeftCpus8,
    RightCpus8,
    Memory,
    Swap,
    LoadAverage,
    Tasks,
    Uptime,
    Battery,
    Hostname,
    Clock,
    Date,
    DateTime,
    DiskIO,
    NetworkIO,
    Blank,
}

impl MeterType {
    /// Create a meter by name
    pub fn create_from_name(name: &str, param: u32) -> Option<Box<dyn Meter>> {
        match name {
            "CPU" => Some(Box::new(CpuMeter::new(Some(param as usize)))),
            "AllCPUs" => Some(Box::new(CpuMeter::all(1))),
            "AllCPUs2" => Some(Box::new(CpuMeter::all(2))),
            "AllCPUs4" => Some(Box::new(CpuMeter::all(4))),
            "AllCPUs8" => Some(Box::new(CpuMeter::all(8))),
            "LeftCPUs" => Some(Box::new(CpuMeter::left(1))),
            "LeftCPUs2" => Some(Box::new(CpuMeter::left(2))),
            "LeftCPUs4" => Some(Box::new(CpuMeter::left(4))),
            "LeftCPUs8" => Some(Box::new(CpuMeter::left(8))),
            "RightCPUs" => Some(Box::new(CpuMeter::right(1))),
            "RightCPUs2" => Some(Box::new(CpuMeter::right(2))),
            "RightCPUs4" => Some(Box::new(CpuMeter::right(4))),
            "RightCPUs8" => Some(Box::new(CpuMeter::right(8))),
            "Memory" => Some(Box::new(MemoryMeter::new())),
            "Swap" => Some(Box::new(SwapMeter::new())),
            "LoadAverage" => Some(Box::new(LoadAverageMeter::new())),
            "Tasks" => Some(Box::new(TasksMeter::new())),
            "Uptime" => Some(Box::new(UptimeMeter::new())),
            "Blank" => Some(Box::new(BlankMeter::new())),
            "Hostname" => Some(Box::new(HostnameMeter::new())),
            "Clock" => Some(Box::new(ClockMeter::new())),
            "Date" => Some(Box::new(DateMeter::new())),
            "DateTime" => Some(Box::new(DateTimeMeter::new())),
            "Battery" => Some(Box::new(BatteryMeter::new())),
            _ => None,
        }
    }
}

/// Draw a bar meter
pub fn draw_bar(crt: &Crt, x: i32, y: i32, width: i32, values: &[(f64, i32)], total: f64) {
    use crate::ui::{bar_meter_char, ColorElement};
    use ncurses::*;

    let bar_width = (width - 2) as usize; // Account for [ and ]

    // Draw brackets
    let bracket_attr = crt.color(ColorElement::BarBorder);
    attrset(bracket_attr);
    mvaddch(y, x, '[' as u32);
    mvaddch(y, x + width - 1, ']' as u32);

    // Calculate bar content
    let mut bar_pos = 0;
    mv(y, x + 1);

    for (idx, (value, color)) in values.iter().enumerate() {
        let attr = crt.colors[*color as usize];
        let bar_chars = if total > 0.0 {
            ((value / total) * bar_width as f64).round() as usize
        } else {
            0
        };

        attrset(attr);
        let bar_ch = bar_meter_char(crt.color_scheme, idx);
        for _ in 0..bar_chars.min(bar_width - bar_pos) {
            addch(bar_ch as u32);
            bar_pos += 1;
        }

        if bar_pos >= bar_width {
            break;
        }
    }

    // Fill remaining with shadow
    let shadow_attr = crt.color(ColorElement::MeterShadow);
    attrset(shadow_attr);
    while bar_pos < bar_width {
        addch(' ' as u32);
        bar_pos += 1;
    }
}

/// Draw a text meter
pub fn draw_text(crt: &Crt, x: i32, y: i32, caption: &str, text: &str) {
    use crate::ui::ColorElement;
    use ncurses::*;

    let caption_attr = crt.color(ColorElement::MeterText);
    let value_attr = crt.color(ColorElement::MeterValue);

    mv(y, x);
    attrset(caption_attr);
    let _ = addstr(caption);

    attrset(value_attr);
    let _ = addstr(text);
}
