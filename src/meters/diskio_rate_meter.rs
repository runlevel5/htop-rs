//! DiskIO Rate Meter
//!
//! Displays disk IO read/write bytes per second.
//! This is a simplified version of DiskIOMeter that focuses only on transfer rates,
//! without utilization percentage.

use std::cell::RefCell;

use super::{draw_bar, draw_graph, draw_led, GraphData, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// Rate status for disk IO rate meter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RateStatus {
    #[default]
    Init, // Initializing, no previous data
    Data,   // Have valid rate data
    Stale,  // Data is stale (> 30 seconds old)
    NoData, // No data available (read failed)
}

/// DiskIO Rate Meter - displays disk IO read/write bytes per second
///
/// Unlike DiskIOMeter which shows both rates and utilization, this meter
/// focuses only on the transfer rates (read/write bytes per second).
#[derive(Debug)]
pub struct DiskIORateMeter {
    mode: MeterMode,
    status: RateStatus,
    /// Cached read rate in bytes per second
    read_rate: f64,
    /// Cached write rate in bytes per second
    write_rate: f64,
    /// Graph data for historical display
    graph_data: RefCell<GraphData>,
}

impl Default for DiskIORateMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl DiskIORateMeter {
    pub fn new() -> Self {
        Self {
            mode: MeterMode::Text,
            status: RateStatus::Init,
            read_rate: 0.0,
            write_rate: 0.0,
            graph_data: RefCell::new(GraphData::new()),
        }
    }

    /// Format a rate value using human-readable units (like C htop's Meter_humanUnit)
    fn human_unit(bytes_per_sec: f64) -> String {
        const UNIT_PREFIXES: [char; 5] = ['K', 'M', 'G', 'T', 'P'];
        // Convert to KiB/s first (divide by 1024)
        let mut val = bytes_per_sec / 1024.0;
        let mut i = 0;

        while val >= 1000.0 && i < UNIT_PREFIXES.len() - 1 {
            val /= 1024.0;
            i += 1;
        }

        if val < 10.0 {
            format!("{:.2}{}", val, UNIT_PREFIXES[i])
        } else if val < 100.0 {
            format!("{:.1}{}", val, UNIT_PREFIXES[i])
        } else {
            format!("{:.0}{}", val, UNIT_PREFIXES[i])
        }
    }
}

