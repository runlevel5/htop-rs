//! Swap Meter

use std::cell::RefCell;

use super::{draw_graph, draw_led, GraphData, Meter, MeterMode};
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
    fn human_unit(value: f64) -> String {
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
        use ncurses::*;

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
                let text_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::MeterValue);

                mv(y, x);
                attrset(text_attr);
                let _ = addstr("Swp:");

                attrset(value_attr);
                let _ = addstr(&format!(
                    "{}/{}",
                    Self::human_unit(self.used),
                    Self::human_unit(self.total)
                ));
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
            MeterMode::Led => {
                // Format swap values for LED display
                let text = format!(
                    "{}/{}",
                    Self::human_unit(self.used),
                    Self::human_unit(self.total)
                );
                draw_led(crt, x, y, width, "Swp ", &text);
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
