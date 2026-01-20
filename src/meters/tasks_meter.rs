//! Tasks Meter

use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;
use super::{Meter, MeterMode};

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
        self.total_tasks = machine.total_tasks
            .saturating_sub(machine.kernel_threads)
            .saturating_sub(machine.userland_threads);
        self.running = machine.running_tasks.min(machine.active_cpus);
    }

    fn draw(&self, crt: &Crt, _machine: &Machine, settings: &Settings, x: i32, y: i32, _width: i32) {
        use ncurses::*;

        let text_attr = crt.color(ColorElement::MeterText);
        let value_attr = crt.color(ColorElement::MeterValue);
        let running_attr = crt.color(ColorElement::TasksRunning);
        let shadow_attr = crt.color(ColorElement::MeterShadow);

        mv(y, x);

        // "Tasks: "
        attron(text_attr);
        let _ = addstr("Tasks: ");
        attroff(text_attr);

        // Process count (total tasks - threads)
        attron(value_attr);
        let _ = addstr(&format!("{}", self.total_tasks));
        attroff(value_attr);

        // ", N thr" - userland threads (shadowed if hideUserlandThreads)
        let (thr_text_attr, thr_value_attr) = if settings.hide_userland_threads {
            (shadow_attr, shadow_attr)
        } else {
            (text_attr, running_attr)
        };
        
        attron(thr_text_attr);
        let _ = addstr(", ");
        attroff(thr_text_attr);

        attron(thr_value_attr);
        let _ = addstr(&format!("{}", self.userland_threads));
        attroff(thr_value_attr);

        attron(thr_text_attr);
        let _ = addstr(" thr");
        attroff(thr_text_attr);

        // ", K kthr" - kernel threads (shadowed if hideKernelThreads)
        let (kthr_text_attr, kthr_value_attr) = if settings.hide_kernel_threads {
            (shadow_attr, shadow_attr)
        } else {
            (text_attr, running_attr)
        };
        
        attron(kthr_text_attr);
        let _ = addstr(", ");
        attroff(kthr_text_attr);

        attron(kthr_value_attr);
        let _ = addstr(&format!("{}", self.kernel_threads));
        attroff(kthr_value_attr);

        attron(kthr_text_attr);
        let _ = addstr(" kthr");
        attroff(kthr_text_attr);

        // "; R running" - always normal colors
        attron(text_attr);
        let _ = addstr("; ");
        attroff(text_attr);

        attron(running_attr);
        let _ = addstr(&format!("{}", self.running));
        attroff(running_attr);

        attron(text_attr);
        let _ = addstr(" running");
        attroff(text_attr);
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}
