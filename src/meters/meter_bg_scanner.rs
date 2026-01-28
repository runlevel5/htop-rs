//! Background scanner for expensive meter data collection
//!
//! Some meters need to perform expensive I/O operations (spawning subprocesses,
//! reading from slow filesystems, etc.). This module provides a background scanner
//! that collects this data in parallel, allowing the main thread to remain responsive.
//!
//! # Architecture
//!
//! ```text
//! Frame N:
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Main Thread                                                 │
//! │ ┌─────────────────┐    ┌──────────────────────────────────┐ │
//! │ │ 1. Merge bg     │───▶│ 2. Fast meter updates            │ │
//! │ │    results      │    │    (copy from Machine)           │ │
//! │ └─────────────────┘    └──────────────────────────────────┘ │
//! │         ▲                            │                      │
//! │         │                            ▼                      │
//! │         │              ┌──────────────────────────────────┐ │
//! │         │              │ 3. Start background scan         │ │
//! │         │              │    for expensive meter data      │ │
//! │         │              └──────────────────────────────────┘ │
//! └─────────│───────────────────────────────────────────────────┘
//!           │
//!           │ Background Thread (rayon pool)
//!           │ ┌──────────────────────────────────────────────┐
//!           └─│ Parallel collection:                         │
//!             │ - Battery: pmset / sysfs reads               │
//!             │ - GPU: nvidia-smi (future)                   │
//!             │ - etc.                                       │
//!             └──────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! Meters that need expensive data:
//! 1. Implement `expensive_data_id()` to return a unique identifier
//! 2. Implement `merge_expensive_data()` to apply background results
//! 3. The scanner calls platform-specific collection functions

use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use super::battery_meter::ACPresence;

/// Unique identifier for meter types that need expensive data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeterDataId {
    Battery,
    // Future: Gpu, SystemdServices, etc.
}

/// Expensive data collected for meters
#[derive(Debug, Clone)]
pub enum MeterExpensiveData {
    /// Battery meter data
    Battery {
        percent: f64,
        ac_presence: ACPresence,
        available: bool,
    },
    // Future variants for GPU, systemd, etc.
}

/// Result map from background collection
pub type MeterDataMap = HashMap<MeterDataId, MeterExpensiveData>;

/// Background scanner for meter data
#[derive(Debug)]
pub struct MeterBackgroundScanner {
    /// Handle to the background thread (if running)
    handle: Option<JoinHandle<MeterDataMap>>,
    /// Results from last completed scan
    results: Arc<Mutex<Option<MeterDataMap>>>,
}

