//! Disk IO Meter
//!
//! Displays disk read/write rates and utilization, matching C htop's DiskIOMeter.

use std::cell::RefCell;

use super::{draw_bar_with_text, draw_graph, draw_led, BarSegment, GraphData, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Rate status for disk IO meter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateStatus {
    Init,   // Initializing, no previous data
    Data,   // Have valid rate data
    Stale,  // Data is stale (> 30 seconds old)
    NoData, // No data available (read failed)
}

/// Disk IO Meter
///
/// Displays disk read/write rates and utilization.
/// In bar mode, shows two half-width bars: one for rate, one for utilization.
/// In text mode, shows "read: XiB/s write: YiB/s; Z% busy (N disks)".
#[derive(Debug)]
pub struct DiskIOMeter {
    mode: MeterMode,
    status: RateStatus,
    /// Cached read rate in bytes per second
    read_rate: f64,
    /// Cached write rate in bytes per second
    write_rate: f64,
    /// Disk utilization as percentage (0-100)
    utilization: f64,
    /// Normalized utilization (0-1) for bar display
    utilization_norm: f64,
    /// Number of disks
    num_disks: u64,
    /// Graph data for historical display
    graph_data: RefCell<GraphData>,
}

impl Default for DiskIOMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl DiskIOMeter {
    pub fn new() -> Self {
        DiskIOMeter {
            mode: MeterMode::Text, // Default to text mode like C htop
            status: RateStatus::Init,
            read_rate: 0.0,
            write_rate: 0.0,
            utilization: 0.0,
            utilization_norm: 0.0,
            num_disks: 0,
            graph_data: RefCell::new(GraphData::new()),
        }
    }

    /// Format a rate value using human-readable units (like C htop's Meter_humanUnit)
    pub(crate) fn human_unit(bytes_per_sec: f64) -> String {
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

impl Meter for DiskIOMeter {
    fn name(&self) -> &'static str {
        "DiskIO"
    }

    fn caption(&self) -> &str {
        "Disk IO"
    }

    fn update(&mut self, machine: &Machine) {
        // Check if we have valid rate data
        // disk_io_last_update > 0 means we've scanned at least once
        // disk_io_num_disks > 0 means we found disks and have valid data
        if machine.disk_io_last_update == 0 || machine.disk_io_num_disks == 0 {
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

        // We have valid data (rates may be 0 if disk is idle, that's fine)
        self.status = RateStatus::Data;
        self.read_rate = machine.disk_io_read_rate;
        self.write_rate = machine.disk_io_write_rate;
        self.utilization = machine.disk_io_utilization;
        self.num_disks = machine.disk_io_num_disks;

        // Normalize utilization for bar display (0-1)
        // Utilization can exceed 100% if multiple disks are busy, so we normalize
        // to the number of disks
        self.utilization_norm = if machine.disk_io_num_disks > 0 {
            (self.utilization / 100.0 / machine.disk_io_num_disks as f64).min(1.0)
        } else {
            0.0
        };
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
                // C htop style: single bar with text inside
                // Format: "Disk IO[|||||      r: 0KiB/s w: 0KiB/s 0.0%]"

                // Handle non-data states
                let text = if self.status != RateStatus::Data {
                    match self.status {
                        RateStatus::Init => "init".to_string(),
                        RateStatus::Stale => "stale".to_string(),
                        RateStatus::NoData => "no data".to_string(),
                        RateStatus::Data => unreachable!(),
                    }
                } else {
                    // Format: "r: <rate>iB/s w: <rate>iB/s <util>%"
                    let read_str = Self::human_unit(self.read_rate);
                    let write_str = Self::human_unit(self.write_rate);
                    format!(
                        "r:{}iB/s w:{}iB/s {:.1}%",
                        read_str, write_str, self.utilization
                    )
                };

                // Bar segments: read rate + write rate (stacked)
                // Normalize rates for display - use a logarithmic scale or fixed max
                const MAX_RATE: f64 = 1024.0 * 1024.0 * 1024.0; // 1 GB/s
                let read_norm = (self.read_rate / MAX_RATE).min(1.0);
                let write_norm = (self.write_rate / MAX_RATE).min(1.0);

                let segments = [
                    BarSegment {
                        value: read_norm,
                        attr: crt.color(ColorElement::MeterValueIORead),
                    },
                    BarSegment {
                        value: write_norm,
                        attr: crt.color(ColorElement::MeterValueIOWrite),
                    },
                ];

                draw_bar_with_text(crt, x, y, width, self.caption(), &segments, 1.0, &text);
            }
            MeterMode::Text => {
                let caption_attr = crt.color(ColorElement::MeterText);
                let reset_attr = crt.color(ColorElement::ResetColor);

                crt.with_window(|win| {
                    let _ = win.mv(y, x);
                    let _ = win.attrset(caption_attr);
                    let _ = win.addstr(self.caption());
                    let _ = win.addstr(": ");
                });

                // Handle non-data states
                if self.status != RateStatus::Data {
                    let (text, color) = match self.status {
                        RateStatus::Init => ("initializing...", ColorElement::MeterValue),
                        RateStatus::Stale => ("stale data", ColorElement::MeterValueWarn),
                        RateStatus::NoData => ("no data", ColorElement::MeterValueError),
                        RateStatus::Data => unreachable!(),
                    };
                    let attr = crt.color(color);
                    crt.with_window(|win| {
                        let _ = win.attrset(attr);
                        let _ = win.addstr(text);
                        let _ = win.attrset(reset_attr);
                    });
                    return;
                }

                let text_attr = crt.color(ColorElement::MeterText);
                let read_attr = crt.color(ColorElement::MeterValueIORead);
                let write_attr = crt.color(ColorElement::MeterValueIOWrite);
                let value_attr = crt.color(ColorElement::MeterValue);

                // Utilization - highlight if > 40%
                let util_color = if self.utilization > 40.0 {
                    crt.color(ColorElement::MeterValueNotice)
                } else {
                    value_attr
                };

                let read_str = Self::human_unit(self.read_rate);
                let write_str = Self::human_unit(self.write_rate);
                let util_str = format!("{:.1}%", self.utilization);

                // Format: "Disk IO: <util>% read: <rate>KiB/s write: <rate>KiB/s"
                crt.with_window(|win| {
                    let _ = win.attrset(util_color);
                    let _ = win.addstr(&util_str);

                    let _ = win.attrset(text_attr);
                    let _ = win.addstr(" read: ");

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
                // This is a simplified approach - C htop doesn't support graph mode for DiskIO
                if self.status == RateStatus::Data {
                    const MAX_RATE: f64 = 1024.0 * 1024.0 * 1024.0; // 1 GB/s
                    let combined = self.read_rate + self.write_rate;
                    let normalized = (combined / MAX_RATE).min(1.0);

                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100);
                }

                let graph_data = self.graph_data.borrow();
                draw_graph(crt, x, y, width, self.height(), &graph_data, self.caption());
            }
            MeterMode::Led => {
                // Format rate values for LED display
                let caption = format!("{}: ", self.caption());
                if self.status != RateStatus::Data {
                    let text = match self.status {
                        RateStatus::Init => "----",
                        RateStatus::Stale => "OLD",
                        RateStatus::NoData => "N/A",
                        RateStatus::Data => unreachable!(),
                    };
                    draw_led(crt, x, y, width, &caption, text);
                } else {
                    // Same format as Text mode: "<util>% read: <rate>iB/s write: <rate>iB/s"
                    let text = format!(
                        "{:.1}% read: {}iB/s write: {}iB/s",
                        self.utilization,
                        Self::human_unit(self.read_rate),
                        Self::human_unit(self.write_rate)
                    );
                    draw_led(crt, x, y, width, &caption, &text);
                }
            }
            MeterMode::StackedGraph => {
                // StackedGraph not supported for DiskIO meter
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Machine;

    // ==================== Constructor Tests ====================

    #[test]
    fn test_diskio_meter_new() {
        let meter = DiskIOMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.status, RateStatus::Init);
        assert_eq!(meter.read_rate, 0.0);
        assert_eq!(meter.write_rate, 0.0);
        assert_eq!(meter.utilization, 0.0);
        assert_eq!(meter.num_disks, 0);
    }

    #[test]
    fn test_diskio_meter_default() {
        let meter = DiskIOMeter::default();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.status, RateStatus::Init);
    }

    // ==================== human_unit Tests ====================
    // DiskIO human_unit converts bytes/sec to KiB/s first, then scales

    #[test]
    fn test_human_unit_bytes_to_kilobytes() {
        // 0 bytes/sec = 0 KiB/s
        assert_eq!(DiskIOMeter::human_unit(0.0), "0.00K");

        // 1024 bytes/sec = 1.00 KiB/s
        assert_eq!(DiskIOMeter::human_unit(1024.0), "1.00K");

        // 5120 bytes/sec = 5.00 KiB/s
        assert_eq!(DiskIOMeter::human_unit(5120.0), "5.00K");
    }

    #[test]
    fn test_human_unit_kilobytes_precision() {
        // Values < 10 get 2 decimal places
        assert_eq!(DiskIOMeter::human_unit(1024.0), "1.00K");
        assert_eq!(DiskIOMeter::human_unit(9.0 * 1024.0), "9.00K");

        // Values 10-99 get 1 decimal place
        assert_eq!(DiskIOMeter::human_unit(10.0 * 1024.0), "10.0K");
        assert_eq!(DiskIOMeter::human_unit(99.0 * 1024.0), "99.0K");

        // Values >= 100 get 0 decimal places
        assert_eq!(DiskIOMeter::human_unit(100.0 * 1024.0), "100K");
        assert_eq!(DiskIOMeter::human_unit(999.0 * 1024.0), "999K");
    }

    #[test]
    fn test_human_unit_megabytes() {
        // 1000 KiB/s = 1000K, which becomes ~0.98M (scales at 1000, not 1024)
        // Actually the scaling happens when val >= 1000

        // 1 MiB/s = 1024 * 1024 bytes/sec
        let mib = 1024.0 * 1024.0;
        assert_eq!(DiskIOMeter::human_unit(mib), "1.00M");

        // 10 MiB/s
        assert_eq!(DiskIOMeter::human_unit(10.0 * mib), "10.0M");

        // 100 MiB/s
        assert_eq!(DiskIOMeter::human_unit(100.0 * mib), "100M");
    }

    #[test]
    fn test_human_unit_gigabytes() {
        // 1 GiB/s = 1024 * 1024 * 1024 bytes/sec
        let gib = 1024.0 * 1024.0 * 1024.0;
        assert_eq!(DiskIOMeter::human_unit(gib), "1.00G");

        // 5 GiB/s
        assert_eq!(DiskIOMeter::human_unit(5.0 * gib), "5.00G");
    }

    #[test]
    fn test_human_unit_typical_disk_rates() {
        // Typical HDD sequential: ~150 MB/s
        let hdd_rate = 150.0 * 1024.0 * 1024.0;
        assert_eq!(DiskIOMeter::human_unit(hdd_rate), "150M");

        // Typical SSD: ~500 MB/s
        let ssd_rate = 500.0 * 1024.0 * 1024.0;
        assert_eq!(DiskIOMeter::human_unit(ssd_rate), "500M");

        // NVMe SSD: ~3 GB/s
        let nvme_rate = 3.0 * 1024.0 * 1024.0 * 1024.0;
        assert_eq!(DiskIOMeter::human_unit(nvme_rate), "3.00G");
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_diskio_meter_update_init() {
        let mut meter = DiskIOMeter::new();
        let machine = Machine::default();

        // No previous data -> Init status
        meter.update(&machine);
        assert_eq!(meter.status, RateStatus::Init);
    }

    #[test]
    fn test_diskio_meter_update_with_data() {
        let mut meter = DiskIOMeter::new();
        let mut machine = Machine::default();

        machine.disk_io_last_update = 1000;
        machine.realtime_ms = 1500; // 500ms since last update
        machine.disk_io_read_rate = 1024.0 * 1024.0; // 1 MiB/s
        machine.disk_io_write_rate = 512.0 * 1024.0; // 512 KiB/s
        machine.disk_io_utilization = 50.0;
        machine.disk_io_num_disks = 2;

        meter.update(&machine);

        assert_eq!(meter.status, RateStatus::Data);
        assert_eq!(meter.read_rate, 1024.0 * 1024.0);
        assert_eq!(meter.write_rate, 512.0 * 1024.0);
        assert_eq!(meter.utilization, 50.0);
        assert_eq!(meter.num_disks, 2);
    }

    #[test]
    fn test_diskio_meter_update_stale() {
        let mut meter = DiskIOMeter::new();
        let mut machine = Machine::default();

        machine.disk_io_last_update = 1000;
        machine.disk_io_num_disks = 1; // Need disks to not be Init
        machine.realtime_ms = 32000; // 31 seconds since last update (> 30s = stale)

        meter.update(&machine);
        assert_eq!(meter.status, RateStatus::Stale);
    }

    // ==================== Meter Trait Tests ====================

    #[test]
    fn test_diskio_meter_name() {
        let meter = DiskIOMeter::new();
        assert_eq!(meter.name(), "DiskIO");
    }

    #[test]
    fn test_diskio_meter_caption() {
        let meter = DiskIOMeter::new();
        assert_eq!(meter.caption(), "Disk IO");
    }

    #[test]
    fn test_diskio_meter_mode() {
        let mut meter = DiskIOMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_diskio_meter_height() {
        let mut meter = DiskIOMeter::new();

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.height(), 1);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.height(), 1);

        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.height(), 4);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.height(), 3);
    }
}
