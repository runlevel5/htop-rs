//! Tasks Meter

use std::cell::RefCell;

use super::{
    draw_bar_with_text, draw_graph_self_scaling, draw_tasks_stacked_graph, draw_text_segments,
    BarSegment, GraphData, Meter, MeterMode, TasksStackedGraphData, TextSegment,
    TASKS_STACKED_GRAPH_SEGMENTS,
};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Tasks Meter
///
/// Displays task counts exactly like C htop:
/// - Text mode: "Tasks: N, M thr, K kthr; R running"
/// - Bar mode: shows running tasks relative to total tasks
/// - Graph mode: historical graph of total tasks (self-scaling)
/// - LED mode: "R/N" (running/total)
/// - StackedGraph mode: stacked graph showing kernel/userland/processes/running
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
    /// Stacked graph data for StackedGraph mode
    stacked_graph_data: RefCell<TasksStackedGraphData>,
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
            stacked_graph_data: RefCell::new(TasksStackedGraphData::new()),
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
        crt: &mut Crt,
        _machine: &Machine,
        settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        match self.mode {
            MeterMode::Text => {
                // Extract all colors BEFORE building segments
                let text_attr = crt.color(ColorElement::MeterText);
                let value_attr = crt.color(ColorElement::MeterValue);
                let running_attr = crt.color(ColorElement::TasksRunning);
                let shadow_attr = crt.color(ColorElement::MeterShadow);

                // Pre-compute conditional attrs based on settings
                let (thr_text_attr, thr_value_attr) = if settings.hide_userland_threads {
                    (shadow_attr, shadow_attr)
                } else {
                    (text_attr, running_attr)
                };
                let (kthr_text_attr, kthr_value_attr) = if settings.hide_kernel_threads {
                    (shadow_attr, shadow_attr)
                } else {
                    (text_attr, running_attr)
                };

                // Pre-compute formatted values
                let total_tasks_str = format!("{}", self.total_tasks);
                let userland_threads_str = format!("{}", self.userland_threads);
                let kernel_threads_str = format!("{}", self.kernel_threads);
                let running_str = format!("{}", self.running);

                // Build segments: "Tasks: N, M thr, K kthr; R running"
                let segments = [
                    TextSegment {
                        text: "Tasks: ",
                        attr: text_attr,
                    },
                    TextSegment {
                        text: &total_tasks_str,
                        attr: value_attr,
                    },
                    TextSegment {
                        text: ", ",
                        attr: thr_text_attr,
                    },
                    TextSegment {
                        text: &userland_threads_str,
                        attr: thr_value_attr,
                    },
                    TextSegment {
                        text: " thr",
                        attr: thr_text_attr,
                    },
                    TextSegment {
                        text: ", ",
                        attr: kthr_text_attr,
                    },
                    TextSegment {
                        text: &kernel_threads_str,
                        attr: kthr_value_attr,
                    },
                    TextSegment {
                        text: " kthr",
                        attr: kthr_text_attr,
                    },
                    TextSegment {
                        text: "; ",
                        attr: text_attr,
                    },
                    TextSegment {
                        text: &running_str,
                        attr: running_attr,
                    },
                    TextSegment {
                        text: " running",
                        attr: text_attr,
                    },
                ];

                draw_text_segments(crt, x, y, &segments);
            }
            MeterMode::Bar => {
                // Get color attributes
                let cpu_system_attr = crt.color(ColorElement::CpuSystem);
                let process_thread_attr = crt.color(ColorElement::ProcessThread);
                let process_attr = crt.color(ColorElement::Process);

                let total = (self.total_all as f64).max(1.0);
                let text = format!("{}/{}", self.running, self.total_all);

                // Build bar segments matching C htop TasksMeter_attributes order:
                // CPU_SYSTEM (kernel threads), PROCESS_THREAD (userland threads), PROCESS (processes)
                let segments = vec![
                    BarSegment {
                        value: self.kernel_threads as f64,
                        attr: cpu_system_attr,
                    },
                    BarSegment {
                        value: self.userland_threads as f64,
                        attr: process_thread_attr,
                    },
                    BarSegment {
                        value: self.total_tasks as f64,
                        attr: process_attr,
                    },
                ];

                draw_bar_with_text(crt, x, y, width, "Tsk", &segments, total, &text);
            }
            MeterMode::Graph => {
                // Graph mode: show historical total tasks (self-scaling like C htop)
                // Sum all components: kernel threads + userland threads + processes + running
                // (matches C htop's Meter_computeSum for TasksMeter)
                let sum =
                    (self.kernel_threads + self.userland_threads + self.total_tasks + self.running)
                        as f64;

                // Record the raw value (not normalized) for self-scaling
                {
                    let mut graph_data = self.graph_data.borrow_mut();
                    graph_data.record_raw(sum, settings.delay * 100);
                }

                // Draw the self-scaling graph
                let graph_data = self.graph_data.borrow();
                draw_graph_self_scaling(crt, x, y, width, self.height(), &graph_data, "Tsk");
            }
            MeterMode::Led => {
                // LED mode: same format as Text mode (C htop uses display function for LED)
                // Format: "N, M thr, K kthr; R running"
                let text = format!(
                    "{}, {} thr, {} kthr; {} running",
                    self.total_tasks, self.userland_threads, self.kernel_threads, self.running
                );
                super::draw_led(crt, x, y, width, "Tasks: ", &text);
            }
            MeterMode::StackedGraph => {
                // StackedGraph mode: show stacked graph of kernel/userland/processes/running
                // Uses self-scaling based on max observed sum
                // Segments: kernel threads (bottom), userland threads, processes, running (top)
                let segments: [f64; TASKS_STACKED_GRAPH_SEGMENTS] = [
                    self.kernel_threads as f64,
                    self.userland_threads as f64,
                    self.total_tasks as f64,
                    self.running as f64,
                ];

                // Record the values in stacked graph data
                {
                    let mut stacked_graph_data = self.stacked_graph_data.borrow_mut();
                    stacked_graph_data.record(segments, settings.delay * 100);
                }

                // Get segment colors (matching C htop TasksMeter_attributes order)
                let segment_colors: [u32; TASKS_STACKED_GRAPH_SEGMENTS] = [
                    crt.color(ColorElement::CpuSystem),     // kernel threads
                    crt.color(ColorElement::ProcessThread), // userland threads
                    crt.color(ColorElement::Process),       // processes
                    crt.color(ColorElement::TasksRunning),  // running
                ];

                // Draw the stacked graph
                let stacked_graph_data = self.stacked_graph_data.borrow();
                draw_tasks_stacked_graph(
                    crt,
                    x,
                    y,
                    width,
                    self.height(),
                    &stacked_graph_data,
                    "Tsk",
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
        // Tasks meter supports all modes including StackedGraph
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

    // ==================== Constructor Tests ====================

    #[test]
    fn test_tasks_meter_new() {
        let meter = TasksMeter::new();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.total_tasks, 0);
        assert_eq!(meter.running, 0);
        assert_eq!(meter.userland_threads, 0);
        assert_eq!(meter.kernel_threads, 0);
        assert_eq!(meter.total_all, 0);
    }

    #[test]
    fn test_tasks_meter_default() {
        let meter = TasksMeter::default();
        assert_eq!(meter.mode, MeterMode::Text);
        assert_eq!(meter.total_tasks, 0);
    }

    // ==================== Update Tests ====================

    #[test]
    fn test_tasks_meter_update() {
        let mut meter = TasksMeter::new();
        let mut machine = Machine::default();

        machine.total_tasks = 300;
        machine.running_tasks = 5;
        machine.kernel_threads = 100;
        machine.userland_threads = 50;
        machine.active_cpus = 8;

        meter.update(&machine);

        // total_tasks = total - kernel - userland = 300 - 100 - 50 = 150
        assert_eq!(meter.total_tasks, 150);
        assert_eq!(meter.kernel_threads, 100);
        assert_eq!(meter.userland_threads, 50);
        // running is capped at active_cpus
        assert_eq!(meter.running, 5);
        assert_eq!(meter.total_all, 300);
    }

    #[test]
    fn test_tasks_meter_update_running_capped() {
        let mut meter = TasksMeter::new();
        let mut machine = Machine::default();

        machine.total_tasks = 100;
        machine.running_tasks = 20; // More than active CPUs
        machine.kernel_threads = 10;
        machine.userland_threads = 10;
        machine.active_cpus = 4;

        meter.update(&machine);

        // Running is capped at active_cpus
        assert_eq!(meter.running, 4);
    }

    #[test]
    fn test_tasks_meter_update_zero() {
        let mut meter = TasksMeter::new();
        let machine = Machine::default();

        meter.update(&machine);

        assert_eq!(meter.total_tasks, 0);
        assert_eq!(meter.running, 0);
        assert_eq!(meter.kernel_threads, 0);
        assert_eq!(meter.userland_threads, 0);
    }

    #[test]
    fn test_tasks_meter_update_saturating_sub() {
        // Test that total_tasks doesn't underflow
        let mut meter = TasksMeter::new();
        let mut machine = Machine::default();

        // total_tasks < kernel + userland would cause underflow without saturating_sub
        machine.total_tasks = 50;
        machine.kernel_threads = 100;
        machine.userland_threads = 100;
        machine.active_cpus = 4;

        meter.update(&machine);

        // Should be 0, not negative/wrapped
        assert_eq!(meter.total_tasks, 0);
    }

    // ==================== Meter Trait Tests ====================

    #[test]
    fn test_tasks_meter_name() {
        let meter = TasksMeter::new();
        assert_eq!(meter.name(), "Tasks");
    }

    #[test]
    fn test_tasks_meter_caption() {
        let meter = TasksMeter::new();
        assert_eq!(meter.caption(), "Tasks: ");
    }

    #[test]
    fn test_tasks_meter_mode() {
        let mut meter = TasksMeter::new();
        assert_eq!(meter.mode(), MeterMode::Text);

        meter.set_mode(MeterMode::Bar);
        assert_eq!(meter.mode(), MeterMode::Bar);

        meter.set_mode(MeterMode::Graph);
        assert_eq!(meter.mode(), MeterMode::Graph);

        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_tasks_meter_height() {
        let mut meter = TasksMeter::new();

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
    fn test_tasks_meter_default_mode() {
        let meter = TasksMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Bar);
    }
}
