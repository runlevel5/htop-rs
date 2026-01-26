//! Meter registry for the setup screen
//!
//! Contains meter type information and platform-specific meter lists

use crate::core::MeterMode;

/// Available meter information
#[derive(Debug, Clone)]
pub struct MeterInfo {
    /// Internal name used in settings (e.g., "CPU", "Memory")
    pub name: &'static str,
    /// Display name shown in UI
    pub display_name: &'static str,
    /// Description for the available meters panel
    pub description: &'static str,
    /// Whether this meter type supports a parameter (e.g., CPU number)
    pub supports_param: bool,
}

impl MeterInfo {
    const fn new(
        name: &'static str,
        display_name: &'static str,
        description: &'static str,
    ) -> Self {
        MeterInfo {
            name,
            display_name,
            description,
            supports_param: false,
        }
    }

    const fn with_param(
        name: &'static str,
        display_name: &'static str,
        description: &'static str,
    ) -> Self {
        MeterInfo {
            name,
            display_name,
            description,
            supports_param: true,
        }
    }
}

/// Common meters available on all platforms
/// Order matches C htop darwin/Platform.c Platform_meterTypes[]
const COMMON_METERS: &[MeterInfo] = &[
    MeterInfo::with_param("CPU", "CPU", "CPU average"),
    MeterInfo::new("Clock", "Clock", "Clock"),
    MeterInfo::new("Date", "Date", "Date"),
    MeterInfo::new("DateTime", "Date and Time", "Date and Time"),
    MeterInfo::new(
        "LoadAverage",
        "Load average",
        "Load averages: 1 minute, 5 minutes, 15 minutes",
    ),
    MeterInfo::new(
        "Load",
        "Load",
        "Load: average of ready processes in the last minute",
    ),
    MeterInfo::new("Memory", "Memory", "Memory"),
    MeterInfo::new("Swap", "Swap", "Swap"),
    MeterInfo::new(
        "MemorySwap",
        "Memory & Swap",
        "Combined memory and swap usage",
    ),
    MeterInfo::new("Tasks", "Task counter", "Task counter"),
    MeterInfo::new("Battery", "Battery", "Battery"),
    MeterInfo::new("Hostname", "Hostname", "Hostname"),
    MeterInfo::new("System", "System", "System"),
    MeterInfo::new("Uptime", "Uptime", "Uptime"),
    // AllCPUs variants
    MeterInfo::new("AllCPUs", "CPUs (1/1)", "CPUs (1/1): all CPUs"),
    MeterInfo::new(
        "AllCPUs2",
        "CPUs (1&2/2)",
        "CPUs (1&2/2): all CPUs in 2 shorter columns",
    ),
    MeterInfo::new(
        "AllCPUs4",
        "CPUs (1&2&3&4/4)",
        "CPUs (1&2&3&4/4): all CPUs in 4 shorter columns",
    ),
    MeterInfo::new(
        "AllCPUs8",
        "CPUs (1-8/8)",
        "CPUs (1-8/8): all CPUs in 8 shorter columns",
    ),
    // Left/Right CPUs variants
    MeterInfo::new("LeftCPUs", "CPUs (1/2)", "CPUs (1/2): first half of list"),
    MeterInfo::new("RightCPUs", "CPUs (2/2)", "CPUs (2/2): second half of list"),
    MeterInfo::new(
        "LeftCPUs2",
        "CPUs (1&2/4)",
        "CPUs (1&2/4): first half in 2 shorter columns",
    ),
    MeterInfo::new(
        "RightCPUs2",
        "CPUs (3&4/4)",
        "CPUs (3&4/4): second half in 2 shorter columns",
    ),
    MeterInfo::new(
        "LeftCPUs4",
        "CPUs (1-4/8)",
        "CPUs (1-4/8): first half in 4 shorter columns",
    ),
    MeterInfo::new(
        "RightCPUs4",
        "CPUs (5-8/8)",
        "CPUs (5-8/8): second half in 4 shorter columns",
    ),
    MeterInfo::new(
        "LeftCPUs8",
        "CPUs (1-8/16)",
        "CPUs (1-8/16): first half in 8 shorter columns",
    ),
    MeterInfo::new(
        "RightCPUs8",
        "CPUs (9-16/16)",
        "CPUs (9-16/16): second half in 8 shorter columns",
    ),
];

