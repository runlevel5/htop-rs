//! Memory Meter

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Memory Meter
///
/// Displays memory usage exactly like C htop:
/// Bar mode: "Mem[|||||||||     XXXM/YYYM]"
/// The value text appears right-aligned INSIDE the bar
#[derive(Debug, Default)]
pub struct MemoryMeter {
    mode: MeterMode,
    used: f64,
    buffers: f64,
    shared: f64,
    compressed: f64,
    cache: f64,
    total: f64,
}

impl MemoryMeter {
    pub fn new() -> Self {
        MemoryMeter::default()
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
    }

    fn draw(&self, crt: &Crt, _machine: &Machine, settings: &Settings, x: i32, y: i32, width: i32) {
        use ncurses::*;

        match self.mode {
            MeterMode::Bar => {
                // Draw caption "Mem" (exactly 3 chars)
                let caption_attr = crt.color(ColorElement::MeterText);
                mv(y, x);
                attron(caption_attr);
                let _ = addstr("Mem");
                attroff(caption_attr);

                // Bar area starts after caption
                let bar_x = x + 3;
                let bar_width = width - 3;

                if bar_width < 4 {
                    return;
                }

                // Draw brackets
                let bracket_attr = crt.color(ColorElement::BarBorder);
                attron(bracket_attr);
                mvaddch(y, bar_x, '[' as u32);
                mvaddch(y, bar_x + bar_width - 1, ']' as u32);
                attroff(bracket_attr);

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
                for (chars, color) in &bar_chars {
                    let attr = crt.color(*color);
                    attron(attr);
                    for _ in 0..*chars {
                        if pos >= padding && pos - padding < text_len {
                            // Draw text character
                            let ch = text.chars().nth(pos - padding).unwrap_or('|');
                            addch(ch as u32);
                        } else {
                            addch('|' as u32);
                        }
                        pos += 1;
                    }
                    attroff(attr);
                }

                // Fill remaining with shadow (and text if extends into shadow)
                let shadow_attr = crt.color(ColorElement::BarShadow);
                attron(shadow_attr);
                while pos < inner_width {
                    if pos >= padding && pos - padding < text_len {
                        let ch = text.chars().nth(pos - padding).unwrap_or(' ');
                        addch(ch as u32);
                    } else {
                        addch(' ' as u32);
                    }
                    pos += 1;
                }
                attroff(shadow_attr);
            }
            MeterMode::Text => {
                let text_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::MeterValue);

                mv(y, x);
                attron(text_attr);
                let _ = addstr("Mem:");
                attroff(text_attr);

                let display_used = self.used + self.shared.max(0.0);
                attron(value_attr);
                let _ = addstr(&format!(
                    "{}/{}",
                    Self::human_unit(display_used),
                    Self::human_unit(self.total)
                ));
                attroff(value_attr);
            }
            _ => {
                // Fall back to bar mode for unsupported modes
                let fallback = MemoryMeter {
                    mode: MeterMode::Bar,
                    ..*self
                };
                fallback.draw(crt, _machine, settings, x, y, width);
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
