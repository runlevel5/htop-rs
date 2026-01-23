//! Linux platform implementation
//!
//! This module provides Linux-specific implementations for reading
//! process, CPU, and memory information from /proc filesystem.

use anyhow::Result;
use procfs::{CpuTime, Current, CurrentSI, KernelStats, Meminfo};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::core::{CpuData, Machine, Process, ProcessState};

// IO Priority constants (from linux/ioprio.h)
pub const IOPRIO_CLASS_NONE: i32 = 0;
pub const IOPRIO_CLASS_RT: i32 = 1; // Real-time
pub const IOPRIO_CLASS_BE: i32 = 2; // Best-effort
pub const IOPRIO_CLASS_IDLE: i32 = 3; // Idle

const IOPRIO_CLASS_SHIFT: i32 = 13;
const IOPRIO_PRIO_MASK: i32 = (1 << IOPRIO_CLASS_SHIFT) - 1;

/// IOPRIO_WHO_PROCESS: get/set IO priority for a process
const IOPRIO_WHO_PROCESS: i32 = 1;

/// Extract the IO priority class from an ioprio value
#[inline]
pub fn ioprio_class(ioprio: i32) -> i32 {
    ioprio >> IOPRIO_CLASS_SHIFT
}

/// Extract the IO priority data (level within class) from an ioprio value
#[inline]
pub fn ioprio_data(ioprio: i32) -> i32 {
    ioprio & IOPRIO_PRIO_MASK
}

/// Get IO priority for a process using ioprio_get syscall
/// Returns the IO priority value, or -1 on error
pub fn get_io_priority(pid: i32) -> i32 {
    #[cfg(target_os = "linux")]
    {
        // SYS_ioprio_get is 252 on x86_64, 290 on i386
        // Use libc::SYS_ioprio_get if available, otherwise use the constant
        #[cfg(target_arch = "x86_64")]
        const SYS_IOPRIO_GET: libc::c_long = 252;
        #[cfg(target_arch = "x86")]
        const SYS_IOPRIO_GET: libc::c_long = 290;
        #[cfg(target_arch = "aarch64")]
        const SYS_IOPRIO_GET: libc::c_long = 31;
        #[cfg(target_arch = "arm")]
        const SYS_IOPRIO_GET: libc::c_long = 315;

        #[cfg(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        ))]
        {
            let result = unsafe {
                libc::syscall(
                    SYS_IOPRIO_GET,
                    IOPRIO_WHO_PROCESS as libc::c_int,
                    pid as libc::c_int,
                )
            };
            if result < 0 {
                -1
            } else {
                result as i32
            }
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        )))]
        {
            // Unsupported architecture
            -1
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Not available on non-Linux
        let _ = pid;
        -1
    }
}

/// Previous CPU times for calculating deltas
static PREV_CPU_TIMES: Mutex<Option<Vec<CpuTime>>> = Mutex::new(None);
static PREV_TOTAL_CPU: Mutex<Option<CpuTime>> = Mutex::new(None);

/// Initialize Linux platform
pub fn init() -> Result<()> {
    Ok(())
}

/// Cleanup Linux platform
pub fn done() {
    if let Ok(mut guard) = PREV_CPU_TIMES.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = PREV_TOTAL_CPU.lock() {
        *guard = None;
    }
}

/// Get system information
pub fn get_system_info(machine: &mut Machine) {
    // Get hostname
    if let Ok(hostname) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
        machine.hostname = hostname.trim().to_string();
    }

    // Get kernel version
    if let Ok(version) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        machine.kernel_version = format!("Linux {}", version.trim());
    }

    // Get uptime
    if let Ok(uptime) = procfs::Uptime::current() {
        machine.uptime = Duration::from_secs_f64(uptime.uptime);
    }

    // Get boot time
    if let Ok(stat) = procfs::KernelStats::current() {
        machine.boot_time = stat.btime as i64;
    }

    // Get load averages
    if let Ok(loadavg) = procfs::LoadAverage::current() {
        machine.load_average = [
            loadavg.one as f64,
            loadavg.five as f64,
            loadavg.fifteen as f64,
        ];
    }
}

/// Convert Linux process state character to our ProcessState
fn convert_process_state(state: char) -> ProcessState {
    match state {
        'R' => ProcessState::Running,
        'S' => ProcessState::Sleeping,
        'D' => ProcessState::UninterruptibleWait,
        'Z' => ProcessState::Zombie,
        'T' => ProcessState::Stopped,
        't' => ProcessState::Traced,
        'W' => ProcessState::Paging,
        'X' | 'x' => ProcessState::Defunct,
        'K' => ProcessState::Idle, // Wakekill
        'P' => ProcessState::Idle, // Parked
        'I' => ProcessState::Idle,
        _ => ProcessState::Unknown,
    }
}

/// Scan CPU statistics
pub fn scan_cpu(machine: &mut Machine) {
    let kernel_stats = match KernelStats::current() {
        Ok(stats) => stats,
        Err(_) => return,
    };

    // Get number of CPUs
    let cpu_count = kernel_stats.cpu_time.len();

    // Initialize CPU data if needed
    if machine.cpus.len() != cpu_count {
        machine.cpus.clear();
        for _ in 0..cpu_count {
            machine.cpus.push(CpuData::new());
        }
    }

    machine.active_cpus = cpu_count as u32;
    machine.existing_cpus = cpu_count as u32;

    // Update total/average CPU
    let total = &kernel_stats.total;
    {
        let prev_total = PREV_TOTAL_CPU.lock().ok();
        let prev_ref = prev_total.as_ref().and_then(|g| g.as_ref());
        update_cpu_data(&mut machine.avg_cpu, total, prev_ref);
    }
    if let Ok(mut guard) = PREV_TOTAL_CPU.lock() {
        *guard = Some(total.clone());
    }

    // Update per-CPU data
    let prev_times = PREV_CPU_TIMES.lock().ok().and_then(|mut g| g.take());
    let prev_map: HashMap<usize, &CpuTime> = prev_times
        .as_ref()
        .map(|v| v.iter().enumerate().collect())
        .unwrap_or_default();

    for (i, cpu_time) in kernel_stats.cpu_time.iter().enumerate() {
        if i < machine.cpus.len() {
            update_cpu_data(&mut machine.cpus[i], cpu_time, prev_map.get(&i).copied());
        }
    }

    if let Ok(mut guard) = PREV_CPU_TIMES.lock() {
        *guard = Some(kernel_stats.cpu_time.clone());
    }
}

