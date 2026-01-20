//! Setup Screen - F2 configuration screen
//!
//! This module implements the htop Setup screen with:
//! - Categories panel on the left (Display options, Header layout, Meters, Screens, Colors)
//! - Content panel(s) on the right depending on selected category

#![allow(dead_code)]

use ncurses::*;

use crate::core::{ColorScheme, Settings};
use super::crt::{ColorElement, KEY_F7, KEY_F8, KEY_F10};
use super::function_bar::FunctionBar;
use super::panel::{Panel, HandlerResult};
use super::rich_string::RichString;
use super::Crt;

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
            OptionItem::Number { text, field, scale, .. } => {
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
    /// Which panel has focus (0 = categories, 1 = content)
    focus: usize,
    /// Function bar for the setup screen
    function_bar: FunctionBar,
    /// Dec/Inc function bar for number items
    dec_inc_bar: FunctionBar,
    /// Whether settings were changed
    pub changed: bool,
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
            changed: false,
        }
    }

    fn build_display_options() -> Vec<OptionItem> {
        vec![
            OptionItem::text("For current screen tab:"),
            OptionItem::check("Tree view", SettingField::TreeView),
            OptionItem::check("- Tree view is always sorted by PID (htop 2 behavior)", SettingField::TreeViewAlwaysByPid),
            OptionItem::check("- Tree view is collapsed by default", SettingField::AllBranchesCollapsed),
            OptionItem::text("Global options:"),
            OptionItem::check("Show tabs for screens", SettingField::ScreenTabs),
            OptionItem::check("Shadow other users' processes", SettingField::ShadowOtherUsers),
            OptionItem::check("Hide kernel threads", SettingField::HideKernelThreads),
            OptionItem::check("Hide userland process threads", SettingField::HideUserlandThreads),
            OptionItem::check("Hide processes running in containers", SettingField::HideRunningInContainer),
            OptionItem::check("Display threads in a different color", SettingField::HighlightThreads),
            OptionItem::check("Show custom thread names", SettingField::ShowThreadNames),
            OptionItem::check("Show program path", SettingField::ShowProgramPath),
            OptionItem::check("Highlight program \"basename\"", SettingField::HighlightBaseName),
            OptionItem::check("Highlight out-dated/removed programs (red) / libraries (yellow)", SettingField::HighlightDeletedExe),
            OptionItem::check("Shadow distribution path prefixes", SettingField::ShadowDistPathPrefix),
            OptionItem::check("Merge exe, comm and cmdline in Command", SettingField::ShowMergedCommand),
            OptionItem::check("- Try to find comm in cmdline (when Command is merged)", SettingField::FindCommInCmdline),
            OptionItem::check("- Try to strip exe from cmdline (when Command is merged)", SettingField::StripExeFromCmdline),
            OptionItem::check("Highlight large numbers in memory counters", SettingField::HighlightMegabytes),
            OptionItem::check("Leave a margin around header", SettingField::HeaderMargin),
            OptionItem::check("Detailed CPU time (System/IO-Wait/Hard-IRQ/Soft-IRQ/Steal/Guest)", SettingField::DetailedCpuTime),
            OptionItem::check("Count CPUs from 1 instead of 0", SettingField::CountCpusFromOne),
            OptionItem::check("Update process names on every refresh", SettingField::UpdateProcessNames),
            OptionItem::check("Add guest time in CPU meter percentage", SettingField::AccountGuestInCpuMeter),
            OptionItem::check("Also show CPU percentage numerically", SettingField::ShowCpuUsage),
            OptionItem::check("Also show CPU frequency", SettingField::ShowCpuFrequency),
            OptionItem::check("Show cached memory in graph and bar modes", SettingField::ShowCachedMemory),
            OptionItem::check("Enable the mouse", SettingField::EnableMouse),
            OptionItem::number_scaled("Update interval (in seconds)", SettingField::Delay, 1, 255, -1),
            OptionItem::check("Highlight new and old processes", SettingField::HighlightChanges),
            OptionItem::number("- Highlight time (in seconds)", SettingField::HighlightDelaySecs, 1, 86400),
            OptionItem::number("Hide main function bar (0 - off, 1 - on ESC until next input, 2 - permanently)", SettingField::HideFunctionBar, 0, 2),
        ]
    }

    /// Calculate layout based on terminal size
    pub fn layout(&mut self, crt: &Crt) {
        let width = crt.width();
        let height = crt.height();

        // Categories panel: fixed width of 16, full height minus function bar
        let panel_height = height - 1; // Leave room for function bar
        self.categories_panel.resize(16, panel_height);
        self.categories_panel.move_to(0, 0);

        // Content panel: remaining width
        let content_width = width - 16;
        self.content_panel.resize(content_width, panel_height);
        self.content_panel.move_to(16, 0);
    }

    /// Draw the setup screen
    pub fn draw(&mut self, crt: &Crt, settings: &Settings) {
        // Clear screen
        erase();

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

    fn draw_header_layout(&self, crt: &Crt, _settings: &Settings) {
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
        let header = "Header layout";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

        // Placeholder for header layout options
        let text_attr = crt.color(ColorElement::Process);
        mv(y + 1, x);
        attron(text_attr);
        let _ = addstr("(Header layout configuration - not yet implemented)");
        attroff(text_attr);

        // Fill remaining lines
        for i in 2..h {
            mv(y + i, x);
            for _ in 0..w {
                addch(' ' as u32);
            }
        }
    }

    fn draw_meters_panel(&self, crt: &Crt, _settings: &Settings) {
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
        let header = "Meters";
        let _ = addstr(header);
        for _ in header.len()..w as usize {
            addch(' ' as u32);
        }
        attroff(header_attr);

        // Placeholder
        let text_attr = crt.color(ColorElement::Process);
        mv(y + 1, x);
        attron(text_attr);
        let _ = addstr("(Meters configuration - not yet implemented)");
        attroff(text_attr);

        // Fill remaining lines
        for i in 2..h {
            mv(y + i, x);
            for _ in 0..w {
                addch(' ' as u32);
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
        
        // Choose which function bar to show based on current selection
        let bar = if self.focus == 1 && self.category == SetupCategory::DisplayOptions {
            if let Some(item) = self.display_options.get(self.content_index) {
                if matches!(item, OptionItem::Number { .. }) {
                    &self.dec_inc_bar
                } else {
                    &self.function_bar
                }
            } else {
                &self.function_bar
            }
        } else {
            &self.function_bar
        };

        bar.draw(crt, y, settings);
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: i32, settings: &mut Settings, crt: &mut Crt) -> HandlerResult {
        match key {
            // Exit keys
            KEY_F10 | KEY_ESC | KEY_Q => {
                // ESC or F10 or q - exit setup
                return HandlerResult::BreakLoop;
            }

            // Tab / Left / Right - switch focus between panels
            KEY_TAB | KEY_LEFT | KEY_RIGHT => {
                if self.category == SetupCategory::DisplayOptions || self.category == SetupCategory::Colors {
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
                        self.content_index = 0;
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
                        self.content_index = 0;
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
                    self.handle_toggle(settings, crt);
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

    fn get_content_count(&self) -> usize {
        match self.category {
            SetupCategory::DisplayOptions => self.display_options.len(),
            SetupCategory::Colors => COLOR_SCHEME_NAMES.len(),
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

    fn handle_toggle(&mut self, settings: &mut Settings, crt: &mut Crt) {
        match self.category {
            SetupCategory::DisplayOptions => {
                if let Some(item) = self.display_options.get(self.content_index) {
                    match item {
                        OptionItem::Check { field, .. } => {
                            field.toggle(settings);
                            settings.changed = true;
                            self.changed = true;
                        }
                        OptionItem::Number { field, min, max, .. } => {
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
    pub fn run(&mut self, settings: &mut Settings, crt: &mut Crt) {
        // Initial layout
        self.layout(crt);
        
        // Start with first interactive item selected
        self.skip_to_interactive(settings, true);

        loop {
            // Draw
            self.draw(crt, settings);

            // Get input
            let key = getch();

            // Handle resize
            if key == KEY_RESIZE {
                crt.handle_resize();
                self.layout(crt);
                continue;
            }

            // Handle key
            let result = self.handle_key(key, settings, crt);
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
