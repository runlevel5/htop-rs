//! Settings module
//!
//! This module contains user-configurable settings for htop.
//!
//! Config file search order (matches C htop Settings.c):
//! 1. $HTOPRC environment variable (if set)
//! 2. $XDG_CONFIG_HOME/htop/htoprc (or $HOME/.config/htop/htoprc)
//! 3. Legacy $HOME/.htoprc (migrated if found)
//! 4. System-wide /etc/htoprc (fallback, read-only)

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use super::process::ProcessField;

/// Minimum config version we can read
const CONFIG_READER_MIN_VERSION: u32 = 3;

/// Header layout options (matches C htop HeaderLayout.h)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeaderLayout {
    OneColumn100,
    #[default]
    TwoColumns5050,
    TwoColumns3367,
    TwoColumns6733,
    ThreeColumns333433,
    ThreeColumns252550,
    ThreeColumns255025,
    ThreeColumns502525,
    ThreeColumns403030,
    ThreeColumns304030,
    ThreeColumns303040,
    ThreeColumns402040,
    FourColumns25252525,
}

impl HeaderLayout {
    /// Get the number of columns for this layout
    pub fn num_columns(self) -> usize {
        match self {
            HeaderLayout::OneColumn100 => 1,
            HeaderLayout::TwoColumns5050
            | HeaderLayout::TwoColumns3367
            | HeaderLayout::TwoColumns6733 => 2,
            HeaderLayout::ThreeColumns333433
            | HeaderLayout::ThreeColumns252550
            | HeaderLayout::ThreeColumns255025
            | HeaderLayout::ThreeColumns502525
            | HeaderLayout::ThreeColumns403030
            | HeaderLayout::ThreeColumns304030
            | HeaderLayout::ThreeColumns303040
            | HeaderLayout::ThreeColumns402040 => 3,
            HeaderLayout::FourColumns25252525 => 4,
        }
    }

    /// Get column width percentages
    pub fn column_widths(self) -> Vec<f64> {
        match self {
            HeaderLayout::OneColumn100 => vec![1.0],
            HeaderLayout::TwoColumns5050 => vec![0.50, 0.50],
            HeaderLayout::TwoColumns3367 => vec![0.33, 0.67],
            HeaderLayout::TwoColumns6733 => vec![0.67, 0.33],
            HeaderLayout::ThreeColumns333433 => vec![0.33, 0.34, 0.33],
            HeaderLayout::ThreeColumns252550 => vec![0.25, 0.25, 0.50],
            HeaderLayout::ThreeColumns255025 => vec![0.25, 0.50, 0.25],
            HeaderLayout::ThreeColumns502525 => vec![0.50, 0.25, 0.25],
            HeaderLayout::ThreeColumns403030 => vec![0.40, 0.30, 0.30],
            HeaderLayout::ThreeColumns304030 => vec![0.30, 0.40, 0.30],
            HeaderLayout::ThreeColumns303040 => vec![0.30, 0.30, 0.40],
            HeaderLayout::ThreeColumns402040 => vec![0.40, 0.20, 0.40],
            HeaderLayout::FourColumns25252525 => vec![0.25, 0.25, 0.25, 0.25],
        }
    }

    /// Get description for display in Setup screen
    pub fn description(self) -> &'static str {
        match self {
            HeaderLayout::OneColumn100 => "1 column  - full width",
            HeaderLayout::TwoColumns5050 => "2 columns - 50/50 (default)",
            HeaderLayout::TwoColumns3367 => "2 columns - 33/67",
            HeaderLayout::TwoColumns6733 => "2 columns - 67/33",
            HeaderLayout::ThreeColumns333433 => "3 columns - 33/34/33",
            HeaderLayout::ThreeColumns252550 => "3 columns - 25/25/50",
            HeaderLayout::ThreeColumns255025 => "3 columns - 25/50/25",
            HeaderLayout::ThreeColumns502525 => "3 columns - 50/25/25",
            HeaderLayout::ThreeColumns403030 => "3 columns - 40/30/30",
            HeaderLayout::ThreeColumns304030 => "3 columns - 30/40/30",
            HeaderLayout::ThreeColumns303040 => "3 columns - 30/30/40",
            HeaderLayout::ThreeColumns402040 => "3 columns - 40/20/40",
            HeaderLayout::FourColumns25252525 => "4 columns - 25/25/25/25",
        }
    }

    /// Get all header layouts
    pub fn all() -> &'static [HeaderLayout] {
        &[
            HeaderLayout::OneColumn100,
            HeaderLayout::TwoColumns5050,
            HeaderLayout::TwoColumns3367,
            HeaderLayout::TwoColumns6733,
            HeaderLayout::ThreeColumns333433,
            HeaderLayout::ThreeColumns252550,
            HeaderLayout::ThreeColumns255025,
            HeaderLayout::ThreeColumns502525,
            HeaderLayout::ThreeColumns403030,
            HeaderLayout::ThreeColumns304030,
            HeaderLayout::ThreeColumns303040,
            HeaderLayout::ThreeColumns402040,
            HeaderLayout::FourColumns25252525,
        ]
    }

    /// Convert from index
    pub fn from_index(index: usize) -> Option<Self> {
        Self::all().get(index).copied()
    }

    /// Get index of this layout
    pub fn to_index(self) -> usize {
        Self::all().iter().position(|&l| l == self).unwrap_or(0)
    }

    /// Get name for settings file
    pub fn name(self) -> &'static str {
        match self {
            HeaderLayout::OneColumn100 => "one_100",
            HeaderLayout::TwoColumns5050 => "two_50_50",
            HeaderLayout::TwoColumns3367 => "two_33_67",
            HeaderLayout::TwoColumns6733 => "two_67_33",
            HeaderLayout::ThreeColumns333433 => "three_33_34_33",
            HeaderLayout::ThreeColumns252550 => "three_25_25_50",
            HeaderLayout::ThreeColumns255025 => "three_25_50_25",
            HeaderLayout::ThreeColumns502525 => "three_50_25_25",
            HeaderLayout::ThreeColumns403030 => "three_40_30_30",
            HeaderLayout::ThreeColumns304030 => "three_30_40_30",
            HeaderLayout::ThreeColumns303040 => "three_30_30_40",
            HeaderLayout::ThreeColumns402040 => "three_40_20_40",
            HeaderLayout::FourColumns25252525 => "four_25_25_25_25",
        }
    }

    /// Parse from name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "one_100" => Some(HeaderLayout::OneColumn100),
            "two_50_50" => Some(HeaderLayout::TwoColumns5050),
            "two_33_67" => Some(HeaderLayout::TwoColumns3367),
            "two_67_33" => Some(HeaderLayout::TwoColumns6733),
            "three_33_34_33" => Some(HeaderLayout::ThreeColumns333433),
            "three_25_25_50" => Some(HeaderLayout::ThreeColumns252550),
            "three_25_50_25" => Some(HeaderLayout::ThreeColumns255025),
            "three_50_25_25" => Some(HeaderLayout::ThreeColumns502525),
            "three_40_30_30" => Some(HeaderLayout::ThreeColumns403030),
            "three_30_40_30" => Some(HeaderLayout::ThreeColumns304030),
            "three_30_30_40" => Some(HeaderLayout::ThreeColumns303040),
            "three_40_20_40" => Some(HeaderLayout::ThreeColumns402040),
            "four_25_25_25_25" => Some(HeaderLayout::FourColumns25252525),
            _ => None,
        }
    }
}

