//! ScreenManager - Manages panels and the main event loop

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::crt::{
    ColorElement, A_NORMAL, CURSOR_INVISIBLE, CURSOR_VISIBLE, KEY_0, KEY_9, KEY_C, KEY_DOT,
    KEY_DOWN, KEY_ESC, KEY_F, KEY_F1, KEY_F2, KEY_F3, KEY_F5, KEY_F6, KEY_F7, KEY_F8, KEY_F9,
    KEY_GT, KEY_H, KEY_HASH, KEY_HEADER_CLICK, KEY_I, KEY_K, KEY_LBRACKET, KEY_LC_C, KEY_LC_E,
    KEY_LC_H, KEY_LC_K, KEY_LC_L, KEY_LC_M, KEY_LC_P, KEY_LC_S, KEY_LC_T, KEY_LC_U, KEY_LC_W,
    KEY_LC_X, KEY_M, KEY_MINUS, KEY_MOUSE, KEY_N, KEY_P, KEY_PLUS, KEY_QUESTION, KEY_RBRACKET,
    KEY_RESIZE, KEY_RETURN, KEY_S, KEY_SF3, KEY_SHIFT_TAB, KEY_SPACE, KEY_STAR, KEY_T, KEY_TAB,
    KEY_TAB_CLICK, KEY_U, KEY_WHEELDOWN, KEY_WHEELUP, KEY_Z,
};
use super::function_bar::FunctionBar;
use super::header::Header;
use super::main_panel::MainPanel;
use super::panel::HandlerResult;
use super::Crt;
#[cfg(target_os = "linux")]
use crate::core::ScanFlags;
use crate::core::{CommandStrParams, Machine, ProcessField, Settings};
use crate::platform;

/// Check if the current process can decrease nice values (increase priority).
///
/// On macOS, only root can decrease nice values.
/// On Linux, non-root users can decrease nice if RLIMIT_NICE is configured.
/// The formula is: nice_ceiling = 20 - rlim_cur
/// - RLIMIT_NICE = 0 → ceiling = 20 → cannot decrease nice
/// - RLIMIT_NICE = 20 → ceiling = 0 → can decrease to 0
/// - RLIMIT_NICE = 40 → ceiling = -20 → full range (like root)
fn can_decrease_nice() -> bool {
    // Root can always decrease nice
    if unsafe { libc::geteuid() == 0 } {
        return true;
    }

    // On Linux, check RLIMIT_NICE
    #[cfg(target_os = "linux")]
    {
        use nix::sys::resource::{getrlimit, Resource};

        if let Ok((soft_limit, _hard_limit)) = getrlimit(Resource::RLIMIT_NICE) {
            // nice_ceiling = 20 - rlim_cur
            // If RLIMIT_NICE > 0, user has some ability to decrease nice
            // (the higher the limit, the lower they can go)
            return soft_limit > 0;
        }
    }

    // On macOS (and fallback), non-root cannot decrease nice
    false
}

/// Screen manager state
pub struct ScreenManager {
    /// Header with meters
    pub header: Header,

    /// Main panel
    main_panel: MainPanel,

    /// Function bar
    function_bar: FunctionBar,

    /// Settings
    settings: Settings,

    /// Whether meters should be hidden
    hide_meters: bool,

    /// Pause updates
    paused: bool,

    /// Last update time
    last_update: Instant,

    /// Function bar temporarily hidden (for hide_function_bar mode 1)
    /// In mode 1, the bar is hidden on ESC and shown again on any other key
    function_bar_hidden: bool,

    /// Whether header meters need redrawing (optimization)
    header_needs_redraw: bool,

    /// Sort timeout counter (like C htop)
    /// When user presses keys, reset to SORT_TIMEOUT_RESET
    /// Decrements on idle, sorting only happens when this reaches 0
    sort_timeout: u8,
}

/// Number of idle cycles before sorting is allowed after user interaction
const SORT_TIMEOUT_RESET: u8 = 5;

impl ScreenManager {
    /// Create a new screen manager
    pub fn new(header: Header, _machine: &mut Machine, settings: &Settings) -> Self {
        ScreenManager {
            header,
            main_panel: MainPanel::new(),
            function_bar: FunctionBar::new(),
            settings: settings.clone(),
            hide_meters: false,
            paused: false,
            last_update: Instant::now(),
            function_bar_hidden: false,
            header_needs_redraw: true,
            sort_timeout: 0,
        }
    }

    /// Build CommandStrParams from current settings and CRT colors
    fn build_command_str_params(&self, crt: &Crt) -> CommandStrParams {
        CommandStrParams {
            show_merged_command: self.settings.show_merged_command,
            show_program_path: self.settings.show_program_path,
            find_comm_in_cmdline: self.settings.find_comm_in_cmdline,
            strip_exe_from_cmdline: self.settings.strip_exe_from_cmdline,
            show_thread_names: self.settings.show_thread_names,
            shadow_dist_path_prefix: self.settings.shadow_dist_path_prefix,
            base_attr: crt.color(ColorElement::ProcessBasename),
            comm_attr: crt.color(ColorElement::ProcessComm),
            thread_base_attr: crt.color(ColorElement::ProcessThreadBasename),
            thread_comm_attr: crt.color(ColorElement::ProcessThreadComm),
            del_exe_attr: crt.color(ColorElement::FailedRead),
            del_lib_attr: crt.color(ColorElement::ProcessTag),
            separator_attr: crt.color(ColorElement::FailedRead),
            shadow_attr: crt.color(ColorElement::ProcessShadow),
        }
    }

    /// Get tree view sort settings for the current screen.
    /// Returns (sort_key, ascending) tuple.
    /// Respects tree_view_always_by_pid setting.
    fn get_tree_sort_settings(&self) -> (ProcessField, bool) {
        let screen = &self.settings.screens[self.settings.active_screen];
        if screen.tree_view_always_by_pid {
            (ProcessField::Pid, true)
        } else {
            (screen.tree_sort_key, screen.tree_direction > 0)
        }
    }

    /// Get selected process pid and command name.
    /// Returns None if no process is selected.
    fn get_selected_pid_command(&self, machine: &Machine) -> Option<(i32, String)> {
        let pid = self.main_panel.get_selected_pid(machine)?;
        let command = machine
            .processes
            .get(pid)
            .map(|p| p.get_command().to_string())
            .unwrap_or_default();
        Some((pid, command))
    }

    /// Sync sort settings from screen to machine and settings.
    /// Call this after updating screen.sort_key and screen.direction.
    fn sync_sort_to_machine(
        &mut self,
        machine: &mut Machine,
        field: ProcessField,
        descending: bool,
    ) {
        machine.sort_key = field;
        machine.sort_descending = descending;
        self.settings.sort_key = Some(field);
        self.settings.sort_descending = descending;
    }

