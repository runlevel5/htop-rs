//! Platform-specific system information
//!
//! This module provides platform-specific implementations for reading
//! process, CPU, and memory information from the operating system.

use anyhow::Result;

use crate::core::Machine;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
mod darwin;

/// Initialize platform-specific resources
pub fn init() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::init()
    }
    #[cfg(target_os = "macos")]
    {
        darwin::init()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Ok(())
    }
}

/// Cleanup platform-specific resources
pub fn done() {
    #[cfg(target_os = "linux")]
    {
        linux::done();
    }
    #[cfg(target_os = "macos")]
    {
        darwin::done();
    }
}

/// Get UID for a username
pub fn get_uid_for_username(name: &str) -> Option<u32> {
    #[cfg(unix)]
    {
        users::get_user_by_name(name).map(|u| u.uid())
    }
    #[cfg(not(unix))]
    {
        let _ = name;
        None
    }
}

/// Scan all processes and update the machine state
pub fn scan_processes(machine: &mut Machine) {
    #[cfg(target_os = "linux")]
    {
        linux::scan_processes(machine);
    }
    #[cfg(target_os = "macos")]
    {
        darwin::scan_processes_with_settings(machine, machine.update_process_names);
    }
}

/// Scan memory statistics
pub fn scan_memory(machine: &mut Machine) {
    #[cfg(target_os = "linux")]
    {
        linux::scan_memory(machine);
    }
    #[cfg(target_os = "macos")]
    {
        darwin::scan_memory(machine);
    }
}

/// Scan CPU statistics
pub fn scan_cpu(machine: &mut Machine) {
    #[cfg(target_os = "linux")]
    {
        linux::scan_cpu(machine);
    }
    #[cfg(target_os = "macos")]
    {
        darwin::scan_cpu(machine);
    }
}

/// Scan CPU frequency (conditionally based on settings)
pub fn scan_cpu_frequency(machine: &mut Machine) {
    if !machine.show_cpu_frequency {
        return;
    }
    #[cfg(target_os = "linux")]
    {
        linux::scan_cpu_frequency(machine);
    }
    #[cfg(target_os = "macos")]
    {
        // macOS doesn't provide per-CPU frequency info
        // Frequency is set to 0.0 (N/A) in darwin.rs
    }
}

/// Scan disk IO statistics
pub fn scan_disk_io(machine: &mut Machine) {
    #[cfg(target_os = "linux")]
    {
        linux::scan_disk_io(machine);
    }
    #[cfg(target_os = "macos")]
    {
        // TODO: macOS disk IO statistics
        let _ = machine;
    }
}

/// Scan network IO statistics
pub fn scan_network_io(machine: &mut Machine) {
    #[cfg(target_os = "linux")]
    {
        linux::scan_network_io(machine);
    }
    #[cfg(target_os = "macos")]
    {
        // TODO: macOS network IO statistics
        let _ = machine;
    }
}

/// Get system information (hostname, kernel version, etc.)
pub fn get_system_info(machine: &mut Machine) {
    #[cfg(target_os = "linux")]
    {
        linux::get_system_info(machine);
    }
    #[cfg(target_os = "macos")]
    {
        darwin::get_system_info(machine);
    }
}

/// Perform a full system scan
pub fn scan(machine: &mut Machine) {
    scan_cpu(machine);
    scan_cpu_frequency(machine);
    scan_memory(machine);
    scan_disk_io(machine);
    scan_network_io(machine);
    scan_processes(machine);
    get_system_info(machine);
    machine.update_processes();
}
