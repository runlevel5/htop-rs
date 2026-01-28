//! Linux platform implementation
//!
//! This module provides Linux-specific implementations for reading
//! process, CPU, and memory information from /proc filesystem.

use anyhow::Result;
use procfs::{CpuTime, Current, CurrentSI, KernelStats, Meminfo};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::core::{CpuData, Machine, Process, ProcessState, ScanFlags};

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

    machine.available_mem = meminfo
        .mem_available
        .map(|v| (v / 1024) as i64)
        .unwrap_or(-1);
    machine.shared_mem = meminfo.shmem.map(|v| (v / 1024) as i64).unwrap_or(-1);

    // Swap
    machine.total_swap = meminfo.swap_total / 1024;
    let swap_free = meminfo.swap_free / 1024;
    machine.used_swap = machine.total_swap.saturating_sub(swap_free);
    machine.cached_swap = meminfo.swap_cached / 1024;
}

/// Check if a PID is a kernel thread (PID 2 is kthreadd, kernel threads have ppid=2)
fn is_kernel_thread_pid(pid: i32) -> bool {
    if pid == 2 {
        return true;
    }
    // Check ppid - kernel threads have kthreadd (PID 2) as parent
    if let Ok(content) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        // Parse ppid from stat (4th field after comm which is in parentheses)
        if let Some(end_paren) = content.rfind(')') {
            let after_comm = &content[end_paren + 1..];
            let fields: Vec<&str> = after_comm.split_whitespace().collect();
            // fields[1] is ppid (index 0 is state)
            if fields.len() > 1 {
                if let Ok(ppid) = fields[1].parse::<i32>() {
                    return ppid == 2;
                }
            }
        }
    }
    false
}