    /// Apply sort field selection - handles both new field and same-field (invert direction).
    /// Used by both F6 sort menu and header column clicks.
    fn apply_sort_field(&mut self, machine: &mut Machine, field: ProcessField) {
        let screen = &mut self.settings.screens[self.settings.active_screen];

        // Track final sort direction for immediate sort
        let sort_descending: bool;
        let mut rebuild_tree = false;

        if screen.tree_view && screen.tree_view_always_by_pid {
            // In tree view with treeViewAlwaysByPID: disable tree view first
            screen.tree_view = false;
            screen.sort_key = field;
            screen.direction = if field.default_sort_desc() { -1 } else { 1 };
            sort_descending = screen.direction < 0;
            self.settings.tree_view = false;
            self.main_panel.tree_view = false;
        } else if screen.tree_view {
            // In tree view (not always-by-PID): update tree_sort_key
            if field == screen.tree_sort_key {
                // Same field: invert direction
                screen.tree_direction = -screen.tree_direction;
            } else {
                // Different field: set new field with default direction
                screen.tree_sort_key = field;
                screen.tree_direction = if field.default_sort_desc() { -1 } else { 1 };
            }
            sort_descending = screen.tree_direction < 0;
            rebuild_tree = true;
        } else {
            // Not in tree view: use regular sort_key
            if field == screen.sort_key {
                // Same field: invert direction
                screen.direction = -screen.direction;
            } else {
                // Different field: set new field with default direction
                screen.sort_key = field;
                screen.direction = if field.default_sort_desc() { -1 } else { 1 };
            }
            sort_descending = screen.direction < 0;
        }

        // Sync to machine and settings
        self.sync_sort_to_machine(machine, field, sort_descending);
        self.settings.changed = true;

        // Rebuild tree or sort immediately
        if rebuild_tree {
            machine.processes.build_tree(field, !sort_descending);
        } else {
            machine.processes.sort_by(field, !sort_descending);
        }
        self.main_panel.invalidate_display_list();
    }

    /// Take the settings out of ScreenManager (consumes internal settings)
    /// Used at the end of run() to return the potentially modified settings
    pub fn take_settings(self) -> Settings {
        self.settings
    }

    /// Get a reference to the current settings
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Add a panel (for compatibility - we have a single main panel)
    pub fn add_panel(&mut self, panel: MainPanel) {
        self.main_panel = panel;
    }

    /// Calculate layout based on terminal size
    fn layout(&mut self, crt: &mut Crt) {
        let width = crt.width();
        let height = crt.height();

        // Calculate header height
        let header_height = if self.hide_meters {
            0
        } else {
            self.header.calculate_height()
        };

        // Screen tabs take 1 line if enabled
        let screen_tabs_height = if self.settings.screen_tabs { 1 } else { 0 };

        // Function bar takes 1 line at the bottom
        // Mode 0: always show
        // Mode 1: hide on ESC until next input (tracked by function_bar_hidden)
        // Mode 2: always hide
        let function_bar_height = match self.settings.hide_function_bar {
            0 => 1,
            1 => {
                if self.function_bar_hidden {
                    0
                } else {
                    1
                }
            }
            _ => 0, // Mode 2+: always hide
        };

        // Main panel gets the rest
        let panel_y = header_height + screen_tabs_height;
        let panel_height = height - header_height - screen_tabs_height - function_bar_height;

        self.main_panel.move_to(0, panel_y);
        self.main_panel.resize(width, panel_height);
    }

    /// Draw screen tabs (like "[Main] [I/O]" above the process panel)
    /// When paused, shows "⏸ [Main] [I/O]" with pause icon before the tabs
    fn draw_screen_tabs(&mut self, crt: &mut Crt) {
        const SCREEN_TAB_MARGIN_LEFT: i32 = 2;

        let y = self.main_panel.y - 1;
        let max_x = crt.width();
        let reset_color = crt.color(ColorElement::ResetColor);
        let cur_border_attr = crt.color(ColorElement::ScreensCurBorder);
        let cur_text_attr = crt.color(ColorElement::ScreensCurText);
        let other_border_attr = crt.color(ColorElement::ScreensOthBorder);
        let other_text_attr = crt.color(ColorElement::ScreensOthText);

        // Prepare pause indicator info
        let paused = self.paused;
        let pause_indicator = if crt.utf8 { " ⏸ " } else { " PAUSED " };
        let pause_width = if crt.utf8 { 4 } else { 8 };
        let paused_color = crt.color(ColorElement::ScreensCurText);

        // Prepare tab info
        let tabs: Vec<_> = self
            .settings
            .screens
            .iter()
            .enumerate()
            .map(|(i, screen)| {
                let is_current = i == self.settings.active_screen;
                (screen.heading.clone(), is_current)
            })
            .collect();

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

        // Draw pause indicator before tabs if paused
        if paused {
            crt.attrset(paused_color);
            crt.mv(y, x);
            crt.addstr_raw(pause_indicator);
            x += pause_width;

            // Add a space after the indicator (with reset color)
            crt.attrset(reset_color);
            crt.addstr_raw(" ");
            x += 1;

            if x >= max_x {
                crt.attrset(reset_color);
                return;
            }
        }

        // Draw all tabs
        for (i, (heading, is_current)) in tabs.iter().enumerate() {
            let border_attr = if *is_current {
                cur_border_attr
            } else {
                other_border_attr
            };
            let text_attr = if *is_current {
                cur_text_attr
            } else {
                other_text_attr
            };

            // Draw '['
            crt.attrset(border_attr);
            crt.mvaddch_raw(y, x, '[' as u32);
            x += 1;

            if x >= max_x {
                crt.attrset(reset_color);
                return;
            }

            // Draw heading text
            let name_width = heading.len().min((max_x - x) as usize);
            crt.attrset(text_attr);
            crt.mv(y, x);
            crt.addnstr_raw(heading, name_width as i32);
            x += name_width as i32;

            if x >= max_x {
                crt.attrset(reset_color);
                return;
            }

            // Draw ']'
            crt.attrset(border_attr);
            crt.mvaddch_raw(y, x, ']' as u32);
            x += 1;

            // Space between tabs
            if i < tabs.len() - 1 {
                x += 1;
            }

            if x >= max_x {
                break;
            }
        }

        // Only reset at the very end (matches C htop)
        crt.attrset(reset_color);
    }

