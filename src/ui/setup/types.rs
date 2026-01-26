//! Setup screen types and enums
//!
//! Contains the core data types used by the setup screen:
//! - SetupCategory: The categories in the left panel
//! - OptionItem: Items in the Display Options list
//! - SettingField: Identifiers for settings fields

use crate::core::{ColorScheme, Settings};
use crate::ui::crt::{ColorElement, Crt};
use crate::ui::rich_string::RichString;

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
            // Per-screen settings (from current screen)
            SettingField::TreeView => settings.screens[settings.active_screen].tree_view,
            SettingField::TreeViewAlwaysByPid => {
                settings.screens[settings.active_screen].tree_view_always_by_pid
            }
            SettingField::AllBranchesCollapsed => {
                settings.screens[settings.active_screen].all_branches_collapsed
            }
            // Global settings
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
            // Per-screen settings (update current screen and sync global)
            SettingField::TreeView => {
                settings.screens[settings.active_screen].tree_view = value;
                settings.tree_view = value;
            }
            SettingField::TreeViewAlwaysByPid => {
                settings.screens[settings.active_screen].tree_view_always_by_pid = value;
                settings.tree_view_always_by_pid = value;
            }
            SettingField::AllBranchesCollapsed => {
                settings.screens[settings.active_screen].all_branches_collapsed = value;
                settings.all_branches_collapsed = value;
            }
            // Global settings
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
