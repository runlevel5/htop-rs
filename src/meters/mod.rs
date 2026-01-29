//! Meters module
//!
//! This module contains the meter system for displaying system statistics
//! at the top of the screen.

#![allow(dead_code)]

mod battery_meter;
mod blank_meter;
mod clock_meter;
mod cpu_meter;
mod date_meter;
mod datetime_meter;
mod diskio_meter;
mod diskio_rate_meter;
mod diskio_time_meter;
mod filedescriptors_meter;
mod gpu_meter;
mod hostname_meter;
mod hugepages_meter;
mod load_meter;
mod memory_meter;
mod memoryswap_meter;
pub mod meter_bg_scanner;
mod networkio_meter;
mod pressure_stall_meter;
mod selinux_meter;
mod swap_meter;
mod system_meter;
mod systemd_meter;
mod tasks_meter;
mod uptime_meter;
mod zfs_arc_meter;
mod zram_meter;

use std::time::Instant;

use crate::core::{Machine, Settings};
use crate::ui::Crt;

pub use battery_meter::*;
pub use blank_meter::*;
pub use clock_meter::*;
pub use cpu_meter::*;
pub use date_meter::*;
pub use datetime_meter::*;
pub use diskio_meter::*;
pub use diskio_rate_meter::*;
pub use diskio_time_meter::*;
pub use filedescriptors_meter::*;
pub use gpu_meter::*;
pub use hostname_meter::*;
pub use hugepages_meter::*;
pub use load_meter::*;
pub use memory_meter::*;
pub use memoryswap_meter::*;
pub use networkio_meter::*;
pub use pressure_stall_meter::*;
pub use selinux_meter::*;
pub use swap_meter::*;
pub use system_meter::*;
pub use systemd_meter::*;
pub use tasks_meter::*;
pub use uptime_meter::*;
pub use zfs_arc_meter::*;
pub use zram_meter::*;

/// Default graph height in rows (matches C htop DEFAULT_GRAPH_HEIGHT)
pub const DEFAULT_GRAPH_HEIGHT: i32 = 4;

/// Maximum number of graph data values to store
pub const MAX_GRAPH_DATA_VALUES: usize = 32768;

/// Graph data storage for historical meter values
#[derive(Debug, Clone)]
pub struct GraphData {
    /// Time of last update
    pub last_update: Option<Instant>,
    /// Historical values (0.0 to 1.0 normalized)
    pub values: Vec<f64>,
}

impl Default for GraphData {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphData {
    pub fn new() -> Self {
        GraphData {
            last_update: None,
            values: Vec::new(),
        }
    }

    /// Record a new value (0.0 to 1.0 normalized)
    /// Returns true if value was recorded, false if too soon since last update
    pub fn record(&mut self, value: f64, delay_ms: u32) -> bool {
        let now = Instant::now();
        let should_record = match self.last_update {
            Some(last) => now.duration_since(last).as_millis() >= delay_ms as u128,
            None => true,
        };

        if should_record {
            self.last_update = Some(now);

            // Shift values left and add new value
            if self.values.len() >= MAX_GRAPH_DATA_VALUES {
                self.values.remove(0);
            }
            self.values.push(value.clamp(0.0, 1.0));
            true
        } else {
            false
        }
    }

    /// Record a raw value (for self-scaling meters like Tasks)
    /// Values are stored as-is without normalization
    /// Returns true if value was recorded, false if too soon since last update
    pub fn record_raw(&mut self, value: f64, delay_ms: u32) -> bool {
        let now = Instant::now();
        let should_record = match self.last_update {
            Some(last) => now.duration_since(last).as_millis() >= delay_ms as u128,
            None => true,
        };

        if should_record {
            self.last_update = Some(now);

            // Shift values left and add new value
            if self.values.len() >= MAX_GRAPH_DATA_VALUES {
                self.values.remove(0);
            }
            self.values.push(value.max(0.0));
            true
        } else {
            false
        }
    }

    /// Get the maximum value in the graph data (for self-scaling)
    /// Returns at least 1.0 to avoid division by zero
    pub fn max_value(&self) -> f64 {
        self.values
            .iter()
            .copied()
            .fold(1.0_f64, |acc, v| acc.max(v))
    }

    /// Ensure the buffer has at least the given capacity
    pub fn ensure_capacity(&mut self, width: usize) {
        // We need 2 values per column for the graph
        let needed = width * 2;
        if self.values.len() < needed {
            // Prepend zeros to fill
            let mut new_values = vec![0.0; needed - self.values.len()];
            new_values.append(&mut self.values);
            self.values = new_values;
        }
    }
}

/// Number of segments in a stacked graph for Memory meter (used, shared, compressed, buffers, cache)
pub const STACKED_GRAPH_SEGMENTS: usize = 5;

/// Number of segments in a stacked graph for CPU meter (user, nice, system, irq, softirq, steal, guest, iowait)
pub const CPU_STACKED_GRAPH_SEGMENTS: usize = 8;

/// Number of segments in a stacked graph for Swap meter (used, cache)
pub const SWAP_STACKED_GRAPH_SEGMENTS: usize = 2;

/// Stacked graph data storage for multi-value historical meter values
/// Each time sample stores multiple values that stack on top of each other
#[derive(Debug, Clone)]
pub struct StackedGraphData {
    /// Time of last update
    pub last_update: Option<Instant>,
    /// Historical values - each entry is an array of segment values (0.0 to 1.0 normalized)
    /// Segments are stacked: [0] is at bottom, [N-1] is at top
    pub values: Vec<[f64; STACKED_GRAPH_SEGMENTS]>,
}

impl Default for StackedGraphData {
    fn default() -> Self {
        Self::new()
    }
}

impl StackedGraphData {
    pub fn new() -> Self {
        StackedGraphData {
            last_update: None,
            values: Vec::new(),
        }
    }

    /// Record new segment values (each 0.0 to 1.0 normalized, sum should be <= 1.0)
    /// Returns true if values were recorded, false if too soon since last update
    pub fn record(&mut self, segments: [f64; STACKED_GRAPH_SEGMENTS], delay_ms: u32) -> bool {
        let now = Instant::now();
        let should_record = match self.last_update {
            Some(last) => now.duration_since(last).as_millis() >= delay_ms as u128,
            None => true,
        };

        if should_record {
            self.last_update = Some(now);

            // Shift values left and add new value
            if self.values.len() >= MAX_GRAPH_DATA_VALUES {
                self.values.remove(0);
            }

            // Clamp each segment to 0.0-1.0
            self.values.push(segments.map(|v| v.clamp(0.0, 1.0)));
            true
        } else {
            false
        }
    }
}

/// CPU stacked graph data storage for multi-value historical CPU meter values
/// Each time sample stores 8 values (user, nice, system, irq, softirq, steal, guest, iowait)
#[derive(Debug, Clone)]
pub struct CpuStackedGraphData {
    /// Time of last update
    pub last_update: Option<Instant>,
    /// Historical values - each entry is an array of segment values (0.0 to 1.0 normalized)
    /// Segments are stacked: [0] user, [1] nice, [2] system, [3] irq, [4] softirq, [5] steal, [6] guest, [7] iowait
    pub values: Vec<[f64; CPU_STACKED_GRAPH_SEGMENTS]>,
}

impl Default for CpuStackedGraphData {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuStackedGraphData {
    pub fn new() -> Self {
        CpuStackedGraphData {
            last_update: None,
            values: Vec::new(),
        }
    }

    /// Record new segment values (each as percentage 0-100, will be normalized to 0.0-1.0)
    /// Returns true if values were recorded, false if too soon since last update
    pub fn record(&mut self, segments: [f64; CPU_STACKED_GRAPH_SEGMENTS], delay_ms: u32) -> bool {
        let now = Instant::now();
        let should_record = match self.last_update {
            Some(last) => now.duration_since(last).as_millis() >= delay_ms as u128,
            None => true,
        };

        if should_record {
            self.last_update = Some(now);

            // Shift values left and add new value
            if self.values.len() >= MAX_GRAPH_DATA_VALUES {
                self.values.remove(0);
            }

            // Normalize from percentage (0-100) to 0.0-1.0 and clamp
            self.values
                .push(segments.map(|v| (v / 100.0).clamp(0.0, 1.0)));
            true
        } else {
            false
        }
    }
}

/// Swap stacked graph data storage for multi-value historical Swap meter values
/// Each time sample stores 2 values (used, cache) normalized to 0.0-1.0
#[derive(Debug, Clone)]
pub struct SwapStackedGraphData {
    /// Time of last update
    pub last_update: Option<Instant>,
    /// Historical values - each entry is an array of segment values (0.0 to 1.0 normalized)
    /// Segments are stacked: [0] used, [1] cache
    pub values: Vec<[f64; SWAP_STACKED_GRAPH_SEGMENTS]>,
}

impl Default for SwapStackedGraphData {
    fn default() -> Self {
        Self::new()
    }
}

impl SwapStackedGraphData {
    pub fn new() -> Self {
        SwapStackedGraphData {
            last_update: None,
            values: Vec::new(),
        }
    }

    /// Record new segment values (each as ratio of total, already normalized to 0.0-1.0)
    /// Returns true if values were recorded, false if too soon since last update
    pub fn record(&mut self, segments: [f64; SWAP_STACKED_GRAPH_SEGMENTS], delay_ms: u32) -> bool {
        let now = Instant::now();
        let should_record = match self.last_update {
            Some(last) => now.duration_since(last).as_millis() >= delay_ms as u128,
            None => true,
        };

        if should_record {
            self.last_update = Some(now);

            // Shift values left and add new value
            if self.values.len() >= MAX_GRAPH_DATA_VALUES {
                self.values.remove(0);
            }

            // Clamp each segment to 0.0-1.0
            self.values.push(segments.map(|v| v.clamp(0.0, 1.0)));
            true
        } else {
            false
        }
    }
}

/// Number of segments in a stacked graph for Load meter (load1, load5, load15)
pub const LOAD_STACKED_GRAPH_SEGMENTS: usize = 3;

/// Load stacked graph data storage for multi-value historical Load meter values
/// Each time sample stores 3 values (load1, load5, load15) - self-scaling based on max value
#[derive(Debug, Clone)]
pub struct LoadStackedGraphData {
    /// Time of last update
    pub last_update: Option<Instant>,
    /// Historical values - each entry is an array of raw load values
    /// Segments: [0] load1, [1] load5, [2] load15
    pub values: Vec<[f64; LOAD_STACKED_GRAPH_SEGMENTS]>,
}

impl Default for LoadStackedGraphData {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadStackedGraphData {
    pub fn new() -> Self {
        LoadStackedGraphData {
            last_update: None,
            values: Vec::new(),
        }
    }

