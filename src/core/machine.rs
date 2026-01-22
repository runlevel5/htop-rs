//! Machine - System state representation
//!
//! This module contains the Machine struct which holds system-wide state
//! including CPU, memory, and process information.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::process::{Process, ProcessField, ProcessList};

/// Memory type alias (in bytes)
pub type Memory = u64;

/// CPU time data for a single CPU core
#[derive(Debug, Clone, Default)]
pub struct CpuData {
    pub user_time: u64,
    pub nice_time: u64,
    pub system_time: u64,
    pub idle_time: u64,
    pub iowait_time: u64,
    pub irq_time: u64,
    pub softirq_time: u64,
    pub steal_time: u64,
    pub guest_time: u64,
    pub guest_nice_time: u64,

    // Previous values for calculating deltas
    pub prev_user: u64,
    pub prev_nice: u64,
    pub prev_system: u64,
    pub prev_idle: u64,
    pub prev_iowait: u64,
    pub prev_irq: u64,
    pub prev_softirq: u64,
    pub prev_steal: u64,
    pub prev_guest: u64,
    pub prev_guest_nice: u64,

    // Calculated percentages
    pub user_percent: f64,
    pub nice_percent: f64,
    pub system_percent: f64,
    pub irq_percent: f64,
    pub softirq_percent: f64,
    pub steal_percent: f64,
    pub guest_percent: f64,
    pub iowait_percent: f64,
    pub total_percent: f64,

    pub online: bool,
    pub frequency: f64,           // in MHz
    pub temperature: Option<f32>, // in Celsius
}

impl CpuData {
    pub fn new() -> Self {
        CpuData {
            online: true,
            ..Default::default()
        }
    }

    /// Store current values as previous and update percentages
    pub fn update(&mut self) {
        let prev_total = self.prev_user
            + self.prev_nice
            + self.prev_system
            + self.prev_idle
            + self.prev_iowait
            + self.prev_irq
            + self.prev_softirq
            + self.prev_steal;

        let curr_total = self.user_time
            + self.nice_time
            + self.system_time
            + self.idle_time
            + self.iowait_time
            + self.irq_time
            + self.softirq_time
            + self.steal_time;

        let total_delta = (curr_total - prev_total) as f64;

        if total_delta > 0.0 {
            self.user_percent = ((self.user_time - self.prev_user) as f64 / total_delta) * 100.0;
            self.nice_percent = ((self.nice_time - self.prev_nice) as f64 / total_delta) * 100.0;
            self.system_percent =
                ((self.system_time - self.prev_system) as f64 / total_delta) * 100.0;
            self.irq_percent = ((self.irq_time - self.prev_irq) as f64 / total_delta) * 100.0;
            self.softirq_percent =
                ((self.softirq_time - self.prev_softirq) as f64 / total_delta) * 100.0;
            self.steal_percent = ((self.steal_time - self.prev_steal) as f64 / total_delta) * 100.0;
            self.guest_percent = ((self.guest_time - self.prev_guest) as f64 / total_delta) * 100.0;
            self.iowait_percent =
                ((self.iowait_time - self.prev_iowait) as f64 / total_delta) * 100.0;

            self.total_percent =
                100.0 - ((self.idle_time - self.prev_idle) as f64 / total_delta) * 100.0;
        }

        // Store current as previous
        self.prev_user = self.user_time;
        self.prev_nice = self.nice_time;
        self.prev_system = self.system_time;
        self.prev_idle = self.idle_time;
        self.prev_iowait = self.iowait_time;
        self.prev_irq = self.irq_time;
        self.prev_softirq = self.softirq_time;
        self.prev_steal = self.steal_time;
        self.prev_guest = self.guest_time;
        self.prev_guest_nice = self.guest_nice_time;
    }
}

/// Users table for caching UID -> username mappings
#[derive(Debug, Default)]
pub struct UsersTable {
    users: HashMap<u32, String>,
}

impl UsersTable {
    pub fn new() -> Self {
        UsersTable {
            users: HashMap::new(),
        }
    }

    /// Get username for a UID, caching the result
    pub fn get_username(&mut self, uid: u32) -> &str {
        self.users.entry(uid).or_insert_with(|| {
            // Try to look up the username
            #[cfg(unix)]
            {
                if let Some(user) = users::get_user_by_uid(uid) {
                    return user.name().to_string_lossy().to_string();
                }
            }
            // Fall back to UID as string
            uid.to_string()
        })
    }
}

/// Main system state container
#[derive(Debug)]
pub struct Machine {
    // Timing
    pub realtime: SystemTime,
    pub realtime_ms: u64,
    pub monotonic_ms: u64,
    pub prev_monotonic_ms: u64,
    pub last_scan: Instant,

    // Iteration control
    pub iterations_remaining: i64,

    // Memory statistics (in KB)
    pub total_mem: Memory,
    pub used_mem: Memory,
    pub buffers_mem: Memory,
    pub cached_mem: Memory,
    pub shared_mem: Memory,
    pub available_mem: Memory,
    pub compressed_mem: Memory, // Compressed memory (macOS)

    // Swap statistics (in KB)
    pub total_swap: Memory,
    pub used_swap: Memory,
    pub cached_swap: Memory,

    // CPU information
    pub active_cpus: u32,
    pub existing_cpus: u32,
    pub cpus: Vec<CpuData>,
    pub avg_cpu: CpuData, // Average/combined CPU stats

