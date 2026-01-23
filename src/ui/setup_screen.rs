//! Setup Screen - F2 configuration screen
//!
//! This module implements the htop Setup screen with:
//! - Categories panel on the left (Display options, Header layout, Meters, Screens, Colors)
//! - Content panel(s) on the right depending on selected category

#![allow(dead_code)]

use ncurses::*;

use super::crt::{ColorElement, KEY_F10, KEY_F2, KEY_F4, KEY_F5, KEY_F6, KEY_F7, KEY_F8, KEY_F9};
use super::function_bar::FunctionBar;
use super::header::Header;
use super::panel::{HandlerResult, Panel};
use super::rich_string::RichString;
use super::Crt;
use crate::core::{
    ColorScheme, HeaderLayout, Machine, MeterConfig, MeterMode, ProcessField, ScreenSettings,
    Settings,
};
use crate::platform;

// Key constants for pattern matching
const KEY_ESC: i32 = 27;
const KEY_TAB: i32 = b'\t' as i32;
const KEY_ENTER_LF: i32 = b'\n' as i32;
const KEY_ENTER_CR: i32 = b'\r' as i32;
const KEY_SPACE: i32 = b' ' as i32;
const KEY_MINUS: i32 = b'-' as i32;
const KEY_PLUS: i32 = b'+' as i32;
const KEY_Q: i32 = b'q' as i32;
const KEY_T: i32 = b't' as i32;
const KEY_L: i32 = b'l' as i32;
const KEY_L_UPPER: i32 = b'L' as i32;
const KEY_R: i32 = b'r' as i32;
const KEY_R_UPPER: i32 = b'R' as i32;
const KEY_LBRACKET: i32 = b'[' as i32;
const KEY_RBRACKET: i32 = b']' as i32;
const KEY_BACKSPACE_ASCII: i32 = 127;
const KEY_CTRL_R: i32 = 18; // Ctrl+R
const KEY_CTRL_N: i32 = 14; // Ctrl+N

/// Maximum length for screen names (matching C htop SCREEN_NAME_LEN)
const SCREEN_NAME_LEN: usize = 20;

/// Setup screen categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCategory {
    DisplayOptions,
    HeaderLayout,
    Meters,
    Screens,
    Colors,
}

impl SetupCategory {
    pub fn all() -> &'static [SetupCategory] {
        &[
            SetupCategory::DisplayOptions,
            SetupCategory::HeaderLayout,
            SetupCategory::Meters,
            SetupCategory::Screens,
            SetupCategory::Colors,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SetupCategory::DisplayOptions => "Display options",
            SetupCategory::HeaderLayout => "Header layout",
            SetupCategory::Meters => "Meters",
            SetupCategory::Screens => "Screens",
            SetupCategory::Colors => "Colors",
        }
    }
}

/// Option item types for the display options panel
#[derive(Debug, Clone)]
pub enum OptionItem {
    /// Section header (non-interactive)
    Text(String),
    /// Boolean checkbox
    Check {
        text: String,
        /// Field identifier for the setting
        field: SettingField,
    },
    /// Numeric value with inc/dec
    Number {
        text: String,
        field: SettingField,
        min: i32,
        max: i32,
        scale: i32, // negative for decimal places
    },
}

impl OptionItem {
    pub fn text(s: &str) -> Self {
        OptionItem::Text(s.to_string())
    }

    pub fn check(text: &str, field: SettingField) -> Self {
        OptionItem::Check {
            text: text.to_string(),
            field,
        }
    }

    pub fn number(text: &str, field: SettingField, min: i32, max: i32) -> Self {
        OptionItem::Number {
            text: text.to_string(),
            field,
            min,
            max,
            scale: 0,
        }
    }

    pub fn number_scaled(text: &str, field: SettingField, min: i32, max: i32, scale: i32) -> Self {
        OptionItem::Number {
            text: text.to_string(),
            field,
            min,
            max,
            scale,
        }
    }

    pub fn is_interactive(&self) -> bool {
        !matches!(self, OptionItem::Text(_))
    }

    pub fn display(&self, settings: &Settings, crt: &Crt) -> RichString {
        let mut str = RichString::new();
        let box_color = crt.color(ColorElement::CheckBox);
        let mark_color = crt.color(ColorElement::CheckMark);
        let text_color = crt.color(ColorElement::CheckText);
        let bold_color = crt.color(ColorElement::HelpBold);

        match self {
            OptionItem::Text(text) => {
                str.append(text, bold_color);
            }
            OptionItem::Check { text, field } => {
                let checked = field.get_bool(settings);
                str.append("[", box_color);
                str.append(if checked { "x" } else { " " }, mark_color);
                str.append("]    ", box_color);
                str.append(text, text_color);
            }
            OptionItem::Number {
                text, field, scale, ..
            } => {
                let value = field.get_int(settings);
                str.append("[", box_color);

                let value_str = if *scale < 0 {
                    // Decimal format
                    let factor = 10f64.powi(-scale);
                    format!("{:.1}", value as f64 / factor)
                } else {
                    format!("{}", value)
                };
                str.append(&value_str, mark_color);
                str.append("]", box_color);

                // Pad to 5 chars for alignment
                let padding = 5usize.saturating_sub(value_str.len());
                for _ in 0..padding {
                    str.append(" ", box_color);
                }
                str.append(text, text_color);
            }
        }
        str
    }
}

/// Identifiers for settings fields
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingField {
    // Tree view (per-screen)
    TreeView,
    TreeViewAlwaysByPid,
    AllBranchesCollapsed,
    // Global display options
    ScreenTabs,
    ShadowOtherUsers,
    HideKernelThreads,
    HideUserlandThreads,
    HideRunningInContainer,
    HighlightThreads,
    ShowThreadNames,
    ShowProgramPath,
    HighlightBaseName,
    HighlightDeletedExe,
    ShadowDistPathPrefix,
    ShowMergedCommand,
    FindCommInCmdline,
    StripExeFromCmdline,
    HighlightMegabytes,
    HeaderMargin,
    DetailedCpuTime,
    CountCpusFromOne,
    UpdateProcessNames,
    AccountGuestInCpuMeter,
    ShowCpuUsage,
    ShowCpuFrequency,
    ShowCachedMemory,
    EnableMouse,
    Delay,
    HighlightChanges,
    HighlightDelaySecs,
    HideFunctionBar,
    // Color scheme
    ColorScheme,
}

impl SettingField {
    pub fn get_bool(&self, settings: &Settings) -> bool {
        match self {
            SettingField::TreeView => settings.tree_view,
            SettingField::TreeViewAlwaysByPid => settings.tree_view_always_by_pid,
            SettingField::AllBranchesCollapsed => settings.all_branches_collapsed,
            SettingField::ScreenTabs => settings.screen_tabs,
            SettingField::ShadowOtherUsers => settings.shadow_other_users,
            SettingField::HideKernelThreads => settings.hide_kernel_threads,
            SettingField::HideUserlandThreads => settings.hide_userland_threads,
            SettingField::HideRunningInContainer => settings.hide_running_in_container,
            SettingField::HighlightThreads => settings.highlight_threads,
            SettingField::ShowThreadNames => settings.show_thread_names,
            SettingField::ShowProgramPath => settings.show_program_path,
            SettingField::HighlightBaseName => settings.highlight_base_name,
            SettingField::HighlightDeletedExe => settings.highlight_deleted_exe,
            SettingField::ShadowDistPathPrefix => settings.shadow_dist_path_prefix,
            SettingField::ShowMergedCommand => settings.show_merged_command,
            SettingField::FindCommInCmdline => settings.find_comm_in_cmdline,
            SettingField::StripExeFromCmdline => settings.strip_exe_from_cmdline,
            SettingField::HighlightMegabytes => settings.highlight_megabytes,
            SettingField::HeaderMargin => settings.header_margin,
            SettingField::DetailedCpuTime => settings.detailed_cpu_time,
            SettingField::CountCpusFromOne => settings.count_cpus_from_one,
            SettingField::UpdateProcessNames => settings.update_process_names,
            SettingField::AccountGuestInCpuMeter => settings.account_guest_in_cpu_meter,
            SettingField::ShowCpuUsage => settings.show_cpu_usage,
            SettingField::ShowCpuFrequency => settings.show_cpu_frequency,
            SettingField::ShowCachedMemory => settings.show_cached_memory,
            SettingField::EnableMouse => settings.enable_mouse,
            SettingField::HighlightChanges => settings.highlight_changes,
            _ => false,
        }
    }

    pub fn set_bool(&self, settings: &mut Settings, value: bool) {
        match self {
            SettingField::TreeView => settings.tree_view = value,
            SettingField::TreeViewAlwaysByPid => settings.tree_view_always_by_pid = value,
            SettingField::AllBranchesCollapsed => settings.all_branches_collapsed = value,
            SettingField::ScreenTabs => settings.screen_tabs = value,
            SettingField::ShadowOtherUsers => settings.shadow_other_users = value,
            SettingField::HideKernelThreads => settings.hide_kernel_threads = value,
            SettingField::HideUserlandThreads => settings.hide_userland_threads = value,
            SettingField::HideRunningInContainer => settings.hide_running_in_container = value,
            SettingField::HighlightThreads => settings.highlight_threads = value,
            SettingField::ShowThreadNames => settings.show_thread_names = value,
            SettingField::ShowProgramPath => settings.show_program_path = value,
            SettingField::HighlightBaseName => settings.highlight_base_name = value,
            SettingField::HighlightDeletedExe => settings.highlight_deleted_exe = value,
            SettingField::ShadowDistPathPrefix => settings.shadow_dist_path_prefix = value,
            SettingField::ShowMergedCommand => settings.show_merged_command = value,
            SettingField::FindCommInCmdline => settings.find_comm_in_cmdline = value,
            SettingField::StripExeFromCmdline => settings.strip_exe_from_cmdline = value,
            SettingField::HighlightMegabytes => settings.highlight_megabytes = value,
            SettingField::HeaderMargin => settings.header_margin = value,
            SettingField::DetailedCpuTime => settings.detailed_cpu_time = value,
            SettingField::CountCpusFromOne => settings.count_cpus_from_one = value,
            SettingField::UpdateProcessNames => settings.update_process_names = value,
            SettingField::AccountGuestInCpuMeter => settings.account_guest_in_cpu_meter = value,
            SettingField::ShowCpuUsage => settings.show_cpu_usage = value,
            SettingField::ShowCpuFrequency => settings.show_cpu_frequency = value,
            SettingField::ShowCachedMemory => settings.show_cached_memory = value,
            SettingField::EnableMouse => settings.enable_mouse = value,
            SettingField::HighlightChanges => settings.highlight_changes = value,
            _ => {}
        }
    }

    pub fn toggle(&self, settings: &mut Settings) {
        let current = self.get_bool(settings);
        self.set_bool(settings, !current);
    }

    pub fn get_int(&self, settings: &Settings) -> i32 {
        match self {
            SettingField::Delay => settings.delay as i32,
            SettingField::HighlightDelaySecs => settings.highlight_delay_secs,
            SettingField::HideFunctionBar => settings.hide_function_bar,
            SettingField::ColorScheme => settings.color_scheme as i32,
            _ => 0,
        }
    }

