//! MainPanel - Main process list panel

#![allow(dead_code)]

use super::crt::{ColorElement, KEY_DEL_MAC, KEY_F10, KEY_F15, KEY_F3, KEY_F4};
use super::function_bar::FunctionBar;
use super::panel::HandlerResult;
use super::rich_string::RichString;
use super::row_print::{print_kbytes, print_left_aligned, print_percentage, print_time};
use super::Crt;
use crate::core::{Machine, Process, ProcessField, ProcessState, Settings};
use ncurses::*;

/// Incremental mode type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncType {
    Search,
    Filter,
}

/// Incremental search/filter state (matches C htop IncSet)
#[derive(Debug, Default)]
pub struct IncSearch {
    pub active: bool,
    pub text: String,
    pub mode: Option<IncType>,
    pub found: bool, // For search mode - whether match was found
}

impl IncSearch {
    pub fn new() -> Self {
        IncSearch {
            active: false,
            text: String::new(),
            mode: None,
            found: true,
        }
    }

    /// Start incremental search/filter mode
    /// For filter mode, pass the current filter text to restore it
    pub fn start(&mut self, mode: IncType, existing_filter: Option<&str>) {
        self.active = true;
        self.mode = Some(mode);
        // For search, always clear text
        // For filter, restore from existing filter if present
        if mode == IncType::Search {
            self.text.clear();
        } else if let Some(filter) = existing_filter {
            self.text = filter.to_string();
        }
        self.found = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
        // For search mode, clear text on stop
        if self.mode == Some(IncType::Search) {
            self.text.clear();
        }
        self.mode = None;
    }

    pub fn is_filter(&self) -> bool {
        self.mode == Some(IncType::Filter)
    }

    pub fn is_search(&self) -> bool {
        self.mode == Some(IncType::Search)
    }

    pub fn add_char(&mut self, ch: char) {
        self.text.push(ch);
    }

    pub fn backspace(&mut self) {
        self.text.pop();
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }
}

/// Main process list panel
pub struct MainPanel {
    // Position and size
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,

    // Selection
    pub selected: i32,
    pub scroll_v: i32,
    pub scroll_h: i32,

    // Process display
    pub fields: Vec<ProcessField>,

    // Search/filter
    pub inc_search: IncSearch,
    pub filter: Option<String>,

    // Following state (for filter/search - shows yellow highlight)
    pub following: bool,
    pub following_pid: Option<i32>, // PID to follow when following is enabled
    pub selection_color: ColorElement,

    // Function bar
    pub function_bar: FunctionBar,

    // Display settings
    pub tree_view: bool,
    pub show_header: bool,
    pub needs_redraw: bool,
    pub wrap_command: bool, // Wrap long command lines

    // PID search
    pub pid_search: Option<String>,
}

impl MainPanel {
    pub fn new() -> Self {
        MainPanel {
            x: 0,
            y: 0,
            w: 80,
            h: 24,
            selected: 0,
            scroll_v: 0,
            scroll_h: 0,
            // Default fields matching C htop darwin/Platform.c
            // "PID USER PRIORITY NICE M_VIRT M_RESIDENT STATE PERCENT_CPU PERCENT_MEM TIME Command"
            fields: vec![
                ProcessField::Pid,
                ProcessField::User,
                ProcessField::Priority,
                ProcessField::Nice,
                ProcessField::MSize,     // M_VIRT
                ProcessField::MResident, // M_RESIDENT
                ProcessField::State,
                ProcessField::PercentCpu, // PERCENT_CPU
                ProcessField::PercentMem, // PERCENT_MEM
                ProcessField::Time,
                ProcessField::Command,
            ],
            inc_search: IncSearch::new(),
            filter: None,
            following: false,
            following_pid: None,
            selection_color: ColorElement::PanelSelectionFocus,
            function_bar: FunctionBar::new(),
            tree_view: false,
            show_header: true,
            needs_redraw: true,
            wrap_command: false,
            pid_search: None,
        }
    }

    /// Set the filter text (used when filter mode completes)
    pub fn set_filter(&mut self, filter: &str) {
        if filter.is_empty() {
            self.filter = None;
        } else {
            self.filter = Some(filter.to_string());
        }
    }