    /// Record new segment values (raw load values, will be normalized at draw time)
    /// Returns true if values were recorded, false if too soon since last update
    pub fn record(&mut self, segments: [f64; LOAD_STACKED_GRAPH_SEGMENTS], delay_ms: u32) -> bool {
        let now = Instant::now();
        let should_record = match self.last_update {
            Some(last) => now.duration_since(last).as_millis() >= delay_ms as u128,
            None => true,
        };

        if should_record {
            self.last_update = Some(now);

            // Shift values left and add new value
            if self.values.len() >= MAX_GRAPH_DATA_VALUES {
                self.values.remove(0);
            }

            // Store raw values (will be normalized at draw time for self-scaling)
            self.values.push(segments.map(|v| v.max(0.0)));
            true
        } else {
            false
        }
    }

    /// Get the maximum sum of all segments across all recorded values
    /// Used for self-scaling the graph. Returns at least 1.0 to avoid division by zero.
    pub fn max_sum(&self) -> f64 {
        self.values
            .iter()
            .map(|segs| segs.iter().sum::<f64>())
            .fold(1.0_f64, |acc, sum| acc.max(sum))
    }
}

/// Number of segments in a stacked graph for Tasks meter (kernel, userland, processes, running)
pub const TASKS_STACKED_GRAPH_SEGMENTS: usize = 4;

/// Tasks stacked graph data storage for multi-value historical Tasks meter values
/// Each time sample stores 4 values (kernel threads, userland threads, processes, running)
/// Self-scaling based on max total
#[derive(Debug, Clone)]
pub struct TasksStackedGraphData {
    /// Time of last update
    pub last_update: Option<Instant>,
    /// Historical values - each entry is an array of raw task counts
    /// Segments: [0] kernel threads, [1] userland threads, [2] processes, [3] running
    pub values: Vec<[f64; TASKS_STACKED_GRAPH_SEGMENTS]>,
}

impl Default for TasksStackedGraphData {
    fn default() -> Self {
        Self::new()
    }
}

impl TasksStackedGraphData {
    pub fn new() -> Self {
        TasksStackedGraphData {
            last_update: None,
            values: Vec::new(),
        }
    }

    /// Record new segment values (raw task counts, will be normalized at draw time)
    /// Returns true if values were recorded, false if too soon since last update
    pub fn record(&mut self, segments: [f64; TASKS_STACKED_GRAPH_SEGMENTS], delay_ms: u32) -> bool {
        let now = Instant::now();
        let should_record = match self.last_update {
            Some(last) => now.duration_since(last).as_millis() >= delay_ms as u128,
            None => true,
        };

        if should_record {
            self.last_update = Some(now);

            // Shift values left and add new value
            if self.values.len() >= MAX_GRAPH_DATA_VALUES {
                self.values.remove(0);
            }

            // Store raw values (will be normalized at draw time for self-scaling)
            self.values.push(segments.map(|v| v.max(0.0)));
            true
        } else {
            false
        }
    }

    /// Get the maximum sum of all segments across all recorded values
    /// Used for self-scaling the graph. Returns at least 1.0 to avoid division by zero.
    pub fn max_sum(&self) -> f64 {
        self.values
            .iter()
            .map(|segs| segs.iter().sum::<f64>())
            .fold(1.0_f64, |acc, sum| acc.max(sum))
    }
}

/// Trait for accessing stacked graph data in a generic way
/// This allows the draw function to work with any stacked graph data type
pub trait StackedGraphValues {
    /// Number of segments in each value entry
    fn segment_count(&self) -> usize;

    /// Number of value entries stored
    fn len(&self) -> usize;

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get segment values at a given index, returning a slice if in bounds
    fn get_segments(&self, index: usize) -> Option<&[f64]>;

    /// Get a zero-filled segment array for this type
    fn zero_segments(&self) -> Vec<f64>;
}

impl StackedGraphValues for StackedGraphData {
    fn segment_count(&self) -> usize {
        STACKED_GRAPH_SEGMENTS
    }
    fn len(&self) -> usize {
        self.values.len()
    }
    fn get_segments(&self, index: usize) -> Option<&[f64]> {
        self.values.get(index).map(|v| v.as_slice())
    }
    fn zero_segments(&self) -> Vec<f64> {
        vec![0.0; STACKED_GRAPH_SEGMENTS]
    }
}

impl StackedGraphValues for CpuStackedGraphData {
    fn segment_count(&self) -> usize {
        CPU_STACKED_GRAPH_SEGMENTS
    }
    fn len(&self) -> usize {
        self.values.len()
    }
    fn get_segments(&self, index: usize) -> Option<&[f64]> {
        self.values.get(index).map(|v| v.as_slice())
    }
    fn zero_segments(&self) -> Vec<f64> {
        vec![0.0; CPU_STACKED_GRAPH_SEGMENTS]
    }
}

impl StackedGraphValues for SwapStackedGraphData {
    fn segment_count(&self) -> usize {
        SWAP_STACKED_GRAPH_SEGMENTS
    }
    fn len(&self) -> usize {
        self.values.len()
    }
    fn get_segments(&self, index: usize) -> Option<&[f64]> {
        self.values.get(index).map(|v| v.as_slice())
    }
    fn zero_segments(&self) -> Vec<f64> {
        vec![0.0; SWAP_STACKED_GRAPH_SEGMENTS]
    }
}

impl StackedGraphValues for LoadStackedGraphData {
    fn segment_count(&self) -> usize {
        LOAD_STACKED_GRAPH_SEGMENTS
    }
    fn len(&self) -> usize {
        self.values.len()
    }
    fn get_segments(&self, index: usize) -> Option<&[f64]> {
        self.values.get(index).map(|v| v.as_slice())
    }
    fn zero_segments(&self) -> Vec<f64> {
        vec![0.0; LOAD_STACKED_GRAPH_SEGMENTS]
    }
}

impl StackedGraphValues for TasksStackedGraphData {
    fn segment_count(&self) -> usize {
        TASKS_STACKED_GRAPH_SEGMENTS
    }
    fn len(&self) -> usize {
        self.values.len()
    }
    fn get_segments(&self, index: usize) -> Option<&[f64]> {
        self.values.get(index).map(|v| v.as_slice())
    }
    fn zero_segments(&self) -> Vec<f64> {
        vec![0.0; TASKS_STACKED_GRAPH_SEGMENTS]
    }
}

/// Meter display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeterMode {
    #[default]
    Bar,
    Text,
    Graph,
    Led,
    StackedGraph,
}

impl MeterMode {
    /// Get the default height for this mode
    pub fn default_height(&self) -> i32 {
        match self {
            MeterMode::Bar => 1,
            MeterMode::Text => 1,
            MeterMode::Graph => DEFAULT_GRAPH_HEIGHT,
            MeterMode::Led => 3,
            MeterMode::StackedGraph => DEFAULT_GRAPH_HEIGHT,
        }
    }
}

impl From<crate::core::MeterMode> for MeterMode {
    fn from(mode: crate::core::MeterMode) -> Self {
        match mode {
            crate::core::MeterMode::Bar => MeterMode::Bar,
            crate::core::MeterMode::Text => MeterMode::Text,
            crate::core::MeterMode::Graph => MeterMode::Graph,
            crate::core::MeterMode::Led => MeterMode::Led,
            crate::core::MeterMode::StackedGraph => MeterMode::StackedGraph,
        }
    }
}

/// Meter trait - all meters implement this
///
/// Meters must be `Send` to support parallel updates via rayon.
pub trait Meter: std::fmt::Debug + Send {
    /// Get the meter name
    fn name(&self) -> &'static str;

    /// Get the caption (prefix in the header)
    fn caption(&self) -> &str;

    /// Initialize the meter
    fn init(&mut self) {}

    /// Update meter values from machine state
    /// This should only do fast operations (copying from Machine).
    /// Expensive operations should use the background scanner.
    fn update(&mut self, machine: &Machine);

    /// Get the height of the meter in lines
    fn height(&self) -> i32 {
        self.mode().default_height()
    }

    /// Draw the meter
    fn draw(
        &self,
        crt: &mut Crt,
        machine: &Machine,
        settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    );

    /// Get the display mode
    fn mode(&self) -> MeterMode {
        MeterMode::Bar
    }

    /// Set the display mode
    fn set_mode(&mut self, mode: MeterMode);

    /// Get supported modes for this meter (default: all modes)
    fn supported_modes(&self) -> u32 {
        // Default: all modes supported (Bar, Text, Graph, Led)
        (1 << MeterMode::Bar as u32)
            | (1 << MeterMode::Text as u32)
            | (1 << MeterMode::Graph as u32)
            | (1 << MeterMode::Led as u32)
    }

    /// Get the default mode for this meter (default: Bar)
    fn default_mode(&self) -> MeterMode {
        MeterMode::Bar
    }

    /// Check if a mode is supported
    fn supports_mode(&self, mode: MeterMode) -> bool {
        (self.supported_modes() & (1 << mode as u32)) != 0
    }

    /// Get the expensive data ID for this meter, if any.
    /// Meters that need background data collection should return Some(id).
    /// Default is None (no expensive data needed).
    fn expensive_data_id(&self) -> Option<meter_bg_scanner::MeterDataId> {
        None
    }