/// Update CPU data from procfs CpuTime
fn update_cpu_data(cpu: &mut CpuData, current: &CpuTime, prev: Option<&CpuTime>) {
    // Store previous values
    cpu.prev_user = cpu.user_time;
    cpu.prev_nice = cpu.nice_time;
    cpu.prev_system = cpu.system_time;
    cpu.prev_idle = cpu.idle_time;
    cpu.prev_iowait = cpu.iowait_time;
    cpu.prev_irq = cpu.irq_time;
    cpu.prev_softirq = cpu.softirq_time;
    cpu.prev_steal = cpu.steal_time;
    cpu.prev_guest = cpu.guest_time;
    cpu.prev_guest_nice = cpu.guest_nice_time;

    // Update current values
    cpu.user_time = current.user;
    cpu.nice_time = current.nice;
    cpu.system_time = current.system;
    cpu.idle_time = current.idle;
    cpu.iowait_time = current.iowait.unwrap_or(0);
    cpu.irq_time = current.irq.unwrap_or(0);
    cpu.softirq_time = current.softirq.unwrap_or(0);
    cpu.steal_time = current.steal.unwrap_or(0);
    cpu.guest_time = current.guest.unwrap_or(0);
    cpu.guest_nice_time = current.guest_nice.unwrap_or(0);

    // Calculate percentages if we have previous values
    if let Some(prev) = prev {
        let prev_total = prev.user
            + prev.nice
            + prev.system
            + prev.idle
            + prev.iowait.unwrap_or(0)
            + prev.irq.unwrap_or(0)
            + prev.softirq.unwrap_or(0)
            + prev.steal.unwrap_or(0);

        let curr_total = current.user
            + current.nice
            + current.system
            + current.idle
            + current.iowait.unwrap_or(0)
            + current.irq.unwrap_or(0)
            + current.softirq.unwrap_or(0)
            + current.steal.unwrap_or(0);

        let total_delta = (curr_total - prev_total) as f64;

        if total_delta > 0.0 {
            cpu.user_percent = ((current.user - prev.user) as f64 / total_delta) * 100.0;
            cpu.nice_percent = ((current.nice - prev.nice) as f64 / total_delta) * 100.0;
            cpu.system_percent = ((current.system - prev.system) as f64 / total_delta) * 100.0;
            cpu.irq_percent =
                ((current.irq.unwrap_or(0) - prev.irq.unwrap_or(0)) as f64 / total_delta) * 100.0;
            cpu.softirq_percent =
                ((current.softirq.unwrap_or(0) - prev.softirq.unwrap_or(0)) as f64 / total_delta)
                    * 100.0;
            cpu.steal_percent = ((current.steal.unwrap_or(0) - prev.steal.unwrap_or(0)) as f64
                / total_delta)
                * 100.0;
            cpu.guest_percent = ((current.guest.unwrap_or(0) - prev.guest.unwrap_or(0)) as f64
                / total_delta)
                * 100.0;
            cpu.iowait_percent = ((current.iowait.unwrap_or(0) - prev.iowait.unwrap_or(0)) as f64
                / total_delta)
                * 100.0;

            let idle_delta = (current.idle - prev.idle) as f64;
            cpu.total_percent = 100.0 - (idle_delta / total_delta) * 100.0;
        }
    }

    cpu.online = true;
}

/// Scan memory statistics
pub fn scan_memory(machine: &mut Machine) {
    let meminfo = match Meminfo::current() {
        Ok(info) => info,
        Err(_) => return,
    };

    // All values from procfs are in bytes, convert to KB
    machine.total_mem = meminfo.mem_total / 1024;

    // Calculate used memory
    // Used = Total - Free - Buffers - Cached - SReclaimable
    let free = meminfo.mem_free / 1024;
    machine.buffers_mem = meminfo.buffers / 1024;
    machine.cached_mem = meminfo.cached / 1024;
    let sreclaimable = meminfo.s_reclaimable.unwrap_or(0) / 1024;

    machine.used_mem = machine
        .total_mem
        .saturating_sub(free)
        .saturating_sub(machine.buffers_mem)
        .saturating_sub(machine.cached_mem)
        .saturating_sub(sreclaimable);

    machine.available_mem = meminfo.mem_available.unwrap_or(0) / 1024;
    machine.shared_mem = meminfo.shmem.unwrap_or(0) / 1024;

    // Swap
    machine.total_swap = meminfo.swap_total / 1024;
    let swap_free = meminfo.swap_free / 1024;
    machine.used_swap = machine.total_swap.saturating_sub(swap_free);
    machine.cached_swap = meminfo.swap_cached / 1024;
}

