//! Network IO Meter
//!
//! Displays network receive/transmit rates and packet counts, matching C htop's NetworkIOMeter.

use std::cell::RefCell;

use super::{draw_bar, draw_graph, draw_led, GraphData, Meter, MeterMode};
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
/// In text mode, shows "rx: XiB/s tx: YiB/s (rx_pps/tx_pps pps)".
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

impl Meter for NetworkIOMeter {
    fn name(&self) -> &'static str {
        "NetworkIO"
    }

    fn caption(&self) -> &str {
        "Net"
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

    fn draw(&self, crt: &Crt, _machine: &Machine, settings: &Settings, x: i32, y: i32, width: i32) {
        use ncurses::*;

        match self.mode {
            MeterMode::Bar => {
                // Draw caption "Net" (exactly 3 chars)
                let caption_attr = crt.color(ColorElement::MeterText);
                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("Net");

                // Bar area starts after caption
                let bar_x = x + 3;
                let bar_width = width - 3;

                if bar_width < 4 {
                    attrset(crt.color(ColorElement::ResetColor));
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
                    let attr = crt.color(ColorElement::MeterValue);
                    attrset(attr);
                    let _ = mvaddnstr(y, bar_x, text, bar_width);
                    attrset(crt.color(ColorElement::ResetColor));
                    return;
                }

                // Split into two half-width bars like DiskIO
                let col_width = bar_width / 2;
                let diff = bar_width % 2;

                // Normalize rates for display - use a fixed max of 1Gbps per direction
                const MAX_RATE: f64 = 125_000_000.0; // 1 Gbps = 125 MB/s
                let rx_norm = (self.receive_rate / MAX_RATE).min(1.0);
                let tx_norm = (self.transmit_rate / MAX_RATE).min(1.0);

                // Draw receive bar
                let rx_values = [(rx_norm, crt.color(ColorElement::MeterValueIORead) as i32)];
                draw_bar(crt, bar_x, y, col_width, &rx_values, 1.0);

                // Draw transmit bar
                let tx_values = [(tx_norm, crt.color(ColorElement::MeterValueIOWrite) as i32)];
                draw_bar(crt, bar_x + col_width + diff, y, col_width, &tx_values, 1.0);

                attrset(crt.color(ColorElement::ResetColor));
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
                    mv(y, x);
                    attrset(attr);
                    let _ = addstr(text);
                    attrset(crt.color(ColorElement::ResetColor));
                    return;
                }

                let text_attr = crt.color(ColorElement::MeterText);
                let rx_attr = crt.color(ColorElement::MeterValueIORead);
                let tx_attr = crt.color(ColorElement::MeterValueIOWrite);

                mv(y, x);

                // "rx: XiB/s tx: YiB/s (rx_pps/tx_pps pps)"
                attrset(text_attr);
                let _ = addstr("rx: ");

                attrset(rx_attr);
                let _ = addstr(&Self::human_unit(self.receive_rate));
                let _ = addstr("iB/s");

                attrset(text_attr);
                let _ = addstr(" tx: ");

                attrset(tx_attr);
                let _ = addstr(&Self::human_unit(self.transmit_rate));
                let _ = addstr("iB/s");

                // Packet counts
                attrset(text_attr);
                let _ = addstr(" (");

                attrset(rx_attr);
                let _ = addstr(&format!("{}", self.receive_packets));

                attrset(text_attr);
                let _ = addstr("/");

                attrset(tx_attr);
                let _ = addstr(&format!("{}", self.transmit_packets));

                attrset(text_attr);
                let _ = addstr(" pps)");

                attrset(crt.color(ColorElement::ResetColor));
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
                draw_graph(crt, x, y, width, self.height(), &graph_data, "Net");
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
                    draw_led(crt, x, y, width, "Net ", text);
                } else {
                    let text = format!(
                        "rx:{}iB/s tx:{}iB/s",
                        Self::human_unit(self.receive_rate),
                        Self::human_unit(self.transmit_rate)
                    );
                    draw_led(crt, x, y, width, "Net ", &text);
                }
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