    /// Merge expensive data from background scanner.
    /// Called when background results are available.
    /// Default implementation does nothing.
    fn merge_expensive_data(&mut self, _data: &meter_bg_scanner::MeterExpensiveData) {}
}

/// Meter type enum for creating meters by name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterType {
    Cpu,
    AllCpus,
    LeftCpus,
    RightCpus,
    LeftCpus2,
    RightCpus2,
    LeftCpus4,
    RightCpus4,
    LeftCpus8,
    RightCpus8,
    Memory,
    Swap,
    LoadAverage,
    Tasks,
    Uptime,
    Battery,
    Hostname,
    Clock,
    Date,
    DateTime,
    DiskIO,
    NetworkIO,
    Blank,
}

impl MeterType {
    /// Create a meter by name
    pub fn create_from_name(name: &str, param: u32) -> Option<Box<dyn Meter>> {
        match name {
            "CPU" => Some(Box::new(CpuMeter::new(Some(param as usize)))),
            "AllCPUs" => Some(Box::new(CpuMeter::all(1))),
            "AllCPUs2" => Some(Box::new(CpuMeter::all(2))),
            "AllCPUs4" => Some(Box::new(CpuMeter::all(4))),
            "AllCPUs8" => Some(Box::new(CpuMeter::all(8))),
            "LeftCPUs" => Some(Box::new(CpuMeter::left(1))),
            "LeftCPUs2" => Some(Box::new(CpuMeter::left(2))),
            "LeftCPUs4" => Some(Box::new(CpuMeter::left(4))),
            "LeftCPUs8" => Some(Box::new(CpuMeter::left(8))),
            "RightCPUs" => Some(Box::new(CpuMeter::right(1))),
            "RightCPUs2" => Some(Box::new(CpuMeter::right(2))),
            "RightCPUs4" => Some(Box::new(CpuMeter::right(4))),
            "RightCPUs8" => Some(Box::new(CpuMeter::right(8))),
            "Memory" => Some(Box::new(MemoryMeter::new())),
            "Swap" => Some(Box::new(SwapMeter::new())),
            "LoadAverage" => Some(Box::new(LoadAverageMeter::new())),
            "Load" => Some(Box::new(LoadMeter::new())),
            "Tasks" => Some(Box::new(TasksMeter::new())),
            "Uptime" => Some(Box::new(UptimeMeter::new())),
            "Blank" => Some(Box::new(BlankMeter::new())),
            "Hostname" => Some(Box::new(HostnameMeter::new())),
            "Clock" => Some(Box::new(ClockMeter::new())),
            "Date" => Some(Box::new(DateMeter::new())),
            "DateTime" => Some(Box::new(DateTimeMeter::new())),
            "Battery" => Some(Box::new(BatteryMeter::new())),
            "DiskIO" => Some(Box::new(DiskIOMeter::new())),
            "NetworkIO" => Some(Box::new(NetworkIOMeter::new())),
            // Stub meters (not yet implemented)
            "MemorySwap" => Some(Box::new(MemorySwapMeter::new())),
            "System" => Some(Box::new(SystemMeter::new())),
            "DiskIORate" => Some(Box::new(DiskIORateMeter::new())),
            "DiskIOTime" => Some(Box::new(DiskIOTimeMeter::new())),
            "FileDescriptors" => Some(Box::new(FileDescriptorsMeter::new())),
            "GPU" => Some(Box::new(GpuMeter::new())),
            "HugePages" => Some(Box::new(HugePagesMeter::new())),
            "PressureStallCPUSome" => Some(Box::new(PressureStallCPUSomeMeter::new())),
            "PressureStallIOSome" => Some(Box::new(PressureStallIOSomeMeter::new())),
            "PressureStallIOFull" => Some(Box::new(PressureStallIOFullMeter::new())),
            "PressureStallIRQFull" => Some(Box::new(PressureStallIRQFullMeter::new())),
            "PressureStallMemorySome" => Some(Box::new(PressureStallMemorySomeMeter::new())),
            "PressureStallMemoryFull" => Some(Box::new(PressureStallMemoryFullMeter::new())),
            "Zram" => Some(Box::new(ZramMeter::new())),
            "SELinux" => Some(Box::new(SELinuxMeter::new())),
            "Systemd" => Some(Box::new(SystemdMeter::new())),
            "SystemdUser" => Some(Box::new(SystemdUserMeter::new())),
            "ZFSARC" => Some(Box::new(ZfsArcMeter::new())),
            "ZFSCARC" => Some(Box::new(ZfsCompressedArcMeter::new())),
            _ => None,
        }
    }
}

// ============================================================================
// Shared Helper Functions
// ============================================================================

/// Format a memory/size value in human-readable format (K/M/G/T/P)
///
/// Matches C htop's Meter_humanUnit behavior:
/// - Kilobytes: no decimal (e.g., "512K")
/// - Megabytes+: 2 decimals for values < 10, 1 decimal for values < 100, 0 decimals otherwise
///
/// # Arguments
/// * `value_kb` - Value in kilobytes
///
/// # Examples
/// ```ignore
/// assert_eq!(human_unit(512.0), "512K");
/// assert_eq!(human_unit(1024.0), "1.00M");
/// assert_eq!(human_unit(10240.0), "10.0M");
/// assert_eq!(human_unit(102400.0), "100M");
/// ```
pub fn human_unit(value_kb: f64) -> String {
    const UNIT_PREFIXES: [char; 5] = ['K', 'M', 'G', 'T', 'P'];
    let mut val = value_kb;
    let mut i = 0;

    while val >= 1024.0 && i < UNIT_PREFIXES.len() - 1 {
        val /= 1024.0;
        i += 1;
    }

    if i == 0 {
        // Kibibytes - no decimal
        format!("{:.0}{}", val, UNIT_PREFIXES[i])
    } else {
        // Mebibytes and above - show decimals based on size
        let precision = if val <= 9.99 {
            2
        } else if val <= 99.9 {
            1
        } else {
            0
        };
        format!("{:.prec$}{}", val, UNIT_PREFIXES[i], prec = precision)
    }
}

/// A segment for the bar meter with a value and color attribute
pub struct BarSegment {
    /// The value for this segment (will be divided by total to get percentage)
    pub value: f64,
    /// The ncurses color attribute for this segment
    pub attr: u32,
}

/// A text segment with content and color attribute for text mode display
pub struct TextSegment<'a> {
    /// The text content to display
    pub text: &'a str,
    /// The ncurses color attribute for this segment
    pub attr: u32,
}

/// Draw text mode meter output with multiple colored segments
///
/// This is a generic helper for meters that display colored text segments
/// in text mode (e.g., "Mem:8.0G used:4.0G buffers:512M").
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position
/// * `segments` - Text segments to draw in order (text, attr pairs)
pub fn draw_text_segments(crt: &mut Crt, x: i32, y: i32, segments: &[TextSegment]) {
    use crate::ui::ColorElement;

    let reset_attr = crt.color(ColorElement::ResetColor);

    crt.with_window(|win| {
        let _ = win.mv(y, x);

        for seg in segments {
            let _ = win.attrset(seg.attr);
            let _ = win.addstr(seg.text);
        }

        let _ = win.attrset(reset_attr);
    });
}

/// Draw a bar meter with multiple colored segments and text overlay
///
/// This is a generic helper for meters like Memory, Swap, and Tasks that display
/// stacked colored segments with right-aligned text overlay.
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position
/// * `width` - Total width including caption
/// * `caption` - Caption to display (typically 3 chars, e.g., "Mem", "Swp", "Tsk")
/// * `segments` - Colored segments to draw (from left to right)
/// * `total` - Denominator for calculating percentages
/// * `text` - Text to display right-aligned inside the bar (e.g., "1.5G/8.0G")
#[allow(clippy::too_many_arguments)]
pub fn draw_bar_with_text(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    caption: &str,
    segments: &[BarSegment],
    total: f64,
    text: &str,
) {
    use crate::ui::ColorElement;

    let caption_attr = crt.color(ColorElement::MeterText);
    let reset_attr = crt.color(ColorElement::ResetColor);
    let bracket_attr = crt.color(ColorElement::BarBorder);
    let shadow_attr = crt.color(ColorElement::BarShadow);
    // Pre-compute bar characters for each segment (theme-agnostic)
    let segment_bar_chars: Vec<char> = (0..segments.len()).map(|i| crt.bar_char(i)).collect();

    // Caption is typically 3 chars
    let caption_len = caption.len() as i32;

    crt.with_window(|win| {
        // Draw caption
        let _ = win.mv(y, x);
        let _ = win.attrset(caption_attr);
        let _ = win.addstr(caption);

        // Bar area starts after caption
        let bar_x = x + caption_len;
        let bar_width = width - caption_len;

        if bar_width < 4 {
            let _ = win.attrset(reset_attr);
            return;
        }

        // Draw brackets
        let _ = win.attrset(bracket_attr);
        let _ = win.mvaddch(y, bar_x, '[' as u32);
        let _ = win.mvaddch(y, bar_x + bar_width - 1, ']' as u32);

        // Inner bar width (between brackets)
        let inner_width = (bar_width - 2) as usize;

        // Build the bar content with text right-aligned
        let text_len = text.len();
        let padding = inner_width.saturating_sub(text_len);

        // Calculate how many chars each segment takes
        let mut bar_chars: Vec<(usize, u32)> = Vec::new();
        let mut total_bar = 0usize;
        for seg in segments {
            let chars = if total > 0.0 {
                ((seg.value / total) * inner_width as f64).ceil() as usize
            } else {
                0
            };
            let chars = chars.min(inner_width - total_bar);
            bar_chars.push((chars, seg.attr));
            total_bar += chars;
        }

        // Draw the bar content
        let _ = win.mv(y, bar_x + 1);
        let mut pos = 0;
        for (idx, (chars, attr)) in bar_chars.iter().enumerate() {
            let _ = win.attrset(*attr);
            let bar_ch = segment_bar_chars.get(idx).copied().unwrap_or('|');
            for _ in 0..*chars {
                if pos >= padding && pos - padding < text_len {
                    // Draw text character
                    let ch = text.chars().nth(pos - padding).unwrap_or(bar_ch);
                    let _ = win.addch(ch as u32);
                } else {
                    let _ = win.addch(bar_ch as u32);
                }
                pos += 1;
            }
        }

        // Fill remaining with shadow (and text if extends into shadow)
        let _ = win.attrset(shadow_attr);
        while pos < inner_width {
            if pos >= padding && pos - padding < text_len {
                let ch = text.chars().nth(pos - padding).unwrap_or(' ');
                let _ = win.addch(ch as u32);
            } else {
                let _ = win.addch(' ' as u32);
            }
            pos += 1;
        }
        let _ = win.attrset(reset_attr);
    });
}