/// Scan all processes
pub fn scan_processes(machine: &mut Machine) {
    // Reset auto-width fields at start of scan (matches C htop Row_resetFieldWidths)
    // This allows widths to shrink back when there are no longer processes with wide values
    machine.field_widths.reset_auto_widths();
    
    let all_procs = match procfs::process::all_processes() {
        Ok(procs) => procs,
        Err(_) => return,
    };

    // Get system values needed for calculations
    let ticks_per_second = procfs::ticks_per_second() as f64;
    let time_delta = machine.time_delta_ms() as f64 / 1000.0;
    let total_mem_kb = machine.total_mem as f64;
    let boot_time = machine.boot_time;

    // Track max values for dynamic column widths
    let mut max_pid: i32 = 0;
    let mut max_uid: u32 = 0;
    let mut max_percent_cpu: f32 = 0.0;

    for proc_result in all_procs {
        let proc = match proc_result {
            Ok(p) => p,
            Err(_) => continue,
        };

        let stat = match proc.stat() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let pid = stat.pid;

        // Check if process already exists
        let is_new = machine.processes.get(pid).is_none();
        let prev_time = if !is_new {
            machine.processes.get(pid).map(|p| p.time).unwrap_or(0)
        } else {
            0
        };

        let mut process = if is_new {
            Process::new(pid)
        } else {
            machine
                .processes
                .get(pid)
                .cloned()
                .unwrap_or_else(|| Process::new(pid))
        };

        // Basic info from stat
        process.pid = pid;
        process.ppid = stat.ppid;
        process.pgrp = stat.pgrp;
        process.session = stat.session;
        process.tty_nr = stat.tty_nr as u64;
        process.tpgid = stat.tpgid;
        process.priority = stat.priority;
        process.nice = stat.nice;
        process.nlwp = stat.num_threads;
        process.processor = stat.processor.unwrap_or(-1);
        process.minflt = stat.minflt;
        process.majflt = stat.majflt;

        // Process state
        process.state = convert_process_state(stat.state);

        // CPU time (convert from ticks to centiseconds)
        let utime = stat.utime;
        let stime = stat.stime;
        let total_ticks = utime + stime;
        process.time = ((total_ticks as f64 / ticks_per_second) * 100.0) as u64;
        
        // Store individual time fields (convert from ticks to centiseconds)
        process.utime = ((utime as f64 / ticks_per_second) * 100.0) as u64;
        process.stime = ((stime as f64 / ticks_per_second) * 100.0) as u64;
        process.cutime = ((stat.cutime as f64 / ticks_per_second) * 100.0) as u64;
        process.cstime = ((stat.cstime as f64 / ticks_per_second) * 100.0) as u64;
        
        // Children's page fault counters
        process.cminflt = stat.cminflt;
        process.cmajflt = stat.cmajflt;

        // Calculate CPU percentage
        if !is_new && time_delta > 0.0 && process.time > prev_time {
            let time_diff = (process.time - prev_time) as f64;
            // time_diff is in centiseconds, time_delta is in seconds
            process.percent_cpu = ((time_diff / 100.0) / time_delta * 100.0) as f32;
            process.percent_cpu = process.percent_cpu.min(100.0 * machine.active_cpus as f32);
        }

        // Start time
        process.starttime_ctime = boot_time + (stat.starttime as i64 / ticks_per_second as i64);

        // Memory from stat (pages to KB)
        let page_size = (procfs::page_size() / 1024) as i64;
        process.m_virt = (stat.vsize / 1024) as i64;
        process.m_resident = (stat.rss as i64) * page_size;

        // Calculate memory percentage
        if total_mem_kb > 0.0 {
            process.percent_mem = ((process.m_resident as f64 / total_mem_kb) * 100.0) as f32;
        }

        // Only update process name/command if it's new or update_process_names is enabled
        // (matches C htop Settings.updateProcessNames behavior)
        // Don't re-read for zombie processes since their cmdline is empty
        let should_update_names =
            is_new || (machine.update_process_names && process.state != ProcessState::Zombie);

        if should_update_names {
            // Get command name from stat (fast, always available)
            process.comm = Some(stat.comm.clone());

            // Try to get full cmdline
            if let Ok(cmdline) = proc.cmdline() {
                if !cmdline.is_empty() {
                    let joined = cmdline.join(" ");
                    // basename_end is the length of the first argument
                    let basename_end = cmdline.first().map(|s| s.len()).unwrap_or(0);
                    process.update_cmdline(joined, basename_end);
                }
            }

            // Try to get exe path and check if deleted
            // Linux marks deleted executables with " (deleted)" suffix in /proc/PID/exe
            if let Ok(exe) = proc.exe() {
                let exe_str = exe.to_string_lossy();
                let (clean_path, deleted) = if exe_str.ends_with(" (deleted)") {
                    // Remove the " (deleted)" suffix from the path
                    (exe_str.trim_end_matches(" (deleted)").to_string(), true)
                } else {
                    (exe_str.to_string(), false)
                };

                // Compute exe_basename_offset (position of basename in exe path)
                process.exe_basename_offset = clean_path.rfind('/').map(|p| p + 1).unwrap_or(0);
                process.exe = Some(clean_path);
                process.exe_deleted = deleted;
            }
        }

        // Try to get cwd (not related to update_process_names, read on each scan)
        // TODO: Could be optimized to only read when CWD column is displayed
        if let Ok(cwd) = proc.cwd() {
            process.cwd = Some(cwd.to_string_lossy().to_string());
        }

        // Get UID from status
        if let Ok(status) = proc.status() {
            process.uid = status.ruid;
            process.user = Some(machine.get_username(process.uid));
        }

        // Memory details from statm
        if let Ok(statm) = proc.statm() {
            process.m_share = (statm.shared * page_size as u64) as i64;
            process.m_text = (statm.text * page_size as u64) as i64;
            process.m_data = (statm.data * page_size as u64) as i64;
        }

        // Check for kernel thread (ppid == 2 is kthreadd)
        process.is_kernel_thread = process.ppid == 2 || pid == 2;

        // Check for userland thread (has TGID different from PID)
        if let Ok(status) = proc.status() {
            if status.tgid != pid {
                process.is_userland_thread = true;
            }
        }

        // IO statistics (requires root or same user)
        // Read full /proc/[pid]/io data (more complete than procfs crate provides)
        read_io_file(&mut process, pid, machine.realtime_ms, is_new);

        // Check for deleted libraries by scanning /proc/PID/maps
        // Only check if exe is not deleted (if exe is deleted, no need to check libs)
        // and only for non-kernel, non-userland threads
        // C htop checks this periodically (~2 seconds) but we check on each scan for simplicity
        if !process.exe_deleted && !process.is_kernel_thread && !process.is_userland_thread {
            process.uses_deleted_lib = check_deleted_libs(pid);
        }

        // OOM score
        if let Ok(oom) = std::fs::read_to_string(format!("/proc/{}/oom_score", pid)) {
            if let Ok(score) = oom.trim().parse::<i32>() {
                process.oom_score = score;
            }
        }

        // IO Priority (thread-specific data, read on every scan)
        process.io_priority = get_io_priority(pid);

        // CGroup
        if let Ok(cgroups) = proc.cgroups() {
            if let Some(cgroup) = cgroups.0.first() {
                let cgroup_path = cgroup.pathname.clone();
                // Generate compressed cgroup name and detect container
                process.cgroup_short = Some(filter_cgroup_name(&cgroup_path));
                process.container_short = filter_container(&cgroup_path);
                process.cgroup = Some(cgroup_path);
            }
        }

        // Smaps data (PSS, Swap, SwapPss) - expensive, only read for non-kernel processes
        // TODO: Could be optimized to only read when these columns are displayed
        if !process.is_kernel_thread {
            read_smaps_file(&mut process, pid);
        }

        // Autogroup data
        read_autogroup(&mut process, pid);

        // Library size from maps (expensive)
        // TODO: Could be optimized to only read when M_LIB column is displayed
        if !process.is_kernel_thread && !process.is_userland_thread {
            read_maps_for_lib_size(&mut process, pid, page_size);
        }

        // Track max values for dynamic column widths
        if pid > max_pid {
            max_pid = pid;
        }
        if process.uid > max_uid {
            max_uid = process.uid;
        }
        if process.percent_cpu > max_percent_cpu {
            max_percent_cpu = process.percent_cpu;
        }

        process.updated = true;
        machine.processes.add(process);
    }

    // Update dynamic field widths based on scan results
    machine.max_pid = max_pid;
    machine.max_user_id = max_uid;
    machine.field_widths.set_pid_width(max_pid);
    machine.field_widths.set_uid_width(max_uid);
    machine.field_widths.update_percent_cpu_width(max_percent_cpu);
}

