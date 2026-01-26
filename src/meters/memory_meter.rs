//! Memory Meter

use std::cell::RefCell;

use super::{
    draw_bar_with_text, draw_graph, draw_stacked_graph, draw_text_segments, human_unit, BarSegment,
    GraphData, Meter, MeterMode, StackedGraphData, TextSegment, STACKED_GRAPH_SEGMENTS,
};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Memory Meter
///
/// Displays memory usage exactly like C htop:
/// Bar mode: "Mem[|||||||||     XXXM/YYYM]"
/// Text mode: ":TOTAL used:VALUE shared:VALUE compressed:VALUE buffers:VALUE cache:VALUE available:VALUE"
/// Graph mode: Single-color graph showing total memory usage (matches C htop)
/// Stacked Graph mode: Multi-colored stacked graph showing used, shared, compressed, buffers, cache
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
    /// Stacked graph data for StackedGraph mode
    /// Segments: [used, shared, compressed, buffers, cache]
    stacked_graph_data: RefCell<StackedGraphData>,
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
            stacked_graph_data: RefCell::new(StackedGraphData::new()),
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
                // Get color attributes
                let memory_used_attr = crt.color(ColorElement::MemoryUsed);
                let memory_shared_attr = crt.color(ColorElement::MemoryShared);
                let memory_compressed_attr = crt.color(ColorElement::MemoryCompressed);
                let memory_buffers_attr = crt.color(ColorElement::MemoryBuffers);
                let memory_cache_attr = crt.color(ColorElement::MemoryCache);

                // Calculate actual used including shared and compressed (like C htop)
                let display_used = self.used + self.shared.max(0.0) + self.compressed.max(0.0);
                let text = format!("{}/{}", human_unit(display_used), human_unit(self.total));

                // Build bar segments
                let mut segments = vec![
                    BarSegment {
                        value: self.used,
                        attr: memory_used_attr,
                    },
                    BarSegment {
                        value: self.shared.max(0.0),
                        attr: memory_shared_attr,
                    },
                    BarSegment {
                        value: self.compressed.max(0.0),
                        attr: memory_compressed_attr,
                    },
                    BarSegment {
                        value: self.buffers,
                        attr: memory_buffers_attr,
                    },
                ];
                if settings.show_cached_memory {
                    segments.push(BarSegment {
                        value: self.cache,
                        attr: memory_cache_attr,
                    });
                }

                draw_bar_with_text(crt, x, y, width, "Mem", &segments, self.total, &text);
            }
            MeterMode::Text => {
                // Extract all colors BEFORE building segments
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

                // Pre-compute formatted strings
                let total_str = human_unit(self.total);
                let used_str = human_unit(self.used);
                let shared_str = if self.shared >= 0.0 {
                    Some(human_unit(self.shared))
                } else {
                    None
                };
                let compressed_str = if self.compressed >= 0.0 {
                    Some(human_unit(self.compressed))
                } else {
                    None
                };
                let buffers_str = human_unit(self.buffers);
                let cache_str = human_unit(self.cache);
                let available_str = if self.available >= 0.0 {
                    Some(human_unit(self.available))
                } else {
                    None
                };

                // Build segments dynamically based on available metrics
                let mut segments: Vec<TextSegment> = vec![
                    TextSegment {
                        text: "Mem:",
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
                        attr: used_attr,
                    },
                ];

                // Optional: shared (only if >= 0, not all platforms support it)
                if let Some(ref shared) = shared_str {
                    segments.push(TextSegment {
                        text: " shared:",
                        attr: text_attr,
                    });
                    segments.push(TextSegment {
                        text: shared,
                        attr: shared_attr,
                    });
                }

                // Optional: compressed (only if >= 0, not all platforms support it)
                if let Some(ref compressed) = compressed_str {
                    segments.push(TextSegment {
                        text: " compressed:",
                        attr: text_attr,
                    });
                    segments.push(TextSegment {
                        text: compressed,
                        attr: compressed_attr,
                    });
                }

                // Buffers and cache (may be shadowed based on settings)
                segments.push(TextSegment {
                    text: " buffers:",
                    attr: buffers_label_attr,
                });
                segments.push(TextSegment {
                    text: &buffers_str,
                    attr: buffers_value_attr,
                });
                segments.push(TextSegment {
                    text: " cache:",
                    attr: cache_label_attr,
                });
                segments.push(TextSegment {
                    text: &cache_str,
                    attr: cache_value_attr,
                });

                // Optional: available (only if >= 0, not all platforms support it)
                if let Some(ref available) = available_str {
                    segments.push(TextSegment {
                        text: " available:",
                        attr: text_attr,
                    });
                    segments.push(TextSegment {
                        text: available,
                        attr: value_attr,
                    });
                }

                draw_text_segments(crt, x, y, &segments);
            }
            MeterMode::Led => {
                // LED mode: show same detailed breakdown as Text mode, rendered with draw_led
                // Format: "Mem:TOTAL used:VALUE shared:VALUE compressed:VALUE buffers:VALUE cache:VALUE available:VALUE"
                let mut text =
                    format!(":{} used:{}", human_unit(self.total), human_unit(self.used));

                if self.shared >= 0.0 {
                    text.push_str(&format!(" shared:{}", human_unit(self.shared)));
                }

                if self.compressed >= 0.0 {
                    text.push_str(&format!(" compressed:{}", human_unit(self.compressed)));
                }

                text.push_str(&format!(" buffers:{}", human_unit(self.buffers)));
                text.push_str(&format!(" cache:{}", human_unit(self.cache)));

                if self.available >= 0.0 {
                    text.push_str(&format!(" available:{}", human_unit(self.available)));
                }

                super::draw_led(crt, x, y, width, "Mem", &text);
            }
            MeterMode::Graph => {
                // Calculate memory usage as percentage (normalized to 0.0-1.0)
                // Matches C htop: sum of used + shared + compressed + buffers + cache
                let display_used = self.used
                    + self.shared.max(0.0)
                    + self.compressed.max(0.0)
                    + self.buffers.max(0.0)
                    + self.cache.max(0.0);
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
            MeterMode::StackedGraph => {
                // Calculate memory usage segments as percentages (normalized to 0.0-1.0)
                // Segments: [used, shared, compressed, buffers, cache]
                let segments: [f64; STACKED_GRAPH_SEGMENTS] = if self.total > 0.0 {
                    [
                        self.used / self.total,
                        self.shared.max(0.0) / self.total,
                        self.compressed.max(0.0) / self.total,
                        self.buffers.max(0.0) / self.total,
                        self.cache.max(0.0) / self.total,
                    ]
                } else {
                    [0.0; STACKED_GRAPH_SEGMENTS]
                };

                // Record the values in stacked graph data
                {
                    let mut stacked_graph_data = self.stacked_graph_data.borrow_mut();
                    stacked_graph_data.record(segments, settings.delay * 100);
                }

                // Get segment colors
                let segment_colors: [u32; STACKED_GRAPH_SEGMENTS] = [
                    crt.color(ColorElement::MemoryUsed),
                    crt.color(ColorElement::MemoryShared),
                    crt.color(ColorElement::MemoryCompressed),
                    crt.color(ColorElement::MemoryBuffers),
                    crt.color(ColorElement::MemoryCache),
                ];

                // Draw the stacked graph
                let stacked_graph_data = self.stacked_graph_data.borrow();
                draw_stacked_graph(
                    crt,
                    x,
                    y,
                    width,
                    self.height(),
                    &stacked_graph_data,
                    "Mem",
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
        // Memory meter supports all modes including StackedGraph
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
    // These tests verify the shared human_unit function behavior

    #[test]
    fn test_human_unit_kilobytes_small() {
        // Values < 1024K stay in K
        assert_eq!(human_unit(100.0), "100K");
        assert_eq!(human_unit(512.0), "512K");
        assert_eq!(human_unit(1023.0), "1023K");
    }

    #[test]
    fn test_human_unit_megabytes() {
        // 1024K = 1M, values 1-9.99 get 2 decimal places
        assert_eq!(human_unit(1024.0), "1.00M");
        assert_eq!(human_unit(2048.0), "2.00M");
        assert_eq!(human_unit(5120.0), "5.00M");
        assert_eq!(human_unit(9216.0), "9.00M"); // 9M
    }

    #[test]
    fn test_human_unit_megabytes_medium() {
        // Values 10-99.9 get 1 decimal place
        assert_eq!(human_unit(10240.0), "10.0M"); // 10M
        assert_eq!(human_unit(51200.0), "50.0M"); // 50M
        assert_eq!(human_unit(102400.0), "100M"); // 100M - 0 decimals
    }

    #[test]
    fn test_human_unit_megabytes_large() {
        // Values >= 100 get 0 decimal places
        assert_eq!(human_unit(102400.0), "100M");
        assert_eq!(human_unit(512000.0), "500M");
        assert_eq!(human_unit(1023000.0), "999M"); // Just under 1G
    }

    #[test]
    fn test_human_unit_gigabytes() {
        // 1024M = 1G (1024 * 1024 K)
        assert_eq!(human_unit(1048576.0), "1.00G"); // 1G
        assert_eq!(human_unit(2097152.0), "2.00G"); // 2G
        assert_eq!(human_unit(8388608.0), "8.00G"); // 8G
        assert_eq!(human_unit(16777216.0), "16.0G"); // 16G
        assert_eq!(human_unit(33554432.0), "32.0G"); // 32G
        assert_eq!(human_unit(134217728.0), "128G"); // 128G
    }

    #[test]
    fn test_human_unit_terabytes() {
        // 1024G = 1T
        assert_eq!(human_unit(1073741824.0), "1.00T"); // 1T
        assert_eq!(human_unit(2147483648.0), "2.00T"); // 2T
    }

    #[test]
    fn test_human_unit_petabytes() {
        // 1024T = 1P
        assert_eq!(human_unit(1099511627776.0), "1.00P"); // 1P
    }

    #[test]
    fn test_human_unit_zero() {
        assert_eq!(human_unit(0.0), "0K");
    }

    #[test]
    fn test_human_unit_fractional_kilobytes() {
        // Fractional values should work (truncated to int for K)
        assert_eq!(human_unit(0.5), "0K");
        assert_eq!(human_unit(1.5), "2K"); // rounds to 2
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_memory_meter_update() {
        let mut meter = MemoryMeter::new();
        let mut machine = Machine::default();

        machine.total_mem = 16 * 1024 * 1024; // 16 GB in KB
        machine.used_mem = 8 * 1024 * 1024; // 8 GB
        machine.buffers_mem = 512 * 1024; // 512 MB
        machine.shared_mem = 256 * 1024; // 256 MB
        machine.compressed_mem = 0;
        machine.cached_mem = 2 * 1024 * 1024; // 2 GB
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
