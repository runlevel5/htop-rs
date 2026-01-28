//! macOS-specific background scanner for expensive process data
//!
//! This module provides macOS-specific expensive process data collection
//! using the generic BackgroundScanner framework and rayon's thread pool.
//! Results are collected in the background and merged on the next scan cycle.
//!
//! On macOS, the main expensive operation is `get_process_cwd()` which calls
//! `proc_pidinfo(PROC_PIDVNODEPATHINFO)` for each process. This can be slow
//! when there are many processes.

use rayon::prelude::*;
use std::collections::HashMap;

use super::bg_scanner::BackgroundScanner;

/// Data collected from expensive syscalls for a single process on macOS
#[derive(Default, Clone, Debug)]
pub struct DarwinExpensiveData {
    /// Current working directory (from proc_pidinfo PROC_PIDVNODEPATHINFO)
    pub cwd: Option<String>,
}

impl DarwinExpensiveData {
    /// Check if any data was collected
    pub fn has_data(&self) -> bool {
        self.cwd.is_some()
    }
}

/// Type alias for the Darwin background scanner
pub type DarwinBackgroundScanner = BackgroundScanner<DarwinExpensiveData>;

/// Result from background scanner - maps PID to collected data
pub type DarwinExpensiveDataMap = HashMap<i32, DarwinExpensiveData>;

/// Start a background scan for the given PIDs using Darwin-specific collection
pub fn start_darwin_bg_scan(scanner: &mut DarwinBackgroundScanner, pids: Vec<i32>) {
    scanner.start_scan(move || collect_expensive_data(&pids));
}

/// Collect expensive data for all PIDs using rayon parallel iterator
fn collect_expensive_data(pids: &[i32]) -> DarwinExpensiveDataMap {
    pids.par_iter()
        .filter_map(|&pid| {
            let data = collect_for_pid(pid);
            // Only include if we got some data
            if data.has_data() {
                Some((pid, data))
            } else {
                None
            }
        })
        .collect()
}

/// Collect expensive data for a single PID
fn collect_for_pid(pid: i32) -> DarwinExpensiveData {
    let mut data = DarwinExpensiveData::default();

    // Skip kernel_task (PID 0)
    if pid == 0 {
        return data;
    }

    // Get current working directory using proc_pidinfo
    data.cwd = get_process_cwd(pid);

    data
}

// ----- Inline copy of cwd fetching logic to avoid circular dependency -----
// We need these here because darwin.rs might not be visible to this module
// and we want to keep the expensive operations self-contained.

use libc::{c_int, c_void};
use std::mem;

const PROC_PIDVNODEPATHINFO: c_int = 9;
const MAXPATHLEN: usize = 1024;

/// vnode_info_path structure from sys/proc_info.h
#[repr(C)]
struct VnodeInfoPath {
    // vnode_info (vip_vi) - 152 bytes
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

extern "C" {
    fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
}

/// Get the current working directory for a process
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_process_cwd_self() {
        // Test getting our own cwd
        let pid = std::process::id() as i32;
        let cwd = get_process_cwd(pid);
        assert!(cwd.is_some(), "Should be able to get own process cwd");
    }
}