/// Check if a process uses deleted libraries by scanning /proc/PID/maps
/// Returns true if any executable memory-mapped file has " (deleted)" suffix
fn check_deleted_libs(pid: i32) -> bool {
    use std::io::{BufRead, BufReader};

    let maps_path = format!("/proc/{}/maps", pid);
    let file = match std::fs::File::open(&maps_path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Maps format: address perms offset dev inode pathname
        // Example: 7f1234-7f5678 r-xp 00000000 08:01 12345 /lib/libc.so.6 (deleted)
        let parts: Vec<&str> = line.splitn(6, char::is_whitespace).collect();
        if parts.len() < 6 {
            continue;
        }

        // Check permissions (2nd field) for execute bit
        let perms = parts[1];
        if perms.len() < 3 || perms.chars().nth(2) != Some('x') {
            continue;
        }

        // Get the pathname (6th field, may contain spaces)
        let pathname = parts[5].trim();

        // Skip non-path entries
        if !pathname.starts_with('/') {
            continue;
        }

        // Skip false positives (matches C htop behavior)
        if pathname.starts_with("/memfd:") {
            continue;
        }
        if pathname == "/dev/zero (deleted)" {
            continue;
        }

        // Check for " (deleted)" suffix
        if pathname.ends_with(" (deleted)") {
            return true;
        }
    }

    false
}

/// Scan CPU frequency from sysfs (preferred method)
/// Returns the number of CPUs with frequency info found, or -1 on error/timeout
fn scan_cpu_frequency_from_sysfs(machine: &mut Machine) -> i32 {
    use std::time::Instant;

    // Timeout mechanism: if reading CPU 0 takes too long (>500us), bail out
    // and use fallback. This matches C htop behavior for slow AMD/Intel CPUs.
    static TIMEOUT_COUNTER: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

    let timeout = TIMEOUT_COUNTER.load(std::sync::atomic::Ordering::Relaxed);
    if timeout > 0 {
        TIMEOUT_COUNTER.store(timeout - 1, std::sync::atomic::Ordering::Relaxed);
        return -1;
    }

    let mut num_cpus_with_frequency = 0;
    let mut total_frequency: f64 = 0.0;

    for i in 0..machine.existing_cpus as usize {
        if i >= machine.cpus.len() || !machine.cpus[i].online {
            continue;
        }

        let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq", i);

        let start = if i == 0 { Some(Instant::now()) } else { None };

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                if let Ok(freq_khz) = content.trim().parse::<u64>() {
                    // Convert kHz to MHz
                    let freq_mhz = freq_khz as f64 / 1000.0;
                    machine.cpus[i].frequency = freq_mhz;
                    num_cpus_with_frequency += 1;
                    total_frequency += freq_mhz;
                }
            }
            Err(_) => {
                // File doesn't exist or can't be read
                return -1;
            }
        }

        // Check timing on first CPU
        if let Some(start_time) = start {
            let elapsed_us = start_time.elapsed().as_micros();
            if elapsed_us > 500 {
                // Too slow, set timeout for next 30 scans
                TIMEOUT_COUNTER.store(30, std::sync::atomic::Ordering::Relaxed);
                return -1;
            }
        }
    }

    // Set average frequency
    if num_cpus_with_frequency > 0 {
        machine.avg_cpu.frequency = total_frequency / num_cpus_with_frequency as f64;
    }

    num_cpus_with_frequency
}

