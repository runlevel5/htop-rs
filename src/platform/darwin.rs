//! macOS (Darwin) platform implementation
//!
//! This module provides macOS-specific implementations for reading
//! process, CPU, and memory information.

#![allow(dead_code)]

use anyhow::Result;
use libc::{c_int, c_void, size_t};
use std::ffi::CStr;
use std::mem;
use std::ptr;
use std::time::Duration;

use crate::core::{CpuData, Machine};
use crate::core::{Process, ProcessState};

// sysctl MIB constants
const CTL_KERN: c_int = 1;
const CTL_HW: c_int = 6;
const CTL_VM: c_int = 2;

const KERN_BOOTTIME: c_int = 21;
const KERN_PROC: c_int = 14;
const KERN_PROC_ALL: c_int = 0;
const KERN_ARGMAX: c_int = 8;
const KERN_PROCARGS2: c_int = 49;

const HW_NCPU: c_int = 3;
const HW_MEMSIZE: c_int = 24;
const HW_PAGESIZE: c_int = 7;

const VM_SWAPUSAGE: c_int = 5;

// Host statistics
const HOST_VM_INFO64: c_int = 4;
const HOST_CPU_LOAD_INFO: c_int = 3;
const PROCESSOR_CPU_LOAD_INFO: c_int = 2;

// CPU state indices
const CPU_STATE_USER: usize = 0;
const CPU_STATE_SYSTEM: usize = 1;
const CPU_STATE_IDLE: usize = 2;
const CPU_STATE_NICE: usize = 3;
const CPU_STATE_MAX: usize = 4;

/// External process info structure from libproc
#[repr(C)]
#[derive(Default)]
struct ProcBsdInfo {
    pbi_flags: u32,
    pbi_status: u32,
    pbi_xstatus: u32,
    pbi_pid: u32,
    pbi_ppid: u32,
    pbi_uid: u32,
    pbi_gid: u32,
    pbi_ruid: u32,
    pbi_rgid: u32,
    pbi_svuid: u32,
    pbi_svgid: u32,
    _reserved: u32,
    pbi_comm: [u8; 16],
    pbi_name: [u8; 32],
    pbi_nfiles: u32,
    pbi_pgid: u32,
    pbi_pjobc: u32,
    e_tdev: u32,
    e_tpgid: u32,
    pbi_nice: i32,
    pbi_start_tvsec: u64,
    pbi_start_tvusec: u64,
}

#[repr(C)]
struct ProcTaskInfo {
    pti_virtual_size: u64,
    pti_resident_size: u64,
    pti_total_user: u64,
    pti_total_system: u64,
    pti_threads_user: u64,
    pti_threads_system: u64,
    pti_policy: i32,
    pti_faults: i32,
    pti_pageins: i32,
    pti_cow_faults: i32,
    pti_messages_sent: i32,
    pti_messages_received: i32,
    pti_syscalls_mach: i32,
    pti_syscalls_unix: i32,
    pti_csw: i32,
    pti_threadnum: i32,
    pti_numrunning: i32,
    pti_priority: i32,
}

