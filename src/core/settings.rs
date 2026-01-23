//! Settings module
//!
//! This module contains user-configurable settings for htop.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use super::process::ProcessField;

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
}

/// Meter configuration
#[derive(Debug, Clone)]
pub struct MeterConfig {
    pub name: String,
    pub param: u32,
    pub mode: MeterMode,
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

        Settings {
            filename: Self::default_config_path(),
            changed: false,
            readonly: false,
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

    /// Get the default config file path
    fn default_config_path() -> Option<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            let htop_dir = config_dir.join("htop");
            Some(htop_dir.join("htoprc"))
        } else {
            dirs::home_dir().map(|home| home.join(".config").join("htop").join("htoprc"))
        }
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

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                self.parse_setting(key.trim(), value.trim());
            }
        }

        Ok(())
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
            _ => {}
        }
    }

    /// Write settings to the config file
    pub fn write(&self) -> anyhow::Result<()> {
        if self.readonly {
            return Ok(());
        }

        let path = match &self.filename {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(&path)?;

        writeln!(file, "# htop-rs configuration file")?;
        writeln!(file, "# Automatically generated by htop-rs")?;
        writeln!(file)?;

        writeln!(file, "delay={}", self.delay)?;
        writeln!(file, "color_scheme={}", self.color_scheme as i32)?;
        writeln!(
            file,
            "enable_mouse={}",
            if self.enable_mouse { 1 } else { 0 }
        )?;
        writeln!(file, "tree_view={}", if self.tree_view { 1 } else { 0 })?;
        writeln!(
            file,
            "show_program_path={}",
            if self.show_program_path { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "shadow_other_users={}",
            if self.shadow_other_users { 1 } else { 0 }
        )?;
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
            "highlight_base_name={}",
            if self.highlight_base_name { 1 } else { 0 }
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
            "detailed_cpu_time={}",
            if self.detailed_cpu_time { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "count_cpus_from_one={}",
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
            "header_margin={}",
            if self.header_margin { 1 } else { 0 }
        )?;
        writeln!(file, "screen_tabs={}", if self.screen_tabs { 1 } else { 0 })?;
        writeln!(
            file,
            "show_thread_names={}",
            if self.show_thread_names { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "account_guest_in_cpu_meter={}",
            if self.account_guest_in_cpu_meter {
                1
            } else {
                0
            }
        )?;
        writeln!(
            file,
            "show_cached_memory={}",
            if self.show_cached_memory { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "update_process_names={}",
            if self.update_process_names { 1 } else { 0 }
        )?;
        writeln!(file, "hide_function_bar={}", self.hide_function_bar)?;
        writeln!(
            file,
            "show_merged_command={}",
            if self.show_merged_command { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "highlight_deleted_exe={}",
            if self.highlight_deleted_exe { 1 } else { 0 }
        )?;
        writeln!(
            file,
            "shadow_dist_path_prefix={}",
            if self.shadow_dist_path_prefix { 1 } else { 0 }
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
