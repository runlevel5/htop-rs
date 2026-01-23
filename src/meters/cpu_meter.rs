//! CPU Meter

use std::cell::RefCell;

use super::{draw_graph, GraphData, Meter, MeterMode, DEFAULT_GRAPH_HEIGHT};
use crate::core::{Machine, Settings};
use crate::ui::bar_meter_char;
use crate::ui::ColorElement;
use crate::ui::Crt;

/// CPU selection mode
#[derive(Debug, Clone, Copy)]
pub enum CpuSelection {
    /// Specific CPU
    Cpu(usize),
    /// Average of all CPUs
    Average,
    /// All CPUs (multiple rows)
    All,
    /// Left half of CPUs
    Left,
    /// Right half of CPUs
    Right,
}

/// CPU Meter
#[derive(Debug)]
pub struct CpuMeter {
    selection: CpuSelection,
    mode: MeterMode,
    /// Number of columns to display CPUs in (1, 2, 4, or 8)
    columns: usize,
    /// Cached values (for single CPU / average mode)
    user: f64,
    nice: f64,
    system: f64,
    irq: f64,
    softirq: f64,
    steal: f64,
    guest: f64,
    iowait: f64,
    frequency: f64,
    /// Number of CPUs (cached for height calculation)
    cpu_count: usize,
    /// Graph data for historical display (RefCell for interior mutability)
    graph_data: RefCell<GraphData>,
}

impl CpuMeter {
    pub fn new(cpu: Option<usize>) -> Self {
        CpuMeter {
            selection: cpu.map(CpuSelection::Cpu).unwrap_or(CpuSelection::Average),
            mode: MeterMode::Bar,
            columns: 1,
            user: 0.0,
            nice: 0.0,
            system: 0.0,
            irq: 0.0,
            softirq: 0.0,
            steal: 0.0,
            guest: 0.0,
            iowait: 0.0,
            frequency: 0.0,
            cpu_count: 1,
            graph_data: RefCell::new(GraphData::new()),
        }
    }

    pub fn average() -> Self {
        CpuMeter {
            selection: CpuSelection::Average,
            ..Self::new(None)
        }
    }

    pub fn all(columns: usize) -> Self {
        CpuMeter {
            selection: CpuSelection::All,
            columns: columns.max(1),
            ..Self::new(None)
        }
    }

    pub fn left(columns: usize) -> Self {
        CpuMeter {
            selection: CpuSelection::Left,
            columns: columns.max(1),
            ..Self::new(None)
        }
    }

    pub fn right(columns: usize) -> Self {
        CpuMeter {
            selection: CpuSelection::Right,
            columns: columns.max(1),
            ..Self::new(None)
        }
    }

    /// Get the range of CPUs this meter displays
    fn cpu_range(&self, total_cpus: usize) -> (usize, usize) {
        match self.selection {
            CpuSelection::All => (0, total_cpus),
            CpuSelection::Left => (0, total_cpus.div_ceil(2)),
            CpuSelection::Right => (total_cpus.div_ceil(2), total_cpus),
            CpuSelection::Cpu(n) => (n, n + 1),
            CpuSelection::Average => (0, 0),
        }
    }

    /// Format CPU frequency for display
    /// Returns frequency string in MHz format, or "N/A" if not available
    fn format_frequency(frequency: f64) -> String {
        if frequency > 0.0 && frequency.is_finite() {
            format!("{:4}MHz", frequency as u32)
        } else {
            "N/A".to_string()
        }
    }

    /// Draw a CPU bar with percentage text inside (like C htop)
    /// When show_cpu_usage is false, no percentage text is displayed (matches C htop showCPUUsage)
    fn draw_cpu_bar_internal(
        crt: &Crt,
        caption: &str,
        values: &[(f64, ColorElement)],
        total_percent: f64,
        x: i32,
        y: i32,
        width: i32,
        show_cpu_usage: bool,
        show_cpu_frequency: bool,
        frequency: f64,
    ) {
        use ncurses::*;

        // Draw caption (exactly 3 chars)
        let caption_attr = crt.color(ColorElement::MeterText);
        mv(y, x);
        attrset(caption_attr);
        let _ = addstr(&format!("{:>3}", caption));

        // Bar area starts after caption
        let bar_x = x + 3;
        let bar_width = width - 3;

        if bar_width < 4 {
            return;
        }

        // Draw brackets
        let bracket_attr = crt.color(ColorElement::BarBorder);
        attrset(bracket_attr);
        mvaddch(y, bar_x, '[' as u32);
        mvaddch(y, bar_x + bar_width - 1, ']' as u32);

        // Inner bar width (between brackets)
        let inner_width = (bar_width - 2) as usize;

        // Build the text to display inside the bar (right-aligned)
        // Matches C htop: cpuUsageBuffer + " " + cpuFrequencyBuffer (all inside the bar)
        let mut text = String::new();
        if show_cpu_usage {
            text.push_str(&format!("{:.1}%", total_percent));
        }
        if show_cpu_frequency {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&Self::format_frequency(frequency));
        }