    pub fn set_int(&self, settings: &mut Settings, value: i32) {
        match self {
            SettingField::Delay => settings.delay = value as u32,
            SettingField::HighlightDelaySecs => settings.highlight_delay_secs = value,
            SettingField::HideFunctionBar => settings.hide_function_bar = value,
            SettingField::ColorScheme => settings.color_scheme = ColorScheme::from_i32(value),
            _ => {}
        }
    }
}

/// Color scheme names matching C htop
pub const COLOR_SCHEME_NAMES: &[&str] = &[
    "Default",
    "Monochromatic",
    "Black on White",
    "Light Terminal",
    "MC",
    "Black Night",
    "Broken Gray",
    "Nord",
];

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
    MeterInfo::new("LoadAverage", "Load average", "Load averages: 1 minute, 5 minutes, 15 minutes"),
    MeterInfo::new("Load", "Load", "Load: average of ready processes in the last minute"),
    MeterInfo::new("Memory", "Memory", "Memory"),
    MeterInfo::new("Swap", "Swap", "Swap"),
    MeterInfo::new("MemorySwap", "Memory & Swap", "Memory & Swap"),
    MeterInfo::new("Tasks", "Task counter", "Task counter"),
    MeterInfo::new("Battery", "Battery", "Battery"),
    MeterInfo::new("Hostname", "Hostname", "Hostname"),
    MeterInfo::new("System", "System", "System"),
    MeterInfo::new("Uptime", "Uptime", "Uptime"),
    // AllCPUs variants
    MeterInfo::new("AllCPUs", "CPUs (1/1)", "CPUs (1/1): all CPUs"),
    MeterInfo::new("AllCPUs2", "CPUs (1&2/2)", "CPUs (1&2/2): all CPUs in 2 shorter columns"),
    MeterInfo::new("AllCPUs4", "CPUs (1&2&3&4/4)", "CPUs (1&2&3&4/4): all CPUs in 4 shorter columns"),
    MeterInfo::new("AllCPUs8", "CPUs (1-8/8)", "CPUs (1-8/8): all CPUs in 8 shorter columns"),
    // Left/Right CPUs variants
    MeterInfo::new("LeftCPUs", "CPUs (1/2)", "CPUs (1/2): first half of list"),
    MeterInfo::new("RightCPUs", "CPUs (2/2)", "CPUs (2/2): second half of list"),
    MeterInfo::new("LeftCPUs2", "CPUs (1&2/4)", "CPUs (1&2/4): first half in 2 shorter columns"),
    MeterInfo::new("RightCPUs2", "CPUs (3&4/4)", "CPUs (3&4/4): second half in 2 shorter columns"),
    MeterInfo::new("LeftCPUs4", "CPUs (1-4/8)", "CPUs (1-4/8): first half in 4 shorter columns"),
    MeterInfo::new("RightCPUs4", "CPUs (5-8/8)", "CPUs (5-8/8): second half in 4 shorter columns"),
    MeterInfo::new("LeftCPUs8", "CPUs (1-8/16)", "CPUs (1-8/16): first half in 8 shorter columns"),
    MeterInfo::new("RightCPUs8", "CPUs (9-16/16)", "CPUs (9-16/16): second half in 8 shorter columns"),
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
    MeterInfo::new("DiskIORate", "Disk IO Rate", "Disk IO read & write bytes per second"),
    MeterInfo::new("DiskIOTime", "Disk IO Time", "Disk percent time busy"),
    MeterInfo::new("DiskIO", "Disk IO", "Disk IO"),
    MeterInfo::new("NetworkIO", "Network IO", "Network IO"),
    MeterInfo::new("FileDescriptors", "File Descriptors", "Number of allocated/available file descriptors"),
];

/// GPU meter - available on Linux and macOS (via IOKit on macOS, various backends on Linux)
#[cfg(any(target_os = "linux", target_os = "macos"))]
const GPU_METERS: &[MeterInfo] = &[
    MeterInfo::new("GPU", "GPU", "GPU"),
];

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const GPU_METERS: &[MeterInfo] = &[];

/// Blank meter - available on all platforms (last in list like C htop)
const BLANK_METER: &[MeterInfo] = &[
    MeterInfo::new("Blank", "Blank", "Blank"),
];

/// Linux-specific meters (inserted at appropriate position for Linux builds)
#[cfg(target_os = "linux")]
const LINUX_METERS: &[MeterInfo] = &[
    MeterInfo::new("HugePages", "HugePages", "HugePages"),
    MeterInfo::new("PressureStallCPUSome", "PSI some CPU", "Pressure Stall Information, some cpu"),
    MeterInfo::new("PressureStallIOSome", "PSI some IO", "Pressure Stall Information, some io"),
    MeterInfo::new("PressureStallIOFull", "PSI full IO", "Pressure Stall Information, full io"),
    MeterInfo::new("PressureStallIRQFull", "PSI full IRQ", "Pressure Stall Information, full irq"),
    MeterInfo::new("PressureStallMemorySome", "PSI some memory", "Pressure Stall Information, some memory"),
    MeterInfo::new("PressureStallMemoryFull", "PSI full memory", "Pressure Stall Information, full memory"),
    MeterInfo::new("Zram", "Zram", "Zram"),
    MeterInfo::new("SELinux", "SELinux", "SELinux state overview"),
    MeterInfo::new("Systemd", "Systemd state", "Systemd system state and unit overview"),
    MeterInfo::new("SystemdUser", "Systemd user state", "Systemd user state and unit overview"),
];

#[cfg(not(target_os = "linux"))]
const LINUX_METERS: &[MeterInfo] = &[];

/// Get available meters for the current platform
/// Order matches C htop darwin/Platform.c Platform_meterTypes[] for macOS
fn available_meters_for_platform() -> Vec<&'static MeterInfo> {
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

/// Get the display name for a meter by its internal name
/// Get the display name for a meter by its internal name (used in meter columns)
/// This matches C htop's uiName field
fn meter_display_name(name: &str, mode: MeterMode) -> String {
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
        _ => name,
    };

    // Add mode suffix like C htop (e.g., "[Bar]", "[Text]", etc.)
    let mode_str = match mode {
        MeterMode::Bar => "[Bar]",
        MeterMode::Text => "[Text]",
        MeterMode::Graph => "[Graph]",
        MeterMode::Led => "[LED]",
    };

    format!("{} {}", base_name, mode_str)
}

/// Setup screen manager
pub struct SetupScreen {
    /// Current category
    category: SetupCategory,
    /// Category selection index
    category_index: usize,
    /// Categories panel
    categories_panel: Panel,
    /// Content panel
    content_panel: Panel,
    /// Display options items
    display_options: Vec<OptionItem>,
    /// Current content selection
    content_index: usize,
    /// Content scroll
    content_scroll: i32,
    /// Which panel has focus (0 = categories, 1 = content, 2+ = meter columns)
    focus: usize,
    /// Function bar for the setup screen
    function_bar: FunctionBar,
    /// Dec/Inc function bar for number items
    dec_inc_bar: FunctionBar,
    /// Meters function bar
    meters_bar: FunctionBar,
    /// Meters moving mode function bar
    meters_moving_bar: FunctionBar,
    /// Available meters function bar
    meters_available_bar: FunctionBar,
    /// Whether settings were changed
    pub changed: bool,
    // === Meters panel state ===
    /// Current column index for meters panel (0..num_columns-1, then available_meters)
    meters_column_focus: usize,
    /// Selection index within each meter column
    meters_column_selection: Vec<usize>,
    /// Selection index for available meters panel
    meters_available_selection: usize,
    /// Scroll position for available meters panel
    meters_available_scroll: i32,
    /// Whether in moving mode (meter is "grabbed" and can be moved with arrows)
    meters_moving: bool,
    // === Screens panel state ===
    /// Function bar for Screens panel
    screens_bar: FunctionBar,
    /// Function bar for Active Columns panel
    columns_bar: FunctionBar,
    /// Function bar for Available Columns panel
    available_columns_bar: FunctionBar,
    /// Which panel has focus in Screens category (0=screens, 1=columns, 2=available)
    screens_panel_focus: usize,
    /// Selection index for screens list
    screens_selection: usize,
    /// Selection index for active columns
    columns_selection: usize,
    /// Selection index for available columns
    available_columns_selection: usize,
    /// Scroll position for available columns
    available_columns_scroll: i32,
    /// Whether in moving mode for screens list
    screens_moving: bool,
    /// Whether in moving mode for columns list
    columns_moving: bool,
    /// Whether renaming a screen
    screens_renaming: bool,
    /// Buffer for renaming
    screens_rename_buffer: String,
    /// Cursor position in rename buffer
    screens_rename_cursor: usize,
}

