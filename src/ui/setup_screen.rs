//! Setup Screen - F2 configuration screen
//!
//! This module implements the htop Setup screen with:
//! - Categories panel on the left (Display options, Header layout, Meters, Screens, Colors)
//! - Content panel(s) on the right depending on selected category

#![allow(dead_code)]

use ncurses::*;

use super::crt::{ColorElement, KEY_F10, KEY_F7, KEY_F8};
use super::function_bar::FunctionBar;
use super::header::Header;
use super::panel::{HandlerResult, Panel};
use super::rich_string::RichString;
use super::Crt;
use crate::core::{ColorScheme, HeaderLayout, Machine, MeterConfig, MeterMode, Settings};

// Key constants for pattern matching
const KEY_ESC: i32 = 27;
const KEY_TAB: i32 = b'\t' as i32;
const KEY_ENTER_LF: i32 = b'\n' as i32;
const KEY_ENTER_CR: i32 = b'\r' as i32;
const KEY_SPACE: i32 = b' ' as i32;
const KEY_MINUS: i32 = b'-' as i32;
const KEY_PLUS: i32 = b'+' as i32;
const KEY_Q: i32 = b'q' as i32;

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

/// List of all available meters (matching C htop Platform_meterTypes)
pub const AVAILABLE_METERS: &[MeterInfo] = &[
    MeterInfo::with_param("CPU", "CPU", "CPU usage for a specific core"),
    MeterInfo::new("AllCPUs", "CPUs (1/1)", "All CPUs in a single row"),
    MeterInfo::new("AllCPUs2", "CPUs (1/2)", "All CPUs in 2 rows"),
    MeterInfo::new("AllCPUs4", "CPUs (1/4)", "All CPUs in 4 rows"),
    MeterInfo::new("AllCPUs8", "CPUs (1/8)", "All CPUs in 8 rows"),
    MeterInfo::new("LeftCPUs", "Left CPUs (1/1)", "Left half of CPUs (1 row)"),
    MeterInfo::new("LeftCPUs2", "Left CPUs (1/2)", "Left half of CPUs (2 rows)"),
    MeterInfo::new("LeftCPUs4", "Left CPUs (1/4)", "Left half of CPUs (4 rows)"),
    MeterInfo::new("LeftCPUs8", "Left CPUs (1/8)", "Left half of CPUs (8 rows)"),
    MeterInfo::new(
        "RightCPUs",
        "Right CPUs (1/1)",
        "Right half of CPUs (1 row)",
    ),
    MeterInfo::new(
        "RightCPUs2",
        "Right CPUs (1/2)",
        "Right half of CPUs (2 rows)",
    ),
    MeterInfo::new(
        "RightCPUs4",
        "Right CPUs (1/4)",
        "Right half of CPUs (4 rows)",
    ),
    MeterInfo::new(
        "RightCPUs8",
        "Right CPUs (1/8)",
        "Right half of CPUs (8 rows)",
    ),
    MeterInfo::new("Memory", "Memory", "Memory usage"),
    MeterInfo::new("Swap", "Swap", "Swap usage"),
    MeterInfo::new("LoadAverage", "Load average", "System load averages"),
    MeterInfo::new("Tasks", "Task counter", "Running/total tasks"),
    MeterInfo::new("Uptime", "Uptime", "System uptime"),
    MeterInfo::new("Battery", "Battery", "Battery charge level"),
    MeterInfo::new("Hostname", "Hostname", "System hostname"),
    MeterInfo::new("Clock", "Clock", "Current time"),
    MeterInfo::new("Date", "Date", "Current date"),
    MeterInfo::new("DateTime", "Date and Time", "Current date and time"),
    MeterInfo::new("Blank", "Blank", "Empty spacer"),
];

