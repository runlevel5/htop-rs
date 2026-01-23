//! Tasks Meter

use std::cell::RefCell;

use super::{draw_graph, GraphData, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::bar_meter_char;
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Tasks Meter
///
/// Displays task counts exactly like C htop:
/// - Text mode: "Tasks: N, M thr, K kthr; R running"
/// - Bar mode: shows running tasks relative to total tasks
/// - Graph mode: historical graph of running tasks
/// - LED mode: "R/N" (running/total)
#[derive(Debug)]
pub struct TasksMeter {
    mode: MeterMode,
    /// Total tasks (processes, not including threads)
    total_tasks: u32,
    /// Running tasks
    running: u32,
    /// Userland threads
    userland_threads: u32,
    /// Kernel threads
    kernel_threads: u32,
    /// Total tasks including threads (for bar/graph total)
    total_all: u32,
    /// Graph data for historical display (RefCell for interior mutability)
    graph_data: RefCell<GraphData>,
}

impl Default for TasksMeter {
    fn default() -> Self {
        Self {
            mode: MeterMode::Text,
            total_tasks: 0,
            running: 0,
            userland_threads: 0,
            kernel_threads: 0,
            total_all: 0,
            graph_data: RefCell::new(GraphData::new()),
        }
    }
}

impl TasksMeter {
    pub fn new() -> Self {
        TasksMeter::default()
    }
}

impl Meter for TasksMeter {
    fn name(&self) -> &'static str {
        "Tasks"
    }

    fn caption(&self) -> &str {
        "Tasks: "
    }

    fn update(&mut self, machine: &Machine) {
        self.kernel_threads = machine.kernel_threads;
        self.userland_threads = machine.userland_threads;
        // Total tasks = total - kernel threads - userland threads (just processes)
        self.total_tasks = machine
            .total_tasks
            .saturating_sub(machine.kernel_threads)
            .saturating_sub(machine.userland_threads);
        self.running = machine.running_tasks.min(machine.active_cpus);
        self.total_all = machine.total_tasks;
    }

    fn draw(
        &self,
        crt: &Crt,
        _machine: &Machine,
        settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        use ncurses::*;

        match self.mode {
            MeterMode::Text => {
                let text_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::MeterValue);
                let running_attr = crt.color(ColorElement::TasksRunning);
                let shadow_attr = crt.color(ColorElement::MeterShadow);

                mv(y, x);

                // "Tasks: "
                attrset(text_attr);
                let _ = addstr("Tasks: ");

                // Process count (total tasks - threads)
                attrset(value_attr);
                let _ = addstr(&format!("{}", self.total_tasks));

                // ", N thr" - userland threads (shadowed if hideUserlandThreads)
                let (thr_text_attr, thr_value_attr) = if settings.hide_userland_threads {
                    (shadow_attr, shadow_attr)
                } else {
                    (text_attr, running_attr)
                };

                attrset(thr_text_attr);
                let _ = addstr(", ");

                attrset(thr_value_attr);
                let _ = addstr(&format!("{}", self.userland_threads));

                attrset(thr_text_attr);
                let _ = addstr(" thr");

                // ", K kthr" - kernel threads (shadowed if hideKernelThreads)
                let (kthr_text_attr, kthr_value_attr) = if settings.hide_kernel_threads {
                    (shadow_attr, shadow_attr)
                } else {
                    (text_attr, running_attr)
                };

                attrset(kthr_text_attr);
                let _ = addstr(", ");

                attrset(kthr_value_attr);
                let _ = addstr(&format!("{}", self.kernel_threads));

                attrset(kthr_text_attr);
                let _ = addstr(" kthr");

                // "; R running" - always normal colors
                attrset(text_attr);
                let _ = addstr("; ");

                attrset(running_attr);
                let _ = addstr(&format!("{}", self.running));

                attrset(text_attr);
                let _ = addstr(" running");

                attrset(crt.color(ColorElement::ResetColor));
            }
            MeterMode::Bar => {
                // Bar mode: show running tasks relative to total tasks
                // C htop uses 4 values: kernelThreads, userlandThreads, processes, running
                // The bar shows stacked segments for each category

                // Draw caption "Tsk" (3 chars like other meters)
                let caption_attr = crt.color(ColorElement::MeterText);
                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("Tsk");

                // Bar area starts after caption
                let bar_x = x + 3;
                let bar_width = width - 3;

                if bar_width < 4 {
                    attrset(crt.color(ColorElement::ResetColor));
                    return;
                }

                // Draw brackets
                let bracket_attr = crt.color(ColorElement::BarBorder);
                attrset(bracket_attr);
                mvaddch(y, bar_x, '[' as u32);
                mvaddch(y, bar_x + bar_width - 1, ']' as u32);

                // Inner bar width (between brackets)
                let inner_width = (bar_width - 2) as usize;

                // Total for bar is total_all (all tasks including threads)
                let total = (self.total_all as f64).max(1.0);

                // Format the text to show inside the bar (running/total)
                let text = format!("{}/{}", self.running, self.total_all);
                let text_len = text.len();
                let padding = inner_width.saturating_sub(text_len);

                // Calculate bar segments matching C htop TasksMeter_attributes order:
                // CPU_SYSTEM (kernel threads), PROCESS_THREAD (userland threads),
                // PROCESS (processes), TASKS_RUNNING (running)
                // We'll show: kernel threads, userland threads, processes (in different colors)
                let values: Vec<(f64, ColorElement)> = vec![
                    (self.kernel_threads as f64, ColorElement::CpuSystem),        // CPU_SYSTEM
                    (self.userland_threads as f64, ColorElement::ProcessThread),  // PROCESS_THREAD
                    (self.total_tasks as f64, ColorElement::Process),             // PROCESS
                ];

                // Calculate how many chars each segment takes
                let mut bar_chars = Vec::new();
                let mut total_bar = 0usize;
                for (value, color) in &values {
                    let chars = if total > 0.0 {
                        ((*value / total) * inner_width as f64).ceil() as usize
                    } else {
                        0
                    };
                    let chars = chars.min(inner_width - total_bar);
                    bar_chars.push((chars, *color));
                    total_bar += chars;
                }

                // Draw the bar content
                mv(y, bar_x + 1);
                let mut pos = 0;
                for (idx, (chars, color)) in bar_chars.iter().enumerate() {
                    let attr = crt.color(*color);
                    attrset(attr);
                    let bar_ch = bar_meter_char(crt.color_scheme, idx);
                    for _ in 0..*chars {
                        if pos >= padding && pos - padding < text_len {
                            // Draw text character
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
                    if pos >= padding && pos - padding < text_len {
                        let ch = text.chars().nth(pos - padding).unwrap_or(' ');
                        addch(ch as u32);
                    } else {
                        addch(' ' as u32);
                    }
                    pos += 1;
                }

                attrset(crt.color(ColorElement::ResetColor));
            }
            MeterMode::Graph => {
                // Graph mode: show historical running tasks
                // Normalized to total tasks
                let total = (self.total_all as f64).max(1.0);
                let normalized = (self.running as f64 / total).clamp(0.0, 1.0);

                // Record the value in graph data
                {
                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record(normalized, settings.delay * 100);
                }

                // Draw the graph
                let graph_data = self.graph_data.borrow();
                draw_graph(crt, x, y, width, self.height(), &graph_data, "Tsk");
            }
            MeterMode::Led => {
                // LED mode: show "R/N" like C htop txtBuffer format
                let text = format!("{}/{}", self.running, self.total_all);
                super::draw_led(crt, x, y, width, "Tsk", &text);
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
