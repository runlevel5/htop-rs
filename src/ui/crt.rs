//! CRT - Terminal abstraction using ncurses
//!
//! This module provides the terminal interface using the ncurses library.

#![allow(dead_code)]

use ncurses::CURSOR_VISIBILITY::{CURSOR_INVISIBLE, CURSOR_VISIBLE};
use ncurses::*;

use crate::core::{ColorScheme, Settings};

/// Tree drawing characters
pub struct TreeStrings {
    pub vert: &'static str,
    pub rtee: &'static str,
    pub bend: &'static str,
    pub tend: &'static str,
    pub open: &'static str,
    pub shut: &'static str,
    pub asc: &'static str,
    pub desc: &'static str,
}

/// ASCII tree characters
pub const TREE_ASCII: TreeStrings = TreeStrings {
    vert: "|",
    rtee: "`",
    bend: "`",
    tend: ",",
    open: "+",
    shut: "-",
    asc: "^",
    desc: "v",
};

/// Unicode tree characters
pub const TREE_UTF8: TreeStrings = TreeStrings {
    vert: "\u{2502}", // │
    rtee: "\u{251c}", // ├
    bend: "\u{2514}", // └
    tend: "\u{250c}", // ┌
    open: "+",        // expand indicator (children hidden)
    shut: "\u{2500}", // ─ collapse indicator (children shown)
    asc: "\u{25b3}",  // △
    desc: "\u{25bd}", // ▽
};

/// Color elements for the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ColorElement {
    ResetColor = 0,
    DefaultColor,
    FunctionBar,
    FunctionKey,
    FailedSearch,
    FailedRead,
    Paused,
    PanelHeaderFocus,
    PanelHeaderUnfocus,
    PanelSelectionFocus,
    PanelSelectionFollow,
    PanelSelectionUnfocus,
    LargeNumber,
    MeterShadow,
    MeterText,
    MeterValue,
    MeterValueError,
    MeterValueIORead,
    MeterValueIOWrite,
    MeterValueNotice,
    MeterValueOk,
    MeterValueWarn,
    LedColor,
    Uptime,
    Battery,
    TasksRunning,
    Swap,
    SwapCache,
    SwapFrontswap,
    Process,
    ProcessShadow,
    ProcessTag,
    ProcessMegabytes,
    ProcessGigabytes,
    ProcessTree,
    ProcessRunState,
    ProcessDState,
    ProcessBasename,
    ProcessHighPriority,
    ProcessLowPriority,
    ProcessNew,
    ProcessTomb,
    ProcessThread,
    ProcessThreadBasename,
    ProcessComm,
    ProcessThreadComm,
    ProcessPriv,
    BarBorder,
    BarShadow,
    Graph1,
    Graph2,
    MemoryUsed,
    MemoryBuffers,
    MemoryBuffersText,
    MemoryCache,
    MemoryShared,
    MemoryCompressed,
    HugePage1,
    HugePage2,
    HugePage3,
    HugePage4,
    Load,
    LoadAverageFifteen,
    LoadAverageFive,
    LoadAverageOne,
    CheckBox,
    CheckMark,
    CheckText,
    Clock,
    Date,
    DateTime,
    HelpBold,
    HelpShadow,
    Hostname,
    CpuNice,
    CpuNiceText,
    CpuNormal,
    CpuSystem,
    CpuIOWait,
    CpuIrq,
    CpuSoftIrq,
    CpuSteal,
    CpuGuest,
    ScreensOthBorder,
    ScreensOthText,
    ScreensCurBorder,
    ScreensCurText,
    PressureStallTen,
    PressureStallSixty,
    PressureStallThreeHundred,
    FileDescriptorUsed,
    FileDescriptorMax,
    ZfsMfu,
    ZfsMru,
    ZfsAnon,
    ZfsHeader,
    ZfsOther,
    ZfsCompressed,
    ZfsRatio,
    ZramCompressed,
    ZramUncompressed,
    DynamicGray,
    DynamicDarkGray,
    DynamicRed,
    DynamicGreen,
    DynamicBlue,
    DynamicCyan,
    DynamicMagenta,
    DynamicYellow,
    DynamicWhite,
    PanelEdit,
    LastColorElement,
}

