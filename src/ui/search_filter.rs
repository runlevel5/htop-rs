//! Search/Filter helpers for popup screens
//!
//! This module provides reusable search and filter functionality
//! for screens like process environment, lsof, strace, etc.

#![allow(dead_code)]

use super::crt::{
    ColorElement, A_NORMAL, KEY_BACKSLASH, KEY_BACKSPACE, KEY_ENTER, KEY_ESC, KEY_F3, KEY_F4,
    KEY_F5, KEY_MOUSE, KEY_SF3, KEY_SLASH,
};
use super::function_bar::FunctionBar;
use super::Crt;

/// Search/filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFilterMode {
    None,
    Search,
    Filter,
}

/// Search and filter state for popup screens
#[derive(Debug, Clone)]
pub struct SearchFilterState {
    pub mode: SearchFilterMode,
    pub search_text: String,
    pub filter_text: String,
}

impl Default for SearchFilterState {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchFilterState {
    pub fn new() -> Self {
        SearchFilterState {
            mode: SearchFilterMode::None,
            search_text: String::new(),
            filter_text: String::new(),
        }
    }

    /// Check if currently in search or filter mode
    pub fn is_active(&self) -> bool {
        self.mode != SearchFilterMode::None
    }

    /// Check if in search mode
    pub fn is_search(&self) -> bool {
        self.mode == SearchFilterMode::Search
    }

    /// Check if in filter mode
    pub fn is_filter(&self) -> bool {
        self.mode == SearchFilterMode::Filter
    }

    /// Start search mode
    pub fn start_search(&mut self) {
        self.mode = SearchFilterMode::Search;
        self.search_text.clear();
    }

    /// Start filter mode
    pub fn start_filter(&mut self) {
        self.mode = SearchFilterMode::Filter;
    }

    /// Cancel current mode (clears search text on cancel)
    pub fn cancel(&mut self) {
        if self.is_search() {
            self.search_text.clear();
        }
        self.mode = SearchFilterMode::None;
    }

    /// Confirm and exit current mode
    pub fn confirm(&mut self) {
        self.mode = SearchFilterMode::None;
    }

    /// Get the current text being edited
    pub fn current_text(&self) -> &str {
        match self.mode {
            SearchFilterMode::Search => &self.search_text,
            SearchFilterMode::Filter => &self.filter_text,
            SearchFilterMode::None => "",
        }
    }

    /// Get mutable reference to current text
    pub fn current_text_mut(&mut self) -> &mut String {
        match self.mode {
            SearchFilterMode::Search => &mut self.search_text,
            SearchFilterMode::Filter => &mut self.filter_text,
            SearchFilterMode::None => &mut self.search_text, // fallback, shouldn't happen
        }
    }

    /// Add a character to the current text
    pub fn add_char(&mut self, c: char) {
        self.current_text_mut().push(c);
    }

    /// Remove last character from current text
    pub fn backspace(&mut self) {
        self.current_text_mut().pop();
    }

    /// Check if filter is active (has text)
    pub fn has_filter(&self) -> bool {
        !self.filter_text.is_empty()
    }

    /// Clear filter
    pub fn clear_filter(&mut self) {
        self.filter_text.clear();
    }

    /// Filter a list of strings, returning indices of matching items
    pub fn filter_indices(&self, items: &[String]) -> Vec<usize> {
        if self.filter_text.is_empty() {
            (0..items.len()).collect()
        } else {
            let filter_lower = self.filter_text.to_lowercase();
            items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.to_lowercase().contains(&filter_lower))
                .map(|(i, _)| i)
                .collect()
        }
    }

