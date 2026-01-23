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
mod diskio_meter;
mod diskio_rate_meter;
mod diskio_time_meter;
mod filedescriptors_meter;
mod gpu_meter;
mod hostname_meter;
mod hugepages_meter;
mod load_meter;
mod memory_meter;
mod memoryswap_meter;
mod networkio_meter;
mod pressure_stall_meter;
mod selinux_meter;
mod swap_meter;
mod system_meter;
mod systemd_meter;
mod tasks_meter;
mod uptime_meter;
mod zfs_arc_meter;
mod zram_meter;

use std::time::Instant;

use crate::core::{Machine, Settings};
use crate::ui::Crt;

pub use battery_meter::*;
pub use blank_meter::*;
pub use clock_meter::*;
pub use cpu_meter::*;
pub use date_meter::*;
pub use datetime_meter::*;
pub use diskio_meter::*;
pub use diskio_rate_meter::*;
pub use diskio_time_meter::*;
pub use filedescriptors_meter::*;
pub use gpu_meter::*;
pub use hostname_meter::*;
pub use hugepages_meter::*;
pub use load_meter::*;
pub use memory_meter::*;
pub use memoryswap_meter::*;
pub use networkio_meter::*;
pub use pressure_stall_meter::*;
pub use selinux_meter::*;
pub use swap_meter::*;
pub use system_meter::*;
pub use systemd_meter::*;
pub use tasks_meter::*;
pub use uptime_meter::*;
pub use zfs_arc_meter::*;
pub use zram_meter::*;

/// Default graph height in rows (matches C htop DEFAULT_GRAPH_HEIGHT)
pub const DEFAULT_GRAPH_HEIGHT: i32 = 4;

/// Maximum number of graph data values to store
pub const MAX_GRAPH_DATA_VALUES: usize = 32768;

/// Graph data storage for historical meter values
#[derive(Debug, Clone)]
pub struct GraphData {
    /// Time of last update
    pub last_update: Option<Instant>,
    /// Historical values (0.0 to 1.0 normalized)
    pub values: Vec<f64>,
}

impl Default for GraphData {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphData {
    pub fn new() -> Self {
        GraphData {
            last_update: None,
            values: Vec::new(),
        }
    }

    /// Record a new value (0.0 to 1.0 normalized)
    /// Returns true if value was recorded, false if too soon since last update
    pub fn record(&mut self, value: f64, delay_ms: u32) -> bool {
        let now = Instant::now();
        let should_record = match self.last_update {
            Some(last) => now.duration_since(last).as_millis() >= delay_ms as u128,
            None => true,
        };

        if should_record {
            self.last_update = Some(now);

            // Shift values left and add new value
            if self.values.len() >= MAX_GRAPH_DATA_VALUES {
                self.values.remove(0);
            }
            self.values.push(value.clamp(0.0, 1.0));
            true
        } else {
            false
        }
    }

    /// Ensure the buffer has at least the given capacity
    pub fn ensure_capacity(&mut self, width: usize) {
        // We need 2 values per column for the graph
        let needed = width * 2;
        if self.values.len() < needed {
            // Prepend zeros to fill
            let mut new_values = vec![0.0; needed - self.values.len()];
            new_values.append(&mut self.values);
            self.values = new_values;
        }
    }
}

/// Meter display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeterMode {
    #[default]
    Bar,
    Text,
    Graph,
    Led,
}

impl MeterMode {
    /// Get the default height for this mode
    pub fn default_height(&self) -> i32 {
        match self {
            MeterMode::Bar => 1,
            MeterMode::Text => 1,
            MeterMode::Graph => DEFAULT_GRAPH_HEIGHT,
            MeterMode::Led => 3,
        }
    }
}