    /// Calculate tab boundaries for mouse click detection
    /// Returns (tab_row_y, Vec<(start_x, end_x)>) for each tab
    fn calculate_tab_boundaries(&self, crt: &Crt) -> (i32, Vec<(i32, i32)>) {
        const SCREEN_TAB_MARGIN_LEFT: i32 = 2;

        let y = self.main_panel.y - 1;
        let max_x = crt.width();
        let mut boundaries = Vec::new();

        // Calculate pause indicator width if paused
        let pause_offset = if self.paused {
            let pause_width = if crt.utf8 { 4 } else { 8 };
            pause_width + 1 // +1 for space after indicator
        } else {
            0
        };

        let mut x = SCREEN_TAB_MARGIN_LEFT + pause_offset;

        // Calculate boundaries for each tab
        for (i, screen) in self.settings.screens.iter().enumerate() {
            if x >= max_x {
                break;
            }

            let start_x = x;
            // Tab is "[heading]" so width = 1 + heading.len() + 1
            let tab_width = 1 + screen.heading.len() as i32 + 1;
            let end_x = (start_x + tab_width - 1).min(max_x - 1);

            boundaries.push((start_x, end_x));

            x += tab_width;
            // Space between tabs
            if i < self.settings.screens.len() - 1 {
                x += 1;
            }
        }

        (y, boundaries)
    }

    /// Draw the incremental search/filter bar at the bottom of the screen
    /// Matches C htop IncSet_drawBar and FunctionBar_drawExtra
    /// Draw the incremental search/filter bar and return the ending x position
    fn draw_inc_bar(&mut self, crt: &mut Crt, y: i32) -> i32 {
        let bar_color = crt.color(ColorElement::FunctionBar);
        let key_color = crt.color(ColorElement::FunctionKey);
        let width = crt.width();
        let is_filter = self.main_panel.inc_search.is_filter();
        let is_search = self.main_panel.inc_search.is_search();
        let search_text = self.main_panel.inc_search.text.clone();
        let search_found = self.main_panel.inc_search.found;
        let text_attr = if !search_found {
            crt.color(ColorElement::FailedSearch)
        } else {
            bar_color
        };

        // Fill the entire line with function bar color first
        crt.mv(y, 0);
        crt.attrset(bar_color);
        for _ in 0..width {
            crt.addch_raw(' ' as u32);
        }
        crt.attrset(A_NORMAL);

        // Move back to start of line
        crt.mv(y, 0);

        let mut x = 0i32;
        let mut show_cursor = false;

        if is_filter {
            // Filter mode: "Enter" "Done  " "Esc" "Clear " "  " " Filter: " [text]
            // Draw "Enter" key
            crt.attrset(key_color);
            crt.addstr_raw("Enter");
            x += 5;
            crt.attrset(A_NORMAL);

            // Draw "Done  " label
            crt.attrset(bar_color);
            crt.addstr_raw("Done  ");
            x += 6;
            crt.attrset(A_NORMAL);

            // Draw "Esc" key
            crt.attrset(key_color);
            crt.addstr_raw("Esc");
            x += 3;
            crt.attrset(A_NORMAL);

            // Draw "Clear " label
            crt.attrset(bar_color);
            crt.addstr_raw("Clear ");
            x += 6;
            crt.attrset(A_NORMAL);

            // Draw "  " spacer (acts as visual separator)
            crt.attrset(key_color);
            crt.addstr_raw("  ");
            x += 2;
            crt.attrset(A_NORMAL);

            // Draw " Filter: " label
            crt.attrset(bar_color);
            crt.addstr_raw(" Filter: ");
            x += 9;
            crt.attrset(A_NORMAL);

            // Draw the filter text
            crt.attrset(bar_color);
            crt.addstr_raw(&search_text);
            x += search_text.len() as i32;
            crt.attrset(A_NORMAL);

            // Show cursor
            show_cursor = true;
        } else if is_search {
            // Search mode: "F3" "Next  " "S-F3" "Prev   " "Esc" "Cancel " "  " " Search: " [text]
            // Draw "F3" key
            crt.attrset(key_color);
            crt.addstr_raw("F3");
            x += 2;
            crt.attrset(A_NORMAL);

            // Draw "Next  " label
            crt.attrset(bar_color);
            crt.addstr_raw("Next  ");
            x += 6;
            crt.attrset(A_NORMAL);

            // Draw "S-F3" key (Shift-F3)
            crt.attrset(key_color);
            crt.addstr_raw("S-F3");
            x += 4;
            crt.attrset(A_NORMAL);

            // Draw "Prev   " label
            crt.attrset(bar_color);
            crt.addstr_raw("Prev   ");
            x += 7;
            crt.attrset(A_NORMAL);

            // Draw "Esc" key
            crt.attrset(key_color);
            crt.addstr_raw("Esc");
            x += 3;
            crt.attrset(A_NORMAL);

            // Draw "Cancel " label
            crt.attrset(bar_color);
            crt.addstr_raw("Cancel ");
            x += 7;
            crt.attrset(A_NORMAL);

            // Draw "  " spacer
            crt.attrset(key_color);
            crt.addstr_raw("  ");
            x += 2;
            crt.attrset(A_NORMAL);

            // Draw " Search: " label
            crt.attrset(bar_color);
            crt.addstr_raw(" Search: ");
            x += 9;
            crt.attrset(A_NORMAL);

            // Draw the search text (with failed search color if not found)
            crt.attrset(text_attr);
            crt.addstr_raw(&search_text);
            x += search_text.len() as i32;
            crt.attrset(A_NORMAL);

            // Show cursor
            show_cursor = true;
        }

        if show_cursor {
            crt.curs_set(CURSOR_VISIBLE);
        }

        x
    }

    /// Draw the entire screen
    fn draw(&mut self, crt: &mut Crt, machine: &mut Machine) {
        // Update function bar labels based on current state
        self.update_function_bar_labels();

        // Note: We don't call crt.clear() here - ncurses handles differential updates.
        // Only call clear() when needed (e.g., after dialogs, resize, etc.)

        // Draw header meters only when data changed (optimization)
        if !self.hide_meters && self.header_needs_redraw {
            self.header.draw(crt, machine, &self.settings);
            self.header_needs_redraw = false;
        }

        // Draw screen tabs if enabled
        if self.settings.screen_tabs {
            self.draw_screen_tabs(crt);
        }

        // Draw main panel
        self.main_panel.draw(crt, machine, &self.settings, true);

        // Draw function bar or search/filter bar
        // Mode 0: always show
        // Mode 1: hide on ESC until next input (tracked by function_bar_hidden)
        // Mode 2: always hide
        let show_function_bar = match self.settings.hide_function_bar {
            0 => true,
            1 => !self.function_bar_hidden,
            _ => false, // Mode 2+: always hide
        };

        if show_function_bar {
            let y = crt.height() - 1;
            let end_x = if self.main_panel.inc_search.active {
                // Draw search/filter bar
                self.draw_inc_bar(crt, y)
            } else {
                // Draw normal function bar and hide cursor
                let x = self.main_panel.function_bar.draw_simple_return_x(crt, y);
                crt.curs_set(CURSOR_INVISIBLE);
                x
            };

            // If paused, append "PAUSED" indicator (like C htop MainPanel_drawFunctionBar)
            if self.paused {
                let paused_color = crt.color(ColorElement::Paused);
                crt.attrset(paused_color);
                crt.mv(y, end_x + 1);
                crt.addstr_raw("PAUSED");
                crt.attrset(A_NORMAL);
            }

            // Show update interval counter on the right side (debug builds only)
            #[cfg(debug_assertions)]
            {
                let delay_ms = self.settings.delay as u64 * 100;
                let elapsed_ms = self.last_update.elapsed().as_millis() as u64;
                let remaining_ms = delay_ms.saturating_sub(elapsed_ms);
                let remaining_secs = remaining_ms as f64 / 1000.0;
                let counter_str = format!("{:.1}s", remaining_secs);
                let counter_x = crt.width() - counter_str.len() as i32;
                crt.mv(y, counter_x);
                crt.attrset(crt.color(ColorElement::MeterText));
                crt.addstr_raw(&counter_str);
                crt.attrset(A_NORMAL);
            }
        }

        crt.refresh();
    }