/// Get the display name for a meter by its internal name
fn meter_display_name(name: &str, mode: MeterMode) -> String {
    let base_name = match name {
        "CPU" => "CPU",
        "AllCPUs" => "CPUs (1/1)",
        "AllCPUs2" => "CPUs (1/2)",
        "AllCPUs4" => "CPUs (1/4)",
        "AllCPUs8" => "CPUs (1/8)",
        "LeftCPUs" => "Left CPUs (1/1)",
        "LeftCPUs2" => "Left CPUs (1/2)",
        "LeftCPUs4" => "Left CPUs (1/4)",
        "LeftCPUs8" => "Left CPUs (1/8)",
        "RightCPUs" => "Right CPUs (1/1)",
        "RightCPUs2" => "Right CPUs (1/2)",
        "RightCPUs4" => "Right CPUs (1/4)",
        "RightCPUs8" => "Right CPUs (1/8)",
        "Memory" => "Memory",
        "Swap" => "Swap",
        "LoadAverage" => "Load average",
        "Tasks" => "Task counter",
        "Uptime" => "Uptime",
        "Battery" => "Battery",
        "Hostname" => "Hostname",
        "Clock" => "Clock",
        "Date" => "Date",
        "DateTime" => "Date and Time",
        "Blank" => "Blank",
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
        let meters_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("Style", " "),
            ("Move", "Enter"),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Delete", "Del"),
            ("Done", "F10"),
        ]);

        // Meters moving mode function bar
        let meters_moving_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("Style", " "),
            ("Lock", "Enter"),
            ("Up", "Up"),
            ("Down", "Dn"),
            ("Left", "<-"),
            ("Right", "->"),
            ("Done", "F10"),
        ]);

        // Available meters function bar (matching C htop AvailableMetersPanel)
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

        if x >= max_x {
            return;
        }

        let border_attr = crt.color(ColorElement::ScreensCurBorder);
        let text_attr = crt.color(ColorElement::ScreensCurText);

        // Draw '['
        attron(border_attr);
        mvaddch(y, x, '[' as u32);
        attroff(border_attr);
        x += 1;

        if x >= max_x {
            attrset(crt.color(ColorElement::ResetColor));
            return;
        }

        // Draw "Setup" text
        let name = "Setup";
        let name_width = name.len().min((max_x - x) as usize);
        attron(text_attr);
        let _ = mvaddnstr(y, x, name, name_width as i32);
        attroff(text_attr);
        x += name_width as i32;

        if x >= max_x {
            attrset(crt.color(ColorElement::ResetColor));
            return;
        }

        // Draw ']'
        attron(border_attr);
        mvaddch(y, x, ']' as u32);
        attroff(border_attr);

        attrset(crt.color(ColorElement::ResetColor));
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

        // Draw header
        let header_attr = if self.focus == 0 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attron(header_attr);
        let header = "Categories";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

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

            attron(attr);
            let name = cat.name();
            let _ = addstr(name);
            for _ in name.len()..w as usize {
                addch(' ' as u32);
            }
            attroff(attr);
        }

        // Fill remaining lines
        for i in (SetupCategory::all().len() as i32 + 1)..h {
            mv(y + i, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }
    }

    fn draw_display_options(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attron(header_attr);
        let header = "Display options";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

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
                    attron(selection_attr);
                    let text = display_str.text();
                    let display_text: String = text.chars().take(w as usize).collect();
                    let _ = addstr(&display_text);
                    for _ in display_text.chars().count()..w as usize {
                        addch(' ' as u32);
                    }
                    attroff(selection_attr);
                } else {
                    // Draw with the RichString's own attributes
                    display_str.draw_at(screen_y, x, w);
                }
            } else {
                // Empty line
                for _ in 0..w {
                    addch(' ' as u32);
                }
            }
        }
    }

    fn draw_colors_panel(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attron(header_attr);
        let header = "Colors";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

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
                attron(selection_attr);
            }

            // Draw checkbox
            if !is_selected {
                attron(box_color);
            }
            let _ = addstr("[");
            if !is_selected {
                attroff(box_color);
                attron(mark_color);
            }
            let _ = addstr(if is_checked { "x" } else { " " });
            if !is_selected {
                attroff(mark_color);
                attron(box_color);
            }
            let _ = addstr("]    ");
            if !is_selected {
                attroff(box_color);
                attron(text_color);
            }
            let _ = addstr(name);
            if !is_selected {
                attroff(text_color);
            }

            // Pad to width
            let used = 7 + name.len(); // "[x]    " + name
            for _ in used..w as usize {
                addch(' ' as u32);
            }

            if is_selected {
                attroff(selection_attr);
            }
        }

        // Fill remaining lines
        for i in (COLOR_SCHEME_NAMES.len() as i32 + 1)..h {
            mv(y + i, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }
    }

    fn draw_header_layout(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attron(header_attr);
        let header = "Header Layout";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

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
                attron(selected_attr);
                let _ = addstr(checkbox);
                let desc = layout.description();
                let _ = addstr(desc);
                // Pad to width
                let used = checkbox.len() + desc.len();
                for _ in used..w as usize {
                    addch(' ' as u32);
                }
                attroff(selected_attr);
            } else {
                // Non-selected row
                attron(check_attr);
                let _ = addstr(checkbox);
                attroff(check_attr);

                attron(text_attr);
                let desc = layout.description();
                let _ = addstr(desc);
                attroff(text_attr);

                // Pad to width
                let used = checkbox.len() + desc.len();
                for _ in used..w as usize {
                    addch(' ' as u32);
                }
            }
        }

        // Fill remaining lines
        for i in (layouts.len() as i32 + 1)..h {
            mv(y + i, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }
    }

    fn draw_meters_panel(&self, crt: &Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let total_width = self.content_panel.w;
        let h = self.content_panel.h;

        // Get number of columns from current layout
        let num_columns = settings.header_layout.num_columns();

        // Calculate panel widths:
        // - Each column panel gets equal space
        // - Available meters panel gets the same width as a column
        // Total panels = num_columns + 1 (available meters)
        let num_panels = num_columns + 1;
        let panel_width = total_width / num_panels as i32;
        let remainder = total_width - (panel_width * num_panels as i32);

        // Determine which panel has focus
        // focus == 1 means we're in meters mode (not categories)
        // meters_column_focus: 0..num_columns-1 = column panels, num_columns = available meters
        let focused_column = if self.focus == 1 {
            Some(self.meters_column_focus)
        } else {
            None
        };

        // Draw column panels
        let mut cur_x = x;
        for col_idx in 0..num_columns {
            let w = if col_idx == num_columns - 1 {
                panel_width + remainder / 2 // Give extra space to last column
            } else {
                panel_width
            };

            self.draw_meter_column_panel(
                crt,
                settings,
                col_idx,
                cur_x,
                y,
                w,
                h,
                focused_column == Some(col_idx),
            );
            cur_x += w;
        }

        // Draw available meters panel (rightmost)
        let available_width = panel_width + (remainder - remainder / 2);
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
        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attron(header_attr);
        let header = format!("Column {}", col_idx + 1);
        let header_display: String = header.chars().take(w as usize).collect();
        let _ = addstr(&header_display);
        for _ in header_display.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

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
                    let display_text: String = display.chars().take((w - 1) as usize).collect();

                    if is_selected {
                        attron(selection_attr);
                        let _ = addstr(&display_text);
                        for _ in display_text.chars().count()..w as usize {
                            addch(' ' as u32);
                        }
                        attroff(selection_attr);
                    } else {
                        attron(normal_attr);
                        let _ = addstr(&display_text);
                        attroff(normal_attr);
                        for _ in display_text.chars().count()..w as usize {
                            addch(' ' as u32);
                        }
                    }
                } else {
                    // Empty row
                    for _ in 0..w {
                        addch(' ' as u32);
                    }
                }
            } else {
                // No meters array
                for _ in 0..w {
                    addch(' ' as u32);
                }
            }
        }
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
        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attron(header_attr);
        let header = "Available meters";
        let header_display: String = header.chars().take(w as usize).collect();
        let _ = addstr(&header_display);
        for _ in header_display.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

        // Selection color
        let selection_attr = if has_focus {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        // Draw available meters
        let display_height = (h - 1) as usize;
        for i in 0..display_height {
            let item_index = self.meters_available_scroll as usize + i;
            let screen_y = y + 1 + i as i32;
            mv(screen_y, x);

            if item_index < AVAILABLE_METERS.len() {
                let meter_info = &AVAILABLE_METERS[item_index];
                let is_selected = has_focus && item_index == self.meters_available_selection;

                // Show display name (description could be shown as tooltip)
                let display_text: String = meter_info
                    .display_name
                    .chars()
                    .take((w - 1) as usize)
                    .collect();

                if is_selected {
                    attron(selection_attr);
                    let _ = addstr(&display_text);
                    for _ in display_text.chars().count()..w as usize {
                        addch(' ' as u32);
                    }
                    attroff(selection_attr);
                } else {
                    attron(normal_attr);
                    let _ = addstr(&display_text);
                    attroff(normal_attr);
                    for _ in display_text.chars().count()..w as usize {
                        addch(' ' as u32);
                    }
                }
            } else {
                // Empty row
                for _ in 0..w {
                    addch(' ' as u32);
                }
            }
        }
    }

    fn draw_screens_panel(&self, crt: &Crt, _settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        mv(y, x);
        attron(header_attr);
        let header = "Screens";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

        // Placeholder
        let text_attr = crt.color(ColorElement::Process);
        mv(y + 1, x);
        attron(text_attr);
        let _ = addstr("(Screens configuration - not yet implemented)");
        attroff(text_attr);

        // Fill remaining lines
        for i in 2..h {
            mv(y + i, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }
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

            // Enter - toggle moving mode or add meter from available
            KEY_ENTER_LF | KEY_ENTER_CR => {
                if is_available_panel {
                    // Add selected meter to current column
                    self.add_meter_from_available(settings, header);
                    return HandlerResult::Handled;
                } else {
                    // Toggle moving mode
                    self.meters_moving = !self.meters_moving;
                    return HandlerResult::Handled;
                }
            }

            // Space - cycle meter style (Bar/Text/Graph/Led)
            KEY_SPACE => {
                if !is_available_panel {
                    self.cycle_meter_style(settings, header);
                    return HandlerResult::Handled;
                }
            }

            // Delete - remove meter from column
            KEY_DC => {
                if !is_available_panel {
                    self.delete_selected_meter(settings, header);
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
                    // Move focus to left panel
                    if self.meters_column_focus > 0 {
                        self.meters_column_focus -= 1;
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
                    // Move focus to right panel
                    if self.meters_column_focus < num_columns {
                        self.meters_column_focus += 1;
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
            if self.meters_available_selection < AVAILABLE_METERS.len().saturating_sub(1) {
                self.meters_available_selection += 1;
                // Adjust scroll
                let display_height = (self.content_panel.h - 1) as i32;
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

    /// Add a meter from available meters to the first column
    fn add_meter_from_available(&mut self, settings: &mut Settings, header: &mut Header) {
        if self.meters_available_selection >= AVAILABLE_METERS.len() {
            return;
        }

        let meter_info = &AVAILABLE_METERS[self.meters_available_selection];

        // Add to the first column (or could be current column - 1 if we want)
        // C htop adds to the last focused column panel
        let target_column = if self.meters_column_focus > 0 {
            self.meters_column_focus - 1
        } else {
            0
        };

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

        // Update selection in that column
        if let Some(sel) = self.meters_column_selection.get_mut(target_column) {
            *sel = settings.header_columns[target_column]
                .len()
                .saturating_sub(1);
        }

        // Repopulate header
        header.populate_from_settings(settings);
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

    /// Cycle the meter style (Bar -> Text -> Graph -> Led -> Bar)
    fn cycle_meter_style(&mut self, settings: &mut Settings, header: &mut Header) {
        let col_idx = self.meters_column_focus;
        let selection = self
            .meters_column_selection
            .get(col_idx)
            .copied()
            .unwrap_or(0);

        if let Some(column) = settings.header_columns.get_mut(col_idx) {
            if let Some(meter) = column.get_mut(selection) {
                meter.mode = match meter.mode {
                    MeterMode::Bar => MeterMode::Text,
                    MeterMode::Text => MeterMode::Graph,
                    MeterMode::Graph => MeterMode::Led,
                    MeterMode::Led => MeterMode::Bar,
                };
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
        let display_height = (self.content_panel.h - 1) as i32;
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
                                let meters: Vec<_> = removed_meters.drain(..).collect();
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
        machine: &Machine,
    ) {
        // Get header height for layout
        let mut header_height = header.get_height();

        // Initial layout
        self.layout(crt, header_height, settings.screen_tabs);

        // Start with first interactive item selected
        self.skip_to_interactive(settings, true);

        loop {
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
                if new_height != header_height {
                    header_height = new_height;
                    self.layout(crt, header_height, settings.screen_tabs);
                }
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
