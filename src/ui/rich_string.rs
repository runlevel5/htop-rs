//! RichString - Attributed string for colored text output
//!
//! This module provides a string type that carries character attributes
//! (colors, bold, etc.) along with each character.

#![allow(dead_code)]

use super::crt::{attr_t, A_NORMAL};
use super::Crt;

/// A character with its display attributes
#[derive(Debug, Clone, Copy)]
pub struct RichChar {
    pub ch: char,
    pub attr: attr_t,
}

impl RichChar {
    pub fn new(ch: char, attr: attr_t) -> Self {
        RichChar { ch, attr }
    }
}

impl Default for RichChar {
    fn default() -> Self {
        RichChar {
            ch: ' ',
            attr: A_NORMAL,
        }
    }
}

/// A string with per-character attributes
#[derive(Debug, Clone, Default)]
pub struct RichString {
    chars: Vec<RichChar>,
}

impl RichString {
    /// Create an empty RichString
    pub fn new() -> Self {
        RichString { chars: Vec::new() }
    }

    /// Create a RichString with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        RichString {
            chars: Vec::with_capacity(capacity),
        }
    }

    /// Clear the string
    pub fn clear(&mut self) {
        self.chars.clear();
    }

    /// Get the length in characters
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Check if the string is empty
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Append a character with attributes
    pub fn append_char(&mut self, ch: char, attr: attr_t) {
        self.chars.push(RichChar::new(ch, attr));
    }

    /// Append a string with attributes
    pub fn append(&mut self, text: &str, attr: attr_t) {
        self.chars.reserve(text.len());
        for ch in text.chars() {
            self.chars.push(RichChar::new(ch, attr));
        }
    }

    /// Append first n characters of a string with attributes
    pub fn append_n(&mut self, text: &str, attr: attr_t, n: usize) {
        self.chars.reserve(n);
        for ch in text.chars().take(n) {
            self.chars.push(RichChar::new(ch, attr));
        }
    }

    /// Append a character repeated n times
    pub fn append_chr(&mut self, ch: char, attr: attr_t, n: usize) {
        self.chars.reserve(n);
        for _ in 0..n {
            self.chars.push(RichChar::new(ch, attr));
        }
    }

    /// Append another RichString
    pub fn append_rich(&mut self, other: &RichString) {
        self.chars.extend_from_slice(&other.chars);
    }

    /// Set the attribute for a range of characters
    pub fn set_attr(&mut self, start: usize, end: usize, attr: attr_t) {
        for i in start..end.min(self.chars.len()) {
            self.chars[i].attr = attr;
        }
    }

    /// Set the attribute for ALL characters (matches C htop's RichString_setAttr)
    /// This is used for selection highlighting to override all per-character colors
    pub fn set_all_attr(&mut self, attr: attr_t) {
        for rc in &mut self.chars {
            rc.attr = attr;
        }
    }

    /// Get the plain text content
    pub fn text(&self) -> String {
        self.chars.iter().map(|rc| rc.ch).collect()
    }

    /// Get a character at a position
    pub fn get(&self, index: usize) -> Option<&RichChar> {
        self.chars.get(index)
    }

    /// Get a mutable character at a position
    pub fn get_mut(&mut self, index: usize) -> Option<&mut RichChar> {
        self.chars.get_mut(index)
    }

    /// Get the last character (matches C htop RichString_getCharVal for last position)
    pub fn last_char(&self) -> Option<char> {
        self.chars.last().map(|rc| rc.ch)
    }

    /// Remove n characters from the end (matches C htop RichString_rewind)
    pub fn rewind(&mut self, n: usize) {
        let new_len = self.chars.len().saturating_sub(n);
        self.chars.truncate(new_len);
    }

    /// Write the string to the screen at the given position
    pub fn write_at(&self, crt: &mut Crt, y: i32, x: i32) {
        let mut current_attr = A_NORMAL;

        crt.mv(y, x);
        crt.attrset(current_attr);

        for rc in &self.chars {
            if rc.attr != current_attr {
                current_attr = rc.attr;
                crt.attrset(current_attr);
            }
            // Use addstr for proper Unicode support
            let mut buf = [0u8; 4];
            let s = rc.ch.encode_utf8(&mut buf);
            crt.addstr_raw(s);
        }

        crt.attrset(A_NORMAL);
    }

    /// Write the string with truncation/padding to fit a width
    pub fn write_at_width(&self, crt: &mut Crt, y: i32, x: i32, width: usize) {
        self.write_at_width_with_pad_attr(crt, y, x, width, None)
    }

    /// Write the string with truncation/padding to fit a width, with optional padding attribute
    pub fn write_at_width_with_pad_attr(
        &self,
        crt: &mut Crt,
        y: i32,
        x: i32,
        width: usize,
        pad_attr: Option<attr_t>,
    ) {
        crt.mv(y, x);

        // Use attrset instead of attron for proper color handling
        let mut current_attr = A_NORMAL;
        crt.attrset(current_attr);

        let mut written = 0;
        for rc in &self.chars {
            if written >= width {
                break;
            }
            if rc.attr != current_attr {
                current_attr = rc.attr;
                crt.attrset(current_attr);
            }
            // Use addstr for proper Unicode support
            let mut buf = [0u8; 4];
            let s = rc.ch.encode_utf8(&mut buf);
            crt.addstr_raw(s);
            written += 1;
        }

        // Pad with spaces if needed
        // Use pad_attr if specified, otherwise keep current_attr for background
        if let Some(attr) = pad_attr {
            if attr != current_attr {
                crt.attrset(attr);
            }
        }
        while written < width {
            crt.addch_raw(' ' as u32);
            written += 1;
        }

        crt.attrset(A_NORMAL);
    }

    /// Write the string with a single override attribute for all characters
    /// This is used for selection highlighting (matches C htop's behavior)
    /// where RichString_setAttr overrides all per-character attributes
    pub fn write_at_width_with_attr(
        &self,
        crt: &mut Crt,
        y: i32,
        x: i32,
        width: usize,
        attr: attr_t,
    ) {
        crt.mv(y, x);
        crt.attrset(attr);

        let mut written = 0;
        for rc in &self.chars {
            if written >= width {
                break;
            }
            // Use addstr for proper Unicode support
            let mut buf = [0u8; 4];
            let s = rc.ch.encode_utf8(&mut buf);
            crt.addstr_raw(s);
            written += 1;
        }

        // Pad with spaces if needed
        while written < width {
            crt.addch_raw(' ' as u32);
            written += 1;
        }

        crt.attrset(A_NORMAL);
    }

    /// Draw the string at position with width, preserving per-character attributes
    /// This is similar to write_at_width but doesn't use a default attribute
    /// Padding uses A_NORMAL - use draw_at_with_bg for themed backgrounds
    pub fn draw_at(&self, crt: &mut Crt, y: i32, x: i32, width: i32) {
        self.draw_at_with_bg(crt, y, x, width, A_NORMAL);
    }

    /// Draw the string at position with width, using specified background for padding
    /// This should be used when drawing on themed backgrounds (e.g., MC theme's blue background)
    pub fn draw_at_with_bg(&self, crt: &mut Crt, y: i32, x: i32, width: i32, bg_attr: attr_t) {
        crt.mv(y, x);
        let mut current_attr: attr_t = A_NORMAL;
        let width = width as usize;

        crt.attrset(current_attr);

        let mut written = 0;
        for rc in &self.chars {
            if written >= width {
                break;
            }
            if rc.attr != current_attr {
                current_attr = rc.attr;
                crt.attrset(current_attr);
            }
            // Use addstr for proper Unicode support
            let mut buf = [0u8; 4];
            let s = rc.ch.encode_utf8(&mut buf);
            crt.addstr_raw(s);
            written += 1;
        }

        // Pad with spaces using the background attribute
        crt.attrset(bg_attr);
        while written < width {
            crt.addch_raw(' ' as u32);
            written += 1;
        }
    }

    /// Get an iterator over the characters
    pub fn iter(&self) -> impl Iterator<Item = &RichChar> {
        self.chars.iter()
    }
}

impl From<&str> for RichString {
    fn from(s: &str) -> Self {
        let mut rs = RichString::new();
        rs.append(s, A_NORMAL);
        rs
    }
}

impl std::fmt::Display for RichString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text())
    }
}