        let text_len = text.len();
        let padding = inner_width.saturating_sub(text_len);

        // Calculate how many chars each segment takes
        let mut bar_chars = Vec::new();
        let mut total_bar = 0usize;
        for (value, color) in values {
            let chars = if total_percent > 0.0 || *value > 0.0 {
                ((*value / 100.0) * inner_width as f64).ceil() as usize
            } else {
                0
            };
            let chars = chars.min(inner_width - total_bar);
            bar_chars.push((chars, *color));
            total_bar += chars;
        }

        // Draw the bar content with text overlaid
        mv(y, bar_x + 1);
        let mut pos = 0;
        for (idx, (chars, color)) in bar_chars.iter().enumerate() {
            let attr = crt.color(*color);
            attrset(attr);
            let bar_ch = bar_meter_char(crt.color_scheme, idx);
            for _ in 0..*chars {
                if !text.is_empty() && pos >= padding && pos - padding < text_len {
                    let ch = text.chars().nth(pos - padding).unwrap_or(bar_ch);
                    addch(ch as u32);
                } else {
                    addch(bar_ch as u32);
                }
                pos += 1;
            }
        }

        // Fill remaining with shadow (and text if extends into shadow)
        let shadow_attr = crt.color(ColorElement::BarShadow);
        attrset(shadow_attr);
        while pos < inner_width {
            if !text.is_empty() && pos >= padding && pos - padding < text_len {
                let ch = text.chars().nth(pos - padding).unwrap_or(' ');
                addch(ch as u32);
            } else {
                addch(' ' as u32);
            }
            pos += 1;
        }

        // Reset color at the end
        attrset(crt.color(ColorElement::ResetColor));
    }

    /// Draw a single CPU bar
    fn draw_cpu_bar(
        &self,
        crt: &Crt,
        cpu: &crate::core::CpuData,
        cpu_idx: usize,
        x: i32,
        y: i32,
        width: i32,
        count_from_one: bool,
        show_cpu_usage: bool,
        account_guest: bool,
        detailed_cpu_time: bool,
        show_cpu_frequency: bool,
    ) {
        // Build values array based on detailed_cpu_time setting
        // When detailed: show all 8 segments
        // When not detailed: combine system+irq+softirq into "kernel", combine steal+guest into one
        let values = if detailed_cpu_time {
            vec![
                (cpu.user_percent, ColorElement::CpuNormal),
                (cpu.nice_percent, ColorElement::CpuNice),
                (cpu.system_percent, ColorElement::CpuSystem),
                (cpu.irq_percent, ColorElement::CpuIrq),
                (cpu.softirq_percent, ColorElement::CpuSoftIrq),
                (cpu.steal_percent, ColorElement::CpuSteal),
                (cpu.guest_percent, ColorElement::CpuGuest),
                (cpu.iowait_percent, ColorElement::CpuIOWait),
            ]
        } else {
            // Non-detailed: combine into simpler view (matches C htop)
            // normal (user), nice, kernel (system+irq+softirq), virtual (steal+guest)
            vec![
                (cpu.user_percent, ColorElement::CpuNormal),
                (cpu.nice_percent, ColorElement::CpuNice),
                (
                    cpu.system_percent + cpu.irq_percent + cpu.softirq_percent,
                    ColorElement::CpuSystem,
                ),
                (
                    cpu.steal_percent + cpu.guest_percent,
                    ColorElement::CpuGuest,
                ),
            ]
        };

        // Calculate total - guest is only included when account_guest is true and detailed mode
        let total = cpu.user_percent
            + cpu.nice_percent
            + cpu.system_percent
            + cpu.irq_percent
            + cpu.softirq_percent
            + cpu.steal_percent
            + if account_guest || !detailed_cpu_time {
                cpu.guest_percent
            } else {
                0.0
            }
            + cpu.iowait_percent;

        // Apply count_cpus_from_one setting (like C htop Settings_cpuId macro)
        let display_id = if count_from_one { cpu_idx + 1 } else { cpu_idx };
        Self::draw_cpu_bar_internal(
            crt,
            &format!("{}", display_id),
            &values,
            total,
            x,
            y,
            width,
            show_cpu_usage,
            show_cpu_frequency,
            cpu.frequency,
        );
    }
}

