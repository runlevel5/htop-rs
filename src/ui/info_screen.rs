//! InfoScreen - Generic info screen for displaying lists of lines
//!
//! Provides a reusable screen component for displaying scrollable lists with:
//! - Title bar
//! - Optional header row
//! - Selectable line list with scrolling
//! - Search/filter support
//! - Standard navigation keys

use super::crt::{
    ColorElement, Crt, A_NORMAL, KEY_CTRL_L, KEY_CTRL_N, KEY_CTRL_P, KEY_DOWN, KEY_END, KEY_F10,
    KEY_F5, KEY_HOME, KEY_NPAGE, KEY_PPAGE, KEY_UP, KEY_WHEELDOWN, KEY_WHEELUP,
};
use super::search_filter::{self, HandleResult, SearchFilterState};

/// Configuration for an info screen
pub struct InfoScreenConfig<'a> {
    /// Title format string (will be displayed at row 0)
    pub title: String,
    /// Optional header row (displayed at row 1)
    pub header: Option<&'a str>,
    /// Whether to use needs_redraw optimization (for screens that may not change often)
    pub use_redraw_optimization: bool,
}

/// Run an info screen with the given configuration
///
/// # Arguments
/// * `crt` - Terminal handle
/// * `config` - Screen configuration
/// * `lines` - Initial lines to display (mutable for refresh support)
/// * `refresh_fn` - Optional callback for F5 refresh, returns new lines
pub fn run_info_screen<F>(
    crt: &mut Crt,
    config: &InfoScreenConfig,
    lines: &mut Vec<String>,
    refresh_fn: Option<F>,
) where
    F: Fn() -> Vec<String>,
{
    let mut selected = 0i32;
    let mut scroll_v = 0i32;
    let mut sf_state = SearchFilterState::new();
    let mut needs_redraw = true;

    let has_header = config.header.is_some();
    let panel_y = if has_header { 2 } else { 1 };
    let height_offset = if has_header { 3 } else { 2 }; // Title + header? + function bar

    loop {
        let filtered_indices = sf_state.filter_indices(lines);
        let panel_height = crt.height() - height_offset;

        // Clamp selection and scroll
        let max_selected = (filtered_indices.len() as i32 - 1).max(0);
        selected = selected.clamp(0, max_selected);

        if selected < scroll_v {
            scroll_v = selected;
        } else if selected >= scroll_v + panel_height {
            scroll_v = selected - panel_height + 1;
        }
        // Clamp scroll_v to valid range
        let max_scroll = (filtered_indices.len() as i32 - panel_height).max(0);
        scroll_v = scroll_v.clamp(0, max_scroll);

        // Only redraw when needed (if optimization enabled)
        if !config.use_redraw_optimization || needs_redraw {
            let screen_width = crt.width();

            // Draw title
            let title_attr = crt.color(ColorElement::MeterText);
            let title_display: String = config.title.chars().take(screen_width as usize).collect();
            crt.mv(0, 0);
            crt.attrset(title_attr);
            crt.hline(0, 0, ' ' as u32, screen_width);
            crt.addstr_raw(&title_display);
            crt.attrset(A_NORMAL);

            // Draw header if present
            if let Some(header) = config.header {
                let header_attr = crt.color(ColorElement::PanelHeaderFocus);
                let header_display: String = header.chars().take(screen_width as usize).collect();
                crt.mv(1, 0);
                crt.attrset(header_attr);
                crt.hline(1, 0, ' ' as u32, screen_width);
                crt.addstr_raw(&header_display);
                crt.attrset(A_NORMAL);
            }

            // Draw lines
            let default_attr = crt.color(ColorElement::DefaultColor);
            let selection_attr = crt.color(ColorElement::PanelSelectionFocus);

            for row in 0..panel_height {
                let y = panel_y + row;
                let line_idx = (scroll_v + row) as usize;

                if line_idx < filtered_indices.len() {
                    let actual_idx = filtered_indices[line_idx];
                    let line = &lines[actual_idx];
                    let is_selected = (scroll_v + row) == selected;

                    let attr = if is_selected {
                        selection_attr
                    } else {
                        default_attr
                    };
                    let display_line: String = line.chars().take(screen_width as usize).collect();
                    crt.mv(y, 0);
                    crt.attrset(attr);
                    crt.hline(y, 0, ' ' as u32, screen_width);
                    crt.addstr_raw(&display_line);
                    crt.attrset(A_NORMAL);
                } else {
                    crt.mv(y, 0);
                    crt.attrset(default_attr);
                    crt.hline(y, 0, ' ' as u32, screen_width);
                    crt.attrset(A_NORMAL);
                }
            }

            // Draw function bar or search/filter bar
            let fb_y = crt.height() - 1;
            sf_state.draw_bar(crt, fb_y, screen_width);

            crt.refresh();
        }

        // Handle input
        crt.set_blocking(true);
        let mut ch = crt.getch();
        needs_redraw = true; // Assume we need redraw

        // Handle mouse events
        ch = search_filter::process_mouse_event(crt, &sf_state, ch);

        // Handle search/filter input
        if sf_state.is_active() {
            match sf_state.handle_input(ch) {
                HandleResult::SearchChanged => {
                    if let Some(idx) = sf_state.find_first_match(lines, &filtered_indices) {
                        selected = idx as i32;
                    }
                }
                HandleResult::FilterChanged => {
                    selected = 0;
                    scroll_v = 0;
                }
                HandleResult::SearchNext => {
                    if let Some(idx) =
                        sf_state.find_next_match(lines, &filtered_indices, selected as usize)
                    {
                        selected = idx as i32;
                    }
                }
                HandleResult::SearchPrev => {
                    if let Some(idx) =
                        sf_state.find_prev_match(lines, &filtered_indices, selected as usize)
                    {
                        selected = idx as i32;
                    }
                }
                HandleResult::Handled | HandleResult::NotHandled => {}
            }
            continue;
        }

        // Handle search/filter start keys
        if sf_state.handle_start_key(ch) {
            continue;
        }

        // Handle standard keys
        match ch {
            27 | 113 | KEY_F10 => break, // Escape, 'q', or F10 - exit
            x if x == KEY_F5 => {
                // F5 - refresh
                if let Some(ref refresh) = refresh_fn {
                    let saved_selected = selected;
                    *lines = refresh();
                    let max_idx = (lines.len() as i32 - 1).max(0);
                    selected = saved_selected.min(max_idx);
                    crt.clear();
                }
            }
            KEY_CTRL_L => {
                // Ctrl+L - refresh screen
                crt.clear();
            }
            KEY_UP | KEY_CTRL_P => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            KEY_DOWN | KEY_CTRL_N => {
                selected += 1;
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
            _ => {
                // Unknown key, no redraw needed if using optimization
                if config.use_redraw_optimization {
                    needs_redraw = false;
                }
            }
        }
    }

    crt.enable_delay();
}
