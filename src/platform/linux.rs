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
    let all_procs = match procfs::process::all_processes() {
        Ok(procs) => procs,
        Err(_) => return,
    };

    // Get system values needed for calculations
    let ticks_per_second = procfs::ticks_per_second() as f64;
    let time_delta = machine.time_delta_ms() as f64 / 1000.0;
    let total_mem_kb = machine.total_mem as f64;
    let boot_time = machine.boot_time;

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
                if exe_str.ends_with(" (deleted)") {
                    // Remove the " (deleted)" suffix from the path
                    let clean_path = exe_str.trim_end_matches(" (deleted)").to_string();
                    process.exe = Some(clean_path);
                    process.exe_deleted = true;
                } else {
                    process.exe = Some(exe_str.to_string());
                    process.exe_deleted = false;
                }
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
        if let Ok(io) = proc.io() {
            let prev_read = process.io_read_bytes;
            let prev_write = process.io_write_bytes;

            process.io_read_bytes = io.read_bytes;
            process.io_write_bytes = io.write_bytes;

            if !is_new && time_delta > 0.0 {
                if process.io_read_bytes >= prev_read {
                    process.io_read_rate = (process.io_read_bytes - prev_read) as f64 / time_delta;
                }
                if process.io_write_bytes >= prev_write {
                    process.io_write_rate =
                        (process.io_write_bytes - prev_write) as f64 / time_delta;
                }
            }
        }

        // OOM score
        if let Ok(oom) = std::fs::read_to_string(format!("/proc/{}/oom_score", pid)) {
            if let Ok(score) = oom.trim().parse::<i32>() {
                process.oom_score = score;
            }
        }

        // CGroup
        if let Ok(cgroups) = proc.cgroups() {
            if let Some(cgroup) = cgroups.0.first() {
                process.cgroup = Some(cgroup.pathname.clone());
            }
        }

        process.updated = true;
        machine.processes.add(process);
    }
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