impl From<crate::core::MeterMode> for MeterMode {
    fn from(mode: crate::core::MeterMode) -> Self {
        match mode {
            crate::core::MeterMode::Bar => MeterMode::Bar,
            crate::core::MeterMode::Text => MeterMode::Text,
            crate::core::MeterMode::Graph => MeterMode::Graph,
            crate::core::MeterMode::Led => MeterMode::Led,
        }
    }
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
        self.mode().default_height()
    }

    /// Draw the meter
    fn draw(&self, crt: &Crt, machine: &Machine, settings: &Settings, x: i32, y: i32, width: i32);

    /// Get the display mode
    fn mode(&self) -> MeterMode {
        MeterMode::Bar
    }

    /// Set the display mode
    fn set_mode(&mut self, mode: MeterMode);

    /// Get supported modes for this meter (default: all modes)
    fn supported_modes(&self) -> u32 {
        // Default: all modes supported (Bar, Text, Graph, Led)
        (1 << MeterMode::Bar as u32)
            | (1 << MeterMode::Text as u32)
            | (1 << MeterMode::Graph as u32)
            | (1 << MeterMode::Led as u32)
    }

    /// Get the default mode for this meter (default: Bar)
    fn default_mode(&self) -> MeterMode {
        MeterMode::Bar
    }

    /// Check if a mode is supported
    fn supports_mode(&self, mode: MeterMode) -> bool {
        (self.supported_modes() & (1 << mode as u32)) != 0
    }
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
            "Load" => Some(Box::new(LoadMeter::new())),
            "Tasks" => Some(Box::new(TasksMeter::new())),
            "Uptime" => Some(Box::new(UptimeMeter::new())),
            "Blank" => Some(Box::new(BlankMeter::new())),
            "Hostname" => Some(Box::new(HostnameMeter::new())),
            "Clock" => Some(Box::new(ClockMeter::new())),
            "Date" => Some(Box::new(DateMeter::new())),
            "DateTime" => Some(Box::new(DateTimeMeter::new())),
            "Battery" => Some(Box::new(BatteryMeter::new())),
            "DiskIO" => Some(Box::new(DiskIOMeter::new())),
            "NetworkIO" => Some(Box::new(NetworkIOMeter::new())),
            // Stub meters (not yet implemented)
            "MemorySwap" => Some(Box::new(MemorySwapMeter::new())),
            "System" => Some(Box::new(SystemMeter::new())),
            "DiskIORate" => Some(Box::new(DiskIORateMeter::new())),
            "DiskIOTime" => Some(Box::new(DiskIOTimeMeter::new())),
            "FileDescriptors" => Some(Box::new(FileDescriptorsMeter::new())),
            "GPU" => Some(Box::new(GpuMeter::new())),
            "HugePages" => Some(Box::new(HugePagesMeter::new())),
            "PressureStallCPUSome" => Some(Box::new(PressureStallCPUSomeMeter::new())),
            "PressureStallIOSome" => Some(Box::new(PressureStallIOSomeMeter::new())),
            "PressureStallIOFull" => Some(Box::new(PressureStallIOFullMeter::new())),
            "PressureStallIRQFull" => Some(Box::new(PressureStallIRQFullMeter::new())),
            "PressureStallMemorySome" => Some(Box::new(PressureStallMemorySomeMeter::new())),
            "PressureStallMemoryFull" => Some(Box::new(PressureStallMemoryFullMeter::new())),
            "Zram" => Some(Box::new(ZramMeter::new())),
            "SELinux" => Some(Box::new(SELinuxMeter::new())),
            "Systemd" => Some(Box::new(SystemdMeter::new())),
            "SystemdUser" => Some(Box::new(SystemdUserMeter::new())),
            "ZFSARC" => Some(Box::new(ZfsArcMeter::new())),
            "ZFSCARC" => Some(Box::new(ZfsCompressedArcMeter::new())),
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

// ============================================================================
// Graph Meter Mode (stub - to be implemented)
// ============================================================================

/// Draw a graph meter (stub implementation)
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `_height` - Height in rows (unused in stub)
/// * `_graph_data` - Historical data to display (unused in stub)
/// * `caption` - Caption to display (3 chars)
pub fn draw_graph(
    crt: &Crt,
    x: i32,
    y: i32,
    width: i32,
    _height: i32,
    _graph_data: &GraphData,
    caption: &str,
) {
    use crate::ui::ColorElement;
    use ncurses::*;

    // Draw the caption (3 chars like bar mode)
    let caption_len = 3;
    if width >= caption_len {
        attrset(crt.color(ColorElement::MeterText));
        let _ = mvaddnstr(y, x, caption, caption_len);
    }

    // Draw stub message
    attrset(crt.color(ColorElement::MeterValueError));
    let _ = addstr(" To be implemented");
    attrset(crt.color(ColorElement::ResetColor));
}

