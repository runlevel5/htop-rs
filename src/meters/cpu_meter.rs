//! CPU Meter

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
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
    /// Number of CPUs (cached for height calculation)
    cpu_count: usize,
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
            cpu_count: 1,
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
            CpuSelection::Left => (0, (total_cpus + 1) / 2),
            CpuSelection::Right => ((total_cpus + 1) / 2, total_cpus),
            CpuSelection::Cpu(n) => (n, n + 1),
            CpuSelection::Average => (0, 0),
        }
    }

    /// Draw a CPU bar with percentage text inside (like C htop)
    fn draw_cpu_bar_internal(
        crt: &Crt,
        caption: &str,
        values: &[(f64, ColorElement)],
        total_percent: f64,
        x: i32,
        y: i32,
        width: i32,
    ) {
        use ncurses::*;

        // Draw caption (exactly 3 chars)
        let caption_attr = crt.color(ColorElement::MeterText);
        mv(y, x);
        attron(caption_attr);
        let _ = addstr(&format!("{:>3}", caption));
        attroff(caption_attr);

        // Bar area starts after caption
        let bar_x = x + 3;
        let bar_width = width - 3;

        if bar_width < 4 {
            return;
        }

        // Draw brackets
        let bracket_attr = crt.color(ColorElement::BarBorder);
        attron(bracket_attr);
        mvaddch(y, bar_x, '[' as u32);
        mvaddch(y, bar_x + bar_width - 1, ']' as u32);
        attroff(bracket_attr);

        // Inner bar width (between brackets)
        let inner_width = (bar_width - 2) as usize;

        // Format the percentage text to show inside the bar (right-aligned)
        let text = format!("{:.1}%", total_percent);
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
        for (chars, color) in &bar_chars {
            let attr = crt.color(*color);
            attron(attr);
            for _ in 0..*chars {
                if pos >= padding && pos - padding < text_len {
                    let ch = text.chars().nth(pos - padding).unwrap_or('|');
                    addch(ch as u32);
                } else {
                    addch('|' as u32);
                }
                pos += 1;
            }
            attroff(attr);
        }

        // Fill remaining with shadow (and text if extends into shadow)
        let shadow_attr = crt.color(ColorElement::BarShadow);
        attron(shadow_attr);
        while pos < inner_width {
            if pos >= padding && pos - padding < text_len {
                let ch = text.chars().nth(pos - padding).unwrap_or(' ');
                addch(ch as u32);
            } else {
                addch(' ' as u32);
            }
            pos += 1;
        }
        attroff(shadow_attr);
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
    ) {
        let values = vec![
            (cpu.user_percent, ColorElement::CpuNormal),
            (cpu.nice_percent, ColorElement::CpuNice),
            (cpu.system_percent, ColorElement::CpuSystem),
            (cpu.irq_percent, ColorElement::CpuIrq),
            (cpu.softirq_percent, ColorElement::CpuSoftIrq),
            (cpu.steal_percent, ColorElement::CpuSteal),
            (cpu.guest_percent, ColorElement::CpuGuest),
            (cpu.iowait_percent, ColorElement::CpuIOWait),
        ];

        let total = cpu.user_percent
            + cpu.nice_percent
            + cpu.system_percent
            + cpu.irq_percent
            + cpu.softirq_percent
            + cpu.steal_percent
            + cpu.guest_percent
            + cpu.iowait_percent;

        Self::draw_cpu_bar_internal(crt, &format!("{}", cpu_idx), &values, total, x, y, width);
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
        }
    }

    fn height(&self) -> i32 {
        let num_cpus = match self.selection {
            CpuSelection::All => self.cpu_count,
            CpuSelection::Left | CpuSelection::Right => (self.cpu_count + 1) / 2,
            _ => return 1,
        };
        // Divide by columns, rounding up
        ((num_cpus + self.columns - 1) / self.columns) as i32
    }

    fn draw(&self, crt: &Crt, machine: &Machine, _settings: &Settings, x: i32, y: i32, width: i32) {
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
                        let nrows = (num_cpus + ncol - 1) / ncol;
                        let col_width = width / ncol as i32;
                        let diff = (width % ncol as i32) as usize;

                        for (i, &cpu_idx) in cpu_indices.iter().enumerate() {
                            if let Some(cpu) = machine.cpus.get(cpu_idx) {
                                let col = i / nrows;
                                let row = i % nrows;
                                let d = if col > diff { diff } else { col };
                                let col_x = x + (col as i32 * col_width) + d as i32;
                                let row_y = y + row as i32;
                                self.draw_cpu_bar(crt, cpu, cpu_idx, col_x, row_y, col_width);
                            }
                        }
                    }
                    _ => {
                        // Single CPU or average - draw one bar
                        let caption = match self.selection {
                            CpuSelection::Average => "Avg".to_string(),
                            CpuSelection::Cpu(n) => format!("{}", n),
                            _ => "CPU".to_string(),
                        };

                        let values = vec![
                            (self.user, ColorElement::CpuNormal),
                            (self.nice, ColorElement::CpuNice),
                            (self.system, ColorElement::CpuSystem),
                            (self.irq, ColorElement::CpuIrq),
                            (self.softirq, ColorElement::CpuSoftIrq),
                            (self.steal, ColorElement::CpuSteal),
                            (self.guest, ColorElement::CpuGuest),
                            (self.iowait, ColorElement::CpuIOWait),
                        ];

                        let total = self.user
                            + self.nice
                            + self.system
                            + self.irq
                            + self.softirq
                            + self.steal
                            + self.guest
                            + self.iowait;

                        Self::draw_cpu_bar_internal(crt, &caption, &values, total, x, y, width);
                    }
                }
            }
            MeterMode::Text => {
                let total = self.user
                    + self.nice
                    + self.system
                    + self.irq
                    + self.softirq
                    + self.steal
                    + self.guest
                    + self.iowait;
                let text = format!("{:.1}%", total);
                super::draw_text(crt, x, y, "CPU: ", &text);
            }
            _ => {
                // Graph/LED modes not implemented - fall back to text
                let total = self.user
                    + self.nice
                    + self.system
                    + self.irq
                    + self.softirq
                    + self.steal
                    + self.guest
                    + self.iowait;
                let text = format!("{:.1}%", total);
                super::draw_text(crt, x, y, "CPU: ", &text);
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