/// Special key definitions (using high function key numbers that are unlikely to conflict)
pub const KEY_WHEELUP: i32 = KEY_F0 + 30;
pub const KEY_WHEELDOWN: i32 = KEY_F0 + 31;
pub const KEY_RECLICK: i32 = KEY_F0 + 32;
pub const KEY_RIGHTCLICK: i32 = KEY_F0 + 33;
pub const KEY_SHIFT_TAB: i32 = KEY_F0 + 34;
pub const KEY_HEADER_CLICK: i32 = KEY_F0 + 35;
pub const KEY_DEL_MAC: i32 = 127;

/// Mouse event data
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseEvent {
    pub x: i32,
    pub y: i32,
    pub bstate: u64,
}

/// BUTTON5_PRESSED constant for wheel down (if not available in ncurses)
/// On macOS ncurses, BUTTON5 may not be defined, so we use a fallback value
/// The typical value from ncurses 6 mouse version 2 is 0x200000
const BUTTON5_PRESSED_COMPAT: u64 = 0x200000;

impl MouseEvent {
    /// Check if this was a left button release
    pub fn is_left_click(&self) -> bool {
        (self.bstate & BUTTON1_RELEASED as u64) != 0
    }

    /// Check if this was a right button release
    pub fn is_right_click(&self) -> bool {
        (self.bstate & BUTTON3_RELEASED as u64) != 0
    }

    /// Check if this was a wheel up event
    pub fn is_wheel_up(&self) -> bool {
        (self.bstate & BUTTON4_PRESSED as u64) != 0
    }

    /// Check if this was a wheel down event
    pub fn is_wheel_down(&self) -> bool {
        (self.bstate & BUTTON5_PRESSED_COMPAT) != 0
    }
}

/// Key F-number helper constants
pub const KEY_F1: i32 = KEY_F0 + 1;
pub const KEY_F2: i32 = KEY_F0 + 2;
pub const KEY_F3: i32 = KEY_F0 + 3;
pub const KEY_F4: i32 = KEY_F0 + 4;
pub const KEY_F5: i32 = KEY_F0 + 5;
pub const KEY_F6: i32 = KEY_F0 + 6;
pub const KEY_F7: i32 = KEY_F0 + 7;
pub const KEY_F8: i32 = KEY_F0 + 8;
pub const KEY_F9: i32 = KEY_F0 + 9;
pub const KEY_F10: i32 = KEY_F0 + 10;
pub const KEY_F15: i32 = KEY_F0 + 15; // Shift-F3

/// Get ALT key code
pub fn key_alt(x: char) -> i32 {
    KEY_F0 + 64 - 26 + (x.to_ascii_uppercase() as i32 - 'A' as i32)
}

/// CRT - Terminal handler
pub struct Crt {
    pub colors: Vec<attr_t>,
    pub color_scheme: ColorScheme,
    pub tree_str: &'static TreeStrings,
    pub utf8: bool,
    pub cursor_x: i32,
    pub scroll_h_amount: i32,
    pub scroll_wheel_v_amount: i32,
    screen_width: i32,
    screen_height: i32,
    delay: u32,
    mouse_enabled: bool,
    /// Last mouse event (stored for position lookup)
    last_mouse_event: Option<MouseEvent>,
}

impl Crt {
    /// Initialize the terminal
    pub fn new(settings: &Settings) -> anyhow::Result<Self> {
        // Initialize locale for UTF-8 support (must be done before initscr)
        // This is required for ncurses to properly handle wide/Unicode characters
        // Match C htop: check LC_CTYPE/LC_ALL env vars, or use "" to get system default
        unsafe {
            let lc_ctype = std::env::var("LC_CTYPE")
                .ok()
                .or_else(|| std::env::var("LC_ALL").ok());

            if let Some(lc) = lc_ctype {
                let c_str = std::ffi::CString::new(lc).unwrap_or_default();
                libc::setlocale(libc::LC_CTYPE, c_str.as_ptr());
            } else {
                libc::setlocale(libc::LC_CTYPE, b"\0".as_ptr() as *const libc::c_char);
            }
        }

        // Initialize ncurses
        initscr();
        noecho();
        cbreak();
        curs_set(CURSOR_INVISIBLE);
        keypad(stdscr(), true);

        // Enable mouse support
        if settings.enable_mouse {
            // Enable mouse events: button 1 released, button 3 released, and wheel events
            // Note: BUTTON5 may not be available on all platforms
            let mask = BUTTON1_RELEASED
                | BUTTON3_RELEASED
                | BUTTON4_PRESSED
                | BUTTON5_PRESSED_COMPAT as i32;
            mousemask(mask as mmask_t, None);
            mouseinterval(0);
        }

        // Set up colors
        if has_colors() {
            start_color();
            use_default_colors();
        }

        let utf8 = settings.allow_unicode && Self::check_utf8_support();
        let tree_str = if utf8 { &TREE_UTF8 } else { &TREE_ASCII };

        let mut crt = Crt {
            colors: vec![0; ColorElement::LastColorElement as usize],
            color_scheme: settings.color_scheme,
            tree_str,
            utf8,
            cursor_x: 0,
            scroll_h_amount: 5,
            scroll_wheel_v_amount: 10, // Match C htop CRT_scrollWheelVAmount
            screen_width: 0,
            screen_height: 0,
            delay: settings.delay,
            mouse_enabled: settings.enable_mouse,
            last_mouse_event: None,
        };

        crt.set_colors(settings.color_scheme);
        crt.update_size();
        crt.set_delay(settings.delay);

        Ok(crt)
    }

