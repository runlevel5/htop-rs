//! CPU Meter

#![allow(clippy::too_many_arguments)] // Drawing functions naturally have many parameters

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
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn cpu_range(&self, total_cpus: usize) -> (usize, usize) {
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
    pub(crate) fn format_frequency(frequency: f64) -> String {
        if frequency > 0.0 && frequency.is_finite() {
            format!("{:4}MHz", frequency as u32)
        } else {
            "N/A".to_string()
        }
    }

    /// Format CPU display text for Text/LED modes (matches C htop CPUMeter_display)
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn format_cpu_display_text(
        user: f64,
        system: f64,
        nice: f64,
        irq: f64,
        softirq: f64,
        steal: f64,
        guest: f64,
        iowait: f64,
        frequency: f64,
        detailed_cpu_time: bool,
        show_cpu_frequency: bool,
    ) -> String {
        let mut text = String::new();

        if detailed_cpu_time {
            // Detailed: ":5.1% sy:5.1% ni:5.1% hi:5.1% si:5.1% st:5.1% gu:5.1% wa:5.1%"
            text.push_str(&format!(":{:5.1}% ", user));
            text.push_str(&format!("sy:{:5.1}% ", system));
            text.push_str(&format!("ni:{:5.1}% ", nice));
            text.push_str(&format!("hi:{:5.1}% ", irq));
            text.push_str(&format!("si:{:5.1}% ", softirq));
            if steal >= 0.0 {
                text.push_str(&format!("st:{:5.1}% ", steal));
            }
            if guest >= 0.0 {
                text.push_str(&format!("gu:{:5.1}% ", guest));
            }
            text.push_str(&format!("wa:{:5.1}% ", iowait));
        } else {
            // Non-detailed: ":5.1% sys:5.1% low:5.1% vir:5.1%"
            let kernel = system + irq + softirq;
            let virt = steal + guest;

            text.push_str(&format!(":{:5.1}% ", user));
            text.push_str(&format!("sys:{:5.1}% ", kernel));
            text.push_str(&format!("low:{:5.1}% ", nice));
            if irq >= 0.0 {
                text.push_str(&format!("vir:{:5.1}% ", virt));
            }
        }

        if show_cpu_frequency {
            text.push_str(&format!("freq: {} ", Self::format_frequency(frequency)));
        }

        text
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
        use crate::ncurses_compat::*;

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

    /// Internal helper to draw CPU text with truncation support.
    /// Takes individual CPU values and a pre-formatted caption.
    #[allow(clippy::too_many_arguments)]
    fn draw_cpu_text_internal(
        crt: &Crt,
        caption: &str,
        user: f64,
        nice: f64,
        system: f64,
        irq: f64,
        softirq: f64,
        steal: f64,
        guest: f64,
        iowait: f64,
        frequency: f64,
        x: i32,
        y: i32,
        width: i32,
        detailed_cpu_time: bool,
        show_cpu_frequency: bool,
    ) {
        use crate::ncurses_compat::*;
        use crate::ui::ColorElement;

        if width <= 0 {
            return;
        }

        let max_x = x + width;
        let mut cur_x = x;

        // Helper to print a string with truncation
        let mut print_str = |s: &str, attr: attr_t| -> bool {
            if cur_x >= max_x {
                return false;
            }
            attrset(attr);
            let available = (max_x - cur_x) as usize;
            let to_print: String = s.chars().take(available).collect();
            let printed_len = to_print.chars().count() as i32;
            mv(y, cur_x);
            let _ = addstr(&to_print);
            cur_x += printed_len;
            cur_x < max_x
        };

        let caption_attr = crt.color(ColorElement::MeterText);
        let value_attr = crt.color(ColorElement::MeterValue);

        // Draw caption
        if !print_str(caption, caption_attr) {
            return;
        }

        if detailed_cpu_time {
            // Detailed: show breakdown like C htop
            // Format: ":5.1% sy:5.1% ni:5.1% hi:5.1% si:5.1% st:5.1% gu:5.1% wa:5.1%"
            if !print_str(":", caption_attr) {
                return;
            }
            if !print_str(
                &format!("{:5.1}% ", user),
                crt.color(ColorElement::CpuNormal),
            ) {
                return;
            }
            if !print_str("sy:", caption_attr) {
                return;
            }
            if !print_str(
                &format!("{:5.1}% ", system),
                crt.color(ColorElement::CpuSystem),
            ) {
                return;
            }
            if !print_str("ni:", caption_attr) {
                return;
            }
            if !print_str(
                &format!("{:5.1}% ", nice),
                crt.color(ColorElement::CpuNiceText),
            ) {
                return;
            }
            if !print_str("hi:", caption_attr) {
                return;
            }
            if !print_str(&format!("{:5.1}% ", irq), crt.color(ColorElement::CpuIrq)) {
                return;
            }
            if !print_str("si:", caption_attr) {
                return;
            }
            if !print_str(
                &format!("{:5.1}% ", softirq),
                crt.color(ColorElement::CpuSoftIrq),
            ) {
                return;
            }
            // st: only shown if steal is non-negative (i.e., supported)
            if steal >= 0.0 {
                if !print_str("st:", caption_attr) {
                    return;
                }
                if !print_str(
                    &format!("{:5.1}% ", steal),
                    crt.color(ColorElement::CpuSteal),
                ) {
                    return;
                }
            }
            // gu: only shown if guest is non-negative (i.e., supported)
            if guest >= 0.0 {
                if !print_str("gu:", caption_attr) {
                    return;
                }
                if !print_str(
                    &format!("{:5.1}% ", guest),
                    crt.color(ColorElement::CpuGuest),
                ) {
                    return;
                }
            }
            if !print_str("wa:", caption_attr) {
                return;
            }
            if !print_str(
                &format!("{:5.1}% ", iowait),
                crt.color(ColorElement::CpuIOWait),
            ) {
                return;
            }
        } else {
            // Non-detailed: simpler display
            let kernel = system + irq + softirq;
            let virt = steal + guest;

            if !print_str(":", caption_attr) {
                return;
            }
            if !print_str(
                &format!("{:5.1}% ", user),
                crt.color(ColorElement::CpuNormal),
            ) {
                return;
            }
            if !print_str("sys:", caption_attr) {
                return;
            }
            if !print_str(
                &format!("{:5.1}% ", kernel),
                crt.color(ColorElement::CpuSystem),
            ) {
                return;
            }
            if !print_str("low:", caption_attr) {
                return;
            }
            if !print_str(
                &format!("{:5.1}% ", nice),
                crt.color(ColorElement::CpuNiceText),
            ) {
                return;
            }
            // vir: only shown if IRQ is non-negative (used as proxy for virtualization support)
            if irq >= 0.0 {
                if !print_str("vir:", caption_attr) {
                    return;
                }
                if !print_str(
                    &format!("{:5.1}% ", virt),
                    crt.color(ColorElement::CpuGuest),
                ) {
                    return;
                }
            }
        }

        if show_cpu_frequency {
            if !print_str("freq: ", caption_attr) {
                return;
            }
            let _ = print_str(
                &format!("{} ", Self::format_frequency(frequency)),
                value_attr,
            );
        }

        attrset(crt.color(ColorElement::ResetColor));
    }

    /// Draw a single CPU in text mode (matches C htop CPUMeter_display)
    /// Respects width limit and truncates output accordingly
    fn draw_cpu_text(
        crt: &Crt,
        cpu: &crate::core::CpuData,
        cpu_idx: usize,
        x: i32,
        y: i32,
        width: i32,
        count_from_one: bool,
        detailed_cpu_time: bool,
        show_cpu_frequency: bool,
    ) {
        // Apply count_cpus_from_one setting
        let display_id = if count_from_one { cpu_idx + 1 } else { cpu_idx };
        let caption = format!("{:>3}", display_id);

        Self::draw_cpu_text_internal(
            crt,
            &caption,
            cpu.user_percent,
            cpu.nice_percent,
            cpu.system_percent,
            cpu.irq_percent,
            cpu.softirq_percent,
            cpu.steal_percent,
            cpu.guest_percent,
            cpu.iowait_percent,
            cpu.frequency,
            x,
            y,
            width,
            detailed_cpu_time,
            show_cpu_frequency,
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

        // Number of rows of CPUs (divided by columns)
        let nrows = num_cpus.div_ceil(self.columns) as i32;

        // LED mode is 3 rows tall per CPU row
        if self.mode == MeterMode::Led {
            nrows * 3
        } else {
            nrows
        }
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
                match self.selection {
                    CpuSelection::All | CpuSelection::Left | CpuSelection::Right => {
                        // Multi-CPU text mode: draw each CPU on its own row in column layout
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

                                Self::draw_cpu_text(
                                    crt,
                                    cpu,
                                    cpu_idx,
                                    col_x,
                                    row_y,
                                    col_width,
                                    settings.count_cpus_from_one,
                                    settings.detailed_cpu_time,
                                    settings.show_cpu_frequency,
                                );
                            }
                        }
                    }
                    _ => {
                        // Single CPU or average - draw one text line with proper colors
                        let caption = match self.selection {
                            CpuSelection::Average => "Avg".to_string(),
                            CpuSelection::Cpu(n) => {
                                let display_id = if settings.count_cpus_from_one {
                                    n + 1
                                } else {
                                    n
                                };
                                format!("{:>3}", display_id)
                            }
                            _ => "CPU".to_string(),
                        };

                        Self::draw_cpu_text_internal(
                            crt,
                            &caption,
                            self.user,
                            self.nice,
                            self.system,
                            self.irq,
                            self.softirq,
                            self.steal,
                            self.guest,
                            self.iowait,
                            self.frequency,
                            x,
                            y,
                            width,
                            settings.detailed_cpu_time,
                            settings.show_cpu_frequency,
                        );
                    }
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
                match self.selection {
                    CpuSelection::All | CpuSelection::Left | CpuSelection::Right => {
                        // Multi-CPU LED mode: draw each CPU in a grid layout
                        // LED mode is 3 rows tall per meter
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
                        let led_height = 3; // LED mode is 3 rows tall

                        for (i, &cpu_idx) in cpu_indices.iter().enumerate() {
                            if let Some(cpu) = machine.cpus.get(cpu_idx) {
                                let col = i / nrows;
                                let row = i % nrows;
                                let d = if col > diff { diff } else { col };
                                let col_x = x + (col as i32 * col_width) + d as i32;
                                let row_y = y + (row as i32 * led_height);

                                // Apply count_cpus_from_one setting
                                let display_id = if settings.count_cpus_from_one {
                                    cpu_idx + 1
                                } else {
                                    cpu_idx
                                };
                                let caption = format!("{:>3}", display_id);

                                // Format text like Text mode (matching C htop CPUMeter_display)
                                let text = Self::format_cpu_display_text(
                                    cpu.user_percent,
                                    cpu.system_percent,
                                    cpu.nice_percent,
                                    cpu.irq_percent,
                                    cpu.softirq_percent,
                                    cpu.steal_percent,
                                    cpu.guest_percent,
                                    cpu.iowait_percent,
                                    cpu.frequency,
                                    settings.detailed_cpu_time,
                                    settings.show_cpu_frequency,
                                );
                                super::draw_led(crt, col_x, row_y, col_width, &caption, &text);
                            }
                        }
                    }
                    _ => {
                        // Single CPU or average - draw one LED display
                        let caption = match self.selection {
                            CpuSelection::Average => "Avg".to_string(),
                            CpuSelection::Cpu(n) => {
                                let display_id = if settings.count_cpus_from_one {
                                    n + 1
                                } else {
                                    n
                                };
                                format!("{:>3}", display_id)
                            }
                            _ => "CPU".to_string(),
                        };

                        // Format text like Text mode (matching C htop CPUMeter_display)
                        let text = Self::format_cpu_display_text(
                            self.user,
                            self.system,
                            self.nice,
                            self.irq,
                            self.softirq,
                            self.steal,
                            self.guest,
                            self.iowait,
                            self.frequency,
                            settings.detailed_cpu_time,
                            settings.show_cpu_frequency,
                        );
                        super::draw_led(crt, x, y, width, &caption, &text);
                    }
                }
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
    use crate::core::{CpuData, Machine};

    // =========================================================================
    // Helper functions for creating test data
    // =========================================================================

    fn create_test_cpu_data(
        user: f64,
        nice: f64,
        system: f64,
        irq: f64,
        softirq: f64,
        steal: f64,
        guest: f64,
        iowait: f64,
        frequency: f64,
    ) -> CpuData {
        let mut cpu = CpuData::new();
        cpu.user_percent = user;
        cpu.nice_percent = nice;
        cpu.system_percent = system;
        cpu.irq_percent = irq;
        cpu.softirq_percent = softirq;
        cpu.steal_percent = steal;
        cpu.guest_percent = guest;
        cpu.iowait_percent = iowait;
        cpu.frequency = frequency;
        cpu
    }

    fn create_test_machine_with_cpus(num_cpus: usize) -> Machine {
        let mut machine = Machine::default();
        for i in 0..num_cpus {
            let cpu = create_test_cpu_data(
                10.0 + i as f64, // user
                1.0,             // nice
                5.0,             // system
                0.5,             // irq
                0.3,             // softirq
                0.0,             // steal
                0.0,             // guest
                2.0,             // iowait
                2400.0,          // frequency
            );
            machine.cpus.push(cpu);
        }
        // Set avg_cpu
        machine.avg_cpu = create_test_cpu_data(
            15.0,   // user
            1.0,    // nice
            5.0,    // system
            0.5,    // irq
            0.3,    // softirq
            0.1,    // steal
            0.1,    // guest
            2.0,    // iowait
            2400.0, // frequency
        );
        machine
    }

    // =========================================================================
    // Constructor tests
    // =========================================================================

    #[test]
    fn test_cpu_meter_new_with_cpu_index() {
        let meter = CpuMeter::new(Some(3));
        assert!(matches!(meter.selection, CpuSelection::Cpu(3)));
        assert_eq!(meter.mode, MeterMode::Bar);
        assert_eq!(meter.columns, 1);
    }

    #[test]
    fn test_cpu_meter_new_without_cpu_index() {
        let meter = CpuMeter::new(None);
        assert!(matches!(meter.selection, CpuSelection::Average));
    }

    #[test]
    fn test_cpu_meter_average() {
        let meter = CpuMeter::average();
        assert!(matches!(meter.selection, CpuSelection::Average));
        assert_eq!(meter.mode, MeterMode::Bar);
    }

    #[test]
    fn test_cpu_meter_all() {
        let meter = CpuMeter::all(4);
        assert!(matches!(meter.selection, CpuSelection::All));
        assert_eq!(meter.columns, 4);
    }

    #[test]
    fn test_cpu_meter_all_minimum_columns() {
        // Columns should be at least 1
        let meter = CpuMeter::all(0);
        assert_eq!(meter.columns, 1);
    }

    #[test]
    fn test_cpu_meter_left() {
        let meter = CpuMeter::left(2);
        assert!(matches!(meter.selection, CpuSelection::Left));
        assert_eq!(meter.columns, 2);
    }

    #[test]
    fn test_cpu_meter_right() {
        let meter = CpuMeter::right(8);
        assert!(matches!(meter.selection, CpuSelection::Right));
        assert_eq!(meter.columns, 8);
    }

    // =========================================================================
    // cpu_range tests
    // =========================================================================

    #[test]
    fn test_cpu_range_all() {
        let meter = CpuMeter::all(1);
        assert_eq!(meter.cpu_range(8), (0, 8));
        assert_eq!(meter.cpu_range(1), (0, 1));
        assert_eq!(meter.cpu_range(16), (0, 16));
    }

    #[test]
    fn test_cpu_range_left_even() {
        let meter = CpuMeter::left(1);
        // 8 CPUs: left half is 0-4
        assert_eq!(meter.cpu_range(8), (0, 4));
        // 4 CPUs: left half is 0-2
        assert_eq!(meter.cpu_range(4), (0, 2));
    }

    #[test]
    fn test_cpu_range_left_odd() {
        let meter = CpuMeter::left(1);
        // 7 CPUs: left half is 0-4 (div_ceil(7,2) = 4)
        assert_eq!(meter.cpu_range(7), (0, 4));
        // 5 CPUs: left half is 0-3
        assert_eq!(meter.cpu_range(5), (0, 3));
    }

    #[test]
    fn test_cpu_range_right_even() {
        let meter = CpuMeter::right(1);
        // 8 CPUs: right half is 4-8
        assert_eq!(meter.cpu_range(8), (4, 8));
        // 4 CPUs: right half is 2-4
        assert_eq!(meter.cpu_range(4), (2, 4));
    }

    #[test]
    fn test_cpu_range_right_odd() {
        let meter = CpuMeter::right(1);
        // 7 CPUs: right half is 4-7 (div_ceil(7,2) = 4)
        assert_eq!(meter.cpu_range(7), (4, 7));
        // 5 CPUs: right half is 3-5
        assert_eq!(meter.cpu_range(5), (3, 5));
    }

    #[test]
    fn test_cpu_range_specific_cpu() {
        let meter = CpuMeter::new(Some(5));
        assert_eq!(meter.cpu_range(8), (5, 6));
        assert_eq!(meter.cpu_range(16), (5, 6));
    }

    #[test]
    fn test_cpu_range_average() {
        let meter = CpuMeter::average();
        // Average mode returns (0, 0) - no specific CPUs
        assert_eq!(meter.cpu_range(8), (0, 0));
    }

    // =========================================================================
    // format_frequency tests
    // =========================================================================

    #[test]
    fn test_format_frequency_normal() {
        assert_eq!(CpuMeter::format_frequency(2400.0), "2400MHz");
        assert_eq!(CpuMeter::format_frequency(3600.0), "3600MHz");
        assert_eq!(CpuMeter::format_frequency(800.0), " 800MHz");
    }

    #[test]
    fn test_format_frequency_zero() {
        assert_eq!(CpuMeter::format_frequency(0.0), "N/A");
    }

    #[test]
    fn test_format_frequency_negative() {
        assert_eq!(CpuMeter::format_frequency(-100.0), "N/A");
    }

    #[test]
    fn test_format_frequency_infinity() {
        assert_eq!(CpuMeter::format_frequency(f64::INFINITY), "N/A");
        assert_eq!(CpuMeter::format_frequency(f64::NEG_INFINITY), "N/A");
    }

    #[test]
    fn test_format_frequency_nan() {
        assert_eq!(CpuMeter::format_frequency(f64::NAN), "N/A");
    }

    #[test]
    fn test_format_frequency_fractional() {
        // Frequency is truncated to integer
        assert_eq!(CpuMeter::format_frequency(2400.75), "2400MHz");
        assert_eq!(CpuMeter::format_frequency(2400.25), "2400MHz");
    }

    // =========================================================================
    // format_cpu_display_text tests
    // =========================================================================

    #[test]
    fn test_format_cpu_display_text_detailed() {
        let text = CpuMeter::format_cpu_display_text(
            50.0,   // user
            10.0,   // system
            5.0,    // nice
            1.0,    // irq
            0.5,    // softirq
            0.2,    // steal
            0.1,    // guest
            3.0,    // iowait
            2400.0, // frequency
            true,   // detailed_cpu_time
            false,  // show_cpu_frequency
        );

        assert!(text.contains(": 50.0%"));
        assert!(text.contains("sy: 10.0%"));
        assert!(text.contains("ni:  5.0%"));
        assert!(text.contains("hi:  1.0%"));
        assert!(text.contains("si:  0.5%"));
        assert!(text.contains("st:  0.2%"));
        assert!(text.contains("gu:  0.1%"));
        assert!(text.contains("wa:  3.0%"));
        assert!(!text.contains("freq:"));
    }

    #[test]
    fn test_format_cpu_display_text_detailed_with_frequency() {
        let text = CpuMeter::format_cpu_display_text(
            50.0, 10.0, 5.0, 1.0, 0.5, 0.2, 0.1, 3.0, 2400.0, true, // detailed_cpu_time
            true, // show_cpu_frequency
        );

        assert!(text.contains("freq: 2400MHz"));
    }

    #[test]
    fn test_format_cpu_display_text_non_detailed() {
        let text = CpuMeter::format_cpu_display_text(
            50.0, // user
            10.0, // system
            5.0,  // nice
            1.0,  // irq
            0.5,  // softirq
            0.2,  // steal
            0.1,  // guest
            3.0,  // iowait
            2400.0, false, // detailed_cpu_time
            false, // show_cpu_frequency
        );

        // Non-detailed should combine values
        assert!(text.contains(": 50.0%")); // user
                                           // sys = system + irq + softirq = 10 + 1 + 0.5 = 11.5
        assert!(text.contains("sys: 11.5%"));
        assert!(text.contains("low:  5.0%")); // nice
                                              // vir = steal + guest = 0.2 + 0.1 = 0.3
        assert!(text.contains("vir:  0.3%"));
    }

    #[test]
    fn test_format_cpu_display_text_negative_steal_guest_hidden() {
        // When steal/guest are negative (unsupported), they should be hidden in detailed mode
        let text = CpuMeter::format_cpu_display_text(
            50.0, 10.0, 5.0, 1.0, 0.5, -1.0, // steal (negative = unsupported)
            -1.0, // guest (negative = unsupported)
            3.0, 0.0, true, // detailed_cpu_time
            false,
        );

        assert!(!text.contains("st:"));
        assert!(!text.contains("gu:"));
    }

    // =========================================================================
    // update tests
    // =========================================================================

    #[test]
    fn test_cpu_meter_update_average() {
        let mut meter = CpuMeter::average();
        let machine = create_test_machine_with_cpus(4);

        meter.update(&machine);

        assert_eq!(meter.user, 15.0);
        assert_eq!(meter.nice, 1.0);
        assert_eq!(meter.system, 5.0);
        assert_eq!(meter.irq, 0.5);
        assert_eq!(meter.softirq, 0.3);
        assert_eq!(meter.steal, 0.1);
        assert_eq!(meter.guest, 0.1);
        assert_eq!(meter.iowait, 2.0);
        assert_eq!(meter.frequency, 2400.0);
        assert_eq!(meter.cpu_count, 4);
    }

    #[test]
    fn test_cpu_meter_update_specific_cpu() {
        let mut meter = CpuMeter::new(Some(2));
        let machine = create_test_machine_with_cpus(4);

        meter.update(&machine);

        // CPU 2 has user = 10.0 + 2 = 12.0
        assert_eq!(meter.user, 12.0);
        assert_eq!(meter.cpu_count, 4);
    }

    #[test]
    fn test_cpu_meter_update_all_cpus() {
        let mut meter = CpuMeter::all(2);
        let machine = create_test_machine_with_cpus(8);

        meter.update(&machine);

        // All/Left/Right modes use avg_cpu for cached values
        assert_eq!(meter.user, 15.0);
        assert_eq!(meter.cpu_count, 8);
    }

    #[test]
    fn test_cpu_meter_update_cpu_count() {
        let mut meter = CpuMeter::all(1);

        // First update with 4 CPUs
        let machine4 = create_test_machine_with_cpus(4);
        meter.update(&machine4);
        assert_eq!(meter.cpu_count, 4);

        // Update with 8 CPUs
        let machine8 = create_test_machine_with_cpus(8);
        meter.update(&machine8);
        assert_eq!(meter.cpu_count, 8);
    }

    // =========================================================================
    // height tests
    // =========================================================================

    #[test]
    fn test_cpu_meter_height_bar_mode_single() {
        let meter = CpuMeter::average();
        assert_eq!(meter.height(), 1);
    }

    #[test]
    fn test_cpu_meter_height_text_mode_single() {
        let mut meter = CpuMeter::average();
        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.height(), 1);
    }

    #[test]
    fn test_cpu_meter_height_graph_mode() {
        let mut meter = CpuMeter::average();
        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.height(), DEFAULT_GRAPH_HEIGHT);
    }

    #[test]
    fn test_cpu_meter_height_led_mode_single() {
        let mut meter = CpuMeter::average();
        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.height(), 3);
    }

    #[test]
    fn test_cpu_meter_height_all_cpus_bar_mode() {
        let mut meter = CpuMeter::all(1);
        meter.cpu_count = 8;
        // 8 CPUs in 1 column = 8 rows
        assert_eq!(meter.height(), 8);
    }

    #[test]
    fn test_cpu_meter_height_all_cpus_multiple_columns() {
        let mut meter = CpuMeter::all(2);
        meter.cpu_count = 8;
        // 8 CPUs in 2 columns = 4 rows
        assert_eq!(meter.height(), 4);

        let mut meter4 = CpuMeter::all(4);
        meter4.cpu_count = 8;
        // 8 CPUs in 4 columns = 2 rows
        assert_eq!(meter4.height(), 2);
    }

    #[test]
    fn test_cpu_meter_height_all_cpus_led_mode() {
        let mut meter = CpuMeter::all(2);
        meter.cpu_count = 8;
        meter.set_mode(MeterMode::Led);
        // 8 CPUs in 2 columns = 4 rows, LED is 3 rows per row = 12
        assert_eq!(meter.height(), 12);
    }

    #[test]
    fn test_cpu_meter_height_left_cpus() {
        let mut meter = CpuMeter::left(1);
        meter.cpu_count = 8;
        // Left half of 8 CPUs = 4 CPUs, 1 column = 4 rows
        assert_eq!(meter.height(), 4);
    }

    #[test]
    fn test_cpu_meter_height_right_cpus_odd() {
        let mut meter = CpuMeter::right(1);
        meter.cpu_count = 7;
        // Right half of 7 CPUs: height uses div_ceil(7,2) = 4
        // Note: This is different from cpu_range which returns (4, 7) = 3 CPUs
        // The height calculation uses the same formula for both Left and Right
        // to maintain symmetry in the UI layout
        assert_eq!(meter.height(), 4);
    }

    #[test]
    fn test_cpu_meter_height_graph_mode_ignores_cpu_count() {
        let mut meter = CpuMeter::all(1);
        meter.cpu_count = 16;
        meter.set_mode(MeterMode::Graph);
        // Graph mode always returns DEFAULT_GRAPH_HEIGHT regardless of CPU count
        assert_eq!(meter.height(), DEFAULT_GRAPH_HEIGHT);
    }

    // =========================================================================
    // Meter trait tests
    // =========================================================================

    #[test]
    fn test_cpu_meter_name() {
        let meter = CpuMeter::average();
        assert_eq!(meter.name(), "CPU");
    }

    #[test]
    fn test_cpu_meter_caption() {
        let meter_avg = CpuMeter::average();
        assert_eq!(meter_avg.caption(), "Avg");

        let meter_all = CpuMeter::all(1);
        assert_eq!(meter_all.caption(), "CPU");

        let meter_cpu = CpuMeter::new(Some(0));
        assert_eq!(meter_cpu.caption(), "CPUn");
    }

    #[test]
    fn test_cpu_meter_mode_get_set() {
        let mut meter = CpuMeter::average();
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    // =========================================================================
    // Edge case tests
    // =========================================================================

    #[test]
    fn test_cpu_meter_update_empty_machine() {
        let mut meter = CpuMeter::new(Some(0));
        let machine = Machine::default();

        meter.update(&machine);

        // cpu_count should be at least 1
        assert_eq!(meter.cpu_count, 1);
        // Values from avg_cpu (defaults to 0)
        assert_eq!(meter.user, 0.0);
    }

    #[test]
    fn test_cpu_meter_update_out_of_bounds_cpu() {
        let mut meter = CpuMeter::new(Some(100)); // CPU index 100 doesn't exist
        let machine = create_test_machine_with_cpus(4);

        meter.update(&machine);

        // Should not crash, values remain at defaults
        assert_eq!(meter.user, 0.0);
        assert_eq!(meter.cpu_count, 4);
    }

    #[test]
    fn test_cpu_range_single_cpu_system() {
        let meter_all = CpuMeter::all(1);
        assert_eq!(meter_all.cpu_range(1), (0, 1));

        let meter_left = CpuMeter::left(1);
        assert_eq!(meter_left.cpu_range(1), (0, 1)); // div_ceil(1,2) = 1

        let meter_right = CpuMeter::right(1);
        assert_eq!(meter_right.cpu_range(1), (1, 1)); // Empty range for right half of 1 CPU
    }

    // =========================================================================
    // Bar values calculation tests (detailed_cpu_time option)
    // =========================================================================

    /// Helper to build bar values array (same logic as draw_cpu_bar_single)
    fn build_bar_values(cpu: &CpuData, detailed_cpu_time: bool) -> Vec<(f64, ColorElement)> {
        use crate::ui::ColorElement;

        if detailed_cpu_time {
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
        }
    }

    /// Helper to calculate total (same logic as draw_cpu_bar_single)
    fn calculate_total(cpu: &CpuData, account_guest: bool, detailed_cpu_time: bool) -> f64 {
        cpu.user_percent
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
            + cpu.iowait_percent
    }

    #[test]
    fn test_bar_values_detailed_mode_segment_count() {
        let cpu = create_test_cpu_data(
            20.0, // user
            5.0,  // nice
            10.0, // system
            2.0,  // irq
            1.0,  // softirq
            0.5,  // steal
            0.3,  // guest
            3.0,  // iowait
            2400.0,
        );

        // Detailed mode should have 8 segments
        let values = build_bar_values(&cpu, true);
        assert_eq!(values.len(), 8, "Detailed mode should have 8 segments");
    }

    #[test]
    fn test_bar_values_non_detailed_mode_segment_count() {
        let cpu = create_test_cpu_data(20.0, 5.0, 10.0, 2.0, 1.0, 0.5, 0.3, 3.0, 2400.0);

        // Non-detailed mode should have 4 segments
        let values = build_bar_values(&cpu, false);
        assert_eq!(values.len(), 4, "Non-detailed mode should have 4 segments");
    }

    #[test]
    fn test_bar_values_detailed_mode_individual_values() {
        use crate::ui::ColorElement;

        let cpu = create_test_cpu_data(
            20.0, // user
            5.0,  // nice
            10.0, // system
            2.0,  // irq
            1.0,  // softirq
            0.5,  // steal
            0.3,  // guest
            3.0,  // iowait
            2400.0,
        );

        let values = build_bar_values(&cpu, true);

        // Verify each segment has correct value and color
        assert_eq!(values[0], (20.0, ColorElement::CpuNormal)); // user
        assert_eq!(values[1], (5.0, ColorElement::CpuNice)); // nice
        assert_eq!(values[2], (10.0, ColorElement::CpuSystem)); // system
        assert_eq!(values[3], (2.0, ColorElement::CpuIrq)); // irq
        assert_eq!(values[4], (1.0, ColorElement::CpuSoftIrq)); // softirq
        assert_eq!(values[5], (0.5, ColorElement::CpuSteal)); // steal
        assert_eq!(values[6], (0.3, ColorElement::CpuGuest)); // guest
        assert_eq!(values[7], (3.0, ColorElement::CpuIOWait)); // iowait
    }

    #[test]
    fn test_bar_values_non_detailed_mode_combined_values() {
        use crate::ui::ColorElement;

        let cpu = create_test_cpu_data(
            20.0, // user
            5.0,  // nice
            10.0, // system
            2.0,  // irq
            1.0,  // softirq
            0.5,  // steal
            0.3,  // guest
            3.0,  // iowait
            2400.0,
        );

        let values = build_bar_values(&cpu, false);

        // user stays as-is
        assert_eq!(values[0], (20.0, ColorElement::CpuNormal));
        // nice stays as-is
        assert_eq!(values[1], (5.0, ColorElement::CpuNice));
        // kernel = system + irq + softirq = 10 + 2 + 1 = 13
        assert_eq!(values[2], (13.0, ColorElement::CpuSystem));
        // virtual = steal + guest = 0.5 + 0.3 = 0.8
        assert_eq!(values[3], (0.8, ColorElement::CpuGuest));
    }

    #[test]
    fn test_bar_total_detailed_with_account_guest() {
        let cpu = create_test_cpu_data(
            20.0, // user
            5.0,  // nice
            10.0, // system
            2.0,  // irq
            1.0,  // softirq
            0.5,  // steal
            0.3,  // guest
            3.0,  // iowait
            2400.0,
        );

        // With account_guest=true, guest is included
        let total = calculate_total(&cpu, true, true);
        // 20 + 5 + 10 + 2 + 1 + 0.5 + 0.3 + 3 = 41.8
        assert!((total - 41.8).abs() < 0.001);
    }

    #[test]
    fn test_bar_total_detailed_without_account_guest() {
        let cpu = create_test_cpu_data(
            20.0, // user
            5.0,  // nice
            10.0, // system
            2.0,  // irq
            1.0,  // softirq
            0.5,  // steal
            0.3,  // guest
            3.0,  // iowait
            2400.0,
        );

        // With account_guest=false and detailed=true, guest is excluded
        let total = calculate_total(&cpu, false, true);
        // 20 + 5 + 10 + 2 + 1 + 0.5 + 0 + 3 = 41.5
        assert!((total - 41.5).abs() < 0.001);
    }

    #[test]
    fn test_bar_total_non_detailed_always_includes_guest() {
        let cpu = create_test_cpu_data(
            20.0, // user
            5.0,  // nice
            10.0, // system
            2.0,  // irq
            1.0,  // softirq
            0.5,  // steal
            0.3,  // guest
            3.0,  // iowait
            2400.0,
        );

        // With detailed=false, guest is always included regardless of account_guest
        let total_with = calculate_total(&cpu, true, false);
        let total_without = calculate_total(&cpu, false, false);

        // Both should be the same: 20 + 5 + 10 + 2 + 1 + 0.5 + 0.3 + 3 = 41.8
        assert!((total_with - 41.8).abs() < 0.001);
        assert!((total_without - 41.8).abs() < 0.001);
        assert!((total_with - total_without).abs() < 0.001);
    }

    #[test]
    fn test_bar_values_zero_values() {
        let cpu = create_test_cpu_data(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);

        let detailed = build_bar_values(&cpu, true);
        let non_detailed = build_bar_values(&cpu, false);

        // All values should be 0
        assert!(detailed.iter().all(|(v, _)| *v == 0.0));
        assert!(non_detailed.iter().all(|(v, _)| *v == 0.0));

        // Total should be 0
        assert_eq!(calculate_total(&cpu, true, true), 0.0);
        assert_eq!(calculate_total(&cpu, false, false), 0.0);
    }

    #[test]
    fn test_bar_values_high_load() {
        // Test with high CPU load (near 100%)
        let cpu = create_test_cpu_data(
            80.0, // user
            5.0,  // nice
            10.0, // system
            1.0,  // irq
            0.5,  // softirq
            0.0,  // steal
            0.0,  // guest
            3.5,  // iowait
            3600.0,
        );

        let total = calculate_total(&cpu, true, true);
        // 80 + 5 + 10 + 1 + 0.5 + 0 + 0 + 3.5 = 100
        assert!((total - 100.0).abs() < 0.001);
    }
}