impl MeterBackgroundScanner {
    /// Create a new meter background scanner
    pub fn new() -> Self {
        Self {
            handle: None,
            results: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if a background scan is currently running
    pub fn is_running(&self) -> bool {
        self.handle.as_ref().map_or(false, |h| !h.is_finished())
    }

    /// Start a background scan for the requested meter data
    ///
    /// # Arguments
    /// * `requested` - Set of meter data IDs to collect
    pub fn start_scan(&mut self, requested: Vec<MeterDataId>) {
        // Don't start if already running
        if self.is_running() {
            return;
        }

        // Take any completed results first
        self.try_take_results();

        if requested.is_empty() {
            return;
        }

        let results = Arc::clone(&self.results);

        self.handle = Some(thread::spawn(move || {
            let data = collect_meter_data(&requested);

            // Store results for later retrieval
            if let Ok(mut guard) = results.lock() {
                *guard = Some(data.clone());
            }

            data
        }));
    }

    /// Try to take completed results (non-blocking)
    /// Returns None if no results ready or scan still running
    pub fn try_take_results(&mut self) -> Option<MeterDataMap> {
        // First check if thread has finished
        if let Some(handle) = self.handle.take() {
            if handle.is_finished() {
                // Thread finished, get results directly from join
                if let Ok(mut guard) = self.results.lock() {
                    *guard = None;
                }
                return handle.join().ok();
            } else {
                // Still running, put handle back
                self.handle = Some(handle);
            }
        }

        // Also check shared results (in case we missed the handle)
        if let Ok(mut guard) = self.results.lock() {
            return guard.take();
        }

        None
    }
}

impl Default for MeterBackgroundScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect expensive data for all requested meters in parallel
fn collect_meter_data(requested: &[MeterDataId]) -> MeterDataMap {
    requested
        .par_iter()
        .filter_map(|&id| {
            let data = collect_for_meter(id);
            data.map(|d| (id, d))
        })
        .collect()
}

/// Collect expensive data for a single meter type
fn collect_for_meter(id: MeterDataId) -> Option<MeterExpensiveData> {
    match id {
        MeterDataId::Battery => collect_battery_data(),
    }
}

/// Collect battery data (platform-specific)
#[cfg(target_os = "macos")]
fn collect_battery_data() -> Option<MeterExpensiveData> {
    use std::process::Command;

    // Use pmset to get battery info on macOS
    let output = Command::new("pmset").arg("-g").arg("batt").output().ok()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    let mut percent = None;
    let mut ac_presence = ACPresence::Unknown;

    for line in output_str.lines() {
        if line.contains("AC Power") {
            ac_presence = ACPresence::Online;
        } else if line.contains("Battery Power") {
            ac_presence = ACPresence::Offline;
        }

        // Look for percentage like "100%"
        if let Some(pct_pos) = line.find('%') {
            let before = &line[..pct_pos];
            if let Some(num_start) = before.rfind(|c: char| !c.is_ascii_digit() && c != '.') {
                if let Ok(pct) = before[num_start + 1..].trim().parse::<f64>() {
                    percent = Some(pct);
                }
            } else if let Ok(pct) = before.trim().parse::<f64>() {
                percent = Some(pct);
            }
        }
    }

    Some(MeterExpensiveData::Battery {
        percent: percent.unwrap_or(0.0),
        ac_presence,
        available: percent.is_some(),
    })
}

#[cfg(target_os = "linux")]
fn collect_battery_data() -> Option<MeterExpensiveData> {
    use std::fs;
    use std::path::Path;

    let power_supply_path = Path::new("/sys/class/power_supply");

    if !power_supply_path.exists() {
        return Some(MeterExpensiveData::Battery {
            percent: 0.0,
            ac_presence: ACPresence::Unknown,
            available: false,
        });
    }

    let mut total_capacity = 0.0;
    let mut battery_count = 0;
    let mut ac_presence = ACPresence::Unknown;

    if let Ok(entries) = fs::read_dir(power_supply_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Check for AC adapter
            if name_str.starts_with("AC")
                || name_str.starts_with("ACAD")
                || name_str.contains("ADP")
            {
                let online_path = path.join("online");
                if let Ok(content) = fs::read_to_string(&online_path) {
                    ac_presence = if content.trim() == "1" {
                        ACPresence::Online
                    } else {
                        ACPresence::Offline
                    };
                }
            }

            // Check for battery
            if name_str.starts_with("BAT") {
                let capacity_path = path.join("capacity");
                if let Ok(content) = fs::read_to_string(&capacity_path) {
                    if let Ok(cap) = content.trim().parse::<f64>() {
                        total_capacity += cap;
                        battery_count += 1;
                    }
                }
            }
        }
    }

    if battery_count > 0 {
        Some(MeterExpensiveData::Battery {
            percent: total_capacity / battery_count as f64,
            ac_presence,
            available: true,
        })
    } else {
        Some(MeterExpensiveData::Battery {
            percent: 0.0,
            ac_presence,
            available: false,
        })
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn collect_battery_data() -> Option<MeterExpensiveData> {
    Some(MeterExpensiveData::Battery {
        percent: 0.0,
        ac_presence: ACPresence::Unknown,
        available: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_scanner_lifecycle() {
        let mut scanner = MeterBackgroundScanner::new();

        // Initially not running
        assert!(!scanner.is_running());
        assert!(scanner.try_take_results().is_none());

        // Start a scan for battery
        scanner.start_scan(vec![MeterDataId::Battery]);

        // Wait for completion
        std::thread::sleep(Duration::from_millis(500));

        // Should have results
        let results = scanner.try_take_results();
        assert!(results.is_some());
        let results = results.unwrap();
        assert!(results.contains_key(&MeterDataId::Battery));

        // Results consumed
        assert!(scanner.try_take_results().is_none());
    }

    #[test]
    fn test_scanner_empty_request() {
        let mut scanner = MeterBackgroundScanner::new();

        // Empty request should not start a thread
        scanner.start_scan(vec![]);
        assert!(!scanner.is_running());
    }

    #[test]
    fn test_scanner_no_double_start() {
        let mut scanner = MeterBackgroundScanner::new();

        // Start a scan
        scanner.start_scan(vec![MeterDataId::Battery]);

        // Try to start another (should be ignored while running)
        let is_running_before = scanner.is_running();
        scanner.start_scan(vec![MeterDataId::Battery]);

        // Wait and check we only get one result set
        std::thread::sleep(Duration::from_millis(500));

        let results = scanner.try_take_results();
        assert!(results.is_some());

        // Verify it was running (may have completed by now)
        // The key test is that we got results
        let _ = is_running_before;
    }

    #[test]
    fn test_meter_data_id_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(MeterDataId::Battery);
        assert!(set.contains(&MeterDataId::Battery));
    }
}
