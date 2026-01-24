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
    pub(crate) fn get_load_color(&self) -> ColorElement {
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

/// Load Meter (1-minute only)
///
/// Displays only the 1-minute load average, matching C htop's LoadMeter:
/// - Text mode: "Load: X.XX" with LOAD color
/// - Bar mode: colored bar based on 1-min load relative to active CPUs
/// - Graph mode: graphical history of 1-min load
/// - LED mode: "X.XX" in LED digits
#[derive(Debug)]
pub struct LoadMeter {
    mode: MeterMode,
    load1: f64,
    active_cpus: u32,
    /// Graph data for historical display (RefCell for interior mutability)
    graph_data: RefCell<GraphData>,
}

impl Default for LoadMeter {
    fn default() -> Self {
        Self {
            mode: MeterMode::Text,
            load1: 0.0,
            active_cpus: 1,
            graph_data: RefCell::new(GraphData::new()),
        }
    }
}

impl LoadMeter {
    pub fn new() -> Self {
        LoadMeter::default()
    }

    /// Get the color for the load bar based on C htop logic:
    /// - Green (OK): load < 1.0
    /// - Yellow (Medium/Warn): load < activeCPUs
    /// - Red (High/Error): load >= activeCPUs
    pub(crate) fn get_load_color(&self) -> ColorElement {
        if self.load1 < 1.0 {
            ColorElement::MeterValueOk
        } else if self.load1 < self.active_cpus as f64 {
            ColorElement::MeterValueWarn
        } else {
            ColorElement::MeterValueError
        }
    }
}

impl Meter for LoadMeter {
    fn name(&self) -> &'static str {
        "Load"
    }

    fn caption(&self) -> &str {
        "Load: "
    }

    fn update(&mut self, machine: &Machine) {
        self.load1 = machine.load_average[0];
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
                let load_attr = crt.color(ColorElement::Load);

                mv(y, x);

                // "Load: "
                attrset(caption_attr);
                let _ = addstr("Load: ");

                // 1-minute load (with trailing space like C htop)
                attrset(load_attr);
                let _ = addstr(&format!("{:.2} ", self.load1));

                attrset(crt.color(ColorElement::ResetColor));
            }
            MeterMode::Bar => {
                // Bar mode: show bar for 1-minute load relative to active CPUs
                // Color changes based on load level (green/yellow/red)

                // Draw caption "Loa" (first 3 chars of "Load: " like C htop)
                let caption_attr = crt.color(ColorElement::MeterText);
                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("Loa");

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
                draw_graph(crt, x, y, width, self.height(), &graph_data, "Loa");
            }
            MeterMode::Led => {
                // LED mode: just show the 1-minute load value
                let text = format!("{:.2} ", self.load1);
                super::draw_led(crt, x, y, width, "Load: ", &text);
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

    // ==================== LoadAverageMeter Constructor Tests ====================

    #[test]
    fn test_load_average_meter_new() {
        let meter = LoadAverageMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.load1, 0.0);
        assert_eq!(meter.load5, 0.0);
        assert_eq!(meter.load15, 0.0);
        assert_eq!(meter.active_cpus, 1);
    }

    #[test]
    fn test_load_average_meter_default() {
        let meter = LoadAverageMeter::default();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.active_cpus, 1);
    }

    // ==================== LoadAverageMeter get_load_color Tests ====================

    #[test]
    fn test_load_average_get_load_color_green() {
        // load < 1.0 -> Green (Ok)
        let mut meter = LoadAverageMeter::new();
        meter.load1 = 0.0;
        meter.active_cpus = 4;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueOk);

        meter.load1 = 0.5;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueOk);

        meter.load1 = 0.99;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueOk);
    }

    #[test]
    fn test_load_average_get_load_color_yellow() {
        // 1.0 <= load < activeCPUs -> Yellow (Warn)
        let mut meter = LoadAverageMeter::new();
        meter.active_cpus = 4;

        meter.load1 = 1.0;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueWarn);

        meter.load1 = 2.5;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueWarn);

        meter.load1 = 3.99;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueWarn);
    }

    #[test]
    fn test_load_average_get_load_color_red() {
        // load >= activeCPUs -> Red (Error)
        let mut meter = LoadAverageMeter::new();
        meter.active_cpus = 4;

        meter.load1 = 4.0;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueError);

        meter.load1 = 8.0;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueError);

        meter.load1 = 100.0;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueError);
    }

    #[test]
    fn test_load_average_get_load_color_single_cpu() {
        // With 1 CPU: load >= 1 is immediately red
        let mut meter = LoadAverageMeter::new();
        meter.active_cpus = 1;

        meter.load1 = 0.5;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueOk);

        meter.load1 = 1.0;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueError);
    }

    // ==================== LoadAverageMeter Update Tests ====================

    #[test]
    fn test_load_average_meter_update() {
        let mut meter = LoadAverageMeter::new();
        let mut machine = Machine::default();
        
        machine.load_average = [1.5, 2.0, 1.8];
        machine.active_cpus = 8;
        
        meter.update(&machine);
        
        assert_eq!(meter.load1, 1.5);
        assert_eq!(meter.load5, 2.0);
        assert_eq!(meter.load15, 1.8);
        assert_eq!(meter.active_cpus, 8);
    }

    // ==================== LoadAverageMeter Trait Tests ====================

    #[test]
    fn test_load_average_meter_name() {
        let meter = LoadAverageMeter::new();
        assert_eq!(meter.name(), "LoadAverage");
    }

    #[test]
    fn test_load_average_meter_caption() {
        let meter = LoadAverageMeter::new();
        assert_eq!(meter.caption(), "Load average: ");
    }

    #[test]
    fn test_load_average_meter_mode() {
        let mut meter = LoadAverageMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);
        
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
        
        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);
        
        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    // ==================== LoadMeter Constructor Tests ====================

    #[test]
    fn test_load_meter_new() {
        let meter = LoadMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.load1, 0.0);
        assert_eq!(meter.active_cpus, 1);
    }

    #[test]
    fn test_load_meter_default() {
        let meter = LoadMeter::default();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.active_cpus, 1);
    }

    // ==================== LoadMeter get_load_color Tests ====================

    #[test]
    fn test_load_get_load_color_green() {
        let mut meter = LoadMeter::new();
        meter.load1 = 0.5;
        meter.active_cpus = 4;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueOk);
    }

    #[test]
    fn test_load_get_load_color_yellow() {
        let mut meter = LoadMeter::new();
        meter.load1 = 2.0;
        meter.active_cpus = 4;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueWarn);
    }

    #[test]
    fn test_load_get_load_color_red() {
        let mut meter = LoadMeter::new();
        meter.load1 = 5.0;
        meter.active_cpus = 4;
        assert_eq!(meter.get_load_color(), ColorElement::MeterValueError);
    }

    // ==================== LoadMeter Update Tests ====================

    #[test]
    fn test_load_meter_update() {
        let mut meter = LoadMeter::new();
        let mut machine = Machine::default();
        
        machine.load_average = [3.5, 2.0, 1.5];
        machine.active_cpus = 4;
        
        meter.update(&machine);
        
        assert_eq!(meter.load1, 3.5);
        assert_eq!(meter.active_cpus, 4);
    }

    // ==================== LoadMeter Trait Tests ====================

    #[test]
    fn test_load_meter_name() {
        let meter = LoadMeter::new();
        assert_eq!(meter.name(), "Load");
    }

    #[test]
    fn test_load_meter_caption() {
        let meter = LoadMeter::new();
        assert_eq!(meter.caption(), "Load: ");
    }

    #[test]
    fn test_load_meter_mode() {
        let mut meter = LoadMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);
        
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);
    }

    #[test]
    fn test_load_meter_height() {
        let mut meter = LoadMeter::new();
        
        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.height(), 1);
        
        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.height(), 4);
    }
}