/// Scan CPU frequency from /proc/cpuinfo (fallback method)
fn scan_cpu_frequency_from_cpuinfo(machine: &mut Machine) {
    let content = match std::fs::read_to_string("/proc/cpuinfo") {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut num_cpus_with_frequency = 0;
    let mut total_frequency: f64 = 0.0;
    let mut current_cpu: Option<usize> = None;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() {
            current_cpu = None;
            continue;
        }

        // Parse processor number
        if line.starts_with("processor") {
            if let Some(value) = line.split(':').nth(1) {
                if let Ok(cpu_id) = value.trim().parse::<usize>() {
                    current_cpu = Some(cpu_id);
                }
            }
            continue;
        }

        // Parse cpu MHz (various formats for different architectures)
        let frequency: Option<f64> = if line.starts_with("cpu MHz") {
            // Standard x86: "cpu MHz : 3400.000"
            line.split(':').nth(1).and_then(|v| v.trim().parse().ok())
        } else if line.starts_with("CPU MHz") {
            // LoongArch: "CPU MHz : 2500.000"
            line.split(':').nth(1).and_then(|v| v.trim().parse().ok())
        } else if line.starts_with("cpu MHz dynamic") {
            // s390: "cpu MHz dynamic : 5200"
            line.split(':').nth(1).and_then(|v| v.trim().parse().ok())
        } else if line.starts_with("clock") && line.contains("MHz") {
            // PowerPC: "clock : 3000.000MHz"
            line.split(':')
                .nth(1)
                .and_then(|v| v.trim().trim_end_matches("MHz").parse().ok())
        } else {
            None
        };

        if let (Some(freq), Some(cpu_id)) = (frequency, current_cpu) {
            if cpu_id < machine.cpus.len() {
                // Only set if not already set by sysfs (sysfs takes precedence)
                if machine.cpus[cpu_id].frequency <= 0.0 {
                    machine.cpus[cpu_id].frequency = freq;
                }
                num_cpus_with_frequency += 1;
                total_frequency += freq;
            }
        }
    }

    // Set average frequency if not already set
    if num_cpus_with_frequency > 0 && machine.avg_cpu.frequency <= 0.0 {
        machine.avg_cpu.frequency = total_frequency / num_cpus_with_frequency as f64;
    }
}

/// Scan CPU frequency (main entry point)
/// This matches C htop's LinuxMachine_scanCPUFrequency behavior:
/// 1. Reset all frequencies to NaN (we use 0.0)
/// 2. Try sysfs first (faster, more accurate)
/// 3. Fall back to /proc/cpuinfo if sysfs fails
pub fn scan_cpu_frequency(machine: &mut Machine) {
    // Reset all frequencies
    for cpu in &mut machine.cpus {
        cpu.frequency = 0.0;
    }
    machine.avg_cpu.frequency = 0.0;

    // Try sysfs first
    if scan_cpu_frequency_from_sysfs(machine) >= 0 {
        return;
    }

    // Fall back to /proc/cpuinfo
    scan_cpu_frequency_from_cpuinfo(machine);
}

/// Minimum time between disk IO cache updates (in milliseconds)
const DISK_IO_CACHE_DELAY_MS: u64 = 500;

/// Scan disk IO statistics from /proc/diskstats
/// This matches C htop's Platform_getDiskIO behavior
pub fn scan_disk_io(machine: &mut Machine) {
    use std::io::{BufRead, BufReader};

    // Rate limiting: only update every DISK_IO_CACHE_DELAY_MS
    let now_ms = machine.realtime_ms;
    if machine.disk_io_last_update > 0
        && now_ms < machine.disk_io_last_update + DISK_IO_CACHE_DELAY_MS
    {
        return;
    }

    let file = match std::fs::File::open("/proc/diskstats") {
        Ok(f) => f,
        Err(_) => return,
    };

    let reader = BufReader::new(file);

    let mut last_top_disk = String::new();
    let mut read_sum: u64 = 0;
    let mut write_sum: u64 = 0;
    let mut time_spend_sum: u64 = 0;
    let mut num_disks: u64 = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // /proc/diskstats format:
        // major minor name reads_completed reads_merged sectors_read ms_reading
        // writes_completed writes_merged sectors_written ms_writing ios_in_progress ms_io total_ms_weighted
        // We need: name (field 2), sectors_read (field 5), sectors_written (field 9), ms_io (field 12)
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 14 {
            continue;
        }

        let diskname = fields[2];

        // Skip dm-*, loop*, md*, zram* devices (like C htop)
        if diskname.starts_with("dm-")
            || diskname.starts_with("loop")
            || diskname.starts_with("md")
            || diskname.starts_with("zram")
        {
            continue;
        }

        // Only count root disks - don't count IO from sda and sda1 twice
        // This assumes disks are listed directly before any of their partitions
        if !last_top_disk.is_empty() && diskname.starts_with(&last_top_disk) {
            continue;
        }

        // Parse the values
        let sectors_read: u64 = match fields[5].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let sectors_written: u64 = match fields[9].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ms_io: u64 = match fields[12].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Update last top disk
        last_top_disk = diskname.to_string();

        read_sum += sectors_read;
        write_sum += sectors_written;
        time_spend_sum += ms_io;
        num_disks += 1;
    }

    // Multiply sectors by 512 to get bytes (standard sector size)
    let total_bytes_read = read_sum * 512;
    let total_bytes_written = write_sum * 512;

    // Calculate rates if we have previous data
    if machine.disk_io_last_update > 0 {
        let elapsed_ms = now_ms.saturating_sub(machine.disk_io_last_update);
        if elapsed_ms > 0 {
            // Calculate read/write rates in bytes per second
            let read_delta = total_bytes_read.saturating_sub(machine.disk_io_read_bytes);
            let write_delta = total_bytes_written.saturating_sub(machine.disk_io_write_bytes);
            let time_delta = time_spend_sum.saturating_sub(machine.disk_io_ms_time_spend);

            // Rate = bytes_delta * 1000 / elapsed_ms (to get bytes/second)
            machine.disk_io_read_rate = (read_delta as f64) * 1000.0 / (elapsed_ms as f64);
            machine.disk_io_write_rate = (write_delta as f64) * 1000.0 / (elapsed_ms as f64);

            // Utilization = 100 * ms_io_delta / elapsed_ms
            // But we normalize by number of disks since utilization can exceed 100%
            // when multiple disks are busy
            machine.disk_io_utilization = (time_delta as f64) * 100.0 / (elapsed_ms as f64);
        }
    }

    // Store current values for next iteration
    machine.disk_io_read_bytes = total_bytes_read;
    machine.disk_io_write_bytes = total_bytes_written;
    machine.disk_io_ms_time_spend = time_spend_sum;
    machine.disk_io_num_disks = num_disks;
    machine.disk_io_last_update = now_ms;
}

