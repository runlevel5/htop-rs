//! Side panel menu helper for displaying selection menus alongside the main process panel.
//!
//! This module provides utilities for displaying side panel menus (like kill signal selection,
//! sort column selection, and user filter selection) that appear on the left side of the screen
//! while the main process panel is shown on the right.

use super::crt::{
    KEY_DOWN, KEY_END, KEY_ESC, KEY_F10, KEY_HOME, KEY_LC_Q, KEY_LINEFEED, KEY_MOUSE, KEY_NPAGE,
    KEY_PPAGE, KEY_PRINTABLE_END, KEY_PRINTABLE_START, KEY_RETURN, KEY_UP, KEY_WHEELDOWN,
    KEY_WHEELUP,
};
use super::header::Header;
use super::main_panel::MainPanel;
use super::panel::Panel;
use super::Crt;
use crate::core::{Machine, Settings};

/// Result of running a side panel menu
pub enum SidePanelResult {
    /// User selected an item (index in the panel's list)
    Selected(usize),
    /// User cancelled the menu
    Cancelled,
}

/// Context needed for drawing the side panel menu
pub struct SidePanelContext<'a> {
    pub main_panel: &'a mut MainPanel,
    pub header: &'a Header,
    pub settings: &'a Settings,
    pub hide_meters: bool,
}

/// Run a side panel menu using an existing Panel.
///
/// This handles:
/// - Resizing the main panel to make room for the side panel
/// - Drawing the header, side panel, and main panel
/// - Handling navigation keys (up, down, page up/down, home, end, mouse wheel)
/// - Handling Enter to confirm and Escape/q/F10 to cancel
/// - Typing search to jump to items
/// - Restoring the main panel to its original position
///
/// Returns `SidePanelResult::Selected(index)` if the user pressed Enter,
/// or `SidePanelResult::Cancelled` if they pressed Escape/q/F10.
pub fn run_side_panel_menu(
    crt: &mut Crt,
    machine: &mut Machine,
    ctx: &mut SidePanelContext,
    panel: &mut Panel,
) -> SidePanelResult {
    let panel_y = ctx.main_panel.y;
    let panel_height = crt.height() - panel_y - 1; // Leave room for function bar
    let panel_width = panel.w;

    // Ensure side panel has correct position and height
    panel.move_to(0, panel_y);
    panel.resize(panel_width, panel_height);

    // Save original main panel position
    let orig_main_x = ctx.main_panel.x;
    let orig_main_w = ctx.main_panel.w;

    // Resize main panel to make room for side panel on left
    ctx.main_panel.move_to(panel_width, panel_y);
    ctx.main_panel
        .resize(crt.width() - panel_width, panel_height);
    ctx.main_panel.needs_redraw = true;

    let result = run_panel_event_loop(crt, machine, ctx, panel);

    // Clear the side panel area before restoring main panel
    // This prevents artifacts and avoids the need for a full screen clear
    crt.clear_area(0, panel_y, panel_width, panel_height + 1); // +1 for function bar area

    // Restore main panel position
    ctx.main_panel.move_to(orig_main_x, panel_y);
    ctx.main_panel.resize(orig_main_w, panel_height);
    ctx.main_panel.needs_redraw = true;

    result
}

/// The main event loop for side panel menus using Panel
fn run_panel_event_loop(
    crt: &mut Crt,
    machine: &mut Machine,
    ctx: &mut SidePanelContext,
    panel: &mut Panel,
) -> SidePanelResult {
    loop {
        // Draw header meters
        if !ctx.hide_meters {
            ctx.header.draw(crt, machine, ctx.settings);
        }

        // Draw side panel on the left (with focus)
        panel.draw(crt, true, true);

        // Draw main panel on the right (no focus)
        ctx.main_panel.draw(crt, machine, ctx.settings, false);

        // Draw the function bar
        let fb_y = crt.height() - 1;
        panel.function_bar.draw_simple(crt, fb_y);

        crt.refresh();

        // Handle input
        let mut key = crt.getch();

        // Handle mouse events
        if key == KEY_MOUSE {
            let func_bar = &panel.function_bar;
            let screen_height = crt.height();
            if let Some(event) = crt.get_mouse_event() {
                // Check wheel events first (they can happen anywhere)
                if event.is_wheel_up() {
                    key = KEY_WHEELUP;
                } else if event.is_wheel_down() {
                    key = KEY_WHEELDOWN;
                } else if event.is_left_click() && event.y == screen_height - 1 {
                    // Click on function bar
                    if let Some(fkey) = func_bar.get_click_key(event.x) {
                        key = fkey;
                    }
                } else if event.x >= panel.x
                    && event.x < panel.x + panel.w
                    && event.y > panel.y
                    && event.y < panel.y + panel.h
                {
                    // Click within the panel content area (below header)
                    let clicked_row = event.y - panel.y - 1; // -1 for header
                    let clicked_index = panel.scroll_v + clicked_row;
                    if clicked_index >= 0 && (clicked_index as usize) < panel.size() as usize {
                        if event.is_left_click() {
                            // Left-click: select the item
                            panel.set_selected(clicked_index);
                        } else if event.is_right_click() {
                            // Right-click: select and confirm (Enter)
                            panel.set_selected(clicked_index);
                            key = KEY_RETURN;
                        }
                    }
                }
            }
        }

        match key {
            KEY_UP => panel.move_up(1),
            KEY_DOWN => panel.move_down(1),
            KEY_PPAGE => panel.page_up(),
            KEY_NPAGE => panel.page_down(),
            KEY_HOME => panel.move_home(),
            KEY_END => panel.move_end(),
            KEY_WHEELUP => panel.scroll_wheel(-10),
            KEY_WHEELDOWN => panel.scroll_wheel(10),
            KEY_LINEFEED | KEY_RETURN => {
                // Enter - select and exit
                return SidePanelResult::Selected(panel.get_selected() as usize);
            }
            KEY_ESC | KEY_LC_Q | KEY_F10 => {
                // Escape, 'q', or F10 - cancel
                return SidePanelResult::Cancelled;
            }
            _ => {
                // Try typing search (jump to item starting with this char)
                // Note: 'q' (KEY_LC_Q) is handled above, so won't trigger typing search
                if (KEY_PRINTABLE_START..KEY_PRINTABLE_END).contains(&key) && key != KEY_LC_Q {
                    panel.select_by_typing(key as u8 as char);
                }
            }
        }
    }
}
