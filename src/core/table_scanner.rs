//! Runs the process table scan on its own thread so the UI never blocks on it.
//!
//! The scanner thread owns a private [`Machine`] that holds the scanning state
//! (the authoritative process list with its per-process history, the username
//! cache, the background scanner, the scan clock). Each refresh the UI sends a
//! [`ScanRequest`] with the settings and system values the scan needs; the
//! thread scans into its own list, runs the cleanup and command-string
//! bookkeeping, and sends back a [`TableSnapshot`] containing a copy of the
//! list. The UI swaps that copy in and re-applies the per-process state that
//! only it changes, such as tags and collapsed tree nodes.
//!
//! Keeping the scan state on the worker means `prev_time`, IO rates and the
//! new/tomb highlight stamps keep working exactly as in the synchronous scan.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use crate::core::ScanFlags;
use crate::core::{CommandStrParams, FieldWidths, Machine, Memory, ProcessList};
use crate::platform;

/// Everything one table scan needs from the UI side
#[derive(Debug, Clone)]
pub struct ScanRequest {
    #[cfg(target_os = "linux")]
    pub scan_flags: ScanFlags,
    pub update_process_names: bool,
    pub hide_userland_threads: bool,
    pub check_deleted_libs: bool,
    /// System values from the most recent `scan_system`
    pub total_mem: Memory,
    pub boot_time: i64,
    pub active_cpus: u32,
    pub running_tasks: u32,
    /// Inputs for building merged command strings
    pub cmd_params: CommandStrParams,
    pub tree_separator: String,
    pub highlight_changes: bool,
    pub highlight_delay_secs: i32,
    /// Processes the UI has drawn, so vanished ones can be tombed rather than dropped
    pub shown_pids: Vec<i32>,
}

/// The result of one table scan
#[derive(Debug)]
pub struct TableSnapshot {
    pub processes: ProcessList,
    pub field_widths: FieldWidths,
    pub total_tasks: u32,
    pub running_tasks: u32,
    pub userland_threads: u32,
    pub kernel_threads: u32,
    pub max_pid: i32,
    pub max_user_id: u32,
    pub monotonic_ms: u64,
    pub prev_monotonic_ms: u64,
    /// How long the scan took on the worker
    pub scan_time: Duration,
}

/// Handle to the scanner thread
#[derive(Debug)]
pub struct TableScanner {
    requests: Sender<ScanRequest>,
    results: Receiver<TableSnapshot>,
    in_flight: bool,
    worker: Option<JoinHandle<()>>,
}

impl TableScanner {
    /// Start the scanner thread
    pub fn spawn() -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("table-scan".to_string())
            .spawn(move || worker_loop(request_rx, result_tx))
            .ok();
        Self {
            requests: request_tx,
            results: result_rx,
            in_flight: false,
            worker,
        }
    }

    /// Whether a scan is in progress
    pub fn in_flight(&self) -> bool {
        self.in_flight
    }

    /// Start a scan. Returns false, without queueing, if one is already running.
    pub fn request(&mut self, request: ScanRequest) -> bool {
        if self.in_flight || self.worker.is_none() {
            return false;
        }
        if self.requests.send(request).is_err() {
            return false;
        }
        self.in_flight = true;
        true
    }

    /// Take the finished snapshot if there is one, without waiting
    pub fn try_take(&mut self) -> Option<TableSnapshot> {
        if !self.in_flight {
            return None;
        }
        match self.results.try_recv() {
            Ok(snapshot) => {
                self.in_flight = false;
                Some(snapshot)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.in_flight = false;
                None
            }
        }
    }

    /// Wait for the in-flight scan to finish
    pub fn wait(&mut self) -> Option<TableSnapshot> {
        if !self.in_flight {
            return None;
        }
        self.in_flight = false;
        self.results.recv().ok()
    }
}

/// The scanner thread: scan on request, reply with a snapshot
fn worker_loop(requests: Receiver<ScanRequest>, results: Sender<TableSnapshot>) {
    let mut machine = Machine::new(None);

    for request in requests {
        let start = Instant::now();

        #[cfg(target_os = "linux")]
        {
            machine.scan_flags = request.scan_flags;
        }
        machine.update_process_names = request.update_process_names;
        machine.hide_userland_threads = request.hide_userland_threads;
        machine.check_deleted_libs = request.check_deleted_libs;
        machine.total_mem = request.total_mem;
        machine.boot_time = request.boot_time;
        machine.active_cpus = request.active_cpus;
        machine.running_tasks = request.running_tasks;

        // The UI knows which processes were on screen; the cleanup below uses
        // that to tomb vanished ones instead of dropping them silently
        for pid in &request.shown_pids {
            if let Some(process) = machine.processes.get_mut(*pid) {
                process.was_shown = true;
            }
        }

        platform::scan_tables(&mut machine);

        // Sorting is the UI's decision; it depends on the sort timeout
        machine.needs_sort = false;
        machine.update_processes(
            Some(&request.cmd_params),
            &request.tree_separator,
            request.highlight_changes,
            request.highlight_delay_secs,
        );

        let snapshot = TableSnapshot {
            processes: machine.processes.clone(),
            field_widths: machine.field_widths.clone(),
            total_tasks: machine.total_tasks,
            running_tasks: machine.running_tasks,
            userland_threads: machine.userland_threads,
            kernel_threads: machine.kernel_threads,
            max_pid: machine.max_pid,
            max_user_id: machine.max_user_id,
            monotonic_ms: machine.monotonic_ms,
            prev_monotonic_ms: machine.prev_monotonic_ms,
            scan_time: start.elapsed(),
        };
        if results.send(snapshot).is_err() {
            break;
        }
    }
}