/// ZFS meters - available on Linux, macOS, FreeBSD
/// Position in list matches C htop (after CPU variants, before DiskIO)
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))]
const ZFS_METERS: &[MeterInfo] = &[
    MeterInfo::new("ZFSARC", "ZFS ARC", "ZFS ARC"),
    MeterInfo::new("ZFSCARC", "ZFS CARC", "ZFS CARC: Compressed ARC statistics"),
];

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
const ZFS_METERS: &[MeterInfo] = &[];

/// DiskIO and NetworkIO meters - available on all platforms
/// Position in list matches C htop darwin (after ZFS, before FileDescriptor)
const DISKIO_NETWORK_METERS: &[MeterInfo] = &[
    MeterInfo::new(
        "DiskIORate",
        "Disk IO Rate",
        "Disk IO read & write bytes per second",
    ),
    MeterInfo::new("DiskIOTime", "Disk IO Time", "Disk percent time busy"),
    MeterInfo::new("DiskIO", "Disk IO", "Disk IO"),
    MeterInfo::new("NetworkIO", "Network IO", "Network IO"),
    MeterInfo::new(
        "FileDescriptors",
        "File Descriptors",
        "Number of allocated/available file descriptors",
    ),
];

/// GPU meter - available on Linux and macOS (via IOKit on macOS, various backends on Linux)
#[cfg(any(target_os = "linux", target_os = "macos"))]
const GPU_METERS: &[MeterInfo] = &[MeterInfo::new("GPU", "GPU", "GPU")];

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const GPU_METERS: &[MeterInfo] = &[];

/// Blank meter - available on all platforms (last in list like C htop)
const BLANK_METER: &[MeterInfo] = &[MeterInfo::new("Blank", "Blank", "Blank")];

/// Linux-specific meters (inserted at appropriate position for Linux builds)
#[cfg(target_os = "linux")]
const LINUX_METERS: &[MeterInfo] = &[
    MeterInfo::new("HugePages", "HugePages", "HugePages"),
    MeterInfo::new(
        "PressureStallCPUSome",
        "PSI some CPU",
        "Pressure Stall Information, some cpu",
    ),
    MeterInfo::new(
        "PressureStallIOSome",
        "PSI some IO",
        "Pressure Stall Information, some io",
    ),
    MeterInfo::new(
        "PressureStallIOFull",
        "PSI full IO",
        "Pressure Stall Information, full io",
    ),
    MeterInfo::new(
        "PressureStallIRQFull",
        "PSI full IRQ",
        "Pressure Stall Information, full irq",
    ),
    MeterInfo::new(
        "PressureStallMemorySome",
        "PSI some memory",
        "Pressure Stall Information, some memory",
    ),
    MeterInfo::new(
        "PressureStallMemoryFull",
        "PSI full memory",
        "Pressure Stall Information, full memory",
    ),
    MeterInfo::new("Zram", "Zram", "Zram"),
    MeterInfo::new("SELinux", "SELinux", "SELinux state overview"),
    MeterInfo::new(
        "Systemd",
        "Systemd state",
        "Systemd system state and unit overview",
    ),
    MeterInfo::new(
        "SystemdUser",
        "Systemd user state",
        "Systemd user state and unit overview",
    ),
];

#[cfg(not(target_os = "linux"))]
const LINUX_METERS: &[MeterInfo] = &[];

