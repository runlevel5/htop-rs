//! FunctionBar - F1-F10 key labels at the bottom of the screen
//!
//! Matches C htop FunctionBar.c implementation exactly:
//! - Keys (F1, F2, etc.) use FUNCTION_KEY color (white on black)
//! - Labels (Help, Setup, etc.) use FUNCTION_BAR color (black on cyan)
//! - Labels are fixed 6-character width with trailing spaces
//! - No padding between key-label pairs

#![allow(dead_code)]

use super::crt::ColorElement;
use super::Crt;
use crate::core::Settings;
use ncurses::*;

/// Default function key labels (6-char fixed width with trailing spaces, like C htop)
pub const DEFAULT_FUNCTIONS: [(&str, &str); 10] = [
    ("F1", "Help  "),
    ("F2", "Setup "),
    ("F3", "Search"),
    ("F4", "Filter"),
    ("F5", "Tree  "),
    ("F6", "SortBy"),
    ("F7", "Nice -"),
    ("F8", "Nice +"),
    ("F9", "Kill  "),
    ("F10", "Quit  "),
];

/// Function bar at the bottom of the screen
#[derive(Debug, Clone)]
pub struct FunctionBar {
    pub functions: Vec<(String, String)>,
}

impl FunctionBar {
    /// Create a new function bar with default labels
    pub fn new() -> Self {
        FunctionBar {
            functions: DEFAULT_FUNCTIONS
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    /// Create a new function bar with custom labels
    /// labels is an array of (label, key) pairs for F1-F10
    /// Empty label means skip that function key
    pub fn new_with_labels(labels: &[(&str, &str); 10]) -> Self {
        let functions: Vec<(String, String)> = labels
            .iter()
            .enumerate()
            .filter_map(|(i, (label, key))| {
                if label.is_empty() && key.is_empty() {
                    None
                } else {
                    // Format label to be 6 chars wide
                    let formatted_label = format!("{:6}", label);
                    let key_str = if key.is_empty() {
                        format!("F{}", i + 1)
                    } else {
                        key.to_string()
                    };
                    Some((key_str, formatted_label))
                }
            })
            .collect();

        FunctionBar { functions }
    }

    /// Create a function bar with custom labels
    pub fn with_functions(functions: Vec<(String, String)>) -> Self {
        FunctionBar { functions }
    }

    /// Create a simple Enter/Esc function bar (like C htop FunctionBar_newEnterEsc)
    pub fn new_enter_esc(enter_label: &str, esc_label: &str) -> Self {
        FunctionBar {
            functions: vec![
                ("Enter".to_string(), enter_label.to_string()),
                ("Esc".to_string(), esc_label.to_string()),
            ],
        }
    }

    /// Set a function key label
    pub fn set_function(&mut self, index: usize, key: &str, label: &str) {
        if index < self.functions.len() {
            self.functions[index] = (key.to_string(), label.to_string());
        }
    }

    /// Draw the function bar (matches C htop FunctionBar_drawExtra)
    pub fn draw(&self, crt: &Crt, y: i32, _settings: &Settings) {
        let width = crt.width();
        let bar_color = crt.color(ColorElement::FunctionBar);
        let key_color = crt.color(ColorElement::FunctionKey);

        // First fill entire line with spaces in FUNCTION_BAR color
        mv(y, 0);
        attrset(bar_color);
        for _ in 0..width {
            addch(' ' as u32);
        }

        // Draw each key-label pair consecutively (no padding between pairs)
        let mut x = 0i32;
        for (key, label) in &self.functions {
            if x >= width {
                break;
            }

            // Draw the key (F1, F2, etc.) in FUNCTION_KEY color
            mv(y, x);
            attrset(key_color);
            let _ = addstr(key);
            x += key.len() as i32;

            // Draw the label (Help, Setup, etc.) in FUNCTION_BAR color
            attrset(bar_color);
            let _ = addstr(label);
            x += label.len() as i32;
        }
        attrset(A_NORMAL);
    }

    /// Draw the function bar without settings (backward compatibility)
    pub fn draw_simple(&self, crt: &Crt, y: i32) {
        self.draw_simple_return_x(crt, y);
    }

    /// Draw the function bar and return the ending x position
    /// This allows appending additional content (like "PAUSED") after the bar
    pub fn draw_simple_return_x(&self, crt: &Crt, y: i32) -> i32 {
        let width = crt.width();
        let bar_color = crt.color(ColorElement::FunctionBar);
        let key_color = crt.color(ColorElement::FunctionKey);

        // First fill entire line with spaces in FUNCTION_BAR color
        mv(y, 0);
        attrset(bar_color);
        for _ in 0..width {
            addch(' ' as u32);
        }

        // Draw each key-label pair consecutively (no padding between pairs)
        let mut x = 0i32;
        for (key, label) in &self.functions {
            if x >= width {
                break;
            }

            // Draw the key (F1, F2, etc.) in FUNCTION_KEY color
            mv(y, x);
            attrset(key_color);
            let _ = addstr(key);
            x += key.len() as i32;

            // Draw the label (Help, Setup, etc.) in FUNCTION_BAR color
            attrset(bar_color);
            let _ = addstr(label);
            x += label.len() as i32;
        }
        attrset(A_NORMAL);
        x
    }
}

impl Default for FunctionBar {
    fn default() -> Self {
        FunctionBar::new()
    }
}