impl SetupScreen {
    pub fn new() -> Self {
        let mut categories_panel = Panel::new(0, 0, 16, 10);
        categories_panel.set_header("Categories");

        // Add category items
        for cat in SetupCategory::all() {
            categories_panel.add_text(cat.name());
        }

        let mut content_panel = Panel::new(16, 0, 60, 10);
        content_panel.set_header("Display options");

        // Build display options list
        let display_options = Self::build_display_options();

        // Create function bars
        let function_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Done", "F10"),
        ]);

        let dec_inc_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Dec", "F7"),
            ("Inc", "F8"),
            ("", ""),
            ("Done", "F10"),
        ]);

        // Meters function bar (matching C htop MetersPanel)
        // C htop: {"Style ", "Move  ", "                                       ", "Delete", "Done  "}
        // Keys:   {"Space", "Enter", "  ", "Del", "F10"}
        let meters_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("Style", "Space"),
            ("Move", "Enter"),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Delete", "Del"),
            ("Done", "F10"),
        ]);

        // Meters moving mode function bar (matching C htop MetersMovingFunctions)
        // C htop: {"Style ", "Lock  ", "Up    ", "Down  ", "Left  ", "Right ", "       ", "Delete", "Done  "}
        // Keys:   {"Space", "Enter", "Up", "Dn", "<-", "->", "  ", "Del", "F10"}
        // We use arrow symbols for better visual appearance
        let meters_moving_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("Style", "Space"),
            ("Lock", "Enter"),
            ("Up", "↑"),
            ("Down", "↓"),
            ("Left", "←"),
            ("Right", "→"),
            ("Delete", "Del"),
        ]);

        // Available meters function bar (matching C htop AvailableMetersPanel)
        // C htop uses FunctionBar_newEnterEsc("Add   ", "Done   ")
        let meters_available_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Add", "Enter"),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Done", "Esc"),
        ]);

        // Screens panel function bar (matching C htop ScreensFunctions)
        // C htop: {"      ", "Rename", "      ", "      ", "New   ", "      ", "MoveUp", "MoveDn", "Remove", "Done  "}
        let screens_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("Rename", "F2"),
            ("", ""),
            ("", ""),
            ("New", "F5"),
            ("", ""),
            ("MoveUp", "F7"),
            ("MoveDn", "F8"),
            ("Remove", "F9"),
            ("Done", "F10"),
        ]);

        // Active Columns panel function bar (matching C htop ColumnsFunctions)
        // C htop: {"      ", "      ", "      ", "      ", "      ", "      ", "MoveUp", "MoveDn", "Remove", "Done  "}
        let columns_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("MoveUp", "F7"),
            ("MoveDn", "F8"),
            ("Remove", "F9"),
            ("Done", "F10"),
        ]);

        // Available Columns panel function bar (matching C htop AvailableColumnsFunctions)
        // C htop: {"      ", "      ", "      ", "      ", "Add   ", "      ", "      ", "      ", "      ", "Done  "}
        let available_columns_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Add", "F5"),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Done", "F10"),
        ]);

        SetupScreen {
            category: SetupCategory::DisplayOptions,
            category_index: 0,
            categories_panel,
            content_panel,
            display_options,
            content_index: 0,
            content_scroll: 0,
            focus: 0,
            function_bar,
            dec_inc_bar,
            meters_bar,
            meters_moving_bar,
            meters_available_bar,
            changed: false,
            // Meters panel state
            meters_column_focus: 0,
            meters_column_selection: vec![0; 4], // Support up to 4 columns
            meters_available_selection: 0,
            meters_available_scroll: 0,
            meters_moving: false,
            // Screens panel state
            screens_bar,
            columns_bar,
            available_columns_bar,
            screens_panel_focus: 0,
            screens_selection: 0,
            columns_selection: 0,
            available_columns_selection: 0,
            available_columns_scroll: 0,
            screens_moving: false,
            columns_moving: false,
            screens_renaming: false,
            screens_rename_buffer: String::new(),
            screens_rename_cursor: 0,
        }
    }

    fn build_display_options() -> Vec<OptionItem> {
        vec![
            OptionItem::text("For current screen tab:"),
            OptionItem::check("Tree view", SettingField::TreeView),
            OptionItem::check(
                "- Tree view is always sorted by PID (htop 2 behavior)",
                SettingField::TreeViewAlwaysByPid,
            ),
            OptionItem::check(
                "- Tree view is collapsed by default",
                SettingField::AllBranchesCollapsed,
            ),
            OptionItem::text("Global options:"),
            OptionItem::check("Show tabs for screens", SettingField::ScreenTabs),
            OptionItem::check(
                "Shadow other users' processes",
                SettingField::ShadowOtherUsers,
            ),
            OptionItem::check("Hide kernel threads", SettingField::HideKernelThreads),
            OptionItem::check(
                "Hide userland process threads",
                SettingField::HideUserlandThreads,
            ),
            OptionItem::check(
                "Hide processes running in containers",
                SettingField::HideRunningInContainer,
            ),
            OptionItem::check(
                "Display threads in a different color",
                SettingField::HighlightThreads,
            ),
            OptionItem::check("Show custom thread names", SettingField::ShowThreadNames),
            OptionItem::check("Show program path", SettingField::ShowProgramPath),
            OptionItem::check(
                "Highlight program \"basename\"",
                SettingField::HighlightBaseName,
            ),
            OptionItem::check(
                "Highlight out-dated/removed programs (red) / libraries (yellow)",
                SettingField::HighlightDeletedExe,
            ),
            OptionItem::check(
                "Shadow distribution path prefixes",
                SettingField::ShadowDistPathPrefix,
            ),
            OptionItem::check(
                "Merge exe, comm and cmdline in Command",
                SettingField::ShowMergedCommand,
            ),
            OptionItem::check(
                "- Try to find comm in cmdline (when Command is merged)",
                SettingField::FindCommInCmdline,
            ),
            OptionItem::check(
                "- Try to strip exe from cmdline (when Command is merged)",
                SettingField::StripExeFromCmdline,
            ),
            OptionItem::check(
                "Highlight large numbers in memory counters",
                SettingField::HighlightMegabytes,
            ),
            OptionItem::check("Leave a margin around header", SettingField::HeaderMargin),
            OptionItem::check(
                "Detailed CPU time (System/IO-Wait/Hard-IRQ/Soft-IRQ/Steal/Guest)",
                SettingField::DetailedCpuTime,
            ),
            OptionItem::check(
                "Count CPUs from 1 instead of 0",
                SettingField::CountCpusFromOne,
            ),
            OptionItem::check(
                "Update process names on every refresh",
                SettingField::UpdateProcessNames,
            ),
            OptionItem::check(
                "Add guest time in CPU meter percentage",
                SettingField::AccountGuestInCpuMeter,
            ),
            OptionItem::check(
                "Also show CPU percentage numerically",
                SettingField::ShowCpuUsage,
            ),
            OptionItem::check("Also show CPU frequency", SettingField::ShowCpuFrequency),
            OptionItem::check(
                "Show cached memory in graph and bar modes",
                SettingField::ShowCachedMemory,
            ),
            OptionItem::check("Enable the mouse", SettingField::EnableMouse),
            OptionItem::number_scaled(
                "Update interval (in seconds)",
                SettingField::Delay,
                1,
                255,
                -1,
            ),
            OptionItem::check(
                "Highlight new and old processes",
                SettingField::HighlightChanges,
            ),
            OptionItem::number(
                "- Highlight time (in seconds)",
                SettingField::HighlightDelaySecs,
                1,
                86400,
            ),
            OptionItem::number(
                "Hide main function bar (0 - off, 1 - on ESC until next input, 2 - permanently)",
                SettingField::HideFunctionBar,
                0,
                2,
            ),
        ]
    }

    /// Calculate layout based on terminal size
    pub fn layout(&mut self, crt: &Crt, header_height: i32, screen_tabs: bool) {
        let width = crt.width();
        let height = crt.height();

        // Account for screen tabs row (1 line) if enabled
        let tabs_height = if screen_tabs { 1 } else { 0 };
        let panel_y = header_height + tabs_height;

        // Categories panel: fixed width of 16, full height minus header, tabs, and function bar
        let panel_height = height - panel_y - 1; // Leave room for function bar
        self.categories_panel.resize(16, panel_height);
        self.categories_panel.move_to(0, panel_y);

        // Content panel: remaining width
        let content_width = width - 16;
        self.content_panel.resize(content_width, panel_height);
        self.content_panel.move_to(16, panel_y);
    }

    /// Draw the "[Setup]" tab (like C htop draws screen tabs with a name)
    fn draw_setup_tab(&self, crt: &Crt, header_height: i32) {
        const SCREEN_TAB_MARGIN_LEFT: i32 = 2;

        let y = header_height; // Tab row is right after header
        let mut x = SCREEN_TAB_MARGIN_LEFT;
        let max_x = crt.width();
        let reset_color = crt.color(ColorElement::ResetColor);

        // Fill the entire tab row with the reset color background first
        attrset(reset_color);
        mv(y, 0);
        for _ in 0..max_x {
            addch(' ' as u32);
        }

        if x >= max_x {
            return;
        }

        let border_attr = crt.color(ColorElement::ScreensCurBorder);
        let text_attr = crt.color(ColorElement::ScreensCurText);

        // Draw '[' - C htop doesn't reset between bracket and text
        attrset(border_attr);
        mvaddch(y, x, '[' as u32);
        x += 1;

        if x >= max_x {
            attrset(reset_color);
            return;
        }

        // Draw "Setup" text
        let name = "Setup";
        let name_width = name.len().min((max_x - x) as usize);
        attrset(text_attr);
        let _ = mvaddnstr(y, x, name, name_width as i32);
        x += name_width as i32;

        if x >= max_x {
            attrset(reset_color);
            return;
        }

        // Draw ']'
        attrset(border_attr);
        mvaddch(y, x, ']' as u32);

        // Only reset at the very end (matches C htop)
        attrset(reset_color);
    }

    /// Draw the setup screen
    pub fn draw(&mut self, crt: &Crt, settings: &Settings, header: &Header, machine: &Machine) {
        // Clear screen
        erase();

        // Draw header meters (like other screens do)
        header.draw(crt, machine, settings);

        // Draw "[Setup]" tab if screen tabs are enabled
        if settings.screen_tabs {
            self.draw_setup_tab(crt, header.get_height());
        }

        // Draw categories panel
        self.draw_categories_panel(crt);

        // Draw content panel based on selected category
        match self.category {
            SetupCategory::DisplayOptions => self.draw_display_options(crt, settings),
            SetupCategory::Colors => self.draw_colors_panel(crt, settings),
            SetupCategory::HeaderLayout => self.draw_header_layout(crt, settings),
            SetupCategory::Meters => self.draw_meters_panel(crt, settings),
            SetupCategory::Screens => self.draw_screens_panel(crt, settings),
        }

        // Draw function bar
        self.draw_function_bar(crt, settings);

        refresh();
    }

    fn draw_categories_panel(&self, crt: &Crt) {
        let x = self.categories_panel.x;
        let y = self.categories_panel.y;
        let w = self.categories_panel.w;
        let h = self.categories_panel.h;

        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if self.focus == 0 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = "Categories";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }

        // Draw items
        let selection_attr = if self.focus == 0 {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        for (i, cat) in SetupCategory::all().iter().enumerate() {
            if i as i32 >= h - 1 {
                break;
            }

            mv(y + 1 + i as i32, x);
            let attr = if i == self.category_index {
                selection_attr
            } else {
                normal_attr
            };

            attrset(attr);
            let name = cat.name();
            let _ = addstr(name);
            for _ in name.len()..w as usize {
                addch(' ' as u32);
            }
        }

        attrset(reset_color);
    }

    fn draw_display_options(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = "Display options";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attrset(reset_color);

        // Draw items
        let selection_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let _normal_attr = crt.color(ColorElement::Process);

        let display_height = (h - 1) as usize;
        for i in 0..display_height {
            let item_index = self.content_scroll as usize + i;
            let screen_y = y + 1 + i as i32;

            mv(screen_y, x);

            if item_index < self.display_options.len() {
                let item = &self.display_options[item_index];
                let is_selected = item_index == self.content_index && self.focus == 1;

                // Get the display string for this item
                let display_str = item.display(settings, crt);

                if is_selected {
                    attrset(selection_attr);
                    let text = display_str.text();
                    let display_text: String = text.chars().take(w as usize).collect();
                    let _ = addstr(&display_text);
                    for _ in display_text.chars().count()..w as usize {
                        addch(' ' as u32);
                    }
                    attrset(reset_color);
                } else {
                    // Draw with the RichString's own attributes
                    display_str.draw_at(screen_y, x, w);
                }
            } else {
                // Empty line - already filled with reset_color background
            }
        }

        attrset(reset_color);
    }

    fn draw_colors_panel(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = "Colors";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }

        // Draw color scheme options
        let selection_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let box_color = crt.color(ColorElement::CheckBox);
        let mark_color = crt.color(ColorElement::CheckMark);
        let text_color = crt.color(ColorElement::CheckText);

        let current_scheme = settings.color_scheme as usize;

        for (i, name) in COLOR_SCHEME_NAMES.iter().enumerate() {
            if i as i32 >= h - 1 {
                break;
            }

            mv(y + 1 + i as i32, x);
            let is_selected = i == self.content_index && self.focus == 1;
            let is_checked = i == current_scheme;

            if is_selected {
                attrset(selection_attr);
            }

            // Draw checkbox
            if !is_selected {
                attrset(box_color);
            }
            let _ = addstr("[");
            if !is_selected {
                attrset(mark_color);
            }
            let _ = addstr(if is_checked { "x" } else { " " });
            if !is_selected {
                attrset(box_color);
            }
            let _ = addstr("]    ");
            if !is_selected {
                attrset(text_color);
            }
            let _ = addstr(name);

            // Pad to width
            let used = 7 + name.len(); // "[x]    " + name
            for _ in used..w as usize {
                addch(' ' as u32);
            }
        }

        attrset(reset_color);
    }

    fn draw_header_layout(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = "Header Layout";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attrset(reset_color);

        // Draw header layout options with checkmarks (like C htop HeaderOptionsPanel)
        let layouts = HeaderLayout::all();
        let current_layout = settings.header_layout;
        let display_height = (h - 1) as usize; // Minus header row

        let text_attr = crt.color(ColorElement::Process);
        let selected_attr = crt.color(ColorElement::PanelSelectionFocus);
        let check_attr = crt.color(ColorElement::CheckBox);

        for (i, &layout) in layouts.iter().enumerate() {
            if i >= display_height {
                break;
            }

            let screen_y = y + 1 + i as i32;
            let is_selected = self.focus == 1 && i == self.content_index;
            let is_checked = layout == current_layout;

            mv(screen_y, x);

            // Draw checkbox
            let checkbox = if is_checked { "[x] " } else { "[ ] " };

            if is_selected {
                // Selected row - highlight entire line
                attrset(selected_attr);
                let _ = addstr(checkbox);
                let desc = layout.description();
                let _ = addstr(desc);
                // Pad to width
                let used = checkbox.len() + desc.len();
                for _ in used..w as usize {
                    addch(' ' as u32);
                }
                attrset(reset_color);
            } else {
                // Non-selected row
                attrset(check_attr);
                let _ = addstr(checkbox);
                attrset(reset_color);

                attrset(text_attr);
                let desc = layout.description();
                let _ = addstr(desc);
                attrset(reset_color);

                // Pad to width - already filled with reset_color background
            }
        }

        attrset(reset_color);
    }

    fn draw_meters_panel(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let total_width = self.content_panel.w;
        let h = self.content_panel.h;

        // Get number of columns from current layout
        let num_columns = settings.header_layout.num_columns();

        // C htop uses fixed width of 20 for each column panel, and remaining for available meters
        const COLUMN_PANEL_WIDTH: i32 = 20;
        let column_panels_width = COLUMN_PANEL_WIDTH * num_columns as i32;
        let available_width = (total_width - column_panels_width).max(COLUMN_PANEL_WIDTH);

        // Determine which panel has focus
        // focus == 1 means we're in meters mode (not categories)
        // meters_column_focus: 0..num_columns-1 = column panels, num_columns = available meters
        let focused_column = if self.focus == 1 {
            Some(self.meters_column_focus)
        } else {
            None
        };

        // Draw column panels (fixed width 20 each, like C htop)
        let mut cur_x = x;
        for col_idx in 0..num_columns {
            self.draw_meter_column_panel(
                crt,
                settings,
                col_idx,
                cur_x,
                y,
                COLUMN_PANEL_WIDTH,
                h,
                focused_column == Some(col_idx),
            );
            cur_x += COLUMN_PANEL_WIDTH;
        }

        // Draw available meters panel (takes remaining width, like C htop)
        self.draw_available_meters_panel(
            crt,
            cur_x,
            y,
            available_width,
            h,
            focused_column == Some(num_columns),
        );
    }

    /// Draw a single meter column panel
    fn draw_meter_column_panel(
        &self,
        crt: &Crt,
        settings: &Settings,
        col_idx: usize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = format!("Column {}", col_idx + 1);
        let header_display: String = header.chars().take(w as usize).collect();
        let _ = addstr(&header_display);
        for _ in header_display.len()..w as usize {
            addch(' ' as u32);
        }
        attrset(reset_color);

        // Get meters for this column
        let meters = settings.header_columns.get(col_idx);
        let selection = self
            .meters_column_selection
            .get(col_idx)
            .copied()
            .unwrap_or(0);

        // Selection color
        let selection_attr = if has_focus {
            if self.meters_moving {
                crt.color(ColorElement::PanelSelectionFollow)
            } else {
                crt.color(ColorElement::PanelSelectionFocus)
            }
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        // Draw meters in this column
        let display_height = (h - 1) as usize;
        for i in 0..display_height {
            let screen_y = y + 1 + i as i32;
            mv(screen_y, x);

                if let Some(meters) = meters {
                if i < meters.len() {
                    let meter = &meters[i];
                    let is_selected = has_focus && i == selection;

                    // Get display name with mode
                    let display = meter_display_name(&meter.name, meter.mode);
                    
                    // Add moving indicator (↕) if this item is being moved
                    let display_text = if is_selected && self.meters_moving {
                        // Use UTF-8 up-down arrow like C htop
                        let prefix = "↕ ";
                        let max_len = (w - 1) as usize;
                        let prefixed = format!("{}{}", prefix, display);
                        prefixed.chars().take(max_len).collect::<String>()
                    } else {
                        display.chars().take((w - 1) as usize).collect::<String>()
                    };

                    if is_selected {
                        attrset(selection_attr);
                        let _ = addstr(&display_text);
                        for _ in display_text.chars().count()..w as usize {
                            addch(' ' as u32);
                        }
                        attrset(reset_color);
                    } else {
                        attrset(normal_attr);
                        let _ = addstr(&display_text);
                        attrset(reset_color);
                        // Rest of line already filled with reset_color background
                    }
                }
                // Empty rows already filled with reset_color background
            }
            // No meters array - already filled with reset_color background
        }

        attrset(reset_color);
    }

    /// Draw the available meters panel
    fn draw_available_meters_panel(
        &self,
        crt: &Crt,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = "Available meters";
        let header_display: String = header.chars().take(w as usize).collect();
        let _ = addstr(&header_display);
        for _ in header_display.len()..w as usize {
            addch(' ' as u32);
        }
        attrset(reset_color);

        // Selection color
        let selection_attr = if has_focus {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        // Get platform-filtered meters
        let available_meters = available_meters_for_platform();

        // Draw available meters
        let display_height = (h - 1) as usize;
        for i in 0..display_height {
            let item_index = self.meters_available_scroll as usize + i;
            let screen_y = y + 1 + i as i32;
            mv(screen_y, x);

            if item_index < available_meters.len() {
                let meter_info = available_meters[item_index];
                let is_selected = has_focus && item_index == self.meters_available_selection;

                // Show description in available meters panel (like C htop)
                let display_text: String = meter_info
                    .description
                    .chars()
                    .take((w - 1) as usize)
                    .collect();

                if is_selected {
                    attrset(selection_attr);
                    let _ = addstr(&display_text);
                    for _ in display_text.chars().count()..w as usize {
                        addch(' ' as u32);
                    }
                    attrset(reset_color);
                } else {
                    attrset(normal_attr);
                    let _ = addstr(&display_text);
                    attrset(reset_color);
                    // Rest of line already filled with reset_color background
                }
            }
            // Empty rows already filled with reset_color background
        }

        attrset(reset_color);
    }

    fn draw_screens_panel(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let total_width = self.content_panel.w;
        let h = self.content_panel.h;

        // C htop uses fixed width of 20 for screens panel, 20 for columns panel, rest for available
        const SCREENS_PANEL_WIDTH: i32 = 20;
        const COLUMNS_PANEL_WIDTH: i32 = 20;
        let available_width = (total_width - SCREENS_PANEL_WIDTH - COLUMNS_PANEL_WIDTH).max(20);

        // Determine which panel has focus
        let focused_panel = if self.focus == 1 {
            Some(self.screens_panel_focus)
        } else {
            None
        };

        // Draw Screens panel (leftmost)
        self.draw_screens_list_panel(
            crt,
            settings,
            x,
            y,
            SCREENS_PANEL_WIDTH,
            h,
            focused_panel == Some(0),
        );

        // Draw Active Columns panel (middle)
        let columns_x = x + SCREENS_PANEL_WIDTH;
        self.draw_active_columns_panel(
            crt,
            settings,
            columns_x,
            y,
            COLUMNS_PANEL_WIDTH,
            h,
            focused_panel == Some(1),
        );

        // Draw Available Columns panel (rightmost)
        let available_x = columns_x + COLUMNS_PANEL_WIDTH;
        self.draw_available_columns_panel(
            crt,
            available_x,
            y,
            available_width,
            h,
            focused_panel == Some(2),
        );
    }

    /// Draw the Screens list panel
    fn draw_screens_list_panel(
        &self,
        crt: &Crt,
        settings: &Settings,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = "Screens";
        let header_display: String = header.chars().take(w as usize).collect();
        let _ = addstr(&header_display);
        for _ in header_display.len()..w as usize {
            addch(' ' as u32);
        }
        attrset(reset_color);

        // Selection colors
        let selection_attr = if has_focus {
            if self.screens_moving {
                crt.color(ColorElement::PanelSelectionFollow)
            } else if self.screens_renaming {
                crt.color(ColorElement::PanelEdit)
            } else {
                crt.color(ColorElement::PanelSelectionFocus)
            }
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        // Draw screen items
        let display_height = (h - 1) as usize;
        for i in 0..display_height {
            let screen_y = y + 1 + i as i32;
            mv(screen_y, x);

            if i < settings.screens.len() {
                let screen = &settings.screens[i];
                let is_selected = has_focus && i == self.screens_selection;

                // Get display name (with renaming support)
                let display = if is_selected && self.screens_renaming {
                    // Show rename buffer with cursor
                    format!("{}_", &self.screens_rename_buffer)
                } else {
                    screen.heading.clone()
                };

                // Add moving indicator (↕) if this item is being moved
                let display_text = if is_selected && self.screens_moving && !self.screens_renaming {
                    let prefix = "↕ ";
                    let max_len = (w - 1) as usize;
                    let prefixed = format!("{}{}", prefix, display);
                    prefixed.chars().take(max_len).collect::<String>()
                } else {
                    display.chars().take((w - 1) as usize).collect::<String>()
                };

                if is_selected {
                    attrset(selection_attr);
                    let _ = addstr(&display_text);
                    for _ in display_text.chars().count()..w as usize {
                        addch(' ' as u32);
                    }
                    attrset(reset_color);
                } else {
                    attrset(normal_attr);
                    let _ = addstr(&display_text);
                    attrset(reset_color);
                    // Rest of line already filled with reset_color background
                }
            }
            // Empty rows already filled with reset_color background
        }

        attrset(reset_color);
    }

    /// Draw the Active Columns panel
    fn draw_active_columns_panel(
        &self,
        crt: &Crt,
        settings: &Settings,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = "Active Columns";
        let header_display: String = header.chars().take(w as usize).collect();
        let _ = addstr(&header_display);
        for _ in header_display.len()..w as usize {
            addch(' ' as u32);
        }
        attrset(reset_color);

        // Selection colors
        let selection_attr = if has_focus {
            if self.columns_moving {
                crt.color(ColorElement::PanelSelectionFollow)
            } else {
                crt.color(ColorElement::PanelSelectionFocus)
            }
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        // Get current screen's fields
        let fields = if self.screens_selection < settings.screens.len() {
            &settings.screens[self.screens_selection].fields
        } else if !settings.screens.is_empty() {
            &settings.screens[0].fields
        } else {
            return;
        };

        // Draw column items
        let display_height = (h - 1) as usize;
        for i in 0..display_height {
            let screen_y = y + 1 + i as i32;
            mv(screen_y, x);

            if i < fields.len() {
                let field = fields[i];
                let is_selected = has_focus && i == self.columns_selection;

                let display = field.name().unwrap_or("?");
                
                // Add moving indicator (↕) if this item is being moved
                let display_text = if is_selected && self.columns_moving {
                    let prefix = "↕ ";
                    let max_len = (w - 1) as usize;
                    let prefixed = format!("{}{}", prefix, display);
                    prefixed.chars().take(max_len).collect::<String>()
                } else {
                    display.chars().take((w - 1) as usize).collect::<String>()
                };

                if is_selected {
                    attrset(selection_attr);
                    let _ = addstr(&display_text);
                    for _ in display_text.chars().count()..w as usize {
                        addch(' ' as u32);
                    }
                    attrset(reset_color);
                } else {
                    attrset(normal_attr);
                    let _ = addstr(&display_text);
                    attrset(reset_color);
                    // Rest of line already filled with reset_color background
                }
            }
            // Empty rows already filled with reset_color background
        }

        attrset(reset_color);
    }

    /// Draw the Available Columns panel
    fn draw_available_columns_panel(
        &self,
        crt: &Crt,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);
        attrset(reset_color);
        for row in 0..h {
            mv(y + row, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attrset(header_attr);
        let header = "Available Columns";
        let header_display: String = header.chars().take(w as usize).collect();
        let _ = addstr(&header_display);
        for _ in header_display.len()..w as usize {
            addch(' ' as u32);
        }
        attrset(reset_color);

        // Selection colors
        let selection_attr = if has_focus {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        // Get available fields
        let available_fields = ProcessField::all();

        // Draw column items
        let display_height = (h - 1) as usize;
        for i in 0..display_height {
            let item_index = self.available_columns_scroll as usize + i;
            let screen_y = y + 1 + i as i32;
            mv(screen_y, x);

            if item_index < available_fields.len() {
                let field = available_fields[item_index];
                let is_selected = has_focus && item_index == self.available_columns_selection;

                // Show name with description
                let display = format!("{} - {}", field.name().unwrap_or("?"), field.description());
                let display_text: String = display.chars().take((w - 1) as usize).collect();

                if is_selected {
                    attrset(selection_attr);
                    let _ = addstr(&display_text);
                    for _ in display_text.chars().count()..w as usize {
                        addch(' ' as u32);
                    }
                    attrset(reset_color);
                } else {
                    attrset(normal_attr);
                    let _ = addstr(&display_text);
                    attrset(reset_color);
                    // Rest of line already filled with reset_color background
                }
            }
            // Empty rows already filled with reset_color background
        }

        attrset(reset_color);
    }

    fn draw_function_bar(&self, crt: &Crt, settings: &Settings) {
        let y = crt.height() - 1;

        // Choose which function bar to show based on current category and selection
        let bar = if self.focus == 1 {
            match self.category {
                SetupCategory::DisplayOptions => {
                    if let Some(item) = self.display_options.get(self.content_index) {
                        if matches!(item, OptionItem::Number { .. }) {
                            &self.dec_inc_bar
                        } else {
                            &self.function_bar
                        }
                    } else {
                        &self.function_bar
                    }
                }
                SetupCategory::Meters => {
                    let num_columns = settings.header_layout.num_columns();
                    if self.meters_column_focus == num_columns {
                        // Available meters panel
                        &self.meters_available_bar
                    } else if self.meters_moving {
                        &self.meters_moving_bar
                    } else {
                        &self.meters_bar
                    }
                }
                SetupCategory::Screens => {
                    match self.screens_panel_focus {
                        0 => &self.screens_bar,           // Screens panel
                        1 => &self.columns_bar,           // Active Columns panel
                        2 => &self.available_columns_bar, // Available Columns panel
                        _ => &self.function_bar,
                    }
                }
                _ => &self.function_bar,
            }
        } else {
            &self.function_bar
        };

        bar.draw(crt, y, settings);
    }

    /// Handle key input
    pub fn handle_key(
        &mut self,
        key: i32,
        settings: &mut Settings,
        crt: &mut Crt,
        header: &mut Header,
    ) -> HandlerResult {
        // Handle Meters category separately since it has different panel structure
        if self.category == SetupCategory::Meters && self.focus == 1 {
            return self.handle_meters_key(key, settings, header);
        }

        // Handle Screens category separately since it has different panel structure
        if self.category == SetupCategory::Screens && self.focus == 1 {
            return self.handle_screens_key(key, settings);
        }

        match key {
            // Exit keys
            KEY_F10 | KEY_ESC | KEY_Q => {
                // ESC or F10 or q - exit setup
                return HandlerResult::BreakLoop;
            }

            // Tab / Left / Right - switch focus between panels
            KEY_TAB | KEY_LEFT | KEY_RIGHT => {
                if self.category == SetupCategory::DisplayOptions
                    || self.category == SetupCategory::Colors
                    || self.category == SetupCategory::HeaderLayout
                {
                    self.focus = if self.focus == 0 { 1 } else { 0 };
                    return HandlerResult::Handled;
                } else if self.category == SetupCategory::Meters {
                    // Switch to meters panel
                    self.focus = if self.focus == 0 { 1 } else { 0 };
                    return HandlerResult::Handled;
                }
            }

            // Navigation
            KEY_UP => {
                if self.focus == 0 {
                    // Categories panel
                    if self.category_index > 0 {
                        self.category_index -= 1;
                        self.category = SetupCategory::all()[self.category_index];
                        // For HeaderLayout, select current layout; otherwise start at 0
                        if self.category == SetupCategory::HeaderLayout {
                            self.content_index = settings.header_layout.to_index();
                        } else {
                            self.content_index = 0;
                        }
                        self.content_scroll = 0;
                    }
                } else {
                    // Content panel
                    self.move_content_up(settings);
                }
                return HandlerResult::Handled;
            }

            KEY_DOWN => {
                if self.focus == 0 {
                    // Categories panel
                    if self.category_index < SetupCategory::all().len() - 1 {
                        self.category_index += 1;
                        self.category = SetupCategory::all()[self.category_index];
                        // For HeaderLayout, select current layout; otherwise start at 0
                        if self.category == SetupCategory::HeaderLayout {
                            self.content_index = settings.header_layout.to_index();
                        } else {
                            self.content_index = 0;
                        }
                        self.content_scroll = 0;
                    }
                } else {
                    // Content panel
                    self.move_content_down(settings, crt);
                }
                return HandlerResult::Handled;
            }

            KEY_PPAGE => {
                if self.focus == 1 {
                    let page_size = (self.content_panel.h - 1) as usize;
                    for _ in 0..page_size {
                        self.move_content_up(settings);
                    }
                }
                return HandlerResult::Handled;
            }

            KEY_NPAGE => {
                if self.focus == 1 {
                    let page_size = (self.content_panel.h - 1) as usize;
                    for _ in 0..page_size {
                        self.move_content_down(settings, crt);
                    }
                }
                return HandlerResult::Handled;
            }

            KEY_HOME => {
                if self.focus == 1 {
                    self.content_index = 0;
                    self.content_scroll = 0;
                    // Skip non-interactive items
                    self.skip_to_interactive(settings, true);
                }
                return HandlerResult::Handled;
            }

            KEY_END => {
                if self.focus == 1 {
                    let max_index = self.get_content_count() - 1;
                    self.content_index = max_index;
                    self.ensure_content_visible(crt);
                    // Skip non-interactive items (going backwards)
                    self.skip_to_interactive(settings, false);
                }
                return HandlerResult::Handled;
            }

            // Toggle / Enter / Space
            KEY_ENTER_LF | KEY_ENTER_CR | KEY_SPACE => {
                if self.focus == 1 {
                    self.handle_toggle(settings, crt, header);
                    return HandlerResult::Handled;
                }
            }

            // Decrease for number items
            KEY_MINUS | KEY_F7 => {
                if self.focus == 1 {
                    self.handle_decrease(settings);
                    return HandlerResult::Handled;
                }
            }

            // Increase for number items
            KEY_PLUS | KEY_F8 => {
                if self.focus == 1 {
                    self.handle_increase(settings);
                    return HandlerResult::Handled;
                }
            }

            _ => {}
        }

        HandlerResult::Ignored
    }

    /// Handle key input for Meters category
    fn handle_meters_key(
        &mut self,
        key: i32,
        settings: &mut Settings,
        header: &mut Header,
    ) -> HandlerResult {
        let num_columns = settings.header_layout.num_columns();
        let is_available_panel = self.meters_column_focus == num_columns;

        match key {
            // Exit keys
            KEY_F10 | KEY_ESC | KEY_Q => {
                if self.meters_moving {
                    // If moving, stop moving instead of exiting
                    self.meters_moving = false;
                    return HandlerResult::Handled;
                }
                return HandlerResult::BreakLoop;
            }

            // Enter, r, R, F6 - add meter to rightmost column (from available panel)
            // or toggle moving mode (from column panel)
            KEY_ENTER_LF | KEY_ENTER_CR | KEY_R | KEY_R_UPPER | KEY_F6 => {
                if is_available_panel {
                    // Add selected meter to rightmost column (like C htop)
                    let rightmost = num_columns.saturating_sub(1);
                    self.add_meter_to_column(settings, header, rightmost);
                    return HandlerResult::Handled;
                } else if key == KEY_ENTER_LF || key == KEY_ENTER_CR {
                    // Toggle moving mode (only for Enter, not r/R/F6)
                    self.meters_moving = !self.meters_moving;
                    return HandlerResult::Handled;
                }
            }

            // l, L, F5 - add meter to leftmost column (from available panel)
            KEY_L | KEY_L_UPPER | KEY_F5 => {
                if is_available_panel {
                    // Add selected meter to leftmost column (column 0)
                    self.add_meter_to_column(settings, header, 0);
                    return HandlerResult::Handled;
                }
            }

            // Space, 't', F4 - cycle meter style (Bar/Text/Graph/Led)
            KEY_SPACE | KEY_T | KEY_F4 => {
                if !is_available_panel {
                    self.cycle_meter_style(settings, header);
                    return HandlerResult::Handled;
                }
            }

            // Delete, F9 - remove meter from column
            KEY_DC | KEY_F9 => {
                if !is_available_panel {
                    self.delete_selected_meter(settings, header);
                    return HandlerResult::Handled;
                }
            }

            // '[', '-', F7 - move meter up
            KEY_LBRACKET | KEY_MINUS | KEY_F7 => {
                if !is_available_panel {
                    self.move_meter_up(settings, header);
                    return HandlerResult::Handled;
                }
            }

            // ']', '+', F8 - move meter down
            KEY_RBRACKET | KEY_PLUS | KEY_F8 => {
                if !is_available_panel {
                    self.move_meter_down(settings, header);
                    return HandlerResult::Handled;
                }
            }

            // Navigation
            KEY_UP => {
                if self.meters_moving && !is_available_panel {
                    // Move meter up in column
                    self.move_meter_up(settings, header);
                } else {
                    // Move selection up
                    self.meters_move_selection_up(settings);
                }
                return HandlerResult::Handled;
            }

            KEY_DOWN => {
                if self.meters_moving && !is_available_panel {
                    // Move meter down in column
                    self.move_meter_down(settings, header);
                } else {
                    // Move selection down
                    self.meters_move_selection_down(settings);
                }
                return HandlerResult::Handled;
            }

            KEY_LEFT => {
                if self.meters_moving && !is_available_panel {
                    // Move meter to left column
                    self.move_meter_left(settings, header);
                } else {
                    // Move focus to left panel, skipping empty columns
                    if self.meters_column_focus > 0 {
                        let mut new_focus = self.meters_column_focus - 1;
                        // Skip empty columns when navigating left
                        while new_focus > 0 {
                            let col_is_empty = settings
                                .header_columns
                                .get(new_focus)
                                .map(|c| c.is_empty())
                                .unwrap_or(true);
                            if !col_is_empty {
                                break;
                            }
                            new_focus -= 1;
                        }
                        // Check if the final column is also empty - if so, go to categories
                        let final_col_empty = settings
                            .header_columns
                            .get(new_focus)
                            .map(|c| c.is_empty())
                            .unwrap_or(true);
                        if final_col_empty && new_focus == 0 {
                            // Column 0 is empty, go to categories
                            self.focus = 0;
                        } else {
                            self.meters_column_focus = new_focus;
                        }
                    } else {
                        // Go back to categories panel
                        self.focus = 0;
                    }
                }
                return HandlerResult::Handled;
            }

            KEY_RIGHT => {
                if self.meters_moving && !is_available_panel {
                    // Move meter to right column
                    self.move_meter_right(settings, header);
                } else {
                    // Move focus to right panel, skipping empty columns
                    if self.meters_column_focus < num_columns {
                        let mut new_focus = self.meters_column_focus + 1;
                        // Skip empty columns when navigating right (but not the Available panel)
                        while new_focus < num_columns {
                            let col_is_empty = settings
                                .header_columns
                                .get(new_focus)
                                .map(|c| c.is_empty())
                                .unwrap_or(true);
                            if !col_is_empty {
                                break;
                            }
                            new_focus += 1;
                        }
                        self.meters_column_focus = new_focus;
                    }
                }
                return HandlerResult::Handled;
            }

            KEY_TAB => {
                // Cycle through panels
                if self.meters_column_focus < num_columns {
                    self.meters_column_focus += 1;
                } else {
                    // Go back to categories
                    self.focus = 0;
                    self.meters_column_focus = 0;
                }
                return HandlerResult::Handled;
            }

            _ => {}
        }

        HandlerResult::Ignored
    }

    /// Handle key events for Screens category
    /// Routes to appropriate handler based on current panel focus
    fn handle_screens_key(&mut self, key: i32, settings: &mut Settings) -> HandlerResult {
        // If renaming, handle that first
        if self.screens_renaming {
            return self.handle_screens_renaming_key(key, settings);
        }

        // Common navigation: Tab/Left/Right to switch panels
        match key {
            KEY_TAB | KEY_RIGHT => {
                self.screens_panel_focus = (self.screens_panel_focus + 1) % 3;
                // Clear moving states when switching panels
                self.screens_moving = false;
                self.columns_moving = false;
                return HandlerResult::Handled;
            }
            KEY_LEFT => {
                if self.screens_panel_focus == 0 {
                    // Go back to categories panel
                    self.focus = 0;
                    self.screens_moving = false;
                    self.columns_moving = false;
                } else {
                    self.screens_panel_focus = self.screens_panel_focus.saturating_sub(1);
                    self.screens_moving = false;
                    self.columns_moving = false;
                }
                return HandlerResult::Handled;
            }
            KEY_F10 | KEY_ESC => {
                // Exit setup
                return HandlerResult::BreakLoop;
            }
            _ => {}
        }

        // Route to panel-specific handler
        match self.screens_panel_focus {
            0 => self.handle_screens_list_key(key, settings),
            1 => self.handle_columns_key(key, settings),
            2 => self.handle_available_columns_key(key, settings),
            _ => HandlerResult::Ignored,
        }
    }

    /// Handle key events while renaming a screen
    fn handle_screens_renaming_key(&mut self, key: i32, settings: &mut Settings) -> HandlerResult {
        match key {
            // Enter - confirm rename
            KEY_ENTER | 10 | 13 => {
                if !self.screens_rename_buffer.is_empty()
                    && self.screens_selection < settings.screens.len()
                {
                    settings.screens[self.screens_selection].heading =
                        self.screens_rename_buffer.clone();
                    settings.changed = true;
                    self.changed = true;
                }
                self.screens_renaming = false;
                self.screens_rename_buffer.clear();
                return HandlerResult::Handled;
            }
            // Escape - cancel rename
            KEY_ESC => {
                self.screens_renaming = false;
                self.screens_rename_buffer.clear();
                return HandlerResult::Handled;
            }
            // Backspace - delete character
            KEY_BACKSPACE | KEY_BACKSPACE_ASCII => {
                if self.screens_rename_cursor > 0 {
                    self.screens_rename_cursor -= 1;
                    self.screens_rename_buffer
                        .remove(self.screens_rename_cursor);
                }
                return HandlerResult::Handled;
            }
            // Delete - delete character at cursor
            KEY_DC => {
                if self.screens_rename_cursor < self.screens_rename_buffer.len() {
                    self.screens_rename_buffer
                        .remove(self.screens_rename_cursor);
                }
                return HandlerResult::Handled;
            }
            // Left arrow - move cursor left
            KEY_LEFT => {
                if self.screens_rename_cursor > 0 {
                    self.screens_rename_cursor -= 1;
                }
                return HandlerResult::Handled;
            }
            // Right arrow - move cursor right
            KEY_RIGHT => {
                if self.screens_rename_cursor < self.screens_rename_buffer.len() {
                    self.screens_rename_cursor += 1;
                }
                return HandlerResult::Handled;
            }
            // Home - move to beginning
            KEY_HOME => {
                self.screens_rename_cursor = 0;
                return HandlerResult::Handled;
            }
            // End - move to end
            KEY_END => {
                self.screens_rename_cursor = self.screens_rename_buffer.len();
                return HandlerResult::Handled;
            }
            _ => {
                // Printable characters
                if (32..127).contains(&key) && self.screens_rename_buffer.len() < SCREEN_NAME_LEN {
                    let ch = key as u8 as char;
                    self.screens_rename_buffer
                        .insert(self.screens_rename_cursor, ch);
                    self.screens_rename_cursor += 1;
                    return HandlerResult::Handled;
                }
            }
        }
        HandlerResult::Ignored
    }

    /// Handle key events for screens list panel (panel 0)
    fn handle_screens_list_key(&mut self, key: i32, settings: &mut Settings) -> HandlerResult {
        let num_screens = settings.screens.len();

        match key {
            // Navigation
            KEY_UP => {
                if self.screens_moving && self.screens_selection > 0 {
                    // Move screen up
                    self.move_screen_up(settings);
                } else if self.screens_selection > 0 {
                    self.screens_selection -= 1;
                    // Reset columns selection when screen changes
                    self.columns_selection = 0;
                }
                return HandlerResult::Handled;
            }
            KEY_DOWN => {
                if self.screens_moving && self.screens_selection < num_screens.saturating_sub(1) {
                    // Move screen down
                    self.move_screen_down(settings);
                } else if self.screens_selection < num_screens.saturating_sub(1) {
                    self.screens_selection += 1;
                    // Reset columns selection when screen changes
                    self.columns_selection = 0;
                }
                return HandlerResult::Handled;
            }
            // Enter - toggle moving mode
            KEY_ENTER | 10 | 13 => {
                self.screens_moving = !self.screens_moving;
                return HandlerResult::Handled;
            }
            // F2 / Ctrl+R - Rename
            KEY_F2 | KEY_CTRL_R => {
                if self.screens_selection < settings.screens.len() {
                    self.screens_renaming = true;
                    self.screens_rename_buffer =
                        settings.screens[self.screens_selection].heading.clone();
                    self.screens_rename_cursor = self.screens_rename_buffer.len();
                }
                return HandlerResult::Handled;
            }
            // F5 / Ctrl+N - New screen
            KEY_F5 | KEY_CTRL_N => {
                self.add_new_screen(settings);
                return HandlerResult::Handled;
            }
            // F7 / [ / - - Move up
            KEY_F7 | 91 | 45 => {
                // [ = 91, - = 45
                self.move_screen_up(settings);
                return HandlerResult::Handled;
            }
            // F8 / ] / + - Move down
            KEY_F8 | 93 | 43 => {
                // ] = 93, + = 43
                self.move_screen_down(settings);
                return HandlerResult::Handled;
            }
            // F9 - Remove screen
            KEY_F9 => {
                self.remove_screen(settings);
                return HandlerResult::Handled;
            }
            _ => {}
        }
        HandlerResult::Ignored
    }

    /// Handle key events for active columns panel (panel 1)
    fn handle_columns_key(&mut self, key: i32, settings: &mut Settings) -> HandlerResult {
        let num_fields = self.get_current_screen_fields_len(settings);

        match key {
            // Navigation
            KEY_UP => {
                if self.columns_moving && self.columns_selection > 0 {
                    // Move column up
                    self.move_column_up(settings);
                } else if self.columns_selection > 0 {
                    self.columns_selection -= 1;
                }
                return HandlerResult::Handled;
            }
            KEY_DOWN => {
                if self.columns_moving && self.columns_selection < num_fields.saturating_sub(1) {
                    // Move column down
                    self.move_column_down(settings);
                } else if self.columns_selection < num_fields.saturating_sub(1) {
                    self.columns_selection += 1;
                }
                return HandlerResult::Handled;
            }
            // Enter - toggle moving mode
            KEY_ENTER | 10 | 13 => {
                self.columns_moving = !self.columns_moving;
                return HandlerResult::Handled;
            }
            // F7 / [ / - - Move up
            KEY_F7 | 91 | 45 => {
                self.move_column_up(settings);
                return HandlerResult::Handled;
            }
            // F8 / ] / + - Move down
            KEY_F8 | 93 | 43 => {
                self.move_column_down(settings);
                return HandlerResult::Handled;
            }
            // F9 / Delete - Remove column
            KEY_F9 | KEY_DC => {
                self.remove_column(settings);
                return HandlerResult::Handled;
            }
            _ => {}
        }
        HandlerResult::Ignored
    }

    /// Handle key events for available columns panel (panel 2)
    fn handle_available_columns_key(&mut self, key: i32, settings: &mut Settings) -> HandlerResult {
        let all_fields = ProcessField::all();
        let num_available = all_fields.len();

        match key {
            // Navigation
            KEY_UP => {
                if self.available_columns_selection > 0 {
                    self.available_columns_selection -= 1;
                    // Adjust scroll
                    if (self.available_columns_selection as i32) < self.available_columns_scroll {
                        self.available_columns_scroll = self.available_columns_selection as i32;
                    }
                }
                return HandlerResult::Handled;
            }
            KEY_DOWN => {
                if self.available_columns_selection < num_available.saturating_sub(1) {
                    self.available_columns_selection += 1;
                    // Adjust scroll
                    let display_height = self.content_panel.h - 1;
                    if (self.available_columns_selection as i32)
                        >= self.available_columns_scroll + display_height
                    {
                        self.available_columns_scroll =
                            self.available_columns_selection as i32 - display_height + 1;
                    }
                }
                return HandlerResult::Handled;
            }
            // Enter / F5 - Add column
            KEY_ENTER | 10 | 13 | KEY_F5 => {
                self.add_column_to_active(settings);
                return HandlerResult::Handled;
            }
            _ => {}
        }
        HandlerResult::Ignored
    }

    /// Get the number of fields in the currently selected screen
    fn get_current_screen_fields_len(&self, settings: &Settings) -> usize {
        if self.screens_selection < settings.screens.len() {
            settings.screens[self.screens_selection].fields.len()
        } else {
            0
        }
    }

    /// Add a new screen
    fn add_new_screen(&mut self, settings: &mut Settings) {
        let new_screen = ScreenSettings {
            heading: "New".to_string(),
            ..Default::default()
        };
        // Insert after current selection
        let insert_pos = (self.screens_selection + 1).min(settings.screens.len());
        settings.screens.insert(insert_pos, new_screen);
        self.screens_selection = insert_pos;
        self.columns_selection = 0;
        settings.changed = true;
        self.changed = true;

        // Start renaming immediately
        self.screens_renaming = true;
        self.screens_rename_buffer = "New".to_string();
        self.screens_rename_cursor = 3;
    }

    /// Move the selected screen up
    fn move_screen_up(&mut self, settings: &mut Settings) {
        if self.screens_selection > 0 && self.screens_selection < settings.screens.len() {
            settings
                .screens
                .swap(self.screens_selection, self.screens_selection - 1);
            self.screens_selection -= 1;
            settings.changed = true;
            self.changed = true;
        }
    }

    /// Move the selected screen down
    fn move_screen_down(&mut self, settings: &mut Settings) {
        if self.screens_selection < settings.screens.len().saturating_sub(1) {
            settings
                .screens
                .swap(self.screens_selection, self.screens_selection + 1);
            self.screens_selection += 1;
            settings.changed = true;
            self.changed = true;
        }
    }

    /// Remove the selected screen
    fn remove_screen(&mut self, settings: &mut Settings) {
        // Don't remove the last screen
        if settings.screens.len() <= 1 {
            return;
        }
        if self.screens_selection < settings.screens.len() {
            settings.screens.remove(self.screens_selection);
            if self.screens_selection >= settings.screens.len() {
                self.screens_selection = settings.screens.len().saturating_sub(1);
            }
            // Reset columns selection
            self.columns_selection = 0;
            settings.changed = true;
            self.changed = true;
        }
    }

    /// Move the selected column up
    fn move_column_up(&mut self, settings: &mut Settings) {
        if self.screens_selection >= settings.screens.len() {
            return;
        }
        let fields = &mut settings.screens[self.screens_selection].fields;
        if self.columns_selection > 0 && self.columns_selection < fields.len() {
            fields.swap(self.columns_selection, self.columns_selection - 1);
            self.columns_selection -= 1;
            settings.changed = true;
            self.changed = true;
        }
    }

    /// Move the selected column down
    fn move_column_down(&mut self, settings: &mut Settings) {
        if self.screens_selection >= settings.screens.len() {
            return;
        }
        let fields = &mut settings.screens[self.screens_selection].fields;
        if self.columns_selection < fields.len().saturating_sub(1) {
            fields.swap(self.columns_selection, self.columns_selection + 1);
            self.columns_selection += 1;
            settings.changed = true;
            self.changed = true;
        }
    }

    /// Remove the selected column
    fn remove_column(&mut self, settings: &mut Settings) {
        if self.screens_selection >= settings.screens.len() {
            return;
        }
        let fields = &mut settings.screens[self.screens_selection].fields;
        // Don't remove the last column
        if fields.len() <= 1 {
            return;
        }
        if self.columns_selection < fields.len() {
            fields.remove(self.columns_selection);
            if self.columns_selection >= fields.len() {
                self.columns_selection = fields.len().saturating_sub(1);
            }
            settings.changed = true;
            self.changed = true;
        }
    }

    /// Add the selected available column to the active columns
    fn add_column_to_active(&mut self, settings: &mut Settings) {
        if self.screens_selection >= settings.screens.len() {
            return;
        }
        let all_fields = ProcessField::all();
        if self.available_columns_selection >= all_fields.len() {
            return;
        }
        let field = all_fields[self.available_columns_selection];

        // Add after current selection in columns panel, or at end if none selected
        let fields = &mut settings.screens[self.screens_selection].fields;
        let insert_pos = if fields.is_empty() {
            0
        } else {
            (self.columns_selection + 1).min(fields.len())
        };
        fields.insert(insert_pos, field);
        self.columns_selection = insert_pos;
        settings.changed = true;
        self.changed = true;

        // Enter moving mode so user can position the new column
        self.columns_moving = true;
        // Switch focus to columns panel
        self.screens_panel_focus = 1;
    }

    /// Move meter selection up in current panel
    fn meters_move_selection_up(&mut self, settings: &Settings) {
        let num_columns = settings.header_layout.num_columns();

        if self.meters_column_focus == num_columns {
            // Available meters panel
            if self.meters_available_selection > 0 {
                self.meters_available_selection -= 1;
                // Adjust scroll
                if (self.meters_available_selection as i32) < self.meters_available_scroll {
                    self.meters_available_scroll = self.meters_available_selection as i32;
                }
            }
        } else {
            // Column panel
            let col_idx = self.meters_column_focus;
            if let Some(sel) = self.meters_column_selection.get_mut(col_idx) {
                if *sel > 0 {
                    *sel -= 1;
                }
            }
        }
    }

    /// Move meter selection down in current panel
    fn meters_move_selection_down(&mut self, settings: &Settings) {
        let num_columns = settings.header_layout.num_columns();

        if self.meters_column_focus == num_columns {
            // Available meters panel
            let available_meters = available_meters_for_platform();
            if self.meters_available_selection < available_meters.len().saturating_sub(1) {
                self.meters_available_selection += 1;
                // Adjust scroll
                let display_height = self.content_panel.h - 1;
                if (self.meters_available_selection as i32)
                    >= self.meters_available_scroll + display_height
                {
                    self.meters_available_scroll =
                        self.meters_available_selection as i32 - display_height + 1;
                }
            }
        } else {
            // Column panel
            let col_idx = self.meters_column_focus;
            let max_index = settings
                .header_columns
                .get(col_idx)
                .map(|m| m.len().saturating_sub(1))
                .unwrap_or(0);
            if let Some(sel) = self.meters_column_selection.get_mut(col_idx) {
                if *sel < max_index {
                    *sel += 1;
                }
            }
        }
    }

    /// Add a meter from available meters to a specific column
    /// If target_column is None, adds to column 0 (leftmost)
    /// After adding, switches focus to that column and enters moving mode
    fn add_meter_to_column(
        &mut self,
        settings: &mut Settings,
        header: &mut Header,
        target_column: usize,
    ) {
        let available_meters = available_meters_for_platform();
        if self.meters_available_selection >= available_meters.len() {
            return;
        }

        let meter_info = available_meters[self.meters_available_selection];

        // Create meter config
        let config = MeterConfig {
            name: meter_info.name.to_string(),
            param: 0,
            mode: MeterMode::Bar,
        };

        // Ensure column exists
        while settings.header_columns.len() <= target_column {
            settings.header_columns.push(Vec::new());
        }

        // Add meter to column
        settings.header_columns[target_column].push(config);
        settings.changed = true;
        self.changed = true;

        // Get the new meter's position
        let new_position = settings.header_columns[target_column]
            .len()
            .saturating_sub(1);

        // Update selection in that column
        if let Some(sel) = self.meters_column_selection.get_mut(target_column) {
            *sel = new_position;
        }

        // Repopulate header
        header.populate_from_settings(settings);

        // Switch focus to the target column and enter moving mode (like C htop)
        self.meters_column_focus = target_column;
        self.meters_moving = true;
    }

    /// Delete the selected meter from current column
    fn delete_selected_meter(&mut self, settings: &mut Settings, header: &mut Header) {
        let col_idx = self.meters_column_focus;
        let selection = self
            .meters_column_selection
            .get(col_idx)
            .copied()
            .unwrap_or(0);

        if let Some(column) = settings.header_columns.get_mut(col_idx) {
            if selection < column.len() {
                column.remove(selection);
                settings.changed = true;
                self.changed = true;

                // Adjust selection
                if let Some(sel) = self.meters_column_selection.get_mut(col_idx) {
                    if *sel > 0 && *sel >= column.len() {
                        *sel = column.len().saturating_sub(1);
                    }
                }

                // Repopulate header
                header.populate_from_settings(settings);
            }
        }

        // Stop moving mode if we were moving
        self.meters_moving = false;
    }

    /// Cycle the meter style, skipping unsupported modes
    fn cycle_meter_style(&mut self, settings: &mut Settings, header: &mut Header) {
        let col_idx = self.meters_column_focus;
        let selection = self
            .meters_column_selection
            .get(col_idx)
            .copied()
            .unwrap_or(0);

        // Get the supported modes for this meter
        let supported_modes = header
            .get_meter_supported_modes(col_idx, selection)
            .unwrap_or(0xFFFF); // Default to all modes if not found

        if let Some(column) = settings.header_columns.get_mut(col_idx) {
            if let Some(meter) = column.get_mut(selection) {
                // Cycle order: Bar -> Text -> Graph -> Led -> Bar
                let modes = [
                    MeterMode::Bar,
                    MeterMode::Text,
                    MeterMode::Graph,
                    MeterMode::Led,
                ];

                // Find current mode index
                let current_idx = modes.iter().position(|&m| m == meter.mode).unwrap_or(0);

                // Find next supported mode
                for i in 1..=modes.len() {
                    let next_idx = (current_idx + i) % modes.len();
                    let next_mode = modes[next_idx];
                    if (supported_modes & (1 << next_mode as u32)) != 0 {
                        meter.mode = next_mode;
                        break;
                    }
                }

                settings.changed = true;
                self.changed = true;

                // Repopulate header
                header.populate_from_settings(settings);
            }
        }
    }

    /// Move meter up within its column
    fn move_meter_up(&mut self, settings: &mut Settings, header: &mut Header) {
        let col_idx = self.meters_column_focus;
        let selection = self
            .meters_column_selection
            .get(col_idx)
            .copied()
            .unwrap_or(0);

        if selection == 0 {
            return;
        }

        if let Some(column) = settings.header_columns.get_mut(col_idx) {
            if selection < column.len() {
                column.swap(selection, selection - 1);
                settings.changed = true;
                self.changed = true;

                // Update selection
                if let Some(sel) = self.meters_column_selection.get_mut(col_idx) {
                    *sel = selection - 1;
                }

                // Repopulate header
                header.populate_from_settings(settings);
            }
        }
    }

    /// Move meter down within its column
    fn move_meter_down(&mut self, settings: &mut Settings, header: &mut Header) {
        let col_idx = self.meters_column_focus;
        let selection = self
            .meters_column_selection
            .get(col_idx)
            .copied()
            .unwrap_or(0);

        if let Some(column) = settings.header_columns.get_mut(col_idx) {
            if selection + 1 < column.len() {
                column.swap(selection, selection + 1);
                settings.changed = true;
                self.changed = true;

                // Update selection
                if let Some(sel) = self.meters_column_selection.get_mut(col_idx) {
                    *sel = selection + 1;
                }

                // Repopulate header
                header.populate_from_settings(settings);
            }
        }
    }

    /// Move meter to the left column
    fn move_meter_left(&mut self, settings: &mut Settings, header: &mut Header) {
        let col_idx = self.meters_column_focus;
        if col_idx == 0 {
            return; // Can't move left from first column
        }

        let selection = self
            .meters_column_selection
            .get(col_idx)
            .copied()
            .unwrap_or(0);
        let target_col = col_idx - 1;

        // Remove from current column
        let meter = if let Some(column) = settings.header_columns.get_mut(col_idx) {
            if selection < column.len() {
                Some(column.remove(selection))
            } else {
                None
            }
        } else {
            None
        };

        // Add to target column
        if let Some(meter) = meter {
            // Ensure target column exists
            while settings.header_columns.len() <= target_col {
                settings.header_columns.push(Vec::new());
            }

            // Insert at same position or end
            let insert_pos = selection.min(settings.header_columns[target_col].len());
            settings.header_columns[target_col].insert(insert_pos, meter);
            settings.changed = true;
            self.changed = true;

            // Move focus to target column
            self.meters_column_focus = target_col;

            // Update selections
            if let Some(sel) = self.meters_column_selection.get_mut(target_col) {
                *sel = insert_pos;
            }
            if let Some(sel) = self.meters_column_selection.get_mut(col_idx) {
                let len = settings
                    .header_columns
                    .get(col_idx)
                    .map(|c| c.len())
                    .unwrap_or(0);
                if *sel >= len && len > 0 {
                    *sel = len - 1;
                }
            }

            // Repopulate header
            header.populate_from_settings(settings);
        }
    }

    /// Move meter to the right column
    fn move_meter_right(&mut self, settings: &mut Settings, header: &mut Header) {
        let col_idx = self.meters_column_focus;
        let num_columns = settings.header_layout.num_columns();

        if col_idx + 1 >= num_columns {
            return; // Can't move right from last column
        }

        let selection = self
            .meters_column_selection
            .get(col_idx)
            .copied()
            .unwrap_or(0);
        let target_col = col_idx + 1;

        // Remove from current column
        let meter = if let Some(column) = settings.header_columns.get_mut(col_idx) {
            if selection < column.len() {
                Some(column.remove(selection))
            } else {
                None
            }
        } else {
            None
        };

        // Add to target column
        if let Some(meter) = meter {
            // Ensure target column exists
            while settings.header_columns.len() <= target_col {
                settings.header_columns.push(Vec::new());
            }

            // Insert at same position or end
            let insert_pos = selection.min(settings.header_columns[target_col].len());
            settings.header_columns[target_col].insert(insert_pos, meter);
            settings.changed = true;
            self.changed = true;

            // Move focus to target column
            self.meters_column_focus = target_col;

            // Update selections
            if let Some(sel) = self.meters_column_selection.get_mut(target_col) {
                *sel = insert_pos;
            }
            if let Some(sel) = self.meters_column_selection.get_mut(col_idx) {
                let len = settings
                    .header_columns
                    .get(col_idx)
                    .map(|c| c.len())
                    .unwrap_or(0);
                if *sel >= len && len > 0 {
                    *sel = len - 1;
                }
            }

            // Repopulate header
            header.populate_from_settings(settings);
        }
    }

    fn get_content_count(&self) -> usize {
        match self.category {
            SetupCategory::DisplayOptions => self.display_options.len(),
            SetupCategory::Colors => COLOR_SCHEME_NAMES.len(),
            SetupCategory::HeaderLayout => HeaderLayout::all().len(),
            _ => 0,
        }
    }

    fn move_content_up(&mut self, _settings: &Settings) {
        if self.content_index > 0 {
            self.content_index -= 1;

            // Skip non-interactive items in display options
            if self.category == SetupCategory::DisplayOptions {
                while self.content_index > 0 {
                    if let Some(item) = self.display_options.get(self.content_index) {
                        if item.is_interactive() {
                            break;
                        }
                    }
                    self.content_index -= 1;
                }
                // Check if current is interactive, if not go back up
                if let Some(item) = self.display_options.get(self.content_index) {
                    if !item.is_interactive() && self.content_index > 0 {
                        // Can't find interactive item going up, stay where we were
                        self.content_index += 1;
                        self.skip_to_interactive(_settings, true);
                    }
                }
            }

            // Adjust scroll
            if (self.content_index as i32) < self.content_scroll {
                self.content_scroll = self.content_index as i32;
            }
        }
    }

    fn move_content_down(&mut self, _settings: &Settings, crt: &Crt) {
        let max_index = self.get_content_count();
        if max_index == 0 {
            return;
        }

        if self.content_index < max_index - 1 {
            self.content_index += 1;

            // Skip non-interactive items in display options
            if self.category == SetupCategory::DisplayOptions {
                while self.content_index < max_index - 1 {
                    if let Some(item) = self.display_options.get(self.content_index) {
                        if item.is_interactive() {
                            break;
                        }
                    }
                    self.content_index += 1;
                }
            }

            self.ensure_content_visible(crt);
        }
    }

    fn skip_to_interactive(&mut self, _settings: &Settings, forward: bool) {
        if self.category != SetupCategory::DisplayOptions {
            return;
        }

        let max_index = self.display_options.len();
        if forward {
            while self.content_index < max_index {
                if let Some(item) = self.display_options.get(self.content_index) {
                    if item.is_interactive() {
                        break;
                    }
                }
                self.content_index += 1;
            }
        } else {
            while self.content_index > 0 {
                if let Some(item) = self.display_options.get(self.content_index) {
                    if item.is_interactive() {
                        break;
                    }
                }
                self.content_index -= 1;
            }
        }
    }

    fn ensure_content_visible(&mut self, _crt: &Crt) {
        let display_height = self.content_panel.h - 1;
        let idx = self.content_index as i32;

        if idx < self.content_scroll {
            self.content_scroll = idx;
        } else if idx >= self.content_scroll + display_height {
            self.content_scroll = idx - display_height + 1;
        }
    }

    fn handle_toggle(&mut self, settings: &mut Settings, crt: &mut Crt, header: &mut Header) {
        match self.category {
            SetupCategory::DisplayOptions => {
                if let Some(item) = self.display_options.get(self.content_index) {
                    match item {
                        OptionItem::Check { field, .. } => {
                            field.toggle(settings);
                            settings.changed = true;
                            self.changed = true;
                        }
                        OptionItem::Number {
                            field, min, max, ..
                        } => {
                            // Toggle cycles through values
                            let current = field.get_int(settings);
                            let new_val = if current >= *max { *min } else { current + 1 };
                            field.set_int(settings, new_val);
                            settings.changed = true;
                            self.changed = true;
                        }
                        OptionItem::Text(_) => {}
                    }
                }
            }
            SetupCategory::Colors => {
                // Set color scheme
                let new_scheme = self.content_index as i32;
                settings.color_scheme = ColorScheme::from_i32(new_scheme);
                settings.changed = true;
                self.changed = true;

                // Update colors immediately
                crt.set_color_scheme(settings.color_scheme);
                clear();
            }
            SetupCategory::HeaderLayout => {
                // Set header layout (like C htop HeaderOptionsPanel_eventHandler)
                if let Some(new_layout) = HeaderLayout::from_index(self.content_index) {
                    let old_num_cols = settings.header_layout.num_columns();
                    let new_num_cols = new_layout.num_columns();

                    settings.header_layout = new_layout;
                    settings.changed = true;
                    self.changed = true;

                    // Handle column count changes (like C htop Header_setLayout)
                    if new_num_cols > old_num_cols {
                        // Add new empty columns
                        while settings.header_columns.len() < new_num_cols {
                            settings.header_columns.push(Vec::new());
                        }
                    } else if new_num_cols < old_num_cols {
                        // Move meters from removed columns to the last remaining column
                        // (matching C htop behavior)
                        for col_idx in (new_num_cols..old_num_cols).rev() {
                            if let Some(removed_meters) = settings.header_columns.get_mut(col_idx) {
                                // Take all meters from this column
                                let meters: Vec<_> = std::mem::take(removed_meters);
                                // Add them to the last remaining column
                                if let Some(last_col) =
                                    settings.header_columns.get_mut(new_num_cols - 1)
                                {
                                    for meter in meters.into_iter().rev() {
                                        last_col.push(meter);
                                    }
                                }
                            }
                        }
                        // Truncate to new column count
                        settings.header_columns.truncate(new_num_cols);
                    }

                    // Update header with new layout and meters
                    header.populate_from_settings(settings);
                }
            }
            _ => {}
        }
    }

    fn handle_decrease(&mut self, settings: &mut Settings) {
        if self.category == SetupCategory::DisplayOptions {
            if let Some(item) = self.display_options.get(self.content_index) {
                match item {
                    OptionItem::Number { field, min, .. } => {
                        let current = field.get_int(settings);
                        let new_val = (current - 1).max(*min);
                        field.set_int(settings, new_val);
                        settings.changed = true;
                        self.changed = true;
                    }
                    OptionItem::Check { field, .. } => {
                        field.set_bool(settings, false);
                        settings.changed = true;
                        self.changed = true;
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_increase(&mut self, settings: &mut Settings) {
        if self.category == SetupCategory::DisplayOptions {
            if let Some(item) = self.display_options.get(self.content_index) {
                match item {
                    OptionItem::Number { field, max, .. } => {
                        let current = field.get_int(settings);
                        let new_val = (current + 1).min(*max);
                        field.set_int(settings, new_val);
                        settings.changed = true;
                        self.changed = true;
                    }
                    OptionItem::Check { field, .. } => {
                        field.set_bool(settings, true);
                        settings.changed = true;
                        self.changed = true;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Run the setup screen
    pub fn run(
        &mut self,
        settings: &mut Settings,
        crt: &mut Crt,
        header: &mut Header,
        machine: &mut Machine,
    ) {
        // Get header height for layout
        let mut header_height = header.get_height();

        // Initial layout
        self.layout(crt, header_height, settings.screen_tabs);

        // Start with first interactive item selected
        self.skip_to_interactive(settings, true);

        // Track last update time for periodic scanning (like C htop checkRecalculation)
        // Use Instant::now() - delay to trigger immediate first scan
        let delay = std::time::Duration::from_millis(settings.delay as u64 * 100);
        let mut last_update = std::time::Instant::now() - delay;

        loop {
            // Check if we need to scan for new data (like C htop checkRecalculation)
            let elapsed = last_update.elapsed();
            if elapsed >= delay {
                // Scan machine for updated system data
                platform::scan(machine);
                // In setup screen, we don't need merged command strings
                machine.update_processes(None, "│");

                // Always update header meters to avoid gaps in graph meters
                // This matches C htop's checkRecalculation() which always calls Header_updateData()
                header.update(machine);

                last_update = std::time::Instant::now();
            }

            // Draw
            self.draw(crt, settings, header, machine);

            // Get input
            let key = getch();

            // Handle resize
            if key == KEY_RESIZE {
                crt.handle_resize();
                self.layout(crt, header_height, settings.screen_tabs);
                continue;
            }

            // Handle key
            let result = self.handle_key(key, settings, crt, header);

            // If settings were changed, update header (like C htop DisplayOptionsPanel)
            if result == HandlerResult::Handled && self.changed {
                // Update header margin setting if it changed
                header.set_header_margin(settings.header_margin);

                // Update meters with current machine data (needed for correct height calculation)
                // This must happen BEFORE calculate_height() because CPU meters need to know
                // the actual CPU count to report their correct height
                header.update(machine);

                // Recalculate header height (may change if layout or headerMargin changed)
                let new_height = header.calculate_height();
                // Always relayout when settings change, since header layout might have changed
                // even if height stays the same (e.g., when meters are redistributed)
                header_height = new_height;
                self.layout(crt, header_height, settings.screen_tabs);
            }

            if result == HandlerResult::BreakLoop {
                break;
            }
        }

        // Save settings if changed
        if self.changed {
            let _ = settings.write();
        }
    }
}