    /// Check if UTF-8 is supported
    fn check_utf8_support() -> bool {
        // Use nl_langinfo(CODESET) like C htop does - this is the most reliable method
        #[cfg(unix)]
        {
            use std::ffi::CStr;

            // Try nl_langinfo first (matches C htop behavior)
            let codeset = unsafe {
                let ptr = libc::nl_langinfo(libc::CODESET);
                if !ptr.is_null() {
                    CStr::from_ptr(ptr).to_string_lossy().to_string()
                } else {
                    String::new()
                }
            };

            if codeset.to_uppercase() == "UTF-8" || codeset.to_uppercase() == "UTF8" {
                return true;
            }
        }

        // Fallback: check locale environment variables
        if let Ok(lang) = std::env::var("LANG") {
            if lang.to_lowercase().contains("utf") {
                return true;
            }
        }
        if let Ok(lc) = std::env::var("LC_ALL") {
            if lc.to_lowercase().contains("utf") {
                return true;
            }
        }
        if let Ok(lc) = std::env::var("LC_CTYPE") {
            if lc.to_lowercase().contains("utf") {
                return true;
            }
        }
        false
    }

    /// Set up color pairs for a color scheme
    pub fn set_colors(&mut self, scheme: ColorScheme) {
        self.color_scheme = scheme;

        if !has_colors() {
            // No color support, use A_NORMAL for everything
            for color in &mut self.colors {
                *color = A_NORMAL;
            }
            return;
        }

        // Initialize basic color pairs
        match scheme {
            ColorScheme::Monochrome => self.setup_monochrome(),
            ColorScheme::BlackOnWhite => self.setup_black_on_white(),
            ColorScheme::LightTerminal => self.setup_light_terminal(),
            ColorScheme::Midnight => self.setup_midnight(),
            ColorScheme::BlackNight => self.setup_black_night(),
            ColorScheme::BrokenGray => self.setup_broken_gray(),
            ColorScheme::Nord => self.setup_nord(),
            ColorScheme::Default => self.setup_default_colors(),
        }
    }