    /// Handle resize
    fn handle_resize(&mut self, crt: &mut Crt) {
        crt.update_size();
        crt.clear();
        self.layout(crt);
        // Force full redraw after resize
        self.header_needs_redraw = true;
        self.main_panel.needs_redraw = true;
    }

    /// Run the main event loop
    pub fn run(
        &mut self,
        crt: &mut Crt,
        machine: &mut Machine,
        running: &AtomicBool,
    ) -> anyhow::Result<()> {
        // Copy settings to machine for platform access
        machine.update_process_names = self.settings.update_process_names;
        machine.show_cpu_frequency = self.settings.show_cpu_frequency;
        machine.hide_userland_threads = self.settings.hide_userland_threads;

        // Compute scan flags from current screen's fields for conditional /proc reads
        #[cfg(target_os = "linux")]
        {
            machine.scan_flags = ScanFlags::from_fields(&self.settings.current_screen().fields);
        }

        // Initial scan BEFORE layout so we know actual CPU count for meter heights
        platform::scan(machine);
        let cmd_params = self.build_command_str_params(crt);
        machine.update_processes(
            Some(&cmd_params),
            crt.tree_str.vert,
            self.settings.highlight_changes,
            self.settings.highlight_delay_secs,
        );
        self.header.update(machine);
        self.last_update = Instant::now();

        // Now calculate layout with correct meter heights
        self.layout(crt);

        // Use a fixed 0.1s (1 tenth) halfdelay for responsive counter updates
        // The actual update interval is controlled by self.settings.delay and self.last_update
        crt.set_delay(1);

        // Initialize tree view state and update function bar labels
        self.main_panel.tree_view = self.settings.tree_view;
        self.update_function_bar_labels();

        // Disable F7 "Nice -" when user cannot decrease nice values.
        // On macOS, only root can decrease nice.
        // On Linux, RLIMIT_NICE may allow non-root users to decrease nice.
        // F7 is at index 6 (0-based: F1=0, F2=1, ..., F7=6)
        self.main_panel
            .function_bar
            .set_enabled(6, can_decrease_nice());

        // Build tree if starting in tree view mode
        if self.settings.tree_view {
            if !self.settings.all_branches_collapsed {
                machine.processes.expand_all();
            }
            let (sort_key, ascending) = self.get_tree_sort_settings();
            machine.processes.build_tree(sort_key, ascending);
        }

        // Redraw flag - matches C htop behavior
        // Only redraw when data changed or user pressed a key
        let mut redraw = true;

        loop {
            // Check if we should exit
            if !running.load(Ordering::SeqCst) {
                break;
            }

            // Check iteration limit
            if machine.iterations_remaining == 0 {
                break;
            }

            // Determine if we should update (time-based, like C htop checkRecalculation)
            let should_update = !self.paused
                && self.last_update.elapsed()
                    >= Duration::from_millis(self.settings.delay as u64 * 100);

            if should_update {
                // Update settings in machine before scan
                machine.update_process_names = self.settings.update_process_names;
                machine.show_cpu_frequency = self.settings.show_cpu_frequency;
                machine.hide_userland_threads = self.settings.hide_userland_threads;

                // Compute scan flags from current screen's fields for conditional /proc reads
                #[cfg(target_os = "linux")]
                {
                    machine.scan_flags =
                        ScanFlags::from_fields(&self.settings.current_screen().fields);
                }

                // Only allow sorting when sort_timeout has elapsed (like C htop)
                // This defers sorting during rapid user interaction
                if self.sort_timeout == 0 || self.settings.tree_view {
                    machine.needs_sort = true;
                }

                // Perform platform scan to update system state
                platform::scan(machine);
                let cmd_params = self.build_command_str_params(crt);
                machine.update_processes(
                    Some(&cmd_params),
                    crt.tree_str.vert,
                    self.settings.highlight_changes,
                    self.settings.highlight_delay_secs,
                );

                // Build tree if in tree view mode
                if self.settings.tree_view {
                    let (sort_key, ascending) = self.get_tree_sort_settings();
                    machine.processes.build_tree(sort_key, ascending);
                }

                // Update header meters with new data
                self.header.update(machine);

                // Data changed - invalidate the cached display list so it gets rebuilt
                // Header always needs redraw to show updated meters
                self.header_needs_redraw = true;
                self.main_panel.invalidate_display_list();

                // Only force full process list redraw when user is NOT actively interacting
                // This keeps navigation smooth while data updates in the background
                // sort_timeout > 0 means user recently pressed a key
                if self.sort_timeout == 0 {
                    self.main_panel.needs_redraw = true;
                }

                self.last_update = Instant::now();

                // Decrement iteration count if set
                if machine.iterations_remaining > 0 {
                    machine.iterations_remaining -= 1;
                }

                // Data updated - need to redraw
                redraw = true;
            }

            // Only draw when needed (matches C htop behavior)
            // This avoids unnecessary redraws during halfdelay timeout
            if redraw {
                self.draw(crt, machine);
                // Mark initial scan as done after first draw (for highlight_changes feature)
                // This prevents all processes from being highlighted as "new" on startup
                // Also mark ALL processes as was_shown to prevent filtered processes from
                // appearing as "new" when filter changes
                if !machine.initial_scan_done {
                    machine.initial_scan_done = true;
                    for process in &mut machine.processes.processes {
                        process.was_shown = true;
                    }
                }
            }

            // Wait for input (halfdelay mode - returns after timeout or key press)
            let mut key = crt.read_key();

            // Handle mouse events
            if let Some(k) = key {
                if k == KEY_MOUSE {
                    // Process mouse event and convert to key code
                    // Pass the panel's header y position for header click detection
                    let panel_y = if self.main_panel.show_header {
                        Some(self.main_panel.panel_y())
                    } else {
                        None
                    };
                    // Calculate tab boundaries for screen tab click detection
                    let tab_info = if self.settings.screen_tabs {
                        let (tab_y, boundaries) = self.calculate_tab_boundaries(crt);
                        Some((tab_y, boundaries))
                    } else {
                        None
                    };
                    // Function bar click handler - only if function bar is visible and search is not active
                    let show_function_bar = match self.settings.hide_function_bar {
                        0 => true,
                        1 => !self.function_bar_hidden,
                        _ => false,
                    };
                    // When search/filter bar is active, use inc bar click handler
                    // Otherwise use function bar click handler
                    let inc_search_active = self.main_panel.inc_search.active;
                    let is_filter = self.main_panel.inc_search.is_filter();
                    let is_search = self.main_panel.inc_search.is_search();
                    let func_bar_click: Option<&dyn Fn(i32) -> Option<i32>> = if inc_search_active {
                        // Search/filter bar click handler
                        Some(&|x: i32| {
                            if is_filter {
                                // Filter mode: "Enter" "Done  " "Esc" "Clear " ...
                                if x < 11 {
                                    return Some(KEY_RETURN); // Enter
                                } else if x < 20 {
                                    return Some(KEY_ESC); // Esc
                                }
                            } else if is_search {
                                // Search mode: "F3" "Next  " "S-F3" "Prev   " "Esc" "Cancel " ...
                                if x < 8 {
                                    return Some(KEY_F3); // F3 - Next
                                } else if x < 19 {
                                    return Some(KEY_SF3); // Shift-F3 - Prev
                                } else if x < 29 {
                                    return Some(KEY_ESC); // Esc - Cancel
                                }
                            }
                            None
                        })
                    } else if show_function_bar {
                        let func_bar = &self.main_panel.function_bar;
                        Some(&|x: i32| func_bar.get_click_key(x))
                    } else {
                        None
                    };
                    key = crt.process_mouse_event(
                        crt.height(),
                        panel_y,
                        tab_info.as_ref().map(|(y, b)| (*y, b.as_slice())),
                        func_bar_click,
                    );
                }
            }

            if let Some(key) = key {
                // User pressed a key - reset sort timeout to defer sorting
                self.sort_timeout = SORT_TIMEOUT_RESET;

                let result = self.handle_key(key, crt, machine);

                match result {
                    HandlerResult::BreakLoop => break,
                    HandlerResult::Resize => self.handle_resize(crt),
                    HandlerResult::Redraw => {
                        crt.clear();
                        // Force full redraw of header and main panel after clear
                        self.header_needs_redraw = true;
                        self.main_panel.needs_redraw = true;
                    }
                    _ => {}
                }

                // Key was pressed - redraw on next iteration
                redraw = true;
            } else {
                // No key pressed (halfdelay timeout) - decrement sort timeout
                if self.sort_timeout > 0 {
                    self.sort_timeout -= 1;
                }
                // In debug builds, always redraw to update the countdown timer
                // In release builds, skip redraw when there's no input
                #[cfg(debug_assertions)]
                {
                    redraw = true;
                }
                #[cfg(not(debug_assertions))]
                {
                    redraw = false;
                }
            }
        }

        Ok(())
    }

