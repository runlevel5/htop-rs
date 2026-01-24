//! ncurses compatibility shim for ncurses-pure-rust
//!
//! This module provides a global/procedural API that mimics the C ncurses bindings,
//! allowing incremental migration to the pure Rust ncurses library.
//!
//! The Screen is stored in thread-local storage and accessed via these wrapper functions.

#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]

use ncurses_rs::{self as nc, Screen, Window};
use std::cell::RefCell;

// Re-export types from ncurses-rs
// ACS chars are imported with _CHAR suffix since we provide function wrappers
use nc::acs::{
    ACS_HLINE as ACS_HLINE_CHAR, ACS_LLCORNER as ACS_LLCORNER_CHAR,
    ACS_LRCORNER as ACS_LRCORNER_CHAR, ACS_ULCORNER as ACS_ULCORNER_CHAR,
    ACS_URCORNER as ACS_URCORNER_CHAR, ACS_VLINE as ACS_VLINE_CHAR,
};
pub use nc::attr::{A_BOLD, A_DIM, A_NORMAL, A_REVERSE, A_STANDOUT, A_UNDERLINE};
pub use nc::color::{
    COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_GREEN, COLOR_MAGENTA, COLOR_RED, COLOR_WHITE,
    COLOR_YELLOW,
};
pub use nc::key::{
    KEY_BACKSPACE, KEY_DC, KEY_DOWN, KEY_END, KEY_ENTER, KEY_F0, KEY_HOME, KEY_IC, KEY_LEFT,
    KEY_MOUSE, KEY_NPAGE, KEY_PPAGE, KEY_RESIZE, KEY_RIGHT, KEY_UP,
};
pub use nc::mouse::{
    MouseEvent as MEVENT, BUTTON1_RELEASED, BUTTON3_RELEASED, BUTTON4_PRESSED, BUTTON5_PRESSED,
};
pub use nc::types::{AttrT, ChType, MmaskT, ERR, OK};

// Type aliases
pub type attr_t = AttrT;
pub type mmask_t = MmaskT;

/// Cursor visibility constants
pub mod CURSOR_VISIBILITY {
    pub const CURSOR_INVISIBLE: i32 = 0;
    pub const CURSOR_VISIBLE: i32 = 1;
    pub const CURSOR_VERY_VISIBLE: i32 = 2;
}

thread_local! {
    static SCREEN: RefCell<Option<Screen>> = const { RefCell::new(None) };
}

/// Execute a function with the global screen
fn with_screen<F, R>(f: F) -> R
where
    F: FnOnce(&mut Screen) -> R,
{
    SCREEN.with(|cell| {
        let mut screen = cell.borrow_mut();
        let screen = screen
            .as_mut()
            .expect("ncurses not initialized - call initscr() first");
        f(screen)
    })
}