/// Draw a bar meter
/// Note: values contains (percentage, attr) where attr is already a color attribute from crt.color()
pub fn draw_bar(crt: &mut Crt, x: i32, y: i32, width: i32, values: &[(f64, i32)], total: f64) {
    use crate::ui::ColorElement;

    // Need at least 3 chars for brackets and one char of content: [x]
    if width < 3 {
        return;
    }

    let bar_width = (width - 2) as usize; // Account for [ and ]

    let bracket_attr = crt.color(ColorElement::BarBorder);
    let shadow_attr = crt.color(ColorElement::MeterShadow);
    // Pre-compute bar characters for each value segment (theme-agnostic)
    let value_bar_chars: Vec<char> = (0..values.len()).map(|i| crt.bar_char(i)).collect();

    // The color values passed in are already attr_t values (from crt.color()), not indices
    let value_attrs: Vec<_> = values.iter().map(|(_, attr)| *attr as u32).collect();

    crt.with_window(|win| {
        // Draw brackets
        let _ = win.attrset(bracket_attr);
        let _ = win.mvaddch(y, x, '[' as u32);
        let _ = win.mvaddch(y, x + width - 1, ']' as u32);

        // Calculate bar content
        let mut bar_pos = 0;
        let _ = win.mv(y, x + 1);

        for (idx, ((value, _), attr)) in values.iter().zip(value_attrs.iter()).enumerate() {
            let bar_chars = if total > 0.0 {
                ((value / total) * bar_width as f64).round() as usize
            } else {
                0
            };

            let _ = win.attrset(*attr);
            let bar_ch = value_bar_chars.get(idx).copied().unwrap_or('|');
            for _ in 0..bar_chars.min(bar_width - bar_pos) {
                let _ = win.addch(bar_ch as u32);
                bar_pos += 1;
            }

            if bar_pos >= bar_width {
                break;
            }
        }

        // Fill remaining with shadow
        let _ = win.attrset(shadow_attr);
        while bar_pos < bar_width {
            let _ = win.addch(' ' as u32);
            bar_pos += 1;
        }
    });
}

/// Draw a text meter
pub fn draw_text(crt: &mut Crt, x: i32, y: i32, caption: &str, text: &str) {
    use crate::ui::ColorElement;

    let caption_attr = crt.color(ColorElement::MeterText);
    let value_attr = crt.color(ColorElement::MeterValue);

    crt.with_window(|win| {
        let _ = win.mv(y, x);
        let _ = win.attrset(caption_attr);
        let _ = win.addstr(caption);

        let _ = win.attrset(value_attr);
        let _ = win.addstr(text);
    });
}

// ============================================================================
// Graph Meter Mode
// ============================================================================

/// Braille dot patterns for graph drawing (matches C htop GraphMeterMode_dotsUtf8)
/// Each character encodes 2 values (left and right columns) with 4 vertical levels each.
/// Index = left_value * 5 + right_value, where each value is 0-4 (number of dots lit)
///
/// Braille dot positions:
///   1 4
///   2 5
///   3 6
///   7 8
///
/// Left column uses dots 1,2,3,7 (from top to bottom)
/// Right column uses dots 4,5,6,8 (from top to bottom)
const GRAPH_DOTS_UTF8: [&str; 25] = [
    /*00*/ " ", /*01*/ "⢀", /*02*/ "⢠", /*03*/ "⢰", /*04*/ "⢸",
    /*10*/ "⡀", /*11*/ "⣀", /*12*/ "⣠", /*13*/ "⣰", /*14*/ "⣸",
    /*20*/ "⡄", /*21*/ "⣄", /*22*/ "⣤", /*23*/ "⣴", /*24*/ "⣼",
    /*30*/ "⡆", /*31*/ "⣆", /*32*/ "⣦", /*33*/ "⣶", /*34*/ "⣾",
    /*40*/ "⡇", /*41*/ "⣇", /*42*/ "⣧", /*43*/ "⣷", /*44*/ "⣿",
];

/// Number of vertical pixels (dots) per row for UTF-8 braille
const PIXPERROW_UTF8: i32 = 4;

/// Draw a graph meter using Braille characters (matches C htop)
///
/// The graph is 4 rows tall (DEFAULT_GRAPH_HEIGHT). Each character column displays
/// 2 data values using Braille patterns, giving 4 vertical dots per row.
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical data to display
/// * `caption` - Caption to display (3 chars)
pub fn draw_graph(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &GraphData,
    caption: &str,
) {
    use crate::ui::ColorElement;

    let meter_text_attr = crt.color(ColorElement::MeterText);
    let graph1_attr = crt.color(ColorElement::Graph1);
    let graph2_attr = crt.color(ColorElement::Graph2);
    let reset_attr = crt.color(ColorElement::ResetColor);

    // Caption takes 3 characters
    let caption_len = 3;
    let graph_width = (width - caption_len).max(0) as usize;

    if graph_width == 0 {
        return;
    }

    // Get values for display - 2 values per column (left and right dot columns in braille)
    let values = &graph_data.values;
    let needed_values = graph_width * 2;

    // Total vertical resolution: height rows × 4 dots per row
    let total_pixels = height * PIXPERROW_UTF8;

    crt.with_window(|win| {
        // Draw caption on the first row (y is top of graph)
        let _ = win.attrset(meter_text_attr);
        let _ = win.mvaddnstr(y, x, caption, caption_len);
        let _ = win.attrset(reset_attr);

        // Calculate graph area
        let graph_x = x + caption_len;

        // For each row (top to bottom)
        for row in 0..height {
            let _ = win.mv(y + row, graph_x);

            for col in 0..graph_width {
                // Each column uses 2 consecutive values (left and right)
                let pair_idx = col * 2;

                // Get the two values for this column
                let (val_left, val_right) = if values.len() >= needed_values {
                    let start = values.len() - needed_values;
                    (values[start + pair_idx], values[start + pair_idx + 1])
                } else {
                    // Not enough data - calculate offset
                    let offset = needed_values - values.len();
                    let left = if pair_idx >= offset {
                        values[pair_idx - offset]
                    } else {
                        0.0
                    };
                    let right = if pair_idx + 1 >= offset {
                        values[pair_idx + 1 - offset]
                    } else {
                        0.0
                    };
                    (left, right)
                };

                // Convert values (0.0-1.0) to filled pixels (1 to total_pixels)
                // Minimum of 1 pixel ensures a baseline across the full width (matches C htop)
                let left_pixels = (val_left * total_pixels as f64)
                    .round()
                    .clamp(1.0, total_pixels as f64) as i32;
                let right_pixels = (val_right * total_pixels as f64)
                    .round()
                    .clamp(1.0, total_pixels as f64) as i32;

                // For this row, calculate how many dots are lit
                // Row 0 is top (fills last), row (height-1) is bottom (fills first)
                let row_from_bottom = height - 1 - row;
                let pixels_below = row_from_bottom * PIXPERROW_UTF8;

                // Dots in this row (0-4 for each column)
                let left_dots = (left_pixels - pixels_below).clamp(0, PIXPERROW_UTF8) as usize;
                let right_dots = (right_pixels - pixels_below).clamp(0, PIXPERROW_UTF8) as usize;

                // Look up the braille character
                let dot_idx = left_dots * 5 + right_dots;
                let braille_char = GRAPH_DOTS_UTF8[dot_idx];

                // Use Graph1 for higher values (>50%), Graph2 for lower
                // Use the higher of the two values to determine color
                let max_val = val_left.max(val_right);
                let attr = if max_val > 0.5 {
                    graph1_attr
                } else {
                    graph2_attr
                };
                let _ = win.attrset(attr);
                let _ = win.addstr(braille_char);
            }
        }

        let _ = win.attrset(reset_attr);
    });
}

