//! Network IO Meter
//!
//! Displays network receive/transmit rates and packet counts, matching C htop's NetworkIOMeter.

use std::cell::RefCell;

use super::{draw_bar_with_text, draw_graph, draw_led, BarSegment, GraphData, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Rate status for network IO meter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateStatus {
    Init,   // Initializing, no previous data
    Data,   // Have valid rate data
    Stale,  // Data is stale (> 30 seconds old)
    NoData, // No data available (read failed)
}

/// Network IO Meter
///
/// Displays network receive/transmit rates and packet counts.
/// In text mode, shows "rx: XiB/s tx: YiB/s rx_pps/tx_pps pkt/s".
#[derive(Debug)]
pub struct NetworkIOMeter {
    mode: MeterMode,
    status: RateStatus,
    /// Cached receive rate in bytes per second
    receive_rate: f64,
    /// Cached transmit rate in bytes per second
    transmit_rate: f64,
    /// Packets received per second
    receive_packets: u64,
    /// Packets transmitted per second
    transmit_packets: u64,
    /// Graph data for historical display
    graph_data: RefCell<GraphData>,
}

impl Default for NetworkIOMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkIOMeter {
    pub fn new() -> Self {
        NetworkIOMeter {
            mode: MeterMode::Text, // Default to text mode like C htop
            status: RateStatus::Init,
            receive_rate: 0.0,
            transmit_rate: 0.0,
            receive_packets: 0,
            transmit_packets: 0,
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

impl Meter for NetworkIOMeter {
    fn name(&self) -> &'static str {
        "NetworkIO"
    }

    fn caption(&self) -> &str {
        "Network"
    }

    fn update(&mut self, machine: &Machine) {
        // Check if we have valid data
        if machine.net_io_last_update == 0 {
            self.status = RateStatus::Init;
            return;
        }

        // Check for stale data (> 30 seconds)
        let time_since_update = machine
            .realtime_ms
            .saturating_sub(machine.net_io_last_update);
        if time_since_update > 30000 {
            self.status = RateStatus::Stale;
            return;
        }

        // We have valid data
        self.status = RateStatus::Data;
        self.receive_rate = machine.net_io_receive_rate;
        self.transmit_rate = machine.net_io_transmit_rate;
        self.receive_packets = machine.net_io_receive_packets;
        self.transmit_packets = machine.net_io_transmit_packets;
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
                // Format: "Net[|||||      rx: XiB/s tx: YiB/s rx_pps/tx_pps pkt/s]"

                // Handle non-data states
                let text = if self.status != RateStatus::Data {
                    match self.status {
                        RateStatus::Init => "init".to_string(),
                        RateStatus::Stale => "stale".to_string(),
                        RateStatus::NoData => "no data".to_string(),
                        RateStatus::Data => unreachable!(),
                    }
                } else {
                    // Format: "rx: <rate>iB/s tx: <rate>iB/s <rx_pps>/<tx_pps> pkt/s"
                    let rx_str = Self::human_unit(self.receive_rate);
                    let tx_str = Self::human_unit(self.transmit_rate);
                    format!(
                        "rx:{}iB/s tx:{}iB/s {}/{} pkt/s",
                        rx_str, tx_str, self.receive_packets, self.transmit_packets
                    )
                };

                // Bar segments: receive rate + transmit rate (stacked)
                // Normalize rates for display - use a fixed max of 1Gbps
                const MAX_RATE: f64 = 125_000_000.0; // 1 Gbps = 125 MB/s
                let rx_norm = (self.receive_rate / MAX_RATE).min(1.0);
                let tx_norm = (self.transmit_rate / MAX_RATE).min(1.0);

                let rx_attr = crt.color(ColorElement::MeterValueIORead);
                let tx_attr = crt.color(ColorElement::MeterValueIOWrite);

                let segments = [
                    BarSegment {
                        value: rx_norm,
                        attr: rx_attr,
                    },
                    BarSegment {
                        value: tx_norm,
                        attr: tx_attr,
                    },
                ];

                draw_bar_with_text(crt, x, y, width, "Net", &segments, 1.0, &text);
            }
            MeterMode::Text => {
                // Extract all colors BEFORE with_window to avoid borrow conflicts
                let text_attr = crt.color(ColorElement::MeterText);
                let rx_attr = crt.color(ColorElement::MeterValueIORead);
                let tx_attr = crt.color(ColorElement::MeterValueIOWrite);
                let value_attr = crt.color(ColorElement::MeterValue);
                let warn_attr = crt.color(ColorElement::MeterValueWarn);
                let error_attr = crt.color(ColorElement::MeterValueError);
                let reset_attr = crt.color(ColorElement::ResetColor);

                // Pre-compute formatted strings
                let status = self.status;
                let rx_rate_str = Self::human_unit(self.receive_rate);
                let tx_rate_str = Self::human_unit(self.transmit_rate);
                let rx_packets_str = format!("{}", self.receive_packets);
                let tx_packets_str = format!("{}", self.transmit_packets);

                crt.with_window(|win| {
                    let _ = win.mv(y, x);

                    // Caption prefix
                    let _ = win.attrset(text_attr);
                    let _ = win.addstr("Network: ");

                    // Handle non-data states
                    if status != RateStatus::Data {
                        let (text, attr) = match status {
                            RateStatus::Init => ("initializing...", value_attr),
                            RateStatus::Stale => ("stale data", warn_attr),
                            RateStatus::NoData => ("no data", error_attr),
                            RateStatus::Data => unreachable!(),
                        };
                        let _ = win.attrset(attr);
                        let _ = win.addstr(text);
                        let _ = win.attrset(reset_attr);
                        return;
                    }

                    // "rx: XiB/s tx: YiB/s rx_pps/tx_pps pkt/s"
                    let _ = win.attrset(text_attr);
                    let _ = win.addstr("rx: ");

                    let _ = win.attrset(rx_attr);
                    let _ = win.addstr(&rx_rate_str);
                    let _ = win.addstr("iB/s");

                    let _ = win.attrset(text_attr);
                    let _ = win.addstr(" tx: ");

                    let _ = win.attrset(tx_attr);
                    let _ = win.addstr(&tx_rate_str);
                    let _ = win.addstr("iB/s");

                    // Packet counts
                    let _ = win.attrset(text_attr);
                    let _ = win.addstr(" ");

                    let _ = win.attrset(rx_attr);
                    let _ = win.addstr(&rx_packets_str);

                    let _ = win.attrset(text_attr);
                    let _ = win.addstr("/");

                    let _ = win.attrset(tx_attr);
                    let _ = win.addstr(&tx_packets_str);

                    let _ = win.attrset(text_attr);
                    let _ = win.addstr(" pkt/s");

                    let _ = win.attrset(reset_attr);
                });
            }
            MeterMode::Graph => {
                // For graph mode, use combined rx+tx rate as percentage of max
                if self.status == RateStatus::Data {
                    const MAX_RATE: f64 = 125_000_000.0; // 1 Gbps
                    let combined = self.receive_rate + self.transmit_rate;
                    let normalized = (combined / MAX_RATE).min(1.0);

                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100);
                }

                let graph_data = self.graph_data.borrow();
                draw_graph(crt, x, y, width, self.height(), &graph_data, "Network");
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
                    draw_led(crt, x, y, width, "Network: ", text);
                } else {
                    let text = format!(
                        "rx:{}iB/s tx:{}iB/s",
                        Self::human_unit(self.receive_rate),
                        Self::human_unit(self.transmit_rate)
                    );
                    draw_led(crt, x, y, width, "Network: ", &text);
                }
            }
            MeterMode::StackedGraph => {
                // StackedGraph not supported for NetworkIO meter
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
    fn test_networkio_meter_new() {
        let meter = NetworkIOMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.status, RateStatus::Init);
        assert_eq!(meter.receive_rate, 0.0);
        assert_eq!(meter.transmit_rate, 0.0);
        assert_eq!(meter.receive_packets, 0);
        assert_eq!(meter.transmit_packets, 0);
    }

    #[test]
    fn test_networkio_meter_default() {
        let meter = NetworkIOMeter::default();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.status, RateStatus::Init);
    }

    // ==================== human_unit Tests ====================
    // NetworkIO human_unit is same as DiskIO - converts bytes/sec to KiB/s first

    #[test]
    fn test_human_unit_bytes_to_kilobytes() {
        assert_eq!(NetworkIOMeter::human_unit(0.0), "0.00K");
        assert_eq!(NetworkIOMeter::human_unit(1024.0), "1.00K");
        assert_eq!(NetworkIOMeter::human_unit(5120.0), "5.00K");
    }

    #[test]
    fn test_human_unit_kilobytes_precision() {
        // Values < 10 get 2 decimal places
        assert_eq!(NetworkIOMeter::human_unit(1024.0), "1.00K");

        // Values 10-99 get 1 decimal place
        assert_eq!(NetworkIOMeter::human_unit(10.0 * 1024.0), "10.0K");

        // Values >= 100 get 0 decimal places
        assert_eq!(NetworkIOMeter::human_unit(100.0 * 1024.0), "100K");
    }

    #[test]
    fn test_human_unit_megabytes() {
        let mib = 1024.0 * 1024.0;
        assert_eq!(NetworkIOMeter::human_unit(mib), "1.00M");
        assert_eq!(NetworkIOMeter::human_unit(10.0 * mib), "10.0M");
        assert_eq!(NetworkIOMeter::human_unit(100.0 * mib), "100M");
    }

    #[test]
    fn test_human_unit_typical_network_rates() {
        // 1 Mbps = 125 KB/s = 128000 bytes/sec
        let one_mbps = 125.0 * 1024.0;
        assert_eq!(NetworkIOMeter::human_unit(one_mbps), "125K");

        // 100 Mbps = 12.5 MB/s
        let hundred_mbps = 12.5 * 1024.0 * 1024.0;
        assert_eq!(NetworkIOMeter::human_unit(hundred_mbps), "12.5M");

        // 1 Gbps = 125 MB/s
        let one_gbps = 125.0 * 1024.0 * 1024.0;
        assert_eq!(NetworkIOMeter::human_unit(one_gbps), "125M");
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_networkio_meter_update_init() {
        let mut meter = NetworkIOMeter::new();
        let machine = Machine::default();

        meter.update(&machine);
        assert_eq!(meter.status, RateStatus::Init);
    }

    #[test]
    fn test_networkio_meter_update_with_data() {
        let mut meter = NetworkIOMeter::new();
        let mut machine = Machine::default();

        machine.net_io_last_update = 1000;
        machine.realtime_ms = 1500;
        machine.net_io_receive_rate = 1024.0 * 1024.0; // 1 MiB/s
        machine.net_io_transmit_rate = 512.0 * 1024.0; // 512 KiB/s
        machine.net_io_receive_packets = 1000;
        machine.net_io_transmit_packets = 500;

        meter.update(&machine);

        assert_eq!(meter.status, RateStatus::Data);
        assert_eq!(meter.receive_rate, 1024.0 * 1024.0);
        assert_eq!(meter.transmit_rate, 512.0 * 1024.0);
        assert_eq!(meter.receive_packets, 1000);
        assert_eq!(meter.transmit_packets, 500);
    }

    #[test]
    fn test_networkio_meter_update_stale() {
        let mut meter = NetworkIOMeter::new();
        let mut machine = Machine::default();

        machine.net_io_last_update = 1000;
        machine.realtime_ms = 32000; // > 30 seconds = stale

        meter.update(&machine);
        assert_eq!(meter.status, RateStatus::Stale);
    }

    // ==================== Meter Trait Tests ====================

    #[test]
    fn test_networkio_meter_name() {
        let meter = NetworkIOMeter::new();
        assert_eq!(meter.name(), "NetworkIO");
    }

    #[test]
    fn test_networkio_meter_caption() {
        let meter = NetworkIOMeter::new();
        assert_eq!(meter.caption(), "Network");
    }

    #[test]
    fn test_networkio_meter_mode() {
        let mut meter = NetworkIOMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_networkio_meter_height() {
        let mut meter = NetworkIOMeter::new();

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