impl Meter for DiskIORateMeter {
    fn name(&self) -> &'static str {
        "DiskIORate"
    }

    fn caption(&self) -> &str {
        "Dsk"
    }

    fn update(&mut self, machine: &Machine) {
        // Check if we have valid data
        if machine.disk_io_last_update == 0 {
            self.status = RateStatus::Init;
            return;
        }

        // Check for stale data (> 30 seconds)
        let time_since_update = machine
            .realtime_ms
            .saturating_sub(machine.disk_io_last_update);
        if time_since_update > 30000 {
            self.status = RateStatus::Stale;
            return;
        }

        // We have valid data
        self.status = RateStatus::Data;
        self.read_rate = machine.disk_io_read_rate;
        self.write_rate = machine.disk_io_write_rate;
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
                // Draw caption "Dsk" (exactly 3 chars)
                let caption_attr = crt.color(ColorElement::MeterText);
                let reset_attr = crt.color(ColorElement::ResetColor);
                let value_attr = crt.color(ColorElement::MeterValue);

                crt.with_window(|win| {
                    let _ = win.mv(y, x);
                    let _ = win.attrset(caption_attr);
                    let _ = win.addstr("Dsk");
                });

                // Bar area starts after caption
                let bar_x = x + 3;
                let bar_width = width - 3;

                if bar_width < 4 {
                    crt.with_window(|win| {
                        let _ = win.attrset(reset_attr);
                    });
                    return;
                }

                // Handle non-data states
                if self.status != RateStatus::Data {
                    let text = match self.status {
                        RateStatus::Init => "init",
                        RateStatus::Stale => "stale",
                        RateStatus::NoData => "no data",
                        RateStatus::Data => unreachable!(),
                    };
                    crt.with_window(|win| {
                        let _ = win.attrset(value_attr);
                        let _ = win.mvaddnstr(y, bar_x, text, bar_width);
                        let _ = win.attrset(reset_attr);
                    });
                    return;
                }

                // Normalize rates for display - use a fixed max of 1GB/s per direction
                const MAX_RATE: f64 = 1024.0 * 1024.0 * 1024.0; // 1 GB/s
                let read_norm = (self.read_rate / MAX_RATE).min(1.0);
                let write_norm = (self.write_rate / MAX_RATE).min(1.0);

                // Draw bar with read and write segments
                let bar_values = [
                    (read_norm, crt.color(ColorElement::MeterValueIORead) as i32),
                    (
                        write_norm,
                        crt.color(ColorElement::MeterValueIOWrite) as i32,
                    ),
                ];
                draw_bar(crt, bar_x, y, bar_width, &bar_values, 1.0);

                crt.with_window(|win| {
                    let _ = win.attrset(reset_attr);
                });
            }
            MeterMode::Text => {
                // Handle non-data states
                if self.status != RateStatus::Data {
                    let (text, color) = match self.status {
                        RateStatus::Init => ("initializing...", ColorElement::MeterValue),
                        RateStatus::Stale => ("stale data", ColorElement::MeterValueWarn),
                        RateStatus::NoData => ("no data", ColorElement::MeterValueError),
                        RateStatus::Data => unreachable!(),
                    };
                    let attr = crt.color(color);
                    let reset_attr = crt.color(ColorElement::ResetColor);
                    crt.with_window(|win| {
                        let _ = win.mv(y, x);
                        let _ = win.attrset(attr);
                        let _ = win.addstr(text);
                        let _ = win.attrset(reset_attr);
                    });
                    return;
                }

                let text_attr = crt.color(ColorElement::MeterText);
                let read_attr = crt.color(ColorElement::MeterValueIORead);
                let write_attr = crt.color(ColorElement::MeterValueIOWrite);
                let reset_attr = crt.color(ColorElement::ResetColor);

                let read_str = Self::human_unit(self.read_rate);
                let write_str = Self::human_unit(self.write_rate);

                crt.with_window(|win| {
                    let _ = win.mv(y, x);

                    // "read: XiB/s write: YiB/s"
                    let _ = win.attrset(text_attr);
                    let _ = win.addstr("read: ");

                    let _ = win.attrset(read_attr);
                    let _ = win.addstr(&read_str);
                    let _ = win.addstr("iB/s");

                    let _ = win.attrset(text_attr);
                    let _ = win.addstr(" write: ");

                    let _ = win.attrset(write_attr);
                    let _ = win.addstr(&write_str);
                    let _ = win.addstr("iB/s");

                    let _ = win.attrset(reset_attr);
                });
            }
            MeterMode::Graph => {
                // For graph mode, use combined read+write rate as percentage of max
                if self.status == RateStatus::Data {
                    const MAX_RATE: f64 = 1024.0 * 1024.0 * 1024.0; // 1 GB/s
                    let combined = self.read_rate + self.write_rate;
                    let normalized = (combined / MAX_RATE).min(1.0);

                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100);
                }

                let graph_data = self.graph_data.borrow();
                draw_graph(crt, x, y, width, self.height(), &graph_data, "Dsk");
            }
            MeterMode::Led => {
                // Format rate values for LED display
                if self.status != RateStatus::Data {
                    let text = match self.status {
                        RateStatus::Init => "----",
                        RateStatus::Stale => "OLD",
                        RateStatus::NoData => "N/A",
                        RateStatus::Data => unreachable!(),
                    };
                    draw_led(crt, x, y, width, "Dsk ", text);
                } else {
                    let text = format!(
                        "r:{}iB/s w:{}iB/s",
                        Self::human_unit(self.read_rate),
                        Self::human_unit(self.write_rate)
                    );
                    draw_led(crt, x, y, width, "Dsk ", &text);
                }
            }
            MeterMode::StackedGraph => {
                // StackedGraph not supported for DiskIORate meter
            }
        }
    }

    fn height(&self) -> i32 {
        self.mode.default_height()
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }

    fn default_mode(&self) -> MeterMode {
        MeterMode::Text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Constructor Tests ====================

    #[test]
    fn test_diskio_rate_meter_new() {
        let meter = DiskIORateMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.status, RateStatus::Init);
        assert_eq!(meter.read_rate, 0.0);
        assert_eq!(meter.write_rate, 0.0);
    }

    #[test]
    fn test_diskio_rate_meter_default() {
        let meter = DiskIORateMeter::default();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.status, RateStatus::Init);
    }

    // ==================== human_unit Tests ====================
    // DiskIORate human_unit converts bytes/sec to KiB/s first, then scales

    #[test]
    fn test_human_unit_bytes_to_kilobytes() {
        // 0 bytes/sec = 0 KiB/s
        assert_eq!(DiskIORateMeter::human_unit(0.0), "0.00K");

        // 1024 bytes/sec = 1.00 KiB/s
        assert_eq!(DiskIORateMeter::human_unit(1024.0), "1.00K");

        // 5120 bytes/sec = 5.00 KiB/s
        assert_eq!(DiskIORateMeter::human_unit(5120.0), "5.00K");
    }

    #[test]
    fn test_human_unit_kilobytes_precision() {
        // Values < 10 get 2 decimal places
        assert_eq!(DiskIORateMeter::human_unit(1024.0), "1.00K");
        assert_eq!(DiskIORateMeter::human_unit(9.0 * 1024.0), "9.00K");

        // Values 10-99 get 1 decimal place
        assert_eq!(DiskIORateMeter::human_unit(10.0 * 1024.0), "10.0K");
        assert_eq!(DiskIORateMeter::human_unit(99.0 * 1024.0), "99.0K");

        // Values >= 100 get 0 decimal places
        assert_eq!(DiskIORateMeter::human_unit(100.0 * 1024.0), "100K");
        assert_eq!(DiskIORateMeter::human_unit(999.0 * 1024.0), "999K");
    }

    #[test]
    fn test_human_unit_megabytes() {
        // 1 MiB/s = 1024 * 1024 bytes/sec
        let mib = 1024.0 * 1024.0;
        assert_eq!(DiskIORateMeter::human_unit(mib), "1.00M");

        // 10 MiB/s
        assert_eq!(DiskIORateMeter::human_unit(10.0 * mib), "10.0M");

        // 100 MiB/s
        assert_eq!(DiskIORateMeter::human_unit(100.0 * mib), "100M");
    }

    #[test]
    fn test_human_unit_gigabytes() {
        // 1 GiB/s = 1024 * 1024 * 1024 bytes/sec
        let gib = 1024.0 * 1024.0 * 1024.0;
        assert_eq!(DiskIORateMeter::human_unit(gib), "1.00G");

        // 5 GiB/s
        assert_eq!(DiskIORateMeter::human_unit(5.0 * gib), "5.00G");
    }

    #[test]
    fn test_human_unit_typical_disk_rates() {
        // Typical HDD sequential: ~150 MB/s
        let hdd_rate = 150.0 * 1024.0 * 1024.0;
        assert_eq!(DiskIORateMeter::human_unit(hdd_rate), "150M");

        // Typical SSD: ~500 MB/s
        let ssd_rate = 500.0 * 1024.0 * 1024.0;
        assert_eq!(DiskIORateMeter::human_unit(ssd_rate), "500M");

        // NVMe SSD: ~3 GB/s
        let nvme_rate = 3.0 * 1024.0 * 1024.0 * 1024.0;
        assert_eq!(DiskIORateMeter::human_unit(nvme_rate), "3.00G");
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_diskio_rate_meter_update_init() {
        let mut meter = DiskIORateMeter::new();
        let machine = Machine::default();

        // No previous data -> Init status
        meter.update(&machine);
        assert_eq!(meter.status, RateStatus::Init);
    }

    #[test]
    fn test_diskio_rate_meter_update_with_data() {
        let mut meter = DiskIORateMeter::new();
        let mut machine = Machine::default();

        machine.disk_io_last_update = 1000;
        machine.realtime_ms = 1500; // 500ms since last update
        machine.disk_io_read_rate = 1024.0 * 1024.0; // 1 MiB/s
        machine.disk_io_write_rate = 512.0 * 1024.0; // 512 KiB/s

        meter.update(&machine);

        assert_eq!(meter.status, RateStatus::Data);
        assert_eq!(meter.read_rate, 1024.0 * 1024.0);
        assert_eq!(meter.write_rate, 512.0 * 1024.0);
    }

    #[test]
    fn test_diskio_rate_meter_update_stale() {
        let mut meter = DiskIORateMeter::new();
        let mut machine = Machine::default();

        machine.disk_io_last_update = 1000;
        machine.realtime_ms = 32000; // 31 seconds since last update (> 30s = stale)

        meter.update(&machine);
        assert_eq!(meter.status, RateStatus::Stale);
    }

    // ==================== Meter Trait Tests ====================

    #[test]
    fn test_diskio_rate_meter_name() {
        let meter = DiskIORateMeter::new();
        assert_eq!(meter.name(), "DiskIORate");
    }

    #[test]
    fn test_diskio_rate_meter_caption() {
        let meter = DiskIORateMeter::new();
        assert_eq!(meter.caption(), "Dsk");
    }

    #[test]
    fn test_diskio_rate_meter_mode() {
        let mut meter = DiskIORateMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_diskio_rate_meter_height() {
        let mut meter = DiskIORateMeter::new();

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.height(), 1);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.height(), 1);

        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.height(), 4);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.height(), 3);
    }

    #[test]
    fn test_diskio_rate_meter_supported_modes() {
        let meter = DiskIORateMeter::new();
        assert!(meter.supports_mode(MeterMode::Bar));
        assert!(meter.supports_mode(MeterMode::Text));
        assert!(meter.supports_mode(MeterMode::Graph));
        assert!(meter.supports_mode(MeterMode::Led));
    }

    #[test]
    fn test_diskio_rate_meter_default_mode() {
        let meter = DiskIORateMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Text);
    }
}
