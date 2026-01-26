//! Setup Screen - F2 configuration screen
//!
//! This module implements the htop Setup screen with:
//! - Categories panel on the left (Display options, Header layout, Meters, Screens, Colors)
//! - Content panel(s) on the right depending on selected category

#![allow(dead_code)]
#![allow(clippy::too_many_arguments)] // UI drawing functions naturally have many parameters

mod meter_registry;
mod types;

pub use meter_registry::*;
pub use types::*;

use super::crt::{
    ColorElement, KEY_BACKSPACE, KEY_DC, KEY_DEL_MAC, KEY_DOWN, KEY_END, KEY_ENTER, KEY_F10,
    KEY_F2, KEY_F4, KEY_F5, KEY_F6, KEY_F7, KEY_F8, KEY_F9, KEY_HOME, KEY_LEFT, KEY_MOUSE,
    KEY_NPAGE, KEY_PPAGE, KEY_RESIZE, KEY_RIGHT, KEY_UP,
};
use super::function_bar::FunctionBar;
use super::header::Header;
use super::panel::{HandlerResult, Panel};
use super::rich_string::RichString;
use super::Crt;
#[cfg(target_os = "linux")]
use crate::core::ScanFlags;
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
    /// Function bar for Screens panel when only 1 screen (no Remove option)
    screens_bar_no_remove: FunctionBar,
    /// Function bar for Screens panel when in moving mode
    screens_moving_bar: FunctionBar,
    /// Function bar for Active Columns panel
    columns_bar: FunctionBar,
    /// Function bar for Active Columns panel when only 1 column (no Remove option)
    columns_bar_no_remove: FunctionBar,
    /// Function bar for Active Columns panel when in moving mode
    columns_moving_bar: FunctionBar,
    /// Function bar for Available Columns panel
    available_columns_bar: FunctionBar,
    /// Function bar for Screens panel when renaming
    screens_renaming_bar: FunctionBar,
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
    pub fn new(settings: &Settings) -> Self {
        let mut categories_panel = Panel::new(0, 0, 16, 10);
        categories_panel.set_header("Categories");

        // Add category items
        for cat in SetupCategory::all() {
            categories_panel.add_text(cat.name());
        }

        let mut content_panel = Panel::new(16, 0, 60, 10);
        content_panel.set_header("Display options");

        // Build display options list with current screen name (like C htop does at construction time)
        let current_screen_name = &settings.screens[settings.active_screen].heading;
        let display_options = Self::build_display_options(current_screen_name);

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
        // Added Enter:Move to indicate moving mode toggle
        let screens_bar = FunctionBar::new_with_labels(&[
            ("Move", "Enter"),
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

        // Screens panel function bar when only 1 screen (no Remove option)
        let screens_bar_no_remove = FunctionBar::new_with_labels(&[
            ("", ""),
            ("Rename", "F2"),
            ("", ""),
            ("", ""),
            ("New", "F5"),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Done", "F10"),
        ]);

        // Screens panel function bar when in moving mode
        let screens_moving_bar = FunctionBar::new_with_labels(&[
            ("Lock", "Enter"),
            ("", ""),
            ("Up", "↑"),
            ("Down", "↓"),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Delete", "Del"),
            ("", ""),
        ]);

        // Active Columns panel function bar (matching C htop ColumnsFunctions)
        // C htop: {"      ", "      ", "      ", "      ", "      ", "      ", "MoveUp", "MoveDn", "Remove", "Done  "}
        // Added Enter:Move to indicate moving mode toggle
        let columns_bar = FunctionBar::new_with_labels(&[
            ("Move", "Enter"),
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

        // Active Columns panel function bar when only 1 column (no Move/Remove options)
        let columns_bar_no_remove = FunctionBar::new_with_labels(&[
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

        // Active Columns panel function bar when in moving mode
        let columns_moving_bar = FunctionBar::new_with_labels(&[
            ("Lock", "Enter"),
            ("", ""),
            ("Up", "↑"),
            ("Down", "↓"),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Delete", "Del"),
            ("", ""),
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

        // Screens renaming function bar - only show Enter: Done
        let screens_renaming_bar = FunctionBar::new_with_labels(&[
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("Done", "Enter"),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
            ("", ""),
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
            screens_bar_no_remove,
            screens_moving_bar,
            columns_bar,
            columns_bar_no_remove,
            columns_moving_bar,
            available_columns_bar,
            screens_renaming_bar,
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

    fn build_display_options(current_screen_name: &str) -> Vec<OptionItem> {
        // Build the first item with the current screen name, like C htop does at construction time
        let tab_header = format!("For current screen tab: {}", current_screen_name);
        vec![
            OptionItem::text(&tab_header),
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
    fn draw_setup_tab(&self, crt: &mut Crt, header_height: i32) {
        const SCREEN_TAB_MARGIN_LEFT: i32 = 2;

        let y = header_height; // Tab row is right after header
        let max_x = crt.width();
        let reset_color = crt.color(ColorElement::ResetColor);
        let border_attr = crt.color(ColorElement::ScreensCurBorder);
        let text_attr = crt.color(ColorElement::ScreensCurText);

        let mut x = SCREEN_TAB_MARGIN_LEFT;

        // Fill the entire tab row with the reset color background first
        crt.attrset(reset_color);
        crt.mv(y, 0);
        for _ in 0..max_x {
            crt.addch_raw(' ' as u32);
        }

        if x >= max_x {
            return;
        }

        // Draw '[' - C htop doesn't reset between bracket and text
        crt.attrset(border_attr);
        crt.mv(y, x);
        crt.addch_raw('[' as u32);
        x += 1;

        if x >= max_x {
            crt.attrset(reset_color);
            return;
        }

        // Draw "Setup" text
        let name = "Setup";
        let name_width = name.len().min((max_x - x) as usize);
        crt.attrset(text_attr);
        crt.mv(y, x);
        crt.addnstr_raw(name, name_width as i32);
        x += name_width as i32;

        if x >= max_x {
            crt.attrset(reset_color);
            return;
        }

        // Draw ']'
        crt.attrset(border_attr);
        crt.mv(y, x);
        crt.addch_raw(']' as u32);

        // Only reset at the very end (matches C htop)
        crt.attrset(reset_color);
    }

    /// Draw the setup screen
    pub fn draw(
        &mut self,
        crt: &mut Crt,
        settings: &Settings,
        header: &mut Header,
        machine: &Machine,
    ) {
        // Note: We don't call crt.clear() here - ncurses handles differential updates.
        // Only call clear() when needed (e.g., after dialogs, resize, etc.)

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

        crt.refresh();
    }

    fn draw_categories_panel(&self, crt: &mut Crt) {
        let x = self.categories_panel.x;
        let y = self.categories_panel.y;
        let w = self.categories_panel.w;
        let h = self.categories_panel.h;

        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);

        // Draw header
        let header_attr = if self.focus == 0 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        // Draw items
        let selection_attr = if self.focus == 0 {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);
        let category_index = self.category_index;

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        let header = "Categories";
        crt.addstr_raw(header);
        for _ in header.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }

        for (i, cat) in SetupCategory::all().iter().enumerate() {
            if i as i32 >= h - 1 {
                break;
            }

            crt.mv(y + 1 + i as i32, x);
            let attr = if i == category_index {
                selection_attr
            } else {
                normal_attr
            };

            crt.attrset(attr);
            let name = cat.name();
            crt.addstr_raw(name);
            for _ in name.len()..w as usize {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.attrset(reset_color);
    }

    fn draw_display_options(&self, crt: &mut Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        // Draw items
        let selection_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };

        let display_height = (h - 1) as usize;
        let content_scroll = self.content_scroll;
        let content_index = self.content_index;
        let focus = self.focus;

        // Prepare display strings for items (screen name is already baked into display_options[0])
        let mut items_to_draw: Vec<(usize, RichString, bool)> = Vec::new();
        for i in 0..display_height {
            let item_index = content_scroll as usize + i;
            if item_index < self.display_options.len() {
                let item = &self.display_options[item_index];
                let is_selected = item_index == content_index && focus == 1;
                let display_str = item.display(settings, crt);
                items_to_draw.push((i, display_str, is_selected));
            }
        }

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        let header = "Display options";
        crt.addstr_raw(header);
        for _ in header.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }
        crt.attrset(reset_color);

        for (i, display_str, is_selected) in &items_to_draw {
            let screen_y = y + 1 + *i as i32;
            crt.mv(screen_y, x);

            if *is_selected {
                crt.attrset(selection_attr);
                let text = display_str.text();
                let display_text: String = text.chars().take(w as usize).collect();
                crt.addstr_raw(&display_text);
                for _ in display_text.chars().count()..w as usize {
                    crt.addch_raw(' ' as u32);
                }
                crt.attrset(reset_color);
            }
        }

        crt.attrset(reset_color);

        // Draw non-selected items with RichString (has its own draw method that uses with_window)
        for (i, display_str, is_selected) in items_to_draw {
            if !is_selected {
                let screen_y = y + 1 + i as i32;
                display_str.draw_at_with_bg(crt, screen_y, x, w, reset_color);
            }
        }
    }

    fn draw_colors_panel(&self, crt: &mut Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

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
        let content_index = self.content_index;
        let focus = self.focus;

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        let header = "Colors";
        crt.addstr_raw(header);
        for _ in header.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }

        for (i, name) in COLOR_SCHEME_NAMES.iter().enumerate() {
            if i as i32 >= h - 1 {
                break;
            }

            crt.mv(y + 1 + i as i32, x);
            let is_selected = i == content_index && focus == 1;
            let is_checked = i == current_scheme;

            if is_selected {
                crt.attrset(selection_attr);
            }

            // Draw checkbox
            if !is_selected {
                crt.attrset(box_color);
            }
            crt.addstr_raw("[");
            if !is_selected {
                crt.attrset(mark_color);
            }
            crt.addstr_raw(if is_checked { "x" } else { " " });
            if !is_selected {
                crt.attrset(box_color);
            }
            crt.addstr_raw("]    ");
            if !is_selected {
                crt.attrset(text_color);
            }
            crt.addstr_raw(name);

            // Pad to width - use reset_color for proper background
            let used = 7 + name.len(); // "[x]    " + name
            if !is_selected {
                crt.attrset(reset_color);
            }
            for _ in used..w as usize {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.attrset(reset_color);
    }

    fn draw_header_layout(&self, crt: &mut Crt, settings: &Settings) {
        let x = self.content_panel.x;
        let y = self.content_panel.y;
        let w = self.content_panel.w;
        let h = self.content_panel.h;

        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);

        // Draw header
        let header_attr = if self.focus == 1 {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        // Draw header layout options with checkmarks (like C htop HeaderOptionsPanel)
        let layouts = HeaderLayout::all();
        let current_layout = settings.header_layout;
        let display_height = (h - 1) as usize; // Minus header row

        let text_attr = crt.color(ColorElement::Process);
        let selected_attr = crt.color(ColorElement::PanelSelectionFocus);
        let check_attr = crt.color(ColorElement::CheckBox);

        let focus = self.focus;
        let content_index = self.content_index;

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        let header = "Header Layout";
        crt.addstr_raw(header);
        for _ in header.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }
        crt.attrset(reset_color);

        for (i, &layout) in layouts.iter().enumerate() {
            if i >= display_height {
                break;
            }

            let screen_y = y + 1 + i as i32;
            let is_selected = focus == 1 && i == content_index;
            let is_checked = layout == current_layout;

            crt.mv(screen_y, x);

            // Draw checkbox
            let checkbox = if is_checked { "[x] " } else { "[ ] " };

            if is_selected {
                // Selected row - highlight entire line
                crt.attrset(selected_attr);
                crt.addstr_raw(checkbox);
                let desc = layout.description();
                crt.addstr_raw(desc);
                // Pad to width
                let used = checkbox.len() + desc.len();
                for _ in used..w as usize {
                    crt.addch_raw(' ' as u32);
                }
                crt.attrset(reset_color);
            } else {
                // Non-selected row
                crt.attrset(check_attr);
                crt.addstr_raw(checkbox);
                crt.attrset(reset_color);

                crt.attrset(text_attr);
                let desc = layout.description();
                crt.addstr_raw(desc);
                crt.attrset(reset_color);

                // Pad to width - already filled with reset_color background
            }
        }

        crt.attrset(reset_color);
    }

    fn draw_meters_panel(&self, crt: &mut Crt, settings: &Settings) {
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
        crt: &mut Crt,
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

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

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
        let meters_moving = self.meters_moving;

        // Build header text
        let header = format!("Column {}", col_idx + 1);
        let header_display: String = header.chars().take(w as usize).collect();

        // Build meter display texts
        let display_height = (h - 1) as usize;
        let mut meter_texts: Vec<(usize, String, bool)> = Vec::new();
        if let Some(meters) = meters {
            for (i, meter) in meters.iter().enumerate().take(display_height) {
                let is_selected = has_focus && i == selection;

                // Get display name with mode
                let display = meter_display_name(&meter.name, meter.mode);

                // Add moving indicator (↕) if this item is being moved
                let display_text = if is_selected && meters_moving {
                    // Use UTF-8 up-down arrow like C htop
                    let prefix = "↕ ";
                    let max_len = (w - 1) as usize;
                    let prefixed = format!("{}{}", prefix, display);
                    prefixed.chars().take(max_len).collect::<String>()
                } else {
                    display.chars().take((w - 1) as usize).collect::<String>()
                };
                meter_texts.push((i, display_text, is_selected));
            }
        }

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        crt.addstr_raw(&header_display);
        for _ in header_display.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }
        crt.attrset(reset_color);

        for (i, display_text, is_selected) in &meter_texts {
            let screen_y = y + 1 + *i as i32;
            crt.mv(screen_y, x);

            if *is_selected {
                crt.attrset(selection_attr);
                crt.addstr_raw(display_text);
                for _ in display_text.chars().count()..w as usize {
                    crt.addch_raw(' ' as u32);
                }
                crt.attrset(reset_color);
            } else {
                crt.attrset(normal_attr);
                crt.addstr_raw(display_text);
                crt.attrset(reset_color);
                // Rest of line already filled with reset_color background
            }
        }

        crt.attrset(reset_color);
    }

    /// Draw the available meters panel
    fn draw_available_meters_panel(
        &self,
        crt: &mut Crt,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        // Selection color
        let selection_attr = if has_focus {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        // Get platform-filtered meters
        let available_meters = available_meters_for_platform();

        // Build display texts
        let display_height = (h - 1) as usize;
        let meters_available_scroll = self.meters_available_scroll;
        let meters_available_selection = self.meters_available_selection;
        let mut meter_texts: Vec<(usize, String, bool)> = Vec::new();
        for i in 0..display_height {
            let item_index = meters_available_scroll as usize + i;
            if item_index < available_meters.len() {
                let meter_info = available_meters[item_index];
                let is_selected = has_focus && item_index == meters_available_selection;

                // Show description in available meters panel (like C htop)
                let display_text: String = meter_info
                    .description
                    .chars()
                    .take((w - 1) as usize)
                    .collect();
                meter_texts.push((i, display_text, is_selected));
            }
        }

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        let header = "Available meters";
        let header_display: String = header.chars().take(w as usize).collect();
        crt.addstr_raw(&header_display);
        for _ in header_display.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }
        crt.attrset(reset_color);

        for (i, display_text, is_selected) in &meter_texts {
            let screen_y = y + 1 + *i as i32;
            crt.mv(screen_y, x);

            if *is_selected {
                crt.attrset(selection_attr);
                crt.addstr_raw(display_text);
                for _ in display_text.chars().count()..w as usize {
                    crt.addch_raw(' ' as u32);
                }
                crt.attrset(reset_color);
            } else {
                crt.attrset(normal_attr);
                crt.addstr_raw(display_text);
                crt.attrset(reset_color);
                // Rest of line already filled with reset_color background
            }
        }

        crt.attrset(reset_color);
    }

    fn draw_screens_panel(&self, crt: &mut Crt, settings: &Settings) {
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
        crt: &mut Crt,
        settings: &Settings,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

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

        // Build display texts
        let display_height = (h - 1) as usize;
        let screens_selection = self.screens_selection;
        let screens_moving = self.screens_moving;
        let screens_renaming = self.screens_renaming;
        let screens_rename_buffer = self.screens_rename_buffer.clone();
        let mut screen_texts: Vec<(usize, String, bool)> = Vec::new();

        for i in 0..display_height.min(settings.screens.len()) {
            let screen = &settings.screens[i];
            let is_selected = i == screens_selection;

            // Get display name (with renaming support)
            let display = if is_selected && has_focus && screens_renaming {
                // Show rename buffer with cursor
                format!("{}_", &screens_rename_buffer)
            } else {
                screen.heading.clone()
            };

            // Add moving indicator (↕) if this item is being moved
            let display_text = if is_selected && has_focus && screens_moving && !screens_renaming {
                let prefix = "↕ ";
                let max_len = (w - 1) as usize;
                let prefixed = format!("{}{}", prefix, display);
                prefixed.chars().take(max_len).collect::<String>()
            } else {
                display.chars().take((w - 1) as usize).collect::<String>()
            };

            screen_texts.push((i, display_text, is_selected));
        }

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        let header = "Screens";
        let header_display: String = header.chars().take(w as usize).collect();
        crt.addstr_raw(&header_display);
        for _ in header_display.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }
        crt.attrset(reset_color);

        for (i, display_text, is_selected) in &screen_texts {
            let screen_y = y + 1 + *i as i32;
            crt.mv(screen_y, x);

            if *is_selected {
                crt.attrset(selection_attr);
                crt.addstr_raw(display_text);
                for _ in display_text.chars().count()..w as usize {
                    crt.addch_raw(' ' as u32);
                }
                crt.attrset(reset_color);
            } else {
                crt.attrset(normal_attr);
                crt.addstr_raw(display_text);
                crt.attrset(reset_color);
                // Rest of line already filled with reset_color background
            }
        }

        crt.attrset(reset_color);
    }

    /// Draw the Active Columns panel
    fn draw_active_columns_panel(
        &self,
        crt: &mut Crt,
        settings: &Settings,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

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

        // Build display texts
        let display_height = (h - 1) as usize;
        let columns_selection = self.columns_selection;
        let columns_moving = self.columns_moving;
        let mut column_texts: Vec<(usize, String, bool)> = Vec::new();

        for (i, &field) in fields.iter().enumerate().take(display_height) {
            let is_selected = i == columns_selection;

            let display = field.name();

            // Add moving indicator (↕) if this item is being moved
            let display_text = if is_selected && has_focus && columns_moving {
                let prefix = "↕ ";
                let max_len = (w - 1) as usize;
                let prefixed = format!("{}{}", prefix, display);
                prefixed.chars().take(max_len).collect::<String>()
            } else {
                display.chars().take((w - 1) as usize).collect::<String>()
            };

            column_texts.push((i, display_text, is_selected));
        }

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        let header = "Active Columns";
        let header_display: String = header.chars().take(w as usize).collect();
        crt.addstr_raw(&header_display);
        for _ in header_display.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }
        crt.attrset(reset_color);

        for (i, display_text, is_selected) in &column_texts {
            let screen_y = y + 1 + *i as i32;
            crt.mv(screen_y, x);

            if *is_selected {
                crt.attrset(selection_attr);
                crt.addstr_raw(display_text);
                for _ in display_text.chars().count()..w as usize {
                    crt.addch_raw(' ' as u32);
                }
                crt.attrset(reset_color);
            } else {
                crt.attrset(normal_attr);
                crt.addstr_raw(display_text);
                crt.attrset(reset_color);
                // Rest of line already filled with reset_color background
            }
        }

        crt.attrset(reset_color);
    }

    /// Draw the Available Columns panel
    fn draw_available_columns_panel(
        &self,
        crt: &mut Crt,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        has_focus: bool,
    ) {
        // Fill panel background with reset color (matches C htop's mvhline behavior)
        let reset_color = crt.color(ColorElement::ResetColor);

        // Draw header
        let header_attr = if has_focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        // Selection colors
        let selection_attr = if has_focus {
            crt.color(ColorElement::PanelSelectionFocus)
        } else {
            crt.color(ColorElement::PanelSelectionUnfocus)
        };
        let normal_attr = crt.color(ColorElement::Process);

        // Get available fields
        let available_fields = ProcessField::all();

        // Build display texts
        let display_height = (h - 1) as usize;
        let available_columns_scroll = self.available_columns_scroll;
        let available_columns_selection = self.available_columns_selection;
        let mut column_texts: Vec<(usize, String, bool)> = Vec::new();

        for i in 0..display_height {
            let item_index = available_columns_scroll as usize + i;
            if item_index < available_fields.len() {
                let field = available_fields[item_index];
                let is_selected = has_focus && item_index == available_columns_selection;

                // Show name with description
                let display = format!("{} - {}", field.name(), field.description());
                let display_text: String = display.chars().take((w - 1) as usize).collect();
                column_texts.push((i, display_text, is_selected));
            }
        }

        crt.attrset(reset_color);
        for row in 0..h {
            crt.mv(y + row, x);
            for _ in 0..w {
                crt.addch_raw(' ' as u32);
            }
        }

        crt.mv(y, x);
        crt.attrset(header_attr);
        let header = "Available Columns";
        let header_display: String = header.chars().take(w as usize).collect();
        crt.addstr_raw(&header_display);
        for _ in header_display.len()..w as usize {
            crt.addch_raw(' ' as u32);
        }
        crt.attrset(reset_color);

        for (i, display_text, is_selected) in &column_texts {
            let screen_y = y + 1 + *i as i32;
            crt.mv(screen_y, x);

            if *is_selected {
                crt.attrset(selection_attr);
                crt.addstr_raw(display_text);
                for _ in display_text.chars().count()..w as usize {
                    crt.addch_raw(' ' as u32);
                }
                crt.attrset(reset_color);
            } else {
                crt.attrset(normal_attr);
                crt.addstr_raw(display_text);
                crt.attrset(reset_color);
                // Rest of line already filled with reset_color background
            }
        }

        crt.attrset(reset_color);
    }

    fn draw_function_bar(&self, crt: &mut Crt, settings: &Settings) {
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
                    if self.screens_renaming {
                        &self.screens_renaming_bar
                    } else {
                        match self.screens_panel_focus {
                            0 => {
                                if self.screens_moving {
                                    &self.screens_moving_bar
                                } else if settings.screens.len() <= 1 {
                                    &self.screens_bar_no_remove
                                } else {
                                    &self.screens_bar
                                }
                            }
                            1 => {
                                if self.columns_moving {
                                    &self.columns_moving_bar
                                } else if self.get_current_screen_fields_len(settings) <= 1 {
                                    &self.columns_bar_no_remove
                                } else {
                                    &self.columns_bar
                                }
                            }
                            2 => &self.available_columns_bar, // Available Columns panel
                            _ => &self.function_bar,
                        }
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
                } else if self.category == SetupCategory::Screens {
                    // Switch to screens panel
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
            KEY_DC | KEY_DEL_MAC | KEY_F9 => {
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
            KEY_F10 => {
                // F10 is disabled in moving mode, otherwise exits setup
                if self.columns_moving || self.screens_moving {
                    return HandlerResult::Handled;
                }
                // Exit setup
                return HandlerResult::BreakLoop;
            }
            KEY_ESC => {
                // Exit setup
                return HandlerResult::BreakLoop;
            }
            KEY_Q => {
                // q exits moving mode if active, otherwise exits setup
                if self.columns_moving {
                    self.columns_moving = false;
                    return HandlerResult::Handled;
                } else if self.screens_moving {
                    self.screens_moving = false;
                    return HandlerResult::Handled;
                }
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

    /// Finish renaming a screen (save if not empty)
    fn finish_screens_renaming(&mut self, settings: &mut Settings) {
        if self.screens_renaming {
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
        }
    }

    /// Handle key events while renaming a screen
    fn handle_screens_renaming_key(&mut self, key: i32, settings: &mut Settings) -> HandlerResult {
        match key {
            // Enter - confirm rename
            KEY_ENTER | 10 | 13 => {
                self.finish_screens_renaming(settings);
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
        let single_screen = num_screens <= 1;

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
            // Enter - toggle moving mode (disabled when only 1 screen)
            KEY_ENTER | 10 | 13 => {
                if !single_screen {
                    self.screens_moving = !self.screens_moving;
                }
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
            // F7 / [ / - - Move up (disabled when only 1 screen)
            KEY_F7 | 91 | 45 => {
                // [ = 91, - = 45
                if !single_screen {
                    self.move_screen_up(settings);
                }
                return HandlerResult::Handled;
            }
            // F8 / ] / + - Move down (disabled when only 1 screen)
            KEY_F8 | 93 | 43 => {
                // ] = 93, + = 43
                if !single_screen {
                    self.move_screen_down(settings);
                }
                return HandlerResult::Handled;
            }
            // F9 - Remove screen (disabled when only 1 screen)
            KEY_F9 => {
                if !single_screen {
                    self.remove_screen(settings);
                }
                return HandlerResult::Handled;
            }
            _ => {}
        }
        HandlerResult::Ignored
    }

    /// Handle key events for active columns panel (panel 1)
    fn handle_columns_key(&mut self, key: i32, settings: &mut Settings) -> HandlerResult {
        let num_fields = self.get_current_screen_fields_len(settings);
        let single_column = num_fields <= 1;

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
            // Enter - toggle moving mode (disabled when only 1 column)
            KEY_ENTER | 10 | 13 => {
                if !single_column {
                    self.columns_moving = !self.columns_moving;
                }
                return HandlerResult::Handled;
            }
            // F7 / [ / - - Move up (disabled when only 1 column)
            KEY_F7 | 91 | 45 => {
                if !single_column {
                    self.move_column_up(settings);
                }
                return HandlerResult::Handled;
            }
            // F8 / ] / + - Move down (disabled when only 1 column)
            KEY_F8 | 93 | 43 => {
                if !single_column {
                    self.move_column_down(settings);
                }
                return HandlerResult::Handled;
            }
            // F9 / Delete - Remove column (disabled when only 1 column)
            // KEY_DC is ncurses delete key, KEY_DEL_MAC is Delete on Mac (fn+backspace)
            KEY_F9 | KEY_DC | KEY_DEL_MAC => {
                if !single_column {
                    self.remove_column(settings);
                    // Exit moving mode after deletion
                    self.columns_moving = false;
                }
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
        // Match C htop: new screens have only PID and Command columns, sorted by PID
        let new_screen = ScreenSettings {
            heading: "New".to_string(),
            fields: vec![ProcessField::Pid, ProcessField::Command],
            sort_key: ProcessField::Pid,
            tree_sort_key: ProcessField::Pid,
            direction: 1, // ascending
            tree_direction: 1,
            tree_view: false,
            tree_view_always_by_pid: false,
            all_branches_collapsed: false,
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

        // Add above (before) current selection in columns panel, matching C htop behavior
        let fields = &mut settings.screens[self.screens_selection].fields;
        let insert_pos = if fields.is_empty() {
            0
        } else {
            self.columns_selection.min(fields.len())
        };
        fields.insert(insert_pos, field);
        // Move selection down to keep the same element selected after insertion
        self.columns_selection += 1;
        settings.changed = true;
        self.changed = true;

        // Keep focus on Available Columns panel
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
                // Cycle order: Bar -> Text -> Graph -> Led -> StackedGraph -> Bar
                let modes = [
                    MeterMode::Bar,
                    MeterMode::Text,
                    MeterMode::Graph,
                    MeterMode::Led,
                    MeterMode::StackedGraph,
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
                        OptionItem::Number { field, max, .. } => {
                            // Increase the value (clamp at max)
                            let current = field.get_int(settings);
                            let new_val = (current + 1).min(*max);
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
                crt.clear();
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

    /// Handle mouse event in the setup screen
    /// Returns Some(HandlerResult) if the click was handled, None if not
    fn handle_mouse(
        &mut self,
        crt: &mut Crt,
        settings: &mut Settings,
        header: &mut Header,
    ) -> Option<HandlerResult> {
        let event = crt.get_mouse_event()?;

        // Handle scroll wheel
        if event.is_wheel_up() {
            if self.focus == 0 {
                // Categories panel - scroll up
                if self.category_index > 0 {
                    self.category_index -= 1;
                    self.category = SetupCategory::all()[self.category_index];
                    if self.category == SetupCategory::HeaderLayout {
                        self.content_index = settings.header_layout.to_index();
                    } else {
                        self.content_index = 0;
                    }
                    self.content_scroll = 0;
                }
            } else if self.category == SetupCategory::Meters && self.meters_moving {
                // Meters category in move mode - move meter up
                let num_columns = settings.header_layout.num_columns();
                if self.meters_column_focus < num_columns {
                    self.move_meter_up(settings, header);
                }
            } else if self.category == SetupCategory::Screens && !self.screens_renaming {
                // Screens category - scroll up in focused panel (disabled during renaming)
                self.scroll_screens_up(settings);
            } else if !self.screens_renaming {
                // Content panel - scroll up
                self.move_content_up(settings);
            }
            return Some(HandlerResult::Handled);
        } else if event.is_wheel_down() {
            if self.focus == 0 {
                // Categories panel - scroll down
                if self.category_index < SetupCategory::all().len() - 1 {
                    self.category_index += 1;
                    self.category = SetupCategory::all()[self.category_index];
                    if self.category == SetupCategory::HeaderLayout {
                        self.content_index = settings.header_layout.to_index();
                    } else {
                        self.content_index = 0;
                    }
                    self.content_scroll = 0;
                }
            } else if self.category == SetupCategory::Meters && self.meters_moving {
                // Meters category in move mode - move meter down
                let num_columns = settings.header_layout.num_columns();
                if self.meters_column_focus < num_columns {
                    self.move_meter_down(settings, header);
                }
            } else if self.category == SetupCategory::Screens && !self.screens_renaming {
                // Screens category - scroll down in focused panel (disabled during renaming)
                self.scroll_screens_down(settings);
            } else if !self.screens_renaming {
                // Content panel - scroll down
                self.move_content_down(settings, crt);
            }
            return Some(HandlerResult::Handled);
        }

        // Handle left click
        if event.is_left_click() {
            let x = event.x;
            let y = event.y;

            // Check if click is in categories panel
            let cat_panel = &self.categories_panel;
            if x >= cat_panel.x
                && x < cat_panel.x + cat_panel.w
                && y > cat_panel.y
                && y < cat_panel.y + cat_panel.h
            {
                // Calculate which category was clicked (y - panel_y - 1 for header)
                let item_y = (y - cat_panel.y - 1) as usize;
                if item_y < SetupCategory::all().len() {
                    // Exit renaming mode if clicking on categories panel
                    self.finish_screens_renaming(settings);
                    self.focus = 0;
                    self.category_index = item_y;
                    self.category = SetupCategory::all()[item_y];
                    // Reset content selection for new category
                    if self.category == SetupCategory::HeaderLayout {
                        self.content_index = settings.header_layout.to_index();
                    } else {
                        self.content_index = 0;
                    }
                    self.content_scroll = 0;
                    // Skip to first interactive item
                    self.skip_to_interactive(settings, true);
                    return Some(HandlerResult::Handled);
                }
            }

            // Check if click is in content panel (for DisplayOptions, Colors, HeaderLayout)
            let content_panel = &self.content_panel;
            if x >= content_panel.x
                && x < content_panel.x + content_panel.w
                && y > content_panel.y
                && y < content_panel.y + content_panel.h
            {
                self.focus = 1;

                // Calculate which item was clicked
                let item_y = (y - content_panel.y - 1) as usize;
                let clicked_index = self.content_scroll as usize + item_y;

                match self.category {
                    SetupCategory::DisplayOptions => {
                        if clicked_index < self.display_options.len() {
                            // Check if item is interactive
                            if self.display_options[clicked_index].is_interactive() {
                                self.content_index = clicked_index;
                                // Toggle the option on click
                                self.handle_toggle(settings, crt, header);
                                return Some(HandlerResult::Handled);
                            }
                        }
                    }
                    SetupCategory::Colors => {
                        if clicked_index < COLOR_SCHEME_NAMES.len() {
                            self.content_index = clicked_index;
                            // Apply color scheme on click
                            self.handle_toggle(settings, crt, header);
                            return Some(HandlerResult::Handled);
                        }
                    }
                    SetupCategory::HeaderLayout => {
                        let layouts = HeaderLayout::all();
                        if clicked_index < layouts.len() {
                            self.content_index = clicked_index;
                            // Apply layout on click
                            self.handle_toggle(settings, crt, header);
                            return Some(HandlerResult::Handled);
                        }
                    }
                    SetupCategory::Meters => {
                        // Meters panel has a different structure
                        // Handle clicks on meter columns and available meters
                        return self.handle_meters_mouse(crt, settings, header, x, y, false, false);
                    }
                    SetupCategory::Screens => {
                        // Screens panel has a different structure
                        return self.handle_screens_mouse(settings, x, y, false);
                    }
                }
            }
        }

        // Handle right click for Screens category (toggle move mode)
        if event.is_right_click() && self.category == SetupCategory::Screens {
            let x = event.x;
            let y = event.y;

            // Check if click is in content panel
            let content_panel = &self.content_panel;
            if x >= content_panel.x
                && x < content_panel.x + content_panel.w
                && y > content_panel.y
                && y < content_panel.y + content_panel.h
            {
                return self.handle_screens_mouse(settings, x, y, true);
            }
        }

        // Handle right-click for Meters category (toggle move mode)
        // Only works on meter columns, not on Available Meters panel
        if event.is_right_click() && self.category == SetupCategory::Meters {
            let x = event.x;
            let y = event.y;

            // Check if click is in content panel
            let content_panel = &self.content_panel;
            if x >= content_panel.x
                && x < content_panel.x + content_panel.w
                && y > content_panel.y
                && y < content_panel.y + content_panel.h
            {
                return self.handle_meters_mouse(crt, settings, header, x, y, true, false);
            }
        }

        // Handle middle-click for Screens category (toggle renaming mode)
        if event.is_middle_click() && self.category == SetupCategory::Screens {
            let x = event.x;
            let y = event.y;

            // Check if click is in content panel
            let content_panel = &self.content_panel;
            if x >= content_panel.x
                && x < content_panel.x + content_panel.w
                && y > content_panel.y
                && y < content_panel.y + content_panel.h
            {
                return self.handle_screens_rename_click(settings, x, y);
            }
        }

        // Handle middle-click for Meters category (cycle meter style)
        // Only works on meter columns, not on Available Meters panel
        if event.is_middle_click() && self.category == SetupCategory::Meters {
            let x = event.x;
            let y = event.y;

            // Check if click is in content panel
            let content_panel = &self.content_panel;
            if x >= content_panel.x
                && x < content_panel.x + content_panel.w
                && y > content_panel.y
                && y < content_panel.y + content_panel.h
            {
                return self.handle_meters_mouse(crt, settings, header, x, y, false, true);
            }
        }

        // Handle right click for DisplayOptions category (decrease Number values)
        if event.is_right_click() && self.category == SetupCategory::DisplayOptions {
            let x = event.x;
            let y = event.y;

            // Check if click is in content panel
            let content_panel = &self.content_panel;
            if x >= content_panel.x
                && x < content_panel.x + content_panel.w
                && y > content_panel.y
                && y < content_panel.y + content_panel.h
            {
                self.focus = 1;

                // Calculate which item was clicked
                let item_y = (y - content_panel.y - 1) as usize;
                let clicked_index = self.content_scroll as usize + item_y;

                if clicked_index < self.display_options.len() {
                    if let Some(OptionItem::Number { field, min, .. }) =
                        self.display_options.get(clicked_index)
                    {
                        self.content_index = clicked_index;
                        // Decrease the value (clamp at min)
                        let current = field.get_int(settings);
                        let new_val = (current - 1).max(*min);
                        field.set_int(settings, new_val);
                        settings.changed = true;
                        self.changed = true;
                        return Some(HandlerResult::Handled);
                    }
                }
            }
        }

        None
    }

    /// Handle mouse event for Meters category
    /// If right_click is true, toggle move mode for the selected meter
    /// If middle_click is true, cycle the meter style
    fn handle_meters_mouse(
        &mut self,
        _crt: &mut Crt,
        settings: &mut Settings,
        header: &mut Header,
        x: i32,
        y: i32,
        right_click: bool,
        middle_click: bool,
    ) -> Option<HandlerResult> {
        let num_columns = settings.header_layout.num_columns();
        let panel_y = self.content_panel.y;
        let panel_x = self.content_panel.x;

        // Skip if click is on header row
        if y <= panel_y {
            return None;
        }

        // Use same fixed width as draw_meters_panel (matching C htop)
        const COLUMN_PANEL_WIDTH: i32 = 20;
        let available_start_x = panel_x + COLUMN_PANEL_WIDTH * num_columns as i32;

        // Check if click is in available meters panel
        // Right-click and middle-click do nothing in available meters panel
        if x >= available_start_x {
            if right_click || middle_click {
                return None;
            }
            self.focus = 1;
            self.meters_column_focus = num_columns;
            // Exit move mode when clicking on available meters
            self.meters_moving = false;
            let item_y = (y - panel_y - 1) as usize;
            let clicked_index = self.meters_available_scroll as usize + item_y;
            let meters = available_meters_for_platform();
            if clicked_index < meters.len() {
                self.meters_available_selection = clicked_index;
                // Double-click would add meter, single click just selects
                // For now, just select the meter
                return Some(HandlerResult::Handled);
            }
        } else {
            // Check which column was clicked
            for col in 0..num_columns {
                let col_start_x = panel_x + COLUMN_PANEL_WIDTH * col as i32;
                let col_end_x = col_start_x + COLUMN_PANEL_WIDTH;
                if x >= col_start_x && x < col_end_x {
                    // Right-click anywhere in any column panel toggles move mode
                    // for the currently selected meter (doesn't change focus)
                    if right_click {
                        // Only toggle if there's at least one meter in the currently focused column
                        let focused_col = self.meters_column_focus;
                        if focused_col < num_columns {
                            let focused_column_meters = &settings.header_columns[focused_col];
                            if !focused_column_meters.is_empty() {
                                self.meters_moving = !self.meters_moving;
                            }
                        }
                        return Some(HandlerResult::Handled);
                    }

                    // Left-click and middle-click behavior
                    let item_y = (y - panel_y - 1) as usize;
                    let column_meters = &settings.header_columns[col];

                    // Middle-click anywhere in column panel cycles the selected meter's style
                    if middle_click {
                        self.focus = 1;
                        self.meters_column_focus = col;
                        // Ensure selection array is large enough
                        while self.meters_column_selection.len() <= col {
                            self.meters_column_selection.push(0);
                        }
                        // Only cycle if there's at least one meter in the column
                        if !column_meters.is_empty() {
                            self.cycle_meter_style(settings, header);
                        }
                        return Some(HandlerResult::Handled);
                    }

                    if self.meters_moving {
                        // In move mode: clicking on an item moves the selected meter above it
                        // Clicking on empty space below items appends to the bottom
                        // IMPORTANT: capture source column BEFORE changing focus
                        let source_col = self.meters_column_focus;
                        let source_idx = self
                            .meters_column_selection
                            .get(source_col)
                            .copied()
                            .unwrap_or(0);

                        // Get the target column length before any mutations
                        let target_col_len = column_meters.len();
                        let clicked_on_item = item_y < target_col_len;

                        // Remove meter from source column
                        let meter = if let Some(sc) = settings.header_columns.get_mut(source_col) {
                            if source_idx < sc.len() {
                                Some(sc.remove(source_idx))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(meter) = meter {
                            // Calculate insert position
                            let insert_idx = if clicked_on_item {
                                // Clicked on an item: insert above it
                                if source_col == col && source_idx < item_y {
                                    // Same column and moving down: adjust for removed item
                                    item_y - 1
                                } else {
                                    // Different column or moving up: insert at clicked position
                                    item_y
                                }
                            } else {
                                // Clicked on empty space: append to bottom
                                if source_col == col {
                                    // Same column: length already reduced by 1 after remove
                                    settings
                                        .header_columns
                                        .get(col)
                                        .map(|c| c.len())
                                        .unwrap_or(0)
                                } else {
                                    // Different column: use original length
                                    target_col_len
                                }
                            };

                            // Ensure selection array is large enough for target column
                            while self.meters_column_selection.len() <= col {
                                self.meters_column_selection.push(0);
                            }

                            // Insert meter into target column
                            if let Some(tc) = settings.header_columns.get_mut(col) {
                                let insert_pos = insert_idx.min(tc.len());
                                tc.insert(insert_pos, meter);

                                // Update selection to the new position
                                self.focus = 1;
                                self.meters_column_focus = col;
                                self.meters_column_selection[col] = insert_pos;
                            }

                            settings.changed = true;
                            self.changed = true;
                            header.populate_from_settings(settings);
                        }
                        // Stay in move mode after move
                        return Some(HandlerResult::Handled);
                    }

                    // Not in move mode: update focus and select the clicked meter
                    self.focus = 1;
                    self.meters_column_focus = col;

                    // Ensure selection array is large enough
                    while self.meters_column_selection.len() <= col {
                        self.meters_column_selection.push(0);
                    }

                    if item_y < column_meters.len() {
                        self.meters_column_selection[col] = item_y;
                        return Some(HandlerResult::Handled);
                    }
                    break;
                }
            }
        }

        None
    }

    /// Handle mouse event for Screens category
    /// If right_click is true, toggle move mode instead of just selecting
    fn handle_screens_mouse(
        &mut self,
        settings: &mut Settings,
        x: i32,
        y: i32,
        right_click: bool,
    ) -> Option<HandlerResult> {
        let panel_y = self.content_panel.y;

        // Skip if click is on header row
        if y <= panel_y {
            return None;
        }

        // Use same fixed widths as draw_screens_panel
        const SCREENS_PANEL_WIDTH: i32 = 20;
        const COLUMNS_PANEL_WIDTH: i32 = 20;

        let screens_start_x = self.content_panel.x;
        let columns_start_x = screens_start_x + SCREENS_PANEL_WIDTH;
        let available_start_x = columns_start_x + COLUMNS_PANEL_WIDTH;

        let item_y = (y - panel_y - 1) as usize;

        // Check if click is in screens list panel
        if x >= screens_start_x && x < columns_start_x {
            // Exit columns move mode if clicking on screens panel
            if self.columns_moving {
                self.columns_moving = false;
            }
            self.focus = 1;
            self.screens_panel_focus = 0;

            if right_click {
                if self.screens_moving {
                    // Right-click anywhere in panel exits move mode, keep current selection
                    self.screens_moving = false;
                } else {
                    // Right-click anywhere enters move mode for currently selected item
                    self.screens_moving = true;
                }
                return Some(HandlerResult::Handled);
            }

            // Left-click behavior
            let num_screens = settings.screens.len();
            if self.screens_moving {
                // In move mode: insert above clicked item, or append to bottom if clicking empty space
                let source_idx = self.screens_selection;

                if source_idx < num_screens {
                    let screen = settings.screens.remove(source_idx);

                    let insert_idx = if item_y < num_screens {
                        // Clicked on an item: insert above it
                        if source_idx < item_y {
                            // Moving down: adjust for removed item
                            item_y - 1
                        } else {
                            item_y
                        }
                    } else {
                        // Clicked on empty space: append to bottom
                        settings.screens.len()
                    };

                    let insert_pos = insert_idx.min(settings.screens.len());
                    settings.screens.insert(insert_pos, screen);
                    self.screens_selection = insert_pos;
                    self.changed = true;
                    settings.changed = true;
                }
                return Some(HandlerResult::Handled);
            } else if item_y < num_screens {
                // Normal left click - just select
                self.screens_selection = item_y;
                return Some(HandlerResult::Handled);
            }
        }
        // Check if click is in active columns panel
        else if x >= columns_start_x && x < available_start_x {
            // Exit renaming mode if clicking on different panel
            self.finish_screens_renaming(settings);
            // Exit screens move mode if clicking on columns panel
            if self.screens_moving {
                self.screens_moving = false;
            }
            self.focus = 1;
            self.screens_panel_focus = 1;

            if right_click {
                if self.columns_moving {
                    // Right-click anywhere in panel exits move mode, keep current selection
                    self.columns_moving = false;
                } else {
                    // Right-click anywhere enters move mode for currently selected item
                    self.columns_moving = true;
                }
                return Some(HandlerResult::Handled);
            } else {
                let screen = &settings.screens[self.screens_selection];
                let num_columns = screen.fields.len();
                if self.columns_moving {
                    // Left click in move mode - insert above clicked item or append to bottom
                    let from = self.columns_selection;
                    let insert_pos = if item_y < num_columns {
                        // Clicked on an item - insert above it
                        item_y
                    } else {
                        // Clicked on empty space - append to bottom
                        num_columns.saturating_sub(1)
                    };

                    if from != insert_pos {
                        let screen = &mut settings.screens[self.screens_selection];
                        let column = screen.fields.remove(from);
                        // Adjust insert position if we removed from before it
                        let adjusted_pos = if from < insert_pos {
                            insert_pos - 1
                        } else {
                            insert_pos
                        };
                        screen.fields.insert(adjusted_pos, column);
                        self.columns_selection = adjusted_pos;
                        self.changed = true;
                        settings.changed = true;
                    }
                    return Some(HandlerResult::Handled);
                } else if item_y < num_columns {
                    // Normal left click - just select
                    self.columns_selection = item_y;
                    return Some(HandlerResult::Handled);
                }
            }
        }
        // Check if click is in available columns panel
        else if x >= available_start_x {
            // Exit renaming mode if clicking on different panel
            self.finish_screens_renaming(settings);
            // Exit any move mode if clicking on available columns panel
            self.screens_moving = false;
            self.columns_moving = false;
            self.focus = 1;
            self.screens_panel_focus = 2;

            if right_click {
                // Right-click anywhere triggers Add action for selected item
                self.add_column_to_active(settings);
                return Some(HandlerResult::Handled);
            } else {
                // Left-click selects item
                let available = ProcessField::all();
                let clicked_index = self.available_columns_scroll as usize + item_y;
                if clicked_index < available.len() {
                    self.available_columns_selection = clicked_index;
                    return Some(HandlerResult::Handled);
                }
            }
        }

        None
    }

    /// Handle middle-click or double-click for renaming screens
    fn handle_screens_rename_click(
        &mut self,
        settings: &mut Settings,
        x: i32,
        y: i32,
    ) -> Option<HandlerResult> {
        let panel_y = self.content_panel.y;

        // Skip if click is on header row
        if y <= panel_y {
            return None;
        }

        // Use same fixed widths as draw_screens_panel
        const SCREENS_PANEL_WIDTH: i32 = 20;

        let screens_start_x = self.content_panel.x;
        let columns_start_x = screens_start_x + SCREENS_PANEL_WIDTH;

        let item_y = (y - panel_y - 1) as usize;

        // Only handle clicks in screens list panel
        if x >= screens_start_x && x < columns_start_x {
            self.focus = 1;
            self.screens_panel_focus = 0;

            if self.screens_renaming {
                // Already renaming - finish it
                self.finish_screens_renaming(settings);
            } else if item_y < settings.screens.len() {
                // Select the item and enter renaming mode
                self.screens_selection = item_y;
                // Start renaming
                self.screens_rename_buffer =
                    settings.screens[self.screens_selection].heading.clone();
                self.screens_rename_cursor = self.screens_rename_buffer.len();
                self.screens_renaming = true;
            }
            return Some(HandlerResult::Handled);
        }

        None
    }

    /// Scroll up in the currently focused Screens panel
    /// In move mode, moves the selected element up instead of just navigating
    fn scroll_screens_up(&mut self, settings: &mut Settings) {
        match self.screens_panel_focus {
            0 => {
                // Screens list panel
                if self.screens_selection > 0 {
                    if self.screens_moving {
                        // Move mode: swap screen with the one above
                        settings
                            .screens
                            .swap(self.screens_selection, self.screens_selection - 1);
                        self.changed = true;
                        settings.changed = true;
                    }
                    self.screens_selection -= 1;
                }
            }
            1 => {
                // Active columns panel
                if self.columns_selection > 0 {
                    if self.columns_moving {
                        // Move mode: swap column with the one above
                        let screen = &mut settings.screens[self.screens_selection];
                        screen
                            .fields
                            .swap(self.columns_selection, self.columns_selection - 1);
                        self.changed = true;
                        settings.changed = true;
                    }
                    self.columns_selection -= 1;
                }
            }
            2 => {
                // Available columns panel (no move mode)
                if self.available_columns_selection > 0 {
                    self.available_columns_selection -= 1;
                    // Adjust scroll
                    if (self.available_columns_selection as i32) < self.available_columns_scroll {
                        self.available_columns_scroll = self.available_columns_selection as i32;
                    }
                }
            }
            _ => {}
        }
    }

    /// Scroll down in the currently focused Screens panel
    /// In move mode, moves the selected element down instead of just navigating
    fn scroll_screens_down(&mut self, settings: &mut Settings) {
        match self.screens_panel_focus {
            0 => {
                // Screens list panel
                if self.screens_selection < settings.screens.len().saturating_sub(1) {
                    if self.screens_moving {
                        // Move mode: swap screen with the one below
                        settings
                            .screens
                            .swap(self.screens_selection, self.screens_selection + 1);
                        self.changed = true;
                        settings.changed = true;
                    }
                    self.screens_selection += 1;
                }
            }
            1 => {
                // Active columns panel
                let max_index = settings.screens[self.screens_selection]
                    .fields
                    .len()
                    .saturating_sub(1);
                if self.columns_selection < max_index {
                    if self.columns_moving {
                        // Move mode: swap column with the one below
                        let screen = &mut settings.screens[self.screens_selection];
                        screen
                            .fields
                            .swap(self.columns_selection, self.columns_selection + 1);
                        self.changed = true;
                        settings.changed = true;
                    }
                    self.columns_selection += 1;
                }
            }
            2 => {
                // Available columns panel (no move mode)
                let available = ProcessField::all();
                if self.available_columns_selection < available.len().saturating_sub(1) {
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
            }
            _ => {}
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

        // Track if we need to redraw
        let mut needs_redraw = true;

        // Clear screen on initial entry
        crt.clear();

        loop {
            // Check if we need to scan for new data (like C htop checkRecalculation)
            let elapsed = last_update.elapsed();
            if elapsed >= delay {
                // Compute scan flags from current screen's fields for conditional /proc reads
                #[cfg(target_os = "linux")]
                {
                    machine.scan_flags = ScanFlags::from_fields(&settings.current_screen().fields);
                }

                // Scan machine for updated system data
                platform::scan(machine);
                // In setup screen, we don't need merged command strings
                machine.update_processes(
                    None,
                    "│",
                    settings.highlight_changes,
                    settings.highlight_delay_secs,
                );

                // Always update header meters to avoid gaps in graph meters
                // This matches C htop's checkRecalculation() which always calls Header_updateData()
                header.update(machine);

                last_update = std::time::Instant::now();
                needs_redraw = true;
            }

            // Draw only when needed
            if needs_redraw {
                self.draw(crt, settings, header, machine);
                needs_redraw = false;
            }

            // Get input (may return ERR on halfdelay timeout)
            let key = crt.getch();

            // Skip processing if no key (halfdelay timeout)
            if key == -1 {
                continue;
            }

            // Handle resize
            if key == KEY_RESIZE {
                crt.handle_resize();
                self.layout(crt, header_height, settings.screen_tabs);
                crt.clear();
                needs_redraw = true;
                continue;
            }

            // Handle mouse events
            if key == KEY_MOUSE {
                if let Some(result) = self.handle_mouse(crt, settings, header) {
                    needs_redraw = true;

                    // If settings were changed, update header
                    if result == HandlerResult::Handled && self.changed {
                        header.set_header_margin(settings.header_margin);
                        header.update(machine);
                        let new_height = header.calculate_height();
                        header_height = new_height;
                        self.layout(crt, header_height, settings.screen_tabs);
                    }

                    if result == HandlerResult::BreakLoop {
                        break;
                    }
                }
                continue;
            }

            // Handle key
            let result = self.handle_key(key, settings, crt, header);

            // Any key press triggers a redraw
            needs_redraw = true;

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