    /// Find the first item matching search text, starting from given position
    /// Returns the index in filtered_indices if found
    pub fn find_first_match(&self, items: &[String], filtered_indices: &[usize]) -> Option<usize> {
        if self.search_text.is_empty() {
            return None;
        }
        let search_lower = self.search_text.to_lowercase();
        for (i, &idx) in filtered_indices.iter().enumerate() {
            if let Some(item) = items.get(idx) {
                if item.to_lowercase().contains(&search_lower) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Find next match after current position (wraps around)
    /// Returns the index in filtered_indices if found
    pub fn find_next_match(
        &self,
        items: &[String],
        filtered_indices: &[usize],
        current: usize,
    ) -> Option<usize> {
        if self.search_text.is_empty() || filtered_indices.is_empty() {
            return None;
        }
        let search_lower = self.search_text.to_lowercase();
        let len = filtered_indices.len();

        // Search from current+1 to end, then from 0 to current
        for offset in 1..=len {
            let i = (current + offset) % len;
            if let Some(&idx) = filtered_indices.get(i) {
                if let Some(item) = items.get(idx) {
                    if item.to_lowercase().contains(&search_lower) {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    /// Find previous match before current position (wraps around)
    /// Returns the index in filtered_indices if found
    pub fn find_prev_match(
        &self,
        items: &[String],
        filtered_indices: &[usize],
        current: usize,
    ) -> Option<usize> {
        if self.search_text.is_empty() || filtered_indices.is_empty() {
            return None;
        }
        let search_lower = self.search_text.to_lowercase();
        let len = filtered_indices.len();

        // Search from current-1 backwards, wrapping around
        for offset in 1..=len {
            let i = (current + len - offset) % len;
            if let Some(&idx) = filtered_indices.get(i) {
                if let Some(item) = items.get(idx) {
                    if item.to_lowercase().contains(&search_lower) {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    /// Draw the search/filter bar or function bar
    /// Returns true if search/filter bar was drawn
    pub fn draw_bar(&self, crt: &mut Crt, y: i32, screen_width: i32) {
        if self.is_active() {
            self.draw_search_filter_bar(crt, y, screen_width);
        } else {
            self.draw_function_bar(crt, y);
        }
    }

    /// Draw the search/filter input bar
    fn draw_search_filter_bar(&self, crt: &mut Crt, y: i32, screen_width: i32) {
        let bar_attr = crt.color(ColorElement::FunctionBar);
        let key_attr = crt.color(ColorElement::FunctionKey);

        crt.mv(y, 0);
        crt.attrset(bar_attr);
        crt.hline(y, 0, ' ' as u32, screen_width);
        crt.attrset(A_NORMAL);
        crt.mv(y, 0);

        if self.is_search() {
            // F3: Next  S-F3: Prev  Esc: Cancel  Search: <input>
            crt.attrset(key_attr);
            crt.addstr_raw("F3");
            crt.attrset(bar_attr);
            crt.addstr_raw("Next  ");
            crt.attrset(key_attr);
            crt.addstr_raw("S-F3");
            crt.attrset(bar_attr);
            crt.addstr_raw("Prev  ");
            crt.attrset(key_attr);
            crt.addstr_raw("Esc");
            crt.attrset(bar_attr);
            crt.addstr_raw("Cancel");
            crt.attrset(key_attr);
            crt.addstr_raw("Search: ");
            crt.attrset(bar_attr);
            crt.addstr_raw(&self.search_text);
            crt.attrset(A_NORMAL);
        } else {
            // Enter: Done  Esc: Clear  Filter: <input>
            crt.attrset(key_attr);
            crt.addstr_raw("Enter");
            crt.attrset(bar_attr);
            crt.addstr_raw("Done  ");
            crt.attrset(key_attr);
            crt.addstr_raw("Esc");
            crt.attrset(bar_attr);
            crt.addstr_raw("Clear ");
            crt.attrset(key_attr);
            crt.addstr_raw("Filter: ");
            crt.attrset(bar_attr);
            crt.addstr_raw(&self.filter_text);
            crt.attrset(A_NORMAL);
        }
    }

    /// Draw the standard function bar
    fn draw_function_bar(&self, crt: &mut Crt, y: i32) {
        let f4_label = if self.has_filter() {
            "FILTER"
        } else {
            "Filter"
        };
        let fb = FunctionBar::with_functions(vec![
            ("F3".to_string(), "Search".to_string()),
            ("F4".to_string(), f4_label.to_string()),
            ("F5".to_string(), "Refresh".to_string()),
            ("Esc".to_string(), "Done  ".to_string()),
        ]);
        fb.draw_simple(crt, y);
    }

    /// Handle a key press while in search/filter mode
    /// Returns HandleResult indicating what happened
    pub fn handle_input(&mut self, ch: i32) -> HandleResult {
        if !self.is_active() {
            return HandleResult::NotHandled;
        }

        match ch {
            27 => {
                // Escape - cancel
                self.cancel();
                HandleResult::Handled
            }
            10 | KEY_ENTER => {
                // Enter - confirm
                self.confirm();
                HandleResult::Handled
            }
            KEY_BACKSPACE | 127 | 8 => {
                self.backspace();
                if self.is_filter() {
                    HandleResult::FilterChanged
                } else {
                    HandleResult::SearchChanged
                }
            }
            x if x == KEY_F3 && self.is_search() => {
                // F3 in search mode - find next
                HandleResult::SearchNext
            }
            x if x == KEY_SF3 && self.is_search() => {
                // Shift-F3 in search mode - find prev
                HandleResult::SearchPrev
            }
            _ if (32..127).contains(&ch) => {
                let c = char::from_u32(ch as u32).unwrap_or(' ');
                self.add_char(c);
                if self.is_filter() {
                    HandleResult::FilterChanged
                } else {
                    HandleResult::SearchChanged
                }
            }
            _ => HandleResult::Handled,
        }
    }

    /// Handle F3/F4 or '/'/'\\' key to start search/filter
    /// Returns true if handled
    pub fn handle_start_key(&mut self, ch: i32) -> bool {
        match ch {
            x if x == KEY_F3 => {
                self.start_search();
                true
            }
            KEY_SLASH => {
                // '/' - search
                self.start_search();
                true
            }
            x if x == KEY_F4 => {
                self.start_filter();
                true
            }
            KEY_BACKSLASH => {
                // '\' - filter
                self.start_filter();
                true
            }
            _ => false,
        }
    }

    /// Translate mouse click on function bar to key code
    /// Should be called when mouse is clicked on function bar row and not in search/filter mode
    /// Returns the translated key code or 0 if not a recognized button
    pub fn translate_function_bar_click(&self, x: i32) -> i32 {
        if self.is_active() {
            return 0;
        }
        // Standard function bar: F3 Search, F4 Filter, F5 Refresh, Esc Done
        // Approximate positions based on label widths
        if x < 10 {
            KEY_F3
        } else if x < 20 {
            KEY_F4
        } else if x < 30 {
            KEY_F5
        } else {
            KEY_ESC
        }
    }
}

/// Result of handling search/filter input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleResult {
    /// Input was not handled (not in search/filter mode)
    NotHandled,
    /// Input was handled, no data change
    Handled,
    /// Search text changed - caller should update selection to first match
    SearchChanged,
    /// Filter text changed - caller should refilter and reset selection
    FilterChanged,
    /// F3 pressed - find next match
    SearchNext,
    /// S-F3 pressed - find previous match
    SearchPrev,
}

/// Process mouse event for search/filter screen
/// Returns the translated key code
pub fn process_mouse_event(crt: &mut Crt, state: &SearchFilterState, ch: i32) -> i32 {
    if ch != KEY_MOUSE {
        return ch;
    }

    let screen_height = crt.height();
    if let Some(event) = crt.get_mouse_event() {
        if event.is_left_click() && event.y == screen_height - 1 {
            // Click on function bar
            return state.translate_function_bar_click(event.x);
        } else if event.is_wheel_up() {
            return super::crt::KEY_WHEELUP;
        } else if event.is_wheel_down() {
            return super::crt::KEY_WHEELDOWN;
        }
    }
    ch
}