/// Color schemes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorScheme {
    #[default]
    Default,
    Monochrome,
    BlackOnWhite,
    LightTerminal,
    Midnight,
    BlackNight,
    BrokenGray,
    Nord,
}

impl ColorScheme {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => ColorScheme::Default,
            1 => ColorScheme::Monochrome,
            2 => ColorScheme::BlackOnWhite,
            3 => ColorScheme::LightTerminal,
            4 => ColorScheme::Midnight,
            5 => ColorScheme::BlackNight,
            6 => ColorScheme::BrokenGray,
            7 => ColorScheme::Nord,
            _ => ColorScheme::Default,
        }
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
}

impl MeterMode {
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => MeterMode::Bar,
            2 => MeterMode::Text,
            3 => MeterMode::Graph,
            4 => MeterMode::Led,
            _ => MeterMode::Bar,
        }
    }

    pub fn to_i32(self) -> i32 {
        match self {
            MeterMode::Bar => 1,
            MeterMode::Text => 2,
            MeterMode::Graph => 3,
            MeterMode::Led => 4,
        }
    }
}

/// Meter configuration
#[derive(Debug, Clone)]
pub struct MeterConfig {
    pub name: String,
    pub param: u32,
    pub mode: MeterMode,
}

/// Parse meter name with optional parameter, e.g., "CPU(1)" -> ("CPU", 1)
fn parse_meter_name(name: &str) -> (String, u32) {
    if let Some(open_paren) = name.find('(') {
        if let Some(close_paren) = name.find(')') {
            let base_name = name[..open_paren].to_string();
            let param_str = &name[open_paren + 1..close_paren];
            let param = param_str.parse::<u32>().unwrap_or(0);
            return (base_name, param);
        }
    }
    (name.to_string(), 0)
}

/// Format meter name with optional parameter for config file
fn format_meter_name(name: &str, param: u32) -> String {
    if param > 0 {
        format!("{}({})", name, param)
    } else {
        name.to_string()
    }
}

/// Screen settings (per-tab settings)
#[derive(Debug, Clone)]
pub struct ScreenSettings {
    pub heading: String,
    pub fields: Vec<ProcessField>,
    pub sort_key: ProcessField,
    pub tree_sort_key: ProcessField,
    pub direction: i32,
    pub tree_direction: i32,
    pub tree_view: bool,
    pub tree_view_always_by_pid: bool,
    pub all_branches_collapsed: bool,
}

impl ScreenSettings {
    /// Create the default "Main" screen
    pub fn main_screen() -> Self {
        #[cfg(target_os = "linux")]
        let fields = vec![
            ProcessField::Pid,
            ProcessField::User,
            ProcessField::Priority,
            ProcessField::Nice,
            ProcessField::MSize,
            ProcessField::MResident,
            ProcessField::MShare,
            ProcessField::State,
            ProcessField::PercentCpu,
            ProcessField::PercentMem,
            ProcessField::Time,
            ProcessField::Command,
        ];
        #[cfg(not(target_os = "linux"))]
        let fields = vec![
            ProcessField::Pid,
            ProcessField::User,
            ProcessField::Priority,
            ProcessField::Nice,
            ProcessField::MSize,
            ProcessField::MResident,
            ProcessField::State,
            ProcessField::PercentCpu,
            ProcessField::PercentMem,
            ProcessField::Time,
            ProcessField::Command,
        ];
        ScreenSettings {
            heading: "Main".to_string(),
            fields,
            sort_key: ProcessField::PercentCpu,
            tree_sort_key: ProcessField::Pid,
            direction: -1, // descending
            tree_direction: 1,
            tree_view: false,
            tree_view_always_by_pid: false,
            all_branches_collapsed: false,
        }
    }

    /// Create the "I/O" screen (like C htop)
    #[cfg(target_os = "linux")]
    pub fn io_screen() -> Self {
        ScreenSettings {
            heading: "I/O".to_string(),
            fields: vec![
                ProcessField::Pid,
                ProcessField::User,
                ProcessField::IOPriority,
                ProcessField::IORate,
                ProcessField::IOReadRate,
                ProcessField::IOWriteRate,
                ProcessField::PercentSwapDelay,
                ProcessField::PercentIODelay,
                ProcessField::Command,
            ],
            sort_key: ProcessField::IORate,
            tree_sort_key: ProcessField::Pid,
            direction: -1, // descending
            tree_direction: 1,
            tree_view: false,
            tree_view_always_by_pid: false,
            all_branches_collapsed: false,
        }
    }

    /// On non-Linux platforms, I/O screen is not available
    #[cfg(not(target_os = "linux"))]
    pub fn io_screen() -> Self {
        // Return Main screen as fallback on non-Linux
        Self::main_screen()
    }

