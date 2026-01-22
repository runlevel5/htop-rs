//! Tasks Meter

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Tasks Meter
///
/// Displays task counts exactly like C htop:
/// "Tasks: N, M thr, K kthr; R running"
#[derive(Debug, Default)]
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
    }

    fn draw(
        &self,
        crt: &Crt,
        _machine: &Machine,
        settings: &Settings,
        x: i32,
        y: i32,
        _width: i32,
    ) {
        use ncurses::*;

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
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}
