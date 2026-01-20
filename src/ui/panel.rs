//! Panel - Scrollable list widget
//!
//! This module provides a generic panel widget that can display a scrollable
//! list of items with selection support.

#![allow(dead_code)]

use ncurses::*;
use super::crt::ColorElement;
use super::function_bar::FunctionBar;
use super::rich_string::RichString;
use super::Crt;

/// Handler result from event processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerResult {
    Handled,
    Ignored,
    BreakLoop,
    Refresh,
    Redraw,
    Rescan,
    Resize,
}

/// Item in a panel
pub trait PanelItem {
    /// Display the item
    fn display(&self, buffer: &mut RichString, highlighted: bool);
    
    /// Get the sort key for typing search
    fn sort_key(&self) -> &str;
}

/// A simple text item
#[derive(Debug, Clone)]
pub struct TextItem {
    pub text: String,
    pub attr: attr_t,
}

impl TextItem {
    pub fn new(text: &str) -> Self {
        TextItem { 
            text: text.to_string(),
            attr: A_NORMAL,
        }
    }

    pub fn with_attr(text: &str, attr: attr_t) -> Self {
        TextItem { 
            text: text.to_string(),
            attr,
        }
    }
}

impl PanelItem for TextItem {
    fn display(&self, buffer: &mut RichString, _highlighted: bool) {
        buffer.append(&self.text, self.attr);
    }
    
    fn sort_key(&self) -> &str {
        &self.text
    }
}

/// A list item with a display value and an integer key (like C htop's ListItem)
#[derive(Debug, Clone)]
pub struct ListItem {
    pub value: String,
    pub key: i32,
}

impl ListItem {
    pub fn new(value: &str, key: i32) -> Self {
        ListItem {
            value: value.to_string(),
            key,
        }
    }
}

impl PanelItem for ListItem {
    fn display(&self, buffer: &mut RichString, _highlighted: bool) {
        buffer.append(&self.value, A_NORMAL);
    }
    
    fn sort_key(&self) -> &str {
        &self.value
    }
}

/// Panel widget
pub struct Panel {
    // Position and size
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    
    // Items
    items: Vec<Box<dyn PanelItem>>,
    
    // Selection state
    pub selected: i32,
    pub old_selected: i32,
    pub scroll_v: i32,
    pub scroll_h: i32,
    
    // Display state
    pub needs_redraw: bool,
    pub cursor_on: bool,
    pub was_focus: bool,
    
    // Header
    pub header: RichString,
    
    // Function bar
    pub function_bar: FunctionBar,
    
    // Selection color
    pub selection_color: ColorElement,
}