/// Minimum time between network IO cache updates (in milliseconds)
const NETWORK_IO_CACHE_DELAY_MS: u64 = 500;

/// Scan network IO statistics from /proc/net/dev
/// This matches C htop's Platform_getNetworkIO behavior
pub fn scan_network_io(machine: &mut Machine) {
    use std::io::{BufRead, BufReader};

    // Rate limiting: only update every NETWORK_IO_CACHE_DELAY_MS
    let now_ms = machine.realtime_ms;
    if machine.net_io_last_update > 0
        && now_ms < machine.net_io_last_update + NETWORK_IO_CACHE_DELAY_MS
    {
        return;
    }

    let file = match std::fs::File::open("/proc/net/dev") {
        Ok(f) => f,
        Err(_) => return,
    };

    let reader = BufReader::new(file);

    let mut bytes_received: u64 = 0;
    let mut packets_received: u64 = 0;
    let mut bytes_transmitted: u64 = 0;
    let mut packets_transmitted: u64 = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // /proc/net/dev format:
        // Inter-|   Receive                                                |  Transmit
        //  face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
        //    lo: 1234567   12345    0    0    0     0          0         0  1234567   12345    0    0    0     0       0          0
        // We need: interface, rx_bytes, rx_packets, tx_bytes, tx_packets

        // Skip header lines (lines without ':')
        let line = line.trim();
        if !line.contains(':') {
            continue;
        }

        // Split on ':' to get interface name
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }

        let interface = parts[0].trim();

        // Skip loopback interface (like C htop)
        if interface == "lo" {
            continue;
        }

        // Parse the statistics
        let stats: Vec<&str> = parts[1].split_whitespace().collect();
        if stats.len() < 10 {
            continue;
        }

        // Fields: rx_bytes rx_packets rx_errs rx_drop rx_fifo rx_frame rx_compressed rx_multicast
        //         tx_bytes tx_packets tx_errs tx_drop tx_fifo tx_colls tx_carrier tx_compressed
        let rx_bytes: u64 = stats[0].parse().unwrap_or(0);
        let rx_packets: u64 = stats[1].parse().unwrap_or(0);
        let tx_bytes: u64 = stats[8].parse().unwrap_or(0);
        let tx_packets: u64 = stats[9].parse().unwrap_or(0);

        bytes_received += rx_bytes;
        packets_received += rx_packets;
        bytes_transmitted += tx_bytes;
        packets_transmitted += tx_packets;
    }

    // Calculate rates if we have previous data
    if machine.net_io_last_update > 0 {
        let elapsed_ms = now_ms.saturating_sub(machine.net_io_last_update);
        if elapsed_ms > 0 {
            // Calculate byte rates in bytes per second
            let rx_bytes_delta = bytes_received.saturating_sub(machine.net_io_bytes_received);
            let tx_bytes_delta = bytes_transmitted.saturating_sub(machine.net_io_bytes_transmitted);

            // Calculate packet rates in packets per second
            let rx_packets_delta = packets_received.saturating_sub(machine.net_io_packets_received);
            let tx_packets_delta =
                packets_transmitted.saturating_sub(machine.net_io_packets_transmitted);

            // Rate = delta * 1000 / elapsed_ms (to get per-second rate)
            machine.net_io_receive_rate = (rx_bytes_delta as f64) * 1000.0 / (elapsed_ms as f64);
            machine.net_io_transmit_rate = (tx_bytes_delta as f64) * 1000.0 / (elapsed_ms as f64);
            machine.net_io_receive_packets =
                ((rx_packets_delta as f64) * 1000.0 / (elapsed_ms as f64)) as u64;
            machine.net_io_transmit_packets =
                ((tx_packets_delta as f64) * 1000.0 / (elapsed_ms as f64)) as u64;
        }
    }

    // Store current values for next iteration
    machine.net_io_bytes_received = bytes_received;
    machine.net_io_bytes_transmitted = bytes_transmitted;
    machine.net_io_packets_received = packets_received;
    machine.net_io_packets_transmitted = packets_transmitted;
    machine.net_io_last_update = now_ms;
}