/// Draw a graph meter using Braille characters with a custom color
///
/// This is similar to draw_graph but uses a single custom color instead of
/// the Graph1/Graph2 color scheme. Used for meters that want to match their
/// bar color in graph mode.
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical data to display
/// * `caption` - Caption to display (3 chars)
/// * `graph_color` - Color attribute to use for the graph
#[allow(clippy::too_many_arguments)]
pub fn draw_graph_colored(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &GraphData,
    caption: &str,
    graph_color: u32,
) {
    use crate::ui::ColorElement;

    let meter_text_attr = crt.color(ColorElement::MeterText);
    let reset_attr = crt.color(ColorElement::ResetColor);

    // Caption takes 3 characters
    let caption_len = 3;
    let graph_width = (width - caption_len).max(0) as usize;

    if graph_width == 0 {
        return;
    }

    // Get values for display - 2 values per column (left and right dot columns in braille)
    let values = &graph_data.values;
    let needed_values = graph_width * 2;

    // Total vertical resolution: height rows × 4 dots per row
    let total_pixels = height * PIXPERROW_UTF8;

    crt.with_window(|win| {
        // Draw caption on the first row (y is top of graph)
        let _ = win.attrset(meter_text_attr);
        let _ = win.mvaddnstr(y, x, caption, caption_len);
        let _ = win.attrset(reset_attr);

        // Calculate graph area
        let graph_x = x + caption_len;

        // For each row (top to bottom)
        for row in 0..height {
            let _ = win.mv(y + row, graph_x);

            for col in 0..graph_width {
                // Each column uses 2 consecutive values (left and right)
                let pair_idx = col * 2;

                // Get the two values for this column
                let (val_left, val_right) = if values.len() >= needed_values {
                    let start = values.len() - needed_values;
                    (values[start + pair_idx], values[start + pair_idx + 1])
                } else {
                    // Not enough data - calculate offset
                    let offset = needed_values - values.len();
                    let left = if pair_idx >= offset {
                        values[pair_idx - offset]
                    } else {
                        0.0
                    };
                    let right = if pair_idx + 1 >= offset {
                        values[pair_idx + 1 - offset]
                    } else {
                        0.0
                    };
                    (left, right)
                };

                // Convert values (0.0-1.0) to filled pixels (1 to total_pixels)
                // Minimum of 1 pixel ensures a baseline across the full width (matches C htop)
                let left_pixels = (val_left * total_pixels as f64)
                    .round()
                    .clamp(1.0, total_pixels as f64) as i32;
                let right_pixels = (val_right * total_pixels as f64)
                    .round()
                    .clamp(1.0, total_pixels as f64) as i32;

                // For this row, calculate how many dots are lit
                // Row 0 is top (fills last), row (height-1) is bottom (fills first)
                let row_from_bottom = height - 1 - row;
                let pixels_below = row_from_bottom * PIXPERROW_UTF8;

                // Dots in this row (0-4 for each column)
                let left_dots = (left_pixels - pixels_below).clamp(0, PIXPERROW_UTF8) as usize;
                let right_dots = (right_pixels - pixels_below).clamp(0, PIXPERROW_UTF8) as usize;

                // Look up the braille character
                let dot_idx = left_dots * 5 + right_dots;
                let braille_char = GRAPH_DOTS_UTF8[dot_idx];

                // Use the custom color for all graph elements
                let _ = win.attrset(graph_color);
                let _ = win.addstr(braille_char);
            }
        }

        let _ = win.attrset(reset_attr);
    });
}

/// Draw a self-scaling graph meter using Braille characters (matches C htop for non-percent meters)
///
/// Unlike the regular draw_graph which expects pre-normalized values (0.0-1.0),
/// this function works with raw values and normalizes them at draw time using
/// the maximum value in the graph data. This is used for meters like Tasks
/// where the scale should auto-adjust to the data.
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical data to display (raw values, not pre-normalized)
/// * `caption` - Caption to display (3 chars)
pub fn draw_graph_self_scaling(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &GraphData,
    caption: &str,
) {
    use crate::ui::ColorElement;

    let meter_text_attr = crt.color(ColorElement::MeterText);
    let graph1_attr = crt.color(ColorElement::Graph1);
    let graph2_attr = crt.color(ColorElement::Graph2);
    let reset_attr = crt.color(ColorElement::ResetColor);

    // Caption takes 3 characters
    let caption_len = 3;
    let graph_width = (width - caption_len).max(0) as usize;

    if graph_width == 0 {
        return;
    }

    // Get values for display - 2 values per column (left and right dot columns in braille)
    let values = &graph_data.values;
    let needed_values = graph_width * 2;

    // For self-scaling, find max value from the portion of data we'll display
    let max_value = if values.len() >= needed_values {
        let start = values.len() - needed_values;
        values[start..]
            .iter()
            .copied()
            .fold(1.0_f64, |acc, v| acc.max(v))
    } else {
        values.iter().copied().fold(1.0_f64, |acc, v| acc.max(v))
    };

    // Total vertical resolution: height rows × 4 dots per row
    let total_pixels = height * PIXPERROW_UTF8;

    crt.with_window(|win| {
        // Draw caption on the first row (y is top of graph)
        let _ = win.attrset(meter_text_attr);
        let _ = win.mvaddnstr(y, x, caption, caption_len);
        let _ = win.attrset(reset_attr);

        // Calculate graph area
        let graph_x = x + caption_len;

        // For each row (top to bottom)
        for row in 0..height {
            let _ = win.mv(y + row, graph_x);

            for col in 0..graph_width {
                // Each column uses 2 consecutive values (left and right)
                let pair_idx = col * 2;

                // Get the two raw values for this column
                let (raw_left, raw_right) = if values.len() >= needed_values {
                    let start = values.len() - needed_values;
                    (values[start + pair_idx], values[start + pair_idx + 1])
                } else {
                    // Not enough data - calculate offset
                    let offset = needed_values - values.len();
                    let left = if pair_idx >= offset {
                        values[pair_idx - offset]
                    } else {
                        0.0
                    };
                    let right = if pair_idx + 1 >= offset {
                        values[pair_idx + 1 - offset]
                    } else {
                        0.0
                    };
                    (left, right)
                };

                // Normalize values by dividing by max
                let val_left = raw_left / max_value;
                let val_right = raw_right / max_value;

                // Convert values (0.0-1.0) to filled pixels (1 to total_pixels)
                // Minimum of 1 pixel ensures a baseline across the full width (matches C htop)
                let left_pixels = (val_left * total_pixels as f64)
                    .round()
                    .clamp(1.0, total_pixels as f64) as i32;
                let right_pixels = (val_right * total_pixels as f64)
                    .round()
                    .clamp(1.0, total_pixels as f64) as i32;

                // For this row, calculate how many dots are lit
                // Row 0 is top (fills last), row (height-1) is bottom (fills first)
                let row_from_bottom = height - 1 - row;
                let pixels_below = row_from_bottom * PIXPERROW_UTF8;

                // Dots in this row (0-4 for each column)
                let left_dots = (left_pixels - pixels_below).clamp(0, PIXPERROW_UTF8) as usize;
                let right_dots = (right_pixels - pixels_below).clamp(0, PIXPERROW_UTF8) as usize;

                // Look up the braille character
                let dot_idx = left_dots * 5 + right_dots;
                let braille_char = GRAPH_DOTS_UTF8[dot_idx];

                // Use Graph1 for higher values (>50%), Graph2 for lower
                // Use the higher of the two values to determine color
                let max_val = val_left.max(val_right);
                let attr = if max_val > 0.5 {
                    graph1_attr
                } else {
                    graph2_attr
                };
                let _ = win.attrset(attr);
                let _ = win.addstr(braille_char);
            }
        }

        let _ = win.attrset(reset_attr);
    });
}

/// Find which segment a pixel at a given height belongs to (slice version)
/// Segments are stacked from bottom: seg[0] is at bottom, seg[N-1] is at top
/// If max_scale is provided, values are normalized to that scale (for self-scaling graphs)
fn find_segment_at_pixel_slice(
    segments: &[f64],
    pixel: i32,
    total_pixels: i32,
    max_scale: Option<f64>,
) -> usize {
    let scale = max_scale.unwrap_or(1.0);
    let mut cumulative = 0.0;
    for (idx, &seg_val) in segments.iter().enumerate() {
        cumulative += seg_val;
        let normalized = cumulative / scale;
        let seg_pixel_top = (normalized * total_pixels as f64).round() as i32;
        if pixel < seg_pixel_top {
            return idx;
        }
    }
    // Default to last segment if pixel is at or above total
    segments.len().saturating_sub(1)
}

/// Generic stacked graph drawing function that works with any StackedGraphValues implementation
///
/// This is the unified implementation for all stacked graph meters (Memory, CPU, Swap, Load, Tasks).
/// It uses the StackedGraphValues trait to abstract over the segment count.
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical stacked data (implements StackedGraphValues)
/// * `caption` - Caption to display (3 chars)
/// * `segment_colors` - Color attributes for each segment (bottom to top)
/// * `max_scale` - Optional max scale for self-scaling graphs. When Some(val), values are
///   normalized by dividing by val. When None, values are assumed to be 0-1 range.
#[allow(clippy::too_many_arguments)]
pub fn draw_stacked_graph_generic<T: StackedGraphValues>(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &T,
    caption: &str,
    segment_colors: &[u32],
    max_scale: Option<f64>,
) {
    use crate::ui::ColorElement;

    let meter_text_attr = crt.color(ColorElement::MeterText);
    let reset_attr = crt.color(ColorElement::ResetColor);

    // Caption takes 3 characters
    let caption_len = 3;
    let graph_width = (width - caption_len).max(0) as usize;

    if graph_width == 0 {
        return;
    }

    // Get values for display - 2 values per column (left and right dot columns in braille)
    let num_values = graph_data.len();
    let needed_values = graph_width * 2;
    let zero_segs = graph_data.zero_segments();

    // Total vertical resolution: height rows × 4 dots per row
    let total_pixels = height * PIXPERROW_UTF8;

    crt.with_window(|win| {
        // Draw caption on the first row (y is top of graph)
        let _ = win.attrset(meter_text_attr);
        let _ = win.mvaddnstr(y, x, caption, caption_len);
        let _ = win.attrset(reset_attr);

        // Calculate graph area
        let graph_x = x + caption_len;

        // For each row (top to bottom)
        for row in 0..height {
            let _ = win.mv(y + row, graph_x);

            for col in 0..graph_width {
                // Each column uses 2 consecutive values (left and right)
                let pair_idx = col * 2;

                // Get the two segment slices for this column
                let (segs_left, segs_right): (&[f64], &[f64]) = if num_values >= needed_values {
                    let start = num_values - needed_values;
                    (
                        graph_data
                            .get_segments(start + pair_idx)
                            .unwrap_or(&zero_segs),
                        graph_data
                            .get_segments(start + pair_idx + 1)
                            .unwrap_or(&zero_segs),
                    )
                } else {
                    // Not enough data - calculate offset
                    let offset = needed_values - num_values;
                    let left = if pair_idx >= offset {
                        graph_data
                            .get_segments(pair_idx - offset)
                            .unwrap_or(&zero_segs)
                    } else {
                        &zero_segs
                    };
                    let right = if pair_idx + 1 >= offset {
                        graph_data
                            .get_segments(pair_idx + 1 - offset)
                            .unwrap_or(&zero_segs)
                    } else {
                        &zero_segs
                    };
                    (left, right)
                };

                // Calculate total value for each column (sum of segments)
                // If max_scale is provided, normalize by dividing by it
                let scale = max_scale.unwrap_or(1.0);
                let total_left: f64 = segs_left.iter().sum::<f64>() / scale;
                let total_right: f64 = segs_right.iter().sum::<f64>() / scale;

                // Convert values to filled pixels (minimum 1 for baseline)
                let left_pixels = (total_left * total_pixels as f64)
                    .round()
                    .clamp(1.0, total_pixels as f64) as i32;
                let right_pixels = (total_right * total_pixels as f64)
                    .round()
                    .clamp(1.0, total_pixels as f64) as i32;

                // For this row, calculate how many dots are lit
                let row_from_bottom = height - 1 - row;
                let pixels_below = row_from_bottom * PIXPERROW_UTF8;

                // Dots in this row (0-4 for each column)
                let left_dots = (left_pixels - pixels_below).clamp(0, PIXPERROW_UTF8) as usize;
                let right_dots = (right_pixels - pixels_below).clamp(0, PIXPERROW_UTF8) as usize;

                // Look up the braille character
                let dot_idx = left_dots * 5 + right_dots;
                let braille_char = GRAPH_DOTS_UTF8[dot_idx];

                // Determine which segment this row primarily belongs to
                // Find the dominant segment at the middle of this row's pixel range
                let row_pixel_start = pixels_below;
                let row_pixel_mid = row_pixel_start + PIXPERROW_UTF8 / 2;

                // Use the column with more dots to determine segment color
                let (segs, pixel_mid) = if left_dots >= right_dots {
                    (segs_left, row_pixel_mid)
                } else {
                    (segs_right, row_pixel_mid)
                };

                // Find which segment this pixel belongs to
                let segment_idx =
                    find_segment_at_pixel_slice(segs, pixel_mid, total_pixels, max_scale);
                let attr = segment_colors
                    .get(segment_idx)
                    .copied()
                    .unwrap_or(segment_colors[0]);

                let _ = win.attrset(attr);
                let _ = win.addstr(braille_char);
            }
        }

        let _ = win.attrset(reset_attr);
    });
}