// ============================================================================
// LED Meter Mode
// ============================================================================

/// ASCII LED digits (3 rows, 10 digits 0-9, each digit is 4 chars wide)
/// Row 0: top of digit, Row 1: middle, Row 2: bottom
/// Index: row * 10 + digit
const LED_DIGITS_ASCII: [&str; 30] = [
    // Row 0 (top): digits 0-9
    " __ ", "    ", " __ ", " __ ", "    ", " __ ", " __ ", " __ ", " __ ", " __ ",
    // Row 1 (middle): digits 0-9
    "|  |", "   |", " __|", " __|", "|__|", "|__ ", "|__ ", "   |", "|__|", "|__|",
    // Row 2 (bottom): digits 0-9
    "|__|", "   |", "|__ ", " __|", "   |", " __|", "|__|", "   |", "|__|", " __|",
];

/// UTF-8 LED digits (3 rows, 10 digits 0-9, each digit is 4 chars wide)
/// Uses box-drawing characters for a cleaner look
const LED_DIGITS_UTF8: [&str; 30] = [
    // Row 0 (top): digits 0-9
    "┌──┐",
    "  ┐ ",
    "╶──┐",
    "╶──┐",
    "╷  ╷",
    "┌──╴",
    "┌──╴",
    "╶──┐",
    "┌──┐",
    "┌──┐",
    // Row 1 (middle): digits 0-9
    "│  │",
    "  │ ",
    "┌──┘",
    " ──┤",
    "└──┤",
    "└──┐",
    "├──┐",
    "   │",
    "├──┤",
    "└──┤",
    // Row 2 (bottom): digits 0-9
    "└──┘",
    "  ╵ ",
    "└──╴",
    "╶──┘",
    "   ╵",
    "╶──┘",
    "└──┘",
    "   ╵",
    "└──┘",
    "╶──┘",
];

/// Draw a single LED digit at position (x, y)
fn draw_led_digit(crt: &Crt, x: i32, y: i32, digit: u8) {
    use ncurses::*;

    let digits = if crt.utf8 {
        &LED_DIGITS_UTF8
    } else {
        &LED_DIGITS_ASCII
    };

    let d = digit as usize;
    if d > 9 {
        return;
    }

    for row in 0..3 {
        let idx = row * 10 + d;
        if let Some(s) = digits.get(idx) {
            let _ = mvaddstr(y + row as i32, x, s);
        }
    }
}

/// Draw an LED meter
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of LED display, 3 rows tall)
/// * `width` - Total width
/// * `caption` - Caption to display
/// * `text` - Text to display as LED digits (digits are rendered as LED, other chars as-is)
pub fn draw_led(crt: &Crt, x: i32, y: i32, width: i32, caption: &str, text: &str) {
    use crate::ui::ColorElement;
    use ncurses::*;

    // Y position for non-digit text (caption and symbols like %, /, .)
    // UTF-8: middle row (y + 1), ASCII: bottom row (y + 2)
    let y_text = if crt.utf8 { y + 1 } else { y + 2 };

    attrset(crt.color(ColorElement::LedColor));

    // Draw the caption
    if width > 0 {
        let caption_display: String = caption.chars().take(width as usize).collect();
        let _ = mvaddstr(y_text, x, &caption_display);
    }

    let caption_width = caption.chars().count().min(width as usize) as i32;
    if width <= caption_width {
        attrset(crt.color(ColorElement::ResetColor));
        return;
    }

    let mut xx = x + caption_width;
    let _remaining_width = width - caption_width;

    // Draw each character
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            // Check if we have room for a 4-char wide digit
            if xx > x + width - 4 {
                break;
            }

            let digit = (ch as u8) - b'0';
            draw_led_digit(crt, xx, y, digit);
            xx += 4;
        } else {
            // Non-digit character - draw on the text line
            if xx > x + width - 1 {
                break;
            }

            let _ = mvaddch(y_text, xx, ch as u32);
            xx += 1;
        }
    }

    attrset(crt.color(ColorElement::ResetColor));
}