    /// Set up default color scheme
    fn setup_default_colors(&mut self) {
        // Color pair numbers
        const PAIR_DEFAULT: i16 = 1;
        const PAIR_CYAN_BLACK: i16 = 2;
        const PAIR_GREEN_BLACK: i16 = 3;
        const PAIR_YELLOW_BLACK: i16 = 4;
        const PAIR_RED_BLACK: i16 = 5;
        const PAIR_BLUE_BLACK: i16 = 6;
        const PAIR_MAGENTA_BLACK: i16 = 7;
        const PAIR_WHITE_BLUE: i16 = 8;
        const PAIR_BLACK_GREEN: i16 = 9;
        const PAIR_BLACK_CYAN: i16 = 10;
        const PAIR_WHITE_BLACK: i16 = 11;
        const PAIR_GRAY_BLACK: i16 = 12; // For dim/shadow colors
        const PAIR_BLUE_BLUE: i16 = 13; // For inactive screen tabs
        const PAIR_GREEN_GREEN: i16 = 14; // For active screen tab border
        const PAIR_BLACK_BLUE: i16 = 15; // For inactive screen tab text
        const PAIR_BLACK_YELLOW: i16 = 16; // For PanelSelectionFollow (filter/search following)
        const PAIR_BLACK_WHITE: i16 = 17; // For PanelEdit (rename editing)

        init_pair(PAIR_DEFAULT, -1, -1);
        init_pair(PAIR_CYAN_BLACK, COLOR_CYAN, -1);
        init_pair(PAIR_GREEN_BLACK, COLOR_GREEN, -1);
        init_pair(PAIR_YELLOW_BLACK, COLOR_YELLOW, -1);
        init_pair(PAIR_RED_BLACK, COLOR_RED, -1);
        init_pair(PAIR_BLUE_BLACK, COLOR_BLUE, -1);
        init_pair(PAIR_MAGENTA_BLACK, COLOR_MAGENTA, -1);
        init_pair(PAIR_WHITE_BLUE, COLOR_WHITE, COLOR_BLUE);
        init_pair(PAIR_BLACK_GREEN, COLOR_BLACK, COLOR_GREEN);
        init_pair(PAIR_BLACK_CYAN, COLOR_BLACK, COLOR_CYAN);
        init_pair(PAIR_WHITE_BLACK, COLOR_WHITE, -1);
        init_pair(PAIR_BLUE_BLUE, COLOR_BLUE, COLOR_BLUE);
        init_pair(PAIR_GREEN_GREEN, COLOR_GREEN, COLOR_GREEN);
        init_pair(PAIR_BLACK_BLUE, COLOR_BLACK, COLOR_BLUE);
        init_pair(PAIR_BLACK_YELLOW, COLOR_BLACK, COLOR_YELLOW);
        init_pair(PAIR_BLACK_WHITE, COLOR_BLACK, COLOR_WHITE);

        // Gray/black pair: use color 8 (dark gray) if available, otherwise black
        // This matches C htop's ColorPairGrayBlack behavior
        let gray_fg = if COLORS() > 8 { 8 } else { COLOR_BLACK };
        init_pair(PAIR_GRAY_BLACK, gray_fg, -1);

        self.colors[ColorElement::DefaultColor as usize] = COLOR_PAIR(PAIR_DEFAULT);
        // FunctionBar: Black text on Cyan background (for labels like "Help", "Setup")
        self.colors[ColorElement::FunctionBar as usize] = COLOR_PAIR(PAIR_BLACK_CYAN);
        // FunctionKey: White text on default/black background (for "F1", "F2", etc.)
        self.colors[ColorElement::FunctionKey as usize] = COLOR_PAIR(PAIR_WHITE_BLACK);
        self.colors[ColorElement::PanelHeaderFocus as usize] = COLOR_PAIR(PAIR_BLACK_GREEN);
        self.colors[ColorElement::PanelHeaderUnfocus as usize] = COLOR_PAIR(PAIR_BLACK_GREEN);
        self.colors[ColorElement::PanelSelectionFocus as usize] =
            COLOR_PAIR(PAIR_BLACK_CYAN) | A_BOLD;
        self.colors[ColorElement::PanelSelectionFollow as usize] =
            COLOR_PAIR(PAIR_BLACK_YELLOW) | A_BOLD;
        self.colors[ColorElement::PanelSelectionUnfocus as usize] = COLOR_PAIR(PAIR_BLACK_CYAN);
        self.colors[ColorElement::PanelEdit as usize] = COLOR_PAIR(PAIR_BLACK_WHITE) | A_BOLD;
        self.colors[ColorElement::Process as usize] = COLOR_PAIR(PAIR_DEFAULT);
        self.colors[ColorElement::ProcessBasename as usize] = COLOR_PAIR(PAIR_DEFAULT) | A_BOLD;
        self.colors[ColorElement::ProcessTree as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::ProcessThread as usize] = COLOR_PAIR(PAIR_GREEN_BLACK);
        self.colors[ColorElement::ProcessRunState as usize] = COLOR_PAIR(PAIR_GREEN_BLACK);
        self.colors[ColorElement::ProcessDState as usize] = COLOR_PAIR(PAIR_RED_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuNormal as usize] = COLOR_PAIR(PAIR_GREEN_BLACK);
        self.colors[ColorElement::CpuSystem as usize] = COLOR_PAIR(PAIR_RED_BLACK);
        self.colors[ColorElement::CpuNice as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::CpuIOWait as usize] = COLOR_PAIR(PAIR_GRAY_BLACK) | A_BOLD; // Match C htop
        self.colors[ColorElement::CpuIrq as usize] = COLOR_PAIR(PAIR_YELLOW_BLACK);
        self.colors[ColorElement::CpuSoftIrq as usize] = COLOR_PAIR(PAIR_MAGENTA_BLACK);
        self.colors[ColorElement::CpuSteal as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::CpuGuest as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::MemoryUsed as usize] = COLOR_PAIR(PAIR_GREEN_BLACK);
        self.colors[ColorElement::MemoryBuffers as usize] = COLOR_PAIR(PAIR_BLUE_BLACK) | A_BOLD;
        self.colors[ColorElement::MemoryBuffersText as usize] =
            COLOR_PAIR(PAIR_BLUE_BLACK) | A_BOLD;
        self.colors[ColorElement::MemoryCache as usize] = COLOR_PAIR(PAIR_YELLOW_BLACK);
        self.colors[ColorElement::MemoryShared as usize] = COLOR_PAIR(PAIR_MAGENTA_BLACK);
        self.colors[ColorElement::MemoryCompressed as usize] = COLOR_PAIR(PAIR_GRAY_BLACK) | A_BOLD; // Dim gray
        self.colors[ColorElement::Swap as usize] = COLOR_PAIR(PAIR_RED_BLACK);
        self.colors[ColorElement::MeterValue as usize] = COLOR_PAIR(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterText as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::MeterShadow as usize] = COLOR_PAIR(PAIR_GRAY_BLACK) | A_BOLD; // Dim gray
        self.colors[ColorElement::BarShadow as usize] = COLOR_PAIR(PAIR_GRAY_BLACK) | A_BOLD; // Dim gray
        self.colors[ColorElement::BarBorder as usize] = A_BOLD; // Bold white for [ and ]
        self.colors[ColorElement::Graph1 as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::Graph2 as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::TasksRunning as usize] = COLOR_PAIR(PAIR_GREEN_BLACK) | A_BOLD;
        self.colors[ColorElement::Load as usize] = COLOR_PAIR(PAIR_DEFAULT);
        self.colors[ColorElement::LoadAverageOne as usize] = COLOR_PAIR(PAIR_DEFAULT) | A_BOLD; // Bold white
        self.colors[ColorElement::LoadAverageFive as usize] = COLOR_PAIR(PAIR_CYAN_BLACK) | A_BOLD; // Bold cyan
        self.colors[ColorElement::LoadAverageFifteen as usize] = COLOR_PAIR(PAIR_CYAN_BLACK); // Cyan (no bold)
        self.colors[ColorElement::Uptime as usize] = COLOR_PAIR(PAIR_CYAN_BLACK) | A_BOLD; // Bold cyan
        self.colors[ColorElement::Hostname as usize] = COLOR_PAIR(PAIR_DEFAULT) | A_BOLD;
        self.colors[ColorElement::ProcessMegabytes as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::ProcessGigabytes as usize] = COLOR_PAIR(PAIR_GREEN_BLACK);
        self.colors[ColorElement::ProcessShadow as usize] = COLOR_PAIR(PAIR_GRAY_BLACK) | A_BOLD; // Dim gray
        self.colors[ColorElement::ProcessHighPriority as usize] = COLOR_PAIR(PAIR_RED_BLACK);
        self.colors[ColorElement::ProcessLowPriority as usize] = COLOR_PAIR(PAIR_GREEN_BLACK);
        self.colors[ColorElement::ProcessPriv as usize] = COLOR_PAIR(PAIR_RED_BLACK) | A_BOLD;
        self.colors[ColorElement::LargeNumber as usize] = COLOR_PAIR(PAIR_RED_BLACK) | A_BOLD;
        self.colors[ColorElement::FailedSearch as usize] = COLOR_PAIR(PAIR_RED_BLACK) | A_BOLD;
        self.colors[ColorElement::HelpBold as usize] = COLOR_PAIR(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::HelpShadow as usize] = COLOR_PAIR(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::Clock as usize] = COLOR_PAIR(PAIR_DEFAULT);
        self.colors[ColorElement::CheckBox as usize] = COLOR_PAIR(PAIR_CYAN_BLACK);
        self.colors[ColorElement::CheckMark as usize] = COLOR_PAIR(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::CheckText as usize] = COLOR_PAIR(PAIR_DEFAULT);

        // Screen tabs colors (for "Main" tab header)
        self.colors[ColorElement::ScreensOthBorder as usize] = COLOR_PAIR(PAIR_BLUE_BLUE);
        self.colors[ColorElement::ScreensOthText as usize] = COLOR_PAIR(PAIR_BLACK_BLUE);
        self.colors[ColorElement::ScreensCurBorder as usize] = COLOR_PAIR(PAIR_GREEN_GREEN);
        self.colors[ColorElement::ScreensCurText as usize] = COLOR_PAIR(PAIR_BLACK_GREEN);
    }

