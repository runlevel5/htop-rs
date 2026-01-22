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
mod hostname_meter;
mod load_meter;
mod memory_meter;
mod networkio_meter;
mod swap_meter;
mod tasks_meter;
mod uptime_meter;

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
pub use hostname_meter::*;
pub use load_meter::*;
pub use memory_meter::*;
pub use networkio_meter::*;
pub use swap_meter::*;
pub use tasks_meter::*;
pub use uptime_meter::*;

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
            "DiskIO" => Some(Box::new(DiskIOMeter::new())),
            "NetworkIO" => Some(Box::new(NetworkIOMeter::new())),
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
// Graph Meter Mode
// ============================================================================

/// UTF-8 braille graph characters (4 pixels per row)
/// Index is calculated as: left_dots * 5 + right_dots
/// where left_dots and right_dots are 0-4 (number of dots filled from bottom)
const GRAPH_DOTS_UTF8: [&str; 25] = [
    /*00*/ " ", /*01*/ "⢀", /*02*/ "⢠", /*03*/ "⢰", /*04*/ "⢸",
    /*10*/ "⡀", /*11*/ "⣀", /*12*/ "⣠", /*13*/ "⣰", /*14*/ "⣸",
    /*20*/ "⡄", /*21*/ "⣄", /*22*/ "⣤", /*23*/ "⣴", /*24*/ "⣼",
    /*30*/ "⡆", /*31*/ "⣆", /*32*/ "⣦", /*33*/ "⣶", /*34*/ "⣾",
    /*40*/ "⡇", /*41*/ "⣇", /*42*/ "⣧", /*43*/ "⣷", /*44*/ "⣿",
];

/// ASCII graph characters (2 pixels per row)
/// Index is calculated as: left_dots * 3 + right_dots
/// where left_dots and right_dots are 0-2 (number of dots filled from bottom)
const GRAPH_DOTS_ASCII: [&str; 9] = [
    /*00*/ " ", /*01*/ ".", /*02*/ ":", /*10*/ ".", /*11*/ ".",
    /*12*/ ":", /*20*/ ":", /*21*/ ":", /*22*/ ":",
];

/// Pixels per row for UTF-8 mode
const PIXPERROW_UTF8: i32 = 4;

/// Pixels per row for ASCII mode
const PIXPERROW_ASCII: i32 = 2;

/// Draw a graph meter
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows
/// * `graph_data` - Historical data to display
/// * `caption` - Caption to display (3 chars)
pub fn draw_graph(
    crt: &Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &GraphData,
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

    let graph_width = width - caption_len;
    if graph_width < 1 || height < 1 {
        attrset(crt.color(ColorElement::ResetColor));
        return;
    }

    let graph_x = x + caption_len;

    // Select character set based on UTF-8 support
    let (dots, pix_per_row, dots_per_row) = if crt.utf8 {
        (&GRAPH_DOTS_UTF8[..], PIXPERROW_UTF8, 5)
    } else {
        (&GRAPH_DOTS_ASCII[..], PIXPERROW_ASCII, 3)
    };

    let total_pix = pix_per_row * height;

    // Calculate how many values we need (2 per column)
    let n_values = graph_data.values.len();
    let needed = (graph_width * 2) as usize;

    // Determine starting position in data and on screen
    let (data_start, col_start) = if n_values >= needed {
        (n_values - needed, 0)
    } else {
        // Not enough data, start drawing from the right
        let empty_cols = (needed - n_values) / 2;
        (0, empty_cols as i32)
    };

    // Draw the graph
    let mut col = 0i32;
    let mut i = data_start;

    while col < graph_width && i + 1 < n_values {
        // Get two values (left and right half of character)
        let v1 = graph_data.values.get(i).copied().unwrap_or(0.0);
        let v2 = graph_data.values.get(i + 1).copied().unwrap_or(0.0);

        // Convert to pixel counts (clamped to 1..total_pix)
        // Values are already normalized to 0.0-1.0
        let pix1 =
            ((v1 * total_pix as f64).round() as i32).clamp(if v1 > 0.0 { 1 } else { 0 }, total_pix);
        let pix2 =
            ((v2 * total_pix as f64).round() as i32).clamp(if v2 > 0.0 { 1 } else { 0 }, total_pix);

        // Draw each row of this column
        let mut color_idx = ColorElement::Graph1;
        for line in 0..height {
            // Calculate how many pixels are filled in this row
            let row_from_bottom = height - 1 - line;
            let base_pix = row_from_bottom * pix_per_row;

            let line1 = (pix1 - base_pix).clamp(0, pix_per_row);
            let line2 = (pix2 - base_pix).clamp(0, pix_per_row);

            // Get the character for this cell
            let char_idx = (line1 * dots_per_row + line2) as usize;
            let ch = dots.get(char_idx).unwrap_or(&" ");

            attrset(crt.color(color_idx));
            let _ = mvaddstr(y + line, graph_x + col_start + col, ch);

            // Alternate colors for visual effect (top row is Graph1, rest is Graph2)
            color_idx = ColorElement::Graph2;
        }

        col += 1;
        i += 2;
    }

    // Fill any remaining empty columns on the left with spaces
    attrset(crt.color(ColorElement::ResetColor));
    for empty_col in 0..col_start {
        for line in 0..height {
            mvaddch(y + line, graph_x + empty_col, ' ' as u32);
        }
    }

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
