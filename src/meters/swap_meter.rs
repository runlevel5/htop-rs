//! Swap Meter

use std::cell::RefCell;

use super::{draw_graph, GraphData, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::bar_meter_char;
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Swap Meter
///
/// Displays swap usage exactly like C htop:
/// Bar mode: "Swp[|||||||||     XXXM/YYYM]"
/// The value text appears right-aligned INSIDE the bar
#[derive(Debug)]
pub struct SwapMeter {
    mode: MeterMode,
    used: f64,
    cache: f64,
    total: f64,
    /// Graph data for historical display (RefCell for interior mutability)
    graph_data: RefCell<GraphData>,
}

impl Default for SwapMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl SwapMeter {
    pub fn new() -> Self {
        SwapMeter {
            mode: MeterMode::Bar,
            used: 0.0,
            cache: 0.0,
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

impl Meter for SwapMeter {
    fn name(&self) -> &'static str {
        "Swap"
    }

    fn caption(&self) -> &str {
        "Swp"
    }

    fn update(&mut self, machine: &Machine) {
        self.total = machine.total_swap as f64;
        self.used = machine.used_swap as f64;
        self.cache = machine.cached_swap as f64;
    }

    fn draw(
        &self,
        crt: &Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        use crate::ncurses_compat::*;

        match self.mode {
            MeterMode::Bar => {
                // Draw caption "Swp" (exactly 3 chars)
                let caption_attr = crt.color(ColorElement::MeterText);
                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("Swp");

                // Bar area starts after caption
                let bar_x = x + 3;
                let bar_width = width - 3;

                if bar_width < 4 {
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
                let text = format!(
                    "{}/{}",
                    Self::human_unit(self.used),
                    Self::human_unit(self.total)
                );

                // Build the bar content with text right-aligned
                let text_len = text.len();
                let padding = inner_width.saturating_sub(text_len);

                // Calculate bar segments
                let values = [
                    (self.used, ColorElement::Swap),
                    (self.cache.max(0.0), ColorElement::SwapCache),
                ];

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
            }
            MeterMode::Text => {
                // Text mode: show detailed breakdown like C htop SwapMeter_display
                // Format: "Swp:TOTAL used:VALUE cache:VALUE"
                let text_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::MeterValue);
                let cache_attr = crt.color(ColorElement::SwapCache);

                mv(y, x);

                // "Swp:TOTAL"
                attrset(text_attr);
                let _ = addstr("Swp:");
                attrset(value_attr);
                let _ = addstr(&Self::human_unit(self.total));

                // " used:VALUE"
                attrset(text_attr);
                let _ = addstr(" used:");
                attrset(value_attr);
                let _ = addstr(&Self::human_unit(self.used));

                // " cache:VALUE" (only if >= 0, not all platforms support it)
                if self.cache >= 0.0 {
                    attrset(text_attr);
                    let _ = addstr(" cache:");
                    attrset(cache_attr);
                    let _ = addstr(&Self::human_unit(self.cache));
                }

                attrset(crt.color(ColorElement::ResetColor));
            }
            MeterMode::Led => {
                // LED mode: show same detailed breakdown as Text mode, rendered with draw_led
                // Format: "Swp:TOTAL used:VALUE cache:VALUE"
                let mut text = format!(
                    ":{} used:{}",
                    Self::human_unit(self.total),
                    Self::human_unit(self.used)
                );

                if self.cache >= 0.0 {
                    text.push_str(&format!(" cache:{}", Self::human_unit(self.cache)));
                }

                super::draw_led(crt, x, y, width, "Swp", &text);
            }
            MeterMode::Graph => {
                // Calculate swap usage as percentage (normalized to 0.0-1.0)
                let normalized = if self.total > 0.0 {
                    self.used / self.total
                } else {
                    0.0
                };

                // Record the value in graph data
                {
                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, _settings.delay * 100);
                }

                // Draw the graph
                let graph_data = self.graph_data.borrow();
                draw_graph(crt, x, y, width, self.height(), &graph_data, "Swp");
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
    fn test_swap_meter_new() {
        let meter = SwapMeter::new();
        assert_eq!(meter.mode, MeterMode::Bar);
        assert_eq!(meter.used, 0.0);
        assert_eq!(meter.cache, 0.0);
        assert_eq!(meter.total, 0.0);
    }

    #[test]
    fn test_swap_meter_default() {
        let meter = SwapMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
        assert_eq!(meter.total, 0.0);
    }

    // ==================== human_unit Tests ====================
    // Same algorithm as MemoryMeter, but we test it here too for coverage

    #[test]
    fn test_human_unit_kilobytes() {
        assert_eq!(SwapMeter::human_unit(100.0), "100K");
        assert_eq!(SwapMeter::human_unit(512.0), "512K");
        assert_eq!(SwapMeter::human_unit(1023.0), "1023K");
    }

    #[test]
    fn test_human_unit_megabytes() {
        assert_eq!(SwapMeter::human_unit(1024.0), "1.00M");
        assert_eq!(SwapMeter::human_unit(2048.0), "2.00M");
        assert_eq!(SwapMeter::human_unit(10240.0), "10.0M");
        assert_eq!(SwapMeter::human_unit(102400.0), "100M");
    }

    #[test]
    fn test_human_unit_gigabytes() {
        assert_eq!(SwapMeter::human_unit(1048576.0), "1.00G");
        assert_eq!(SwapMeter::human_unit(8388608.0), "8.00G");
        assert_eq!(SwapMeter::human_unit(16777216.0), "16.0G");
    }

    #[test]
    fn test_human_unit_zero() {
        assert_eq!(SwapMeter::human_unit(0.0), "0K");
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_swap_meter_update() {
        let mut meter = SwapMeter::new();
        let mut machine = Machine::default();
        
        machine.total_swap = 8 * 1024 * 1024; // 8 GB in KB
        machine.used_swap = 2 * 1024 * 1024;   // 2 GB
        machine.cached_swap = 512 * 1024;      // 512 MB
        
        meter.update(&machine);
        
        assert_eq!(meter.total, 8.0 * 1024.0 * 1024.0);
        assert_eq!(meter.used, 2.0 * 1024.0 * 1024.0);
        assert_eq!(meter.cache, 512.0 * 1024.0);
    }

    #[test]
    fn test_swap_meter_update_zero_values() {
        let mut meter = SwapMeter::new();
        let machine = Machine::default();
        
        meter.update(&machine);
        
        assert_eq!(meter.total, 0.0);
        assert_eq!(meter.used, 0.0);
        assert_eq!(meter.cache, 0.0);
    }

    // ==================== Meter Trait Tests ====================

    #[test]
    fn test_swap_meter_name() {
        let meter = SwapMeter::new();
        assert_eq!(meter.name(), "Swap");
    }

    #[test]
    fn test_swap_meter_caption() {
        let meter = SwapMeter::new();
        assert_eq!(meter.caption(), "Swp");
    }

    #[test]
    fn test_swap_meter_mode() {
        let mut meter = SwapMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);
        
        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);
        
        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);
        
        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_swap_meter_height() {
        let mut meter = SwapMeter::new();
        
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.height(), 1);
        
        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.height(), 1);
        
        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.height(), 3);
        
        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.height(), 4);
    }

    #[test]
    fn test_swap_meter_default_mode() {
        let meter = SwapMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Bar);
    }
}