    /// Set up monochrome color scheme
    fn setup_monochrome(&mut self) {
        for color in &mut self.colors {
            *color = A_NORMAL;
        }
        self.colors[ColorElement::FunctionBar as usize] = A_REVERSE;
        self.colors[ColorElement::FunctionKey as usize] = A_REVERSE | A_BOLD;
        self.colors[ColorElement::PanelHeaderFocus as usize] = A_REVERSE | A_BOLD;
        self.colors[ColorElement::PanelHeaderUnfocus as usize] = A_REVERSE;
        self.colors[ColorElement::PanelSelectionFocus as usize] = A_REVERSE | A_BOLD;
        self.colors[ColorElement::PanelSelectionUnfocus as usize] = A_REVERSE;
        self.colors[ColorElement::ProcessBasename as usize] = A_BOLD;
        self.colors[ColorElement::HelpBold as usize] = A_BOLD;
    }

    fn setup_black_on_white(&mut self) {
        // Simplified - use reverse video
        self.setup_monochrome();
    }

    fn setup_light_terminal(&mut self) {
        self.setup_default_colors();
    }

    fn setup_midnight(&mut self) {
        self.setup_default_colors();
    }

    fn setup_black_night(&mut self) {
        self.setup_default_colors();
    }

    fn setup_broken_gray(&mut self) {
        self.setup_default_colors();
    }

