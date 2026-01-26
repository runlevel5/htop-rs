//! Swap Meter

use std::cell::RefCell;

use super::{
    draw_bar_with_text, draw_graph, draw_swap_stacked_graph, draw_text_segments, human_unit,
    BarSegment, GraphData, Meter, MeterMode, SwapStackedGraphData, TextSegment,
    SWAP_STACKED_GRAPH_SEGMENTS,
};
use crate::core::{Machine, Settings};
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
    /// Stacked graph data for historical display with per-component colors
    stacked_graph_data: RefCell<SwapStackedGraphData>,
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
            stacked_graph_data: RefCell::new(SwapStackedGraphData::new()),
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
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        match self.mode {
            MeterMode::Bar => {
                // Get color attributes
                let swap_attr = crt.color(ColorElement::Swap);
                let swap_cache_attr = crt.color(ColorElement::SwapCache);

                let text = format!("{}/{}", human_unit(self.used), human_unit(self.total));

                // Build bar segments
                let segments = vec![
                    BarSegment {
                        value: self.used,
                        attr: swap_attr,
                    },
                    BarSegment {
                        value: self.cache.max(0.0),
                        attr: swap_cache_attr,
                    },
                ];

                draw_bar_with_text(crt, x, y, width, "Swp", &segments, self.total, &text);
            }
            MeterMode::Text => {
                // Extract all colors BEFORE building segments
                let text_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::MeterValue);
                let cache_attr = crt.color(ColorElement::SwapCache);

                // Pre-compute values
                let total_str = human_unit(self.total);
                let used_str = human_unit(self.used);
                let cache_str = if self.cache >= 0.0 {
                    Some(human_unit(self.cache))
                } else {
                    None
                };

                // Build segments: "Swp:TOTAL used:VALUE cache:VALUE"
                let mut segments = vec![
                    TextSegment {
                        text: "Swp:",
                        attr: text_attr,
                    },
                    TextSegment {
                        text: &total_str,
                        attr: value_attr,
                    },
                    TextSegment {
                        text: " used:",
                        attr: text_attr,
                    },
                    TextSegment {
                        text: &used_str,
                        attr: value_attr,
                    },
                ];

                // Add cache segment if supported
                if let Some(ref cache) = cache_str {
                    segments.push(TextSegment {
                        text: " cache:",
                        attr: text_attr,
                    });
                    segments.push(TextSegment {
                        text: cache,
                        attr: cache_attr,
                    });
                }

                draw_text_segments(crt, x, y, &segments);
            }
            MeterMode::Led => {
                // LED mode: show same detailed breakdown as Text mode, rendered with draw_led
                // Format: "Swp:TOTAL used:VALUE cache:VALUE"
                let mut text =
                    format!(":{} used:{}", human_unit(self.total), human_unit(self.used));

                if self.cache >= 0.0 {
                    text.push_str(&format!(" cache:{}", human_unit(self.cache)));
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
            MeterMode::StackedGraph => {
                // StackedGraph mode: show used and cache as separate colored segments
                use crate::ui::ColorElement;

                // Pre-compute segment colors
                let segment_colors: [u32; SWAP_STACKED_GRAPH_SEGMENTS] = [
                    crt.color(ColorElement::Swap),      // used
                    crt.color(ColorElement::SwapCache), // cache
                ];

                // Calculate segment values as ratio of total (0.0-1.0)
                let (used_ratio, cache_ratio) = if self.total > 0.0 {
                    (
                        self.used / self.total,
                        self.cache.max(0.0) / self.total, // Hide negative (unsupported) values
                    )
                } else {
                    (0.0, 0.0)
                };

                let segments: [f64; SWAP_STACKED_GRAPH_SEGMENTS] = [used_ratio, cache_ratio];

                // Record the value in stacked graph data
                {
                    let mut stacked_graph_data = self.stacked_graph_data.borrow_mut();
                    stacked_graph_data.record(segments, _settings.delay * 100);
                }

                // Draw the stacked graph
                let stacked_graph_data = self.stacked_graph_data.borrow();
                draw_swap_stacked_graph(
                    crt,
                    x,
                    y,
                    width,
                    self.height(),
                    &stacked_graph_data,
                    "Swp",
                    &segment_colors,
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
        // Swap meter supports all modes including StackedGraph
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
    use crate::core::Machine;
    use crate::meters::human_unit;

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
    // Same algorithm as in mod.rs, verified here for swap-specific coverage

    #[test]
    fn test_human_unit_kilobytes() {
        assert_eq!(human_unit(100.0), "100K");
        assert_eq!(human_unit(512.0), "512K");
        assert_eq!(human_unit(1023.0), "1023K");
    }

    #[test]
    fn test_human_unit_megabytes() {
        assert_eq!(human_unit(1024.0), "1.00M");
        assert_eq!(human_unit(2048.0), "2.00M");
        assert_eq!(human_unit(10240.0), "10.0M");
        assert_eq!(human_unit(102400.0), "100M");
    }

    #[test]
    fn test_human_unit_gigabytes() {
        assert_eq!(human_unit(1048576.0), "1.00G");
        assert_eq!(human_unit(8388608.0), "8.00G");
        assert_eq!(human_unit(16777216.0), "16.0G");
    }

    #[test]
    fn test_human_unit_zero() {
        assert_eq!(human_unit(0.0), "0K");
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_swap_meter_update() {
        let mut meter = SwapMeter::new();
        let mut machine = Machine::default();

        machine.total_swap = 8 * 1024 * 1024; // 8 GB in KB
        machine.used_swap = 2 * 1024 * 1024; // 2 GB
        machine.cached_swap = 512 * 1024; // 512 MB

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
