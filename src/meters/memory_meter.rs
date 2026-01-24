//! Memory Meter

use std::cell::RefCell;

use super::{draw_graph, GraphData, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::bar_meter_char;
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Memory Meter
///
/// Displays memory usage exactly like C htop:
/// Bar mode: "Mem[|||||||||     XXXM/YYYM]"
/// Text mode: ":TOTAL used:VALUE shared:VALUE compressed:VALUE buffers:VALUE cache:VALUE available:VALUE"
/// The value text appears right-aligned INSIDE the bar
#[derive(Debug)]
pub struct MemoryMeter {
    mode: MeterMode,
    used: f64,
    buffers: f64,
    shared: f64,
    compressed: f64,
    cache: f64,
    available: f64,
    total: f64,
    /// Graph data for historical display (RefCell for interior mutability)
    graph_data: RefCell<GraphData>,
}

impl Default for MemoryMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryMeter {
    pub fn new() -> Self {
        MemoryMeter {
            mode: MeterMode::Bar,
            used: 0.0,
            buffers: 0.0,
            shared: 0.0,
            compressed: 0.0,
            cache: 0.0,
            available: 0.0,
            total: 0.0,
            graph_data: RefCell::new(GraphData::new()),
        }
    }

    /// Format memory value like C htop's Meter_humanUnit
    pub(crate) fn human_unit(value: f64) -> String {
        const UNIT_PREFIXES: [char; 5] = ['K', 'M', 'G', 'T', 'P'];
        let mut val = value;
        let mut i = 0;

        while val >= 1024.0 && i < UNIT_PREFIXES.len() - 1 {
            val /= 1024.0;
            i += 1;
        }

        if i == 0 {
            // Kibibytes - no decimal
            format!("{:.0}{}", val, UNIT_PREFIXES[i])
        } else {
            // Mebibytes and above - show decimals based on size
            let precision = if val <= 9.99 {
                2
            } else if val <= 99.9 {
                1
            } else {
                0
            };
            format!("{:.prec$}{}", val, UNIT_PREFIXES[i], prec = precision)
        }
    }
}

impl Meter for MemoryMeter {
    fn name(&self) -> &'static str {
        "Memory"
    }

    fn caption(&self) -> &str {
        "Mem"
    }

    fn update(&mut self, machine: &Machine) {
        self.total = machine.total_mem as f64;
        self.used = machine.used_mem as f64;
        self.buffers = machine.buffers_mem as f64;
        self.shared = machine.shared_mem as f64;
        self.compressed = machine.compressed_mem as f64;
        self.cache = machine.cached_mem as f64;
        self.available = machine.available_mem as f64;
    }

    fn draw(&self, crt: &Crt, _machine: &Machine, settings: &Settings, x: i32, y: i32, width: i32) {
        use ncurses::*;

        match self.mode {
            MeterMode::Bar => {
                // Draw caption "Mem" (exactly 3 chars)
                let caption_attr = crt.color(ColorElement::MeterText);
                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("Mem");

                // Bar area starts after caption
                let bar_x = x + 3;
                let bar_width = width - 3;

                if bar_width < 4 {
                    attrset(crt.color(ColorElement::ResetColor));
                    return;
                }

                // Draw brackets
                let bracket_attr = crt.color(ColorElement::BarBorder);
                attrset(bracket_attr);
                mvaddch(y, bar_x, '[' as u32);
                mvaddch(y, bar_x + bar_width - 1, ']' as u32);

                // Inner bar width (between brackets)
                let inner_width = (bar_width - 2) as usize;

                // Format the text to show inside the bar (right-aligned)
                // Calculate actual used including shared and compressed (like C htop)
                let display_used = self.used + self.shared.max(0.0) + self.compressed.max(0.0);
                let text = format!(
                    "{}/{}",
                    Self::human_unit(display_used),
                    Self::human_unit(self.total)
                );

                // Build the bar content with text right-aligned
                let text_len = text.len();
                let padding = inner_width.saturating_sub(text_len);

                // Calculate bar segments (order matches C htop MemoryMeter_attributes)
                // Only include cache if show_cached_memory is enabled (matches C htop)
                let values: Vec<(f64, ColorElement)> = if settings.show_cached_memory {
                    vec![
                        (self.used, ColorElement::MemoryUsed),
                        (self.shared.max(0.0), ColorElement::MemoryShared),
                        (self.compressed.max(0.0), ColorElement::MemoryCompressed),
                        (self.buffers, ColorElement::MemoryBuffers),
                        (self.cache, ColorElement::MemoryCache),
                    ]
                } else {
                    vec![
                        (self.used, ColorElement::MemoryUsed),
                        (self.shared.max(0.0), ColorElement::MemoryShared),
                        (self.compressed.max(0.0), ColorElement::MemoryCompressed),
                        (self.buffers, ColorElement::MemoryBuffers),
                        // Cache omitted when show_cached_memory is false
                    ]
                };

                // Calculate how many chars each segment takes
                let mut bar_chars = Vec::new();
                let mut total_bar = 0usize;
                for (value, color) in &values {
                    let chars = if self.total > 0.0 {
                        ((*value / self.total) * inner_width as f64).ceil() as usize
                    } else {
                        0
                    };
                    let chars = chars.min(inner_width - total_bar);
                    bar_chars.push((chars, *color));
                    total_bar += chars;
                }

                // Draw the bar content
                mv(y, bar_x + 1);
                let mut pos = 0;
                for (idx, (chars, color)) in bar_chars.iter().enumerate() {
                    let attr = crt.color(*color);
                    attrset(attr);
                    let bar_ch = bar_meter_char(crt.color_scheme, idx);
                    for _ in 0..*chars {
                        if pos >= padding && pos - padding < text_len {
                            // Draw text character
                            let ch = text.chars().nth(pos - padding).unwrap_or(bar_ch);
                            addch(ch as u32);
                        } else {
                            addch(bar_ch as u32);
                        }
                        pos += 1;
                    }
                }

                // Fill remaining with shadow (and text if extends into shadow)
                let shadow_attr = crt.color(ColorElement::BarShadow);
                attrset(shadow_attr);
                while pos < inner_width {
                    if pos >= padding && pos - padding < text_len {
                        let ch = text.chars().nth(pos - padding).unwrap_or(' ');
                        addch(ch as u32);
                    } else {
                        addch(' ' as u32);
                    }
                    pos += 1;
                }
                attrset(crt.color(ColorElement::ResetColor));
            }
            MeterMode::Text => {
                // Text mode: show detailed breakdown like C htop MemoryMeter_display
                // Format: "Mem:TOTAL used:VALUE shared:VALUE compressed:VALUE buffers:VALUE cache:VALUE available:VALUE"
                let text_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::MeterValue);
                let shadow_attr = crt.color(ColorElement::MeterShadow);

                // Colors for memory components (matching C htop)
                let used_attr = crt.color(ColorElement::MemoryUsed);
                let shared_attr = crt.color(ColorElement::MemoryShared);
                let compressed_attr = crt.color(ColorElement::MemoryCompressed);
                let buffers_attr = crt.color(ColorElement::MemoryBuffersText);
                let cache_attr = crt.color(ColorElement::MemoryCache);

                // Buffer/cache label and value colors depend on show_cached_memory setting
                let (buffers_label_attr, buffers_value_attr) = if settings.show_cached_memory {
                    (text_attr, buffers_attr)
                } else {
                    (shadow_attr, shadow_attr)
                };
                let (cache_label_attr, cache_value_attr) = if settings.show_cached_memory {
                    (text_attr, cache_attr)
                } else {
                    (shadow_attr, shadow_attr)
                };

                mv(y, x);

                // "Mem:TOTAL"
                attrset(text_attr);
                let _ = addstr("Mem:");
                attrset(value_attr);
                let _ = addstr(&Self::human_unit(self.total));

                // " used:VALUE"
                attrset(text_attr);
                let _ = addstr(" used:");
                attrset(used_attr);
                let _ = addstr(&Self::human_unit(self.used));

                // " shared:VALUE" (only if >= 0, not all platforms support it)
                if self.shared >= 0.0 {
                    attrset(text_attr);
                    let _ = addstr(" shared:");
                    attrset(shared_attr);
                    let _ = addstr(&Self::human_unit(self.shared));
                }

                // " compressed:VALUE" (only if >= 0, not all platforms support it)
                if self.compressed >= 0.0 {
                    attrset(text_attr);
                    let _ = addstr(" compressed:");
                    attrset(compressed_attr);
                    let _ = addstr(&Self::human_unit(self.compressed));
                }

                // " buffers:VALUE"
                attrset(buffers_label_attr);
                let _ = addstr(" buffers:");
                attrset(buffers_value_attr);
                let _ = addstr(&Self::human_unit(self.buffers));

                // " cache:VALUE"
                attrset(cache_label_attr);
                let _ = addstr(" cache:");
                attrset(cache_value_attr);
                let _ = addstr(&Self::human_unit(self.cache));

                // " available:VALUE" (only if >= 0, not all platforms support it)
                if self.available >= 0.0 {
                    attrset(text_attr);
                    let _ = addstr(" available:");
                    attrset(value_attr);
                    let _ = addstr(&Self::human_unit(self.available));
                }

                attrset(crt.color(ColorElement::ResetColor));
            }
            MeterMode::Led => {
                // LED mode: show same detailed breakdown as Text mode, rendered with draw_led
                // Format: "Mem:TOTAL used:VALUE shared:VALUE compressed:VALUE buffers:VALUE cache:VALUE available:VALUE"
                let mut text = format!(
                    ":{} used:{}",
                    Self::human_unit(self.total),
                    Self::human_unit(self.used)
                );

                if self.shared >= 0.0 {
                    text.push_str(&format!(" shared:{}", Self::human_unit(self.shared)));
                }

                if self.compressed >= 0.0 {
                    text.push_str(&format!(
                        " compressed:{}",
                        Self::human_unit(self.compressed)
                    ));
                }

                text.push_str(&format!(" buffers:{}", Self::human_unit(self.buffers)));
                text.push_str(&format!(" cache:{}", Self::human_unit(self.cache)));

                if self.available >= 0.0 {
                    text.push_str(&format!(" available:{}", Self::human_unit(self.available)));
                }

                super::draw_led(crt, x, y, width, "Mem", &text);
            }
            MeterMode::Graph => {
                // Calculate memory usage as percentage (normalized to 0.0-1.0)
                let display_used = self.used + self.shared.max(0.0) + self.compressed.max(0.0);
                let normalized = if self.total > 0.0 {
                    display_used / self.total
                } else {
                    0.0
                };

                // Record the value in graph data
                {
                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100);
                }

                // Draw the graph
                let graph_data = self.graph_data.borrow();
                draw_graph(crt, x, y, width, self.height(), &graph_data, "Mem");
            }
        }
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
    use crate::core::Machine;

    // ==================== Constructor Tests ====================

    #[test]
    fn test_memory_meter_new() {
        let meter = MemoryMeter::new();
        assert_eq!(meter.mode, MeterMode::Bar);
        assert_eq!(meter.used, 0.0);
        assert_eq!(meter.buffers, 0.0);
        assert_eq!(meter.shared, 0.0);
        assert_eq!(meter.compressed, 0.0);
        assert_eq!(meter.cache, 0.0);
        assert_eq!(meter.available, 0.0);
        assert_eq!(meter.total, 0.0);
    }

    #[test]
    fn test_memory_meter_default() {
        let meter = MemoryMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
        assert_eq!(meter.total, 0.0);
    }

    // ==================== human_unit Tests ====================

    #[test]
    fn test_human_unit_kilobytes_small() {
        // Values < 1024K stay in K
        assert_eq!(MemoryMeter::human_unit(100.0), "100K");
        assert_eq!(MemoryMeter::human_unit(512.0), "512K");
        assert_eq!(MemoryMeter::human_unit(1023.0), "1023K");
    }

    #[test]
    fn test_human_unit_megabytes() {
        // 1024K = 1M, values 1-9.99 get 2 decimal places
        assert_eq!(MemoryMeter::human_unit(1024.0), "1.00M");
        assert_eq!(MemoryMeter::human_unit(2048.0), "2.00M");
        assert_eq!(MemoryMeter::human_unit(5120.0), "5.00M");
        assert_eq!(MemoryMeter::human_unit(9216.0), "9.00M"); // 9M
    }

    #[test]
    fn test_human_unit_megabytes_medium() {
        // Values 10-99.9 get 1 decimal place
        assert_eq!(MemoryMeter::human_unit(10240.0), "10.0M"); // 10M
        assert_eq!(MemoryMeter::human_unit(51200.0), "50.0M"); // 50M
        assert_eq!(MemoryMeter::human_unit(102400.0), "100M"); // 100M - 0 decimals
    }

    #[test]
    fn test_human_unit_megabytes_large() {
        // Values >= 100 get 0 decimal places
        assert_eq!(MemoryMeter::human_unit(102400.0), "100M");
        assert_eq!(MemoryMeter::human_unit(512000.0), "500M");
        assert_eq!(MemoryMeter::human_unit(1023000.0), "999M"); // Just under 1G
    }

    #[test]
    fn test_human_unit_gigabytes() {
        // 1024M = 1G (1024 * 1024 K)
        assert_eq!(MemoryMeter::human_unit(1048576.0), "1.00G"); // 1G
        assert_eq!(MemoryMeter::human_unit(2097152.0), "2.00G"); // 2G
        assert_eq!(MemoryMeter::human_unit(8388608.0), "8.00G"); // 8G
        assert_eq!(MemoryMeter::human_unit(16777216.0), "16.0G"); // 16G
        assert_eq!(MemoryMeter::human_unit(33554432.0), "32.0G"); // 32G
        assert_eq!(MemoryMeter::human_unit(134217728.0), "128G"); // 128G
    }

    #[test]
    fn test_human_unit_terabytes() {
        // 1024G = 1T
        assert_eq!(MemoryMeter::human_unit(1073741824.0), "1.00T"); // 1T
        assert_eq!(MemoryMeter::human_unit(2147483648.0), "2.00T"); // 2T
    }

    #[test]
    fn test_human_unit_petabytes() {
        // 1024T = 1P
        assert_eq!(MemoryMeter::human_unit(1099511627776.0), "1.00P"); // 1P
    }

    #[test]
    fn test_human_unit_zero() {
        assert_eq!(MemoryMeter::human_unit(0.0), "0K");
    }

    #[test]
    fn test_human_unit_fractional_kilobytes() {
        // Fractional values should work (truncated to int for K)
        assert_eq!(MemoryMeter::human_unit(0.5), "0K");
        assert_eq!(MemoryMeter::human_unit(1.5), "2K"); // rounds to 2
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_memory_meter_update() {
        let mut meter = MemoryMeter::new();
        let mut machine = Machine::default();
        
        machine.total_mem = 16 * 1024 * 1024; // 16 GB in KB
        machine.used_mem = 8 * 1024 * 1024;    // 8 GB
        machine.buffers_mem = 512 * 1024;      // 512 MB
        machine.shared_mem = 256 * 1024;       // 256 MB
        machine.compressed_mem = 0;
        machine.cached_mem = 2 * 1024 * 1024;  // 2 GB
        machine.available_mem = 6 * 1024 * 1024; // 6 GB
        
        meter.update(&machine);
        
        assert_eq!(meter.total, 16.0 * 1024.0 * 1024.0);
        assert_eq!(meter.used, 8.0 * 1024.0 * 1024.0);
        assert_eq!(meter.buffers, 512.0 * 1024.0);
        assert_eq!(meter.shared, 256.0 * 1024.0);
        assert_eq!(meter.compressed, 0.0);
        assert_eq!(meter.cache, 2.0 * 1024.0 * 1024.0);
        assert_eq!(meter.available, 6.0 * 1024.0 * 1024.0);
    }

    #[test]
    fn test_memory_meter_update_zero_values() {
        let mut meter = MemoryMeter::new();
        let machine = Machine::default();
        
        meter.update(&machine);
        
        assert_eq!(meter.total, 0.0);
        assert_eq!(meter.used, 0.0);
    }

    // ==================== Meter Trait Tests ====================

    #[test]
    fn test_memory_meter_name() {
        let meter = MemoryMeter::new();
        assert_eq!(meter.name(), "Memory");
    }

    #[test]
    fn test_memory_meter_caption() {
        let meter = MemoryMeter::new();
        assert_eq!(meter.caption(), "Mem");
    }

    #[test]
    fn test_memory_meter_mode() {
        let mut meter = MemoryMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);
        
        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);
        
        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);
        
        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_memory_meter_height() {
        let mut meter = MemoryMeter::new();
        
        // Bar mode: height 1
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.height(), 1);
        
        // Text mode: height 1
        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.height(), 1);
        
        // LED mode: height 3
        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.height(), 3);
        
        // Graph mode: height 4
        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.height(), 4);
    }

    #[test]
    fn test_memory_meter_supported_modes() {
        let meter = MemoryMeter::new();
        // Default: all modes supported
        let modes = meter.supported_modes();
        assert!(modes & (1 << MeterMode::Bar as u32) != 0);
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        assert!(modes & (1 << MeterMode::Graph as u32) != 0);
        assert!(modes & (1 << MeterMode::Led as u32) != 0);
    }

    #[test]
    fn test_memory_meter_default_mode() {
        let meter = MemoryMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Bar);
    }
}