/// Read /proc/[pid]/io for detailed IO statistics
/// This matches C htop's LinuxProcessTable_readIoFile behavior
fn read_io_file(process: &mut Process, pid: i32, realtime_ms: u64, is_new: bool) {
    use std::io::{BufRead, BufReader};

    let io_path = format!("/proc/{}/io", pid);
    let file = match std::fs::File::open(&io_path) {
        Ok(f) => f,
        Err(_) => {
            // Can't read IO file - mark all values as unavailable
            process.io_rate_read_bps = f64::NAN;
            process.io_rate_write_bps = f64::NAN;
            process.io_rchar = u64::MAX;
            process.io_wchar = u64::MAX;
            process.io_syscr = u64::MAX;
            process.io_syscw = u64::MAX;
            process.io_read_bytes = u64::MAX;
            process.io_write_bytes = u64::MAX;
            process.io_cancelled_write_bytes = u64::MAX;
            process.io_last_scan_time_ms = realtime_ms;
            return;
        }
    };

    // Store previous values for rate calculation
    let last_read = process.io_read_bytes;
    let last_write = process.io_write_bytes;
    let time_delta = if process.io_last_scan_time_ms > 0 {
        realtime_ms.saturating_sub(process.io_last_scan_time_ms)
    } else {
        0
    };

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Parse format: "key: value"
        // Keys: rchar, wchar, syscr, syscw, read_bytes, write_bytes, cancelled_write_bytes
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            match key {
                "rchar" => {
                    if let Ok(v) = value.parse::<u64>() {
                        process.io_rchar = v;
                    }
                }
                "wchar" => {
                    if let Ok(v) = value.parse::<u64>() {
                        process.io_wchar = v;
                    }
                }
                "syscr" => {
                    if let Ok(v) = value.parse::<u64>() {
                        process.io_syscr = v;
                    }
                }
                "syscw" => {
                    if let Ok(v) = value.parse::<u64>() {
                        process.io_syscw = v;
                    }
                }
                "read_bytes" => {
                    if let Ok(v) = value.parse::<u64>() {
                        process.io_read_bytes = v;
                        // Calculate read rate
                        if !is_new && time_delta > 0 && v >= last_read {
                            process.io_rate_read_bps =
                                (v - last_read) as f64 * 1000.0 / time_delta as f64;
                            process.io_read_rate = process.io_rate_read_bps; // Legacy alias
                        } else if is_new {
                            process.io_rate_read_bps = f64::NAN;
                        }
                    }
                }
                "write_bytes" => {
                    if let Ok(v) = value.parse::<u64>() {
                        process.io_write_bytes = v;
                        // Calculate write rate
                        if !is_new && time_delta > 0 && v >= last_write {
                            process.io_rate_write_bps =
                                (v - last_write) as f64 * 1000.0 / time_delta as f64;
                            process.io_write_rate = process.io_rate_write_bps; // Legacy alias
                        } else if is_new {
                            process.io_rate_write_bps = f64::NAN;
                        }
                    }
                }
                "cancelled_write_bytes" => {
                    if let Ok(v) = value.parse::<u64>() {
                        process.io_cancelled_write_bytes = v;
                    }
                }
                _ => {}
            }
        }
    }

    process.io_last_scan_time_ms = realtime_ms;
}

/// Read /proc/[pid]/smaps_rollup or /proc/[pid]/smaps for PSS, Swap, SwapPss
/// This matches C htop's LinuxProcessTable_readSmapsFile behavior
fn read_smaps_file(process: &mut Process, pid: i32) {
    use std::io::{BufRead, BufReader};

    // Try smaps_rollup first (faster, available since Linux 4.14)
    let smaps_path = format!("/proc/{}/smaps_rollup", pid);
    let file = match std::fs::File::open(&smaps_path) {
        Ok(f) => f,
        Err(_) => {
            // Fall back to full smaps
            let smaps_path = format!("/proc/{}/smaps", pid);
            match std::fs::File::open(&smaps_path) {
                Ok(f) => f,
                Err(_) => return,
            }
        }
    };

    process.m_pss = 0;
    process.m_swap = 0;
    process.m_psswp = 0;

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Parse lines like "Pss:           1234 kB"
        if line.starts_with("Pss:") {
            if let Some(value) = parse_smaps_value(&line, 4) {
                process.m_pss += value;
            }
        } else if line.starts_with("Swap:") {
            if let Some(value) = parse_smaps_value(&line, 5) {
                process.m_swap += value;
            }
        } else if line.starts_with("SwapPss:") {
            if let Some(value) = parse_smaps_value(&line, 8) {
                process.m_psswp += value;
            }
        }
    }
}

/// Parse a value from smaps line like "Pss:           1234 kB"
fn parse_smaps_value(line: &str, skip: usize) -> Option<i64> {
    line.get(skip..)?
        .trim()
        .split_whitespace()
        .next()?
        .parse::<i64>()
        .ok()
}

/// Read /proc/[pid]/autogroup for autogroup ID and nice value
/// This matches C htop's LinuxProcessTable_readAutogroup behavior
fn read_autogroup(process: &mut Process, pid: i32) {
    let autogroup_path = format!("/proc/{}/autogroup", pid);
    let content = match std::fs::read_to_string(&autogroup_path) {
        Ok(c) => c,
        Err(_) => {
            process.autogroup_id = -1;
            return;
        }
    };

    // Format: "/autogroup-123 nice 0"
    process.autogroup_id = -1;
    process.autogroup_nice = 0;

    // Parse using sscanf-like approach
    if content.starts_with("/autogroup-") {
        let rest = &content[11..]; // Skip "/autogroup-"
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() >= 3 && parts[1] == "nice" {
            if let Ok(id) = parts[0].parse::<i64>() {
                process.autogroup_id = id;
            }
            if let Ok(nice) = parts[2].parse::<i32>() {
                process.autogroup_nice = nice;
            }
        }
    }
}