    /// Get default screens for the platform
    pub fn default_screens() -> Vec<Self> {
        #[cfg(target_os = "linux")]
        {
            vec![Self::main_screen(), Self::io_screen()]
        }
        #[cfg(not(target_os = "linux"))]
        {
            vec![Self::main_screen()]
        }
    }
}

impl Default for ScreenSettings {
    fn default() -> Self {
        Self::main_screen()
    }
}

/// Main settings structure
#[derive(Debug, Clone)]
pub struct Settings {
    pub filename: Option<PathBuf>,
    pub changed: bool,
    pub readonly: bool,

    // Header configuration
    pub header_layout: HeaderLayout,
    pub header_columns: Vec<Vec<MeterConfig>>,

    // Screen settings
    pub screens: Vec<ScreenSettings>,
    pub active_screen: usize,

    // Display settings
    pub color_scheme: ColorScheme,
    pub delay: u32, // in tenths of a second
    pub enable_mouse: bool,
    pub allow_unicode: bool,
    pub hide_function_bar: i32, // 0 = show, 1 = hide on ESC, 2 = always hide
    pub header_margin: bool,
    pub screen_tabs: bool,

    // CPU display
    pub count_cpus_from_one: bool,
    pub detailed_cpu_time: bool,
    pub show_cpu_usage: bool,
    pub show_cpu_frequency: bool,
    pub show_cpu_temperature: bool,
    pub degree_fahrenheit: bool,
    pub account_guest_in_cpu_meter: bool,

    // Process display
    pub show_program_path: bool,
    pub show_thread_names: bool,
    pub shadow_other_users: bool,
    pub hide_kernel_threads: bool,
    pub hide_userland_threads: bool,
    pub hide_running_in_container: bool,
    pub highlight_base_name: bool,
    pub highlight_deleted_exe: bool,
    pub shadow_dist_path_prefix: bool,
    pub highlight_megabytes: bool,
    pub highlight_threads: bool,
    pub highlight_changes: bool,
    pub highlight_delay_secs: i32,
    pub find_comm_in_cmdline: bool,
    pub strip_exe_from_cmdline: bool,
    pub show_merged_command: bool,
    pub update_process_names: bool,
    pub show_cached_memory: bool,

    // Tree view
    pub tree_view: bool,
    pub tree_view_always_by_pid: bool,
    pub all_branches_collapsed: bool,

    // Sort settings
    pub sort_key: Option<ProcessField>,
    pub sort_descending: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings::new()
    }
}

impl Settings {
    pub fn new() -> Self {
        let default_meters_left = vec![
            MeterConfig {
                name: "LeftCPUs2".to_string(),
                param: 0,
                mode: MeterMode::Bar,
            },
            MeterConfig {
                name: "Memory".to_string(),
                param: 0,
                mode: MeterMode::Bar,
            },
            MeterConfig {
                name: "Swap".to_string(),
                param: 0,
                mode: MeterMode::Bar,
            },
        ];

        let default_meters_right = vec![
            MeterConfig {
                name: "RightCPUs2".to_string(),
                param: 0,
                mode: MeterMode::Bar,
            },
            MeterConfig {
                name: "Tasks".to_string(),
                param: 0,
                mode: MeterMode::Text,
            },
            MeterConfig {
                name: "LoadAverage".to_string(),
                param: 0,
                mode: MeterMode::Text,
            },
            MeterConfig {
                name: "Uptime".to_string(),
                param: 0,
                mode: MeterMode::Text,
            },
        ];

        // Find config file using C htop's search order
        let (filename, readonly) = match find_config_path() {
            Some(result) => (Some(result.path), result.readonly),
            None => (None, false),
        };

        Settings {
            filename,
            changed: false,
            readonly,
            header_layout: HeaderLayout::TwoColumns5050,
            header_columns: vec![default_meters_left, default_meters_right],
            screens: ScreenSettings::default_screens(),
            active_screen: 0,
            color_scheme: ColorScheme::Default,
            delay: 15, // 1.5 seconds
            enable_mouse: true,
            allow_unicode: true,
            hide_function_bar: 0,
            header_margin: true,
            screen_tabs: true,
            count_cpus_from_one: false,
            detailed_cpu_time: false,
            show_cpu_usage: true,
            show_cpu_frequency: false,
            show_cpu_temperature: false,
            degree_fahrenheit: false,
            account_guest_in_cpu_meter: false,
            show_program_path: true,
            show_thread_names: false,
            shadow_other_users: true,
            hide_kernel_threads: true,
            hide_userland_threads: false,
            hide_running_in_container: false,
            highlight_base_name: false,
            highlight_deleted_exe: true,
            shadow_dist_path_prefix: false,
            highlight_megabytes: true,
            highlight_threads: true,
            highlight_changes: false,
            highlight_delay_secs: 5,
            find_comm_in_cmdline: true,
            strip_exe_from_cmdline: true,
            show_merged_command: false,
            update_process_names: false,
            show_cached_memory: true,
            tree_view: false,
            tree_view_always_by_pid: false,
            all_branches_collapsed: false,
            sort_key: None,
            sort_descending: true,
        }
    }

    /// Get the preferred path for writing config (XDG location)
    fn get_write_path(&self) -> Option<PathBuf> {
        // If HTOPRC is set, use that
        if let Ok(htoprc) = std::env::var("HTOPRC") {
            return Some(PathBuf::from(htoprc));
        }

        // Otherwise use XDG config path
        if let Some(config_dir) = dirs::config_dir() {
            return Some(config_dir.join("htop").join("htoprc"));
        }

        // Fallback to ~/.config/htop/htoprc
        dirs::home_dir().map(|home| home.join(".config").join("htop").join("htoprc"))
    }