/// Draw a stacked graph meter using Braille characters with multiple colored segments
///
/// Unlike the regular graph which uses Graph1/Graph2 colors based on value threshold,
/// this draws each segment with its own color, creating a stacked bar-like appearance
/// in graph form.
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical stacked data to display
/// * `caption` - Caption to display (3 chars)
/// * `segment_colors` - Color attributes for each segment (bottom to top)
#[allow(clippy::too_many_arguments)]
pub fn draw_stacked_graph(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &StackedGraphData,
    caption: &str,
    segment_colors: &[u32; STACKED_GRAPH_SEGMENTS],
) {
    draw_stacked_graph_generic(
        crt,
        x,
        y,
        width,
        height,
        graph_data,
        caption,
        segment_colors,
        None,
    )
}

/// Draw a CPU stacked graph meter using Braille characters with multiple colored segments
///
/// Similar to draw_stacked_graph but for 8-segment CPU data:
/// user, nice, system, irq, softirq, steal, guest, iowait
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical CPU stacked data to display
/// * `caption` - Caption to display (3 chars)
/// * `segment_colors` - Color attributes for each segment (bottom to top)
#[allow(clippy::too_many_arguments)]
pub fn draw_cpu_stacked_graph(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &CpuStackedGraphData,
    caption: &str,
    segment_colors: &[u32; CPU_STACKED_GRAPH_SEGMENTS],
) {
    draw_stacked_graph_generic(
        crt,
        x,
        y,
        width,
        height,
        graph_data,
        caption,
        segment_colors,
        None,
    )
}

/// Draw a Swap stacked graph meter using Braille characters with multiple colored segments
///
/// Similar to draw_stacked_graph but for 2-segment Swap data:
/// used, cache
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical Swap stacked data to display
/// * `caption` - Caption to display (3 chars)
/// * `segment_colors` - Color attributes for each segment (bottom to top)
#[allow(clippy::too_many_arguments)]
pub fn draw_swap_stacked_graph(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &SwapStackedGraphData,
    caption: &str,
    segment_colors: &[u32; SWAP_STACKED_GRAPH_SEGMENTS],
) {
    draw_stacked_graph_generic(
        crt,
        x,
        y,
        width,
        height,
        graph_data,
        caption,
        segment_colors,
        None,
    )
}

/// Draw a Load stacked graph meter using Braille characters with multiple colored segments
///
/// Similar to draw_stacked_graph but for 3-segment Load data with self-scaling:
/// load1, load5, load15
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical Load stacked data to display
/// * `caption` - Caption to display (3 chars)
/// * `segment_colors` - Color attributes for each segment (bottom to top)
#[allow(clippy::too_many_arguments)]
pub fn draw_load_stacked_graph(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &LoadStackedGraphData,
    caption: &str,
    segment_colors: &[u32; LOAD_STACKED_GRAPH_SEGMENTS],
) {
    // For self-scaling, compute max_sum from visible data
    let caption_len = 3;
    let graph_width = (width - caption_len).max(0) as usize;
    let needed_values = graph_width * 2;
    let values = &graph_data.values;

    let max_sum = if values.len() >= needed_values {
        let start = values.len() - needed_values;
        values[start..]
            .iter()
            .map(|segs| segs.iter().sum::<f64>())
            .fold(1.0_f64, |acc, sum| acc.max(sum))
    } else {
        values
            .iter()
            .map(|segs| segs.iter().sum::<f64>())
            .fold(1.0_f64, |acc, sum| acc.max(sum))
    };

    draw_stacked_graph_generic(
        crt,
        x,
        y,
        width,
        height,
        graph_data,
        caption,
        segment_colors,
        Some(max_sum),
    )
}

/// Draw a Tasks stacked graph meter using Braille characters with multiple colored segments
///
/// Similar to draw_stacked_graph but for 4-segment Tasks data with self-scaling:
/// kernel threads, userland threads, processes, running
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of graph)
/// * `width` - Total width including caption
/// * `height` - Height in rows (typically 4)
/// * `graph_data` - Historical Tasks stacked data to display
/// * `caption` - Caption to display (3 chars)
/// * `segment_colors` - Color attributes for each segment (bottom to top)
#[allow(clippy::too_many_arguments)]
pub fn draw_tasks_stacked_graph(
    crt: &mut Crt,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    graph_data: &TasksStackedGraphData,
    caption: &str,
    segment_colors: &[u32; TASKS_STACKED_GRAPH_SEGMENTS],
) {
    // For self-scaling, compute max_sum from visible data
    let caption_len = 3;
    let graph_width = (width - caption_len).max(0) as usize;
    let needed_values = graph_width * 2;
    let values = &graph_data.values;

    let max_sum = if values.len() >= needed_values {
        let start = values.len() - needed_values;
        values[start..]
            .iter()
            .map(|segs| segs.iter().sum::<f64>())
            .fold(1.0_f64, |acc, sum| acc.max(sum))
    } else {
        values
            .iter()
            .map(|segs| segs.iter().sum::<f64>())
            .fold(1.0_f64, |acc, sum| acc.max(sum))
    };

    draw_stacked_graph_generic(
        crt,
        x,
        y,
        width,
        height,
        graph_data,
        caption,
        segment_colors,
        Some(max_sum),
    )
}

// ============================================================================
// LED Meter Mode
// ============================================================================

/// ASCII LED digits (3 rows, 10 digits 0-9, each digit is 4 chars wide)
/// Row 0: top of digit, Row 1: middle, Row 2: bottom
/// Index: row * 10 + digit
const LED_DIGITS_ASCII: [&str; 30] = [
    // Row 0 (top): digits 0-9
    " __ ", "    ", " __ ", " __ ", "    ", " __ ", " __ ", " __ ", " __ ", " __ ",
    // Row 1 (middle): digits 0-9
    "|  |", "   |", " __|", " __|", "|__|", "|__ ", "|__ ", "   |", "|__|", "|__|",
    // Row 2 (bottom): digits 0-9
    "|__|", "   |", "|__ ", " __|", "   |", " __|", "|__|", "   |", "|__|", " __|",
];

/// UTF-8 LED digits (3 rows, 10 digits 0-9, each digit is 4 chars wide)
/// Uses box-drawing characters for a cleaner look
const LED_DIGITS_UTF8: [&str; 30] = [
    // Row 0 (top): digits 0-9
    "┌──┐",
    "  ┐ ",
    "╶──┐",
    "╶──┐",
    "╷  ╷",
    "┌──╴",
    "┌──╴",
    "╶──┐",
    "┌──┐",
    "┌──┐",
    // Row 1 (middle): digits 0-9
    "│  │",
    "  │ ",
    "┌──┘",
    " ──┤",
    "└──┤",
    "└──┐",
    "├──┐",
    "   │",
    "├──┤",
    "└──┤",
    // Row 2 (bottom): digits 0-9
    "└──┘",
    "  ╵ ",
    "└──╴",
    "╶──┘",
    "   ╵",
    "╶──┘",
    "└──┘",
    "   ╵",
    "└──┘",
    "╶──┘",
];

/// Draw a single LED digit at position (x, y)
fn draw_led_digit(crt: &mut Crt, x: i32, y: i32, digit: u8) {
    let digits = if crt.utf8 {
        &LED_DIGITS_UTF8
    } else {
        &LED_DIGITS_ASCII
    };

    let d = digit as usize;
    if d > 9 {
        return;
    }

    crt.with_window(|win| {
        for row in 0..3 {
            let idx = row * 10 + d;
            if let Some(s) = digits.get(idx) {
                let _ = win.mvaddstr(y + row as i32, x, s);
            }
        }
    });
}