/// Scan all processes
pub fn scan_processes(machine: &mut Machine) {
    // Timing instrumentation (enabled via HTOP_DEBUG_TIMING=1 env var)
    let debug_timing = std::env::var("HTOP_DEBUG_TIMING")
        .map(|v| v == "1")
        .unwrap_or(false);
    let scan_start = if debug_timing {
        Some(std::time::Instant::now())
    } else {
        None
    };

    // Get scan flags for conditional expensive reads
    let flags = machine.scan_flags;

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
    let page_size = (procfs::page_size() / 1024) as i64;

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
        process.m_virt = (stat.vsize / 1024) as i64;
        process.m_resident = (stat.rss as i64) * page_size;

        // Extract comm from stat before dropping (move instead of clone to avoid allocation)
        let stat_comm = stat.comm;

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
            // Get command name from stat (move instead of clone - saves ~16 bytes allocation per process)
            process.comm = Some(stat_comm);

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

        // Try to get cwd - only read when CWD column is displayed
        if flags.contains(ScanFlags::CWD) {
            if let Ok(cwd) = proc.cwd() {
                process.cwd = Some(cwd.to_string_lossy().to_string());
            }
        }

        // Get UID using fast fstat() instead of parsing /proc/PID/status (51x faster!)
        // proc.uid() uses fstat() on the already-open directory FD
        if let Ok(uid) = proc.uid() {
            process.uid = uid;
            process.user = Some(machine.get_username(process.uid));
        }

        // Memory details from statm - only read when M_SHARE/M_TEXT/M_DATA columns displayed
        if flags.contains(ScanFlags::STATM) {
            if let Ok(statm) = proc.statm() {
                process.m_share = (statm.shared * page_size as u64) as i64;
                process.m_text = (statm.text * page_size as u64) as i64;
                process.m_data = (statm.data * page_size as u64) as i64;
            }
        }

        // Check for kernel thread (ppid == 2 is kthreadd)
        process.is_kernel_thread = process.ppid == 2 || pid == 2;

        // For main processes (enumerated from /proc/), TGID always equals PID
        // Threads are only found in /proc/PID/task/, not at the top /proc/ level
        // This eliminates the need to read /proc/PID/status just for TGID
        process.tgid = pid;
        process.is_userland_thread = false;

        // IO statistics - only read when IO columns are displayed
        // (requires root or same user)
        if flags.contains(ScanFlags::IO) {
            if let Ok(io) = proc.io() {
                let prev_read = process.io_read_bytes;
                let prev_write = process.io_write_bytes;

                process.io_read_bytes = io.read_bytes;
                process.io_write_bytes = io.write_bytes;

                if !is_new && time_delta > 0.0 {
                    if process.io_read_bytes >= prev_read {
                        process.io_read_rate =
                            (process.io_read_bytes - prev_read) as f64 / time_delta;
                    }
                    if process.io_write_bytes >= prev_write {
                        process.io_write_rate =
                            (process.io_write_bytes - prev_write) as f64 / time_delta;
                    }
                }
            }
        }

        // Check for deleted libraries by scanning /proc/PID/maps
        // Only check if:
        // - check_deleted_libs is enabled (highlight_deleted_exe setting)
        // - exe is not deleted (if exe is deleted, no need to check libs)
        // - not a kernel thread or userland thread
        // Throttle check to every ~2 seconds per process (like C htop) to avoid performance hit
        const DELETED_LIB_CHECK_INTERVAL_MS: u64 = 2000;
        if machine.check_deleted_libs
            && !process.exe_deleted
            && !process.is_kernel_thread
            && !process.is_userland_thread
        {
            let time_since_last_check = machine
                .realtime_ms
                .saturating_sub(process.last_deleted_lib_check_ms);
            if process.last_deleted_lib_check_ms == 0
                || time_since_last_check >= DELETED_LIB_CHECK_INTERVAL_MS
            {
                process.uses_deleted_lib = check_deleted_libs(pid);
                process.last_deleted_lib_check_ms = machine.realtime_ms;
            }
        }

        // OOM score - only read when OOM column is displayed
        if flags.contains(ScanFlags::OOM) {
            if let Ok(oom) = std::fs::read_to_string(format!("/proc/{}/oom_score", pid)) {
                if let Ok(score) = oom.trim().parse::<i32>() {
                    process.oom_score = score;
                }
            }
        }

        // IO Priority - only read when IO_PRIORITY column is displayed
        // (syscall per process)
        if flags.contains(ScanFlags::IO_PRIORITY) {
            process.io_priority = get_io_priority(pid);
        }

        // CGroup - only read when CGROUP/CCGROUP/CONTAINER columns are displayed
        if flags.contains(ScanFlags::CGROUP) {
            if let Ok(cgroups) = proc.cgroups() {
                if let Some(cgroup) = cgroups.0.first() {
                    let cgroup_path = cgroup.pathname.clone();
                    // Generate compressed cgroup name and detect container
                    process.cgroup_short = Some(filter_cgroup_name(&cgroup_path));
                    process.container_short = filter_container(&cgroup_path);
                    process.cgroup = Some(cgroup_path);
                }
            }
        }

        // Smaps data (PSS, Swap, SwapPss) - DISABLED: too expensive, causes 100% CPU
        // TODO: Only read when PSS/MSwap/MPsswp columns are displayed
        // if !process.is_kernel_thread {
        //     read_smaps_file(&mut process, pid);
        // }

        // Autogroup data - DISABLED: causes CPU spike
        // TODO: Only read when autogroup columns are displayed
        // read_autogroup(&mut process, pid);

        // Library size from maps - DISABLED: too expensive, causes 100% CPU
        // TODO: Only read when M_LIB column is displayed
        // if !process.is_kernel_thread && !process.is_userland_thread {
        //     read_maps_for_lib_size(&mut process, pid, page_size);
        // }

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

        // Store main process info for thread inheritance
        let main_uid = process.uid;
        let main_user = process.user.clone();
        let main_cmdline = process.cmdline.clone();
        let main_cmdline_basename_start = process.cmdline_basename_start;
        let main_cmdline_basename_end = process.cmdline_basename_end;
        let main_exe = process.exe.clone();
        let main_exe_basename_offset = process.exe_basename_offset;
        let main_exe_deleted = process.exe_deleted;
        let main_cwd = process.cwd.clone();
        let main_tty_nr = process.tty_nr;
        let main_tty_name = process.tty_name.clone();
        let main_cgroup = process.cgroup.clone();
        let main_cgroup_short = process.cgroup_short.clone();
        let main_container_short = process.container_short.clone();

        machine.processes.add(process, machine.monotonic_ms);

        // Scan threads (tasks) for this process
        // Skip if:
        // - This is a kernel thread (kernel threads don't have userland tasks)
        // - hide_userland_threads is enabled AND we've already discovered threads once
        //   (optimization: skip expensive task directory scan for hidden threads)
        //
        // We always scan threads at least once (threads_discovered=false) to populate
        // the list, even if hide_userland_threads is true. This ensures threads exist
        // in the list when the user toggles to show them.
        let should_scan_threads = (!machine.hide_userland_threads || !machine.threads_discovered)
            && !is_kernel_thread_pid(pid);
        if should_scan_threads {
            if let Ok(tasks) = proc.tasks() {
                for task_result in tasks {
                    let task = match task_result {
                        Ok(t) => t,
                        Err(_) => continue,
                    };

                    // Skip the main thread (tid == pid)
                    let tid = task.tid;
                    if tid == pid {
                        continue;
                    }

                    // Get task stat
                    let task_stat = match task.stat() {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    // Check if thread already exists
                    let thread_is_new = machine.processes.get(tid).is_none();
                    let thread_prev_time = if !thread_is_new {
                        machine.processes.get(tid).map(|p| p.time).unwrap_or(0)
                    } else {
                        0
                    };

                    let mut thread = if thread_is_new {
                        Process::new(tid)
                    } else {
                        machine
                            .processes
                            .get(tid)
                            .cloned()
                            .unwrap_or_else(|| Process::new(tid))
                    };

                    // Basic info from task stat
                    thread.pid = tid;
                    thread.ppid = pid; // Thread's parent is the main process
                    thread.tgid = pid; // Thread group ID is the main process PID
                    thread.pgrp = task_stat.pgrp;
                    thread.session = task_stat.session;
                    thread.tty_nr = main_tty_nr;
                    thread.tty_name = main_tty_name.clone();
                    thread.tpgid = task_stat.tpgid;
                    thread.priority = task_stat.priority;
                    thread.nice = task_stat.nice;
                    thread.nlwp = 1; // Threads show as 1
                    thread.processor = task_stat.processor.unwrap_or(-1);
                    thread.minflt = task_stat.minflt;
                    thread.majflt = task_stat.majflt;

                    // Process state
                    thread.state = convert_process_state(task_stat.state);

                    // CPU time
                    let thread_utime = task_stat.utime;
                    let thread_stime = task_stat.stime;
                    let thread_total_ticks = thread_utime + thread_stime;
                    thread.time = ((thread_total_ticks as f64 / ticks_per_second) * 100.0) as u64;
                    thread.utime = ((thread_utime as f64 / ticks_per_second) * 100.0) as u64;
                    thread.stime = ((thread_stime as f64 / ticks_per_second) * 100.0) as u64;

                    // Calculate CPU percentage
                    if !thread_is_new && time_delta > 0.0 && thread.time > thread_prev_time {
                        let time_diff = (thread.time - thread_prev_time) as f64;
                        thread.percent_cpu = ((time_diff / 100.0) / time_delta * 100.0) as f32;
                        thread.percent_cpu =
                            thread.percent_cpu.min(100.0 * machine.active_cpus as f32);
                    }

                    // Start time
                    thread.starttime_ctime =
                        boot_time + (task_stat.starttime as i64 / ticks_per_second as i64);

                    // Memory (shared with main process, but we read from task stat for consistency)
                    thread.m_virt = (task_stat.vsize / 1024) as i64;
                    thread.m_resident = (task_stat.rss as i64) * page_size;
                    if total_mem_kb > 0.0 {
                        thread.percent_mem =
                            ((thread.m_resident as f64 / total_mem_kb) * 100.0) as f32;
                    }

                    // Extract thread's own comm (move instead of clone to avoid allocation)
                    let thread_comm = task_stat.comm;

                    // Inherit shared data from main process
                    thread.uid = main_uid;
                    thread.user = main_user.clone();
                    thread.comm = Some(thread_comm); // Thread has its own comm (moved, not cloned)
                    thread.cmdline = main_cmdline.clone();
                    thread.cmdline_basename_start = main_cmdline_basename_start;
                    thread.cmdline_basename_end = main_cmdline_basename_end;
                    thread.exe = main_exe.clone();
                    thread.exe_basename_offset = main_exe_basename_offset;
                    thread.exe_deleted = main_exe_deleted;
                    thread.cwd = main_cwd.clone();
                    thread.cgroup = main_cgroup.clone();
                    thread.cgroup_short = main_cgroup_short.clone();
                    thread.container_short = main_container_short.clone();

                    // Mark as userland thread
                    thread.is_userland_thread = true;
                    thread.is_kernel_thread = false;

                    // IO Priority (thread-specific)
                    thread.io_priority = get_io_priority(tid);

                    // Track max values
                    if tid > max_pid {
                        max_pid = tid;
                    }
                    if thread.percent_cpu > max_percent_cpu {
                        max_percent_cpu = thread.percent_cpu;
                    }

                    thread.updated = true;
                    machine.processes.add(thread, machine.monotonic_ms);
                }
            }
        }
    }

    // When hide_userland_threads is enabled, we skip scanning threads but they still
    // exist in the process list from previous scans. We need to mark them as updated
    // so they don't get pruned by cleanup(). This matches C htop's behavior where
    // pre-existing hidden threads are marked as updated but expensive reads are skipped.
    if machine.hide_userland_threads {
        for process in &mut machine.processes.processes {
            if process.is_userland_thread {
                process.updated = true;
            }
        }
    }

    // Mark that we've done at least one thread scan
    machine.threads_discovered = true;

    // Update dynamic field widths based on scan results
    machine.max_pid = max_pid;
    machine.max_user_id = max_uid;
    machine.field_widths.set_pid_width(max_pid);
    machine.field_widths.set_uid_width(max_uid);
    machine
        .field_widths
        .update_percent_cpu_width(max_percent_cpu);

    // Output timing if debug enabled
    if let Some(start) = scan_start {
        let elapsed = start.elapsed();
        eprintln!(
            "[SCAN] {:>8.2}ms ({} processes, flags={:?})",
            elapsed.as_secs_f64() * 1000.0,
            machine.processes.len(),
            flags
        );
    }
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

/// Scan disk IO statistics using procfs crate
/// This matches C htop's Platform_getDiskIO behavior
pub fn scan_disk_io(machine: &mut Machine) {
    // Rate limiting: only update every DISK_IO_CACHE_DELAY_MS
    let now_ms = machine.realtime_ms;
    if machine.disk_io_last_update > 0
        && now_ms < machine.disk_io_last_update + DISK_IO_CACHE_DELAY_MS
    {
        return;
    }

    let stats = match procfs::diskstats() {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut last_top_disk = String::new();
    let mut read_sum: u64 = 0;
    let mut write_sum: u64 = 0;
    let mut time_spend_sum: u64 = 0;
    let mut num_disks: u64 = 0;

    for stat in stats {
        let diskname = &stat.name;

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

        // Update last top disk
        last_top_disk = diskname.clone();

        read_sum += stat.sectors_read;
        write_sum += stat.sectors_written;
        time_spend_sum += stat.time_in_progress;
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

/// Scan network IO statistics using procfs crate
/// This matches C htop's Platform_getNetworkIO behavior
pub fn scan_network_io(machine: &mut Machine) {
    // Rate limiting: only update every NETWORK_IO_CACHE_DELAY_MS
    let now_ms = machine.realtime_ms;
    if machine.net_io_last_update > 0
        && now_ms < machine.net_io_last_update + NETWORK_IO_CACHE_DELAY_MS
    {
        return;
    }

    let dev_stats = match procfs::net::dev_status() {
        Ok(stats) => stats,
        Err(_) => return,
    };

    let mut bytes_received: u64 = 0;
    let mut packets_received: u64 = 0;
    let mut bytes_transmitted: u64 = 0;
    let mut packets_transmitted: u64 = 0;

    for (interface, status) in dev_stats {
        // Skip loopback interface (like C htop)
        if interface == "lo" {
            continue;
        }

        bytes_received += status.recv_bytes;
        packets_received += status.recv_packets;
        bytes_transmitted += status.sent_bytes;
        packets_transmitted += status.sent_packets;
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

/// Scan file descriptor statistics using procfs crate
/// This matches C htop's Platform_getFileDescriptors behavior
pub fn scan_file_descriptors(machine: &mut Machine) {
    match procfs::sys::fs::file_nr() {
        Ok(state) => {
            machine.fd_used = Some(state.allocated);
            machine.fd_max = Some(state.max);
        }
        Err(_) => {
            machine.fd_used = None;
            machine.fd_max = Some(65536); // Default fallback
        }
    }
}

/// Read /proc/[pid]/io for detailed IO statistics
/// This matches C htop's LinuxProcessTable_readIoFile behavior
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
fn parse_smaps_value(line: &str, skip: usize) -> Option<i64> {
    line.get(skip..)?
        .split_whitespace()
        .next()?
        .parse::<i64>()
        .ok()
}

/// Read /proc/[pid]/autogroup for autogroup ID and nice value
/// This matches C htop's LinuxProcessTable_readAutogroup behavior
#[allow(dead_code)]
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

    // Parse using strip_prefix
    if let Some(rest) = content.strip_prefix("/autogroup-") {
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
#[allow(dead_code)]
fn read_maps_for_lib_size(process: &mut Process, pid: i32, _page_size_kb: i64) {
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