    /// Handle a key event
    fn handle_key(&mut self, key: i32, crt: &mut Crt, machine: &mut Machine) -> HandlerResult {
        // Handle hide_function_bar mode 1:
        // - ESC hides the function bar temporarily
        // - Any other key shows it again
        if self.settings.hide_function_bar == 1 {
            if key == KEY_ESC && !self.main_panel.inc_search.active {
                // ESC hides the function bar (when not in search/filter mode)
                if !self.function_bar_hidden {
                    self.function_bar_hidden = true;
                    self.layout(crt);
                    return HandlerResult::Handled;
                }
            } else if self.function_bar_hidden {
                // Any other key shows the function bar again
                self.function_bar_hidden = false;
                self.layout(crt);
                // Don't return - continue processing the key
            }
        }

        // If search/filter mode is active, pass keys to main panel FIRST
        // This prevents global shortcuts from intercepting typed characters
        if self.main_panel.inc_search.active {
            let result = self.main_panel.on_key(key, machine);
            if result != HandlerResult::Ignored {
                return result;
            }
        }

        // Global key handling
        match key {
            KEY_RESIZE => {
                return HandlerResult::Resize;
            }
            KEY_F1 | KEY_QUESTION => {
                // F1 or '?' - show help
                self.show_help(crt);
                return HandlerResult::Redraw;
            }
            KEY_F2 => {
                self.show_setup(crt, machine);
                // Relayout in case header layout changed (meters moved/reorganized)
                self.layout(crt);
                return HandlerResult::Redraw;
            }
            KEY_F5 | KEY_LC_T => {
                // F5 or 't' - Toggle tree view (like C htop actionToggleTreeView)
                self.toggle_tree_view(machine);
                return HandlerResult::Handled;
            }
            KEY_F6 => {
                // Sort by
                self.show_sort_menu(crt, machine);
                return HandlerResult::Redraw;
            }
            KEY_F7 | KEY_RBRACKET => {
                // F7 or ']' - higher priority (nice -)
                // Applies to tagged processes if any, otherwise selected process
                if !self.settings.readonly {
                    let ok = self.change_priority_for_processes(machine, -1);
                    if !ok {
                        crt.beep();
                    }
                }
                return HandlerResult::Handled;
            }
            KEY_F8 | KEY_LBRACKET => {
                // F8 or '[' - lower priority (nice +)
                // Applies to tagged processes if any, otherwise selected process
                if !self.settings.readonly {
                    let ok = self.change_priority_for_processes(machine, 1);
                    if !ok {
                        crt.beep();
                    }
                }
                return HandlerResult::Handled;
            }
            KEY_F9 | KEY_LC_K => {
                // F9 or 'k' - kill
                if !self.settings.readonly {
                    self.show_kill_menu(crt, machine);
                }
                return HandlerResult::Redraw;
            }
            KEY_WHEELUP => {
                // Scroll up by scroll wheel amount (matches C htop PANEL_SCROLL)
                let amount = crt.scroll_wheel_amount();
                self.main_panel.scroll_wheel(-amount, machine);
                return HandlerResult::Handled;
            }
            KEY_WHEELDOWN => {
                // Scroll down by scroll wheel amount (matches C htop PANEL_SCROLL)
                let amount = crt.scroll_wheel_amount();
                self.main_panel.scroll_wheel(amount, machine);
                return HandlerResult::Handled;
            }
            KEY_HEADER_CLICK => {
                // Handle click on header row - change sort field or invert order
                if let Some(event) = crt.last_mouse_event() {
                    if let Some(field) = self.main_panel.field_at_x(event.x) {
                        self.apply_sort_field(machine, field);
                    }
                }
                return HandlerResult::Handled;
            }
            KEY_SPACE => {
                // Space - tag process (like C htop actionTag)
                if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                    machine.processes.toggle_tag(pid);
                }
                // Move selection down after tagging
                self.main_panel.on_key(KEY_DOWN, machine);
                return HandlerResult::Handled;
            }
            KEY_HASH => {
                // '#' - hide/show header meters
                self.hide_meters = !self.hide_meters;
                self.layout(crt);
                return HandlerResult::Redraw;
            }
            KEY_PLUS => {
                // '+' - expand tree node
                if self.settings.tree_view {
                    if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                        machine.processes.expand_tree(pid);
                        let (sort_key, ascending) = self.get_tree_sort_settings();
                        machine.processes.build_tree(sort_key, ascending);
                    }
                }
                return HandlerResult::Handled;
            }
            KEY_MINUS => {
                // '-' - collapse tree node
                if self.settings.tree_view {
                    if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                        machine.processes.collapse_tree(pid);
                        let (sort_key, ascending) = self.get_tree_sort_settings();
                        machine.processes.build_tree(sort_key, ascending);
                    }
                }
                return HandlerResult::Handled;
            }
            KEY_STAR => {
                // '*' - toggle all tree nodes
                if self.settings.tree_view {
                    machine.processes.toggle_all_tree();
                    let (sort_key, ascending) = self.get_tree_sort_settings();
                    machine.processes.build_tree(sort_key, ascending);
                }
                return HandlerResult::Handled;
            }
            KEY_DOT | KEY_GT => {
                // '.' or '>' - select sort column (same as F6)
                self.show_sort_menu(crt, machine);
                return HandlerResult::Redraw;
            }
            KEY_C | KEY_S => {
                // 'C' or 'S' - setup (same as F2)
                self.show_setup(crt, machine);
                // Relayout in case header layout changed (meters moved/reorganized)
                self.layout(crt);
                return HandlerResult::Redraw;
            }
            KEY_F => {
                // 'F' - cursor follows process
                self.main_panel.toggle_following(machine);
                return HandlerResult::Handled;
            }
            KEY_H => {
                // 'H' - hide/show user process threads
                // Remember currently selected PID before changing filter
                let selected_pid = self.main_panel.get_selected_pid(machine);

                self.settings.hide_userland_threads = !self.settings.hide_userland_threads;
                self.settings.changed = true;
                self.main_panel.invalidate_display_list();

                // Try to keep the same process selected, fall back to first row if not visible
                if let Some(pid) = selected_pid {
                    if !self.main_panel.try_select_pid(pid, machine, &self.settings) {
                        self.main_panel.selected = 0;
                        self.main_panel.scroll_v = 0;
                    }
                }

                // Force full redraw since visible process list changed
                self.main_panel.needs_redraw = true;
                // Redraw header so Tasks meter updates immediately (shows thr shadowed/unshadowed)
                self.header_needs_redraw = true;
                return HandlerResult::Handled;
            }
            KEY_I => {
                // 'I' - invert sort order
                let screen = &mut self.settings.screens[self.settings.active_screen];
                if self.settings.tree_view && !screen.tree_view_always_by_pid {
                    // In tree view: toggle tree direction and rebuild tree
                    screen.tree_direction = -screen.tree_direction;
                    let sort_key = screen.tree_sort_key;
                    let ascending = screen.tree_direction > 0;
                    machine.processes.build_tree(sort_key, ascending);
                } else {
                    // In list view: toggle sort direction
                    screen.direction = -screen.direction;
                    machine.sort_descending = !machine.sort_descending;
                    self.settings.sort_descending = machine.sort_descending;
                    // Sort immediately so the display updates right away
                    let ascending = !machine.sort_descending;
                    machine.processes.sort_by(machine.sort_key, ascending);
                }
                self.settings.changed = true;
                // Process order changed, need full redraw
                self.main_panel.invalidate_display_list();
                self.main_panel.needs_redraw = true;
                return HandlerResult::Handled;
            }
            KEY_K => {
                // 'K' - hide/show kernel threads
                // Remember currently selected PID before changing filter
                let selected_pid = self.main_panel.get_selected_pid(machine);

                self.settings.hide_kernel_threads = !self.settings.hide_kernel_threads;
                self.settings.changed = true;
                self.main_panel.invalidate_display_list();

                // Try to keep the same process selected, fall back to first row if not visible
                if let Some(pid) = selected_pid {
                    if !self.main_panel.try_select_pid(pid, machine, &self.settings) {
                        self.main_panel.selected = 0;
                        self.main_panel.scroll_v = 0;
                    }
                }

                // Force full redraw since visible process list changed
                self.main_panel.needs_redraw = true;
                // Redraw header so Tasks meter updates immediately (shows kthr shadowed/unshadowed)
                self.header_needs_redraw = true;
                return HandlerResult::Handled;
            }
            KEY_M => {
                // 'M' - sort by MEM%
                machine.sort_key = ProcessField::PercentMem;
                machine.sort_descending = true;
                // Sort immediately so the display updates right away
                machine.processes.sort_by(machine.sort_key, false);
                // Process order changed, need full redraw
                self.main_panel.invalidate_display_list();
                self.main_panel.needs_redraw = true;
                return HandlerResult::Handled;
            }
            KEY_N => {
                // 'N' - sort by PID
                machine.sort_key = ProcessField::Pid;
                machine.sort_descending = false;
                // Sort immediately so the display updates right away
                machine.processes.sort_by(machine.sort_key, true);
                // Process order changed, need full redraw
                self.main_panel.invalidate_display_list();
                self.main_panel.needs_redraw = true;
                return HandlerResult::Handled;
            }
            KEY_P => {
                // 'P' - sort by CPU%
                machine.sort_key = ProcessField::PercentCpu;
                machine.sort_descending = true;
                // Sort immediately so the display updates right away
                machine.processes.sort_by(machine.sort_key, false);
                // Process order changed, need full redraw
                self.main_panel.invalidate_display_list();
                self.main_panel.needs_redraw = true;
                return HandlerResult::Handled;
            }
            KEY_T => {
                // 'T' - sort by TIME
                machine.sort_key = ProcessField::Time;
                machine.sort_descending = true;
                // Sort immediately so the display updates right away
                machine.processes.sort_by(machine.sort_key, false);
                // Process order changed, need full redraw
                self.main_panel.invalidate_display_list();
                self.main_panel.needs_redraw = true;
                return HandlerResult::Handled;
            }
            KEY_U => {
                // 'U' - untag all processes
                machine.processes.untag_all();
                return HandlerResult::Handled;
            }
            KEY_Z => {
                // 'Z' - pause/resume process updates
                self.paused = !self.paused;
                return HandlerResult::Handled;
            }
            KEY_LC_C => {
                // 'c' - tag process and its children
                if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                    machine.processes.tag_with_children(pid);
                }
                return HandlerResult::Handled;
            }
            KEY_LC_E => {
                // 'e' - show process environment
                if let Some((pid, command)) = self.get_selected_pid_command(machine) {
                    super::process_info_screens::show_process_env(crt, pid, &command);
                }
                return HandlerResult::Redraw;
            }
            KEY_LC_H => {
                // 'h' - show help (same as F1)
                self.show_help(crt);
                return HandlerResult::Redraw;
            }
            KEY_LC_L => {
                // 'l' - list open files with lsof
                if !self.settings.readonly {
                    if let Some((pid, command)) = self.get_selected_pid_command(machine) {
                        super::process_info_screens::show_lsof(crt, pid, &command);
                    }
                }
                return HandlerResult::Redraw;
            }
            KEY_LC_M => {
                // 'm' - toggle merged command (like C htop actionToggleMergedCommand)
                self.settings.show_merged_command = !self.settings.show_merged_command;
                self.settings.changed = true;
                // Rebuild command strings immediately with new setting
                let cmd_params = self.build_command_str_params(crt);
                machine.update_processes(
                    Some(&cmd_params),
                    crt.tree_str.vert,
                    self.settings.highlight_changes,
                    self.settings.highlight_delay_secs,
                );
                // Display format changed, need full redraw
                self.main_panel.invalidate_display_list();
                return HandlerResult::Handled;
            }
            KEY_LC_P => {
                // 'p' - Toggle program path (like C htop actionToggleProgramPath)
                self.settings.show_program_path = !self.settings.show_program_path;
                self.settings.changed = true;
                // Rebuild command strings immediately with new setting
                let cmd_params = self.build_command_str_params(crt);
                machine.update_processes(
                    Some(&cmd_params),
                    crt.tree_str.vert,
                    self.settings.highlight_changes,
                    self.settings.highlight_delay_secs,
                );
                // Display format changed, need full redraw
                self.main_panel.invalidate_display_list();
                return HandlerResult::Handled;
            }
            KEY_LC_S => {
                // 's' - trace syscalls with strace
                if !self.settings.readonly {
                    if let Some((pid, command)) = self.get_selected_pid_command(machine) {
                        super::process_info_screens::show_strace(crt, pid, &command);
                    }
                }
                return HandlerResult::Redraw;
            }
            KEY_LC_U => {
                // 'u' - show processes of a single user
                self.show_user_menu(crt, machine);
                return HandlerResult::Redraw;
            }
            KEY_LC_W => {
                // 'w' - show command screen (wrap process command in multiple lines)
                if let Some((pid, command)) = self.get_selected_pid_command(machine) {
                    super::process_info_screens::show_command_screen(crt, pid, &command);
                }
                return HandlerResult::Redraw;
            }
            KEY_LC_X => {
                // 'x' - list file locks of process
                if let Some((pid, command)) = self.get_selected_pid_command(machine) {
                    super::process_info_screens::show_file_locks(crt, pid, &command);
                }
                return HandlerResult::Redraw;
            }
            KEY_TAB => {
                // Tab - switch to next screen tab
                self.switch_screen(1, machine);
                return HandlerResult::Redraw;
            }
            KEY_SHIFT_TAB => {
                // Shift-Tab - switch to previous screen tab
                self.switch_screen(-1, machine);
                return HandlerResult::Redraw;
            }
            k if (KEY_TAB_CLICK..KEY_TAB_CLICK + 100).contains(&k) => {
                // Tab click - switch to the clicked screen tab
                let tab_index = (k - KEY_TAB_CLICK) as usize;
                self.switch_to_screen(tab_index, machine);
                return HandlerResult::Redraw;
            }
            KEY_0..=KEY_9 => {
                // '0'-'9' - incremental PID search
                self.main_panel.start_pid_search(key as u8 as char, machine);
                return HandlerResult::Handled;
            }
            _ => {}
        }

        // Pass to main panel (if not already handled above for inc_search)

        self.main_panel.on_key(key, machine)
    }

    /// Switch to a different screen tab
    /// direction: 1 for next, -1 for previous
    fn switch_screen(&mut self, direction: i32, machine: &mut Machine) {
        let num_screens = self.settings.screens.len();
        if num_screens <= 1 {
            return;
        }

        let current = self.settings.active_screen as i32;
        let new_screen = if direction > 0 {
            ((current + 1) % num_screens as i32) as usize
        } else {
            ((current - 1 + num_screens as i32) % num_screens as i32) as usize
        };

        self.settings.active_screen = new_screen;

        // Update main panel with new screen's fields
        let screen = &self.settings.screens[new_screen];
        self.main_panel.fields = screen.fields.clone();
        self.main_panel.tree_view = screen.tree_view;

        // Update global sort settings from the screen
        self.settings.sort_key = Some(screen.sort_key);
        self.settings.sort_descending = screen.direction < 0;
        self.settings.tree_view = screen.tree_view;

        // Update machine sort settings for immediate effect
        machine.sort_key = screen.sort_key;
        machine.sort_descending = screen.direction < 0;

        // Rebuild labels for the new columns
        let has_filter = self.main_panel.filter.is_some();
        self.main_panel.update_labels(screen.tree_view, has_filter);
    }

    /// Switch to a specific screen tab by index
    fn switch_to_screen(&mut self, index: usize, machine: &mut Machine) {
        let num_screens = self.settings.screens.len();
        if index >= num_screens || index == self.settings.active_screen {
            return;
        }

        self.settings.active_screen = index;

        // Update main panel with new screen's fields
        let screen = &self.settings.screens[index];
        self.main_panel.fields = screen.fields.clone();
        self.main_panel.tree_view = screen.tree_view;

        // Update global sort settings from the screen
        self.settings.sort_key = Some(screen.sort_key);
        self.settings.sort_descending = screen.direction < 0;
        self.settings.tree_view = screen.tree_view;

        // Update machine sort settings for immediate effect
        machine.sort_key = screen.sort_key;
        machine.sort_descending = screen.direction < 0;

        // Rebuild labels for the new columns
        let has_filter = self.main_panel.filter.is_some();
        self.main_panel.update_labels(screen.tree_view, has_filter);
    }

    /// Toggle tree view - matches C htop actionToggleTreeView behavior
    fn toggle_tree_view(&mut self, machine: &mut Machine) {
        // Toggle the tree view setting
        self.settings.tree_view = !self.settings.tree_view;
        self.main_panel.tree_view = self.settings.tree_view;

        // Also update the screen settings to keep them in sync
        let screen = &mut self.settings.screens[self.settings.active_screen];
        screen.tree_view = self.settings.tree_view;

        if self.settings.tree_view {
            // Entering tree view
            if !self.settings.all_branches_collapsed {
                // Expand all branches by default (like C htop)
                machine.processes.expand_all();
            }
            // Build the tree structure
            let (sort_key, ascending) = self.get_tree_sort_settings();
            machine.processes.build_tree(sort_key, ascending);
        }

        // Invalidate display list since tree/list order is different
        self.main_panel.invalidate_display_list();
        // Force full redraw since the entire process list display changed
        self.main_panel.needs_redraw = true;

        // Mark settings as changed for saving
        self.settings.changed = true;

        // Update function bar to show "List" or "Tree"
        self.update_function_bar_labels();
    }

    /// Update function bar labels based on current state
    /// Matches C htop MainPanel_updateLabels behavior
    fn update_function_bar_labels(&mut self) {
        // F5: Show "List  " when in tree mode, "Tree  " when in list mode
        // This matches the action the key will perform
        let tree_label = if self.settings.tree_view {
            "List  "
        } else {
            "Tree  "
        };
        self.main_panel
            .function_bar
            .set_function(4, "F5", tree_label);

        // F4: Show "FILTER" (uppercase) when filter is active, "Filter" otherwise
        // Matches C htop MainPanel_updateLabels behavior
        let filter_label = if self.main_panel.is_filtering() {
            "FILTER"
        } else {
            "Filter"
        };
        self.main_panel
            .function_bar
            .set_function(3, "F4", filter_label);
    }

    /// Change process priority (nice) for tagged processes or selected process
    /// Returns true if at least one operation succeeded, false if all failed
    fn change_priority_for_processes(&mut self, machine: &mut Machine, delta: i32) -> bool {
        // Get tagged PIDs, or fall back to selected PID
        let tagged = machine.processes.get_tagged();
        let pids: Vec<i32> = if tagged.is_empty() {
            // No tagged processes - use selected
            self.main_panel
                .get_selected_pid(machine)
                .map(|p| vec![p])
                .unwrap_or_default()
        } else {
            tagged
        };

        if pids.is_empty() {
            return false;
        }

        let mut any_ok = false;
        for pid in pids {
            if let Some(new_nice) = Self::change_priority(pid, delta) {
                // Update the process in memory immediately for instant UI feedback
                if let Some(process) = machine.processes.get_mut(pid) {
                    process.nice = new_nice as i64;
                }
                any_ok = true;
            }
        }

        if any_ok {
            // Force redraw to show updated nice values immediately
            self.main_panel.needs_redraw = true;
        }

        any_ok
    }

    /// Change process priority (nice) for a single process
    /// Returns Some(new_nice) on success, None on failure
    fn change_priority(pid: i32, delta: i32) -> Option<i32> {
        #[cfg(unix)]
        {
            use std::io::Error;

            // Get current nice value
            // Clear errno first, then call getpriority
            // Note: -1 can be a valid nice value, so we check errno after
            unsafe {
                // Set errno to 0 before the call
                #[cfg(target_os = "macos")]
                {
                    *libc::__error() = 0;
                }
                #[cfg(target_os = "linux")]
                {
                    *libc::__errno_location() = 0;
                }
            }

            let current_nice = unsafe { libc::getpriority(libc::PRIO_PROCESS, pid as libc::id_t) };

            // Check if getpriority failed
            let err = Error::last_os_error();
            if current_nice == -1 && err.raw_os_error() != Some(0) {
                return None;
            }

            let new_nice = (current_nice + delta).clamp(-20, 19);

            let result =
                unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, new_nice) };

            if result == 0 {
                Some(new_nice)
            } else {
                None
            }
        }

        #[cfg(not(unix))]
        {
            let _ = (pid, delta);
            None
        }
    }

    /// Show kill signal selection menu (matches C htop SignalsPanel)
    fn show_kill_menu(&mut self, crt: &mut Crt, machine: &mut Machine) {
        let pid = match self.main_panel.get_selected_pid(machine) {
            Some(p) => p,
            None => return,
        };

        let mut ctx = super::menus::KillMenuContext {
            main_panel: &mut self.main_panel,
            header: &self.header,
            settings: &self.settings,
            hide_meters: self.hide_meters,
        };

        super::menus::show_kill_menu(crt, machine, &mut ctx, pid);
    }

    /// Show sort column selection menu (matches C htop actionSetSortColumn)
    fn show_sort_menu(&mut self, crt: &mut Crt, machine: &mut Machine) {
        let fields_copy: Vec<ProcessField> = self.main_panel.fields.clone();

        let mut ctx = super::menus::SortMenuContext {
            main_panel: &mut self.main_panel,
            header: &self.header,
            settings: &self.settings,
            hide_meters: self.hide_meters,
        };

        let result = super::menus::show_sort_menu(
            crt,
            machine,
            &mut ctx,
            self.settings.tree_view,
            machine.sort_key,
            &fields_copy,
        );

        // Apply the selection
        if let Some(field) = result.field {
            self.apply_sort_field(machine, field);
        }
    }

    /// Show help screen (matches C htop actionHelp)
    fn show_help(&self, crt: &mut Crt) {
        super::menus::show_help(crt, &self.settings);
    }

    /// Show setup screen
    fn show_setup(&mut self, crt: &mut Crt, machine: &mut Machine) {
        let mut setup_screen = super::setup::SetupScreen::new(&self.settings);
        setup_screen.run(&mut self.settings, crt, &mut self.header, machine);

        // Sync main panel with current screen's settings (in case they were changed)
        let screen = &self.settings.screens[self.settings.active_screen];
        self.main_panel.fields = screen.fields.clone();
        self.main_panel.tree_view = screen.tree_view;
        self.settings.tree_view = screen.tree_view;

        // Update function bar labels to reflect tree view state
        let has_filter = self.main_panel.filter.is_some();
        self.main_panel.update_labels(screen.tree_view, has_filter);
    }

    /// Show user selection menu (like C htop actionFilterByUser)
    /// Displays a panel on the left side with the main process list on the right
    fn show_user_menu(&mut self, crt: &mut Crt, machine: &mut Machine) {
        let mut ctx = super::menus::UserMenuContext {
            main_panel: &mut self.main_panel,
            header: &self.header,
            settings: &self.settings,
            hide_meters: self.hide_meters,
        };

        let result = super::menus::show_user_menu(crt, machine, &mut ctx);

        // Apply the selection
        if let Some(user_filter) = result.user_id {
            machine.filter_user_id = user_filter;

            // Reset selection and scroll to top when filter changes
            self.main_panel.selected = 0;
            self.main_panel.scroll_v = 0;
            self.main_panel.invalidate_display_list();
        }
    }
}