/// Draw an LED meter
///
/// # Arguments
/// * `crt` - Terminal context
/// * `x` - X position
/// * `y` - Y position (top of LED display, 3 rows tall)
/// * `width` - Total width
/// * `caption` - Caption to display
/// * `text` - Text to display as LED digits (digits are rendered as LED, other chars as-is)
pub fn draw_led(crt: &mut Crt, x: i32, y: i32, width: i32, caption: &str, text: &str) {
    use crate::ui::ColorElement;

    // Y position for non-digit text (caption and symbols like %, /, .)
    // UTF-8: middle row (y + 1), ASCII: bottom row (y + 2)
    let y_text = if crt.utf8 { y + 1 } else { y + 2 };
    let led_attr = crt.color(ColorElement::LedColor);
    let reset_attr = crt.color(ColorElement::ResetColor);

    crt.with_window(|win| {
        let _ = win.attrset(led_attr);

        // Draw the caption
        if width > 0 {
            let caption_display: String = caption.chars().take(width as usize).collect();
            let _ = win.mvaddstr(y_text, x, &caption_display);
        }
    });

    let caption_width = caption.chars().count().min(width as usize) as i32;
    if width <= caption_width {
        crt.with_window(|win| {
            let _ = win.attrset(reset_attr);
        });
        return;
    }

    let mut xx = x + caption_width;
    let _remaining_width = width - caption_width;

    // Draw each character
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            // Check if we have room for a 4-char wide digit
            if xx > x + width - 4 {
                break;
            }

            let digit = (ch as u8) - b'0';
            draw_led_digit(crt, xx, y, digit);
            xx += 4;
        } else {
            // Non-digit character - draw on the text line
            if xx > x + width - 1 {
                break;
            }

            crt.with_window(|win| {
                let _ = win.mvaddch(y_text, xx, ch as u32);
            });
            xx += 1;
        }
    }

    crt.with_window(|win| {
        let _ = win.attrset(reset_attr);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    // =========================================================================
    // MeterMode tests
    // =========================================================================

    #[test]
    fn test_meter_mode_default() {
        let mode: MeterMode = Default::default();
        assert_eq!(mode, MeterMode::Bar);
    }

    #[test]
    fn test_meter_mode_default_height_bar() {
        assert_eq!(MeterMode::Bar.default_height(), 1);
    }

    #[test]
    fn test_meter_mode_default_height_text() {
        assert_eq!(MeterMode::Text.default_height(), 1);
    }

    #[test]
    fn test_meter_mode_default_height_graph() {
        assert_eq!(MeterMode::Graph.default_height(), DEFAULT_GRAPH_HEIGHT);
        assert_eq!(MeterMode::Graph.default_height(), 4);
    }

    #[test]
    fn test_meter_mode_default_height_led() {
        assert_eq!(MeterMode::Led.default_height(), 3);
    }

    #[test]
    fn test_meter_mode_from_core() {
        use crate::core::MeterMode as CoreMeterMode;

        assert_eq!(MeterMode::from(CoreMeterMode::Bar), MeterMode::Bar);
        assert_eq!(MeterMode::from(CoreMeterMode::Text), MeterMode::Text);
        assert_eq!(MeterMode::from(CoreMeterMode::Graph), MeterMode::Graph);
        assert_eq!(MeterMode::from(CoreMeterMode::Led), MeterMode::Led);
    }

    // =========================================================================
    // GRAPH_DOTS_UTF8 tests
    // =========================================================================

    #[test]
    fn test_graph_dots_utf8_array_length() {
        // Array should have exactly 25 elements (5x5 grid for left_dots * 5 + right_dots)
        assert_eq!(GRAPH_DOTS_UTF8.len(), 25);
    }

    #[test]
    fn test_graph_dots_utf8_empty_is_space() {
        // Index 0 (left=0, right=0) should be a space
        assert_eq!(GRAPH_DOTS_UTF8[0], " ");
    }

    #[test]
    fn test_graph_dots_utf8_full_is_full_block() {
        // Index 24 (left=4, right=4) should be full braille block
        assert_eq!(GRAPH_DOTS_UTF8[24], "⣿");
    }

    #[test]
    fn test_graph_dots_utf8_index_calculation() {
        // Test the index formula: left_dots * 5 + right_dots
        // Each value 0-4 represents number of dots lit in that column

        // left=0, right=1 -> index 1
        assert_eq!(GRAPH_DOTS_UTF8[0 * 5 + 1], "⢀");

        // left=1, right=0 -> index 5
        assert_eq!(GRAPH_DOTS_UTF8[1 * 5 + 0], "⡀");

        // left=2, right=2 -> index 12
        assert_eq!(GRAPH_DOTS_UTF8[2 * 5 + 2], "⣤");

        // left=4, right=0 -> index 20
        assert_eq!(GRAPH_DOTS_UTF8[4 * 5 + 0], "⡇");

        // left=0, right=4 -> index 4
        assert_eq!(GRAPH_DOTS_UTF8[0 * 5 + 4], "⢸");
    }

    #[test]
    fn test_graph_dots_utf8_all_valid_indices() {
        // Verify all valid index combinations (0-4 for each) return valid strings
        for left_dots in 0..=4 {
            for right_dots in 0..=4 {
                let idx = left_dots * 5 + right_dots;
                let result = GRAPH_DOTS_UTF8.get(idx);
                assert!(
                    result.is_some(),
                    "Index {} (left={}, right={}) should be valid",
                    idx,
                    left_dots,
                    right_dots
                );
                assert!(
                    !result.unwrap().is_empty() || idx == 0,
                    "Character at index {} should not be empty (except space at 0)",
                    idx
                );
            }
        }
    }

    #[test]
    fn test_graph_dots_utf8_out_of_bounds_safe() {
        // Using get() should return None for out-of-bounds indices
        assert!(GRAPH_DOTS_UTF8.get(25).is_none());
        assert!(GRAPH_DOTS_UTF8.get(100).is_none());

        // With unwrap_or, should return fallback
        assert_eq!(GRAPH_DOTS_UTF8.get(25).unwrap_or(&" "), &" ");
    }

    #[test]
    fn test_graph_dots_utf8_symmetry() {
        // Test some symmetric patterns
        // left=1, right=1 should show dots in both columns
        assert_eq!(GRAPH_DOTS_UTF8[1 * 5 + 1], "⣀");

        // left=3, right=3 should show 3 dots in each column
        assert_eq!(GRAPH_DOTS_UTF8[3 * 5 + 3], "⣶");
    }

    #[test]
    fn test_graph_dots_utf8_row_patterns() {
        // Test first row (left=0, varying right)
        assert_eq!(GRAPH_DOTS_UTF8[0], " ");
        assert_eq!(GRAPH_DOTS_UTF8[1], "⢀");
        assert_eq!(GRAPH_DOTS_UTF8[2], "⢠");
        assert_eq!(GRAPH_DOTS_UTF8[3], "⢰");
        assert_eq!(GRAPH_DOTS_UTF8[4], "⢸");

        // Test last row (left=4, varying right)
        assert_eq!(GRAPH_DOTS_UTF8[20], "⡇");
        assert_eq!(GRAPH_DOTS_UTF8[21], "⣇");
        assert_eq!(GRAPH_DOTS_UTF8[22], "⣧");
        assert_eq!(GRAPH_DOTS_UTF8[23], "⣷");
        assert_eq!(GRAPH_DOTS_UTF8[24], "⣿");
    }

    #[test]
    fn test_pixperrow_utf8_constant() {
        // Each braille character has 4 vertical dots per column
        assert_eq!(PIXPERROW_UTF8, 4);
    }

    // =========================================================================
    // GraphData tests
    // =========================================================================

    #[test]
    fn test_graph_data_new() {
        let data = GraphData::new();
        assert!(data.last_update.is_none());
        assert!(data.values.is_empty());
    }

    #[test]
    fn test_graph_data_default() {
        let data = GraphData::default();
        assert!(data.last_update.is_none());
        assert!(data.values.is_empty());
    }

    #[test]
    fn test_graph_data_record_first_value() {
        let mut data = GraphData::new();
        let recorded = data.record(0.5, 100);

        assert!(recorded);
        assert!(data.last_update.is_some());
        assert_eq!(data.values.len(), 1);
        assert_eq!(data.values[0], 0.5);
    }

    #[test]
    fn test_graph_data_record_clamps_values() {
        let mut data = GraphData::new();

        // Values should be clamped to 0.0-1.0
        data.record(1.5, 0); // Above 1.0
        assert_eq!(data.values[0], 1.0);

        data.record(-0.5, 0); // Below 0.0
        assert_eq!(data.values[1], 0.0);

        data.record(0.75, 0); // Normal value
        assert_eq!(data.values[2], 0.75);
    }

    #[test]
    fn test_graph_data_record_respects_delay() {
        let mut data = GraphData::new();

        // First record should succeed
        assert!(data.record(0.5, 1000)); // 1000ms delay

        // Immediate second record should fail (within delay)
        assert!(!data.record(0.6, 1000));

        // Values should still have only one entry
        assert_eq!(data.values.len(), 1);
        assert_eq!(data.values[0], 0.5);
    }

    #[test]
    fn test_graph_data_record_after_delay() {
        let mut data = GraphData::new();

        // First record
        assert!(data.record(0.5, 10)); // 10ms delay

        // Wait for delay
        thread::sleep(Duration::from_millis(15));

        // Second record should succeed
        assert!(data.record(0.6, 10));
        assert_eq!(data.values.len(), 2);
        assert_eq!(data.values[1], 0.6);
    }

    #[test]
    fn test_graph_data_record_zero_delay() {
        let mut data = GraphData::new();

        // With zero delay, all records should succeed
        assert!(data.record(0.1, 0));
        assert!(data.record(0.2, 0));
        assert!(data.record(0.3, 0));

        assert_eq!(data.values.len(), 3);
    }

    #[test]
    fn test_graph_data_record_max_values() {
        let mut data = GraphData::new();

        // Fill to max capacity
        for i in 0..MAX_GRAPH_DATA_VALUES {
            data.record(i as f64 / MAX_GRAPH_DATA_VALUES as f64, 0);
        }
        assert_eq!(data.values.len(), MAX_GRAPH_DATA_VALUES);

        // Add one more - should shift and maintain max size
        data.record(1.0, 0);
        assert_eq!(data.values.len(), MAX_GRAPH_DATA_VALUES);

        // Last value should be the new one
        assert_eq!(*data.values.last().unwrap(), 1.0);

        // First value should have shifted (no longer 0.0)
        assert!(data.values[0] > 0.0);
    }

    #[test]
    fn test_graph_data_ensure_capacity_empty() {
        let mut data = GraphData::new();

        data.ensure_capacity(10);

        // Should have 10 * 2 = 20 values (2 per column)
        assert_eq!(data.values.len(), 20);
        // All should be zeros
        assert!(data.values.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_graph_data_ensure_capacity_partial() {
        let mut data = GraphData::new();
        data.record(0.5, 0);
        data.record(0.6, 0);
        data.record(0.7, 0);

        data.ensure_capacity(5); // Need 10 values

        assert_eq!(data.values.len(), 10);
        // First 7 should be zeros, last 3 should be our values
        assert_eq!(data.values[7], 0.5);
        assert_eq!(data.values[8], 0.6);
        assert_eq!(data.values[9], 0.7);
    }

    #[test]
    fn test_graph_data_ensure_capacity_already_sufficient() {
        let mut data = GraphData::new();
        for _ in 0..30 {
            data.record(0.5, 0);
        }

        data.ensure_capacity(10); // Need 20, have 30

        // Should not change
        assert_eq!(data.values.len(), 30);
    }

    #[test]
    fn test_graph_data_clone() {
        let mut data = GraphData::new();
        data.record(0.5, 0);
        data.record(0.6, 0);

        let cloned = data.clone();

        assert_eq!(cloned.values.len(), 2);
        assert_eq!(cloned.values[0], 0.5);
        assert_eq!(cloned.values[1], 0.6);
    }

    #[test]
    fn test_graph_data_record_raw_stores_values() {
        let mut data = GraphData::new();

        // record_raw should store values as-is (not clamped to 0.0-1.0)
        assert!(data.record_raw(100.0, 0));
        assert!(data.record_raw(200.0, 0));
        assert!(data.record_raw(150.0, 0));

        assert_eq!(data.values.len(), 3);
        assert_eq!(data.values[0], 100.0);
        assert_eq!(data.values[1], 200.0);
        assert_eq!(data.values[2], 150.0);
    }

    #[test]
    fn test_graph_data_record_raw_clamps_negative() {
        let mut data = GraphData::new();

        // Negative values should be clamped to 0
        data.record_raw(-10.0, 0);
        assert_eq!(data.values[0], 0.0);
    }

    #[test]
    fn test_graph_data_max_value_empty() {
        let data = GraphData::new();

        // Empty data should return 1.0 (minimum to avoid division by zero)
        assert_eq!(data.max_value(), 1.0);
    }

    #[test]
    fn test_graph_data_max_value_small_values() {
        let mut data = GraphData::new();
        data.record_raw(0.5, 0);
        data.record_raw(0.3, 0);

        // Max is 1.0 (minimum) even when all values are < 1.0
        assert_eq!(data.max_value(), 1.0);
    }

    #[test]
    fn test_graph_data_max_value_large_values() {
        let mut data = GraphData::new();
        data.record_raw(100.0, 0);
        data.record_raw(300.0, 0);
        data.record_raw(200.0, 0);

        // Max should be the largest value
        assert_eq!(data.max_value(), 300.0);
    }

    // =========================================================================
    // LoadStackedGraphData tests
    // =========================================================================

    #[test]
    fn test_load_stacked_graph_data_new() {
        let data = LoadStackedGraphData::new();
        assert!(data.last_update.is_none());
        assert!(data.values.is_empty());
    }

    #[test]
    fn test_load_stacked_graph_data_record() {
        let mut data = LoadStackedGraphData::new();
        let segments = [1.5, 2.0, 1.8];
        let recorded = data.record(segments, 0);

        assert!(recorded);
        assert!(data.last_update.is_some());
        assert_eq!(data.values.len(), 1);
        assert_eq!(data.values[0], [1.5, 2.0, 1.8]);
    }

    #[test]
    fn test_load_stacked_graph_data_max_sum_empty() {
        let data = LoadStackedGraphData::new();
        // Empty data should return 1.0 (minimum)
        assert_eq!(data.max_sum(), 1.0);
    }

    #[test]
    fn test_load_stacked_graph_data_max_sum() {
        let mut data = LoadStackedGraphData::new();
        data.record([1.0, 2.0, 1.0], 0); // sum = 4.0
        data.record([2.0, 3.0, 2.0], 0); // sum = 7.0
        data.record([0.5, 0.5, 0.5], 0); // sum = 1.5

        assert_eq!(data.max_sum(), 7.0);
    }

    // =========================================================================
    // TasksStackedGraphData tests
    // =========================================================================

    #[test]
    fn test_tasks_stacked_graph_data_new() {
        let data = TasksStackedGraphData::new();
        assert!(data.last_update.is_none());
        assert!(data.values.is_empty());
    }

    #[test]
    fn test_tasks_stacked_graph_data_record() {
        let mut data = TasksStackedGraphData::new();
        let segments = [100.0, 50.0, 150.0, 5.0];
        let recorded = data.record(segments, 0);

        assert!(recorded);
        assert!(data.last_update.is_some());
        assert_eq!(data.values.len(), 1);
        assert_eq!(data.values[0], [100.0, 50.0, 150.0, 5.0]);
    }

    #[test]
    fn test_tasks_stacked_graph_data_max_sum_empty() {
        let data = TasksStackedGraphData::new();
        // Empty data should return 1.0 (minimum)
        assert_eq!(data.max_sum(), 1.0);
    }

    #[test]
    fn test_tasks_stacked_graph_data_max_sum() {
        let mut data = TasksStackedGraphData::new();
        data.record([100.0, 50.0, 150.0, 5.0], 0); // sum = 305
        data.record([120.0, 60.0, 180.0, 8.0], 0); // sum = 368
        data.record([80.0, 40.0, 120.0, 3.0], 0); // sum = 243

        assert_eq!(data.max_sum(), 368.0);
    }

    // =========================================================================
    // MeterType::create_from_name tests
    // =========================================================================

    #[test]
    fn test_meter_type_create_cpu() {
        let meter = MeterType::create_from_name("CPU", 0);
        assert!(meter.is_some());
        assert_eq!(meter.unwrap().name(), "CPU");
    }

    #[test]
    fn test_meter_type_create_all_cpus() {
        let meter = MeterType::create_from_name("AllCPUs", 0);
        assert!(meter.is_some());

        let meter2 = MeterType::create_from_name("AllCPUs2", 0);
        assert!(meter2.is_some());

        let meter4 = MeterType::create_from_name("AllCPUs4", 0);
        assert!(meter4.is_some());

        let meter8 = MeterType::create_from_name("AllCPUs8", 0);
        assert!(meter8.is_some());
    }

    #[test]
    fn test_meter_type_create_left_right_cpus() {
        let left = MeterType::create_from_name("LeftCPUs", 0);
        assert!(left.is_some());

        let right = MeterType::create_from_name("RightCPUs", 0);
        assert!(right.is_some());

        let left2 = MeterType::create_from_name("LeftCPUs2", 0);
        assert!(left2.is_some());

        let right4 = MeterType::create_from_name("RightCPUs4", 0);
        assert!(right4.is_some());
    }

    #[test]
    fn test_meter_type_create_memory() {
        let meter = MeterType::create_from_name("Memory", 0);
        assert!(meter.is_some());
        assert_eq!(meter.unwrap().name(), "Memory");
    }

    #[test]
    fn test_meter_type_create_swap() {
        let meter = MeterType::create_from_name("Swap", 0);
        assert!(meter.is_some());
        assert_eq!(meter.unwrap().name(), "Swap");
    }

    #[test]
    fn test_meter_type_create_load() {
        let load_avg = MeterType::create_from_name("LoadAverage", 0);
        assert!(load_avg.is_some());

        let load = MeterType::create_from_name("Load", 0);
        assert!(load.is_some());
    }

    #[test]
    fn test_meter_type_create_tasks() {
        let meter = MeterType::create_from_name("Tasks", 0);
        assert!(meter.is_some());
        assert_eq!(meter.unwrap().name(), "Tasks");
    }

    #[test]
    fn test_meter_type_create_uptime() {
        let meter = MeterType::create_from_name("Uptime", 0);
        assert!(meter.is_some());
        assert_eq!(meter.unwrap().name(), "Uptime");
    }

    #[test]
    fn test_meter_type_create_clock_date() {
        let clock = MeterType::create_from_name("Clock", 0);
        assert!(clock.is_some());

        let date = MeterType::create_from_name("Date", 0);
        assert!(date.is_some());

        let datetime = MeterType::create_from_name("DateTime", 0);
        assert!(datetime.is_some());
    }

    #[test]
    fn test_meter_type_create_hostname() {
        let meter = MeterType::create_from_name("Hostname", 0);
        assert!(meter.is_some());
        assert_eq!(meter.unwrap().name(), "Hostname");
    }

    #[test]
    fn test_meter_type_create_blank() {
        let meter = MeterType::create_from_name("Blank", 0);
        assert!(meter.is_some());
        assert_eq!(meter.unwrap().name(), "Blank");
    }

    #[test]
    fn test_meter_type_create_io() {
        let disk = MeterType::create_from_name("DiskIO", 0);
        assert!(disk.is_some());

        let network = MeterType::create_from_name("NetworkIO", 0);
        assert!(network.is_some());
    }

    #[test]
    fn test_meter_type_create_battery() {
        let meter = MeterType::create_from_name("Battery", 0);
        assert!(meter.is_some());
        assert_eq!(meter.unwrap().name(), "Battery");
    }

    #[test]
    fn test_meter_type_create_unknown() {
        let meter = MeterType::create_from_name("UnknownMeter", 0);
        assert!(meter.is_none());

        let meter2 = MeterType::create_from_name("", 0);
        assert!(meter2.is_none());
    }

    #[test]
    fn test_meter_type_create_case_sensitive() {
        // Names should be case-sensitive
        let cpu = MeterType::create_from_name("cpu", 0);
        assert!(cpu.is_none());

        let memory = MeterType::create_from_name("memory", 0);
        assert!(memory.is_none());
    }
}