    /// Load settings from the config file
    pub fn load(&mut self) -> anyhow::Result<()> {
        let path = match &self.filename {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        if !path.exists() {
            return Ok(());
        }

        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);

        // Track config version for legacy format support
        let mut config_version: u32 = CONFIG_READER_MIN_VERSION;
        // Track if we found any screen definitions
        let mut found_screens = false;
        // Current screen being parsed (for .sort_key, etc.)
        let mut current_screen_idx: Option<usize> = None;
        // Temporary storage for legacy meter format
        let mut legacy_left_meters: Vec<String> = Vec::new();
        let mut legacy_right_meters: Vec<String> = Vec::new();
        let mut legacy_left_modes: Vec<i32> = Vec::new();
        let mut legacy_right_modes: Vec<i32> = Vec::new();
        // New-style column meters (indexed)
        let mut column_meters: std::collections::HashMap<usize, Vec<String>> =
            std::collections::HashMap::new();
        let mut column_modes: std::collections::HashMap<usize, Vec<i32>> =
            std::collections::HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for screen definition: "screen:Name=FIELD1 FIELD2 ..."
            if let Some(rest) = line.strip_prefix("screen:") {
                if let Some((name, fields_str)) = rest.split_once('=') {
                    found_screens = true;
                    // Clear default screens on first screen definition
                    if self.screens.len() == ScreenSettings::default_screens().len()
                        && current_screen_idx.is_none()
                    {
                        self.screens.clear();
                    }
                    let screen = self.parse_screen_definition(name.trim(), fields_str.trim());
                    self.screens.push(screen);
                    current_screen_idx = Some(self.screens.len() - 1);
                }
                continue;
            }

            // Check for screen property: ".sort_key=value"
            if line.starts_with('.') {
                if let Some(idx) = current_screen_idx {
                    if let Some((key, value)) = line[1..].split_once('=') {
                        self.parse_screen_property(idx, key.trim(), value.trim());
                    }
                }
                continue;
            }

            // Regular key=value pairs
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                // Check for config version
                if key == "config_reader_min_version" {
                    if let Ok(v) = value.parse::<u32>() {
                        config_version = v;
                    }
                    continue;
                }

                // Check for column_meters_N and column_meter_modes_N
                if let Some(idx_str) = key.strip_prefix("column_meters_") {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        let meters: Vec<String> =
                            value.split_whitespace().map(|s| s.to_string()).collect();
                        column_meters.insert(idx, meters);
                    }
                    continue;
                }

