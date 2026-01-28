//! Linux-specific background scanner for expensive /proc reads
//!
//! This module provides Linux-specific expensive process data collection
//! using the generic BackgroundScanner framework and rayon's thread pool.
//! Results are collected in the background and merged on the next scan cycle.

use rayon::prelude::*;
use std::collections::HashMap;

use super::bg_scanner::BackgroundScanner;
use super::linux::{check_deleted_libs, filter_cgroup_name, filter_container};
use crate::core::ScanFlags;

/// Data collected from expensive /proc reads for a single process
#[derive(Default, Clone, Debug)]
pub struct LinuxExpensiveData {
    /// Memory share (from statm)
    pub m_share: Option<i64>,
    /// Memory text (from statm)
    pub m_text: Option<i64>,
    /// Memory data (from statm)
    pub m_data: Option<i64>,
    /// CGroup path
    pub cgroup: Option<String>,
    /// Compressed cgroup name
    pub cgroup_short: Option<String>,
    /// Container short name
    pub container_short: Option<String>,
    /// OOM score
    pub oom_score: Option<i32>,
    /// PSS memory (from smaps_rollup)
    pub m_pss: Option<i64>,
    /// Swap memory (from smaps_rollup)
    pub m_swap: Option<i64>,
    /// SwapPss memory (from smaps_rollup)
    pub m_psswp: Option<i64>,
    /// Autogroup ID
    pub autogroup_id: Option<i64>,
    /// Autogroup nice value
    pub autogroup_nice: Option<i32>,
    /// Security attribute
    pub secattr: Option<String>,
    /// Uses deleted library
    pub uses_deleted_lib: Option<bool>,
}

impl LinuxExpensiveData {
    /// Check if any data was collected
    pub fn has_data(&self) -> bool {
        self.m_share.is_some()
            || self.cgroup.is_some()
            || self.oom_score.is_some()
            || self.m_pss.is_some()
            || self.autogroup_id.is_some()
            || self.secattr.is_some()
            || self.uses_deleted_lib.is_some()
    }
}

/// Type alias for the Linux background scanner
pub type LinuxBackgroundScanner = BackgroundScanner<LinuxExpensiveData>;

/// Result from background scanner - maps PID to collected data
pub type LinuxExpensiveDataMap = HashMap<i32, LinuxExpensiveData>;

/// Parameters for background data collection
#[derive(Clone)]
pub struct LinuxScanParams {
    pub pids: Vec<i32>,
    pub flags: ScanFlags,
    pub check_deleted_libs: bool,
    pub page_size: i64,
}

/// Start a background scan for the given PIDs using Linux-specific collection
pub fn start_linux_bg_scan(scanner: &mut LinuxBackgroundScanner, params: LinuxScanParams) {
    let LinuxScanParams {
        pids,
        flags,
        check_deleted_libs,
        page_size,
    } = params;

    scanner.start_scan(move || collect_expensive_data(&pids, flags, check_deleted_libs, page_size));
}

/// Collect expensive data for all PIDs using rayon parallel iterator
fn collect_expensive_data(
    pids: &[i32],
    flags: ScanFlags,
    should_check_deleted_libs: bool,
    page_size: i64,
) -> LinuxExpensiveDataMap {
    pids.par_iter()
        .filter_map(|&pid| {
            let data = collect_for_pid(pid, flags, should_check_deleted_libs, page_size);
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
fn collect_for_pid(
    pid: i32,
    flags: ScanFlags,
    should_check_deleted_libs: bool,
    page_size: i64,
) -> LinuxExpensiveData {
    let mut data = LinuxExpensiveData::default();

    let proc = match procfs::process::Process::new(pid) {
        Ok(p) => p,
        Err(_) => return data,
    };

    // STATM - memory details
    if flags.contains(ScanFlags::STATM) {
        if let Ok(statm) = proc.statm() {
            data.m_share = Some((statm.shared * page_size as u64) as i64);
            data.m_text = Some((statm.text * page_size as u64) as i64);
            data.m_data = Some((statm.data * page_size as u64) as i64);
        }
    }

    // CGROUP
    if flags.contains(ScanFlags::CGROUP) {
        if let Ok(cgroups) = proc.cgroups() {
            if let Some(cgroup) = cgroups.0.first() {
                let cgroup_path = cgroup.pathname.clone();
                data.cgroup_short = Some(filter_cgroup_name(&cgroup_path));
                data.container_short = filter_container(&cgroup_path);
                data.cgroup = Some(cgroup_path);
            }
        }
    }

    // OOM score
    if flags.contains(ScanFlags::OOM) {
        if let Ok(oom) = std::fs::read_to_string(format!("/proc/{}/oom_score", pid)) {
            if let Ok(score) = oom.trim().parse::<i32>() {
                data.oom_score = Some(score);
            }
        }
    }

    // SMAPS (PSS, Swap, SwapPss)
    if flags.contains(ScanFlags::SMAPS) {
        if let Ok(content) = std::fs::read_to_string(format!("/proc/{}/smaps_rollup", pid)) {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("Pss:") {
                    if let Some(kb) = parse_kb_value(rest) {
                        data.m_pss = Some(kb * 1024);
                    }
                } else if let Some(rest) = line.strip_prefix("Swap:") {
                    if let Some(kb) = parse_kb_value(rest) {
                        data.m_swap = Some(kb * 1024);
                    }
                } else if let Some(rest) = line.strip_prefix("SwapPss:") {
                    if let Some(kb) = parse_kb_value(rest) {
                        data.m_psswp = Some(kb * 1024);
                    }
                }
            }
        }
    }

    // Autogroup
    if flags.contains(ScanFlags::AUTOGROUP) {
        if let Ok(content) = std::fs::read_to_string(format!("/proc/{}/autogroup", pid)) {
            // Format: "/autogroup-ID nice NICE"
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                // Parse ID from "/autogroup-123"
                if let Some(id_str) = parts[0].strip_prefix("/autogroup-") {
                    if let Ok(id) = id_str.parse::<i64>() {
                        data.autogroup_id = Some(id);
                    }
                }
                // Parse nice value
                if let Ok(nice) = parts[2].parse::<i32>() {
                    data.autogroup_nice = Some(nice);
                }
            }
        }
    }

    // Security attribute
    if flags.contains(ScanFlags::SEC_ATTR) {
        if let Ok(attr) = std::fs::read_to_string(format!("/proc/{}/attr/current", pid)) {
            let attr = attr.trim().trim_end_matches('\0').to_string();
            if !attr.is_empty() {
                data.secattr = Some(attr);
            }
        }
    }

    // Deleted library check (expensive - reads /proc/PID/maps)
    if should_check_deleted_libs {
        data.uses_deleted_lib = Some(check_deleted_libs(pid));
    }

    data
}

/// Parse a value like "  12345 kB" -> Some(12345)
fn parse_kb_value(s: &str) -> Option<i64> {
    s.trim()
        .strip_suffix("kB")
        .or_else(|| s.trim().strip_suffix(" kB"))
        .and_then(|v| v.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kb_value() {
        assert_eq!(parse_kb_value("  12345 kB"), Some(12345));
        assert_eq!(parse_kb_value("0 kB"), Some(0));
        assert_eq!(parse_kb_value("invalid"), None);
    }
}