    fn setup_nord(&mut self) {
        self.setup_default_colors();
    }

    /// Get color attribute for an element
    pub fn color(&self, element: ColorElement) -> attr_t {
        self.colors
            .get(element as usize)
            .copied()
            .unwrap_or(A_NORMAL)
    }

    /// Update screen dimensions
    pub fn update_size(&mut self) {
        getmaxyx(stdscr(), &mut self.screen_height, &mut self.screen_width);
    }

    /// Get screen width
    pub fn width(&self) -> i32 {
        self.screen_width
    }

    /// Get screen height
    pub fn height(&self) -> i32 {
        self.screen_height
    }

    /// Set input delay
    pub fn set_delay(&mut self, delay: u32) {
        self.delay = delay;
        // Convert tenths of seconds to milliseconds for timeout
        let timeout_ms = (delay as i32 * 100).min(25500);
        ncurses::timeout(timeout_ms);
    }

    /// Disable input delay (for instant response)
    pub fn disable_delay(&self) {
        nodelay(stdscr(), true);
    }

    /// Enable input delay
    pub fn enable_delay(&self) {
        nodelay(stdscr(), false);
        let timeout_ms = (self.delay as i32 * 100).min(25500);
        ncurses::timeout(timeout_ms);
    }

    /// Read a key from input
    pub fn read_key(&self) -> Option<i32> {
        let ch = getch();
        if ch == ERR {
            None
        } else {
            Some(ch)
        }
    }

    /// Get mouse event after KEY_MOUSE was returned and store it
    pub fn get_mouse_event(&mut self) -> Option<MouseEvent> {
        if !self.mouse_enabled {
            return None;
        }

        let mut mevent = MEVENT {
            id: 0,
            x: 0,
            y: 0,
            z: 0,
            bstate: 0,
        };

        let result = getmouse(&mut mevent);
        if result == OK {
            let event = MouseEvent {
                x: mevent.x,
                y: mevent.y,
                bstate: mevent.bstate as u64,
            };
            self.last_mouse_event = Some(event);
            Some(event)
        } else {
            None
        }
    }

    /// Get the last stored mouse event
    pub fn last_mouse_event(&self) -> Option<MouseEvent> {
        self.last_mouse_event
    }

