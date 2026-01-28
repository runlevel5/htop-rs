//! Cross-platform background scanner for expensive process data
//!
//! This module provides a generic background scanner framework that each platform
//! can use to parallelize expensive per-process data collection. The scanner runs
//! in a background thread and results are merged on the next scan cycle.
//!
//! # Architecture
//!
//! ```text
//! Main Thread                    Background Thread (rayon pool)
//! ───────────                    ─────────────────────────────
//! scan_processes()
//!   │
//!   ├─► merge background results (if ready)
//!   │   └─► apply to existing processes
//!   │
//!   ├─► quick synchronous scan
//!   │   └─► /proc/PID/stat, status, etc. (cheap reads)
//!   │
//!   └─► start_scan(pids) ────────► spawn thread
//!                                    │
//!                                    └─► rayon parallel collect
//!                                        ├─► PID 1: expensive data
//!                                        ├─► PID 2: expensive data
//!                                        └─► ... (parallel)
//! ```
//!
//! # Usage
//!
//! Each platform implements:
//! 1. A data struct (e.g., `LinuxExpensiveData`, `DarwinExpensiveData`)
//! 2. A collection function that uses rayon to parallelize reads
//! 3. A merge function to apply results to processes

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Generic background scanner that collects data of type T for each PID
#[derive(Debug)]
pub struct BackgroundScanner<T: Send + Clone + 'static> {
    /// Handle to the background thread (if running)
    handle: Option<JoinHandle<HashMap<i32, T>>>,
    /// Results from last completed scan (shared for checking without joining)
    results: Arc<Mutex<Option<HashMap<i32, T>>>>,
}

impl<T: Send + Clone + 'static> BackgroundScanner<T> {
    /// Create a new background scanner
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

    /// Start a background scan using the provided collection function
    ///
    /// The `collect_fn` should use rayon to parallelize data collection.
    /// Only starts if no scan is currently running.
    ///
    /// # Arguments
    /// * `collect_fn` - A function that collects expensive data for all PIDs
    pub fn start_scan<F>(&mut self, collect_fn: F)
    where
        F: FnOnce() -> HashMap<i32, T> + Send + 'static,
    {
        // Don't start if already running
        if self.is_running() {
            return;
        }

        // Take any completed results first
        self.try_take_results();

        let results = Arc::clone(&self.results);

        self.handle = Some(thread::spawn(move || {
            let data = collect_fn();

            // Store results for later retrieval
            if let Ok(mut guard) = results.lock() {
                *guard = Some(data.clone());
            }

            data
        }));
    }

    /// Try to take completed results (non-blocking)
    /// Returns None if no results ready or scan still running
    pub fn try_take_results(&mut self) -> Option<HashMap<i32, T>> {
        // First check if thread has finished
        if let Some(handle) = self.handle.take() {
            if handle.is_finished() {
                // Thread finished, get results directly from join
                // Also clear the shared results since we're returning from join
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

impl<T: Send + Clone + 'static> Default for BackgroundScanner<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_scanner_lifecycle() {
        let mut scanner: BackgroundScanner<i32> = BackgroundScanner::new();

        // Initially not running
        assert!(!scanner.is_running());
        assert!(scanner.try_take_results().is_none());

        // Start a scan
        scanner.start_scan(|| {
            let mut map = HashMap::new();
            map.insert(1, 100);
            map.insert(2, 200);
            map
        });

        // Wait for completion
        std::thread::sleep(Duration::from_millis(10));

        // Should have results
        let results = scanner.try_take_results();
        assert!(results.is_some());
        let results = results.unwrap();
        assert_eq!(results.get(&1), Some(&100));
        assert_eq!(results.get(&2), Some(&200));

        // Results consumed
        assert!(scanner.try_take_results().is_none());
    }

    #[test]
    fn test_scanner_no_double_start() {
        let mut scanner: BackgroundScanner<i32> = BackgroundScanner::new();

        // Start a slow scan
        scanner.start_scan(|| {
            std::thread::sleep(Duration::from_millis(100));
            let mut map = HashMap::new();
            map.insert(1, 1);
            map
        });

        // Try to start another (should be ignored)
        scanner.start_scan(|| {
            let mut map = HashMap::new();
            map.insert(2, 2);
            map
        });

        // Wait for completion
        std::thread::sleep(Duration::from_millis(150));

        // Should only have results from first scan
        let results = scanner.try_take_results().unwrap();
        assert!(results.contains_key(&1));
        assert!(!results.contains_key(&2));
    }
}
