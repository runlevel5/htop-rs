//! MainPanel - Main process list panel

#![allow(dead_code)]

use super::crt::{ColorElement, KEY_DEL_MAC, KEY_F10, KEY_F15, KEY_F3, KEY_F4};
use super::function_bar::FunctionBar;
use super::panel::HandlerResult;
use super::rich_string::RichString;
use super::row_print::{
    print_kbytes, print_left_aligned, print_percentage, print_time,
};
#[cfg(target_os = "linux")]
use super::row_print::print_rate;
use super::Crt;
use crate::core::{highlight_flags, FieldWidths, Machine, Process, ProcessField, ProcessState, Settings};
#[cfg(target_os = "linux")]
use crate::platform::linux::{
    ioprio_class, ioprio_data, IOPRIO_CLASS_BE, IOPRIO_CLASS_IDLE, IOPRIO_CLASS_NONE,
    IOPRIO_CLASS_RT,
};
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

    // Selection (matches C htop Panel struct)
    pub selected: i32,
    old_selected: i32, // Track previous selection for partial redraw optimization
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

    // Cached display list - indices into process list (rebuilt on data change)
    // This avoids filtering on every draw, matching C htop's Table_rebuildPanel
    cached_display_indices: Vec<usize>,
    display_list_valid: bool,
}

impl MainPanel {
    pub fn new() -> Self {
        MainPanel {
            x: 0,
            y: 0,
            w: 80,
            h: 24,
            selected: 0,
            old_selected: 0,
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
            cached_display_indices: Vec::new(),
            display_list_valid: false,
        }
    }

    /// Set the filter text (used when filter mode completes)
    pub fn set_filter(&mut self, filter: &str) {
        if filter.is_empty() {
            self.filter = None;
        } else {
            self.filter = Some(filter.to_string());
        }
        self.invalidate_display_list();
    }

    /// Clear the filter
    pub fn clear_filter(&mut self) {
        self.filter = None;
        self.inc_search.text.clear();
        // Reset to normal selection color
        self.following = false;
        self.selection_color = ColorElement::PanelSelectionFocus;
        self.invalidate_display_list();
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

    /// Invalidate the cached display list (call when filters change)
    pub fn invalidate_display_list(&mut self) {
        self.display_list_valid = false;
        // When display list changes (filter/sort/etc), we need a full redraw
        // since the set of visible processes may have changed
        self.needs_redraw = true;
    }

    /// Rebuild the cached display list (like C htop's Table_rebuildPanel)
    /// This filters processes once per update cycle instead of on every draw
    pub fn rebuild_display_list(&mut self, machine: &Machine, settings: &Settings) {
        use std::collections::HashMap;
        
        self.cached_display_indices.clear();

        // Build PID→index map for O(1) lookups (avoids O(n²) linear search)
        // This is built fresh each time since indices change after sorting
        let pid_to_idx: HashMap<i32, usize> = machine
            .processes
            .processes
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.pid, idx))
            .collect();

        if self.tree_view {
            // In tree view, iterate tree_display_order directly (PIDs in tree order)
            // We use the PID→index map to avoid the O(n²) lookup that iter_tree() does
            for &pid in &machine.processes.tree_display_order {
                // O(1) lookup to get process index
                if let Some(&idx) = pid_to_idx.get(&pid) {
                    if let Some(process) = machine.processes.processes.get(idx) {
                        // Only include visible processes (show_children handling)
                        if process.is_visible && self.should_show_process(process, settings, machine) {
                            self.cached_display_indices.push(idx);
                        }
                    }
                }
            }
        } else {
            // Normal view - iterate in sorted order
            for (i, process) in machine.processes.iter().enumerate() {
                if self.should_show_process(process, settings, machine) {
                    self.cached_display_indices.push(i);
                }
            }
        }