                if let Some(idx_str) = key.strip_prefix("column_meter_modes_") {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        let modes: Vec<i32> = value
                            .split_whitespace()
                            .filter_map(|s| s.parse::<i32>().ok())
                            .collect();
                        column_modes.insert(idx, modes);
                    }
                    continue;
                }

                // Legacy meter format
                if key == "left_meters" {
                    legacy_left_meters =
                        value.split_whitespace().map(|s| s.to_string()).collect();
                    continue;
                }
                if key == "right_meters" {
                    legacy_right_meters =
                        value.split_whitespace().map(|s| s.to_string()).collect();
                    continue;
                }
                if key == "left_meter_modes" {
                    legacy_left_modes = value
                        .split_whitespace()
                        .filter_map(|s| s.parse::<i32>().ok())
                        .collect();
                    continue;
                }
                if key == "right_meter_modes" {
                    legacy_right_modes = value
                        .split_whitespace()
                        .filter_map(|s| s.parse::<i32>().ok())
                        .collect();
                    continue;
                }

                // Legacy fields format (config_version <= 2)
                if key == "fields" && config_version < CONFIG_READER_MIN_VERSION {
                    self.parse_legacy_fields(value);
                    continue;
                }

                self.parse_setting(key, value);
            }
        }

        // Apply meter configurations
        if !column_meters.is_empty() {
            // New-style column meters
            self.apply_column_meters(&column_meters, &column_modes);
        } else if !legacy_left_meters.is_empty() || !legacy_right_meters.is_empty() {
            // Legacy two-column format
            self.apply_legacy_meters(
                &legacy_left_meters,
                &legacy_right_meters,
                &legacy_left_modes,
                &legacy_right_modes,
            );
        }

        // If no screens were defined, ensure we have default screens
        if !found_screens && self.screens.is_empty() {
            self.screens = ScreenSettings::default_screens();
        }

        Ok(())
    }

    /// Parse a screen definition line: "Name=FIELD1 FIELD2 ..."
    fn parse_screen_definition(&self, name: &str, fields_str: &str) -> ScreenSettings {
        let fields: Vec<ProcessField> = fields_str
            .split_whitespace()
            .filter_map(|s| ProcessField::from_name(s))
            .collect();

        ScreenSettings {
            heading: name.to_string(),
            fields: if fields.is_empty() {
                ScreenSettings::main_screen().fields
            } else {
                fields
            },
            sort_key: ProcessField::PercentCpu,
            tree_sort_key: ProcessField::Pid,
            direction: -1,
            tree_direction: 1,
            tree_view: false,
            tree_view_always_by_pid: false,
            all_branches_collapsed: false,
        }
    }

    /// Parse a screen property: sort_key, tree_sort_key, etc.
    fn parse_screen_property(&mut self, screen_idx: usize, key: &str, value: &str) {
        if screen_idx >= self.screens.len() {
            return;
        }

        let screen = &mut self.screens[screen_idx];

        match key {
            "sort_key" => {
                // Can be field name or numeric ID
                if let Some(field) = ProcessField::from_name(value) {
                    screen.sort_key = field;
                } else if let Ok(id) = value.parse::<u32>() {
                    if let Some(field) = ProcessField::from_id(id) {
                        screen.sort_key = field;
                    }
                }
            }
            "tree_sort_key" => {
                if let Some(field) = ProcessField::from_name(value) {
                    screen.tree_sort_key = field;
                } else if let Ok(id) = value.parse::<u32>() {
                    if let Some(field) = ProcessField::from_id(id) {
                        screen.tree_sort_key = field;
                    }
                }
            }
            "sort_direction" => {
                if let Ok(dir) = value.parse::<i32>() {
                    screen.direction = if dir >= 0 { 1 } else { -1 };
                }
            }
            "tree_sort_direction" => {
                if let Ok(dir) = value.parse::<i32>() {
                    screen.tree_direction = if dir >= 0 { 1 } else { -1 };
                }
            }
            "tree_view" => {
                screen.tree_view = value == "1";
            }
            "tree_view_always_by_pid" => {
                screen.tree_view_always_by_pid = value == "1";
            }
            "all_branches_collapsed" => {
                screen.all_branches_collapsed = value == "1";
            }
            _ => {}
        }
    }

    /// Parse legacy fields format (numeric IDs separated by spaces)
    fn parse_legacy_fields(&mut self, value: &str) {
        let fields: Vec<ProcessField> = value
            .split_whitespace()
            .filter_map(|s| s.parse::<u32>().ok())
            .filter_map(ProcessField::from_id)
            .filter(|f| *f != ProcessField::Pid) // 0 is used as terminator
            .collect();

        if !fields.is_empty() && !self.screens.is_empty() {
            self.screens[0].fields = fields;
        }
    }

    /// Apply column meters from new format
    fn apply_column_meters(
        &mut self,
        meters: &std::collections::HashMap<usize, Vec<String>>,
        modes: &std::collections::HashMap<usize, Vec<i32>>,
    ) {
        let num_columns = self.header_layout.num_columns();
        self.header_columns.clear();

        for col_idx in 0..num_columns {
            let col_meters = meters.get(&col_idx).cloned().unwrap_or_default();
            let col_modes = modes.get(&col_idx).cloned().unwrap_or_default();

            let configs: Vec<MeterConfig> = col_meters
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let mode_id = col_modes.get(i).copied().unwrap_or(1);
                    let (meter_name, param) = parse_meter_name(name);
                    MeterConfig {
                        name: meter_name,
                        param,
                        mode: MeterMode::from_i32(mode_id),
                    }
                })
                .collect();

            self.header_columns.push(configs);
        }
    }

    /// Apply legacy two-column meter format
    fn apply_legacy_meters(
        &mut self,
        left_meters: &[String],
        right_meters: &[String],
        left_modes: &[i32],
        right_modes: &[i32],
    ) {
        self.header_columns.clear();

        // Left column
        let left_configs: Vec<MeterConfig> = left_meters
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let mode_id = left_modes.get(i).copied().unwrap_or(1);
                let (meter_name, param) = parse_meter_name(name);
                MeterConfig {
                    name: meter_name,
                    param,
                    mode: MeterMode::from_i32(mode_id),
                }
            })
            .collect();

        // Right column
        let right_configs: Vec<MeterConfig> = right_meters
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let mode_id = right_modes.get(i).copied().unwrap_or(1);
                let (meter_name, param) = parse_meter_name(name);
                MeterConfig {
                    name: meter_name,
                    param,
                    mode: MeterMode::from_i32(mode_id),
                }
            })
            .collect();

        self.header_columns.push(left_configs);
        self.header_columns.push(right_configs);
    }

    /// Parse a single setting line
    fn parse_setting(&mut self, key: &str, value: &str) {
        match key {
            "delay" => {
                if let Ok(v) = value.parse::<u32>() {
                    self.delay = v.clamp(1, 100);
                }
            }
            "color_scheme" => {
                if let Ok(v) = value.parse::<i32>() {
                    self.color_scheme = ColorScheme::from_i32(v);
                }
            }
            "enable_mouse" => {
                self.enable_mouse = value == "1";
            }
            "tree_view" => {
                self.tree_view = value == "1";
            }
            "show_program_path" => {
                self.show_program_path = value == "1";
            }
            "shadow_other_users" => {
                self.shadow_other_users = value == "1";
            }
            "hide_kernel_threads" => {
                self.hide_kernel_threads = value == "1";
            }
            "hide_userland_threads" => {
                self.hide_userland_threads = value == "1";
            }
            "highlight_base_name" => {
                self.highlight_base_name = value == "1";
            }
            "highlight_megabytes" => {
                self.highlight_megabytes = value == "1";
            }
            "highlight_threads" => {
                self.highlight_threads = value == "1";
            }
            "highlight_changes" => {
                self.highlight_changes = value == "1";
            }
            "highlight_changes_delay_secs" => {
                if let Ok(v) = value.parse::<i32>() {
                    self.highlight_delay_secs = v.max(1);
                }
            }
            "detailed_cpu_time" => {
                self.detailed_cpu_time = value == "1";
            }
            "count_cpus_from_one" => {
                self.count_cpus_from_one = value == "1";
            }
            "show_cpu_usage" => {
                self.show_cpu_usage = value == "1";
            }
            "show_cpu_frequency" => {
                self.show_cpu_frequency = value == "1";
            }
            "header_margin" => {
                self.header_margin = value == "1";
            }
            "screen_tabs" => {
                self.screen_tabs = value == "1";
            }
            "show_thread_names" => {
                self.show_thread_names = value == "1";
            }
            "account_guest_in_cpu_meter" => {
                self.account_guest_in_cpu_meter = value == "1";
            }
            "show_cached_memory" => {
                self.show_cached_memory = value == "1";
            }
            "update_process_names" => {
                self.update_process_names = value == "1";
            }
            "hide_function_bar" => {
                if let Ok(v) = value.parse::<i32>() {
                    self.hide_function_bar = v.clamp(0, 2);
                }
            }
            "show_merged_command" => {
                self.show_merged_command = value == "1";
            }
            "highlight_deleted_exe" => {
                self.highlight_deleted_exe = value == "1";
            }
            "shadow_dist_path_prefix" => {
                self.shadow_dist_path_prefix = value == "1";
            }
            "find_comm_in_cmdline" => {
                self.find_comm_in_cmdline = value == "1";
            }
            "strip_exe_from_cmdline" => {
                self.strip_exe_from_cmdline = value == "1";
            }
            "header_layout" => {
                // Try to parse as name first, then as numeric index
                if let Some(layout) = HeaderLayout::from_name(value) {
                    self.header_layout = layout;
                } else if let Ok(idx) = value.parse::<usize>() {
                    if let Some(layout) = HeaderLayout::from_index(idx) {
                        self.header_layout = layout;
                    }
                }
            }
            "sort_key" => {
                // Global sort key (for legacy format or first screen)
                if let Some(field) = ProcessField::from_name(value) {
                    self.sort_key = Some(field);
                    // Also update first screen
                    if !self.screens.is_empty() {
                        self.screens[0].sort_key = field;
                    }
                } else if let Ok(id) = value.parse::<u32>() {
                    if let Some(field) = ProcessField::from_id(id) {
                        self.sort_key = Some(field);
                        if !self.screens.is_empty() {
                            self.screens[0].sort_key = field;
                        }
                    }
                }
            }
            "tree_sort_key" => {
                if let Some(field) = ProcessField::from_name(value) {
                    if !self.screens.is_empty() {
                        self.screens[0].tree_sort_key = field;
                    }
                } else if let Ok(id) = value.parse::<u32>() {
                    if let Some(field) = ProcessField::from_id(id) {
                        if !self.screens.is_empty() {
                            self.screens[0].tree_sort_key = field;
                        }
                    }
                }
            }
            "sort_direction" => {
                if let Ok(dir) = value.parse::<i32>() {
                    self.sort_descending = dir < 0;
                    if !self.screens.is_empty() {
                        self.screens[0].direction = if dir >= 0 { 1 } else { -1 };
                    }
                }
            }
            "tree_sort_direction" => {
                if let Ok(dir) = value.parse::<i32>() {
                    if !self.screens.is_empty() {
                        self.screens[0].tree_direction = if dir >= 0 { 1 } else { -1 };
                    }
                }
            }
            "tree_view_always_by_pid" => {
                self.tree_view_always_by_pid = value == "1";
                if !self.screens.is_empty() {
                    self.screens[0].tree_view_always_by_pid = value == "1";
                }
            }
            "all_branches_collapsed" => {
                self.all_branches_collapsed = value == "1";
                if !self.screens.is_empty() {
                    self.screens[0].all_branches_collapsed = value == "1";
                }
            }
            "cpu_count_from_one" => {
                // Alternative name used in some configs
                self.count_cpus_from_one = value == "1";
            }
            "show_cpu_temperature" => {
                self.show_cpu_temperature = value == "1";
            }
            "degree_fahrenheit" => {
                self.degree_fahrenheit = value == "1";
            }
            "hide_running_in_container" => {
                self.hide_running_in_container = value == "1";
            }
            "shadow_distribution_path_prefix" => {
                // Alternative name from C htop
                self.shadow_dist_path_prefix = value == "1";
            }
            _ => {}
        }
    }

    /// Write settings to the config file
    /// Matches C htop's Settings_write() format
    pub fn write(&self) -> anyhow::Result<()> {
        if self.readonly {
            return Ok(());
        }

        // Get the write path (prefer XDG location, handles migration from legacy)
        let path = match self.get_write_path() {
            Some(p) => p,
            None => return Ok(()),
        };

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temporary file first for atomic operation
        let temp_path = path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path)?;

        // Header comment (matches C htop)
        writeln!(
            file,
            "# Beware! This file is rewritten by htop when settings are changed in the interface."
        )?;
        writeln!(
            file,
            "# The parser is also very primitive, and not human-friendly."
        )?;

        // Version info
        writeln!(file, "htop_version=3.3.0")?;
        writeln!(
            file,
            "config_reader_min_version={}",
            CONFIG_READER_MIN_VERSION
        )?;

        // Boolean settings (in C htop order)
        writeln!(
            file,
            "hide_kernel_threads={}",
            if self.hide_kernel_threads { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "hide_userland_threads={}",
            if self.hide_userland_threads { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "hide_running_in_container={}",
            if self.hide_running_in_container { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "shadow_other_users={}",
            if self.shadow_other_users { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "show_thread_names={}",
            if self.show_thread_names { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "show_program_path={}",
            if self.show_program_path { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "highlight_base_name={}",
            if self.highlight_base_name { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "highlight_deleted_exe={}",
            if self.highlight_deleted_exe { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "shadow_distribution_path_prefix={}",
            if self.shadow_dist_path_prefix { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "highlight_megabytes={}",
            if self.highlight_megabytes { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "highlight_threads={}",
            if self.highlight_threads { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "highlight_changes={}",
            if self.highlight_changes { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "highlight_changes_delay_secs={}",
            self.highlight_delay_secs
        )?;
        writeln!(
            file,
            "find_comm_in_cmdline={}",
            if self.find_comm_in_cmdline { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "strip_exe_from_cmdline={}",
            if self.strip_exe_from_cmdline { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "show_merged_command={}",
            if self.show_merged_command { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "header_margin={}",
            if self.header_margin { 1 } else { 0 }
        )?;
        writeln!(file, "screen_tabs={}", if self.screen_tabs { 1 } else { 0 })?;
        writeln!(
            file,
            "detailed_cpu_time={}",
            if self.detailed_cpu_time { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "cpu_count_from_one={}",
            if self.count_cpus_from_one { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "show_cpu_usage={}",
            if self.show_cpu_usage { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "show_cpu_frequency={}",
            if self.show_cpu_frequency { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "show_cpu_temperature={}",
            if self.show_cpu_temperature { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "degree_fahrenheit={}",
            if self.degree_fahrenheit { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "update_process_names={}",
            if self.update_process_names { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "account_guest_in_cpu_meter={}",
            if self.account_guest_in_cpu_meter { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "enable_mouse={}",
            if self.enable_mouse { 1 } else { 0 }
        )?;

        // Integer settings
        writeln!(file, "delay={}", self.delay)?;
        writeln!(file, "color_scheme={}", self.color_scheme as i32)?;
        writeln!(file, "hide_function_bar={}", self.hide_function_bar)?;

        // Header layout
        writeln!(file, "header_layout={}", self.header_layout.name())?;

        // Meter columns
        for (idx, column) in self.header_columns.iter().enumerate() {
            // Write meter names
            let meter_names: Vec<String> = column
                .iter()
                .map(|m| format_meter_name(&m.name, m.param))
                .collect();
            writeln!(file, "column_meters_{}={}", idx, meter_names.join(" "))?;

            // Write meter modes
            let meter_modes: Vec<String> = column.iter().map(|m| m.mode.to_i32().to_string()).collect();
            writeln!(file, "column_meter_modes_{}={}", idx, meter_modes.join(" "))?;
        }

        // Screen definitions
        for screen in &self.screens {
            // Screen header: "screen:Name=FIELD1 FIELD2 ..."
            let field_names: Vec<&str> = screen.fields.iter().map(|f| f.name()).collect();
            writeln!(file, "screen:{}={}", screen.heading, field_names.join(" "))?;

            // Screen properties
            writeln!(file, ".sort_key={}", screen.sort_key.name())?;
            writeln!(file, ".tree_sort_key={}", screen.tree_sort_key.name())?;
            writeln!(file, ".sort_direction={}", screen.direction)?;
            writeln!(file, ".tree_sort_direction={}", screen.tree_direction)?;
            writeln!(file, ".tree_view={}", if screen.tree_view { 1 } else { 0 })?;
            writeln!(
                file,
                ".tree_view_always_by_pid={}",
                if screen.tree_view_always_by_pid { 1 } else { 0 }
            )?;
            writeln!(
                file,
                ".all_branches_collapsed={}",
                if screen.all_branches_collapsed { 1 } else { 0 }
            )?;
        }

        // Ensure file is fully written
        file.sync_all()?;
        drop(file);

        // Atomic rename
        fs::rename(&temp_path, &path)?;

        Ok(())
    }

    /// Get the current screen settings
    pub fn current_screen(&self) -> &ScreenSettings {
        &self.screens[self.active_screen]
    }

    /// Get mutable current screen settings
    pub fn current_screen_mut(&mut self) -> &mut ScreenSettings {
        &mut self.screens[self.active_screen]
    }
}

// Stub for dirs crate functionality
mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|h| h.join(".config")))
    }

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Config file search result
struct ConfigSearchResult {
    path: PathBuf,
    readonly: bool,
}

/// Find config file following C htop's search order:
/// 1. $HTOPRC environment variable
/// 2. $XDG_CONFIG_HOME/htop/htoprc
/// 3. Legacy $HOME/.htoprc (will be migrated)
/// 4. /etc/htoprc (read-only fallback)
fn find_config_path() -> Option<ConfigSearchResult> {
    // 1. Check HTOPRC environment variable
    if let Ok(htoprc) = std::env::var("HTOPRC") {
        let path = PathBuf::from(&htoprc);
        if path.exists() {
            return Some(ConfigSearchResult {
                path,
                readonly: false,
            });
        }
        // HTOPRC is set but doesn't exist - use it as target for writing
        return Some(ConfigSearchResult {
            path,
            readonly: false,
        });
    }

    // 2. Check XDG_CONFIG_HOME/htop/htoprc
    if let Some(config_dir) = dirs::config_dir() {
        let xdg_path = config_dir.join("htop").join("htoprc");
        if xdg_path.exists() {
            return Some(ConfigSearchResult {
                path: xdg_path,
                readonly: false,
            });
        }

        // 3. Check legacy ~/.htoprc
        if let Some(home) = dirs::home_dir() {
            let legacy_path = home.join(".htoprc");
            if legacy_path.exists() {
                // Legacy file exists - we'll use it but prefer XDG path for writing
                // (migration happens on write)
                return Some(ConfigSearchResult {
                    path: legacy_path,
                    readonly: false,
                });
            }
        }

        // XDG path doesn't exist but is our preferred location
        return Some(ConfigSearchResult {
            path: xdg_path,
            readonly: false,
        });
    }

    // 4. Fall back to system-wide config (read-only)
    let etc_path = PathBuf::from("/etc/htoprc");
    if etc_path.exists() {
        return Some(ConfigSearchResult {
            path: etc_path,
            readonly: true,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== MeterMode Tests ====================

    #[test]
    fn test_meter_mode_default() {
        let mode: MeterMode = Default::default();
        assert_eq!(mode, MeterMode::Bar);
    }

    #[test]
    fn test_meter_mode_from_i32_valid() {
        assert_eq!(MeterMode::from_i32(1), MeterMode::Bar);
        assert_eq!(MeterMode::from_i32(2), MeterMode::Text);
        assert_eq!(MeterMode::from_i32(3), MeterMode::Graph);
        assert_eq!(MeterMode::from_i32(4), MeterMode::Led);
    }

    #[test]
    fn test_meter_mode_from_i32_invalid() {
        // Invalid values should default to Bar
        assert_eq!(MeterMode::from_i32(0), MeterMode::Bar);
        assert_eq!(MeterMode::from_i32(5), MeterMode::Bar);
        assert_eq!(MeterMode::from_i32(-1), MeterMode::Bar);
        assert_eq!(MeterMode::from_i32(100), MeterMode::Bar);
    }

    #[test]
    fn test_meter_mode_to_i32() {
        assert_eq!(MeterMode::Bar.to_i32(), 1);
        assert_eq!(MeterMode::Text.to_i32(), 2);
        assert_eq!(MeterMode::Graph.to_i32(), 3);
        assert_eq!(MeterMode::Led.to_i32(), 4);
    }

    #[test]
    fn test_meter_mode_roundtrip() {
        // Test that from_i32(to_i32(mode)) == mode
        for mode in [MeterMode::Bar, MeterMode::Text, MeterMode::Graph, MeterMode::Led] {
            let i = mode.to_i32();
            let recovered = MeterMode::from_i32(i);
            assert_eq!(recovered, mode, "Roundtrip failed for {:?}", mode);
        }
    }

    #[test]
    fn test_meter_mode_clone_copy() {
        let mode = MeterMode::Graph;
        let cloned = mode.clone();
        let copied = mode;
        assert_eq!(mode, cloned);
        assert_eq!(mode, copied);
    }

    #[test]
    fn test_meter_mode_debug() {
        assert_eq!(format!("{:?}", MeterMode::Bar), "Bar");
        assert_eq!(format!("{:?}", MeterMode::Text), "Text");
        assert_eq!(format!("{:?}", MeterMode::Graph), "Graph");
        assert_eq!(format!("{:?}", MeterMode::Led), "Led");
    }

    // ==================== MeterConfig Tests ====================

    #[test]
    fn test_meter_config_creation() {
        let config = MeterConfig {
            name: "CPU".to_string(),
            param: 0,
            mode: MeterMode::Bar,
        };
        assert_eq!(config.name, "CPU");
        assert_eq!(config.param, 0);
        assert_eq!(config.mode, MeterMode::Bar);
    }

    #[test]
    fn test_meter_config_with_param() {
        let config = MeterConfig {
            name: "CPU".to_string(),
            param: 2,
            mode: MeterMode::Graph,
        };
        assert_eq!(config.name, "CPU");
        assert_eq!(config.param, 2);
        assert_eq!(config.mode, MeterMode::Graph);
    }

    #[test]
    fn test_meter_config_clone() {
        let config = MeterConfig {
            name: "Memory".to_string(),
            param: 0,
            mode: MeterMode::Text,
        };
        let cloned = config.clone();
        assert_eq!(cloned.name, config.name);
        assert_eq!(cloned.param, config.param);
        assert_eq!(cloned.mode, config.mode);
    }

    // ==================== parse_meter_name Tests ====================

    #[test]
    fn test_parse_meter_name_simple() {
        let (name, param) = parse_meter_name("CPU");
        assert_eq!(name, "CPU");
        assert_eq!(param, 0);
    }

    #[test]
    fn test_parse_meter_name_with_param() {
        let (name, param) = parse_meter_name("CPU(1)");
        assert_eq!(name, "CPU");
        assert_eq!(param, 1);
    }

    #[test]
    fn test_parse_meter_name_with_larger_param() {
        let (name, param) = parse_meter_name("CPU(42)");
        assert_eq!(name, "CPU");
        assert_eq!(param, 42);
    }

    #[test]
    fn test_parse_meter_name_with_zero_param() {
        let (name, param) = parse_meter_name("Memory(0)");
        assert_eq!(name, "Memory");
        assert_eq!(param, 0);
    }

    #[test]
    fn test_parse_meter_name_invalid_param() {
        // Non-numeric param should default to 0
        let (name, param) = parse_meter_name("CPU(abc)");
        assert_eq!(name, "CPU");
        assert_eq!(param, 0);
    }

    #[test]
    fn test_parse_meter_name_empty_param() {
        // Empty param should default to 0
        let (name, param) = parse_meter_name("CPU()");
        assert_eq!(name, "CPU");
        assert_eq!(param, 0);
    }

    #[test]
    fn test_parse_meter_name_unclosed_paren() {
        // Unclosed paren should be treated as plain name
        let (name, param) = parse_meter_name("CPU(1");
        assert_eq!(name, "CPU(1");
        assert_eq!(param, 0);
    }

    #[test]
    fn test_parse_meter_name_various_meters() {
        assert_eq!(parse_meter_name("Memory"), ("Memory".to_string(), 0));
        assert_eq!(parse_meter_name("Swap"), ("Swap".to_string(), 0));
        assert_eq!(parse_meter_name("LoadAverage"), ("LoadAverage".to_string(), 0));
        assert_eq!(parse_meter_name("AllCPUs"), ("AllCPUs".to_string(), 0));
        assert_eq!(parse_meter_name("AllCPUs2"), ("AllCPUs2".to_string(), 0));
    }

    // ==================== format_meter_name Tests ====================

    #[test]
    fn test_format_meter_name_no_param() {
        assert_eq!(format_meter_name("CPU", 0), "CPU");
        assert_eq!(format_meter_name("Memory", 0), "Memory");
        assert_eq!(format_meter_name("Swap", 0), "Swap");
    }

    #[test]
    fn test_format_meter_name_with_param() {
        assert_eq!(format_meter_name("CPU", 1), "CPU(1)");
        assert_eq!(format_meter_name("CPU", 2), "CPU(2)");
        assert_eq!(format_meter_name("CPU", 42), "CPU(42)");
    }

    #[test]
    fn test_format_meter_name_roundtrip() {
        // Test that parse_meter_name(format_meter_name(name, param)) == (name, param)
        let test_cases = [
            ("CPU", 0),
            ("CPU", 1),
            ("CPU", 10),
            ("Memory", 0),
            ("LoadAverage", 0),
        ];

        for (name, param) in test_cases {
            let formatted = format_meter_name(name, param);
            let (parsed_name, parsed_param) = parse_meter_name(&formatted);
            assert_eq!(parsed_name, name, "Name roundtrip failed for ({}, {})", name, param);
            assert_eq!(parsed_param, param, "Param roundtrip failed for ({}, {})", name, param);
        }
    }

    // ==================== ScreenSettings Tests ====================

    #[test]
    fn test_screen_settings_main_screen() {
        let screen = ScreenSettings::main_screen();
        assert_eq!(screen.heading, "Main");
        assert!(!screen.fields.is_empty());
        assert!(screen.fields.contains(&ProcessField::Pid));
        assert!(screen.fields.contains(&ProcessField::Command));
        assert!(screen.fields.contains(&ProcessField::PercentCpu));
        assert_eq!(screen.sort_key, ProcessField::PercentCpu);
        assert_eq!(screen.tree_sort_key, ProcessField::Pid);
        assert_eq!(screen.direction, -1); // Descending
        assert_eq!(screen.tree_direction, 1); // Ascending
        assert!(!screen.tree_view);
        assert!(!screen.tree_view_always_by_pid); // Default is false
        assert!(!screen.all_branches_collapsed);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_screen_settings_io_screen() {
        let screen = ScreenSettings::io_screen();
        assert_eq!(screen.heading, "I/O");
        assert!(!screen.fields.is_empty());
        assert!(screen.fields.contains(&ProcessField::Pid));
        assert!(screen.fields.contains(&ProcessField::IORate));
        assert_eq!(screen.sort_key, ProcessField::IORate);
    }

    #[test]
    fn test_screen_settings_main_screen_has_required_fields() {
        let screen = ScreenSettings::main_screen();
        
        // Required fields for basic operation
        let required_fields = [
            ProcessField::Pid,
            ProcessField::User,
            ProcessField::PercentCpu,
            ProcessField::PercentMem,
            ProcessField::Time,
            ProcessField::Command,
        ];

        for field in required_fields {
            assert!(
                screen.fields.contains(&field),
                "Main screen should contain {:?}",
                field
            );
        }
    }

    #[test]
    fn test_screen_settings_clone() {
        let screen = ScreenSettings::main_screen();
        let cloned = screen.clone();
        
        assert_eq!(cloned.heading, screen.heading);
        assert_eq!(cloned.fields, screen.fields);
        assert_eq!(cloned.sort_key, screen.sort_key);
        assert_eq!(cloned.tree_view, screen.tree_view);
    }
}