    // Users
    pub users_table: UsersTable,
    pub htop_user_id: u32,
    pub max_user_id: u32,
    pub filter_user_id: Option<u32>,

    // Processes
    pub processes: ProcessList,
    pub max_pid: i32,
    pub pid_filter: Option<HashSet<u32>>,

    // Sorting
    pub sort_key: ProcessField,
    pub sort_descending: bool,

    // Running/thread counts
    pub running_tasks: u32,
    pub total_tasks: u32,
    pub userland_threads: u32,
    pub kernel_threads: u32,

    // System information
    pub hostname: String,
    pub kernel_version: String,
    pub uptime: Duration,
    pub load_average: [f64; 3],

    // Boot time
    pub boot_time: i64,

    // Settings copied from Settings for platform access
    pub update_process_names: bool,
    pub show_cpu_frequency: bool,
}

impl Machine {
    pub fn new(filter_user_id: Option<u32>) -> Self {
        let htop_user_id = unsafe { libc::geteuid() };

        Machine {
            realtime: SystemTime::now(),
            realtime_ms: 0,
            monotonic_ms: 0,
            prev_monotonic_ms: 0,
            last_scan: Instant::now(),
            iterations_remaining: -1,
            total_mem: 0,
            used_mem: 0,
            buffers_mem: 0,
            cached_mem: 0,
            shared_mem: 0,
            available_mem: 0,
            compressed_mem: 0,
            total_swap: 0,
            used_swap: 0,
            cached_swap: 0,
            active_cpus: 1,
            existing_cpus: 1,
            cpus: Vec::new(),
            avg_cpu: CpuData::new(),
            users_table: UsersTable::new(),
            htop_user_id,
            max_user_id: 0,
            filter_user_id,
            processes: ProcessList::new(),
            max_pid: 32768,
            pid_filter: None,
            sort_key: ProcessField::PercentCpu,
            sort_descending: true,
            running_tasks: 0,
            total_tasks: 0,
            userland_threads: 0,
            kernel_threads: 0,
            hostname: String::new(),
            kernel_version: String::new(),
            uptime: Duration::ZERO,
            load_average: [0.0, 0.0, 0.0],
            boot_time: 0,
            update_process_names: false,
            show_cpu_frequency: false,
        }
    }

    /// Set PID filter for showing only specific PIDs
    pub fn set_pid_filter(&mut self, pids: Vec<u32>) {
        self.pid_filter = Some(pids.into_iter().collect());
    }

    /// Check if a process should be shown based on filters
    pub fn should_show_process(&self, process: &Process) -> bool {
        // Check PID filter
        if let Some(ref filter) = self.pid_filter {
            if !filter.contains(&(process.pid as u32)) {
                return false;
            }
        }

        // Check user filter
        if let Some(uid) = self.filter_user_id {
            if process.uid != uid {
                return false;
            }
        }

        true
    }

    /// Perform a full system scan
    pub fn scan(&mut self) {
        // Update timing
        self.prev_monotonic_ms = self.monotonic_ms;
        self.realtime = SystemTime::now();
        self.realtime_ms = self
            .realtime
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.monotonic_ms = self.last_scan.elapsed().as_millis() as u64;
        self.last_scan = Instant::now();

        // Platform-specific scanning is done in the platform module
        // This just updates timestamps
    }

    /// Update the process list after scanning
    pub fn update_processes(&mut self) {
        // Clean up processes that no longer exist
        self.processes.cleanup();

        // Note: Task counts (total_tasks, running_tasks, userland_threads, kernel_threads)
        // are set by the platform-specific scan_processes() function, not here.
        // This matches C htop where DarwinProcessTable_scan() accumulates these values
        // during process iteration rather than after.

        // Sort processes
        let ascending = !self.sort_descending;
        self.processes.sort_by(self.sort_key, ascending);
    }

    /// Get time delta in milliseconds since last scan
    pub fn time_delta_ms(&self) -> u64 {
        if self.prev_monotonic_ms > 0 {
            self.monotonic_ms.saturating_sub(self.prev_monotonic_ms)
        } else {
            1000 // Assume 1 second for first scan
        }
    }

    /// Check if a CPU is online
    pub fn is_cpu_online(&self, id: usize) -> bool {
        self.cpus.get(id).map(|c| c.online).unwrap_or(false)
    }

    /// Get memory usage percentage
    pub fn memory_percentage(&self) -> f64 {
        if self.total_mem > 0 {
            (self.used_mem as f64 / self.total_mem as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get swap usage percentage
    pub fn swap_percentage(&self) -> f64 {
        if self.total_swap > 0 {
            (self.used_swap as f64 / self.total_swap as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Format memory value with human-readable units
    pub fn format_memory(kb: Memory) -> String {
        if kb < 1024 {
            format!("{}K", kb)
        } else if kb < 1024 * 1024 {
            format!("{:.1}M", kb as f64 / 1024.0)
        } else if kb < 1024 * 1024 * 1024 {
            format!("{:.2}G", kb as f64 / 1024.0 / 1024.0)
        } else {
            format!("{:.2}T", kb as f64 / 1024.0 / 1024.0 / 1024.0)
        }
    }

    /// Get the username for a process
    pub fn get_username(&mut self, uid: u32) -> String {
        self.users_table.get_username(uid).to_string()
    }
}

impl Default for Machine {
    fn default() -> Self {
        Machine::new(None)
    }
}