impl Default for ProcTaskInfo {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/// vnode_info_path structure from sys/proc_info.h
/// We only need the path field, but need to include the full vnode_info for correct sizing
#[repr(C)]
struct VnodeInfoPath {
    // vnode_info (vip_vi) - 152 bytes based on struct vinfo_stat + vi_type + vi_pad + vi_fsid
    _vi_padding: [u8; 152],
    // The actual path we care about
    vip_path: [u8; MAXPATHLEN],
}

impl Default for VnodeInfoPath {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/// proc_vnodepathinfo structure from sys/proc_info.h
#[repr(C)]
struct ProcVnodePathInfo {
    pvi_cdir: VnodeInfoPath, // current working directory
    pvi_rdir: VnodeInfoPath, // root directory
}

impl Default for ProcVnodePathInfo {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct VmStatistics64 {
    // natural_t fields (u32)
    free_count: u32,
    active_count: u32,
    inactive_count: u32,
    wire_count: u32,
    // u64 fields
    zero_fill_count: u64,
    reactivations: u64,
    pageins: u64,
    pageouts: u64,
    faults: u64,
    cow_faults: u64,
    lookups: u64,
    hits: u64,
    purges: u64,
    // natural_t fields (u32)
    purgeable_count: u32,
    speculative_count: u32,
    // u64 fields (added for rev1)
    decompressions: u64,
    compressions: u64,
    swapins: u64,
    swapouts: u64,
    // natural_t fields (u32)
    compressor_page_count: u32,
    throttled_count: u32,
    external_page_count: u32,
    internal_page_count: u32,
    // u64 field
    total_uncompressed_pages_in_compressor: u64,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct HostCpuLoadInfo {
    cpu_ticks: [u32; CPU_STATE_MAX],
}

/// Per-processor CPU load info (from host_processor_info)
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct ProcessorCpuLoadInfo {
    cpu_ticks: [u32; CPU_STATE_MAX],
}

#[repr(C)]
struct XswUsage {
    xsu_total: u64,
    xsu_avail: u64,
    xsu_used: u64,
    xsu_pagesize: u32,
    xsu_encrypted: bool,
}

extern "C" {
    fn mach_host_self() -> u32;
    fn mach_task_self() -> u32;
    fn host_statistics64(host: u32, flavor: c_int, info: *mut c_void, count: *mut u32) -> c_int;
    fn host_statistics(host: u32, flavor: c_int, info: *mut c_void, count: *mut u32) -> c_int;
    fn host_processor_info(
        host: u32,
        flavor: c_int,
        out_processor_count: *mut u32,
        out_processor_info: *mut *mut c_int,
        out_processor_info_cnt: *mut u32,
    ) -> c_int;
    fn vm_deallocate(target_task: u32, address: usize, size: usize) -> c_int;
}

// libproc functions
extern "C" {
    fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;
    fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
    fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
}

const PROC_PIDTASKINFO: c_int = 4;
const PROC_PIDTBSDINFO: c_int = 3;
const PROC_PIDVNODEPATHINFO: c_int = 9;
const MAXPATHLEN: usize = 1024;
const MAXCOMLEN: usize = 16;

/// Safe wrapper for sysctl that uses zeroed memory instead of Default
fn sysctl<T>(mib: &[c_int]) -> Option<T> {
    let mut value: T = unsafe { mem::zeroed() };
    let mut size = mem::size_of::<T>() as size_t;

    let ret = unsafe {
        libc::sysctl(
            mib.as_ptr() as *mut c_int,
            mib.len() as u32,
            &mut value as *mut T as *mut c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if ret == 0 {
        Some(value)
    } else {
        None
    }
}

/// Safe wrapper for sysctlbyname
fn sysctlbyname_string(name: &str) -> Option<String> {
    let cname = std::ffi::CString::new(name).ok()?;
    let mut size: size_t = 0;

    // First call to get size
    let ret = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            ptr::null_mut(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if ret != 0 || size == 0 {
        return None;
    }

    let mut buffer = vec![0u8; size];
    let ret = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            buffer.as_mut_ptr() as *mut c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if ret == 0 {
        // Remove trailing null
        if let Some(pos) = buffer.iter().position(|&c| c == 0) {
            buffer.truncate(pos);
        }
        String::from_utf8(buffer).ok()
    } else {
        None
    }
}

fn sysctlbyname_u64(name: &str) -> Option<u64> {
    let cname = std::ffi::CString::new(name).ok()?;
    let mut value: u64 = 0;
    let mut size = mem::size_of::<u64>() as size_t;

    let ret = unsafe {
        libc::sysctlbyname(
            cname.as_ptr(),
            &mut value as *mut u64 as *mut c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if ret == 0 {
        Some(value)
    } else {
        None
    }
}

/// Initialize macOS platform
pub fn init() -> Result<()> {
    Ok(())
}

/// Cleanup macOS platform
pub fn done() {}

/// Get system information
pub fn get_system_info(machine: &mut Machine) {
    // Get hostname
    let mut hostname = [0u8; 256];
    if unsafe { libc::gethostname(hostname.as_mut_ptr() as *mut i8, hostname.len()) } == 0 {
        if let Ok(name) = CStr::from_bytes_until_nul(&hostname) {
            machine.hostname = name.to_string_lossy().to_string();
        }
    }

    // Get kernel version
    if let Some(version) = sysctlbyname_string("kern.osrelease") {
        machine.kernel_version = format!("Darwin {}", version);
    }

    // Get boot time and calculate uptime
    let mib = [CTL_KERN, KERN_BOOTTIME];
    if let Some(boottime) = sysctl::<libc::timeval>(&mib) {
        machine.boot_time = boottime.tv_sec;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let uptime_secs = now.as_secs().saturating_sub(boottime.tv_sec as u64);
        machine.uptime = Duration::from_secs(uptime_secs);
    }

    // Get load averages
    let mut loadavg: [f64; 3] = [0.0; 3];
    if unsafe { libc::getloadavg(loadavg.as_mut_ptr(), 3) } == 3 {
        machine.load_average = loadavg;
    }
}

/// Scan CPU statistics
pub fn scan_cpu(machine: &mut Machine) {
    // Get number of CPUs first
    let mib = [CTL_HW, HW_NCPU];
    if let Some(ncpu) = sysctl::<c_int>(&mib) {
        let ncpu = ncpu as u32;
        if machine.cpus.len() != ncpu as usize {
            machine.cpus.clear();
            for _ in 0..ncpu {
                machine.cpus.push(CpuData::new());
            }
        }
        machine.active_cpus = ncpu;
        machine.existing_cpus = ncpu;
    }

    // Use host_processor_info to get per-CPU load data
    let host = unsafe { mach_host_self() };
    let mut cpu_count: u32 = 0;
    let mut processor_info: *mut c_int = ptr::null_mut();
    let mut processor_info_cnt: u32 = 0;

    let ret = unsafe {
        host_processor_info(
            host,
            PROCESSOR_CPU_LOAD_INFO,
            &mut cpu_count,
            &mut processor_info,
            &mut processor_info_cnt,
        )
    };

    if ret == 0 && !processor_info.is_null() {
        // Ensure we have the right number of CPUs
        let ncpu = cpu_count as usize;
        if machine.cpus.len() != ncpu {
            machine.cpus.clear();
            for _ in 0..ncpu {
                machine.cpus.push(CpuData::new());
            }
        }
        machine.active_cpus = cpu_count;
        machine.existing_cpus = cpu_count;

        // Each CPU has CPU_STATE_MAX (4) values: user, system, idle, nice
        // processor_info is an array of [cpu0_user, cpu0_sys, cpu0_idle, cpu0_nice, cpu1_user, ...]
        let cpu_info = processor_info as *const u32;

        // Accumulators for average CPU
        let mut total_user: u64 = 0;
        let mut total_system: u64 = 0;
        let mut total_idle: u64 = 0;
        let mut total_nice: u64 = 0;

        for i in 0..ncpu {
            let base = i * CPU_STATE_MAX;
            let user = unsafe { *cpu_info.add(base + CPU_STATE_USER) } as u64;
            let system = unsafe { *cpu_info.add(base + CPU_STATE_SYSTEM) } as u64;
            let idle = unsafe { *cpu_info.add(base + CPU_STATE_IDLE) } as u64;
            let nice = unsafe { *cpu_info.add(base + CPU_STATE_NICE) } as u64;

            // Update this CPU's data
            let cpu = &mut machine.cpus[i];
            cpu.user_time = user;
            cpu.system_time = system;
            cpu.idle_time = idle;
            cpu.nice_time = nice;
            cpu.update();

            // Accumulate for average
            total_user += user;
            total_system += system;
            total_idle += idle;
            total_nice += nice;
        }

        // Update average CPU (sum of all CPUs, not divided)
        machine.avg_cpu.user_time = total_user;
        machine.avg_cpu.system_time = total_system;
        machine.avg_cpu.idle_time = total_idle;
        machine.avg_cpu.nice_time = total_nice;
        machine.avg_cpu.update();

        // Deallocate the processor info buffer
        let buffer_size = processor_info_cnt as usize * mem::size_of::<c_int>();
        unsafe {
            vm_deallocate(mach_task_self(), processor_info as usize, buffer_size);
        }
    }
}

/// Scan memory statistics
pub fn scan_memory(machine: &mut Machine) {
    // Get total physical memory
    if let Some(memsize) = sysctlbyname_u64("hw.memsize") {
        machine.total_mem = memsize / 1024; // Convert to KB
    }

    // Get page size
    let mib = [CTL_HW, HW_PAGESIZE];
    let page_size = sysctl::<c_int>(&mib).unwrap_or(4096) as u64;

    // Get VM statistics
    let host = unsafe { mach_host_self() };
    let mut vm_stat: VmStatistics64 = Default::default();
    let mut count = (mem::size_of::<VmStatistics64>() / mem::size_of::<u32>()) as u32;

    let ret = unsafe {
        host_statistics64(
            host,
            HOST_VM_INFO64,
            &mut vm_stat as *mut _ as *mut c_void,
            &mut count,
        )
    };

    if ret == 0 {
        let page_kb = page_size / 1024;

        // Calculate memory values to match C htop darwin/Platform.c
        let wired = vm_stat.wire_count as u64 * page_kb;
        let active = vm_stat.active_count as u64 * page_kb;
        let compressed = vm_stat.compressor_page_count as u64 * page_kb;
        let purgeable = vm_stat.purgeable_count as u64 * page_kb;
        let inactive = vm_stat.inactive_count as u64 * page_kb;

        // Used memory (excluding compressed, like C htop)
        machine.used_mem = wired + active;
        machine.compressed_mem = compressed;
        machine.buffers_mem = purgeable; // Purgeable = buffers in C htop
        machine.cached_mem = inactive; // Inactive = cache in C htop

        // Available memory
        let free = vm_stat.free_count as u64 * page_kb;
        let speculative = vm_stat.speculative_count as u64 * page_kb;
        machine.available_mem = free + inactive + speculative;
    }

    // Get swap usage
    let mib = [CTL_VM, VM_SWAPUSAGE];
    let mut size = mem::size_of::<XswUsage>() as size_t;
    let mut swap: XswUsage = unsafe { mem::zeroed() };

    let ret = unsafe {
        libc::sysctl(
            mib.as_ptr() as *mut c_int,
            mib.len() as u32,
            &mut swap as *mut _ as *mut c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if ret == 0 {
        machine.total_swap = swap.xsu_total / 1024;
        machine.used_swap = swap.xsu_used / 1024;
    }
}

/// Convert macOS process state to our ProcessState
/// This is only used for zombie/stopped states from kinfo_proc p_stat
fn convert_process_state(status: u32) -> ProcessState {
    // macOS process status values (from sys/proc.h)
    const SIDL: u32 = 1; // Process being created
    const SRUN: u32 = 2; // Running
    const SSLEEP: u32 = 3; // Sleeping
    const SSTOP: u32 = 4; // Stopped
    const SZOMB: u32 = 5; // Zombie

    match status {
        SIDL => ProcessState::Idle,
        SRUN => ProcessState::Running,
        SSLEEP => ProcessState::Sleeping,
        SSTOP => ProcessState::Stopped,
        SZOMB => ProcessState::Zombie,
        _ => ProcessState::Unknown,
    }
}

/// Determine process state like C htop does
/// Uses pti_numrunning for Running vs Sleeping, and pbi_status for special states
fn determine_process_state(pbi_status: u32, pti_numrunning: i32) -> ProcessState {
    // macOS process status values (from sys/proc.h)
    const SSTOP: u32 = 4; // Stopped
    const SZOMB: u32 = 5; // Zombie

    // First check for zombie/stopped states from BSD info
    // (matches C htop DarwinProcessTable.c lines 107-110)
    if pbi_status == SZOMB {
        return ProcessState::Zombie;
    }
    if pbi_status == SSTOP {
        return ProcessState::Stopped;
    }

    // Use pti_numrunning to determine running vs sleeping
    // (matches C htop DarwinProcess.c line 387)
    if pti_numrunning > 0 {
        ProcessState::Running
    } else {
        ProcessState::Sleeping
    }
}

/// Get command line arguments for a process
/// Returns (cmdline, basename_end) where basename_end is the position after the first argument
/// This matches C htop's DarwinProcess_setFromKInfoProc/KERN_PROCARGS2 logic
fn get_process_cmdline(pid: i32) -> Option<(String, usize)> {
    // Get the maximum argument size
    let mut argmax: c_int = 0;
    let mut size = mem::size_of::<c_int>();
    let mut mib: [c_int; 2] = [CTL_KERN, KERN_ARGMAX];

    let ret = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            2,
            &mut argmax as *mut _ as *mut c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if ret != 0 || argmax <= 0 {
        return None;
    }

    // Allocate buffer for arguments
    let mut procargs: Vec<u8> = vec![0; argmax as usize];
    let mut size = argmax as size_t;
    let mut mib: [c_int; 3] = [CTL_KERN, KERN_PROCARGS2, pid];

    let ret = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            3,
            procargs.as_mut_ptr() as *mut c_void,
            &mut size,
            ptr::null_mut(),
            0,
        )
    };

    if ret != 0 {
        return None;
    }

    // Parse the procargs buffer
    // Layout: nargs (int) | exec_path | \0+ | arg0 | \0 | arg1 | \0 | ...
    if size < mem::size_of::<c_int>() {
        return None;
    }

    // Get nargs
    let nargs = i32::from_ne_bytes([procargs[0], procargs[1], procargs[2], procargs[3]]);
    if nargs <= 0 {
        return None;
    }

    let mut pos = mem::size_of::<c_int>();

    // Skip exec_path (find first \0)
    while pos < size && procargs[pos] != 0 {
        pos += 1;
    }
    if pos >= size {
        return None;
    }

    // Skip trailing \0 characters
    while pos < size && procargs[pos] == 0 {
        pos += 1;
    }
    if pos >= size {
        return None;
    }

    // Now we're at the start of arg0
    let _args_start = pos;
    let mut arg_count = 0;
    let mut cmdline = String::new();
    let mut basename_end = 0;

    while pos < size && arg_count < nargs as usize {
        if procargs[pos] == 0 {
            arg_count += 1;
            if arg_count == 1 {
                // End of first argument - record position in cmdline
                basename_end = cmdline.len();
            }
            if arg_count < nargs as usize {
                // Add space between arguments
                cmdline.push(' ');
            }
        } else {
            // Add character to cmdline
            cmdline.push(procargs[pos] as char);
        }
        pos += 1;
    }

    // If basename_end wasn't set (single arg), it's the whole string
    if basename_end == 0 {
        basename_end = cmdline.len();
    }

    if cmdline.is_empty() {
        None
    } else {
        Some((cmdline, basename_end))
    }
}

/// Get the current working directory for a process
/// This matches C htop's DarwinProcess_updateCwd behavior
fn get_process_cwd(pid: i32) -> Option<String> {
    let mut vpi: ProcVnodePathInfo = Default::default();
    
    let result = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDVNODEPATHINFO,
            0,
            &mut vpi as *mut _ as *mut c_void,
            mem::size_of::<ProcVnodePathInfo>() as c_int,
        )
    };
    
    if result <= 0 {
        return None;
    }
    
    // Check if path is empty
    if vpi.pvi_cdir.vip_path[0] == 0 {
        return None;
    }
    
    // Convert to string
    let path_bytes = &vpi.pvi_cdir.vip_path;
    if let Some(pos) = path_bytes.iter().position(|&c| c == 0) {
        if let Ok(path) = std::str::from_utf8(&path_bytes[..pos]) {
            return Some(path.to_string());
        }
    }
    
    None
}

/// Scan all processes
pub fn scan_processes(machine: &mut Machine) {
    scan_processes_with_settings(machine, false);
}

/// Scan all processes with settings control
pub fn scan_processes_with_settings(machine: &mut Machine, update_process_names: bool) {
    // Get list of all PIDs
    let mut pids: Vec<i32> = vec![0; 4096];
    let count = unsafe {
        proc_listallpids(
            pids.as_mut_ptr() as *mut c_void,
            (pids.len() * mem::size_of::<i32>()) as c_int,
        )
    };

    if count <= 0 {
        return;
    }

    let pid_count = count as usize;
    pids.truncate(pid_count);

    // Get current time for CPU% calculation
    let time_delta = machine.time_delta_ms() as f64 / 1000.0;

    // Get total memory for MEM% calculation
    let total_mem_kb = machine.total_mem as f64;

    // Reset task counts (will be accumulated during scan)
    // Match C htop: kernelThreads is always 0 on Darwin, userlandThreads is sum of pti_threadnum
    machine.total_tasks = 0;
    machine.running_tasks = 0;
    machine.userland_threads = 0;
    machine.kernel_threads = 0;

    // Track max values for dynamic column widths
    let mut max_pid: i32 = 0;
    let mut max_uid: u32 = 0;
    let mut max_percent_cpu: f32 = 0.0;

    // Process each PID
    for &pid in &pids {
        if pid <= 0 {
            continue;
        }

        // Get BSD info
        let mut bsd_info: ProcBsdInfo = Default::default();
        let bsd_size = unsafe {
            proc_pidinfo(
                pid,
                PROC_PIDTBSDINFO,
                0,
                &mut bsd_info as *mut _ as *mut c_void,
                mem::size_of::<ProcBsdInfo>() as c_int,
            )
        };

        if bsd_size <= 0 {
            continue;
        }

        // Get task info
        let mut task_info: ProcTaskInfo = Default::default();
        let task_size = unsafe {
            proc_pidinfo(
                pid,
                PROC_PIDTASKINFO,
                0,
                &mut task_info as *mut _ as *mut c_void,
                mem::size_of::<ProcTaskInfo>() as c_int,
            )
        };

        // Create or update process
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

        // Basic info from BSD
        process.pid = pid;
        process.ppid = bsd_info.pbi_ppid as i32;
        process.pgrp = bsd_info.pbi_pgid as i32;
        process.uid = bsd_info.pbi_uid;
        process.nice = bsd_info.pbi_nice as i64;
        process.starttime_ctime = bsd_info.pbi_start_tvsec as i64;

        // Determine process state using task info (pti_numrunning)
        // This matches C htop behavior - see DarwinProcess.c line 387
        let pti_numrunning = if task_size > 0 {
            task_info.pti_numrunning
        } else {
            0
        };
        process.state = determine_process_state(bsd_info.pbi_status, pti_numrunning);

        // Get username
        process.user = Some(machine.get_username(process.uid));

        // Only update process name/command if it's new or update_process_names is enabled
        // (matches C htop Settings.updateProcessNames behavior)
        if is_new || update_process_names {
            // Get process name/command
            let mut name_buf = [0u8; MAXCOMLEN + 1];
            let name_len = unsafe {
                proc_name(
                    pid,
                    name_buf.as_mut_ptr() as *mut c_void,
                    name_buf.len() as u32,
                )
            };

            if name_len > 0 {
                if let Ok(name) = CStr::from_bytes_until_nul(&name_buf) {
                    process.comm = Some(name.to_string_lossy().to_string());
                }
            }

            // Fallback to pbi_comm
            if process.comm.is_none() {
                if let Some(pos) = bsd_info.pbi_comm.iter().position(|&c| c == 0) {
                    if let Ok(comm) = std::str::from_utf8(&bsd_info.pbi_comm[..pos]) {
                        process.comm = Some(comm.to_string());
                    }
                }
            }

            // Get executable path
            let mut path_buf = [0u8; MAXPATHLEN];
            let path_len = unsafe {
                proc_pidpath(pid, path_buf.as_mut_ptr() as *mut c_void, MAXPATHLEN as u32)
            };

            if path_len > 0 {
                if let Ok(path) = CStr::from_bytes_until_nul(&path_buf) {
                    let path_str = path.to_string_lossy().to_string();
                    process.exe = Some(path_str);
                }
            }

            // Get full command line with arguments via KERN_PROCARGS2
            // This matches C htop's DarwinProcess_setFromKInfoProc behavior
            if let Some((cmdline, basename_end)) = get_process_cmdline(pid) {
                process.update_cmdline(cmdline, basename_end);
            } else if let Some(ref exe) = process.exe {
                // Fallback to exe path if KERN_PROCARGS2 fails
                process.update_cmdline(exe.clone(), 0);
            } else if let Some(ref comm) = process.comm {
                // Fallback to comm
                process.update_cmdline(comm.clone(), comm.len());
            }
        }

        // Get current working directory (matching C htop DarwinProcess_updateCwd)
        process.cwd = get_process_cwd(pid);

        // Task info
        if task_size > 0 {
            // Memory (convert bytes to KB)
            process.m_virt = (task_info.pti_virtual_size / 1024) as i64;
            process.m_resident = (task_info.pti_resident_size / 1024) as i64;

            // CPU time (in nanoseconds -> convert to hundredths of a second)
            let total_time_ns = task_info.pti_total_user + task_info.pti_total_system;
            process.time = total_time_ns / 10_000_000; // ns to centiseconds

            // Thread count
            process.nlwp = task_info.pti_threadnum as i64;

            // Priority
            process.priority = task_info.pti_priority as i64;

            // Page faults
            process.minflt = task_info.pti_faults as u64;
            process.majflt = task_info.pti_pageins as u64;

            // Calculate CPU%
            if !is_new && time_delta > 0.0 && prev_time < process.time {
                let time_diff = (process.time - prev_time) as f64;
                // time_diff is in centiseconds, time_delta is in seconds
                process.percent_cpu = ((time_diff / 100.0) / time_delta * 100.0) as f32;
                // Clamp to reasonable value
                process.percent_cpu = process.percent_cpu.min(100.0 * machine.active_cpus as f32);
            }

            // Calculate MEM%
            if total_mem_kb > 0.0 {
                process.percent_mem = ((process.m_resident as f64 / total_mem_kb) * 100.0) as f32;
            }

            // Accumulate task counts (matching C htop darwin/DarwinProcess.c)
            // kernelThreads is always 0 on Darwin
            // userlandThreads is sum of pti_threadnum across all processes
            // totalTasks includes both the process AND its threads
            machine.userland_threads += task_info.pti_threadnum as u32;
            machine.total_tasks += task_info.pti_threadnum as u32; // Add threads to totalTasks
            machine.running_tasks += task_info.pti_numrunning as u32;
        }

        // Count this process in total_tasks (matching DarwinProcessTable.c line 125)
        machine.total_tasks += 1;

        // Check if kernel thread (PID 0 or ppid 0 with specific patterns)
        process.is_kernel_thread = pid == 0 || process.ppid == 0;

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
