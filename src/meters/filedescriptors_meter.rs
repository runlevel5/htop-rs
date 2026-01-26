//! FileDescriptors Meter
//!
//! Displays number of allocated/available file descriptors.

use std::cell::RefCell;

use super::{draw_graph, draw_graph_colored, draw_led, GraphData, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// Threshold above which fd_max is considered "effectively unlimited"
/// (matches C htop's 1 << 30)
const FD_EFFECTIVELY_UNLIMITED: u64 = 1 << 30;

/// FileDescriptors Meter - displays allocated/available file descriptors
#[derive(Debug)]
pub struct FileDescriptorsMeter {
    mode: MeterMode,
    fd_used: Option<u64>,
    fd_max: Option<u64>,
    /// Calculated total for bar scaling (adaptive like C htop)
    bar_total: f64,
    /// Graph data for historical display
    graph_data: RefCell<GraphData>,
}

impl Default for FileDescriptorsMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDescriptorsMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text, // Default mode is Text (matches C htop)
            fd_used: None,
            fd_max: None,
            bar_total: 65536.0,
            graph_data: RefCell::new(GraphData::new()),
        }
    }

    /// Check if max is effectively unlimited (> 1 << 30)
    fn is_unlimited(&self) -> bool {
        match self.fd_max {
            Some(max) => max >= FD_EFFECTIVELY_UNLIMITED,
            None => true,
        }
    }

    /// Calculate adaptive bar total like C htop
    /// 1. If max <= 65536, use max directly
    /// 2. If max > 65536, use powers of 2 starting at 65536, doubled until > 16 * used
    /// 3. Cap at 1 << 30 or actual max, whichever is smaller
    fn calculate_bar_total(&mut self) {
        let used = self.fd_used.unwrap_or(0) as f64;
        let max = self.fd_max.unwrap_or(65536) as f64;

        if max <= 65536.0 {
            self.bar_total = max;
        } else {
            // Start at 65536 and double until > 16 * used
            let mut total = 65536.0_f64;
            while total < 16.0 * used && total < (1u64 << 30) as f64 {
                total *= 2.0;
            }

            // Cap at actual max if smaller
            if total > max {
                total = max;
            }

            // Cap at 1 << 30
            if total > (1u64 << 30) as f64 {
                total = (1u64 << 30) as f64;
            }

            self.bar_total = total;
        }
    }

    /// Format the bar text like C htop: "used/max" or "used/unlimited"
    fn bar_text(&self) -> String {
        match self.fd_used {
            None => "unknown/unknown".to_string(),
            Some(used) => {
                if self.is_unlimited() {
                    format!("{}/unlimited", used)
                } else {
                    format!("{}/{}", used, self.fd_max.unwrap_or(0))
                }
            }
        }
    }
}