/// Read /proc/[pid]/maps to calculate library size (m_lib)
/// This matches C htop's LinuxProcessTable_readMaps behavior for calcSize
fn read_maps_for_lib_size(process: &mut Process, pid: i32, page_size_kb: i64) {
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader};

    let maps_path = format!("/proc/{}/maps", pid);
    let file = match std::fs::File::open(&maps_path) {
        Ok(f) => f,
        Err(_) => return,
    };

    // Track library sizes by inode to avoid counting duplicates
    // Key: inode, Value: (size, is_executable)
    let mut libs: HashMap<u64, (u64, bool)> = HashMap::new();

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Skip lines without a path (no '/')
        if !line.contains('/') {
            continue;
        }

        // Maps format: address perms offset dev inode pathname
        // Example: 7f1234-7f5678 r-xp 00000000 08:01 12345 /lib/libc.so.6
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        // Parse address range
        let addr_parts: Vec<&str> = parts[0].split('-').collect();
        if addr_parts.len() != 2 {
            continue;
        }
        let map_start = match u64::from_str_radix(addr_parts[0], 16) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let map_end = match u64::from_str_radix(addr_parts[1], 16) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Check if executable (3rd char of perms)
        let perms = parts[1];
        let is_exec = perms.len() >= 3 && perms.chars().nth(2) == Some('x');

        // Parse device (skip if 00:00 - no file backing)
        let dev_parts: Vec<&str> = parts[3].split(':').collect();
        if dev_parts.len() == 2 {
            let devmaj = u32::from_str_radix(dev_parts[0], 16).unwrap_or(0);
            let devmin = u32::from_str_radix(dev_parts[1], 16).unwrap_or(0);
            if devmaj == 0 && devmin == 0 {
                continue;
            }
        }

        // Parse inode
        let inode = match parts[4].parse::<u64>() {
            Ok(v) if v > 0 => v,
            _ => continue,
        };

        // Update library tracking
        let entry = libs.entry(inode).or_insert((0, false));
        entry.0 += map_end - map_start;
        entry.1 |= is_exec;
    }

    // Sum up executable library sizes
    let total_size: u64 = libs
        .values()
        .filter(|(_, is_exec)| *is_exec)
        .map(|(size, _)| size)
        .sum();

    // Convert to KB (divide by page_size to get pages, then multiply by page_size_kb)
    // Actually C htop divides by pageSize (in bytes), so we convert bytes to KB
    process.m_lib = (total_size / 1024) as i64;
}

/// Filter and compress cgroup path for display
/// This matches C htop's CGroup_filterName behavior
pub fn filter_cgroup_name(cgroup: &str) -> String {
    // Remove common prefixes and compress the path
    let mut result = cgroup.to_string();

    // Remove leading /
    if result.starts_with('/') {
        result = result[1..].to_string();
    }

    // Common patterns to compress (from C htop CGroupUtils.c)
    // Remove "system.slice/" prefix
    if result.starts_with("system.slice/") {
        result = result.replacen("system.slice/", "", 1);
    }
    // Remove "user.slice/" prefix
    if result.starts_with("user.slice/") {
        result = result.replacen("user.slice/", "", 1);
    }

    // Truncate .service suffix
    if let Some(pos) = result.find(".service") {
        result.truncate(pos);
    }

    // Truncate .scope suffix
    if let Some(pos) = result.find(".scope") {
        result.truncate(pos);
    }

    result
}

/// Detect container name from cgroup path
/// This matches C htop's CGroup_filterContainer behavior
pub fn filter_container(cgroup: &str) -> Option<String> {
    // Docker container detection
    // Pattern: /docker/<container_id> or /system.slice/docker-<id>.scope
    if cgroup.contains("/docker/") {
        if let Some(id_start) = cgroup.rfind("/docker/") {
            let id = &cgroup[id_start + 8..];
            // Truncate to first 12 chars like docker does
            let short_id = if id.len() > 12 { &id[..12] } else { id };
            return Some(format!("docker:{}", short_id));
        }
    }

    // Docker with systemd cgroup driver
    // Pattern: docker-<id>.scope
    if cgroup.contains("docker-") {
        if let Some(start) = cgroup.find("docker-") {
            let rest = &cgroup[start + 7..];
            if let Some(end) = rest.find('.') {
                let id = &rest[..end];
                let short_id = if id.len() > 12 { &id[..12] } else { id };
                return Some(format!("docker:{}", short_id));
            }
        }
    }

    // Podman container detection
    if cgroup.contains("/libpod-") {
        if let Some(start) = cgroup.find("/libpod-") {
            let id = &cgroup[start + 8..];
            let short_id = if id.len() > 12 { &id[..12] } else { id };
            return Some(format!("podman:{}", short_id));
        }
    }

    // LXC container detection
    // Pattern: /lxc/<name> or /lxc.payload.<name>
    if cgroup.contains("/lxc/") {
        if let Some(start) = cgroup.rfind("/lxc/") {
            let name = &cgroup[start + 5..];
            let name = name.split('/').next().unwrap_or(name);
            return Some(format!("lxc:{}", name));
        }
    }

    if cgroup.contains("/lxc.payload.") {
        if let Some(start) = cgroup.find("/lxc.payload.") {
            let name = &cgroup[start + 13..];
            let name = name.split('/').next().unwrap_or(name);
            return Some(format!("lxc:{}", name));
        }
    }

    // systemd-nspawn container detection
    // Pattern: /machine.slice/machine-<name>.scope
    if cgroup.contains("/machine.slice/machine-") {
        if let Some(start) = cgroup.find("/machine.slice/machine-") {
            let name = &cgroup[start + 23..];
            if let Some(end) = name.find('.') {
                return Some(format!("nspawn:{}", &name[..end]));
            }
        }
    }

    None
}
