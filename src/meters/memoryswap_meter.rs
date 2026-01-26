//! MemorySwap Meter
//!
//! Combined memory and swap usage display.
//! This is a composite meter that draws Memory and Swap meters side-by-side.

use super::{MemoryMeter, Meter, MeterMode, SwapMeter};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// MemorySwap Meter - displays combined memory and swap usage side-by-side
///
/// This is a composite meter that embeds a Memory meter and a Swap meter,
/// drawing them next to each other with the available width split in half.
/// This matches C htop's MemorySwapMeter which uses isMultiColumn=true.
#[derive(Debug)]
pub struct MemorySwapMeter {
    mode: MeterMode,
    memory_meter: MemoryMeter,
    swap_meter: SwapMeter,
}

impl Default for MemorySwapMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySwapMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Bar,
            memory_meter: MemoryMeter::new(),
            swap_meter: SwapMeter::new(),
        }
    }
}

impl Meter for MemorySwapMeter {
    fn name(&self) -> &'static str {
        "MemorySwap"
    }

    fn caption(&self) -> &str {
        "M&S"
    }

    fn init(&mut self) {
        self.memory_meter.init();
        self.swap_meter.init();
    }

    fn update(&mut self, machine: &Machine) {
        // Update both sub-meters
        self.memory_meter.update(machine);
        self.swap_meter.update(machine);
    }

    fn draw(
        &self,
        crt: &mut Crt,
        machine: &Machine,
        settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        // For Text and LED modes, add spaces between meters for readability
        let gap = match self.mode {
            MeterMode::Text | MeterMode::Led => 2,
            _ => 0,
        };

        // Split width in half for each sub-meter (like C htop)
        // Account for gap in width calculation
        let available_width = width - gap;
        let col_width = available_width / 2;
        let diff = available_width % 2;

        // Draw memory meter on the left
        self.memory_meter
            .draw(crt, machine, settings, x, y, col_width);

        // Calculate where swap meter starts
        let swap_x = x + col_width + diff;

        // For Text/LED modes, draw gap spaces at the swap position first
        // This ensures spacing even if Memory meter's text overflowed its width
        if gap > 0 {
            let reset_attr = crt.color(ColorElement::ResetColor);
            crt.with_window(|win| {
                let _ = win.mv(y, swap_x);
                let _ = win.attrset(reset_attr);
                let _ = win.addstr("  "); // 2 spaces for gap
            });
        }

        // Draw swap meter on the right (after the gap)
        self.swap_meter
            .draw(crt, machine, settings, swap_x + gap, y, col_width);
    }

    fn height(&self) -> i32 {
        // Height is the maximum of both sub-meters
        self.memory_meter.height().max(self.swap_meter.height())
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
        // Sync mode to both sub-meters
        self.memory_meter.set_mode(mode);
        self.swap_meter.set_mode(mode);
    }

    fn supported_modes(&self) -> u32 {
        // Support intersection of both sub-meters' supported modes
        self.memory_meter.supported_modes() & self.swap_meter.supported_modes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memoryswap_meter_new() {
        let meter = MemorySwapMeter::new();
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_memoryswap_meter_default() {
        let meter = MemorySwapMeter::default();
        // MeterMode::default() is Bar
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_memoryswap_meter_name() {
        let meter = MemorySwapMeter::new();
        assert_eq!(meter.name(), "MemorySwap");
    }

    #[test]
    fn test_memoryswap_meter_caption() {
        let meter = MemorySwapMeter::new();
        assert_eq!(meter.caption(), "M&S");
    }

    #[test]
    fn test_memoryswap_meter_mode() {
        let mut meter = MemorySwapMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);
        // Check that sub-meters also have mode set
        assert_eq!(meter.memory_meter.mode(), MeterMode::Text);
        assert_eq!(meter.swap_meter.mode(), MeterMode::Text);
    }

    #[test]
    fn test_memoryswap_meter_height_bar() {
        let meter = MemorySwapMeter::new();
        // Bar mode: height 1 for both meters
        assert_eq!(meter.height(), 1);
    }

    #[test]
    fn test_memoryswap_meter_height_graph() {
        let mut meter = MemorySwapMeter::new();
        meter.set_mode(MeterMode::Graph);
        // Graph mode: height 4 for both meters
        assert_eq!(meter.height(), 4);
    }

    #[test]
    fn test_memoryswap_meter_height_led() {
        let mut meter = MemorySwapMeter::new();
        meter.set_mode(MeterMode::Led);
        // LED mode: height 3 for both meters
        assert_eq!(meter.height(), 3);
    }

    #[test]
    fn test_memoryswap_meter_update() {
        let mut meter = MemorySwapMeter::new();
        let mut machine = Machine::default();

        // Set up memory values
        machine.total_mem = 16 * 1024 * 1024; // 16 GB in KB
        machine.used_mem = 8 * 1024 * 1024; // 8 GB
        machine.buffers_mem = 512 * 1024; // 512 MB
        machine.cached_mem = 2 * 1024 * 1024; // 2 GB

        // Set up swap values
        machine.total_swap = 4 * 1024 * 1024; // 4 GB in KB
        machine.used_swap = 1 * 1024 * 1024; // 1 GB

        // Update should not panic
        meter.update(&machine);
    }

    #[test]
    fn test_memoryswap_meter_supported_modes() {
        let meter = MemorySwapMeter::new();
        let modes = meter.supported_modes();
        // Should support at least Bar, Text, Graph, Led (intersection of Memory and Swap)
        assert!(modes & (1 << MeterMode::Bar as u32) != 0);
        assert!(modes & (1 << MeterMode::Text as u32) != 0);
        assert!(modes & (1 << MeterMode::Graph as u32) != 0);
        assert!(modes & (1 << MeterMode::Led as u32) != 0);
    }
}
