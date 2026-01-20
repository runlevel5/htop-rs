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
        machine.load_average = [loadavg.one as f64, loadavg.five as f64, loadavg.fifteen as f64];
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
        'K' => ProcessState::Idle,  // Wakekill
        'P' => ProcessState::Idle,  // Parked
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
        let prev_total = prev.user + prev.nice + prev.system + prev.idle 
            + prev.iowait.unwrap_or(0) + prev.irq.unwrap_or(0) 
            + prev.softirq.unwrap_or(0) + prev.steal.unwrap_or(0);
        
        let curr_total = current.user + current.nice + current.system + current.idle
            + current.iowait.unwrap_or(0) + current.irq.unwrap_or(0)
            + current.softirq.unwrap_or(0) + current.steal.unwrap_or(0);
        
        let total_delta = (curr_total - prev_total) as f64;
        
        if total_delta > 0.0 {
            cpu.user_percent = ((current.user - prev.user) as f64 / total_delta) * 100.0;
            cpu.nice_percent = ((current.nice - prev.nice) as f64 / total_delta) * 100.0;
            cpu.system_percent = ((current.system - prev.system) as f64 / total_delta) * 100.0;
            cpu.irq_percent = ((current.irq.unwrap_or(0) - prev.irq.unwrap_or(0)) as f64 / total_delta) * 100.0;
            cpu.softirq_percent = ((current.softirq.unwrap_or(0) - prev.softirq.unwrap_or(0)) as f64 / total_delta) * 100.0;
            cpu.steal_percent = ((current.steal.unwrap_or(0) - prev.steal.unwrap_or(0)) as f64 / total_delta) * 100.0;
            cpu.guest_percent = ((current.guest.unwrap_or(0) - prev.guest.unwrap_or(0)) as f64 / total_delta) * 100.0;
            cpu.iowait_percent = ((current.iowait.unwrap_or(0) - prev.iowait.unwrap_or(0)) as f64 / total_delta) * 100.0;
            
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
    
    machine.used_mem = machine.total_mem
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
            machine.processes.get(pid).cloned().unwrap_or_else(|| Process::new(pid))
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
        
        // Get command name
        process.comm = Some(stat.comm.clone());
        
        // Try to get full cmdline
        if let Ok(cmdline) = proc.cmdline() {
            if !cmdline.is_empty() {
                process.cmdline = Some(cmdline.join(" "));
            }
        }
        
        // Try to get exe path
        if let Ok(exe) = proc.exe() {
            process.exe = Some(exe.to_string_lossy().to_string());
        }
        
        // Try to get cwd
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
                    process.io_write_rate = (process.io_write_bytes - prev_write) as f64 / time_delta;
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
