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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Runs one collection at a time on the rayon pool and hands the result back
/// to the main thread on a later tick.
#[derive(Debug)]
pub struct BackgroundScanner<T: Send + 'static> {
    /// True while a collection is in flight
    running: Arc<AtomicBool>,
    /// Completed results waiting to be taken by the main thread
    results: Arc<Mutex<Option<HashMap<i32, T>>>>,
}

impl<T: Send + 'static> BackgroundScanner<T> {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            results: Arc::new(Mutex::new(None)),
        }
    }

    /// Whether a scan is currently in progress
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Start a scan on the rayon pool unless one is already running.
    ///
    /// Any results that were completed but never taken are discarded first,
    /// so callers should take results before starting the next scan.
    /// The `collect_fn` may itself use rayon's parallel iterators.
    pub fn start_scan<F>(&mut self, collect_fn: F)
    where
        F: FnOnce() -> HashMap<i32, T> + Send + 'static,
    {
        if self.is_running() {
            return;
        }

        self.try_take_results();

        self.running.store(true, Ordering::Release);
        let running = Arc::clone(&self.running);
        let results = Arc::clone(&self.results);

        rayon::spawn(move || {
            let data = collect_fn();
            if let Ok(mut guard) = results.lock() {
                *guard = Some(data);
            }
            running.store(false, Ordering::Release);
        });
    }

    /// Take the results of the last completed scan, if any
    pub fn try_take_results(&mut self) -> Option<HashMap<i32, T>> {
        self.results.lock().ok()?.take()
    }
}

impl<T: Send + 'static> Default for BackgroundScanner<T> {
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

        // Wait for completion with polling instead of fixed sleep
        let mut results = None;
        for _ in 0..50 {
            std::thread::sleep(Duration::from_millis(10));
            if let Some(r) = scanner.try_take_results() {
                results = Some(r);
                break;
            }
        }

        // Should only have results from first scan
        let results = results.expect("scan should have completed");
        assert!(results.contains_key(&1));
        assert!(!results.contains_key(&2));
    }
}