/// Get available meters for the current platform
/// Order matches C htop darwin/Platform.c Platform_meterTypes[] for macOS
pub fn available_meters_for_platform() -> Vec<&'static MeterInfo> {
    let mut meters: Vec<&'static MeterInfo> = Vec::new();

    // Add common meters (CPU through RightCPUs8)
    for m in COMMON_METERS {
        meters.push(m);
    }

    // Add ZFS meters (after CPU variants)
    for m in ZFS_METERS {
        meters.push(m);
    }

    // Add DiskIO/NetworkIO/FileDescriptors meters
    for m in DISKIO_NETWORK_METERS {
        meters.push(m);
    }

    // Add GPU meter (Linux and macOS only)
    for m in GPU_METERS {
        meters.push(m);
    }

    // Add Blank meter (always last)
    for m in BLANK_METER {
        meters.push(m);
    }

    // Note: Linux-specific meters are not added here to match macOS order
    // For Linux, we would need a different ordering function
    // For now, Linux meters are appended after Blank for simplicity
    for m in LINUX_METERS {
        meters.push(m);
    }

    meters
}

/// Get the display name for a meter by its internal name (used in meter columns)
/// This matches C htop's uiName field
pub fn meter_display_name(name: &str, mode: MeterMode) -> String {
    let base_name = match name {
        "CPU" => "CPU",
        "AllCPUs" => "CPUs (1/1)",
        "AllCPUs2" => "CPUs (1&2/2)",
        "AllCPUs4" => "CPUs (1&2&3&4/4)",
        "AllCPUs8" => "CPUs (1-8/8)",
        "LeftCPUs" => "CPUs (1/2)",
        "LeftCPUs2" => "CPUs (1&2/4)",
        "LeftCPUs4" => "CPUs (1-4/8)",
        "LeftCPUs8" => "CPUs (1-8/16)",
        "RightCPUs" => "CPUs (2/2)",
        "RightCPUs2" => "CPUs (3&4/4)",
        "RightCPUs4" => "CPUs (5-8/8)",
        "RightCPUs8" => "CPUs (9-16/16)",
        "Memory" => "Memory",
        "MemorySwap" => "Memory & Swap",
        "Swap" => "Swap",
        "System" => "System",
        "LoadAverage" => "Load average",
        "Load" => "Load",
        "Tasks" => "Task counter",
        "Uptime" => "Uptime",
        "Battery" => "Battery",
        "Hostname" => "Hostname",
        "Clock" => "Clock",
        "Date" => "Date",
        "DateTime" => "Date and Time",
        "DiskIO" => "Disk IO",
        "DiskIORate" => "Disk IO Rate",
        "DiskIOTime" => "Disk IO Time",
        "NetworkIO" => "Network IO",
        "FileDescriptors" => "File Descriptors",
        "Blank" => "Blank",
        // Linux-specific
        "HugePages" => "HugePages",
        "PressureStallCPUSome" => "PSI some CPU",
        "PressureStallIOSome" => "PSI some IO",
        "PressureStallIOFull" => "PSI full IO",
        "PressureStallIRQFull" => "PSI full IRQ",
        "PressureStallMemorySome" => "PSI some memory",
        "PressureStallMemoryFull" => "PSI full memory",
        "Zram" => "Zram",
        "SELinux" => "SELinux",
        "Systemd" => "Systemd state",
        "SystemdUser" => "Systemd user state",
        // ZFS
        "ZFSARC" => "ZFS ARC",
        "ZFSCARC" => "ZFS CARC",
        // GPU
        "GPU" => "GPU",
        _ => name,
    };

    // Add mode suffix like C htop (e.g., "[Bar]", "[Text]", etc.)
    let mode_str = match mode {
        MeterMode::Bar => "[Bar]",
        MeterMode::Text => "[Text]",
        MeterMode::Graph => "[Graph]",
        MeterMode::Led => "[LED]",
        MeterMode::StackedGraph => "[Stacked]",
    };

    format!("{} {}", base_name, mode_str)
}