impl Panel {
    /// Create a new panel
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Panel {
            x,
            y,
            w,
            h,
            items: Vec::new(),
            selected: 0,
            old_selected: 0,
            scroll_v: 0,
            scroll_h: 0,
            needs_redraw: true,
            cursor_on: false,
            was_focus: false,
            header: RichString::new(),
            function_bar: FunctionBar::new(),
            selection_color: ColorElement::PanelSelectionFocus,
        }
    }

    /// Set the header text
    pub fn set_header(&mut self, text: &str) {
        self.header.clear();
        self.header.append(text, A_BOLD);
    }

    /// Add an item to the panel
    pub fn add(&mut self, item: Box<dyn PanelItem>) {
        self.items.push(item);
        self.needs_redraw = true;
    }

    /// Add a text item
    pub fn add_text(&mut self, text: &str) {
        self.add(Box::new(TextItem::new(text)));
    }

    /// Add a list item (with value and key)
    pub fn add_list_item(&mut self, value: &str, key: i32) {
        self.add(Box::new(ListItem::new(value, key)));
    }

    /// Clear all items
    pub fn prune(&mut self) {
        self.items.clear();
        self.selected = 0;
        self.scroll_v = 0;
        self.needs_redraw = true;
    }

    /// Get the number of items
    pub fn size(&self) -> i32 {
        self.items.len() as i32
    }

    /// Get the selected index
    pub fn get_selected(&self) -> i32 {
        self.selected
    }

    /// Set the selected index
    pub fn set_selected(&mut self, selected: i32) {
        let max = (self.items.len() as i32 - 1).max(0);
        self.old_selected = self.selected;
        self.selected = selected.clamp(0, max);
        self.needs_redraw = true;
        self.ensure_visible();
    }

    /// Move selection up
    pub fn move_up(&mut self, delta: i32) {
        self.set_selected(self.selected - delta);
    }

    /// Move selection down
    pub fn move_down(&mut self, delta: i32) {
        self.set_selected(self.selected + delta);
    }

    /// Move selection to the start
    pub fn move_home(&mut self) {
        self.set_selected(0);
    }

    /// Move selection to the end
    pub fn move_end(&mut self) {
        self.set_selected(self.items.len() as i32 - 1);
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.move_up(self.h - 1);
    }

    /// Page down
    pub fn page_down(&mut self) {
        self.move_down(self.h - 1);
    }

    /// Ensure the selected item is visible
    fn ensure_visible(&mut self) {
        // Adjust scroll to make selection visible
        if self.selected < self.scroll_v {
            self.scroll_v = self.selected;
        } else if self.selected >= self.scroll_v + self.h {
            self.scroll_v = self.selected - self.h + 1;
        }
    }

    /// Resize the panel
    pub fn resize(&mut self, w: i32, h: i32) {
        self.w = w;
        self.h = h;
        self.needs_redraw = true;
        self.ensure_visible();
    }

    /// Move the panel
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.needs_redraw = true;
    }

    /// Draw the panel header
    fn draw_header(&self, crt: &Crt, focus: bool) {
        let attr = if focus {
            crt.color(ColorElement::PanelHeaderFocus)
        } else {
            crt.color(ColorElement::PanelHeaderUnfocus)
        };

        // Fill header line
        mv(self.y, self.x);
        attron(attr);
        for _ in 0..self.w {
            addch(' ' as u32);
        }
        attroff(attr);

        // Draw header text
        if !self.header.is_empty() {
            mv(self.y, self.x);
            attron(attr);
            let text = self.header.text();
            let display_text: String = text.chars().take(self.w as usize).collect();
            let _ = addstr(&display_text);
            attroff(attr);
        }
    }

    /// Draw the panel
    pub fn draw(&mut self, crt: &Crt, focus: bool, show_header: bool) {
        let start_y = if show_header {
            self.draw_header(crt, focus);
            self.y + 1
        } else {
            self.y
        };

        let display_height = if show_header { self.h - 1 } else { self.h };
        let selection_attr = crt.color(self.selection_color);
        let default_attr = crt.color(ColorElement::Process);

        for i in 0..display_height {
            let item_index = (self.scroll_v + i) as usize;
            let y = start_y + i;

            mv(y, self.x);

            if item_index < self.items.len() {
                let is_selected = item_index as i32 == self.selected;
                let attr = if is_selected { selection_attr } else { default_attr };

                // Get item display
                let mut buffer = RichString::new();
                self.items[item_index].display(&mut buffer, is_selected);

                // Draw with selection highlighting
                attron(attr);
                let text = buffer.text();
                let display_text: String = text.chars().take(self.w as usize).collect();
                let _ = addstr(&display_text);
                
                // Pad to width
                let padding = self.w as usize - display_text.chars().count();
                for _ in 0..padding {
                    addch(' ' as u32);
                }
                attroff(attr);
            } else {
                // Empty line
                for _ in 0..self.w {
                    addch(' ' as u32);
                }
            }
        }

        self.needs_redraw = false;
    }

    /// Handle a key event
    pub fn on_key(&mut self, key: i32) -> HandlerResult {
        match key {
            KEY_UP => {
                self.move_up(1);
                HandlerResult::Handled
            }
            KEY_DOWN => {
                self.move_down(1);
                HandlerResult::Handled
            }
            KEY_PPAGE => {
                self.page_up();
                HandlerResult::Handled
            }
            KEY_NPAGE => {
                self.page_down();
                HandlerResult::Handled
            }
            KEY_HOME => {
                self.move_home();
                HandlerResult::Handled
            }
            KEY_END => {
                self.move_end();
                HandlerResult::Handled
            }
            _ => HandlerResult::Ignored,
        }
    }

    /// Select by typing (incremental search)
    pub fn select_by_typing(&mut self, ch: char) -> HandlerResult {
        let search_char = ch.to_lowercase().next().unwrap_or(ch);
        
        // Search from current selection
        for i in (self.selected as usize + 1)..self.items.len() {
            if let Some(first_char) = self.items[i].sort_key().chars().next() {
                if first_char.to_lowercase().next() == Some(search_char) {
                    self.set_selected(i as i32);
                    return HandlerResult::Handled;
                }
            }
        }
        
        // Wrap around to beginning
        for i in 0..self.selected as usize {
            if let Some(first_char) = self.items[i].sort_key().chars().next() {
                if first_char.to_lowercase().next() == Some(search_char) {
                    self.set_selected(i as i32);
                    return HandlerResult::Handled;
                }
            }
        }
        
        HandlerResult::Ignored
    }
}