    /// Clear the filter
    pub fn clear_filter(&mut self) {
        self.filter = None;
        self.inc_search.text.clear();
        // Reset to normal selection color
        self.following = false;
        self.selection_color = ColorElement::PanelSelectionFocus;
    }

    /// Check if filtering is active (has filter text)
    pub fn is_filtering(&self) -> bool {
        self.filter.is_some()
    }

    /// Update function bar labels based on current state
    /// Matches C htop MainPanel_updateLabels behavior
    pub fn update_labels(&mut self, tree_view: bool, has_filter: bool) {
        self.tree_view = tree_view;

        // Update F5 label: shows what action will be taken
        let tree_label = if tree_view { "List  " } else { "Tree  " };
        self.function_bar.set_function(4, "F5", tree_label);

        // Update F4 label: "FILTER" when filter active, "Filter" otherwise
        // C htop uses uppercase to indicate filter is active
        let filter_label = if has_filter { "FILTER" } else { "Filter" };
        self.function_bar.set_function(3, "F4", filter_label);
    }

    /// Move the panel
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.needs_redraw = true;
    }

    /// Get the panel's y position (header row)
    pub fn panel_y(&self) -> i32 {
        self.y
    }

    /// Get the field at a given x position in the header
    /// This is used for header click handling
    pub fn field_at_x(&self, x: i32) -> Option<ProcessField> {
        // Account for panel's x offset and horizontal scroll
        let rel_x = x - self.x + self.scroll_h;
        if rel_x < 0 {
            return None;
        }

        let mut current_x = 0;
        for field in &self.fields {
            let title_len = field.title().len() as i32;
            if rel_x >= current_x && rel_x < current_x + title_len {
                return Some(*field);
            }
            current_x += title_len;
        }

        // Default to Command if clicked past all fields
        Some(ProcessField::Command)
    }

    /// Resize the panel
    pub fn resize(&mut self, w: i32, h: i32) {
        self.w = w;
        self.h = h;
        self.needs_redraw = true;
    }

    /// Ensure the selected process is visible
    fn ensure_visible(&mut self, process_count: i32) {
        let visible_height = if self.show_header { self.h - 1 } else { self.h };

        if self.selected < self.scroll_v {
            self.scroll_v = self.selected;
        } else if self.selected >= self.scroll_v + visible_height {
            self.scroll_v = self.selected - visible_height + 1;
        }

        // Clamp scroll
        let max_scroll = (process_count - visible_height).max(0);
        self.scroll_v = self.scroll_v.clamp(0, max_scroll);
    }

    /// Draw the panel header with sort indicator
    fn draw_header(
        &self,
        crt: &Crt,
        y: i32,
        _settings: &Settings,
        sort_key: ProcessField,
        sort_descending: bool,
    ) {
        let header_attr = crt.color(ColorElement::PanelHeaderFocus);
        let sort_attr = crt.color(ColorElement::PanelSelectionFocus);

        // Determine the active sort key - in tree view, it's always PID
        let active_sort_key = if self.tree_view {
            ProcessField::Pid
        } else {
            sort_key
        };

        // In tree view, sort is always ascending
        let ascending = if self.tree_view {
            true
        } else {
            !sort_descending
        };

        // Fill the line with the header attribute (starting at self.x, not 0)
        mv(y, self.x);
        attron(header_attr);
        for _ in 0..self.w {
            addch(' ' as u32);
        }
        attroff(header_attr);

        // Draw field titles with highlighting for sort column
        mv(y, self.x);
        let mut str = RichString::with_capacity(256);

        for field in &self.fields {
            let title = field.title();
            let is_sort_column = *field == active_sort_key;
            let attr = if is_sort_column {
                sort_attr
            } else {
                header_attr
            };

            str.append(title, attr);

            if is_sort_column {
                // Replace trailing space with sort indicator (matches C htop Table.c:308-314)
                // Check if last char is a space, if so replace it with indicator
                if str.len() > 0 && str.last_char() == Some(' ') {
                    str.rewind(1);
                    // Ascending (small to large): △ (north), Descending (large to small): ▽ (south)
                    let indicator = if ascending {
                        crt.tree_str.asc
                    } else {
                        crt.tree_str.desc
                    };
                    str.append(indicator, attr);
                }
            }
        }

        str.write_at_width(y, self.x, self.w as usize);
    }

    /// Draw a single process line with C htop-compatible coloring
    fn draw_process(
        &self,
        crt: &Crt,
        y: i32,
        process: &Process,
        selected: bool,
        settings: &Settings,
        current_uid: u32,
    ) {
        let selection_attr = if selected {
            crt.color(self.selection_color)
        } else {
            0 // No selection highlight
        };

        let mut str = RichString::with_capacity(256);
        let coloring = settings.highlight_megabytes;

        // Compute if this row should be shadowed (other user's process)
        let is_shadowed = settings.shadow_other_users && process.uid != current_uid;

        for field in &self.fields {
            self.write_field(
                &mut str,
                process,
                *field,
                coloring,
                crt,
                settings.show_program_path,
                settings.highlight_threads,
                settings.highlight_base_name,
                settings.show_thread_names,
                is_shadowed,
            );
        }

        // Apply selection highlighting if selected
        if selected {
            // For selected rows, we draw with selection background
            mv(y, self.x);
            attron(selection_attr);

            let text = str.text();
            let display_text: String = text.chars().take(self.w as usize).collect();
            let _ = addstr(&display_text);

            // Pad to width
            let current_len = display_text.chars().count();
            for _ in current_len..self.w as usize {
                addch(' ' as u32);
            }
            attroff(selection_attr);
        } else {
            // For non-selected rows, use the RichString with per-field colors
            str.write_at_width(y, self.x, self.w as usize);
        }
    }

    /// Write a single field to a RichString with C htop-compatible coloring
    fn write_field(
        &self,
        str: &mut RichString,
        process: &Process,
        field: ProcessField,
        coloring: bool,
        crt: &Crt,
        show_program_path: bool,
        highlight_threads: bool,
        highlight_base_name: bool,
        show_thread_names: bool,
        is_shadowed: bool,
    ) {
        let process_color = crt.color(ColorElement::Process);
        let shadow_color = crt.color(ColorElement::ProcessShadow);
        let basename_color = crt.color(ColorElement::ProcessBasename);

        // When is_shadowed is true, use shadow_color for all fields that would normally
        // use process_color (matches C htop behavior for shadow_other_users)
        let base_color = if is_shadowed {
            shadow_color
        } else {
            process_color
        };

        match field {
            ProcessField::Pid => {
                // PID: right-aligned, typically 5-7 digits
                str.append(&format!("{:>5} ", process.pid), base_color);
            }
            ProcessField::Ppid => {
                str.append(&format!("{:>5} ", process.ppid), base_color);
            }
            ProcessField::User => {
                // USER: left-aligned, 10 chars, always use is_shadowed computation
                let user = process.user.as_deref().unwrap_or("?");
                print_left_aligned(str, base_color, user, 10);
            }
            ProcessField::State => {
                // STATE: single char with colors (state colors override shadowing)
                let state_char = process.state.to_char();
                let attr = if is_shadowed {
                    shadow_color
                } else {
                    match process.state {
                        ProcessState::Running | ProcessState::Runnable | ProcessState::Traced => {
                            crt.color(ColorElement::ProcessRunState)
                        }
                        ProcessState::Blocked
                        | ProcessState::Defunct
                        | ProcessState::Stopped
                        | ProcessState::UninterruptibleWait
                        | ProcessState::Zombie => crt.color(ColorElement::ProcessDState),
                        ProcessState::Queued
                        | ProcessState::Waiting
                        | ProcessState::Idle
                        | ProcessState::Sleeping => shadow_color,
                        _ => process_color,
                    }
                };
                str.append(&format!("{} ", state_char), attr);
            }
            ProcessField::Priority => {
                // PRIORITY: 3 chars, RT for realtime
                if process.priority <= -100 {
                    str.append(" RT ", base_color);
                } else {
                    str.append(&format!("{:>3} ", process.priority), base_color);
                }
            }
            ProcessField::Nice => {
                // NICE: 3 chars, colored by value (nice colors override shadowing)
                let attr = if is_shadowed {
                    shadow_color
                } else if process.nice < 0 {
                    crt.color(ColorElement::ProcessHighPriority)
                } else if process.nice > 0 {
                    crt.color(ColorElement::ProcessLowPriority)
                } else {
                    shadow_color
                };
                str.append(&format!("{:>3} ", process.nice), attr);
            }
            ProcessField::MSize => {
                // VIRT: memory in KiB with coloring (disabled if shadowed)
                print_kbytes(str, process.m_virt as u64, coloring && !is_shadowed, crt);
            }
            ProcessField::MResident => {
                // RES: memory in KiB with coloring (disabled if shadowed)
                print_kbytes(
                    str,
                    process.m_resident as u64,
                    coloring && !is_shadowed,
                    crt,
                );
            }
            ProcessField::MShare => {
                // SHR: memory in KiB with coloring (disabled if shadowed)
                print_kbytes(str, process.m_share as u64, coloring && !is_shadowed, crt);
            }
            ProcessField::PercentCpu => {
                // CPU%: percentage with coloring (width 5 to match C htop)
                // When shadowed, use shadow color
                if is_shadowed {
                    str.append(&format!("{:>4.1} ", process.percent_cpu), shadow_color);
                } else {
                    print_percentage(str, process.percent_cpu, 5, crt);
                }
            }
            ProcessField::PercentMem => {
                // MEM%: percentage with coloring (width 4, no autoWidth)
                // When shadowed, use shadow color
                if is_shadowed {
                    str.append(&format!("{:>3.1} ", process.percent_mem), shadow_color);
                } else {
                    print_percentage(str, process.percent_mem, 4, crt);
                }
            }
            ProcessField::Time => {
                // TIME+: time with coloring (disabled if shadowed)
                print_time(str, process.time, coloring && !is_shadowed, crt);
            }
            ProcessField::Nlwp => {
                // NLWP: thread count, dim if 1 or if shadowed
                let attr = if is_shadowed || process.nlwp == 1 {
                    shadow_color
                } else {
                    process_color
                };
                str.append(&format!("{:>4} ", process.nlwp), attr);
            }
            ProcessField::Processor => {
                str.append(&format!("{:>3} ", process.processor), base_color);
            }
            ProcessField::Command | ProcessField::CmdLine => {
                // Command: use basename highlighting with tree view support
                // When show_program_path is false, show command starting from basename
                // (basename + arguments), matching C htop behavior
                //
                // Settings that affect command display:
                // - show_thread_names: for threads, show thread's own name instead of parent command
                // - highlight_threads: use ProcessThread color for threads
                // - highlight_base_name: highlight the basename portion
                // - is_shadowed: use shadow_color for other users' processes

                // Determine the command text to display
                let cmd = if show_thread_names && process.is_thread() {
                    // Show thread's own name (comm field)
                    process.comm.as_deref().unwrap_or_else(|| {
                        if show_program_path {
                            process.get_command()
                        } else {
                            process.get_command_from_basename()
                        }
                    })
                } else if show_program_path {
                    process.get_command()
                } else {
                    process.get_command_from_basename()
                };

                // Draw tree indentation if in tree view mode
                if self.tree_view && process.indent != 0 {
                    let tree_attr = crt.color(ColorElement::ProcessTree);
                    let is_last = process.indent < 0;
                    let mut indent_bits = if is_last {
                        -process.indent
                    } else {
                        process.indent
                    };

                    // Right-shift through indent bits, drawing vertical lines where needed
                    // (matches C htop Process.c lines 599-612)
                    while indent_bits > 1 {
                        if indent_bits & 1 != 0 {
                            str.append(crt.tree_str.vert, tree_attr);
                            str.append("  ", tree_attr);
                        } else {
                            str.append("   ", tree_attr);
                        }
                        indent_bits >>= 1;
                    }

                    // Draw the branch connector (├ or └)
                    let branch = if is_last {
                        crt.tree_str.bend
                    } else {
                        crt.tree_str.rtee
                    };
                    str.append(branch, tree_attr);
                    // Draw expand/collapse indicator
                    let indicator = if process.show_children {
                        crt.tree_str.shut
                    } else {
                        crt.tree_str.open
                    };
                    str.append(indicator, tree_attr);
                    str.append(" ", tree_attr);
                }

                // Determine colors based on settings
                if is_shadowed {
                    // Shadow overrides all other coloring for commands
                    str.append(cmd, shadow_color);
                } else if process.is_thread() && highlight_threads {
                    // Thread highlighting (only if enabled)
                    str.append(cmd, crt.color(ColorElement::ProcessThread));
                } else if highlight_base_name {
                    // Basename highlighting (only if enabled)
                    if show_program_path {
                        // Highlight basename portion when showing full path
                        let basename = process.get_basename();
                        if let Some(pos) = cmd.find(basename) {
                            if pos > 0 {
                                str.append(&cmd[..pos], base_color);
                            }
                            str.append(basename, basename_color);
                            let after = pos + basename.len();
                            if after < cmd.len() {
                                str.append(&cmd[after..], base_color);
                            }
                        } else {
                            str.append(cmd, base_color);
                        }
                    } else {
                        // When not showing path, cmd starts with basename followed by arguments
                        // Highlight the basename portion, then show arguments in normal color
                        let basename = process.get_basename();
                        if cmd.starts_with(basename) {
                            str.append(basename, basename_color);
                            let after = basename.len();
                            if after < cmd.len() {
                                str.append(&cmd[after..], base_color);
                            }
                        } else {
                            // Fallback: just show entire command highlighted
                            str.append(cmd, basename_color);
                        }
                    }
                } else {
                    // No special highlighting, use base color
                    str.append(cmd, base_color);
                }
                str.append_char(' ', base_color);
            }
            ProcessField::Tty => {
                let tty = process.tty_name.as_deref().unwrap_or("?");
                let attr = if is_shadowed || tty == "?" {
                    shadow_color
                } else {
                    process_color
                };
                print_left_aligned(str, attr, tty, 8);
            }
            _ => {
                // Default: show placeholder
                str.append("? ", base_color);
            }
        }
    }

    /// Draw the panel
    pub fn draw(&mut self, crt: &Crt, machine: &Machine, settings: &Settings) {
        let visible_height = if self.show_header { self.h - 1 } else { self.h };
        let start_y = if self.show_header {
            self.draw_header(
                crt,
                self.y,
                settings,
                machine.sort_key,
                machine.sort_descending,
            );
            self.y + 1
        } else {
            self.y
        };

        // Filter and collect visible processes
        let processes: Vec<&Process> = if self.tree_view {
            // In tree view, use tree display order
            machine
                .processes
                .iter_tree()
                .filter(|p| self.should_show_process(p, settings, machine))
                .collect()
        } else {
            machine
                .processes
                .iter()
                .filter(|p| self.should_show_process(p, settings, machine))
                .collect()
        };

        let process_count = processes.len() as i32;
        self.ensure_visible(process_count);

        // Get current user ID for highlighting
        let current_uid = machine.htop_user_id;

        // Draw processes
        for i in 0..visible_height {
            let process_idx = (self.scroll_v + i) as usize;
            let y = start_y + i;

            if process_idx < processes.len() {
                let selected = process_idx as i32 == self.selected;
                self.draw_process(
                    crt,
                    y,
                    processes[process_idx],
                    selected,
                    settings,
                    current_uid,
                );
            } else {
                // Empty line
                mv(y, self.x);
                for _ in 0..self.w {
                    addch(' ' as u32);
                }
            }
        }

        // Note: Search/filter bar is now drawn by ScreenManager at the bottom of the screen

        self.needs_redraw = false;
    }

    /// Check if a process matches the current filter
    fn matches_filter(&self, process: &Process) -> bool {
        if let Some(ref filter) = self.filter {
            let cmd = process.get_command().to_lowercase();
            let filter_lower = filter.to_lowercase();
            cmd.contains(&filter_lower)
        } else {
            true
        }
    }

    /// Check if a process should be visible based on settings
    fn should_show_process(
        &self,
        process: &Process,
        settings: &Settings,
        machine: &Machine,
    ) -> bool {
        // Check text filter first
        if !self.matches_filter(process) {
            return false;
        }

        // Check user filter
        if let Some(filter_uid) = machine.filter_user_id {
            if process.uid != filter_uid {
                return false;
            }
        }

        // Check kernel threads filter
        if settings.hide_kernel_threads && process.is_kernel_thread {
            return false;
        }

        // Check userland threads filter
        if settings.hide_userland_threads && process.is_userland_thread {
            return false;
        }

        true
    }

    /// Handle a key event
    pub fn on_key(&mut self, key: i32, machine: &Machine) -> HandlerResult {
        // Handle search mode
        if self.inc_search.active {
            return self.handle_search_key(key, machine);
        }

        match key {
            KEY_UP | 0x10 => {
                // Up or Ctrl+P
                self.move_selection(-1, machine);
                HandlerResult::Handled
            }
            KEY_DOWN | 0x0E => {
                // Down or Ctrl+N
                self.move_selection(1, machine);
                HandlerResult::Handled
            }
            KEY_PPAGE => {
                self.move_selection(-(self.h - 2), machine);
                HandlerResult::Handled
            }
            KEY_NPAGE => {
                self.move_selection(self.h - 2, machine);
                HandlerResult::Handled
            }
            KEY_HOME => {
                self.selected = 0;
                self.scroll_v = 0;
                HandlerResult::Handled
            }
            KEY_END => {
                let count = self.get_visible_count(machine);
                self.selected = (count - 1).max(0);
                HandlerResult::Handled
            }
            KEY_F3 | 0x2F => {
                // F3 or /
                self.inc_search.start(IncType::Search, None);
                HandlerResult::Handled
            }
            KEY_F4 | 0x5C => {
                // F4 or \
                // Start filter mode with existing filter text (if any)
                // Matches C htop actionIncFilter - always opens filter mode
                self.inc_search
                    .start(IncType::Filter, self.filter.as_deref());
                HandlerResult::Handled
            }
            // Note: F5 (tree view toggle) is handled by ScreenManager
            KEY_F10 | 0x71 | 0x51 => {
                // F10 or q/Q
                HandlerResult::BreakLoop
            }
            _ => HandlerResult::Ignored,
        }
    }

    /// Handle key in search/filter mode (matches C htop IncSet_handleKey)
    fn handle_search_key(&mut self, key: i32, machine: &Machine) -> HandlerResult {
        let is_filter = self.inc_search.is_filter();
        let is_search = self.inc_search.is_search();

        match key {
            // F3 - find next (search mode only)
            KEY_F3 if is_search => {
                self.find_next(machine, 1);
                HandlerResult::Handled
            }
            // Shift-F3 - find previous (search mode only)
            KEY_F15 if is_search => {
                self.find_next(machine, -1);
                HandlerResult::Handled
            }
            0x1B => {
                // ESC - for filter: clear and exit; for search: just exit
                if is_filter {
                    // Esc in filter mode clears the filter (like C htop)
                    self.inc_search.clear();
                    self.filter = None;
                }
                self.inc_search.stop();
                // Reset to normal selection color when exiting search/filter without active filter
                if self.filter.is_none() {
                    self.following = false;
                    self.selection_color = ColorElement::PanelSelectionFocus;
                }
                HandlerResult::Handled
            }
            // Ctrl+U - clear the text
            0x15 => {
                self.inc_search.clear();
                if is_filter {
                    self.filter = None;
                }
                // Reset selection color since text is cleared
                self.following = false;
                self.selection_color = ColorElement::PanelSelectionFocus;
                self.do_incremental_search(machine);
                HandlerResult::Handled
            }
            KEY_BACKSPACE | KEY_DEL_MAC | 0x08 => {
                self.inc_search.backspace();
                if is_filter {
                    // Update filter in real-time
                    self.filter = if self.inc_search.text.is_empty() {
                        None
                    } else {
                        Some(self.inc_search.text.clone())
                    };
                }
                self.do_incremental_search(machine);
                HandlerResult::Handled
            }
            ch if ch >= 0x20 && ch < 0x7F => {
                self.inc_search.add_char(ch as u8 as char);
                if is_filter {
                    // Update filter in real-time
                    self.filter = Some(self.inc_search.text.clone());
                }
                self.do_incremental_search(machine);
                HandlerResult::Handled
            }
            _ => {
                // Any other key (Enter, arrows, F-keys, etc.) exits filter/search mode
                // but keeps the filter if one was set (matches C htop IncSet_handleKey)
                if is_filter {
                    // Apply the current filter text
                    self.filter = if self.inc_search.text.is_empty() {
                        None
                    } else {
                        Some(self.inc_search.text.clone())
                    };
                }
                self.inc_search.stop();
                // Keep following state if filter is active, otherwise reset
                if self.filter.is_none() {
                    self.following = false;
                    self.selection_color = ColorElement::PanelSelectionFocus;
                }
                // Return Ignored so the key can be processed by the caller
                // (e.g., arrow keys should still navigate after exiting filter mode)
                HandlerResult::Ignored
            }
        }
    }

    /// Find next/previous match (matches C htop IncMode_find)
    /// step: 1 for next, -1 for previous
    fn find_next(&mut self, machine: &Machine, step: i32) {
        if self.inc_search.text.is_empty() {
            return;
        }

        let search_lower = self.inc_search.text.to_lowercase();
        let processes: Vec<&Process> = machine
            .processes
            .iter()
            .filter(|p| self.matches_filter(p))
            .collect();

        let size = processes.len() as i32;
        if size == 0 {
            return;
        }

        let mut i = self.selected;
        loop {
            i += step;
            // Wrap around
            if i >= size {
                i = 0;
            }
            if i < 0 {
                i = size - 1;
            }
            // If we're back where we started, no match found
            if i == self.selected {
                self.inc_search.found = false;
                return;
            }

            let cmd = processes[i as usize].get_command().to_lowercase();
            if cmd.contains(&search_lower) {
                self.selected = i;
                self.ensure_visible(size);
                self.inc_search.found = true;
                self.following = true;
                self.selection_color = ColorElement::PanelSelectionFollow;
                return;
            }
        }
    }

    /// Perform incremental search (matches C htop search function)
    fn do_incremental_search(&mut self, machine: &Machine) {
        if self.inc_search.text.is_empty() {
            self.inc_search.found = true;
            // No search text - reset to normal selection color
            self.following = false;
            self.selection_color = ColorElement::PanelSelectionFocus;
            return;
        }

        let search_lower = self.inc_search.text.to_lowercase();

        // Search from current position
        let processes: Vec<&Process> = machine
            .processes
            .iter()
            .filter(|p| self.matches_filter(p))
            .collect();

        for (i, process) in processes.iter().enumerate() {
            let cmd = process.get_command().to_lowercase();
            if cmd.contains(&search_lower) {
                self.selected = i as i32;
                self.ensure_visible(processes.len() as i32);
                self.inc_search.found = true;
                // Match found - set following state (yellow highlight)
                self.following = true;
                self.selection_color = ColorElement::PanelSelectionFollow;
                return;
            }
        }

        // No match found
        self.inc_search.found = false;
        // Keep following state but indicate no match
        self.following = false;
        self.selection_color = ColorElement::PanelSelectionFocus;
    }

    /// Move selection by delta
    fn move_selection(&mut self, delta: i32, machine: &Machine) {
        let count = self.get_visible_count(machine);
        if count == 0 {
            return;
        }

        self.selected = (self.selected + delta).clamp(0, count - 1);
        self.ensure_visible(count);
    }

    /// Scroll by wheel amount (matches C htop PANEL_SCROLL macro)
    /// This moves BOTH selection AND scroll position by the given amount
    pub fn scroll_wheel(&mut self, amount: i32, machine: &Machine) {
        let count = self.get_visible_count(machine);
        if count == 0 {
            return;
        }

        let visible_height = if self.show_header { self.h - 1 } else { self.h };
        let max_scroll = (count - visible_height).max(0);

        // Move both selected and scroll_v by the amount (like C htop PANEL_SCROLL)
        self.selected += amount;
        self.scroll_v = (self.scroll_v + amount).clamp(0, max_scroll);

        // Clamp selected to valid range
        self.selected = self.selected.clamp(0, count - 1);
    }

    /// Get count of visible processes
    fn get_visible_count(&self, machine: &Machine) -> i32 {
        machine
            .processes
            .iter()
            .filter(|p| self.matches_filter(p))
            .count() as i32
    }

    /// Get the currently selected process
    pub fn get_selected_process<'a>(&self, machine: &'a Machine) -> Option<&'a Process> {
        let processes: Vec<&Process> = machine
            .processes
            .iter()
            .filter(|p| self.matches_filter(p))
            .collect();

        processes.get(self.selected as usize).copied()
    }

    /// Get the selected PID
    pub fn get_selected_pid(&self, machine: &Machine) -> Option<i32> {
        self.get_selected_process(machine).map(|p| p.pid)
    }

    /// Toggle cursor following mode
    pub fn toggle_following(&mut self, machine: &Machine) {
        self.following = !self.following;
        if self.following {
            // Store the current selected PID to follow
            self.following_pid = self.get_selected_pid(machine);
            self.selection_color = ColorElement::PanelSelectionFollow;
        } else {
            self.following_pid = None;
            self.selection_color = ColorElement::PanelSelectionFocus;
        }
    }

    /// Update selection to follow the tracked PID
    pub fn update_following(&mut self, machine: &Machine) {
        if let Some(pid) = self.following_pid {
            let processes: Vec<&Process> = if self.tree_view {
                machine
                    .processes
                    .iter_tree()
                    .filter(|p| self.matches_filter(p))
                    .collect()
            } else {
                machine
                    .processes
                    .iter()
                    .filter(|p| self.matches_filter(p))
                    .collect()
            };

            // Find the process with the tracked PID
            for (i, p) in processes.iter().enumerate() {
                if p.pid == pid {
                    self.selected = i as i32;
                    self.ensure_visible(processes.len() as i32);
                    return;
                }
            }
            // Process no longer exists, stop following
            self.following = false;
            self.following_pid = None;
            self.selection_color = ColorElement::PanelSelectionFocus;
        }
    }

    /// Toggle wrap command display
    pub fn toggle_wrap_command(&mut self) {
        self.wrap_command = !self.wrap_command;
    }

    /// Start incremental PID search
    pub fn start_pid_search(&mut self, digit: char, machine: &Machine) {
        // Initialize or append to PID search string
        if self.pid_search.is_none() {
            self.pid_search = Some(String::new());
        }

        if let Some(ref mut search) = self.pid_search {
            search.push(digit);
        }

        // Get the search string (non-mutable borrow)
        let search_str = self.pid_search.as_ref().unwrap().clone();
        let filter = self.filter.clone();

        // Find process matching the PID prefix
        let processes: Vec<&Process> = if self.tree_view {
            machine
                .processes
                .iter_tree()
                .filter(|p| self.matches_filter_with(p, &filter))
                .collect()
        } else {
            machine
                .processes
                .iter()
                .filter(|p| self.matches_filter_with(p, &filter))
                .collect()
        };

        // Find first process whose PID starts with the search number
        for (i, p) in processes.iter().enumerate() {
            if p.pid.to_string().starts_with(&search_str) {
                self.selected = i as i32;
                self.ensure_visible(processes.len() as i32);
                break;
            }
        }

        // Clear the search after a delay (we'll clear it next time a non-digit key is pressed)
        // For now, just clear after processing
        // In C htop, there's a timeout - we'll simplify by clearing on next non-digit
    }

    /// Helper for filtering with explicit filter
    fn matches_filter_with(&self, process: &Process, filter: &Option<String>) -> bool {
        if let Some(ref f) = filter {
            let cmd = process.get_command().to_lowercase();
            let filter_lower = f.to_lowercase();
            cmd.contains(&filter_lower)
        } else {
            true
        }
    }

    /// Clear PID search state
    pub fn clear_pid_search(&mut self) {
        self.pid_search = None;
    }
}

impl Default for MainPanel {
    fn default() -> Self {
        MainPanel::new()
    }
}
