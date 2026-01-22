//! ScreenManager - Manages panels and the main event loop

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ncurses::CURSOR_VISIBILITY;
use ncurses::*;

use super::crt::{
    ColorElement, KEY_F1, KEY_F10, KEY_F2, KEY_F3, KEY_F4, KEY_F5, KEY_F6, KEY_F7, KEY_F8, KEY_F9,
    KEY_HEADER_CLICK, KEY_SHIFT_TAB, KEY_WHEELDOWN, KEY_WHEELUP,
};
use super::function_bar::FunctionBar;
use super::header::Header;
use super::main_panel::MainPanel;
use super::panel::{HandlerResult, Panel};
use super::Crt;
use crate::core::{Machine, ProcessField, Settings};
use crate::platform;

/// Parsed lsof file entry (from lsof -F output)
#[derive(Default)]
struct LsofFileEntry {
    fd: String,        // File descriptor
    file_type: String, // File type (REG, DIR, etc.)
    mode: String,      // Access mode (r/w/u)
    device: String,    // Device number
    size: String,      // File size
    offset: String,    // File offset
    inode: String,     // Inode number
    name: String,      // File name/path
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
}

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
        }
    }

    /// Add a panel (for compatibility - we have a single main panel)
    pub fn add_panel(&mut self, panel: Box<MainPanel>) {
        self.main_panel = *panel;
    }

    /// Calculate layout based on terminal size
    fn layout(&mut self, crt: &Crt) {
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

    /// Draw screen tabs (like "[Main]" above the process panel)
    fn draw_screen_tabs(&self, crt: &Crt) {
        const SCREEN_TAB_MARGIN_LEFT: i32 = 2;

        let y = self.main_panel.y - 1;
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

        // Get current screen heading
        let heading = &self.settings.current_screen().heading;

        // Draw the tab: [heading]
        // Border color
        let border_attr = crt.color(ColorElement::ScreensCurBorder);
        let text_attr = crt.color(ColorElement::ScreensCurText);

        // Draw '[' - matches C htop: just set attr and draw, no reset between chars
        attrset(border_attr);
        mvaddch(y, x, '[' as u32);
        x += 1;

        if x >= max_x {
            attrset(reset_color);
            return;
        }

        // Draw heading text
        let name_width = heading.len().min((max_x - x) as usize);
        attrset(text_attr);
        let _ = mvaddnstr(y, x, heading, name_width as i32);
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

    /// Draw the incremental search/filter bar at the bottom of the screen
    /// Matches C htop IncSet_drawBar and FunctionBar_drawExtra
    fn draw_inc_bar(&self, crt: &Crt, y: i32) {
        let bar_color = crt.color(ColorElement::FunctionBar);
        let key_color = crt.color(ColorElement::FunctionKey);
        let width = crt.width();

        // Fill the entire line with function bar color first
        mv(y, 0);
        attrset(bar_color);
        for _ in 0..width {
            addch(' ' as u32);
        }
        attrset(A_NORMAL);

        // Move back to start of line
        mv(y, 0);

        if self.main_panel.inc_search.is_filter() {
            // Filter mode: "Enter" "Done  " "Esc" "Clear " "  " " Filter: " [text]
            // Draw "Enter" key
            attrset(key_color);
            let _ = addstr("Enter");
            attrset(A_NORMAL);

            // Draw "Done  " label
            attrset(bar_color);
            let _ = addstr("Done  ");
            attrset(A_NORMAL);

            // Draw "Esc" key
            attrset(key_color);
            let _ = addstr("Esc");
            attrset(A_NORMAL);

            // Draw "Clear " label
            attrset(bar_color);
            let _ = addstr("Clear ");
            attrset(A_NORMAL);

            // Draw "  " spacer (acts as visual separator)
            attrset(key_color);
            let _ = addstr("  ");
            attrset(A_NORMAL);

            // Draw " Filter: " label
            attrset(bar_color);
            let _ = addstr(" Filter: ");
            attrset(A_NORMAL);

            // Draw the filter text
            attrset(bar_color);
            let _ = addstr(&self.main_panel.inc_search.text);
            attrset(A_NORMAL);

            // Show cursor
            curs_set(CURSOR_VISIBILITY::CURSOR_VISIBLE);
        } else if self.main_panel.inc_search.is_search() {
            // Search mode: "F3" "Next  " "S-F3" "Prev   " "Esc" "Cancel " "  " " Search: " [text]
            // Determine if search was found
            let text_attr = if !self.main_panel.inc_search.found {
                crt.color(ColorElement::FailedSearch)
            } else {
                bar_color
            };

            // Draw "F3" key
            attrset(key_color);
            let _ = addstr("F3");
            attrset(A_NORMAL);

            // Draw "Next  " label
            attrset(bar_color);
            let _ = addstr("Next  ");
            attrset(A_NORMAL);

            // Draw "S-F3" key (Shift-F3)
            attrset(key_color);
            let _ = addstr("S-F3");
            attrset(A_NORMAL);

            // Draw "Prev   " label
            attrset(bar_color);
            let _ = addstr("Prev   ");
            attrset(A_NORMAL);

            // Draw "Esc" key
            attrset(key_color);
            let _ = addstr("Esc");
            attrset(A_NORMAL);

            // Draw "Cancel " label
            attrset(bar_color);
            let _ = addstr("Cancel ");
            attrset(A_NORMAL);

            // Draw "  " spacer
            attrset(key_color);
            let _ = addstr("  ");
            attrset(A_NORMAL);

            // Draw " Search: " label
            attrset(bar_color);
            let _ = addstr(" Search: ");
            attrset(A_NORMAL);

            // Draw the search text (with failed search color if not found)
            attrset(text_attr);
            let _ = addstr(&self.main_panel.inc_search.text);
            attrset(A_NORMAL);

            // Show cursor
            curs_set(CURSOR_VISIBILITY::CURSOR_VISIBLE);
        }
    }

    /// Draw the entire screen
    fn draw(&mut self, crt: &Crt, machine: &Machine) {
        // Update function bar labels based on current state
        self.update_function_bar_labels();

        crt.clear();

        // Draw header meters
        if !self.hide_meters {
            self.header.draw(crt, machine, &self.settings);
        }

        // Draw screen tabs if enabled
        if self.settings.screen_tabs {
            self.draw_screen_tabs(crt);
        }

        // Draw main panel
        self.main_panel.draw(crt, machine, &self.settings);

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
            if self.main_panel.inc_search.active {
                // Draw search/filter bar
                self.draw_inc_bar(crt, y);
            } else {
                // Draw normal function bar and hide cursor
                self.main_panel.function_bar.draw_simple(crt, y);
                curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
            }
        }

        crt.refresh();
    }

    /// Handle resize
    fn handle_resize(&mut self, crt: &mut Crt) {
        crt.update_size();
        self.layout(crt);
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

        // Initial scan BEFORE layout so we know actual CPU count for meter heights
        platform::scan(machine);
        self.header.update(machine);
        self.last_update = Instant::now();

        // Now calculate layout with correct meter heights
        self.layout(crt);

        // Initialize tree view state and update function bar labels
        self.main_panel.tree_view = self.settings.tree_view;
        self.update_function_bar_labels();

        // Build tree if starting in tree view mode
        if self.settings.tree_view {
            if !self.settings.all_branches_collapsed {
                machine.processes.expand_all();
            }
            machine.processes.build_tree();
        }

        loop {
            // Check if we should exit
            if !running.load(Ordering::SeqCst) {
                break;
            }

            // Check iteration limit
            if machine.iterations_remaining == 0 {
                break;
            }

            // Determine if we should update
            let should_update = !self.paused
                && self.last_update.elapsed()
                    >= Duration::from_millis(self.settings.delay as u64 * 100);

            if should_update {
                // Update settings in machine before scan
                machine.update_process_names = self.settings.update_process_names;
                machine.show_cpu_frequency = self.settings.show_cpu_frequency;

                // Perform platform scan to update system state
                platform::scan(machine);

                // Build tree if in tree view mode
                if self.settings.tree_view {
                    machine.processes.build_tree();
                }

                // Update header meters with new data
                self.header.update(machine);

                self.last_update = Instant::now();

                // Decrement iteration count if set
                if machine.iterations_remaining > 0 {
                    machine.iterations_remaining -= 1;
                }
            }

            // Draw the screen with real machine data
            self.draw(crt, machine);

            // Wait for input
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
                    key = crt.process_mouse_event(crt.height(), panel_y);
                }
            }

            if let Some(key) = key {
                let result = self.handle_key(key, crt, machine);

                match result {
                    HandlerResult::BreakLoop => break,
                    HandlerResult::Resize => self.handle_resize(crt),
                    HandlerResult::Redraw => {
                        crt.clear();
                        self.draw(crt, machine);
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Handle a key event
    fn handle_key(&mut self, key: i32, crt: &mut Crt, machine: &mut Machine) -> HandlerResult {
        // Handle hide_function_bar mode 1:
        // - ESC (0x1B) hides the function bar temporarily
        // - Any other key shows it again
        if self.settings.hide_function_bar == 1 {
            if key == 0x1B && !self.main_panel.inc_search.active {
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
            KEY_F1 | 0x3F => {
                // F1 or ?
                self.show_help(crt);
                return HandlerResult::Redraw;
            }
            KEY_F2 => {
                self.show_setup(crt, machine);
                // Relayout in case header layout changed (meters moved/reorganized)
                self.layout(crt);
                return HandlerResult::Redraw;
            }
            KEY_F5 | 0x74 => {
                // F5 or 't' - Toggle tree view (like C htop actionToggleTreeView)
                self.toggle_tree_view(machine);
                return HandlerResult::Handled;
            }
            KEY_F6 => {
                // Sort by
                self.show_sort_menu(crt, machine);
                return HandlerResult::Redraw;
            }
            KEY_F7 | 0x5D => {
                // F7 or ] - nice -
                if !self.settings.readonly {
                    if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                        Self::change_priority(pid, -1);
                    }
                }
                return HandlerResult::Handled;
            }
            KEY_F8 | 0x5B => {
                // F8 or [ - nice +
                if !self.settings.readonly {
                    if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                        Self::change_priority(pid, 1);
                    }
                }
                return HandlerResult::Handled;
            }
            KEY_F9 | 0x6B => {
                // F9 or k - kill
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
                        // In tree view with treeViewAlwaysByPID, disable tree view first
                        if self.settings.tree_view && self.settings.tree_view_always_by_pid {
                            self.settings.tree_view = false;
                            self.main_panel.tree_view = false;
                            machine.sort_descending = false;
                            self.settings.sort_descending = false;
                            machine.sort_key = field;
                            self.settings.sort_key = Some(field);
                        } else if field == machine.sort_key {
                            // Clicking on current sort column inverts the order
                            machine.sort_descending = !machine.sort_descending;
                            self.settings.sort_descending = machine.sort_descending;
                        } else {
                            // Clicking on different column changes the sort field
                            machine.sort_key = field;
                            self.settings.sort_key = Some(field);
                        }
                        self.settings.changed = true;
                        machine
                            .processes
                            .sort(machine.sort_key, machine.sort_descending);
                    }
                }
                return HandlerResult::Handled;
            }
            0x20 => {
                // Space - tag process (like C htop actionTag)
                if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                    machine.processes.toggle_tag(pid);
                }
                // Move selection down after tagging
                self.main_panel.on_key(KEY_DOWN, machine);
                return HandlerResult::Handled;
            }
            0x23 => {
                // '#' - hide/show header meters
                self.hide_meters = !self.hide_meters;
                self.layout(crt);
                return HandlerResult::Redraw;
            }
            0x2B => {
                // '+' - expand tree node
                if self.settings.tree_view {
                    if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                        machine.processes.expand_tree(pid);
                        machine.processes.build_tree();
                    }
                }
                return HandlerResult::Handled;
            }
            0x2D => {
                // '-' - collapse tree node
                if self.settings.tree_view {
                    if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                        machine.processes.collapse_tree(pid);
                        machine.processes.build_tree();
                    }
                }
                return HandlerResult::Handled;
            }
            0x2A => {
                // '*' - toggle all tree nodes
                if self.settings.tree_view {
                    machine.processes.toggle_all_tree();
                    machine.processes.build_tree();
                }
                return HandlerResult::Handled;
            }
            0x2E | 0x3E => {
                // '.' or '>' - select sort column (same as F6)
                self.show_sort_menu(crt, machine);
                return HandlerResult::Redraw;
            }
            0x43 | 0x53 => {
                // 'C' or 'S' - setup (same as F2)
                self.show_setup(crt, machine);
                // Relayout in case header layout changed (meters moved/reorganized)
                self.layout(crt);
                return HandlerResult::Redraw;
            }
            0x46 => {
                // 'F' - cursor follows process
                self.main_panel.toggle_following(machine);
                return HandlerResult::Handled;
            }
            0x48 => {
                // 'H' - hide/show user process threads
                self.settings.hide_userland_threads = !self.settings.hide_userland_threads;
                self.settings.changed = true;
                return HandlerResult::Handled;
            }
            0x49 => {
                // 'I' - invert sort order
                machine.sort_descending = !machine.sort_descending;
                self.settings.sort_descending = machine.sort_descending;
                self.settings.changed = true;
                machine
                    .processes
                    .sort(machine.sort_key, machine.sort_descending);
                return HandlerResult::Handled;
            }
            0x4B => {
                // 'K' - hide/show kernel threads
                self.settings.hide_kernel_threads = !self.settings.hide_kernel_threads;
                self.settings.changed = true;
                return HandlerResult::Handled;
            }
            0x4D => {
                // 'M' - sort by MEM%
                machine.sort_key = ProcessField::PercentMem;
                machine.sort_descending = true;
                machine
                    .processes
                    .sort(machine.sort_key, machine.sort_descending);
                return HandlerResult::Handled;
            }
            0x4E => {
                // 'N' - sort by PID
                machine.sort_key = ProcessField::Pid;
                machine.sort_descending = false;
                machine
                    .processes
                    .sort(machine.sort_key, machine.sort_descending);
                return HandlerResult::Handled;
            }
            0x50 => {
                // 'P' - sort by CPU%
                machine.sort_key = ProcessField::PercentCpu;
                machine.sort_descending = true;
                machine
                    .processes
                    .sort(machine.sort_key, machine.sort_descending);
                return HandlerResult::Handled;
            }
            0x54 => {
                // 'T' - sort by TIME
                machine.sort_key = ProcessField::Time;
                machine.sort_descending = true;
                machine
                    .processes
                    .sort(machine.sort_key, machine.sort_descending);
                return HandlerResult::Handled;
            }
            0x55 => {
                // 'U' - untag all processes
                machine.processes.untag_all();
                return HandlerResult::Handled;
            }
            0x5A => {
                // 'Z' - pause/resume process updates
                self.paused = !self.paused;
                return HandlerResult::Handled;
            }
            0x63 => {
                // 'c' - tag process and its children
                if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                    machine.processes.tag_with_children(pid);
                }
                return HandlerResult::Handled;
            }
            0x65 => {
                // 'e' - show process environment
                if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                    self.show_process_env(crt, pid);
                }
                return HandlerResult::Redraw;
            }
            0x68 => {
                // 'h' - show help (same as F1)
                self.show_help(crt);
                return HandlerResult::Redraw;
            }
            0x6C => {
                // 'l' - list open files with lsof
                if !self.settings.readonly {
                    if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                        // Get the command name for the title
                        let command = machine
                            .processes
                            .get(pid)
                            .map(|p| p.get_command().to_string())
                            .unwrap_or_default();
                        self.show_lsof(crt, pid, &command);
                    }
                }
                return HandlerResult::Redraw;
            }
            0x70 => {
                // 'p' - Toggle program path (like C htop actionToggleProgramPath)
                self.settings.show_program_path = !self.settings.show_program_path;
                self.settings.changed = true;
                return HandlerResult::Handled;
            }
            0x73 => {
                // 's' - trace syscalls with strace/dtruss
                if !self.settings.readonly {
                    if let Some(pid) = self.main_panel.get_selected_pid(machine) {
                        self.show_strace(crt, pid);
                    }
                }
                return HandlerResult::Redraw;
            }
            0x75 => {
                // 'u' - show processes of a single user
                self.show_user_menu(crt, machine);
                return HandlerResult::Redraw;
            }
            0x77 => {
                // 'w' - wrap process command lines
                self.main_panel.toggle_wrap_command();
                return HandlerResult::Handled;
            }
            0x09 => {
                // Tab - switch to next screen tab
                // Visual only for now - we only have one screen
                return HandlerResult::Handled;
            }
            KEY_SHIFT_TAB => {
                // Shift-Tab - switch to previous screen tab
                // Visual only for now - we only have one screen
                return HandlerResult::Handled;
            }
            0x30..=0x39 => {
                // '0'-'9' - incremental PID search
                self.main_panel.start_pid_search(key as u8 as char, machine);
                return HandlerResult::Handled;
            }
            _ => {}
        }

        // Pass to main panel (if not already handled above for inc_search)
        let result = self.main_panel.on_key(key, machine);

        result
    }

    /// Toggle tree view - matches C htop actionToggleTreeView behavior
    fn toggle_tree_view(&mut self, machine: &mut Machine) {
        // Toggle the tree view setting
        self.settings.tree_view = !self.settings.tree_view;
        self.main_panel.tree_view = self.settings.tree_view;

        if self.settings.tree_view {
            // Entering tree view
            if !self.settings.all_branches_collapsed {
                // Expand all branches by default (like C htop)
                machine.processes.expand_all();
            }
            // Build the tree structure
            machine.processes.build_tree();
        }

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

    /// Change process priority (nice)
    fn change_priority(pid: i32, delta: i32) {
        #[cfg(unix)]
        {
            // Get current nice value
            let current_nice = unsafe {
                // Clear errno by reading it (cross-platform approach)
                let _ = std::io::Error::last_os_error();
                let nice = libc::getpriority(libc::PRIO_PROCESS, pid as libc::id_t);
                // Check if getpriority failed (returns -1 and sets errno)
                // Note: -1 can be a valid nice value, so we need to check errno
                let err = std::io::Error::last_os_error();
                if nice == -1 && err.raw_os_error() != Some(0) {
                    return;
                }
                nice
            };

            let new_nice = (current_nice + delta).clamp(-20, 19);

            unsafe {
                libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, new_nice);
            }
        }
    }

    /// Show kill signal selection menu (matches C htop SignalsPanel)
    fn show_kill_menu(&mut self, crt: &Crt, machine: &Machine) {
        let pid = match self.main_panel.get_selected_pid(machine) {
            Some(p) => p,
            None => return,
        };

        // Platform-specific signals list (from C htop Platform.c for each OS)
        // Format matches C htop: " N SIGNAME" with leading space for single digits
        #[cfg(target_os = "linux")]
        let base_signals: &[(&str, i32)] = &[
            (" 0 Cancel", 0),
            (" 1 SIGHUP", 1),
            (" 2 SIGINT", 2),
            (" 3 SIGQUIT", 3),
            (" 4 SIGILL", 4),
            (" 5 SIGTRAP", 5),
            (" 6 SIGABRT", 6),
            (" 6 SIGIOT", 6),
            (" 7 SIGBUS", 7),
            (" 8 SIGFPE", 8),
            (" 9 SIGKILL", 9),
            ("10 SIGUSR1", 10),
            ("11 SIGSEGV", 11),
            ("12 SIGUSR2", 12),
            ("13 SIGPIPE", 13),
            ("14 SIGALRM", 14),
            ("15 SIGTERM", 15),
            ("16 SIGSTKFLT", 16),
            ("17 SIGCHLD", 17),
            ("18 SIGCONT", 18),
            ("19 SIGSTOP", 19),
            ("20 SIGTSTP", 20),
            ("21 SIGTTIN", 21),
            ("22 SIGTTOU", 22),
            ("23 SIGURG", 23),
            ("24 SIGXCPU", 24),
            ("25 SIGXFSZ", 25),
            ("26 SIGVTALRM", 26),
            ("27 SIGPROF", 27),
            ("28 SIGWINCH", 28),
            ("29 SIGIO", 29),
            ("29 SIGPOLL", 29),
            ("30 SIGPWR", 30),
            ("31 SIGSYS", 31),
        ];

        #[cfg(target_os = "macos")]
        let base_signals: &[(&str, i32)] = &[
            (" 0 Cancel", 0),
            (" 1 SIGHUP", 1),
            (" 2 SIGINT", 2),
            (" 3 SIGQUIT", 3),
            (" 4 SIGILL", 4),
            (" 5 SIGTRAP", 5),
            (" 6 SIGABRT", 6),
            (" 6 SIGIOT", 6),
            (" 7 SIGEMT", 7),
            (" 8 SIGFPE", 8),
            (" 9 SIGKILL", 9),
            ("10 SIGBUS", 10),
            ("11 SIGSEGV", 11),
            ("12 SIGSYS", 12),
            ("13 SIGPIPE", 13),
            ("14 SIGALRM", 14),
            ("15 SIGTERM", 15),
            ("16 SIGURG", 16),
            ("17 SIGSTOP", 17),
            ("18 SIGTSTP", 18),
            ("19 SIGCONT", 19),
            ("20 SIGCHLD", 20),
            ("21 SIGTTIN", 21),
            ("22 SIGTTOU", 22),
            ("23 SIGIO", 23),
            ("24 SIGXCPU", 24),
            ("25 SIGXFSZ", 25),
            ("26 SIGVTALRM", 26),
            ("27 SIGPROF", 27),
            ("28 SIGWINCH", 28),
            ("29 SIGINFO", 29),
            ("30 SIGUSR1", 30),
            ("31 SIGUSR2", 31),
        ];

        // Build signal list with optional real-time signals (Linux only)
        #[allow(unused_mut)]
        let mut signals: Vec<(String, i32)> = base_signals
            .iter()
            .map(|(name, num)| (name.to_string(), *num))
            .collect();

        // Add real-time signals on Linux (SIGRTMIN to SIGRTMAX)
        // These are determined at runtime, typically 34-64 on Linux
        #[cfg(target_os = "linux")]
        {
            // SIGRTMIN and SIGRTMAX are functions in glibc, not constants
            // libc::SIGRTMIN() returns the minimum real-time signal number
            // libc::SIGRTMAX() returns the maximum real-time signal number
            let rtmin = libc::SIGRTMIN();
            let rtmax = libc::SIGRTMAX();

            // Safety check: only add if range is reasonable (C htop checks <= 100)
            if rtmax - rtmin <= 100 {
                for sig in rtmin..=rtmax {
                    let n = sig - rtmin;
                    let name = if n == 0 {
                        format!("{:2} SIGRTMIN", sig)
                    } else {
                        format!("{:2} SIGRTMIN+{}", sig, n)
                    };
                    signals.push((name, sig));
                }
            }
        }

        // Create signal panel (matches C htop SignalsPanel_new)
        // C htop uses width 14 in Action_pickFromVector
        // We use 15 to accommodate "64 SIGRTMIN+30" (14 chars) plus padding
        let signal_panel_width = 15i32;
        let panel_y = self.main_panel.y;
        let panel_height = crt.height() - panel_y - 1; // Leave room for function bar

        let mut signal_panel = Panel::new(0, panel_y, signal_panel_width, panel_height);
        signal_panel.set_header("Send signal:");
        signal_panel.function_bar = FunctionBar::new_enter_esc("Send   ", "Cancel ");

        // Add all signals and find SIGTERM (15) for default selection
        let mut default_position = 0i32;
        for (i, (name, number)) in signals.iter().enumerate() {
            signal_panel.add_list_item(name, *number);
            // Signal 15 (SIGTERM) is the default, but it's not always at index 15
            if *number == 15 {
                default_position = i as i32;
            }
        }
        signal_panel.set_selected(default_position);

        // Save original main panel position
        let orig_main_x = self.main_panel.x;
        let orig_main_w = self.main_panel.w;

        // Resize main panel to make room for signal panel on left
        self.main_panel.move_to(signal_panel_width, panel_y);
        self.main_panel
            .resize(crt.width() - signal_panel_width, panel_height);

        // Event loop
        let mut selected_signal: Option<i32> = None;

        loop {
            // Draw header meters
            if !self.hide_meters {
                self.header.draw(crt, machine, &self.settings);
            }

            // Draw signal panel on the left (with focus)
            signal_panel.draw(crt, true, true);

            // Draw main panel on the right (no focus)
            self.main_panel.draw(crt, machine, &self.settings);

            // Draw the Enter/Esc function bar
            let fb_y = crt.height() - 1;
            signal_panel.function_bar.draw_simple(crt, fb_y);

            crt.refresh();

            // Handle input
            let mut key = getch();

            // Convert mouse events to wheel keys
            if key == KEY_MOUSE {
                if let Some(wheel_key) = Crt::convert_mouse_to_key() {
                    key = wheel_key;
                }
            }

            match key {
                KEY_UP => signal_panel.move_up(1),
                KEY_DOWN => signal_panel.move_down(1),
                KEY_PPAGE => signal_panel.page_up(),
                KEY_NPAGE => signal_panel.page_down(),
                KEY_HOME => signal_panel.move_home(),
                KEY_END => signal_panel.move_end(),
                KEY_WHEELUP => signal_panel.scroll_wheel(-10),
                KEY_WHEELDOWN => signal_panel.scroll_wheel(10),
                0x0A | 0x0D => {
                    // Enter - send signal and exit
                    let selected_idx = signal_panel.get_selected() as usize;
                    if selected_idx < signals.len() {
                        let (_, sig_num) = signals[selected_idx];
                        // Signal 0 means "Cancel" in C htop
                        if sig_num != 0 {
                            selected_signal = Some(sig_num);
                        }
                    }
                    break;
                }
                0x1B | KEY_F10 => {
                    // Escape or F10 - cancel
                    break;
                }
                _ => {
                    // Try typing search (jump to signal starting with this char)
                    if key >= 0x20 && key < 0x7F {
                        signal_panel.select_by_typing(key as u8 as char);
                    }
                }
            }
        }

        // Restore main panel position
        self.main_panel.move_to(orig_main_x, panel_y);
        self.main_panel.resize(orig_main_w, panel_height);

        // Send the signal if one was selected
        if let Some(signal) = selected_signal {
            Self::send_signal(pid, signal);
        }
    }

    /// Send signal to process
    fn send_signal(pid: i32, signal: i32) {
        #[cfg(unix)]
        unsafe {
            libc::kill(pid, signal);
        }
    }

    /// Show sort column selection menu (matches C htop actionSetSortColumn)
    fn show_sort_menu(&mut self, crt: &Crt, machine: &mut Machine) {
        // Determine the active sort key - in tree view, it's always PID
        let active_sort_key = if self.settings.tree_view {
            ProcessField::Pid
        } else {
            machine.sort_key
        };

        // Create the sort panel (matches C htop Panel_new with FunctionBar_newEnterEsc)
        // C htop uses width 14 in Action_pickFromVector
        let sort_panel_width = 14i32;
        let panel_y = self.main_panel.y;
        let panel_height = crt.height() - panel_y - 1; // Leave room for function bar

        let mut sort_panel = Panel::new(0, panel_y, sort_panel_width, panel_height);
        sort_panel.set_header("Sort by");
        sort_panel.function_bar = FunctionBar::new_enter_esc("Sort   ", "Cancel ");

        // Add fields from the currently displayed columns (like C htop)
        // C htop uses settings->ss->fields, we use self.main_panel.fields
        let mut current_selection = 0i32;
        for (i, field) in self.main_panel.fields.iter().enumerate() {
            // Get the field name (trimmed, like C htop String_trim)
            let name = field.name().unwrap_or("?").trim();
            sort_panel.add_list_item(name, *field as i32);

            // Pre-select the current sort key
            if *field == active_sort_key {
                current_selection = i as i32;
            }
        }
        sort_panel.set_selected(current_selection);

        // Save original main panel position
        let orig_main_x = self.main_panel.x;
        let orig_main_w = self.main_panel.w;

        // Resize main panel to make room for sort panel
        self.main_panel.move_to(sort_panel_width, panel_y);
        self.main_panel
            .resize(crt.width() - sort_panel_width, panel_height);

        // Event loop
        let mut selected_field: Option<ProcessField> = None;

        loop {
            // Draw header meters
            if !self.hide_meters {
                self.header.draw(crt, machine, &self.settings);
            }

            // Draw sort panel on the left (with focus)
            sort_panel.draw(crt, true, true);

            // Draw main panel on the right (no focus)
            self.main_panel.draw(crt, machine, &self.settings);

            // Draw the Enter/Esc function bar
            let fb_y = crt.height() - 1;
            sort_panel.function_bar.draw_simple(crt, fb_y);

            crt.refresh();

            // Handle input
            let mut key = getch();

            // Convert mouse events to wheel keys
            if key == KEY_MOUSE {
                if let Some(wheel_key) = Crt::convert_mouse_to_key() {
                    key = wheel_key;
                }
            }

            match key {
                KEY_UP => sort_panel.move_up(1),
                KEY_DOWN => sort_panel.move_down(1),
                KEY_PPAGE => sort_panel.page_up(),
                KEY_NPAGE => sort_panel.page_down(),
                KEY_HOME => sort_panel.move_home(),
                KEY_END => sort_panel.move_end(),
                KEY_WHEELUP => sort_panel.scroll_wheel(-10),
                KEY_WHEELDOWN => sort_panel.scroll_wheel(10),
                0x0A | 0x0D => {
                    // Enter - select and exit
                    let selected_idx = sort_panel.get_selected() as usize;
                    if selected_idx < self.main_panel.fields.len() {
                        selected_field = Some(self.main_panel.fields[selected_idx]);
                    }
                    break;
                }
                0x1B | KEY_F10 => {
                    // Escape or F10 - cancel
                    break;
                }
                _ => {
                    // Try typing search (jump to item starting with this char)
                    if key >= 0x20 && key < 0x7F {
                        sort_panel.select_by_typing(key as u8 as char);
                    }
                }
            }
        }

        // Restore main panel position
        self.main_panel.move_to(orig_main_x, panel_y);
        self.main_panel.resize(orig_main_w, panel_height);

        // Apply the selection
        if let Some(field) = selected_field {
            // Match C htop ScreenSettings_setSortKey behavior:
            // Setting sort key in tree view exits tree view
            if self.settings.tree_view {
                self.settings.tree_view = false;
                self.main_panel.tree_view = false;
            }
            machine.sort_key = field;
            machine.sort_descending = field.default_sort_desc();
            self.settings.changed = true;
        }
    }

    /// Show help screen (matches C htop actionHelp)
    #[allow(unused_must_use)]
    fn show_help(&self, crt: &Crt) {
        crt.clear();

        let default_color = crt.color(ColorElement::DefaultColor);
        let bold = crt.color(ColorElement::HelpBold);
        let bar_border = crt.color(ColorElement::BarBorder);
        let bar_shadow = crt.color(ColorElement::BarShadow);

        // Fill screen with HELP_BOLD background (like C htop)
        attrset(bold);
        for i in 0..crt.height() - 1 {
            mv(i, 0);
            for _ in 0..crt.width() {
                addch(' ' as u32);
            }
        }

        let mut line = 0;

        // Title
        attrset(bold);
        mvaddstr(
            line,
            0,
            &format!(
                "htop-rs {} - Interactive process viewer",
                env!("CARGO_PKG_VERSION")
            ),
        );
        line += 1;
        mvaddstr(line, 0, "Released under the MIT License.");
        line += 2;

        // CPU usage bar legend (non-detailed mode)
        // Content: low/normal/kernel/guest + spaces + used% = 56 chars total
        // low(3) + /(1) + normal(6) + /(1) + kernel(6) + /(1) + guest(5) = 23
        // Need 28 spaces + used%(5) = 33 more chars to reach 56
        attrset(default_color);
        mvaddstr(line, 0, "CPU usage bar: ");
        attrset(bar_border);
        addstr("[");
        attrset(crt.color(ColorElement::CpuNice));
        addstr("low");
        attrset(default_color);
        addstr("/");
        attrset(crt.color(ColorElement::CpuNormal));
        addstr("normal");
        attrset(default_color);
        addstr("/");
        attrset(crt.color(ColorElement::CpuSystem));
        addstr("kernel");
        attrset(default_color);
        addstr("/");
        attrset(crt.color(ColorElement::CpuGuest));
        addstr("guest");
        attrset(bar_shadow);
        addstr("                            used%"); // 28 spaces + used%
        attrset(bar_border);
        addstr("]");
        line += 1;

        // Memory bar legend
        // Content: used/shared/compressed/buffers/cache + spaces + used/total = 56 chars
        // used(4) + /(1) + shared(6) + /(1) + compressed(10) + /(1) + buffers(7) + /(1) + cache(5) = 36
        // Need 10 spaces + used(4) + /(1) + total(5) = 20 more chars to reach 56
        attrset(default_color);
        mvaddstr(line, 0, "Memory bar:    ");
        attrset(bar_border);
        addstr("[");
        attrset(crt.color(ColorElement::MemoryUsed));
        addstr("used");
        attrset(default_color);
        addstr("/");
        attrset(crt.color(ColorElement::MemoryShared));
        addstr("shared");
        attrset(default_color);
        addstr("/");
        attrset(crt.color(ColorElement::MemoryCompressed));
        addstr("compressed");
        attrset(default_color);
        addstr("/");
        attrset(crt.color(ColorElement::MemoryBuffersText));
        addstr("buffers");
        attrset(default_color);
        addstr("/");
        attrset(crt.color(ColorElement::MemoryCache));
        addstr("cache");
        attrset(bar_shadow);
        addstr("          used"); // 10 spaces + used
        attrset(default_color);
        addstr("/");
        attrset(bar_shadow);
        addstr("total");
        attrset(bar_border);
        addstr("]");
        line += 1;

        // Swap bar legend (non-Linux: no cache/frontswap)
        // Content: used + spaces + used/total = 56 chars
        // used(4) + 42 spaces + used(4) + /(1) + total(5) = 56
        attrset(default_color);
        mvaddstr(line, 0, "Swap bar:      ");
        attrset(bar_border);
        addstr("[");
        attrset(crt.color(ColorElement::Swap));
        addstr("used");
        attrset(bar_shadow);
        addstr("                                          used"); // 42 spaces + used
        attrset(default_color);
        addstr("/");
        attrset(bar_shadow);
        addstr("total");
        attrset(bar_border);
        addstr("]");
        line += 2;

        // Info about meter configuration
        attrset(default_color);
        mvaddstr(
            line,
            0,
            "Type and layout of header meters are configurable in the setup screen.",
        );
        line += 2;

        // Process state legend
        attrset(default_color);
        mvaddstr(line, 0, "Process state: ");
        attrset(crt.color(ColorElement::ProcessRunState));
        addstr("R");
        attrset(default_color);
        addstr(": running; ");
        attrset(crt.color(ColorElement::ProcessShadow));
        addstr("S");
        attrset(default_color);
        addstr(": sleeping; ");
        attrset(crt.color(ColorElement::ProcessRunState));
        addstr("t");
        attrset(default_color);
        addstr(": traced/stopped; ");
        attrset(crt.color(ColorElement::ProcessDState));
        addstr("Z");
        attrset(default_color);
        addstr(": zombie; ");
        attrset(crt.color(ColorElement::ProcessDState));
        addstr("D");
        attrset(default_color);
        addstr(": disk sleep");
        line += 2;

        // Two-column key bindings (matching C htop helpLeft/helpRight)
        let readonly = self.settings.readonly;
        let shadow = crt.color(ColorElement::HelpShadow);

        // Left column items: key at col 1, info at col 10
        let help_left = [
            ("      #: ", "hide/show header meters", false),
            ("    Tab: ", "switch to next screen tab", false),
            (" Arrows: ", "scroll process list", false),
            (" Digits: ", "incremental PID search", false),
            ("   F3 /: ", "incremental name search", false),
            ("   F4 \\: ", "incremental name filtering", false),
            ("   F5 t: ", "tree view", false),
            ("      p: ", "toggle program path", false),
            ("      Z: ", "pause/resume process updates", false),
            ("      u: ", "show processes of a single user", false),
            ("      H: ", "hide/show user process threads", false),
            ("      K: ", "hide/show kernel threads", false),
            ("      F: ", "cursor follows process", false),
            ("  + - *: ", "expand/collapse tree/toggle all", false),
            ("N P M T: ", "sort by PID, CPU%, MEM% or TIME", false),
            ("      I: ", "invert sort order", false),
            (" F6 > .: ", "select sort column", false),
        ];

        // Right column items: key at col 43, info at col 52
        let help_right = [
            ("  S-Tab: ", "switch to previous screen tab", false),
            ("  Space: ", "tag process", false),
            ("      c: ", "tag process and its children", false),
            ("      U: ", "untag all processes", false),
            ("   F9 k: ", "kill process/tagged processes", true),
            ("   F7 ]: ", "higher priority (root only)", true),
            ("   F8 [: ", "lower priority (+ nice)", true),
            ("      e: ", "show process environment", false),
            ("      l: ", "list open files with lsof", true),
            ("      s: ", "trace syscalls with strace", true),
            ("      w: ", "wrap process command", false),
            (" F2 C S: ", "setup", false),
            (" F1 h ?: ", "show this help screen", false),
            ("  F10 q: ", "quit", false),
        ];

        let start_line = line;

        // Draw left column
        for (i, (key, info, ro_inactive)) in help_left.iter().enumerate() {
            let inactive = *ro_inactive && readonly;
            let key_attr = if inactive { shadow } else { bold };
            let info_attr = if inactive { shadow } else { default_color };

            attrset(key_attr);
            mvaddstr(start_line + i as i32, 1, key);
            attrset(info_attr);
            mvaddstr(start_line + i as i32, 10, info);

            // Special coloring for "threads" keyword (matching C htop)
            let thread_color = if inactive {
                shadow
            } else {
                crt.color(ColorElement::ProcessThread)
            };
            if *key == "      H: " {
                // "hide/show user process threads" - "threads" at column 33
                attrset(thread_color);
                mvaddstr(start_line + i as i32, 33, "threads");
            } else if *key == "      K: " {
                // "hide/show kernel threads" - "threads" at column 27
                attrset(thread_color);
                mvaddstr(start_line + i as i32, 27, "threads");
            }
        }

        // Draw right column
        for (i, (key, info, ro_inactive)) in help_right.iter().enumerate() {
            let inactive = *ro_inactive && readonly;
            let key_attr = if inactive { shadow } else { bold };
            let info_attr = if inactive { shadow } else { default_color };

            attrset(key_attr);
            mvaddstr(start_line + i as i32, 43, key);
            attrset(info_attr);
            mvaddstr(start_line + i as i32, 52, info);
        }

        line = start_line + help_left.len().max(help_right.len()) as i32 + 1;

        // "Press any key to return"
        attrset(bold);
        mvaddstr(line, 0, "Press any key to return.");
        attrset(default_color);

        crt.refresh();

        // Wait for key - disable timeout so we block until key press
        // (matches C htop CRT_readKey behavior)
        nodelay(stdscr(), false);
        getch();
        // Re-enable delay for main loop
        crt.enable_delay();
    }

    /// Show setup screen
    fn show_setup(&mut self, crt: &mut Crt, machine: &Machine) {
        let mut setup_screen = super::setup_screen::SetupScreen::new();
        setup_screen.run(&mut self.settings, crt, &mut self.header, machine);
    }

    /// Show process environment (like C htop actionShowEnv)
    fn show_process_env(&self, crt: &Crt, pid: i32) {
        crt.clear();

        let bold = crt.color(ColorElement::HelpBold);
        let default_attr = crt.color(ColorElement::DefaultColor);

        mv(0, 0);
        attrset(bold);
        let _ = addstr(&format!("Environment for PID {}", pid));
        attrset(A_NORMAL);

        // Read environment from /proc on Linux, or use ps on macOS
        #[cfg(target_os = "macos")]
        let env_output = {
            use std::process::Command;
            Command::new("ps")
                .args(["-p", &pid.to_string(), "-E", "-o", "command="])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
        };

        #[cfg(target_os = "linux")]
        let env_output = {
            use std::fs;
            fs::read_to_string(format!("/proc/{}/environ", pid))
                .ok()
                .map(|s| s.replace('\0', "\n"))
        };

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let env_output: Option<String> = None;

        mv(2, 0);
        attrset(default_attr);
        if let Some(env) = env_output {
            for (i, line) in env.lines().take((crt.height() - 4) as usize).enumerate() {
                mv(2 + i as i32, 0);
                let _ = addstr(line);
            }
        } else {
            let _ = addstr("Unable to read process environment.");
        }
        attrset(A_NORMAL);

        mv(crt.height() - 1, 0);
        attrset(bold);
        let _ = addstr("Press any key to return.");
        attrset(A_NORMAL);

        crt.refresh();
        nodelay(stdscr(), false);
        getch();
        crt.enable_delay();
    }

    /// Show lsof output for process (like C htop OpenFilesScreen)
    fn show_lsof(&self, crt: &Crt, pid: i32, command: &str) {
        // Parse lsof output using -F (machine-readable format)
        let lsof_data = Self::run_lsof(pid);

        // Create function bar: Search, Filter, Refresh, Done
        let _function_bar = FunctionBar::with_functions(vec![
            ("F3".to_string(), "Search ".to_string()),
            ("F4".to_string(), "Filter ".to_string()),
            ("F5".to_string(), "Refresh".to_string()),
            ("Esc".to_string(), "Done   ".to_string()),
        ]);

        // Build lines from parsed data
        let mut lines: Vec<String> = Vec::new();
        let mut col_widths = [5usize, 7, 4, 6, 8, 8, 8]; // FD, TYPE, MODE, DEVICE, SIZE, OFFSET, NODE

        match &lsof_data {
            Ok(files) => {
                // Calculate dynamic column widths
                for file in files {
                    col_widths[4] = col_widths[4].max(file.size.len());
                    col_widths[5] = col_widths[5].max(file.offset.len());
                    col_widths[6] = col_widths[6].max(file.inode.len());
                }

                // Build formatted lines
                for file in files {
                    let line = format!(
                        "{:>5} {:7} {:4} {:>6} {:>width_s$} {:>width_o$} {:>width_i$}  {}",
                        file.fd,
                        file.file_type,
                        file.mode,
                        file.device,
                        file.size,
                        file.offset,
                        file.inode,
                        file.name,
                        width_s = col_widths[4],
                        width_o = col_widths[5],
                        width_i = col_widths[6],
                    );
                    lines.push(line);
                }
            }
            Err(msg) => {
                lines.push(msg.clone());
            }
        }

        // Build header with dynamic column widths
        let header_str = format!(
            "{:>5} {:7} {:4} {:>6} {:>width_s$} {:>width_o$} {:>width_i$}  {}",
            "FD",
            "TYPE",
            "MODE",
            "DEVICE",
            "SIZE",
            "OFFSET",
            "NODE",
            "NAME",
            width_s = col_widths[4],
            width_o = col_widths[5],
            width_i = col_widths[6],
        );

        // State for the info screen
        let mut selected = 0i32;
        let mut scroll_v = 0i32;
        let mut filter_text = String::new();
        let mut search_text = String::new();
        let mut filter_active = false;
        let mut search_active = false;

        // Get filtered lines
        let get_filtered_lines = |lines: &[String], filter: &str| -> Vec<usize> {
            if filter.is_empty() {
                (0..lines.len()).collect()
            } else {
                let filter_lower = filter.to_lowercase();
                lines
                    .iter()
                    .enumerate()
                    .filter(|(_, line)| line.to_lowercase().contains(&filter_lower))
                    .map(|(i, _)| i)
                    .collect()
            }
        };

        loop {
            let filtered_indices = get_filtered_lines(&lines, &filter_text);
            let panel_height = crt.height() - 3; // Title + header + function bar
            let panel_y = 2; // After title and header

            // Clamp selection and scroll
            let max_selected = (filtered_indices.len() as i32 - 1).max(0);
            selected = selected.clamp(0, max_selected);

            if selected < scroll_v {
                scroll_v = selected;
            } else if selected >= scroll_v + panel_height {
                scroll_v = selected - panel_height + 1;
            }
            scroll_v = scroll_v.max(0);

            // Draw title
            let title_attr = crt.color(ColorElement::MeterText);
            mv(0, 0);
            attrset(title_attr);
            let title = format!("Snapshot of files open in process {} - {}", pid, command);
            let title_display: String = title.chars().take(crt.width() as usize).collect();
            hline(' ' as u32, crt.width());
            let _ = addstr(&title_display);
            attrset(A_NORMAL);

            // Draw header
            let header_attr = crt.color(ColorElement::PanelHeaderFocus);
            mv(1, 0);
            attrset(header_attr);
            hline(' ' as u32, crt.width());
            let header_display: String = header_str.chars().take(crt.width() as usize).collect();
            let _ = addstr(&header_display);
            attrset(A_NORMAL);

            // Draw lines
            let default_attr = crt.color(ColorElement::DefaultColor);
            let selection_attr = crt.color(ColorElement::PanelSelectionFocus);

            for row in 0..panel_height {
                let y = panel_y + row;
                let line_idx = (scroll_v + row) as usize;

                mv(y, 0);

                if line_idx < filtered_indices.len() {
                    let actual_idx = filtered_indices[line_idx];
                    let line = &lines[actual_idx];
                    let is_selected = (scroll_v + row) == selected;

                    let attr = if is_selected {
                        selection_attr
                    } else {
                        default_attr
                    };
                    attrset(attr);
                    hline(' ' as u32, crt.width());
                    let display_line: String = line.chars().take(crt.width() as usize).collect();
                    let _ = addstr(&display_line);
                    attrset(A_NORMAL);
                } else {
                    attrset(default_attr);
                    hline(' ' as u32, crt.width());
                    attrset(A_NORMAL);
                }
            }

            // Draw function bar or search/filter bar
            let fb_y = crt.height() - 1;
            mv(fb_y, 0);

            if search_active || filter_active {
                let bar_attr = crt.color(ColorElement::FunctionBar);
                let key_attr = crt.color(ColorElement::FunctionKey);
                attrset(bar_attr);
                hline(' ' as u32, crt.width());
                attrset(A_NORMAL);
                mv(fb_y, 0);

                if search_active {
                    attrset(key_attr);
                    let _ = addstr("Search: ");
                    attrset(A_NORMAL);
                    attrset(bar_attr);
                    let _ = addstr(&search_text);
                    attrset(A_NORMAL);
                } else {
                    attrset(key_attr);
                    let _ = addstr("Filter: ");
                    attrset(A_NORMAL);
                    attrset(bar_attr);
                    let _ = addstr(&filter_text);
                    attrset(A_NORMAL);
                }
            } else {
                // Update F4 label based on filter state
                let f4_label = if filter_text.is_empty() {
                    "Filter "
                } else {
                    "FILTER "
                };
                let fb = FunctionBar::with_functions(vec![
                    ("F3".to_string(), "Search ".to_string()),
                    ("F4".to_string(), f4_label.to_string()),
                    ("F5".to_string(), "Refresh".to_string()),
                    ("Esc".to_string(), "Done   ".to_string()),
                ]);
                fb.draw_simple(crt, fb_y);
            }

            crt.refresh();

            // Handle input
            nodelay(stdscr(), false);
            let ch = getch();

            if search_active || filter_active {
                match ch {
                    27 => {
                        // Escape - cancel search/filter
                        if search_active {
                            search_text.clear();
                        }
                        search_active = false;
                        filter_active = false;
                    }
                    10 | KEY_ENTER => {
                        // Enter - confirm and exit mode
                        search_active = false;
                        filter_active = false;
                    }
                    KEY_BACKSPACE | 127 | 8 => {
                        if search_active && !search_text.is_empty() {
                            search_text.pop();
                        } else if filter_active && !filter_text.is_empty() {
                            filter_text.pop();
                        }
                    }
                    _ if ch >= 32 && ch < 127 => {
                        // Printable character
                        let c = char::from_u32(ch as u32).unwrap_or(' ');
                        if search_active {
                            search_text.push(c);
                            // Incremental search - find next match
                            let search_lower = search_text.to_lowercase();
                            for (i, idx) in filtered_indices.iter().enumerate() {
                                if lines[*idx].to_lowercase().contains(&search_lower) {
                                    selected = i as i32;
                                    break;
                                }
                            }
                        } else if filter_active {
                            filter_text.push(c);
                            selected = 0;
                            scroll_v = 0;
                        }
                    }
                    _ => {}
                }
                continue;
            }

            match ch {
                27 => {
                    // Escape - exit
                    break;
                }
                x if x == KEY_F10 => {
                    // F10 - exit
                    break;
                }
                113 => {
                    // 'q' - exit
                    break;
                }
                x if x == KEY_F3 || x == 47 => {
                    // F3 or '/' - search
                    search_active = true;
                    search_text.clear();
                }
                x if x == KEY_F4 || x == 92 => {
                    // F4 or '\' - filter
                    filter_active = true;
                }
                x if x == KEY_F5 => {
                    // F5 - refresh
                    let new_data = Self::run_lsof(pid);
                    lines.clear();
                    match new_data {
                        Ok(files) => {
                            // Recalculate column widths
                            col_widths = [5, 7, 4, 6, 8, 8, 8];
                            for file in &files {
                                col_widths[4] = col_widths[4].max(file.size.len());
                                col_widths[5] = col_widths[5].max(file.offset.len());
                                col_widths[6] = col_widths[6].max(file.inode.len());
                            }
                            for file in files {
                                let line = format!(
                                    "{:>5} {:7} {:4} {:>6} {:>width_s$} {:>width_o$} {:>width_i$}  {}",
                                    file.fd,
                                    file.file_type,
                                    file.mode,
                                    file.device,
                                    file.size,
                                    file.offset,
                                    file.inode,
                                    file.name,
                                    width_s = col_widths[4],
                                    width_o = col_widths[5],
                                    width_i = col_widths[6],
                                );
                                lines.push(line);
                            }
                        }
                        Err(msg) => {
                            lines.push(msg);
                        }
                    }
                    selected = 0;
                    scroll_v = 0;
                    crt.clear();
                }
                12 => {
                    // Ctrl+L - redraw
                    crt.clear();
                }
                KEY_UP => {
                    selected = (selected - 1).max(0);
                }
                KEY_DOWN => {
                    selected = (selected + 1).min(max_selected);
                }
                KEY_PPAGE => {
                    selected = (selected - panel_height).max(0);
                }
                KEY_NPAGE => {
                    selected = (selected + panel_height).min(max_selected);
                }
                KEY_HOME => {
                    selected = 0;
                }
                KEY_END => {
                    selected = max_selected;
                }
                KEY_WHEELUP => {
                    selected = (selected - 3).max(0);
                }
                KEY_WHEELDOWN => {
                    selected = (selected + 3).min(max_selected);
                }
                _ => {}
            }
        }

        crt.enable_delay();
    }

    /// Run lsof and parse output into structured data
    fn run_lsof(pid: i32) -> Result<Vec<LsofFileEntry>, String> {
        use std::process::Command;

        // Run lsof with -F flag for machine-readable output
        // -P: inhibit conversion of port numbers to port names
        // -o: always print file offset
        // -F: produce output suitable for processing
        let output = Command::new("lsof")
            .args(["-P", "-o", "-p", &pid.to_string(), "-F"])
            .output();

        let output = match output {
            Ok(o) => o,
            Err(_) => {
                return Err(
                    "Could not execute 'lsof'. Please make sure it is available in your $PATH."
                        .to_string(),
                )
            }
        };

        if !output.status.success() {
            let code = output.status.code().unwrap_or(1);
            if code == 127 {
                return Err(
                    "Could not execute 'lsof'. Please make sure it is available in your $PATH."
                        .to_string(),
                );
            }
            return Err("Failed listing open files.".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse lsof -F output format
        // Fields are prefixed with a single character:
        // f = file descriptor
        // a = access mode (r/w/u)
        // t = type
        // D = device
        // s = size
        // o = offset
        // i = inode
        // n = name

        let mut files: Vec<LsofFileEntry> = Vec::new();
        let mut current_file: Option<LsofFileEntry> = None;
        let mut has_size_field = false;

        for line in stdout.lines() {
            if line.is_empty() {
                continue;
            }

            let cmd = line.chars().next().unwrap_or(' ');
            let value = &line[1..];

            match cmd {
                'f' => {
                    // New file entry - save previous if exists
                    if let Some(file) = current_file.take() {
                        files.push(file);
                    }
                    current_file = Some(LsofFileEntry {
                        fd: value.to_string(),
                        ..Default::default()
                    });
                }
                'a' => {
                    if let Some(ref mut file) = current_file {
                        file.mode = value.to_string();
                    }
                }
                't' => {
                    if let Some(ref mut file) = current_file {
                        file.file_type = value.to_string();
                    }
                }
                'D' => {
                    if let Some(ref mut file) = current_file {
                        file.device = value.to_string();
                    }
                }
                's' => {
                    if let Some(ref mut file) = current_file {
                        file.size = value.to_string();
                        has_size_field = true;
                    }
                }
                'o' => {
                    if let Some(ref mut file) = current_file {
                        // Remove "0t" prefix if present
                        let offset = if value.starts_with("0t") {
                            &value[2..]
                        } else {
                            value
                        };
                        file.offset = offset.to_string();
                    }
                }
                'i' => {
                    if let Some(ref mut file) = current_file {
                        file.inode = value.to_string();
                    }
                }
                'n' => {
                    if let Some(ref mut file) = current_file {
                        file.name = value.to_string();
                    }
                }
                // Ignore other fields (p, c, u, g, R, etc.)
                _ => {}
            }
        }

        // Save last file
        if let Some(file) = current_file {
            files.push(file);
        }

        // On Linux, lsof -o -F omits SIZE, so get it from stat() if needed
        #[cfg(target_os = "linux")]
        if !has_size_field {
            for file in &mut files {
                if file.size.is_empty() {
                    if let Ok(metadata) = std::fs::metadata(&file.name) {
                        file.size = metadata.len().to_string();
                    }
                }
            }
        }

        // Suppress unused variable warning on non-Linux
        #[cfg(not(target_os = "linux"))]
        let _ = has_size_field;

        if files.is_empty() {
            return Err("No open files found.".to_string());
        }

        Ok(files)
    }

    /// Show strace/dtruss output for process (like C htop actionStrace)
    fn show_strace(&self, crt: &Crt, pid: i32) {
        crt.clear();

        let bold = crt.color(ColorElement::HelpBold);
        let default_attr = crt.color(ColorElement::DefaultColor);

        mv(0, 0);
        attrset(bold);
        #[cfg(target_os = "macos")]
        let _ = addstr(&format!(
            "System calls for PID {} (dtruss - requires sudo)",
            pid
        ));
        #[cfg(not(target_os = "macos"))]
        let _ = addstr(&format!("System calls for PID {} (strace)", pid));
        attrset(A_NORMAL);

        mv(2, 0);
        attrset(default_attr);
        #[cfg(target_os = "macos")]
        let _ = addstr("Run manually: sudo dtruss -p ");
        #[cfg(target_os = "linux")]
        let _ = addstr("Run manually: strace -p ");
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let _ = addstr("Syscall tracing not supported on this platform.");

        let _ = addstr(&pid.to_string());

        mv(4, 0);
        let _ = addstr("(Interactive tracing requires running htop with elevated privileges)");
        attrset(A_NORMAL);

        mv(crt.height() - 1, 0);
        attrset(bold);
        let _ = addstr("Press any key to return.");
        attrset(A_NORMAL);

        crt.refresh();
        nodelay(stdscr(), false);
        getch();
        crt.enable_delay();
    }

    /// Show user selection menu (like C htop actionFilterByUser)
    /// Displays a panel on the left side with the main process list on the right
    fn show_user_menu(&mut self, crt: &Crt, machine: &mut Machine) {
        // Collect unique users from processes
        let mut users: Vec<(u32, String)> = machine
            .processes
            .iter()
            .filter_map(|p| p.user.as_ref().map(|u| (p.uid, u.clone())))
            .collect();
        users.sort_by(|a, b| a.1.cmp(&b.1));
        users.dedup_by(|a, b| a.0 == b.0);

        // Add "All users" at the top
        let mut menu_items: Vec<(Option<u32>, String)> = vec![(None, "All users".to_string())];
        menu_items.extend(users.into_iter().map(|(uid, name)| (Some(uid), name)));

        // Panel styling colors (matching C htop)
        let panel_header_attr = crt.color(ColorElement::PanelHeaderFocus);
        let header_unfocus_attr = crt.color(ColorElement::PanelHeaderUnfocus);
        let selection_attr = crt.color(ColorElement::PanelSelectionFocus);
        let default_attr = crt.color(ColorElement::DefaultColor);
        let func_bar_attr = crt.color(ColorElement::FunctionBar);
        let func_key_attr = crt.color(ColorElement::FunctionKey);
        let reset_attr = crt.color(ColorElement::ResetColor);

        // Calculate header height (meters)
        let meters_height = if self.hide_meters {
            0
        } else {
            self.header.calculate_height()
        };

        // Screen tabs take 1 line if enabled
        let screen_tabs_height = if self.settings.screen_tabs { 1 } else { 0 };

        // Panel starts after meters and screen tabs
        let panel_start_y = meters_height + screen_tabs_height;

        // User panel dimensions (matches C htop's x=19 for user panel)
        let user_panel_width = 19i32;
        let user_panel_x = 0i32;
        let user_panel_header_text = "Show processes of:";

        // Save main panel's original position and resize it for side-by-side display
        let orig_main_x = self.main_panel.x;
        let orig_main_w = self.main_panel.w;
        let orig_main_y = self.main_panel.y;
        let orig_main_h = self.main_panel.h;
        let orig_main_scroll_v = self.main_panel.scroll_v;

        // Main panel starts after user panel
        self.main_panel.x = user_panel_width;
        self.main_panel.w = crt.width() - user_panel_width;

        let mut selected = 0usize;
        // Panel height: from panel_start_y to function bar (leave 1 row for function bar)
        // Subtract 1 more for the panel header row
        let panel_content_height = (crt.height() - panel_start_y - 1 - 1) as usize;
        let mut scroll = 0usize;

        loop {
            // Clear screen
            crt.clear();

            // === Draw meters at the top ===
            if !self.hide_meters {
                self.header.draw(crt, machine, &self.settings);
            }

            // === Draw screen tabs if enabled ===
            if self.settings.screen_tabs {
                self.draw_screen_tabs(crt);
            }

            // === Draw user selection panel on the left ===

            // Draw user panel header
            mv(panel_start_y, user_panel_x);
            attrset(panel_header_attr);
            // Fill header row with spaces, then draw header text
            for _ in 0..user_panel_width {
                let _ = addch(' ' as u32);
            }
            mv(panel_start_y, user_panel_x);
            let _ = addstr(user_panel_header_text);
            attrset(reset_attr);

            // Draw user menu items starting after panel header
            for i in 0..panel_content_height {
                let item_idx = scroll + i;
                let row_y = panel_start_y + 1 + i as i32;
                mv(row_y, user_panel_x);

                if item_idx < menu_items.len() {
                    let (_, ref name) = menu_items[item_idx];
                    let is_selected = item_idx == selected;

                    if is_selected {
                        attrset(selection_attr);
                    } else {
                        attrset(default_attr);
                    }

                    // Fill line with spaces first
                    for _ in 0..user_panel_width {
                        let _ = addch(' ' as u32);
                    }
                    // Draw item name (truncate if needed)
                    mv(row_y, user_panel_x);
                    let display_name: String =
                        name.chars().take(user_panel_width as usize).collect();
                    let _ = addstr(&display_name);

                    attrset(reset_attr);
                } else {
                    // Empty row
                    attrset(default_attr);
                    for _ in 0..user_panel_width {
                        let _ = addch(' ' as u32);
                    }
                    attrset(reset_attr);
                }
            }

            // === Draw main process panel on the right (unfocused) ===

            // Draw main panel header
            mv(panel_start_y, user_panel_width);
            attrset(header_unfocus_attr);
            // Fill the rest of the header row
            for _ in user_panel_width..crt.width() {
                let _ = addch(' ' as u32);
            }
            // Draw column headers
            mv(panel_start_y, user_panel_width);
            let header_str = self.main_panel.build_header_string(
                &self.settings,
                machine.sort_key,
                machine.sort_descending,
            );
            let header_display: String = header_str
                .chars()
                .take((crt.width() - user_panel_width) as usize)
                .collect();
            let _ = addstr(&header_display);
            attrset(reset_attr);

            // Draw the main panel content (processes)
            // Temporarily disable header drawing since we drew it above
            let orig_show_header = self.main_panel.show_header;
            self.main_panel.show_header = false;
            self.main_panel.y = panel_start_y + 1; // Start after our manually drawn header
            self.main_panel.h = panel_content_height as i32;
            // Ensure selected row is visible with the smaller panel height
            self.main_panel
                .ensure_visible(machine.processes.len() as i32);
            self.main_panel.draw(crt, machine, &self.settings);
            self.main_panel.show_header = orig_show_header;

            // === Draw function bar at bottom ===
            mv(crt.height() - 1, 0);
            attrset(func_bar_attr);
            // Fill entire bottom row
            for _ in 0..crt.width() {
                let _ = addch(' ' as u32);
            }
            mv(crt.height() - 1, 0);
            // Draw "Enter" key
            attrset(func_key_attr);
            let _ = addstr("Enter");
            attrset(func_bar_attr);
            let _ = addstr("Show   ");
            // Draw "Esc" key
            attrset(func_key_attr);
            let _ = addstr("Esc");
            attrset(func_bar_attr);
            let _ = addstr("Cancel ");
            attrset(reset_attr);

            crt.refresh();

            let mut key = getch();

            // Convert mouse events to wheel keys
            if key == KEY_MOUSE {
                if let Some(wheel_key) = Crt::convert_mouse_to_key() {
                    key = wheel_key;
                }
            }

            match key {
                KEY_UP => {
                    if selected > 0 {
                        selected -= 1;
                        if selected < scroll {
                            scroll = selected;
                        }
                    }
                }
                KEY_DOWN => {
                    if selected < menu_items.len() - 1 {
                        selected += 1;
                        if selected >= scroll + panel_content_height {
                            scroll = selected - panel_content_height + 1;
                        }
                    }
                }
                KEY_WHEELUP => {
                    let amount = 10usize;
                    selected = selected.saturating_sub(amount);
                    scroll = scroll.saturating_sub(amount);
                }
                KEY_WHEELDOWN => {
                    let amount = 10usize;
                    let max_selected = menu_items.len().saturating_sub(1);
                    let max_scroll = menu_items.len().saturating_sub(panel_content_height);
                    selected = (selected + amount).min(max_selected);
                    scroll = (scroll + amount).min(max_scroll);
                }
                KEY_HOME => {
                    selected = 0;
                    scroll = 0;
                }
                KEY_END => {
                    selected = menu_items.len().saturating_sub(1);
                    if selected >= panel_content_height {
                        scroll = selected - panel_content_height + 1;
                    }
                }
                KEY_PPAGE => {
                    let amount = panel_content_height.saturating_sub(1);
                    selected = selected.saturating_sub(amount);
                    scroll = scroll.saturating_sub(amount);
                }
                KEY_NPAGE => {
                    let amount = panel_content_height.saturating_sub(1);
                    let max_selected = menu_items.len().saturating_sub(1);
                    let max_scroll = menu_items.len().saturating_sub(panel_content_height);
                    selected = (selected + amount).min(max_selected);
                    scroll = (scroll + amount).min(max_scroll);
                }
                0x0A | 0x0D => {
                    // Enter
                    machine.filter_user_id = menu_items[selected].0;
                    break;
                }
                0x1B => {
                    // Escape
                    break;
                }
                _ => {}
            }
        }

        // Restore main panel's original position and size
        self.main_panel.x = orig_main_x;
        self.main_panel.w = orig_main_w;
        self.main_panel.y = orig_main_y;
        self.main_panel.h = orig_main_h;
        self.main_panel.scroll_v = orig_main_scroll_v;

        // Clear screen to trigger full redraw when returning to main view
        crt.clear();
        crt.enable_delay();
    }
}
