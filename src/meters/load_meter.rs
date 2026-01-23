//! Load Average Meter

use std::cell::RefCell;

use super::{draw_graph, GraphData, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::bar_meter_char;
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Load Average Meter
///
/// Displays load averages exactly like C htop:
/// - Text mode: "Load average: X.XX Y.YY Z.ZZ" (colored values)
/// - Bar mode: colored bar based on 1-min load relative to active CPUs
/// - Graph mode: graphical history of 1-min load
/// - LED mode: "X.XX/Y.YY/Z.ZZ" in LED digits
#[derive(Debug)]
pub struct LoadAverageMeter {
    mode: MeterMode,
    load1: f64,
    load5: f64,
    load15: f64,
    active_cpus: u32,
    /// Graph data for historical display (RefCell for interior mutability)
    graph_data: RefCell<GraphData>,
}

impl Default for LoadAverageMeter {
    fn default() -> Self {
        Self {
            mode: MeterMode::Text,
            load1: 0.0,
            load5: 0.0,
            load15: 0.0,
            active_cpus: 1,
            graph_data: RefCell::new(GraphData::new()),
        }
    }
}

impl LoadAverageMeter {
    pub fn new() -> Self {
        LoadAverageMeter::default()
    }

    /// Get the color for the load bar based on C htop logic:
    /// - Green (OK): load < 1.0
    /// - Yellow (Medium/Warn): load < activeCPUs
    /// - Red (High/Error): load >= activeCPUs
    fn get_load_color(&self) -> ColorElement {
        if self.load1 < 1.0 {
            ColorElement::MeterValueOk
        } else if self.load1 < self.active_cpus as f64 {
            ColorElement::MeterValueWarn
        } else {
            ColorElement::MeterValueError
        }
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
        self.active_cpus = machine.active_cpus;
    }

    fn draw(
        &self,
        crt: &Crt,
        _machine: &Machine,
        settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        use ncurses::*;

        match self.mode {
            MeterMode::Text => {
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

                attrset(crt.color(ColorElement::ResetColor));
            }
            MeterMode::Bar => {
                // Bar mode: show bar for 1-minute load relative to active CPUs
                // Color changes based on load level (green/yellow/red)

                // Draw caption "LA " (3 chars like other meters)
                let caption_attr = crt.color(ColorElement::MeterText);
                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("LA ");

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

                // Total for bar is activeCPUs (like C htop)
                let total = (self.active_cpus as f64).max(1.0);

                // Format the text to show inside the bar
                let text = format!("{:.2}", self.load1);
                let text_len = text.len();
                let padding = inner_width.saturating_sub(text_len);

                // Calculate bar fill
                let bar_fill = if total > 0.0 {
                    ((self.load1 / total) * inner_width as f64).round() as usize
                } else {
                    0
                };
                let bar_fill = bar_fill.min(inner_width);

                // Get color based on load level
                let bar_color = self.get_load_color();
                let bar_attr = crt.color(bar_color);
                let bar_ch = bar_meter_char(crt.color_scheme, 0);

                // Draw the bar content
                mv(y, bar_x + 1);
                let mut pos = 0;

                // Draw filled portion
                attrset(bar_attr);
                while pos < bar_fill {
                    if pos >= padding && pos - padding < text_len {
                        // Draw text character
                        let ch = text.chars().nth(pos - padding).unwrap_or(bar_ch);
                        addch(ch as u32);
                    } else {
                        addch(bar_ch as u32);
                    }
                    pos += 1;
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
            MeterMode::Graph => {
                // Graph mode: show historical 1-minute load
                // Normalized to activeCPUs (so full graph = 100% of CPUs)
                let total = (self.active_cpus as f64).max(1.0);
                let normalized = (self.load1 / total).clamp(0.0, 1.0);

                // Record the value in graph data
                {
                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100);
                }

                // Draw the graph
                let graph_data = self.graph_data.borrow();
                draw_graph(crt, x, y, width, self.height(), &graph_data, "LA ");
            }
            MeterMode::Led => {
                // LED mode: same format as Text mode (C htop uses display function for LED)
                // Format: "X.XX Y.YY Z.ZZ " (each value followed by space)
                let text = format!("{:.2} {:.2} {:.2} ", self.load1, self.load5, self.load15);
                super::draw_led(crt, x, y, width, "Load average: ", &text);
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