impl Meter for CpuMeter {
    fn name(&self) -> &'static str {
        "CPU"
    }

    fn caption(&self) -> &str {
        match self.selection {
            CpuSelection::Cpu(_) => "CPUn",
            CpuSelection::Average => "Avg",
            CpuSelection::All => "CPU",
            CpuSelection::Left => "CPU",
            CpuSelection::Right => "CPU",
        }
    }

    fn update(&mut self, machine: &Machine) {
        // Update CPU count for height calculation
        self.cpu_count = machine.cpus.len().max(1);

        let cpu = match self.selection {
            CpuSelection::Cpu(n) => machine.cpus.get(n),
            CpuSelection::Average
            | CpuSelection::All
            | CpuSelection::Left
            | CpuSelection::Right => Some(&machine.avg_cpu),
        };

        if let Some(cpu) = cpu {
            self.user = cpu.user_percent;
            self.nice = cpu.nice_percent;
            self.system = cpu.system_percent;
            self.irq = cpu.irq_percent;
            self.softirq = cpu.softirq_percent;
            self.steal = cpu.steal_percent;
            self.guest = cpu.guest_percent;
            self.iowait = cpu.iowait_percent;
            self.frequency = cpu.frequency;
        }
    }

    fn height(&self) -> i32 {
        // Graph mode has fixed height regardless of CPU selection
        if self.mode == MeterMode::Graph {
            return DEFAULT_GRAPH_HEIGHT;
        }

        let num_cpus = match self.selection {
            CpuSelection::All => self.cpu_count,
            CpuSelection::Left | CpuSelection::Right => self.cpu_count.div_ceil(2),
            _ => return self.mode.default_height(),
        };
        // Divide by columns, rounding up
        num_cpus.div_ceil(self.columns) as i32
    }

    fn draw(&self, crt: &Crt, machine: &Machine, settings: &Settings, x: i32, y: i32, width: i32) {
        match self.mode {
            MeterMode::Bar => {
                match self.selection {
                    CpuSelection::All | CpuSelection::Left | CpuSelection::Right => {
                        let (start_cpu, end_cpu) = self.cpu_range(machine.cpus.len());
                        let cpu_indices: Vec<usize> = (start_cpu..end_cpu).collect();
                        let num_cpus = cpu_indices.len();

                        if num_cpus == 0 {
                            return;
                        }

                        // Calculate layout (column-first like C htop)
                        let ncol = self.columns;
                        let nrows = num_cpus.div_ceil(ncol);
                        let col_width = width / ncol as i32;
                        let diff = (width % ncol as i32) as usize;

                        for (i, &cpu_idx) in cpu_indices.iter().enumerate() {
                            if let Some(cpu) = machine.cpus.get(cpu_idx) {
                                let col = i / nrows;
                                let row = i % nrows;
                                let d = if col > diff { diff } else { col };
                                let col_x = x + (col as i32 * col_width) + d as i32;
                                let row_y = y + row as i32;
                                self.draw_cpu_bar(
                                    crt,
                                    cpu,
                                    cpu_idx,
                                    col_x,
                                    row_y,
                                    col_width,
                                    settings.count_cpus_from_one,
                                    settings.show_cpu_usage,
                                    settings.account_guest_in_cpu_meter,
                                    settings.detailed_cpu_time,
                                    settings.show_cpu_frequency,
                                );
                            }
                        }
                    }
                    _ => {
                        // Single CPU or average - draw one bar
                        let caption = match self.selection {
                            CpuSelection::Average => "Avg".to_string(),
                            CpuSelection::Cpu(n) => {
                                // Apply count_cpus_from_one setting
                                let display_id = if settings.count_cpus_from_one {
                                    n + 1
                                } else {
                                    n
                                };
                                format!("{}", display_id)
                            }
                            _ => "CPU".to_string(),
                        };

                        // Build values array based on detailed_cpu_time setting
                        let values = if settings.detailed_cpu_time {
                            vec![
                                (self.user, ColorElement::CpuNormal),
                                (self.nice, ColorElement::CpuNice),
                                (self.system, ColorElement::CpuSystem),
                                (self.irq, ColorElement::CpuIrq),
                                (self.softirq, ColorElement::CpuSoftIrq),
                                (self.steal, ColorElement::CpuSteal),
                                (self.guest, ColorElement::CpuGuest),
                                (self.iowait, ColorElement::CpuIOWait),
                            ]
                        } else {
                            // Non-detailed: combine into simpler view
                            vec![
                                (self.user, ColorElement::CpuNormal),
                                (self.nice, ColorElement::CpuNice),
                                (
                                    self.system + self.irq + self.softirq,
                                    ColorElement::CpuSystem,
                                ),
                                (self.steal + self.guest, ColorElement::CpuGuest),
                            ]
                        };

                        // Calculate total - guest only included when account_guest is true and detailed
                        let total = self.user
                            + self.nice
                            + self.system
                            + self.irq
                            + self.softirq
                            + self.steal
                            + if settings.account_guest_in_cpu_meter || !settings.detailed_cpu_time
                            {
                                self.guest
                            } else {
                                0.0
                            }
                            + self.iowait;

                        Self::draw_cpu_bar_internal(
                            crt,
                            &caption,
                            &values,
                            total,
                            x,
                            y,
                            width,
                            settings.show_cpu_usage,
                            settings.show_cpu_frequency,
                            self.frequency,
                        );
                    }
                }
            }
            MeterMode::Text => {
                // Text mode respects detailed_cpu_time (matches C htop CPUMeter_display)
                let freq_suffix = if settings.show_cpu_frequency {
                    format!(" freq: {} ", Self::format_frequency(self.frequency))
                } else {
                    String::new()
                };

                if settings.detailed_cpu_time {
                    // Detailed: show breakdown like C htop
                    // Format: ":5.1% sy:5.1% ni:5.1% hi:5.1% si:5.1% st:5.1% gu:5.1% wa:5.1%"
                    let text = format!(
                        "{:.1}% sy:{:.1}% ni:{:.1}% hi:{:.1}% si:{:.1}% st:{:.1}% gu:{:.1}% wa:{:.1}%{}",
                        self.user,
                        self.system,
                        self.nice,
                        self.irq,
                        self.softirq,
                        self.steal,
                        self.guest,
                        self.iowait,
                        freq_suffix
                    );
                    super::draw_text(crt, x, y, "CPU:", &text);
                } else {
                    // Non-detailed: simpler display
                    let kernel = self.system + self.irq + self.softirq;
                    let virt = self.steal + self.guest;
                    let text = format!(
                        "{:.1}% sys:{:.1}% low:{:.1}% vir:{:.1}%{}",
                        self.user, kernel, self.nice, virt, freq_suffix
                    );
                    super::draw_text(crt, x, y, "CPU:", &text);
                }
            }
            MeterMode::Graph => {
                // Calculate total CPU usage (normalized to 0.0-1.0)
                let total = self.user
                    + self.nice
                    + self.system
                    + self.irq
                    + self.softirq
                    + self.steal
                    + if settings.account_guest_in_cpu_meter || !settings.detailed_cpu_time {
                        self.guest
                    } else {
                        0.0
                    }
                    + self.iowait;

                // Normalize to 0.0-1.0 (CPU percentage is already 0-100)
                let normalized = total / 100.0;

                // Record the value in graph data
                {
                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100); // delay is in tenths of a second
                }

                // Draw the graph
                let graph_data = self.graph_data.borrow();
                let caption = match self.selection {
                    CpuSelection::Average => "Avg",
                    CpuSelection::Cpu(_n) => {
                        // For individual CPU, we can't return a &str with dynamic content
                        // Just use "CPU" for now (matches C htop behavior for graph)
                        "CPU"
                    }
                    _ => "CPU",
                };
                draw_graph(crt, x, y, width, self.height(), &graph_data, caption);
            }
            MeterMode::Led => {
                // Calculate total CPU usage
                let total = self.user
                    + self.nice
                    + self.system
                    + self.irq
                    + self.softirq
                    + self.steal
                    + if settings.account_guest_in_cpu_meter || !settings.detailed_cpu_time {
                        self.guest
                    } else {
                        0.0
                    }
                    + self.iowait;

                // Format the text (matches C htop CPUMeter_display text buffer)
                let text = format!("{:.1}%", total);
                super::draw_led(crt, x, y, width, "CPU ", &text);
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