/// Execute a function with stdscr
fn with_stdscr<F, R>(f: F) -> R
where
    F: FnOnce(&mut Window) -> R,
{
    with_screen(|screen| f(screen.stdscr_mut()))
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the screen (equivalent to initscr())
pub fn initscr() -> *mut Window {
    let screen = Screen::init().expect("Failed to initialize ncurses");
    SCREEN.with(|cell| {
        *cell.borrow_mut() = Some(screen);
    });
    // Return a dummy pointer (we don't actually use it)
    std::ptr::null_mut()
}

/// End curses mode
pub fn endwin() {
    SCREEN.with(|cell| {
        if let Some(ref mut screen) = *cell.borrow_mut() {
            let _ = screen.endwin();
        }
    });
}

/// Get the standard screen window (returns dummy, use with_stdscr instead)
pub fn stdscr() -> *mut Window {
    std::ptr::null_mut()
}

// ============================================================================
// Input mode control
// ============================================================================

/// Enable cbreak mode
pub fn cbreak() {
    with_screen(|s| {
        let _ = s.cbreak();
    });
}

/// Disable cbreak mode
pub fn nocbreak() {
    with_screen(|s| {
        let _ = s.nocbreak();
    });
}

/// Disable echo
pub fn noecho() {
    with_screen(|s| {
        let _ = s.noecho();
    });
}

/// Enable echo
pub fn echo() {
    with_screen(|s| {
        let _ = s.echo();
    });
}

/// Enable keypad mode
pub fn keypad(_win: *mut Window, bf: bool) {
    with_screen(|s| s.keypad(bf));
}

/// Set half-delay mode
pub fn halfdelay(tenths: i32) {
    with_screen(|s| {
        let _ = s.halfdelay(tenths);
    });
}

/// Set node-delay mode (non-blocking input)
pub fn nodelay(_win: *mut Window, bf: bool) {
    with_screen(|s| s.nodelay(bf));
}

/// Set escape delay
pub fn set_escdelay(ms: i32) {
    with_screen(|s| s.set_escdelay(ms));
}

// ============================================================================
// Cursor control
// ============================================================================

/// Set cursor visibility
pub fn curs_set(visibility: i32) -> i32 {
    with_screen(|s| s.curs_set(visibility).unwrap_or(0))
}

// ============================================================================
// Screen/window operations
// ============================================================================

/// Refresh the screen
pub fn refresh() {
    with_screen(|s| {
        let _ = s.refresh();
    });
}

/// Clear the screen
pub fn clear() {
    with_stdscr(|w| {
        let _ = w.clear();
    });
}

/// Erase the screen (same as clear but doesn't clear on next refresh)
pub fn erase() {
    with_stdscr(|w| {
        let _ = w.erase();
    });
}

/// Sound the terminal bell
pub fn beep() {
    with_screen(|s| {
        let _ = s.beep();
    });
}

/// Flash the screen
pub fn flash() {
    with_screen(|s| {
        let _ = s.flash();
    });
}

/// Move cursor to position
pub fn mv(y: i32, x: i32) {
    with_stdscr(|w| {
        let _ = w.mv(y, x);
    });
}

/// Add a string at current position
pub fn addstr(s: &str) -> i32 {
    with_stdscr(|w| match w.addstr(s) {
        Ok(()) => OK,
        Err(_) => ERR,
    })
}

/// Move and add a string
pub fn mvaddstr(y: i32, x: i32, s: &str) -> i32 {
    with_stdscr(|w| match w.mvaddstr(y, x, s) {
        Ok(()) => OK,
        Err(_) => ERR,
    })
}

/// Move and add a string with max length
pub fn mvaddnstr(y: i32, x: i32, s: &str, n: i32) -> i32 {
    with_stdscr(|w| match w.mvaddnstr(y, x, s, n) {
        Ok(()) => OK,
        Err(_) => ERR,
    })
}

/// Add a character at current position
pub fn addch(ch: ChType) {
    with_stdscr(|w| {
        let _ = w.addch(ch);
    });
}

/// Move and add a character
pub fn mvaddch(y: i32, x: i32, ch: ChType) {
    with_stdscr(|w| {
        let _ = w.mvaddch(y, x, ch);
    });
}

/// Draw a horizontal line
pub fn hline(ch: ChType, n: i32) {
    with_stdscr(|w| {
        let _ = w.hline(ch, n);
    });
}

/// Move and draw a horizontal line
pub fn mvhline(y: i32, x: i32, ch: ChType, n: i32) {
    with_stdscr(|w| {
        let _ = w.mv(y, x);
        let _ = w.hline(ch, n);
    });
}

/// Draw a vertical line
pub fn vline(ch: ChType, n: i32) {
    with_stdscr(|w| {
        let _ = w.vline(ch, n);
    });
}

/// Move and draw a vertical line
pub fn mvvline(y: i32, x: i32, ch: ChType, n: i32) {
    with_stdscr(|w| {
        let _ = w.mv(y, x);
        let _ = w.vline(ch, n);
    });
}

// ============================================================================
// Attributes
// ============================================================================

/// Set attributes
pub fn attrset(attr: AttrT) {
    with_stdscr(|w| {
        let _ = w.attrset(attr);
    });
}

/// Turn on attributes
pub fn attron(attr: AttrT) {
    with_stdscr(|w| {
        let _ = w.attron(attr);
    });
}

/// Turn off attributes
pub fn attroff(attr: AttrT) {
    with_stdscr(|w| {
        let _ = w.attroff(attr);
    });
}

/// Set background character
pub fn bkgdset(ch: ChType) {
    with_stdscr(|w| w.bkgdset(ch));
}

// ============================================================================
// Color support
// ============================================================================

/// Check if terminal has color support
pub fn has_colors() -> bool {
    with_screen(|s| s.has_colors())
}

/// Start color mode
pub fn start_color() {
    with_screen(|s| {
        let _ = s.start_color();
    });
}

/// Enable use of default colors (-1)
pub fn use_default_colors() {
    with_screen(|s| {
        let _ = s.use_default_colors();
    });
}

/// Initialize a color pair
pub fn init_pair(pair: i16, fg: i16, bg: i16) {
    with_screen(|s| {
        let _ = s.init_pair(pair, fg, bg);
    });
}

/// Get number of colors
pub fn COLORS() -> i32 {
    with_screen(|s| s.num_colors())
}

/// Get number of color pairs
pub fn COLOR_PAIRS() -> i32 {
    with_screen(|s| s.num_color_pairs())
}

/// Convert a color pair number to an attribute
pub fn COLOR_PAIR(n: i16) -> AttrT {
    nc::attr::color_pair(n)
}

// ============================================================================
// Input
// ============================================================================

/// Get a character
pub fn getch() -> i32 {
    with_screen(|s| s.getch().unwrap_or(ERR))
}

// ============================================================================
// Mouse support
// ============================================================================

/// Set mouse event mask
pub fn mousemask(newmask: MmaskT, _oldmask: Option<&mut MmaskT>) -> MmaskT {
    with_screen(|s| s.mousemask(newmask))
}

/// Set mouse click interval
pub fn mouseinterval(interval: i32) -> i32 {
    with_screen(|s| s.mouseinterval(interval))
}

/// Get mouse event
pub fn getmouse(event: &mut MEVENT) -> i32 {
    with_screen(|s| {
        if let Some(e) = s.getmouse() {
            event.id = e.id;
            event.x = e.x;
            event.y = e.y;
            event.z = e.z;
            event.bstate = e.bstate;
            OK
        } else {
            ERR
        }
    })
}

// ============================================================================
// Screen information
// ============================================================================

/// Get screen dimensions
pub fn getmaxyx(_win: *mut Window, y: &mut i32, x: &mut i32) {
    with_screen(|s| {
        *y = s.lines();
        *x = s.cols();
    });
}

/// Get number of lines
pub fn LINES() -> i32 {
    with_screen(|s| s.lines())
}

/// Get number of columns
pub fn COLS() -> i32 {
    with_screen(|s| s.cols())
}

// ============================================================================
// ACS character functions (for compatibility with old code that uses functions)
// ============================================================================

/// Get ACS horizontal line character
pub fn ACS_HLINE() -> ChType {
    ACS_HLINE_CHAR as ChType
}

/// Get ACS vertical line character
pub fn ACS_VLINE() -> ChType {
    ACS_VLINE_CHAR as ChType
}

/// Get ACS upper left corner character
pub fn ACS_ULCORNER() -> ChType {
    ACS_ULCORNER_CHAR as ChType
}

/// Get ACS upper right corner character
pub fn ACS_URCORNER() -> ChType {
    ACS_URCORNER_CHAR as ChType
}

/// Get ACS lower left corner character
pub fn ACS_LLCORNER() -> ChType {
    ACS_LLCORNER_CHAR as ChType
}

/// Get ACS lower right corner character
pub fn ACS_LRCORNER() -> ChType {
    ACS_LRCORNER_CHAR as ChType
}