        self.display_list_valid = true;
    }

    /// Ensure the selected process is visible
    /// Sets needs_redraw if scroll position changed (matches C htop Panel_draw)
    pub fn ensure_visible(&mut self, process_count: i32) {
        let visible_height = if self.show_header { self.h - 1 } else { self.h };

        // Matches C htop Panel_draw() lines 265-271:
        // When scroll needs to change to keep selection visible, set needsRedraw
        if self.selected < self.scroll_v {
            self.scroll_v = self.selected;
            self.needs_redraw = true;
        } else if self.selected >= self.scroll_v + visible_height {
            self.scroll_v = self.selected - visible_height + 1;
            self.needs_redraw = true;
        }

        // Clamp scroll (matches C htop Panel_draw() lines 257-263)
        let max_scroll = (process_count - visible_height).max(0);
        let old_scroll = self.scroll_v;
        self.scroll_v = self.scroll_v.clamp(0, max_scroll);
        if self.scroll_v != old_scroll {
            self.needs_redraw = true;
        }
    }

    /// Draw the panel header with sort indicator
    fn draw_header(
        &self,
        crt: &Crt,
        y: i32,
        settings: &Settings,
        sort_key: ProcessField,
        sort_descending: bool,
        filter_active: bool,
        field_widths: &FieldWidths,
    ) {
        // Normal header color (green)
        let header_attr = crt.color(ColorElement::PanelHeaderFocus);
        // Yellow for filter indicator on Command column
        let filter_attr = crt.color(ColorElement::PanelSelectionFollow);
        let sort_attr = crt.color(ColorElement::PanelSelectionFocus);

        // Get active sort key and direction matching C htop's ScreenSettings_getActiveSortKey
        let screen = &settings.screens[settings.active_screen];
        let (active_sort_key, ascending) = if self.tree_view {
            if screen.tree_view_always_by_pid {
                // In tree view with always-by-PID: sort is by PID ascending
                (ProcessField::Pid, true)
            } else {
                // In tree view: use tree_sort_key
                (screen.tree_sort_key, screen.tree_direction > 0)
            }
        } else {
            // Not in tree view: use regular sort key
            (sort_key, !sort_descending)
        };

        // Fill the line with the header attribute (starting at self.x, not 0)
        mv(y, self.x);
        attrset(header_attr);
        for _ in 0..self.w {
            addch(' ' as u32);
        }
        attrset(A_NORMAL);

        // Draw field titles with highlighting for sort column
        mv(y, self.x);
        let mut str = RichString::with_capacity(256);

        // Track if Command column has filter active (for padding color)
        let command_filter_active =
            filter_active && self.fields.contains(&ProcessField::Command);

        for field in &self.fields {
            // Get title - use dynamic title for PID/UID/PERCENT_CPU columns
            let title = if FieldWidths::is_pid_column(*field) 
                || *field == ProcessField::StUid 
                || FieldWidths::is_auto_width(*field) 
            {
                field_widths.get_title(*field)
            } else {
                field.title().to_string()
            };
            
            let is_sort_column = *field == active_sort_key;
            // Command column turns yellow when filter is active
            let is_filter_column = filter_active && *field == ProcessField::Command;

            // Determine base attribute for this column
            let base_attr = if is_filter_column {
                filter_attr
            } else {
                header_attr
            };

            // For the column text itself, use sort color if it's the sort column
            let text_attr = if is_sort_column {
                sort_attr
            } else {
                base_attr
            };

            // Split title into text and trailing spaces
            let trimmed = title.trim_end();
            let trailing_spaces = title.len() - trimmed.len();

            // Draw the column text (without trailing spaces)
            str.append(trimmed, text_attr);

            if is_sort_column {
                // Add sort indicator with sort color
                let indicator = if ascending {
                    crt.tree_str.asc
                } else {
                    crt.tree_str.desc
                };
                str.append(indicator, text_attr);

                // Add remaining trailing spaces (minus 1 for the indicator) with base color
                // If filter is active on this column, trailing spaces are yellow
                if trailing_spaces > 1 {
                    let spaces: String = " ".repeat(trailing_spaces - 1);
                    str.append(&spaces, base_attr);
                }
            } else {
                // Add trailing spaces with base color
                if trailing_spaces > 0 {
                    let spaces: String = " ".repeat(trailing_spaces);
                    str.append(&spaces, base_attr);
                }
            }

            // Add "(merged)" after Command field when showMergedCommand is enabled
            // (matches C htop Table.c:315-317)
            // Use base_attr so it's yellow if filter is active
            if *field == ProcessField::Command && settings.show_merged_command {
                str.append("(merged)", base_attr);
            }
        }

        // Use filter color for padding if Command column filter is active
        // This ensures the entire Command column area (which extends to screen edge) is yellow
        let pad_attr = if command_filter_active {
            Some(filter_attr)
        } else {
            None
        };
        str.write_at_width_with_pad_attr(y, self.x, self.w as usize, pad_attr);
    }

    /// Build header string for display (used when drawing header separately)
    pub fn build_header_string(
        &self,
        settings: &Settings,
        sort_key: ProcessField,
        sort_descending: bool,
    ) -> String {
        // Get active sort key and direction matching C htop's ScreenSettings_getActiveSortKey
        let screen = &settings.screens[settings.active_screen];
        let (active_sort_key, ascending) = if self.tree_view {
            if screen.tree_view_always_by_pid {
                // In tree view with always-by-PID: sort is by PID ascending
                (ProcessField::Pid, true)
            } else {
                // In tree view: use tree_sort_key
                (screen.tree_sort_key, screen.tree_direction > 0)
            }
        } else {
            // Not in tree view: use regular sort key
            (sort_key, !sort_descending)
        };

        let mut result = String::new();

        for field in &self.fields {
            let title = field.title();
            let is_sort_column = *field == active_sort_key;

            // For string version, we just use the title without indicator
            // (indicators are special characters that need proper rendering)
            if is_sort_column && title.ends_with(' ') {
                // Replace trailing space with sort indicator character
                result.push_str(&title[..title.len() - 1]);
                result.push(if ascending { '△' } else { '▽' });
            } else {
                result.push_str(title);
            }

            // Add "(merged)" after Command field when showMergedCommand is enabled
            // (matches C htop Table.c:315-317)
            if *field == ProcessField::Command && settings.show_merged_command {
                result.push_str("(merged)");
            }
        }

        result
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
        realtime_ms: u64,
        active_cpus: u32,
        field_widths: &FieldWidths,
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
                settings.show_merged_command,
                settings.highlight_deleted_exe,
                settings.shadow_dist_path_prefix,
                settings.find_comm_in_cmdline,
                settings.strip_exe_from_cmdline,
                is_shadowed,
                realtime_ms,
                active_cpus,
                field_widths,
            );
        }

        // Apply selection highlighting if selected
        // Priority order (lowest to highest): normal -> shadow -> tagged -> selected
        // This matches C htop Row_display behavior
        if selected {
            // For selected rows, use the override attribute method
            // This matches C htop's behavior where RichString_setAttr overrides all per-char colors
            str.write_at_width_with_attr(y, self.x, self.w as usize, selection_attr);
        } else if process.tagged {
            // For tagged rows, apply PROCESS_TAG color to entire row
            let tag_attr = crt.color(ColorElement::ProcessTag);
            str.write_at_width_with_attr(y, self.x, self.w as usize, tag_attr);
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
        _show_thread_names: bool,
        _show_merged_command: bool,
        highlight_deleted_exe: bool,
        shadow_dist_path_prefix: bool,
        _find_comm_in_cmdline: bool,
        _strip_exe_from_cmdline: bool,
        is_shadowed: bool,
        realtime_ms: u64,
        active_cpus: u32,
        field_widths: &FieldWidths,
    ) {
        let process_color = crt.color(ColorElement::Process);
        let shadow_color = crt.color(ColorElement::ProcessShadow);
        let basename_color = crt.color(ColorElement::ProcessBasename);

        // Colors for deleted exe highlighting are now stored in the highlights
        let _ = crt.color(ColorElement::FailedRead); // deleted_exe_color
        let _ = crt.color(ColorElement::ProcessTag); // deleted_lib_color

        // When is_shadowed is true, use shadow_color for all fields that would normally
        // use process_color (matches C htop behavior for shadow_other_users)
        let base_color = if is_shadowed {
            shadow_color
        } else {
            process_color
        };

        match field {
            ProcessField::Pid => {
                // PID: dynamic width based on max PID
                let width = field_widths.pid_digits;
                str.append(&format!("{:>width$} ", process.pid, width = width), base_color);
            }
            ProcessField::Ppid => {
                // PPID: dynamic width based on max PID
                let width = field_widths.pid_digits;
                str.append(&format!("{:>width$} ", process.ppid, width = width), base_color);
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
            #[cfg(target_os = "linux")]
            ProcessField::MShare => {
                // SHR: memory in KiB with coloring (disabled if shadowed)
                print_kbytes(str, process.m_share as u64, coloring && !is_shadowed, crt);
            }
            ProcessField::PercentCpu => {
                // CPU%: percentage with coloring (dynamic width)
                // When shadowed, use shadow color
                let width = field_widths.percent_cpu_width;
                if is_shadowed {
                    str.append(&format!("{:>width$.1} ", process.percent_cpu, width = width), shadow_color);
                } else {
                    print_percentage(str, process.percent_cpu, width, crt);
                }
            }
            ProcessField::PercentMem => {
                // MEM%: percentage with coloring (width 4, no autoWidth)
                // When shadowed, use shadow color
                if is_shadowed {
                    str.append(&format!("{:>4.1} ", process.percent_mem), shadow_color);
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
            ProcessField::Command => {
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

                // Use pre-computed merged command string if available
                // This matches C htop's Process_writeCommand() which uses the 
                // pre-computed mergedCommand.str and applies highlights
                let str_start = str.len();

                // Determine base colors
                let (attr, base_attr) = if is_shadowed {
                    (shadow_color, shadow_color)
                } else if process.is_thread() && highlight_threads {
                    (
                        crt.color(ColorElement::ProcessThread),
                        crt.color(ColorElement::ProcessThreadBasename),
                    )
                } else {
                    (process_color, basename_color)
                };

                // Get the command string to display
                let merged_command = &process.merged_command;
                if let Some(ref cmd_str) = merged_command.str_value {
                    // Append the pre-computed merged command string
                    str.append(cmd_str, attr);

                    // Apply highlights based on settings
                    for hl in &merged_command.highlights {
                        if hl.length == 0 {
                            continue;
                        }

                        // Check if this highlight type should be applied
                        let should_apply = if hl.flags & highlight_flags::SEPARATOR != 0 {
                            true // Always apply separator highlighting
                        } else if hl.flags & highlight_flags::BASENAME != 0 {
                            highlight_base_name
                        } else if hl.flags & highlight_flags::DELETED != 0 {
                            highlight_deleted_exe
                        } else if hl.flags & highlight_flags::PREFIXDIR != 0 {
                            shadow_dist_path_prefix
                        } else if hl.flags & highlight_flags::COMM != 0 {
                            true // Always apply comm highlighting when present
                        } else {
                            true
                        };

                        if should_apply {
                            let hl_start = str_start + hl.offset;
                            let hl_end = hl_start + hl.length;
                            str.set_attr(hl_start, hl_end, hl.attr);
                        }
                    }
                } else {
                    // Fallback: no pre-computed string, use simple cmdline display
                    // This handles cases where make_command_str wasn't called
                    let cmdline = process.cmdline.as_deref().unwrap_or("<unknown>");
                    let cmd = if show_program_path {
                        cmdline
                    } else {
                        &cmdline[process.cmdline_basename_start.min(cmdline.len())..]
                    };
                    str.append(cmd, attr);

                    // Apply basename highlighting if enabled
                    if highlight_base_name {
                        let basename_len = if process.cmdline_basename_end > process.cmdline_basename_start {
                            process.cmdline_basename_end - process.cmdline_basename_start
                        } else {
                            0
                        };
                        if basename_len > 0 {
                            let hl_offset = if show_program_path {
                                process.cmdline_basename_start
                            } else {
                                0
                            };
                            let hl_start = str_start + hl_offset;
                            let hl_end = hl_start + basename_len;
                            str.set_attr(hl_start, hl_end, base_attr);
                        }
                    }
                }

                str.append_char(' ', attr);
            }
            ProcessField::Tty => {
                // TTY: controlling terminal (8 chars + 1 space = 9 total)
                // C htop shows "(no tty) " for processes without a terminal
                if let Some(tty) = process.tty_name.as_deref() {
                    // Strip /dev/ prefix if present (matching C htop)
                    let name = tty.strip_prefix("/dev/").unwrap_or(tty);
                    let attr = if is_shadowed {
                        shadow_color
                    } else {
                        process_color
                    };
                    print_left_aligned(str, attr, name, 8);
                } else {
                    str.append("(no tty) ", shadow_color);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::IOPriority => {
                // IO priority: display class and data
                // Format: Bn (Best-effort level n), Rn (Realtime level n), id (Idle)
                // When IOPRIO_CLASS_NONE, derive from nice value: B((nice+20)/5)
                let ioprio = process.io_priority;
                if ioprio < 0 {
                    // Could not read IO priority (permission denied or error)
                    str.append("?? ", shadow_color);
                } else {
                    let klass = ioprio_class(ioprio);
                    if klass == IOPRIO_CLASS_NONE {
                        // No explicit IO priority set - derive from nice
                        // See note in C htop: when NONE, the kernel uses
                        // (nice+20)/5 to derive the best-effort level
                        let derived = (process.nice + 20) / 5;
                        str.append(&format!("B{} ", derived), base_color);
                    } else if klass == IOPRIO_CLASS_BE {
                        // Best-effort class
                        let data = ioprio_data(ioprio);
                        str.append(&format!("B{} ", data), base_color);
                    } else if klass == IOPRIO_CLASS_RT {
                        // Realtime class - high priority color
                        let data = ioprio_data(ioprio);
                        let attr = if is_shadowed {
                            shadow_color
                        } else {
                            crt.color(ColorElement::ProcessHighPriority)
                        };
                        str.append(&format!("R{} ", data), attr);
                    } else if klass == IOPRIO_CLASS_IDLE {
                        // Idle class - low priority color
                        let attr = if is_shadowed {
                            shadow_color
                        } else {
                            crt.color(ColorElement::ProcessLowPriority)
                        };
                        str.append("id ", attr);
                    } else {
                        // Unknown class
                        str.append("?? ", shadow_color);
                    }
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::IORate => {
                // Total I/O rate (read + write combined)
                let total_rate = process.io_read_rate + process.io_write_rate;
                print_rate(str, total_rate, coloring && !is_shadowed, crt);
            }
            #[cfg(target_os = "linux")]
            ProcessField::IOReadRate => {
                // I/O read rate in bytes per second
                print_rate(str, process.io_read_rate, coloring && !is_shadowed, crt);
            }
            #[cfg(target_os = "linux")]
            ProcessField::IOWriteRate => {
                // I/O write rate in bytes per second
                print_rate(str, process.io_write_rate, coloring && !is_shadowed, crt);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Rbytes => {
                // Total bytes read
                print_kbytes(str, process.io_read_bytes, coloring && !is_shadowed, crt);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Wbytes => {
                // Total bytes written
                print_kbytes(str, process.io_write_bytes, coloring && !is_shadowed, crt);
            }
            #[cfg(target_os = "linux")]
            ProcessField::PercentIODelay => {
                // Block I/O delay percentage (Linux delay accounting)
                if is_shadowed {
                    str.append(
                        &format!("{:>4.1} ", process.blkio_delay_percent),
                        shadow_color,
                    );
                } else {
                    print_percentage(str, process.blkio_delay_percent, 5, crt);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::PercentSwapDelay => {
                // Swapin delay percentage (Linux delay accounting)
                if is_shadowed {
                    str.append(
                        &format!("{:>4.1} ", process.swapin_delay_percent),
                        shadow_color,
                    );
                } else {
                    print_percentage(str, process.swapin_delay_percent, 5, crt);
                }
            }
            
            // === Stub implementations for unimplemented common fields ===
            ProcessField::Pgrp => {
                // PGRP: process group ID (dynamic width like PID)
                let width = field_widths.pid_digits;
                str.append(&format!("{:>width$} ", process.pgrp, width = width), base_color);
            }
            ProcessField::Session => {
                // SID: session ID (dynamic width like PID)
                let width = field_widths.pid_digits;
                str.append(&format!("{:>width$} ", process.session, width = width), base_color);
            }
            ProcessField::Tpgid => {
                // TPGID: terminal process group ID (dynamic width like PID)
                let width = field_widths.pid_digits;
                str.append(&format!("{:>width$} ", process.tpgid, width = width), base_color);
            }
            ProcessField::Minflt => {
                // MINFLT: minor faults (11 chars)
                str.append(&format!("{:>11} ", process.minflt), base_color);
            }
            ProcessField::Majflt => {
                // MAJFLT: major faults (11 chars)
                str.append(&format!("{:>11} ", process.majflt), base_color);
            }
            ProcessField::Starttime => {
                // START: start time (6 chars total matching C htop title "START ")
                // Format based on how long ago the process started:
                // - < 24 hours: "HH:MM " (time today) - %R format
                // - < 365 days: "MmmDD " (month + day, e.g., "Jan23 ") - %b%d format
                // - >= 365 days: " YYYY " (year, e.g., " 2024 ") - %Y format with leading space
                use chrono::{Local, TimeZone, Datelike, Timelike};
                
                let now = realtime_ms / 1000; // current time in seconds
                let start = process.starttime_ctime;
                
                if start <= 0 {
                    str.append("  N/A ", shadow_color);
                } else {
                    let age_seconds = (now as i64).saturating_sub(start);
                    
                    if let Some(dt) = Local.timestamp_opt(start, 0).single() {
                        let formatted = if age_seconds < 86400 {
                            // Started within last 24 hours: show time "HH:MM "
                            format!("{:02}:{:02} ", dt.hour(), dt.minute())
                        } else if age_seconds < 364 * 86400 {
                            // Started within last ~year: show "MmmDD "
                            let month = match dt.month() {
                                1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
                                5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
                                9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
                                _ => "???",
                            };
                            format!("{}{:02} ", month, dt.day())
                        } else {
                            // Started more than a year ago: show " YYYY "
                            format!(" {} ", dt.year())
                        };
                        str.append(&formatted, base_color);
                    } else {
                        str.append("  N/A ", shadow_color);
                    }
                }
            }
            ProcessField::StUid => {
                // UID: user ID (dynamic width)
                let width = field_widths.uid_digits;
                str.append(&format!("{:>width$} ", process.uid, width = width), base_color);
            }
            ProcessField::Tgid => {
                // TGID: thread group ID (dynamic width like PID)
                // For userland threads, tgid == ppid (the main process)
                // For main processes, tgid == pid
                let tgid = if process.is_userland_thread {
                    process.ppid
                } else {
                    process.pid
                };
                let width = field_widths.pid_digits;
                str.append(&format!("{:>width$} ", tgid, width = width), base_color);
            }
            ProcessField::PercentNormCpu => {
                // NCPU%: normalized CPU percentage (dynamic width)
                // This is percent_cpu divided by the number of active CPUs
                let norm_cpu = if active_cpus > 0 {
                    process.percent_cpu / active_cpus as f32
                } else {
                    process.percent_cpu
                };
                
                let width = field_widths.percent_norm_cpu_width;
                if is_shadowed {
                    str.append(&format!("{:>width$.1} ", norm_cpu, width = width), shadow_color);
                } else {
                    print_percentage(str, norm_cpu, width, crt);
                }
            }
            ProcessField::Elapsed => {
                // ELAPSED: time since process started (9 chars)
                // Uses the same Row_printTime format as TIME field
                let now_ms = realtime_ms;
                let start_ms = (process.starttime_ctime as u64).saturating_mul(1000);
                let elapsed_ms = if now_ms > start_ms {
                    now_ms - start_ms
                } else {
                    0
                };
                // Convert ms to hundredths of a second for print_time
                let elapsed_hundredths = elapsed_ms / 10;
                
                if is_shadowed {
                    // For shadowed rows, use shadow color version
                    let formatted = Process::format_time(elapsed_hundredths);
                    str.append(&formatted, shadow_color);
                } else {
                    print_time(str, elapsed_hundredths, coloring, crt);
                }
            }
            ProcessField::SchedulerPolicy => {
                // SCHED: scheduler policy (6 chars)
                // Linux scheduler policies from sched.h
                let policy_str = match process.scheduling_policy {
                    0 => "OTHER",  // SCHED_OTHER / SCHED_NORMAL
                    1 => "FIFO",   // SCHED_FIFO
                    2 => "RR",     // SCHED_RR
                    3 => "BATCH",  // SCHED_BATCH
                    5 => "IDLE",   // SCHED_IDLE
                    6 => "EDF",    // SCHED_DEADLINE
                    -1 => "N/A",   // Not available
                    _ => "???",    // Unknown
                };
                str.append(&format!("{:<5} ", policy_str), base_color);
            }
            ProcessField::ProcComm => {
                // COMM: process name from /proc/[pid]/comm (15 chars + space = 16 total)
                // Matches C htop's TASK_COMM_LEN - 1 = 15
                let comm = process.comm.as_deref().unwrap_or("?");
                print_left_aligned(str, base_color, comm, 15);
            }
            ProcessField::ProcExe => {
                // EXE: executable basename (15 chars + space)
                // Shows the basename of the executable (not full path), matching C htop
                if let Some(exe) = process.exe.as_deref() {
                    // Get basename (after last '/')
                    let basename = exe.rsplit('/').next().unwrap_or(exe);
                    // Use ProcessBasename color for the executable name
                    let exe_color = crt.color(ColorElement::ProcessBasename);
                    print_left_aligned(str, exe_color, basename, 15);
                } else if process.is_kernel_thread {
                    // Kernel threads show as "KERNEL" or similar
                    print_left_aligned(str, shadow_color, "kernel", 15);
                } else {
                    print_left_aligned(str, shadow_color, "N/A", 15);
                }
            }
            ProcessField::Cwd => {
                // CWD: current working directory (25 chars + space), matching C htop
                if let Some(cwd) = process.cwd.as_deref() {
                    // Check for deleted main thread case
                    if cwd.starts_with("/proc/") && cwd.contains(" (deleted)") {
                        print_left_aligned(str, shadow_color, "main thread terminated", 25);
                    } else {
                        print_left_aligned(str, base_color, cwd, 25);
                    }
                } else {
                    print_left_aligned(str, shadow_color, "N/A", 25);
                }
            }
            
            // === Linux-specific fields ===
            #[cfg(target_os = "linux")]
            ProcessField::Cminflt => {
                // CMINFLT: children's minor faults (11 chars)
                str.append(&format!("{:>11} ", process.cminflt), base_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Cmajflt => {
                // CMAJFLT: children's major faults (11 chars)
                str.append(&format!("{:>11} ", process.cmajflt), base_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Utime => {
                // UTIME+: user time (9 chars)
                if is_shadowed {
                    let formatted = Process::format_time(process.utime);
                    str.append(&formatted, shadow_color);
                } else {
                    print_time(str, process.utime, coloring, crt);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::Stime => {
                // STIME+: system time (9 chars)
                if is_shadowed {
                    let formatted = Process::format_time(process.stime);
                    str.append(&formatted, shadow_color);
                } else {
                    print_time(str, process.stime, coloring, crt);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::Cutime => {
                // CUTIME+: children's user time (9 chars)
                if is_shadowed {
                    let formatted = Process::format_time(process.cutime);
                    str.append(&formatted, shadow_color);
                } else {
                    print_time(str, process.cutime, coloring, crt);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::Cstime => {
                // CSTIME+: children's system time (9 chars)
                if is_shadowed {
                    let formatted = Process::format_time(process.cstime);
                    str.append(&formatted, shadow_color);
                } else {
                    print_time(str, process.cstime, coloring, crt);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::MText => {
                // CODE: code size (6 chars) - m_text is in KB
                if is_shadowed {
                    str.append(&format!("{:>5} ", process.m_text), shadow_color);
                } else {
                    print_kbytes(str, process.m_text as u64, coloring, crt);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::MData => {
                // DATA: data size (6 chars) - m_data is in KB
                if is_shadowed {
                    str.append(&format!("{:>5} ", process.m_data), shadow_color);
                } else {
                    print_kbytes(str, process.m_data as u64, coloring, crt);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::MLib => {
                // LIB: library size (6 chars) - m_lib is in KB
                // Note: m_lib may be 0 if not computed (requires reading /proc/PID/maps)
                if process.m_lib == 0 {
                    str.append("  N/A ", shadow_color);
                } else if is_shadowed {
                    str.append(&format!("{:>5} ", process.m_lib), shadow_color);
                } else {
                    print_kbytes(str, process.m_lib as u64, coloring, crt);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::Rchar => {
                // RCHAR: chars read (6 chars) - requires /proc/[pid]/io
                str.append("  N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Wchar => {
                // WCHAR: chars written (6 chars) - requires /proc/[pid]/io
                str.append("  N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Syscr => {
                // READ_SYSC: read syscalls (12 chars) - requires /proc/[pid]/io
                str.append("         N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Syscw => {
                // WRITE_SYSC: write syscalls (12 chars) - requires /proc/[pid]/io
                str.append("         N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Cnclwb => {
                // IO_C: cancelled write bytes (6 chars) - requires /proc/[pid]/io
                str.append("  N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::CGroup => {
                // CGROUP: cgroup path (variable, use 20 chars)
                let cgroup = process.cgroup.as_deref().unwrap_or("?");
                print_left_aligned(str, base_color, cgroup, 20);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Oom => {
                // OOM: OOM score (5 chars)
                str.append(&format!("{:>5} ", process.oom_score), base_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::PercentCpuDelay => {
                // CPUD%: CPU delay percentage (6 chars) - requires taskstats
                str.append("  N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::MPss => {
                // PSS: proportional set size (6 chars) - requires /proc/[pid]/smaps
                str.append("  N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::MSwap => {
                // SWAP: swap size (6 chars) - requires /proc/[pid]/smaps
                str.append("  N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::MPsswp => {
                // PSSWP: PSS + swap (7 chars) - requires /proc/[pid]/smaps
                str.append("   N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Ctxt => {
                // CTXT: context switches (6 chars)
                // We have ctxt_switches in process struct
                if process.ctxt_switches == 0 {
                    str.append("     0 ", shadow_color);
                } else {
                    str.append(&format!("{:>6} ", process.ctxt_switches), base_color);
                }
            }
            #[cfg(target_os = "linux")]
            ProcessField::SecAttr => {
                // Security Attribute (18 chars)
                let sec_attr = process.sec_attr.as_deref().unwrap_or("?");
                print_left_aligned(str, base_color, sec_attr, 18);
            }
            #[cfg(target_os = "linux")]
            ProcessField::AutogroupId => {
                // AGRP: autogroup ID (4 chars) - requires /proc/[pid]/autogroup
                str.append(" N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::AutogroupNice => {
                // ANI: autogroup nice (4 chars) - requires /proc/[pid]/autogroup
                str.append(" N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::CCGroup => {
                // CGROUP (compressed) (19 chars)
                // Show compressed version of cgroup (last component)
                let cgroup = process.cgroup.as_deref().unwrap_or("?");
                // Get last component of cgroup path for compressed display
                let compressed = cgroup.rsplit('/').next().unwrap_or(cgroup);
                print_left_aligned(str, base_color, compressed, 19);
            }
            #[cfg(target_os = "linux")]
            ProcessField::Container => {
                // CONTAINER (9 chars) - requires container detection heuristics
                str.append("     N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::MPriv => {
                // PRIV: private memory (6 chars) - requires /proc/[pid]/smaps
                str.append("  N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::GpuTime => {
                // GPU_TIME (9 chars) - requires DRM/GPU support
                str.append("     N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::GpuPercent => {
                // GPU% (6 chars) - requires DRM/GPU support
                str.append("  N/A ", shadow_color);
            }
            #[cfg(target_os = "linux")]
            ProcessField::IsContainer => {
                // CONT: is container (5 chars) - requires container detection
                str.append("  N/A", shadow_color);
            }
            
            // === macOS-specific fields ===
            #[cfg(target_os = "macos")]
            ProcessField::Translated => {
                // T: translated process (Rosetta 2) (2 chars)
                // Show N for native, T for translated (Rosetta), - for unknown
                // For now, we don't have this info, so show "-"
                str.append("- ", shadow_color);
            }
            
            #[allow(unreachable_patterns)]
            _ => {
                // Fallback for any truly unhandled field
                str.append("N/A ", shadow_color);
            }
        }
    }

    /// Draw the panel
    pub fn draw(&mut self, crt: &Crt, machine: &Machine, settings: &Settings) {
        let visible_height = if self.show_header { self.h - 1 } else { self.h };
        let start_y = if self.show_header {
            // Show yellow header when filter is active OR search bar is open with text
            let search_active = self.inc_search.is_search() && !self.inc_search.text.is_empty();
            self.draw_header(
                crt,
                self.y,
                settings,
                machine.sort_key,
                machine.sort_descending,
                self.filter.is_some() || search_active,
                &machine.field_widths,
            );
            self.y + 1
        } else {
            self.y
        };

        // Rebuild display list if invalid (data changed, filter changed, etc.)
        if !self.display_list_valid {
            self.rebuild_display_list(machine, settings);
        }

        let process_count = self.cached_display_indices.len() as i32;
        self.ensure_visible(process_count);

        // Get current user ID for highlighting
        let current_uid = machine.htop_user_id;
        let realtime_ms = machine.realtime_ms;
        let active_cpus = machine.active_cpus;

        // Partial vs full redraw logic - matches C htop Panel_draw() exactly
        //
        // C htop Panel_draw() has two paths:
        // 1. if (needsRedraw || force_redraw) → full redraw of all visible rows
        // 2. else → partial redraw of just old and new selected rows
        //
        // The key insight is that C htop ALWAYS does a partial redraw when
        // needsRedraw is false, even if selection didn't change. This is because
        // the old_selected/selected values are updated AFTER draw, so on the next
        // draw with no input, old_selected == selected and it just redraws that
        // one row (which is a no-op visually but keeps things consistent).
        //
        // For scroll changes, needsRedraw is set by PANEL_SCROLL macro in C htop,
        // so we don't need special handling here.

        if self.needs_redraw {
            // Full redraw: draw all visible rows
            for i in 0..visible_height {
                let display_idx = (self.scroll_v + i) as usize;
                let y = start_y + i;

                if display_idx < self.cached_display_indices.len() {
                    let process_idx = self.cached_display_indices[display_idx];
                    if let Some(process) = machine.processes.processes.get(process_idx) {
                        let selected = display_idx as i32 == self.selected;
                        self.draw_process(crt, y, process, selected, settings, current_uid, realtime_ms, active_cpus, &machine.field_widths);
                    }
                } else {
                    // Empty line
                    mv(y, self.x);
                    for _ in 0..self.w {
                        addch(' ' as u32);
                    }
                }
            }
        } else {
            // Partial redraw: only redraw old and new selected rows
            // This matches C htop Panel_draw() lines 309-332
            //
            // Even if old_selected == selected, we still redraw that row to handle
            // any data changes (like CPU% updates) for the selected process.
            
            let old_in_range = self.old_selected >= self.scroll_v
                && self.old_selected < self.scroll_v + visible_height;
            let new_in_range =
                self.selected >= self.scroll_v && self.selected < self.scroll_v + visible_height;

            // Redraw old selected row (remove highlight) if in range and different from new
            if old_in_range && self.old_selected != self.selected {
                let old_row = (self.old_selected - self.scroll_v) as usize;
                let old_y = start_y + old_row as i32;
                if old_row < self.cached_display_indices.len() {
                    let process_idx = self.cached_display_indices[old_row];
                    if let Some(process) = machine.processes.processes.get(process_idx) {
                        self.draw_process(crt, old_y, process, false, settings, current_uid, realtime_ms, active_cpus, &machine.field_widths);
                    }
                }
            }

            // Redraw new selected row (add highlight) if in range
            if new_in_range {
                let new_row = (self.selected - self.scroll_v) as usize;
                let new_y = start_y + new_row as i32;
                if new_row < self.cached_display_indices.len() {
                    let process_idx = self.cached_display_indices[new_row];
                    if let Some(process) = machine.processes.processes.get(process_idx) {
                        self.draw_process(crt, new_y, process, true, settings, current_uid, realtime_ms, active_cpus, &machine.field_widths);
                    }
                }
            }
        }

        // Update tracking state for next draw (matches C htop Panel_draw lines 341-343)
        self.old_selected = self.selected;
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
                // Up or Ctrl+P - does NOT set needsRedraw (matches C htop)
                self.move_selection(-1, machine);
                HandlerResult::Handled
            }
            KEY_DOWN | 0x0E => {
                // Down or Ctrl+N - does NOT set needsRedraw (matches C htop)
                self.move_selection(1, machine);
                HandlerResult::Handled
            }
            KEY_PPAGE => {
                // Page Up - uses PANEL_SCROLL macro in C htop which sets needsRedraw
                let visible_height = if self.show_header { self.h - 1 } else { self.h };
                self.scroll_wheel(-visible_height, machine);
                HandlerResult::Handled
            }
            KEY_NPAGE => {
                // Page Down - uses PANEL_SCROLL macro in C htop which sets needsRedraw
                let visible_height = if self.show_header { self.h - 1 } else { self.h };
                self.scroll_wheel(visible_height, machine);
                HandlerResult::Handled
            }
            KEY_HOME => {
                // Home - does NOT set needsRedraw in C htop
                self.selected = 0;
                self.scroll_v = 0;
                HandlerResult::Handled
            }
            KEY_END => {
                // End - does NOT set needsRedraw in C htop
                let count = self.get_visible_count(machine);
                self.selected = (count - 1).max(0);
                self.ensure_visible(count);
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
                    self.invalidate_display_list();
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
                    self.invalidate_display_list();
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
                    self.invalidate_display_list();
                }
                self.do_incremental_search(machine);
                HandlerResult::Handled
            }
            ch if (0x20..0x7F).contains(&ch) => {
                self.inc_search.add_char(ch as u8 as char);
                if is_filter {
                    // Update filter in real-time
                    self.filter = Some(self.inc_search.text.clone());
                    self.invalidate_display_list();
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
                    self.invalidate_display_list();
                    // Keep yellow "following" selection when filter is active
                    if self.filter.is_some() {
                        self.following = true;
                        self.selection_color = ColorElement::PanelSelectionFollow;
                    } else {
                        self.following = false;
                        self.selection_color = ColorElement::PanelSelectionFocus;
                    }
                }
                self.inc_search.stop();
                // Reset if no filter/search active
                if self.filter.is_none() && !is_search {
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
                // Use yellow "following" selection for search mode
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
                // Use yellow "following" selection for both search and filter modes
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
    /// Sets needs_redraw = true because the viewport scrolled
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

        // C htop PANEL_SCROLL macro sets needsRedraw = true
        // because scrolling changes the viewport
        self.needs_redraw = true;
    }

    /// Get count of visible processes
    /// Uses cached display list which includes all filtering
    fn get_visible_count(&self, _machine: &Machine) -> i32 {
        self.cached_display_indices.len() as i32
    }

    /// Get the currently selected process
    pub fn get_selected_process<'a>(&self, machine: &'a Machine) -> Option<&'a Process> {
        // Use cached display indices if available
        if self.display_list_valid && !self.cached_display_indices.is_empty() {
            let display_idx = self.selected as usize;
            if let Some(&process_idx) = self.cached_display_indices.get(display_idx) {
                return machine.processes.processes.get(process_idx);
            }
        }
        
        // Fallback: use old method
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

    /// Try to select a process by PID, returns true if found and selected
    /// If not found, does not change selection
    pub fn try_select_pid(
        &mut self,
        pid: i32,
        machine: &Machine,
        settings: &Settings,
    ) -> bool {
        let processes: Vec<&Process> = if self.tree_view {
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

        // Find the process with the given PID
        for (i, p) in processes.iter().enumerate() {
            if p.pid == pid {
                self.selected = i as i32;
                self.ensure_visible(processes.len() as i32);
                return true;
            }
        }
        false
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