    /// Process a mouse event and convert to a key code
    /// This is called when KEY_MOUSE is received
    /// panel_y is the y position of the panel header row
    pub fn process_mouse_event(&mut self, screen_height: i32, panel_y: Option<i32>) -> Option<i32> {
        if let Some(event) = self.get_mouse_event() {
            if event.is_wheel_up() {
                return Some(KEY_WHEELUP);
            } else if event.is_wheel_down() {
                return Some(KEY_WHEELDOWN);
            } else if event.is_right_click() {
                return Some(KEY_RIGHTCLICK);
            } else if event.is_left_click() {
                // Check if click is on function bar (bottom row)
                if event.y == screen_height - 1 {
                    // Synthesize function key based on X position
                    // Each button is typically 6 chars wide
                    let button_width = 6;
                    let button_num = event.x / button_width;
                    if button_num < 10 {
                        return Some(KEY_F1 + button_num);
                    }
                }
                // Check if click is on panel header
                if let Some(py) = panel_y {
                    if event.y == py {
                        return Some(KEY_HEADER_CLICK);
                    }
                }
                // Return KEY_MOUSE for other left clicks (panel handling)
                return Some(KEY_MOUSE);
            }
        }
        None
    }

    /// Convert a mouse event to a key code (wheel events only)
    /// This is a simpler version that doesn't store the event, useful for menus
    /// Call this when KEY_MOUSE is received from getch()
    pub fn convert_mouse_to_key() -> Option<i32> {
        let mut mevent = MEVENT {
            id: 0,
            x: 0,
            y: 0,
            z: 0,
            bstate: 0,
        };

        if getmouse(&mut mevent) == OK {
            let event = MouseEvent {
                x: mevent.x,
                y: mevent.y,
                bstate: mevent.bstate as u64,
            };
            if event.is_wheel_up() {
                return Some(KEY_WHEELUP);
            } else if event.is_wheel_down() {
                return Some(KEY_WHEELDOWN);
            }
        }
        None
    }

    /// Check if mouse is enabled
    pub fn is_mouse_enabled(&self) -> bool {
        self.mouse_enabled
    }

    /// Get scroll wheel amount
    pub fn scroll_wheel_amount(&self) -> i32 {
        self.scroll_wheel_v_amount as i32
    }

    /// Clear the screen
    pub fn clear(&self) {
        clear();
    }

    /// Refresh the screen
    pub fn refresh(&self) {
        refresh();
    }

    /// Move cursor
    pub fn move_cursor(&self, y: i32, x: i32) {
        mv(y, x);
    }

    /// Print a string with attributes
    pub fn print_at(&self, y: i32, x: i32, attr: attr_t, text: &str) {
        attron(attr);
        let _ = mvaddstr(y, x, text);
        attroff(attr);
    }

    /// Print a string with a specific color element
    pub fn print_colored(&self, y: i32, x: i32, element: ColorElement, text: &str) {
        self.print_at(y, x, self.color(element), text);
    }

    /// Draw a horizontal line
    pub fn hline(&self, y: i32, x: i32, ch: u32, n: i32) {
        mv(y, x);
        hline(ch, n);
    }

    /// Draw a box
    pub fn draw_box(&self, y: i32, x: i32, h: i32, w: i32) {
        // Top border
        self.hline(y, x, ACS_HLINE(), w);
        mvaddch(y, x, ACS_ULCORNER());
        mvaddch(y, x + w - 1, ACS_URCORNER());

        // Side borders
        for i in 1..h - 1 {
            mvaddch(y + i, x, ACS_VLINE());
            mvaddch(y + i, x + w - 1, ACS_VLINE());
        }

        // Bottom border
        self.hline(y + h - 1, x, ACS_HLINE(), w);
        mvaddch(y + h - 1, x, ACS_LLCORNER());
        mvaddch(y + h - 1, x + w - 1, ACS_LRCORNER());
    }

    /// Clean up terminal
    pub fn done(&self) {
        curs_set(CURSOR_VISIBLE);
        endwin();
    }

    /// Set mouse enabled state
    pub fn set_mouse(&mut self, enabled: bool) {
        self.mouse_enabled = enabled;
        if enabled {
            let mask = BUTTON1_RELEASED
                | BUTTON3_RELEASED
                | BUTTON4_PRESSED
                | BUTTON5_PRESSED_COMPAT as i32;
            mousemask(mask as mmask_t, None);
        } else {
            mousemask(0, None);
        }
    }

    /// Check if colors are available
    pub fn has_colors(&self) -> bool {
        has_colors()
    }

    /// Handle terminal resize
    pub fn handle_resize(&mut self) {
        self.update_size();
    }

    /// Set color scheme and update display
    pub fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.set_colors(scheme);
    }
}

impl Drop for Crt {
    fn drop(&mut self) {
        self.done();
    }
}
