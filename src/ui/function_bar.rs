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
use crate::ncurses_compat::*;

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

/// Function bar item with key, label, and enabled state
#[derive(Debug, Clone)]
pub struct FunctionBarItem {
    pub key: String,
    pub label: String,
    pub enabled: bool,
}

impl FunctionBarItem {
    pub fn new(key: &str, label: &str) -> Self {
        FunctionBarItem {
            key: key.to_string(),
            label: label.to_string(),
            enabled: true,
        }
    }

    pub fn disabled(key: &str, label: &str) -> Self {
        FunctionBarItem {
            key: key.to_string(),
            label: label.to_string(),
            enabled: false,
        }
    }
}

/// Function bar at the bottom of the screen
#[derive(Debug, Clone)]
pub struct FunctionBar {
    pub items: Vec<FunctionBarItem>,
}

impl FunctionBar {
    /// Create a new function bar with default labels
    pub fn new() -> Self {
        FunctionBar {
            items: DEFAULT_FUNCTIONS
                .iter()
                .map(|(k, v)| FunctionBarItem::new(k, v))
                .collect(),
        }
    }

    /// Create a new function bar with custom labels
    /// labels is an array of (label, key) pairs for F1-F10
    /// Empty label means skip that function key
    pub fn new_with_labels(labels: &[(&str, &str); 10]) -> Self {
        let items: Vec<FunctionBarItem> = labels
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
                    Some(FunctionBarItem::new(&key_str, &formatted_label))
                }
            })
            .collect();

        FunctionBar { items }
    }

    /// Create a function bar with custom labels (backward compatibility)
    pub fn with_functions(functions: Vec<(String, String)>) -> Self {
        FunctionBar {
            items: functions
                .into_iter()
                .map(|(k, v)| FunctionBarItem::new(&k, &v))
                .collect(),
        }
    }

    /// Create a simple Enter/Esc function bar (like C htop FunctionBar_newEnterEsc)
    pub fn new_enter_esc(enter_label: &str, esc_label: &str) -> Self {
        FunctionBar {
            items: vec![
                FunctionBarItem::new("Enter", enter_label),
                FunctionBarItem::new("Esc", esc_label),
            ],
        }
    }

    /// Set a function key label
    pub fn set_function(&mut self, index: usize, key: &str, label: &str) {
        if index < self.items.len() {
            self.items[index].key = key.to_string();
            self.items[index].label = label.to_string();
        }
    }

    /// Set whether a function key is enabled (grayed out if disabled)
    pub fn set_enabled(&mut self, index: usize, enabled: bool) {
        if index < self.items.len() {
            self.items[index].enabled = enabled;
        }
    }

    /// Draw the function bar (matches C htop FunctionBar_drawExtra)
    pub fn draw(&self, crt: &Crt, y: i32, _settings: &Settings) {
        let width = crt.width();
        let bar_color = crt.color(ColorElement::FunctionBar);
        let key_color = crt.color(ColorElement::FunctionKey);
        let disabled_color = crt.color(ColorElement::Disabled);

        // First fill entire line with spaces in FUNCTION_BAR color
        mv(y, 0);
        attrset(bar_color);
        for _ in 0..width {
            addch(' ' as u32);
        }

        // Draw each key-label pair consecutively (no padding between pairs)
        let mut x = 0i32;
        for item in &self.items {
            if x >= width {
                break;
            }

            // Use disabled color for disabled items
            let effective_key_color = if item.enabled {
                key_color
            } else {
                disabled_color
            };
            let effective_bar_color = if item.enabled {
                bar_color
            } else {
                disabled_color
            };

            // Draw the key (F1, F2, etc.)
            mv(y, x);
            attrset(effective_key_color);
            let _ = addstr(&item.key);
            x += item.key.len() as i32;

            // Draw the label (Help, Setup, etc.)
            attrset(effective_bar_color);
            let _ = addstr(&item.label);
            x += item.label.len() as i32;
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
        let disabled_color = crt.color(ColorElement::Disabled);

        // First fill entire line with spaces in FUNCTION_BAR color
        mv(y, 0);
        attrset(bar_color);
        for _ in 0..width {
            addch(' ' as u32);
        }

        // Draw each key-label pair consecutively (no padding between pairs)
        let mut x = 0i32;
        for item in &self.items {
            if x >= width {
                break;
            }

            // Use disabled color for disabled items
            let effective_key_color = if item.enabled {
                key_color
            } else {
                disabled_color
            };
            let effective_bar_color = if item.enabled {
                bar_color
            } else {
                disabled_color
            };

            // Draw the key (F1, F2, etc.)
            mv(y, x);
            attrset(effective_key_color);
            let _ = addstr(&item.key);
            x += item.key.len() as i32;

            // Draw the label (Help, Setup, etc.)
            attrset(effective_bar_color);
            let _ = addstr(&item.label);
            x += item.label.len() as i32;
        }
        attrset(A_NORMAL);
        x
    }

    /// Backward compatibility: get functions as Vec<(String, String)>
    pub fn functions(&self) -> Vec<(String, String)> {
        self.items
            .iter()
            .map(|item| (item.key.clone(), item.label.clone()))
            .collect()
    }
}

impl Default for FunctionBar {
    fn default() -> Self {
        FunctionBar::new()
    }
}