impl Meter for FileDescriptorsMeter {
    fn name(&self) -> &'static str {
        "FileDescriptors"
    }

    fn caption(&self) -> &str {
        "FDs"
    }

    fn update(&mut self, machine: &Machine) {
        self.fd_used = machine.fd_used;
        self.fd_max = machine.fd_max;
        self.calculate_bar_total();
    }

    fn default_mode(&self) -> MeterMode {
        MeterMode::Text
    }

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        match self.mode {
            MeterMode::Bar => {
                let caption_attr = crt.color(ColorElement::MeterText);
                let bracket_attr = crt.color(ColorElement::BarBorder);
                let fd_used_attr = crt.color(ColorElement::FileDescriptorUsed);
                let shadow_attr = crt.color(ColorElement::BarShadow);
                let reset_attr = crt.color(ColorElement::ResetColor);
                // Pre-compute bar character (theme-agnostic) - FD meter only uses segment 0
                let bar_ch = crt.bar_char(0);

                let used = self.fd_used.unwrap_or(0) as f64;
                let total = self.bar_total;
                let text = self.bar_text();

                crt.with_window(|win| {
                    // Draw caption "FDs" (3 chars)
                    let _ = win.mv(y, x);
                    let _ = win.attrset(caption_attr);
                    let _ = win.addstr("FDs");

                    // Bar area starts after caption
                    let bar_x = x + 3;
                    let bar_width = width - 3;

                    if bar_width < 4 {
                        let _ = win.attrset(reset_attr);
                        return;
                    }

                    // Draw brackets
                    let _ = win.attrset(bracket_attr);
                    let _ = win.mvaddch(y, bar_x, '[' as u32);
                    let _ = win.mvaddch(y, bar_x + bar_width - 1, ']' as u32);

                    // Inner bar width (between brackets)
                    let inner_width = (bar_width - 2) as usize;

                    // Build the bar content with text right-aligned
                    let text_len = text.len();
                    let padding = inner_width.saturating_sub(text_len);

                    // Calculate bar fill
                    let bar_chars = if total > 0.0 {
                        ((used / total) * inner_width as f64).ceil() as usize
                    } else {
                        0
                    };
                    let bar_chars = bar_chars.min(inner_width);

                    // Draw the bar content
                    let _ = win.mv(y, bar_x + 1);
                    let mut pos = 0;

                    // Draw used portion
                    let _ = win.attrset(fd_used_attr);
                    while pos < bar_chars {
                        if pos >= padding && pos - padding < text_len {
                            let ch = text.chars().nth(pos - padding).unwrap_or(bar_ch);
                            let _ = win.addch(ch as u32);
                        } else {
                            let _ = win.addch(bar_ch as u32);
                        }
                        pos += 1;
                    }

                    // Fill remaining with shadow
                    let _ = win.attrset(shadow_attr);
                    while pos < inner_width {
                        if pos >= padding && pos - padding < text_len {
                            let ch = text.chars().nth(pos - padding).unwrap_or(' ');
                            let _ = win.addch(ch as u32);
                        } else {
                            let _ = win.addch(' ' as u32);
                        }
                        pos += 1;
                    }

                    let _ = win.attrset(reset_attr);
                });
            }
            MeterMode::Text => {
                let text_attr = crt.color(ColorElement::MeterText);
                let fd_used_attr = crt.color(ColorElement::FileDescriptorUsed);
                let fd_max_attr = crt.color(ColorElement::FileDescriptorMax);
                let reset_attr = crt.color(ColorElement::ResetColor);

                crt.with_window(|win| {
                    let _ = win.mv(y, x);

                    // Check if unknown
                    if self.fd_used.is_none() {
                        let _ = win.attrset(text_attr);
                        let _ = win.addstr("FDs: ");
                        let _ = win.addstr("unknown");
                        let _ = win.attrset(reset_attr);
                        return;
                    }

                    // "FDs: used: VALUE max: VALUE"
                    let _ = win.attrset(text_attr);
                    let _ = win.addstr("FDs: used: ");
                    let _ = win.attrset(fd_used_attr);
                    let _ = win.addstr(&format!("{}", self.fd_used.unwrap_or(0)));

                    let _ = win.attrset(text_attr);
                    let _ = win.addstr(" max: ");
                    let _ = win.attrset(fd_max_attr);
                    if self.is_unlimited() {
                        let _ = win.addstr("unlimited");
                    } else {
                        let _ = win.addstr(&format!("{}", self.fd_max.unwrap_or(0)));
                    }

                    let _ = win.attrset(reset_attr);
                });
            }
            MeterMode::Led => {
                // LED mode: same info as text but rendered with draw_led
                let text = if self.fd_used.is_none() {
                    ": unknown".to_string()
                } else if self.is_unlimited() {
                    format!(": used:{} max:unlimited", self.fd_used.unwrap_or(0))
                } else {
                    format!(
                        ": used:{} max:{}",
                        self.fd_used.unwrap_or(0),
                        self.fd_max.unwrap_or(0)
                    )
                };

                draw_led(crt, x, y, width, "FDs", &text);
            }
            MeterMode::Graph => {
                // Calculate usage as percentage for graph
                let normalized = if self.fd_used.is_some() && self.bar_total > 0.0 {
                    (self.fd_used.unwrap_or(0) as f64 / self.bar_total).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                // Record the value in graph data
                {
                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100);
                }

                // Draw the graph with default Graph1/Graph2 colors
                let graph_data = self.graph_data.borrow();
                draw_graph(crt, x, y, width, self.height(), &graph_data, "FDs");
            }
            MeterMode::StackedGraph => {
                // Calculate usage as percentage for graph
                let normalized = if self.fd_used.is_some() && self.bar_total > 0.0 {
                    (self.fd_used.unwrap_or(0) as f64 / self.bar_total).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                // Record the value in graph data
                {
                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100);
                }

                // Draw the graph with FileDescriptorUsed color (matches bar mode color)
                let graph_data = self.graph_data.borrow();
                let fd_used_color = crt.color(ColorElement::FileDescriptorUsed);
                draw_graph_colored(
                    crt,
                    x,
                    y,
                    width,
                    self.height(),
                    &graph_data,
                    "FDs",
                    fd_used_color,
                );
            }
        }
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }

    fn supported_modes(&self) -> u32 {
        // FileDescriptors meter supports all modes
        (1 << MeterMode::Bar as u32)
            | (1 << MeterMode::Text as u32)
            | (1 << MeterMode::Graph as u32)
            | (1 << MeterMode::Led as u32)
            | (1 << MeterMode::StackedGraph as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filedescriptors_meter_new() {
        let meter = FileDescriptorsMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.fd_used, None);
        assert_eq!(meter.fd_max, None);
    }

    #[test]
    fn test_filedescriptors_meter_default() {
        let meter = FileDescriptorsMeter::default();
        assert_eq!(meter.mode, MeterMode::Text);
    }

    #[test]
    fn test_filedescriptors_meter_name() {
        let meter = FileDescriptorsMeter::new();
        assert_eq!(meter.name(), "FileDescriptors");
    }

    #[test]
    fn test_filedescriptors_meter_caption() {
        let meter = FileDescriptorsMeter::new();
        assert_eq!(meter.caption(), "FDs");
    }

    #[test]
    fn test_filedescriptors_meter_mode() {
        let mut meter = FileDescriptorsMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    #[test]
    fn test_filedescriptors_meter_default_mode() {
        let meter = FileDescriptorsMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }

    #[test]
    fn test_filedescriptors_meter_update() {
        let mut meter = FileDescriptorsMeter::new();
        let mut machine = Machine::default();

        machine.fd_used = Some(1234);
        machine.fd_max = Some(65536);

        meter.update(&machine);

        assert_eq!(meter.fd_used, Some(1234));
        assert_eq!(meter.fd_max, Some(65536));
    }

    #[test]
    fn test_filedescriptors_meter_is_unlimited() {
        let mut meter = FileDescriptorsMeter::new();

        // No max = unlimited
        assert!(meter.is_unlimited());

        // Small max = not unlimited
        meter.fd_max = Some(65536);
        assert!(!meter.is_unlimited());

        // Large max = unlimited
        meter.fd_max = Some(1 << 30);
        assert!(meter.is_unlimited());
    }

    #[test]
    fn test_filedescriptors_meter_bar_text() {
        let mut meter = FileDescriptorsMeter::new();

        // Unknown
        assert_eq!(meter.bar_text(), "unknown/unknown");

        // Normal
        meter.fd_used = Some(1234);
        meter.fd_max = Some(65536);
        assert_eq!(meter.bar_text(), "1234/65536");

        // Unlimited
        meter.fd_max = Some(1 << 30);
        assert_eq!(meter.bar_text(), "1234/unlimited");
    }

    #[test]
    fn test_filedescriptors_meter_calculate_bar_total_small() {
        let mut meter = FileDescriptorsMeter::new();
        meter.fd_used = Some(1000);
        meter.fd_max = Some(10000);
        meter.calculate_bar_total();
        assert_eq!(meter.bar_total, 10000.0);
    }

    #[test]
    fn test_filedescriptors_meter_calculate_bar_total_large() {
        let mut meter = FileDescriptorsMeter::new();
        meter.fd_used = Some(1000);
        meter.fd_max = Some(1_000_000);
        meter.calculate_bar_total();
        // Should be power of 2 >= 16 * 1000 = 16000, starting from 65536
        assert!(meter.bar_total >= 65536.0);
        assert!(meter.bar_total <= 1_000_000.0);
    }

    #[test]
    fn test_filedescriptors_meter_supported_modes() {
        let meter = FileDescriptorsMeter::new();
        let modes = meter.supported_modes();
        assert!(modes & (1 << MeterMode::Bar as u32) != 0);
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        assert!(modes & (1 << MeterMode::Graph as u32) != 0);
        assert!(modes & (1 << MeterMode::Led as u32) != 0);
        assert!(modes & (1 << MeterMode::StackedGraph as u32) != 0);
    }
}
