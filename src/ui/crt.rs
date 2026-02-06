//! CRT - Terminal abstraction using ncurses
//!
//! This module provides the terminal interface using the pure Rust ncurses library.
//! The Crt struct owns the ncurses Screen directly and provides methods for all
//! terminal operations.

#![allow(dead_code)]

pub use ncurses::acs::{
    ACS_HLINE, ACS_LLCORNER, ACS_LRCORNER, ACS_ULCORNER, ACS_URCORNER, ACS_VLINE,
};
pub use ncurses::attr::{self, A_BOLD, A_DIM, A_NORMAL, A_REVERSE};
pub use ncurses::color::{
    COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_GREEN, COLOR_MAGENTA, COLOR_RED, COLOR_WHITE,
    COLOR_YELLOW,
};
pub use ncurses::key::{
    KEY_BACKSPACE, KEY_DC, KEY_DOWN, KEY_END, KEY_ENTER, KEY_F0, KEY_HOME, KEY_LEFT, KEY_MOUSE,
    KEY_NPAGE, KEY_PPAGE, KEY_RESIZE, KEY_RIGHT, KEY_UP,
};
pub use ncurses::mouse::{
    BUTTON1_DOUBLE_CLICKED, BUTTON1_RELEASED, BUTTON2_RELEASED, BUTTON3_RELEASED, BUTTON4_PRESSED,
    BUTTON5_PRESSED,
};
pub use ncurses::types::{AttrT, ERR};
pub use ncurses::Screen;
pub use ncurses::Window;

use crate::core::{ColorScheme, Settings};

// Type alias for attribute type (matches ncurses convention)
#[allow(non_camel_case_types)]
pub type attr_t = AttrT;

/// Cursor visibility constants
pub const CURSOR_INVISIBLE: i32 = 0;
pub const CURSOR_VISIBLE: i32 = 1;
#[allow(dead_code)]
pub const CURSOR_VERY_VISIBLE: i32 = 2;

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

// =============================================================================
// Theme - Encapsulates theme-specific rendering behavior
// =============================================================================
//
// The Theme struct provides theme-agnostic rendering by encapsulating all
// theme-specific behavior (bar characters, help text, etc.) rather than having
// components check which color scheme is active.

/// Bar meter characters for different themes
/// Monochrome uses different characters to distinguish segments: |, #, *, @, $, %, &, .
/// Colored themes use '|' for all segments (colors distinguish them)
const BAR_CHARS_MONOCHROME: [char; 8] = ['|', '#', '*', '@', '$', '%', '&', '.'];
const BAR_CHARS_COLORED: [char; 8] = ['|', '|', '|', '|', '|', '|', '|', '|'];

/// Theme-specific help text for the monochrome theme
const MONOCHROME_HELP_TEXT: &str =
    "In monochrome, meters display as different chars, in order: |#*@$%&.";

/// Theme configuration - encapsulates all theme-specific rendering behavior
///
/// Components should use Theme methods instead of checking which ColorScheme is active.
/// This allows themes to be added or modified without changing component code.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Characters used for bar meter segments (index 0-7)
    bar_chars: [char; 8],
    /// Optional extra help text shown in the help screen
    help_text: Option<&'static str>,
}

impl Theme {
    /// Create theme configuration for a color scheme
    pub fn from_color_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Monochrome => Self {
                bar_chars: BAR_CHARS_MONOCHROME,
                help_text: Some(MONOCHROME_HELP_TEXT),
            },
            // All colored themes use the same bar characters
            _ => Self {
                bar_chars: BAR_CHARS_COLORED,
                help_text: None,
            },
        }
    }

    /// Get the bar character for a given value index
    ///
    /// In Monochrome mode, each segment gets a different character to distinguish them.
    /// In colored modes, all segments use '|' since colors distinguish them.
    #[inline]
    pub fn bar_char(&self, value_index: usize) -> char {
        self.bar_chars.get(value_index).copied().unwrap_or('|')
    }

    /// Get optional theme-specific help text
    ///
    /// Returns extra text to display in the help screen, if any.
    /// For example, Monochrome theme explains the bar character meanings.
    #[inline]
    pub fn help_text(&self) -> Option<&'static str> {
        self.help_text
    }
}

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
    Disabled, // Dimmed/disabled text (e.g., for actions requiring root)
    Last,     // Sentinel value for array sizing
}

/// Special key definitions (using high function key numbers that are unlikely to conflict)
pub const KEY_WHEELUP: i32 = KEY_F0 + 30;
pub const KEY_WHEELDOWN: i32 = KEY_F0 + 31;
pub const KEY_RECLICK: i32 = KEY_F0 + 32;
pub const KEY_RIGHTCLICK: i32 = KEY_F0 + 33;
pub const KEY_SHIFT_TAB: i32 = KEY_F0 + 34;
pub const KEY_HEADER_CLICK: i32 = KEY_F0 + 35;
/// Tab click - the actual tab index is encoded as KEY_TAB_CLICK + tab_index
/// Use key - KEY_TAB_CLICK to get the tab index (0-based)
pub const KEY_TAB_CLICK: i32 = KEY_F0 + 100;
pub const KEY_DEL_MAC: i32 = 127;

// Function key constants (re-export for convenience)
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
pub const KEY_F15: i32 = KEY_F0 + 15;

/// Shift+F3 (used for reverse search)
pub const KEY_SF3: i32 = KEY_F15;

// =============================================================================
// ASCII Key Constants (more readable than hex literals)
// =============================================================================
// Control characters
pub const KEY_CTRL_BS: i32 = 0x08; // Ctrl+Backspace (some terminals)
pub const KEY_CTRL_E: i32 = 0x05; // Ctrl+E
pub const KEY_TAB: i32 = 0x09; // Tab
pub const KEY_LINEFEED: i32 = 0x0A; // Line feed (Enter on some terminals)
pub const KEY_CTRL_L: i32 = 0x0C; // Ctrl+L (refresh screen)
pub const KEY_RETURN: i32 = 0x0D; // Carriage return (Enter)
pub const KEY_CTRL_N: i32 = 0x0E; // Ctrl+N (down in Emacs)
pub const KEY_CTRL_P: i32 = 0x10; // Ctrl+P (up in Emacs)
pub const KEY_CTRL_U: i32 = 0x15; // Ctrl+U (clear line)
pub const KEY_ESC: i32 = 0x1B; // Escape

// Printable characters
pub const KEY_SPACE: i32 = 0x20; // Space
pub const KEY_HASH: i32 = 0x23; // '#'
pub const KEY_STAR: i32 = 0x2A; // '*'
pub const KEY_PLUS: i32 = 0x2B; // '+'
pub const KEY_MINUS: i32 = 0x2D; // '-'
pub const KEY_DOT: i32 = 0x2E; // '.'
pub const KEY_SLASH: i32 = 0x2F; // '/'
pub const KEY_0: i32 = 0x30; // '0'
pub const KEY_9: i32 = 0x39; // '9'
pub const KEY_LT: i32 = 0x3C; // '<'
pub const KEY_GT: i32 = 0x3E; // '>'
pub const KEY_QUESTION: i32 = 0x3F; // '?'

// Uppercase letters
pub const KEY_C: i32 = 0x43; // 'C'
pub const KEY_F: i32 = 0x46; // 'F'
pub const KEY_H: i32 = 0x48; // 'H'
pub const KEY_I: i32 = 0x49; // 'I'
pub const KEY_K: i32 = 0x4B; // 'K'
pub const KEY_M: i32 = 0x4D; // 'M'
pub const KEY_N: i32 = 0x4E; // 'N'
pub const KEY_O: i32 = 0x4F; // 'O'
pub const KEY_P: i32 = 0x50; // 'P'
pub const KEY_Q: i32 = 0x51; // 'Q'
pub const KEY_S: i32 = 0x53; // 'S'
pub const KEY_T: i32 = 0x54; // 'T'
pub const KEY_U: i32 = 0x55; // 'U'
pub const KEY_Z: i32 = 0x5A; // 'Z'
pub const KEY_LBRACKET: i32 = 0x5B; // '['
pub const KEY_BACKSLASH: i32 = 0x5C; // '\\'
pub const KEY_RBRACKET: i32 = 0x5D; // ']'

// Lowercase letters
pub const KEY_LC_C: i32 = 0x63; // 'c'
pub const KEY_LC_E: i32 = 0x65; // 'e'
pub const KEY_LC_F: i32 = 0x66; // 'f'
pub const KEY_LC_H: i32 = 0x68; // 'h'
pub const KEY_LC_K: i32 = 0x6B; // 'k'
pub const KEY_LC_L: i32 = 0x6C; // 'l'
pub const KEY_LC_M: i32 = 0x6D; // 'm'
pub const KEY_LC_P: i32 = 0x70; // 'p'
pub const KEY_LC_Q: i32 = 0x71; // 'q'
pub const KEY_LC_S: i32 = 0x73; // 's'
pub const KEY_LC_T: i32 = 0x74; // 't'
pub const KEY_LC_U: i32 = 0x75; // 'u'
pub const KEY_LC_W: i32 = 0x77; // 'w'
pub const KEY_LC_X: i32 = 0x78; // 'x'

// Character ranges (for matching)
pub const KEY_PRINTABLE_START: i32 = 0x20;
pub const KEY_PRINTABLE_END: i32 = 0x7F;

// =============================================================================
// Global Color Pair System (matches C htop's ColorIndex/ColorPair scheme)
// =============================================================================
//
// C htop uses ColorIndex(i,j) = (7-i)*8+j to compute unique pair indices for
// all foreground/background color combinations. This ensures all 64 pairs
// are consistently numbered across ALL color schemes, preventing issues when
// switching between schemes.
//
// NOTE: Color pair 0 cannot be redefined by ncurses - it always represents
// the terminal's default colors. The formula produces index 0 for white-on-black
// (ColorIndex(7,0) = 0), so we need special handling to avoid pair 0.

/// Compute color pair index for a foreground/background combination
/// Matches C htop's ColorIndex(i,j) = (7-i)*8+j
/// NOTE: This can return 0 for white-on-black, which is pair 0 (terminal default)
#[inline]
const fn color_index(fg: i16, bg: i16) -> i16 {
    (7 - fg) * 8 + bg
}

// Special color pair indices (match C htop's ColorPairGrayBlack and ColorPairWhiteDefault)
const COLOR_INDEX_GRAY_BLACK: i16 = color_index(COLOR_MAGENTA, COLOR_MAGENTA);
const COLOR_INDEX_WHITE_DEFAULT: i16 = color_index(COLOR_RED, COLOR_RED);
// Special index for white-on-black to avoid pair 0 (which can't be customized)
// We use an unused slot: yellow-on-yellow (index 35) - not used by any color scheme
// Note: cyan-on-cyan (14) is used by MC theme for ScreensCurBorder, so we can't use that
const COLOR_INDEX_WHITE_BLACK: i16 = color_index(COLOR_YELLOW, COLOR_YELLOW);

/// Convert a color pair number to an attribute (replacement for ncurses COLOR_PAIR macro)
#[inline]
fn color_pair_attr(n: i16) -> attr_t {
    attr::color_pair(n)
}

/// Get COLOR_PAIR for a foreground/background combination
/// Matches C htop's ColorPair(i,j)
/// NOTE: ColorIndex(White, Black) = 0, and pair 0 cannot be customized by ncurses.
/// We redirect white-on-black to use COLOR_INDEX_WHITE_BLACK which is properly initialized.
#[inline]
fn color_pair(fg: i16, bg: i16) -> attr_t {
    let idx = color_index(fg, bg);
    // Pair 0 (white-on-black) cannot be customized, so redirect to our special pair
    if idx == 0 {
        color_pair_attr(COLOR_INDEX_WHITE_BLACK)
    } else {
        color_pair_attr(idx)
    }
}

/// Get the GrayBlack color pair (for shadow/dim colors)
#[inline]
fn color_pair_gray_black() -> attr_t {
    color_pair_attr(COLOR_INDEX_GRAY_BLACK)
}

/// Get the WhiteDefault color pair (white on default background)
#[inline]
fn color_pair_white_default() -> attr_t {
    color_pair_attr(COLOR_INDEX_WHITE_DEFAULT)
}

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

    /// Check if this was a left button double-click
    pub fn is_double_click(&self) -> bool {
        (self.bstate & BUTTON1_DOUBLE_CLICKED as u64) != 0
    }

    /// Check if this was a middle button release
    pub fn is_middle_click(&self) -> bool {
        (self.bstate & BUTTON2_RELEASED as u64) != 0
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

// Note: KEY_F1-F10, KEY_SF3 defined with other key constants above (lines 229-241)

/// Get ALT key code
pub fn key_alt(x: char) -> i32 {
    KEY_F0 + 64 - 26 + (x.to_ascii_uppercase() as i32 - 'A' as i32)
}

/// CRT - Terminal handler
///
/// This struct provides terminal operations via ncurses.
/// It owns the ncurses Screen directly and provides methods for drawing.
///
/// Pass `&mut Crt` to functions that need to draw. Use `crt.with_window()`
/// for drawing operations.
pub struct Crt {
    /// The ncurses Screen - owned directly by Crt
    screen: Screen,
    pub colors: Vec<attr_t>,
    pub color_scheme: ColorScheme,
    /// Theme configuration - provides theme-agnostic access to rendering behavior
    pub theme: Theme,
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
                libc::setlocale(libc::LC_ALL, c_str.as_ptr());
            } else {
                libc::setlocale(libc::LC_ALL, c"".as_ptr());
            }
        }

        // Initialize ncurses - Screen is now owned directly by Crt
        let mut screen = Screen::init().expect("Failed to initialize ncurses");

        // Set cursor invisible
        let _ = screen.curs_set(CURSOR_INVISIBLE);

        // Enable keypad mode on stdscr
        screen.keypad(true);

        // Enable mouse support
        if settings.enable_mouse {
            // Enable mouse events: button releases, double-click, and wheel events
            let mask = BUTTON1_RELEASED
                | BUTTON1_DOUBLE_CLICKED
                | BUTTON2_RELEASED
                | BUTTON3_RELEASED
                | BUTTON4_PRESSED
                | BUTTON5_PRESSED;
            screen.mousemask(mask);
            screen.mouseinterval(0);
        }

        // Set up colors
        if screen.has_colors() {
            if let Err(e) = screen.start_color() {
                eprintln!("Warning: Failed to start colors: {:?}", e);
            }
            if let Err(e) = screen.use_default_colors() {
                eprintln!("Warning: Failed to enable default colors: {:?}", e);
            }
        }

        let utf8 = settings.allow_unicode && Self::check_utf8_support();
        let tree_str = if utf8 { &TREE_UTF8 } else { &TREE_ASCII };

        // Get initial screen dimensions
        let screen_height = screen.lines();
        let screen_width = screen.cols();

        let mut crt = Crt {
            screen,
            colors: vec![0; ColorElement::Last as usize],
            color_scheme: settings.color_scheme,
            theme: Theme::from_color_scheme(settings.color_scheme),
            tree_str,
            utf8,
            cursor_x: 0,
            scroll_h_amount: 5,
            scroll_wheel_v_amount: 10, // Match C htop CRT_scrollWheelVAmount
            screen_width,
            screen_height,
            delay: settings.delay,
            mouse_enabled: settings.enable_mouse,
            last_mouse_event: None,
        };

        crt.set_colors(settings.color_scheme);
        crt.set_delay(settings.delay);

        Ok(crt)
    }

    /// Execute a function with the window for drawing operations.
    /// This is the primary way for components to draw to the screen.
    #[inline]
    pub fn with_window<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Window) -> R,
    {
        f(self.screen.stdscr_mut())
    }

    /// Execute a function with direct access to the Screen.
    /// Use this for operations that need screen-level access (not just window drawing).
    #[inline]
    pub fn with_screen<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Screen) -> R,
    {
        f(&mut self.screen)
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
    /// This matches C htop's CRT_setColors which initializes ALL color pairs globally
    pub fn set_colors(&mut self, scheme: ColorScheme) {
        self.color_scheme = scheme;
        self.theme = Theme::from_color_scheme(scheme);

        if !self.screen.has_colors() {
            // No color support, use A_NORMAL for everything
            for color in &mut self.colors {
                *color = A_NORMAL;
            }
            return;
        }

        // Initialize ALL color pairs globally (matches C htop's approach)
        // This ensures consistent color pair indices across all schemes
        // ColorIndex(i,j) = (7-i)*8+j maps each fg/bg combination to a unique pair number
        let debug_colors = std::env::var("HTOP_DEBUG_COLORS").is_ok();

        if debug_colors {
            eprintln!(
                "DEBUG: Initializing colors, num_colors={}, num_pairs={}",
                self.screen.num_colors(),
                self.screen.num_color_pairs()
            );
        }

        // Match C htop's behavior: use -1 (terminal default) for background when bg=0 (black)
        // except for BlackNight scheme which uses explicit black
        let use_default_bg = scheme != ColorScheme::BlackNight;

        for i in 0i16..8 {
            for j in 0i16..8 {
                let idx = color_index(i, j);
                // Skip special pairs (GrayBlack, WhiteDefault, and WhiteBlack have special handling)
                // Also skip pair 0 (white-on-black via formula) as it can't be customized
                if idx != COLOR_INDEX_GRAY_BLACK
                    && idx != COLOR_INDEX_WHITE_DEFAULT
                    && idx != COLOR_INDEX_WHITE_BLACK
                    && idx != 0
                {
                    // Use -1 (terminal default) for black background, matching C htop
                    let bg = if use_default_bg && j == 0 { -1 } else { j };
                    if let Err(e) = self.screen.init_pair(idx, i, bg) {
                        if debug_colors {
                            eprintln!(
                                "DEBUG: init_pair({}, fg={}, bg={}) failed: {:?}",
                                idx, i, bg, e
                            );
                        }
                    }
                }
            }
        }

        // Special handling for GrayBlack pair (uses color 8 if available for dark gray)
        let num_colors = self.screen.num_colors();
        let gray_fg = if num_colors > 8 { 8 } else { COLOR_BLACK };
        // Use -1 (terminal default) for background, matching C htop
        let gray_bg = if use_default_bg { -1 } else { COLOR_BLACK };
        if let Err(e) = self
            .screen
            .init_pair(COLOR_INDEX_GRAY_BLACK, gray_fg, gray_bg)
        {
            if debug_colors {
                eprintln!(
                    "DEBUG: init_pair(GRAY_BLACK={}, fg={}, bg={}) failed: {:?}",
                    COLOR_INDEX_GRAY_BLACK, gray_fg, gray_bg, e
                );
            }
        }

        // WhiteDefault pair (white on terminal default background)
        if let Err(e) = self
            .screen
            .init_pair(COLOR_INDEX_WHITE_DEFAULT, COLOR_WHITE, -1)
        {
            if debug_colors {
                eprintln!(
                    "DEBUG: init_pair(WHITE_DEFAULT={}, fg={}, bg=-1) failed: {:?}",
                    COLOR_INDEX_WHITE_DEFAULT, COLOR_WHITE, e
                );
            }
        }

        // WhiteBlack pair - replacement for pair 0 (white-on-black) which can't be customized
        // Use terminal default background (-1) for non-BlackNight schemes, matching C htop
        let white_black_bg = if use_default_bg { -1 } else { COLOR_BLACK };
        if let Err(e) = self
            .screen
            .init_pair(COLOR_INDEX_WHITE_BLACK, COLOR_WHITE, white_black_bg)
        {
            if debug_colors {
                eprintln!(
                    "DEBUG: init_pair(WHITE_BLACK={}, fg={}, bg={}) failed: {:?}",
                    COLOR_INDEX_WHITE_BLACK, COLOR_WHITE, white_black_bg, e
                );
            }
        }

        // Now set up the color scheme
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
        // All color pairs are already initialized in set_colors()
        // Just assign color elements using the global color_pair() function

        // Use A_NORMAL for ResetColor and DefaultColor to get terminal's native pair 0
        // (bright white on default background), matching C htop's COLOR_PAIR(0) behavior
        self.colors[ColorElement::ResetColor as usize] = A_NORMAL;
        self.colors[ColorElement::DefaultColor as usize] = A_NORMAL;
        // FunctionBar: Black text on Cyan background (for labels like "Help", "Setup")
        self.colors[ColorElement::FunctionBar as usize] = color_pair(COLOR_BLACK, COLOR_CYAN);
        // FunctionKey: Use A_NORMAL for terminal's native white (matching C htop's COLOR_PAIR(0))
        self.colors[ColorElement::FunctionKey as usize] = A_NORMAL;
        // Disabled: Gray text on cyan background (dimmed function bar items)
        self.colors[ColorElement::Disabled as usize] = color_pair_gray_black();
        self.colors[ColorElement::PanelHeaderFocus as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::PanelHeaderUnfocus as usize] =
            color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::PanelSelectionFocus as usize] =
            color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::PanelSelectionFollow as usize] =
            color_pair(COLOR_BLACK, COLOR_YELLOW);
        self.colors[ColorElement::PanelSelectionUnfocus as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::PanelEdit as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::FailedSearch as usize] = color_pair(COLOR_RED, COLOR_CYAN);
        self.colors[ColorElement::FailedRead as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::Paused as usize] = color_pair(COLOR_YELLOW, COLOR_CYAN) | A_BOLD;
        self.colors[ColorElement::Uptime as usize] = color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::Battery as usize] = color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::LargeNumber as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::MeterText as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::MeterValue as usize] =
            color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterValueError as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterValueIORead as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MeterValueIOWrite as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterValueNotice as usize] =
            color_pair(COLOR_WHITE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterValueOk as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MeterValueWarn as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::LedColor as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::TasksRunning as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::Process as usize] = A_NORMAL;
        self.colors[ColorElement::ProcessShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::ProcessTag as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessMegabytes as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::ProcessGigabytes as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessBasename as usize] =
            color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessTree as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::ProcessThread as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessThreadBasename as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessComm as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::ProcessThreadComm as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::ProcessRunState as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessDState as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessHighPriority as usize] =
            color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::ProcessLowPriority as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessNew as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::ProcessTomb as usize] = color_pair(COLOR_BLACK, COLOR_RED);
        self.colors[ColorElement::ProcessPriv as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::BarBorder as usize] = A_BOLD;
        self.colors[ColorElement::BarShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::Swap as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::SwapCache as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::SwapFrontswap as usize] = color_pair_gray_black();
        self.colors[ColorElement::Graph1 as usize] = color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::Graph2 as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::MemoryUsed as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MemoryBuffers as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::MemoryBuffersText as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MemoryCache as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::MemoryShared as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::MemoryCompressed as usize] = color_pair_gray_black();
        self.colors[ColorElement::Load as usize] = A_BOLD;
        self.colors[ColorElement::LoadAverageOne as usize] = A_BOLD;
        self.colors[ColorElement::LoadAverageFive as usize] =
            color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::LoadAverageFifteen as usize] =
            color_pair(COLOR_CYAN, COLOR_BLACK) | A_DIM;
        self.colors[ColorElement::HelpBold as usize] = color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::HelpShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::Clock as usize] = color_pair(COLOR_WHITE, COLOR_BLACK);
        self.colors[ColorElement::Date as usize] = color_pair(COLOR_WHITE, COLOR_BLACK);
        self.colors[ColorElement::DateTime as usize] = color_pair(COLOR_WHITE, COLOR_BLACK);
        self.colors[ColorElement::CheckBox as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::CheckMark as usize] = A_BOLD;
        self.colors[ColorElement::CheckText as usize] = A_NORMAL;
        self.colors[ColorElement::Hostname as usize] = A_BOLD;
        self.colors[ColorElement::CpuNice as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::CpuNiceText as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuNormal as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::CpuSystem as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::CpuIOWait as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::CpuIrq as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::CpuSoftIrq as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::CpuSteal as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::CpuGuest as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);

        // Screen tabs colors
        self.colors[ColorElement::ScreensOthBorder as usize] = color_pair(COLOR_BLUE, COLOR_BLUE);
        self.colors[ColorElement::ScreensOthText as usize] = color_pair(COLOR_BLACK, COLOR_BLUE);
        self.colors[ColorElement::ScreensCurBorder as usize] = color_pair(COLOR_GREEN, COLOR_GREEN);
        self.colors[ColorElement::ScreensCurText as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);

        // File descriptor colors
        self.colors[ColorElement::FileDescriptorUsed as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::FileDescriptorMax as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;

        // Dynamic colors
        self.colors[ColorElement::DynamicGray as usize] = color_pair_gray_black();
        self.colors[ColorElement::DynamicDarkGray as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::DynamicRed as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::DynamicGreen as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::DynamicBlue as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::DynamicCyan as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::DynamicMagenta as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::DynamicYellow as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::DynamicWhite as usize] =
            color_pair(COLOR_WHITE, COLOR_BLACK) | A_BOLD;
    }

    /// Set up monochrome color scheme
    fn setup_monochrome(&mut self) {
        // Monochrome uses terminal attributes (A_BOLD, A_DIM, A_REVERSE) instead of colors
        // Initialize all to A_NORMAL first
        for color in &mut self.colors {
            *color = A_NORMAL;
        }

        // Function bar and keys
        self.colors[ColorElement::FunctionBar as usize] = A_REVERSE;
        self.colors[ColorElement::FunctionKey as usize] = A_NORMAL;

        // Panel headers and selection
        self.colors[ColorElement::PanelHeaderFocus as usize] = A_REVERSE;
        self.colors[ColorElement::PanelHeaderUnfocus as usize] = A_REVERSE;
        self.colors[ColorElement::PanelSelectionFocus as usize] = A_REVERSE;
        self.colors[ColorElement::PanelSelectionFollow as usize] = A_REVERSE;
        self.colors[ColorElement::PanelSelectionUnfocus as usize] = A_BOLD;
        self.colors[ColorElement::PanelEdit as usize] = A_BOLD;
        self.colors[ColorElement::Disabled as usize] = A_DIM;

        // Search/errors
        self.colors[ColorElement::FailedSearch as usize] = A_REVERSE | A_BOLD;
        self.colors[ColorElement::FailedRead as usize] = A_BOLD;
        self.colors[ColorElement::Paused as usize] = A_BOLD | A_REVERSE;

        // Meter elements
        self.colors[ColorElement::MeterShadow as usize] = A_DIM;
        self.colors[ColorElement::MeterText as usize] = A_NORMAL;
        self.colors[ColorElement::MeterValue as usize] = A_BOLD;
        self.colors[ColorElement::MeterValueError as usize] = A_BOLD;
        self.colors[ColorElement::MeterValueNotice as usize] = A_BOLD;
        self.colors[ColorElement::MeterValueWarn as usize] = A_BOLD;
        self.colors[ColorElement::LedColor as usize] = A_NORMAL;

        // Status elements
        self.colors[ColorElement::Uptime as usize] = A_BOLD;
        self.colors[ColorElement::Battery as usize] = A_BOLD;
        self.colors[ColorElement::LargeNumber as usize] = A_BOLD;
        self.colors[ColorElement::TasksRunning as usize] = A_BOLD;

        // Process elements
        self.colors[ColorElement::ProcessShadow as usize] = A_DIM;
        self.colors[ColorElement::ProcessTag as usize] = A_BOLD;
        self.colors[ColorElement::ProcessMegabytes as usize] = A_BOLD;
        self.colors[ColorElement::ProcessGigabytes as usize] = A_BOLD;
        self.colors[ColorElement::ProcessBasename as usize] = A_BOLD;
        self.colors[ColorElement::ProcessTree as usize] = A_BOLD;
        self.colors[ColorElement::ProcessRunState as usize] = A_BOLD;
        self.colors[ColorElement::ProcessDState as usize] = A_BOLD;
        self.colors[ColorElement::ProcessHighPriority as usize] = A_BOLD;
        self.colors[ColorElement::ProcessLowPriority as usize] = A_DIM;
        self.colors[ColorElement::ProcessNew as usize] = A_BOLD;
        self.colors[ColorElement::ProcessTomb as usize] = A_DIM;
        self.colors[ColorElement::ProcessThread as usize] = A_BOLD;
        self.colors[ColorElement::ProcessThreadBasename as usize] = A_REVERSE;
        self.colors[ColorElement::ProcessComm as usize] = A_BOLD;
        self.colors[ColorElement::ProcessThreadComm as usize] = A_REVERSE;
        self.colors[ColorElement::ProcessPriv as usize] = A_BOLD;

        // Bar elements
        self.colors[ColorElement::BarBorder as usize] = A_BOLD;
        self.colors[ColorElement::BarShadow as usize] = A_DIM;

        // Memory/Swap elements
        self.colors[ColorElement::Swap as usize] = A_BOLD;
        self.colors[ColorElement::SwapCache as usize] = A_NORMAL;
        self.colors[ColorElement::SwapFrontswap as usize] = A_DIM;
        self.colors[ColorElement::MemoryUsed as usize] = A_BOLD;
        self.colors[ColorElement::MemoryCompressed as usize] = A_DIM;

        // Graph elements
        self.colors[ColorElement::Graph1 as usize] = A_BOLD;
        self.colors[ColorElement::Graph2 as usize] = A_NORMAL;

        // Huge pages
        self.colors[ColorElement::HugePage1 as usize] = A_BOLD;
        self.colors[ColorElement::HugePage2 as usize] = A_NORMAL;
        self.colors[ColorElement::HugePage3 as usize] = A_REVERSE | A_BOLD;
        self.colors[ColorElement::HugePage4 as usize] = A_REVERSE;

        // Load average
        self.colors[ColorElement::LoadAverageFifteen as usize] = A_DIM;
        self.colors[ColorElement::LoadAverageFive as usize] = A_NORMAL;
        self.colors[ColorElement::LoadAverageOne as usize] = A_BOLD;
        self.colors[ColorElement::Load as usize] = A_BOLD;

        // Help
        self.colors[ColorElement::HelpBold as usize] = A_BOLD;
        self.colors[ColorElement::HelpShadow as usize] = A_DIM;

        // Clock/Date
        self.colors[ColorElement::Clock as usize] = A_BOLD;
        self.colors[ColorElement::Date as usize] = A_BOLD;
        self.colors[ColorElement::DateTime as usize] = A_BOLD;

        // Checkbox
        self.colors[ColorElement::CheckBox as usize] = A_BOLD;
        self.colors[ColorElement::CheckMark as usize] = A_NORMAL;
        self.colors[ColorElement::CheckText as usize] = A_NORMAL;

        // Hostname
        self.colors[ColorElement::Hostname as usize] = A_BOLD;

        // CPU elements
        self.colors[ColorElement::CpuNormal as usize] = A_BOLD;
        self.colors[ColorElement::CpuSystem as usize] = A_BOLD;
        self.colors[ColorElement::CpuIrq as usize] = A_BOLD;
        self.colors[ColorElement::CpuSoftIrq as usize] = A_BOLD;
        self.colors[ColorElement::CpuSteal as usize] = A_DIM;
        self.colors[ColorElement::CpuGuest as usize] = A_DIM;

        // Screen tabs - THIS IS THE KEY FIX
        self.colors[ColorElement::ScreensOthBorder as usize] = A_DIM;
        self.colors[ColorElement::ScreensOthText as usize] = A_DIM;
        self.colors[ColorElement::ScreensCurBorder as usize] = A_REVERSE;
        self.colors[ColorElement::ScreensCurText as usize] = A_REVERSE;

        // Pressure stall
        self.colors[ColorElement::PressureStallThreeHundred as usize] = A_DIM;
        self.colors[ColorElement::PressureStallSixty as usize] = A_NORMAL;
        self.colors[ColorElement::PressureStallTen as usize] = A_BOLD;

        // File descriptors
        self.colors[ColorElement::FileDescriptorUsed as usize] = A_BOLD;
        self.colors[ColorElement::FileDescriptorMax as usize] = A_BOLD;

        // ZFS elements
        self.colors[ColorElement::ZfsAnon as usize] = A_DIM;
        self.colors[ColorElement::ZfsHeader as usize] = A_BOLD;
        self.colors[ColorElement::ZfsOther as usize] = A_DIM;
        self.colors[ColorElement::ZfsCompressed as usize] = A_BOLD;
        self.colors[ColorElement::ZfsRatio as usize] = A_BOLD;

        // Dynamic colors
        self.colors[ColorElement::DynamicGray as usize] = A_DIM;
        self.colors[ColorElement::DynamicDarkGray as usize] = A_DIM;
        self.colors[ColorElement::DynamicRed as usize] = A_BOLD;
        self.colors[ColorElement::DynamicCyan as usize] = A_BOLD;
        self.colors[ColorElement::DynamicWhite as usize] = A_BOLD;
    }

    /// Set up Black on White color scheme
    fn setup_black_on_white(&mut self) {
        // All color pairs are already initialized in set_colors()
        self.colors[ColorElement::ResetColor as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::DefaultColor as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::FunctionBar as usize] = color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::FunctionKey as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::PanelHeaderFocus as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::PanelHeaderUnfocus as usize] =
            color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::PanelSelectionFocus as usize] =
            color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::PanelSelectionFollow as usize] =
            color_pair(COLOR_BLACK, COLOR_YELLOW);
        self.colors[ColorElement::PanelSelectionUnfocus as usize] =
            color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::PanelEdit as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::Disabled as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::FailedSearch as usize] = color_pair(COLOR_RED, COLOR_CYAN);
        self.colors[ColorElement::FailedRead as usize] = color_pair(COLOR_RED, COLOR_WHITE);
        self.colors[ColorElement::Paused as usize] = color_pair(COLOR_YELLOW, COLOR_CYAN) | A_BOLD;
        self.colors[ColorElement::Uptime as usize] = color_pair(COLOR_YELLOW, COLOR_WHITE);
        self.colors[ColorElement::Battery as usize] = color_pair(COLOR_YELLOW, COLOR_WHITE);
        self.colors[ColorElement::LargeNumber as usize] = color_pair(COLOR_RED, COLOR_WHITE);
        self.colors[ColorElement::MeterShadow as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::MeterText as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::MeterValue as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::MeterValueError as usize] =
            color_pair(COLOR_RED, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::MeterValueIORead as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::MeterValueIOWrite as usize] =
            color_pair(COLOR_YELLOW, COLOR_WHITE);
        self.colors[ColorElement::MeterValueNotice as usize] =
            color_pair(COLOR_YELLOW, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::MeterValueOk as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::MeterValueWarn as usize] =
            color_pair(COLOR_YELLOW, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::LedColor as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::TasksRunning as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::Process as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::ProcessShadow as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::ProcessTag as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::ProcessMegabytes as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::ProcessGigabytes as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::ProcessBasename as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::ProcessTree as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::ProcessThread as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::ProcessThreadBasename as usize] =
            color_pair(COLOR_BLUE, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::ProcessComm as usize] = color_pair(COLOR_MAGENTA, COLOR_WHITE);
        self.colors[ColorElement::ProcessThreadComm as usize] =
            color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::ProcessRunState as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::ProcessDState as usize] =
            color_pair(COLOR_RED, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::ProcessHighPriority as usize] =
            color_pair(COLOR_RED, COLOR_WHITE);
        self.colors[ColorElement::ProcessLowPriority as usize] =
            color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::ProcessNew as usize] = color_pair(COLOR_WHITE, COLOR_GREEN);
        self.colors[ColorElement::ProcessTomb as usize] = color_pair(COLOR_WHITE, COLOR_RED);
        self.colors[ColorElement::ProcessPriv as usize] = color_pair(COLOR_MAGENTA, COLOR_WHITE);
        self.colors[ColorElement::BarBorder as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::BarShadow as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::Swap as usize] = color_pair(COLOR_RED, COLOR_WHITE);
        self.colors[ColorElement::SwapCache as usize] = color_pair(COLOR_YELLOW, COLOR_WHITE);
        self.colors[ColorElement::SwapFrontswap as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::Graph1 as usize] = color_pair(COLOR_BLUE, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::Graph2 as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::MemoryUsed as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::MemoryBuffers as usize] = color_pair(COLOR_CYAN, COLOR_WHITE);
        self.colors[ColorElement::MemoryBuffersText as usize] = color_pair(COLOR_CYAN, COLOR_WHITE);
        self.colors[ColorElement::MemoryCache as usize] = color_pair(COLOR_YELLOW, COLOR_WHITE);
        self.colors[ColorElement::MemoryShared as usize] = color_pair(COLOR_MAGENTA, COLOR_WHITE);
        self.colors[ColorElement::MemoryCompressed as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::Load as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::LoadAverageOne as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::LoadAverageFive as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::LoadAverageFifteen as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::HelpBold as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::HelpShadow as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::Clock as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::Date as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::DateTime as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::CheckBox as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::CheckMark as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::CheckText as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::Hostname as usize] = color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::CpuNice as usize] = color_pair(COLOR_CYAN, COLOR_WHITE);
        self.colors[ColorElement::CpuNiceText as usize] = color_pair(COLOR_CYAN, COLOR_WHITE);
        self.colors[ColorElement::CpuNormal as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::CpuSystem as usize] = color_pair(COLOR_RED, COLOR_WHITE);
        self.colors[ColorElement::CpuIOWait as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::CpuIrq as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::CpuSoftIrq as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::CpuSteal as usize] = color_pair(COLOR_CYAN, COLOR_WHITE);
        self.colors[ColorElement::CpuGuest as usize] = color_pair(COLOR_CYAN, COLOR_WHITE);
        self.colors[ColorElement::ScreensOthBorder as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::ScreensOthText as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::ScreensCurBorder as usize] = color_pair(COLOR_GREEN, COLOR_GREEN);
        self.colors[ColorElement::ScreensCurText as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);

        // File descriptor colors
        self.colors[ColorElement::FileDescriptorUsed as usize] =
            color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::FileDescriptorMax as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);

        self.colors[ColorElement::DynamicGray as usize] = color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::DynamicDarkGray as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
        self.colors[ColorElement::DynamicRed as usize] = color_pair(COLOR_RED, COLOR_WHITE);
        self.colors[ColorElement::DynamicGreen as usize] = color_pair(COLOR_GREEN, COLOR_WHITE);
        self.colors[ColorElement::DynamicBlue as usize] = color_pair(COLOR_BLUE, COLOR_WHITE);
        self.colors[ColorElement::DynamicCyan as usize] = color_pair(COLOR_YELLOW, COLOR_WHITE);
        self.colors[ColorElement::DynamicMagenta as usize] = color_pair(COLOR_MAGENTA, COLOR_WHITE);
        self.colors[ColorElement::DynamicYellow as usize] = color_pair(COLOR_YELLOW, COLOR_WHITE);
        self.colors[ColorElement::DynamicWhite as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE) | A_BOLD;
    }

    /// Set up Light Terminal color scheme
    fn setup_light_terminal(&mut self) {
        // All color pairs are already initialized in set_colors()
        // Light Terminal uses default (-1) background which maps to BLACK in the pair system
        self.colors[ColorElement::ResetColor as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::DefaultColor as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::FunctionBar as usize] = color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::FunctionKey as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::PanelHeaderFocus as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::PanelHeaderUnfocus as usize] =
            color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::PanelSelectionFocus as usize] =
            color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::PanelSelectionFollow as usize] =
            color_pair(COLOR_BLACK, COLOR_YELLOW);
        self.colors[ColorElement::PanelSelectionUnfocus as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::PanelEdit as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::Disabled as usize] = color_pair_gray_black();
        self.colors[ColorElement::FailedSearch as usize] = color_pair(COLOR_RED, COLOR_CYAN);
        self.colors[ColorElement::FailedRead as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::Paused as usize] = color_pair(COLOR_YELLOW, COLOR_CYAN) | A_BOLD;
        self.colors[ColorElement::Uptime as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::Battery as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::LargeNumber as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::MeterShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::MeterText as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::MeterValue as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::MeterValueError as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterValueIORead as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MeterValueIOWrite as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::MeterValueNotice as usize] = color_pair_white_default() | A_BOLD;
        self.colors[ColorElement::MeterValueOk as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MeterValueWarn as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::LedColor as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::TasksRunning as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::Process as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::ProcessShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::ProcessTag as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::ProcessMegabytes as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::ProcessGigabytes as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessBasename as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessTree as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::ProcessThread as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::ProcessThreadBasename as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessComm as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::ProcessThreadComm as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::ProcessRunState as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessDState as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessHighPriority as usize] =
            color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::ProcessLowPriority as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessNew as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::ProcessTomb as usize] = color_pair(COLOR_BLACK, COLOR_RED);
        self.colors[ColorElement::ProcessPriv as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::BarBorder as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::BarShadow as usize] = color_pair_gray_black();
        self.colors[ColorElement::Swap as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::SwapCache as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::SwapFrontswap as usize] = color_pair_gray_black();
        self.colors[ColorElement::Graph1 as usize] = color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::Graph2 as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::MemoryUsed as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MemoryBuffers as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::MemoryBuffersText as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::MemoryCache as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::MemoryShared as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::MemoryCompressed as usize] = color_pair_gray_black();
        self.colors[ColorElement::Load as usize] = color_pair_white_default() | A_BOLD;
        self.colors[ColorElement::LoadAverageOne as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::LoadAverageFive as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::LoadAverageFifteen as usize] =
            color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::HelpBold as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::HelpShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::Clock as usize] = A_BOLD;
        self.colors[ColorElement::Date as usize] = A_BOLD;
        self.colors[ColorElement::DateTime as usize] = A_BOLD;
        self.colors[ColorElement::CheckBox as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::CheckMark as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::CheckText as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::Hostname as usize] = A_BOLD;
        self.colors[ColorElement::CpuNice as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::CpuNiceText as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::CpuNormal as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::CpuSystem as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::CpuIOWait as usize] =
            color_pair(COLOR_BLACK, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuIrq as usize] = color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuSoftIrq as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::CpuSteal as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::CpuGuest as usize] = color_pair(COLOR_BLACK, COLOR_BLACK);
        self.colors[ColorElement::ScreensOthBorder as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::ScreensOthText as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::ScreensCurBorder as usize] = color_pair(COLOR_GREEN, COLOR_GREEN);
        self.colors[ColorElement::ScreensCurText as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);

        // File descriptor colors (Light Terminal uses same as Default)
        self.colors[ColorElement::FileDescriptorUsed as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::FileDescriptorMax as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;

        self.colors[ColorElement::DynamicGray as usize] = color_pair_gray_black();
        self.colors[ColorElement::DynamicDarkGray as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::DynamicRed as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::DynamicGreen as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::DynamicBlue as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::DynamicCyan as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::DynamicMagenta as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::DynamicYellow as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::DynamicWhite as usize] = A_BOLD;
    }

    /// Set up Midnight color scheme (blue background like Midnight Commander)
    fn setup_midnight(&mut self) {
        // All color pairs are already initialized in set_colors()
        self.colors[ColorElement::ResetColor as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::DefaultColor as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::FunctionBar as usize] = color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::FunctionKey as usize] = A_NORMAL;
        self.colors[ColorElement::PanelHeaderFocus as usize] = color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::PanelHeaderUnfocus as usize] =
            color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::PanelSelectionFocus as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::PanelSelectionFollow as usize] =
            color_pair(COLOR_BLACK, COLOR_YELLOW);
        self.colors[ColorElement::PanelSelectionUnfocus as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::PanelEdit as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::Disabled as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::FailedSearch as usize] = color_pair(COLOR_RED, COLOR_CYAN);
        self.colors[ColorElement::FailedRead as usize] = color_pair(COLOR_RED, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::Paused as usize] = color_pair(COLOR_BLACK, COLOR_CYAN) | A_BOLD;
        self.colors[ColorElement::Uptime as usize] = color_pair(COLOR_YELLOW, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::Battery as usize] = color_pair(COLOR_YELLOW, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::LargeNumber as usize] =
            color_pair(COLOR_RED, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MeterShadow as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::MeterText as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::MeterValue as usize] =
            color_pair(COLOR_CYAN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MeterValueError as usize] =
            color_pair(COLOR_RED, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MeterValueIORead as usize] = color_pair(COLOR_GREEN, COLOR_BLUE);
        self.colors[ColorElement::MeterValueIOWrite as usize] = color_pair(COLOR_BLACK, COLOR_BLUE);
        self.colors[ColorElement::MeterValueNotice as usize] =
            color_pair(COLOR_WHITE, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MeterValueOk as usize] = color_pair(COLOR_GREEN, COLOR_BLUE);
        self.colors[ColorElement::MeterValueWarn as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::LedColor as usize] = color_pair(COLOR_GREEN, COLOR_BLUE);
        self.colors[ColorElement::TasksRunning as usize] =
            color_pair(COLOR_GREEN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::Process as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::ProcessShadow as usize] =
            color_pair(COLOR_BLACK, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::ProcessTag as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::ProcessMegabytes as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::ProcessGigabytes as usize] = color_pair(COLOR_GREEN, COLOR_BLUE);
        self.colors[ColorElement::ProcessBasename as usize] =
            color_pair(COLOR_CYAN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::ProcessTree as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::ProcessThread as usize] = color_pair(COLOR_GREEN, COLOR_BLUE);
        self.colors[ColorElement::ProcessThreadBasename as usize] =
            color_pair(COLOR_GREEN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::ProcessComm as usize] = color_pair(COLOR_MAGENTA, COLOR_BLUE);
        self.colors[ColorElement::ProcessThreadComm as usize] = color_pair(COLOR_BLACK, COLOR_BLUE);
        self.colors[ColorElement::ProcessRunState as usize] = color_pair(COLOR_GREEN, COLOR_BLUE);
        self.colors[ColorElement::ProcessDState as usize] =
            color_pair(COLOR_RED, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::ProcessHighPriority as usize] = color_pair(COLOR_RED, COLOR_BLUE);
        self.colors[ColorElement::ProcessLowPriority as usize] =
            color_pair(COLOR_GREEN, COLOR_BLUE);
        self.colors[ColorElement::ProcessNew as usize] = color_pair(COLOR_BLUE, COLOR_GREEN);
        self.colors[ColorElement::ProcessTomb as usize] = color_pair(COLOR_BLUE, COLOR_RED);
        self.colors[ColorElement::ProcessPriv as usize] = color_pair(COLOR_MAGENTA, COLOR_BLUE);
        self.colors[ColorElement::BarBorder as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::BarShadow as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::Swap as usize] = color_pair(COLOR_RED, COLOR_BLUE);
        self.colors[ColorElement::SwapCache as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::SwapFrontswap as usize] =
            color_pair(COLOR_BLACK, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::Graph1 as usize] = color_pair(COLOR_CYAN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::Graph2 as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::MemoryUsed as usize] =
            color_pair(COLOR_GREEN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MemoryBuffers as usize] =
            color_pair(COLOR_CYAN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MemoryBuffersText as usize] =
            color_pair(COLOR_CYAN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MemoryCache as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MemoryShared as usize] =
            color_pair(COLOR_MAGENTA, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::MemoryCompressed as usize] =
            color_pair(COLOR_BLACK, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::Load as usize] = color_pair(COLOR_WHITE, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::LoadAverageOne as usize] =
            color_pair(COLOR_WHITE, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::LoadAverageFive as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::LoadAverageFifteen as usize] =
            color_pair(COLOR_BLACK, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::HelpBold as usize] = color_pair(COLOR_CYAN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::HelpShadow as usize] =
            color_pair(COLOR_BLACK, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::Clock as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::Date as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::DateTime as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::CheckBox as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::CheckMark as usize] =
            color_pair(COLOR_WHITE, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::CheckText as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::Hostname as usize] = color_pair(COLOR_WHITE, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::CpuNice as usize] = color_pair(COLOR_CYAN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::CpuNiceText as usize] =
            color_pair(COLOR_CYAN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::CpuNormal as usize] =
            color_pair(COLOR_GREEN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::CpuSystem as usize] = color_pair(COLOR_RED, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::CpuIOWait as usize] =
            color_pair(COLOR_BLACK, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::CpuIrq as usize] = color_pair(COLOR_BLACK, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::CpuSoftIrq as usize] = color_pair(COLOR_BLACK, COLOR_BLUE);
        self.colors[ColorElement::CpuSteal as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::CpuGuest as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
        self.colors[ColorElement::ScreensOthBorder as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::ScreensOthText as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::ScreensCurBorder as usize] = color_pair(COLOR_CYAN, COLOR_CYAN);
        self.colors[ColorElement::ScreensCurText as usize] = color_pair(COLOR_BLACK, COLOR_CYAN);

        // File descriptor colors (Midnight uses blue background)
        self.colors[ColorElement::FileDescriptorUsed as usize] =
            color_pair(COLOR_GREEN, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::FileDescriptorMax as usize] =
            color_pair(COLOR_RED, COLOR_BLUE) | A_BOLD;

        self.colors[ColorElement::DynamicGray as usize] = color_pair(COLOR_BLACK, COLOR_BLUE);
        self.colors[ColorElement::DynamicDarkGray as usize] =
            color_pair(COLOR_BLACK, COLOR_BLUE) | A_BOLD;
        self.colors[ColorElement::DynamicRed as usize] = color_pair(COLOR_RED, COLOR_BLUE);
        self.colors[ColorElement::DynamicGreen as usize] = color_pair(COLOR_GREEN, COLOR_BLUE);
        self.colors[ColorElement::DynamicBlue as usize] = color_pair(COLOR_BLACK, COLOR_BLUE);
        self.colors[ColorElement::DynamicCyan as usize] = color_pair(COLOR_CYAN, COLOR_BLUE);
        self.colors[ColorElement::DynamicMagenta as usize] = color_pair(COLOR_MAGENTA, COLOR_BLUE);
        self.colors[ColorElement::DynamicYellow as usize] = color_pair(COLOR_YELLOW, COLOR_BLUE);
        self.colors[ColorElement::DynamicWhite as usize] = color_pair(COLOR_WHITE, COLOR_BLUE);
    }

    /// Set up Black Night color scheme
    fn setup_black_night(&mut self) {
        // All color pairs are already initialized in set_colors()
        self.colors[ColorElement::ResetColor as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::DefaultColor as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::FunctionBar as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::FunctionKey as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::PanelHeaderFocus as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::PanelHeaderUnfocus as usize] =
            color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::PanelSelectionFocus as usize] =
            color_pair(COLOR_BLACK, COLOR_CYAN);
        self.colors[ColorElement::PanelSelectionFollow as usize] =
            color_pair(COLOR_BLACK, COLOR_YELLOW);
        self.colors[ColorElement::PanelSelectionUnfocus as usize] =
            color_pair(COLOR_BLACK, COLOR_WHITE);
        self.colors[ColorElement::PanelEdit as usize] = color_pair(COLOR_WHITE, COLOR_CYAN);
        self.colors[ColorElement::Disabled as usize] = color_pair_gray_black();
        self.colors[ColorElement::FailedSearch as usize] = color_pair(COLOR_RED, COLOR_GREEN);
        self.colors[ColorElement::FailedRead as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::Paused as usize] = color_pair(COLOR_YELLOW, COLOR_GREEN) | A_BOLD;
        self.colors[ColorElement::Uptime as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::Battery as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::LargeNumber as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::MeterText as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::MeterValue as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MeterValueError as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterValueIORead as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MeterValueIOWrite as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::MeterValueNotice as usize] =
            color_pair(COLOR_WHITE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterValueOk as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MeterValueWarn as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::LedColor as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::TasksRunning as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::Process as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::ProcessShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::ProcessTag as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessMegabytes as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessGigabytes as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessBasename as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessTree as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::ProcessThread as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessThreadBasename as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessComm as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::ProcessThreadComm as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::ProcessRunState as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessDState as usize] =
            color_pair(COLOR_RED, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessHighPriority as usize] =
            color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::ProcessLowPriority as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::ProcessNew as usize] = color_pair(COLOR_BLACK, COLOR_GREEN);
        self.colors[ColorElement::ProcessTomb as usize] = color_pair(COLOR_BLACK, COLOR_RED);
        self.colors[ColorElement::ProcessPriv as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::BarBorder as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::BarShadow as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::Swap as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::SwapCache as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::SwapFrontswap as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::Graph1 as usize] = color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::Graph2 as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MemoryUsed as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::MemoryBuffers as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::MemoryBuffersText as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::MemoryCache as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::MemoryShared as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::MemoryCompressed as usize] =
            color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::Load as usize] = A_BOLD;
        self.colors[ColorElement::LoadAverageOne as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::LoadAverageFive as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::LoadAverageFifteen as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::HelpBold as usize] = color_pair(COLOR_CYAN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::HelpShadow as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::Clock as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::Date as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::DateTime as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::CheckBox as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::CheckMark as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::CheckText as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::Hostname as usize] = color_pair(COLOR_WHITE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuNice as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::CpuNiceText as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuNormal as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::CpuSystem as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::CpuIOWait as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::CpuIrq as usize] = color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuSoftIrq as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::CpuSteal as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::CpuGuest as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::ScreensOthBorder as usize] = color_pair(COLOR_WHITE, COLOR_BLACK);
        self.colors[ColorElement::ScreensOthText as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::ScreensCurBorder as usize] =
            color_pair(COLOR_WHITE, COLOR_BLACK) | A_BOLD;
        self.colors[ColorElement::ScreensCurText as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK) | A_BOLD;

        // File descriptor colors
        self.colors[ColorElement::FileDescriptorUsed as usize] =
            color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::FileDescriptorMax as usize] =
            color_pair(COLOR_BLUE, COLOR_BLACK) | A_BOLD;

        self.colors[ColorElement::DynamicGray as usize] = color_pair_gray_black();
        self.colors[ColorElement::DynamicDarkGray as usize] = color_pair_gray_black() | A_BOLD;
        self.colors[ColorElement::DynamicRed as usize] = color_pair(COLOR_RED, COLOR_BLACK);
        self.colors[ColorElement::DynamicGreen as usize] = color_pair(COLOR_GREEN, COLOR_BLACK);
        self.colors[ColorElement::DynamicBlue as usize] = color_pair(COLOR_BLUE, COLOR_BLACK);
        self.colors[ColorElement::DynamicCyan as usize] = color_pair(COLOR_CYAN, COLOR_BLACK);
        self.colors[ColorElement::DynamicMagenta as usize] = color_pair(COLOR_MAGENTA, COLOR_BLACK);
        self.colors[ColorElement::DynamicYellow as usize] = color_pair(COLOR_YELLOW, COLOR_BLACK);
        self.colors[ColorElement::DynamicWhite as usize] = color_pair(COLOR_WHITE, COLOR_BLACK);
    }

    /// Set up Broken Gray color scheme
    /// This is like Default but fixes gray rendering for terminals that don't support it
    fn setup_broken_gray(&mut self) {
        // First set up default colors
        self.setup_default_colors();

        // Then fix the gray colors - replace gray with white
        // In C htop, this replaces A_BOLD | ColorPairGrayBlack with ColorPair(White, Black)
        const PAIR_WHITE_BLACK: i16 = 20; // Use a new pair number
        let _ = self.screen.init_pair(PAIR_WHITE_BLACK, COLOR_WHITE, -1);
        let white_attr = color_pair_attr(PAIR_WHITE_BLACK);

        // Replace all gray-based colors with white
        self.colors[ColorElement::MeterShadow as usize] = white_attr;
        self.colors[ColorElement::BarShadow as usize] = white_attr;
        self.colors[ColorElement::ProcessShadow as usize] = white_attr;
        self.colors[ColorElement::CpuIOWait as usize] = white_attr;
        self.colors[ColorElement::MemoryCompressed as usize] = white_attr;
        self.colors[ColorElement::HelpShadow as usize] = white_attr;
        self.colors[ColorElement::DynamicGray as usize] = white_attr;
        self.colors[ColorElement::DynamicDarkGray as usize] = white_attr;
    }

    /// Set up Nord color scheme
    /// Minimalist scheme inspired by the Nord color palette
    fn setup_nord(&mut self) {
        // Color pair numbers for Nord scheme
        const PAIR_BLACK_CYAN: i16 = 1;
        const PAIR_YELLOW_BLACK: i16 = 2;
        const PAIR_CYAN_BLACK: i16 = 3;
        const PAIR_WHITE_BLACK: i16 = 4;
        const PAIR_GRAY_BLACK: i16 = 5;

        let _ = self
            .screen
            .init_pair(PAIR_BLACK_CYAN, COLOR_BLACK, COLOR_CYAN);
        let _ = self.screen.init_pair(PAIR_YELLOW_BLACK, COLOR_YELLOW, -1);
        let _ = self.screen.init_pair(PAIR_CYAN_BLACK, COLOR_CYAN, -1);
        let _ = self.screen.init_pair(PAIR_WHITE_BLACK, COLOR_WHITE, -1);

        // Gray/black pair
        let num_colors = self.screen.num_colors();
        let gray_fg = if num_colors > 8 { 8 } else { COLOR_BLACK };
        let _ = self.screen.init_pair(PAIR_GRAY_BLACK, gray_fg, -1);

        self.colors[ColorElement::ResetColor as usize] = A_NORMAL;
        self.colors[ColorElement::DefaultColor as usize] = A_NORMAL;
        self.colors[ColorElement::FunctionBar as usize] = color_pair_attr(PAIR_BLACK_CYAN);
        self.colors[ColorElement::FunctionKey as usize] = A_NORMAL;
        self.colors[ColorElement::PanelHeaderFocus as usize] = color_pair_attr(PAIR_BLACK_CYAN);
        self.colors[ColorElement::PanelHeaderUnfocus as usize] = color_pair_attr(PAIR_BLACK_CYAN);
        self.colors[ColorElement::PanelSelectionFocus as usize] = color_pair_attr(PAIR_BLACK_CYAN);
        self.colors[ColorElement::PanelSelectionFollow as usize] = A_REVERSE;
        self.colors[ColorElement::PanelSelectionUnfocus as usize] = A_BOLD;
        self.colors[ColorElement::PanelEdit as usize] = A_BOLD;
        self.colors[ColorElement::Disabled as usize] = color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::FailedSearch as usize] =
            color_pair_attr(PAIR_YELLOW_BLACK) | A_REVERSE | A_BOLD;
        self.colors[ColorElement::FailedRead as usize] =
            color_pair_attr(PAIR_YELLOW_BLACK) | A_BOLD;
        self.colors[ColorElement::Paused as usize] = color_pair_attr(PAIR_BLACK_CYAN) | A_BOLD;
        self.colors[ColorElement::Uptime as usize] = A_BOLD;
        self.colors[ColorElement::Battery as usize] = A_BOLD;
        self.colors[ColorElement::LargeNumber as usize] =
            color_pair_attr(PAIR_YELLOW_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterShadow as usize] = color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterText as usize] = A_NORMAL;
        self.colors[ColorElement::MeterValue as usize] = A_BOLD;
        self.colors[ColorElement::MeterValueError as usize] = A_BOLD;
        self.colors[ColorElement::MeterValueIORead as usize] = A_NORMAL;
        self.colors[ColorElement::MeterValueIOWrite as usize] = A_NORMAL;
        self.colors[ColorElement::MeterValueNotice as usize] =
            color_pair_attr(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::MeterValueOk as usize] = A_NORMAL;
        self.colors[ColorElement::MeterValueWarn as usize] = A_BOLD;
        self.colors[ColorElement::LedColor as usize] = A_NORMAL;
        self.colors[ColorElement::TasksRunning as usize] = A_BOLD;
        self.colors[ColorElement::Process as usize] = A_NORMAL;
        self.colors[ColorElement::ProcessShadow as usize] =
            color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessTag as usize] = color_pair_attr(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessMegabytes as usize] =
            color_pair_attr(PAIR_WHITE_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessGigabytes as usize] =
            color_pair_attr(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessBasename as usize] = A_BOLD;
        self.colors[ColorElement::ProcessTree as usize] = A_BOLD;
        self.colors[ColorElement::ProcessThread as usize] = A_NORMAL;
        self.colors[ColorElement::ProcessThreadBasename as usize] = A_BOLD;
        self.colors[ColorElement::ProcessComm as usize] = A_BOLD;
        self.colors[ColorElement::ProcessThreadComm as usize] = A_BOLD;
        self.colors[ColorElement::ProcessRunState as usize] = A_BOLD;
        self.colors[ColorElement::ProcessDState as usize] =
            color_pair_attr(PAIR_YELLOW_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessHighPriority as usize] = A_BOLD;
        self.colors[ColorElement::ProcessLowPriority as usize] =
            color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessNew as usize] = A_BOLD;
        self.colors[ColorElement::ProcessTomb as usize] = color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::ProcessPriv as usize] = color_pair_attr(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::BarBorder as usize] = A_BOLD;
        self.colors[ColorElement::BarShadow as usize] = color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::Swap as usize] = A_BOLD;
        self.colors[ColorElement::SwapCache as usize] = A_NORMAL;
        self.colors[ColorElement::SwapFrontswap as usize] =
            color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::Graph1 as usize] = A_BOLD;
        self.colors[ColorElement::Graph2 as usize] = A_NORMAL;
        self.colors[ColorElement::MemoryUsed as usize] =
            color_pair_attr(PAIR_YELLOW_BLACK) | A_BOLD;
        self.colors[ColorElement::MemoryBuffers as usize] = A_NORMAL;
        self.colors[ColorElement::MemoryBuffersText as usize] = A_NORMAL;
        self.colors[ColorElement::MemoryCache as usize] = A_NORMAL;
        self.colors[ColorElement::MemoryShared as usize] = A_NORMAL;
        self.colors[ColorElement::MemoryCompressed as usize] =
            color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::Load as usize] = A_BOLD;
        self.colors[ColorElement::LoadAverageOne as usize] = A_BOLD;
        self.colors[ColorElement::LoadAverageFive as usize] = A_NORMAL;
        self.colors[ColorElement::LoadAverageFifteen as usize] =
            color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::HelpBold as usize] = A_BOLD;
        self.colors[ColorElement::HelpShadow as usize] = color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::Clock as usize] = A_BOLD;
        self.colors[ColorElement::Date as usize] = A_BOLD;
        self.colors[ColorElement::DateTime as usize] = A_BOLD;
        self.colors[ColorElement::CheckBox as usize] = A_BOLD;
        self.colors[ColorElement::CheckMark as usize] = A_NORMAL;
        self.colors[ColorElement::CheckText as usize] = A_NORMAL;
        self.colors[ColorElement::Hostname as usize] = color_pair_attr(PAIR_WHITE_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuNice as usize] = A_NORMAL;
        self.colors[ColorElement::CpuNiceText as usize] = A_NORMAL;
        self.colors[ColorElement::CpuNormal as usize] = A_BOLD;
        self.colors[ColorElement::CpuSystem as usize] = color_pair_attr(PAIR_YELLOW_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuIOWait as usize] = A_NORMAL;
        self.colors[ColorElement::CpuIrq as usize] = A_BOLD;
        self.colors[ColorElement::CpuSoftIrq as usize] = A_BOLD;
        self.colors[ColorElement::CpuSteal as usize] = color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::CpuGuest as usize] = color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::ScreensOthBorder as usize] =
            color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::ScreensOthText as usize] =
            color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::ScreensCurBorder as usize] = color_pair_attr(PAIR_BLACK_CYAN);
        self.colors[ColorElement::ScreensCurText as usize] = color_pair_attr(PAIR_BLACK_CYAN);

        // File descriptor colors
        self.colors[ColorElement::FileDescriptorUsed as usize] = A_BOLD;
        self.colors[ColorElement::FileDescriptorMax as usize] =
            color_pair_attr(PAIR_YELLOW_BLACK) | A_BOLD;

        self.colors[ColorElement::DynamicGray as usize] = color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::DynamicDarkGray as usize] =
            color_pair_attr(PAIR_GRAY_BLACK) | A_BOLD;
        self.colors[ColorElement::DynamicRed as usize] =
            color_pair_attr(PAIR_YELLOW_BLACK) | A_BOLD;
        self.colors[ColorElement::DynamicGreen as usize] = A_BOLD;
        self.colors[ColorElement::DynamicBlue as usize] = color_pair_attr(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::DynamicCyan as usize] = color_pair_attr(PAIR_CYAN_BLACK) | A_BOLD;
        self.colors[ColorElement::DynamicMagenta as usize] = A_BOLD;
        self.colors[ColorElement::DynamicYellow as usize] =
            color_pair_attr(PAIR_YELLOW_BLACK) | A_BOLD;
        self.colors[ColorElement::DynamicWhite as usize] = A_BOLD;
    }

    /// Get color attribute for an element
    pub fn color(&self, element: ColorElement) -> attr_t {
        self.colors
            .get(element as usize)
            .copied()
            .unwrap_or(A_NORMAL)
    }

    /// Get the bar character for a given value index
    ///
    /// This is the theme-agnostic way to get bar characters. In Monochrome mode,
    /// each segment gets a different character. In colored modes, all segments use '|'.
    #[inline]
    pub fn bar_char(&self, value_index: usize) -> char {
        self.theme.bar_char(value_index)
    }

    /// Get optional theme-specific help text
    ///
    /// Returns extra text to display in the help screen, if any.
    #[inline]
    pub fn theme_help_text(&self) -> Option<&'static str> {
        self.theme.help_text()
    }

    /// Update screen dimensions
    pub fn update_size(&mut self) {
        self.screen_height = self.screen.lines();
        self.screen_width = self.screen.cols();
    }

    /// Get screen width
    pub fn width(&self) -> i32 {
        self.screen_width
    }

    /// Get screen height
    pub fn height(&self) -> i32 {
        self.screen_height
    }

    /// Set input delay (matches C htop CRT_enableDelay using halfdelay)
    pub fn set_delay(&mut self, delay: u32) {
        self.delay = delay;
        // Use halfdelay like C htop - delay is in tenths of seconds (1-255)
        // halfdelay makes getch() wait up to delay tenths of a second
        let delay_tenths = (delay as i32).clamp(1, 255);
        let _ = self.screen.halfdelay(delay_tenths);
    }

    /// Disable input delay (for instant response)
    /// Matches C htop CRT_disableDelay
    pub fn disable_delay(&mut self) {
        let _ = self.screen.nocbreak();
        let _ = self.screen.cbreak();
        self.screen.nodelay(true);
    }

    /// Enable input delay
    /// Matches C htop CRT_enableDelay
    pub fn enable_delay(&mut self) {
        let delay_tenths = (self.delay as i32).clamp(1, 255);
        let _ = self.screen.halfdelay(delay_tenths);
    }

    /// Read a key from input
    /// Matches C htop Panel_getCh behavior with set_escdelay(25) for faster ESC handling
    pub fn read_key(&mut self) -> Option<i32> {
        // Set escape delay to 25ms for faster ESC key response (matches C htop)
        // This reduces the delay when pressing ESC (which is also prefix for arrow keys)
        self.screen.set_escdelay(25);
        let ch = self.screen.getch().unwrap_or(ERR);
        if ch == ERR {
            None
        } else {
            Some(ch)
        }
    }

    /// Read a key in non-blocking mode with fast escape handling
    /// Used for screens that poll for input (like strace screen)
    pub fn read_key_nonblocking(&mut self) -> Option<i32> {
        self.screen.set_escdelay(25);
        self.screen.nodelay(true);
        let ch = self.screen.getch().unwrap_or(ERR);
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

        if let Some(mevent) = self.screen.getmouse() {
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
    /// - panel_y: y position of the panel header row
    /// - tab_info: Optional (tab_row_y, tab_boundaries) for screen tab click detection
    ///   tab_boundaries is a slice of (start_x, end_x) for each tab
    /// - func_bar_click: Optional closure to get function key from x position
    pub fn process_mouse_event(
        &mut self,
        screen_height: i32,
        panel_y: Option<i32>,
        tab_info: Option<(i32, &[(i32, i32)])>,
        func_bar_click: Option<&dyn Fn(i32) -> Option<i32>>,
    ) -> Option<i32> {
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
                    if let Some(get_key) = func_bar_click {
                        if let Some(key) = get_key(event.x) {
                            return Some(key);
                        }
                    }
                }
                // Check if click is on screen tabs row
                if let Some((tab_y, tab_boundaries)) = tab_info {
                    if event.y == tab_y {
                        // Find which tab was clicked
                        for (idx, &(start_x, end_x)) in tab_boundaries.iter().enumerate() {
                            if event.x >= start_x && event.x <= end_x {
                                return Some(KEY_TAB_CLICK + idx as i32);
                            }
                        }
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
    pub fn convert_mouse_to_key(&mut self) -> Option<i32> {
        if let Some(mevent) = self.screen.getmouse() {
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
        self.scroll_wheel_v_amount
    }

    /// Clear the screen
    pub fn clear(&mut self) {
        let _ = self.screen.stdscr_mut().clear();
    }

    /// Refresh the screen
    pub fn refresh(&mut self) {
        let _ = self.screen.refresh();
    }

    /// Move cursor
    pub fn move_cursor(&mut self, y: i32, x: i32) {
        let _ = self.screen.stdscr_mut().mv(y, x);
    }

    /// Print a string with attributes
    pub fn print_at(&mut self, y: i32, x: i32, attr: attr_t, text: &str) {
        let win = self.screen.stdscr_mut();
        let _ = win.attrset(attr);
        let _ = win.mvaddstr(y, x, text);
        let _ = win.attrset(A_NORMAL);
    }

    /// Print a string with a specific color element
    pub fn print_colored(&mut self, y: i32, x: i32, element: ColorElement, text: &str) {
        let attr = self.color(element);
        self.print_at(y, x, attr, text);
    }

    // =========================================================================
    // High-level drawing methods (avoid need for with_window in most cases)
    // =========================================================================

    /// Fill a line with spaces using the given attribute
    pub fn fill_line(&mut self, y: i32, attr: attr_t) {
        let width = self.screen_width;
        let win = self.screen.stdscr_mut();
        let _ = win.mv(y, 0);
        let _ = win.attrset(attr);
        for _ in 0..width {
            let _ = win.addch(' ' as u32);
        }
        let _ = win.attrset(A_NORMAL);
    }

    /// Fill part of a line with a character using the given attribute
    pub fn fill_chars(&mut self, y: i32, x: i32, attr: attr_t, ch: char, count: i32) {
        let win = self.screen.stdscr_mut();
        let _ = win.mv(y, x);
        let _ = win.attrset(attr);
        for _ in 0..count {
            let _ = win.addch(ch as u32);
        }
        let _ = win.attrset(A_NORMAL);
    }

    /// Print a string at current cursor position with attribute
    pub fn addstr(&mut self, attr: attr_t, text: &str) {
        let win = self.screen.stdscr_mut();
        let _ = win.attrset(attr);
        let _ = win.addstr(text);
        let _ = win.attrset(A_NORMAL);
    }

    /// Print a character at position with attribute
    pub fn mvaddch(&mut self, y: i32, x: i32, attr: attr_t, ch: u32) {
        let win = self.screen.stdscr_mut();
        let _ = win.attrset(attr);
        let _ = win.mvaddch(y, x, ch);
        let _ = win.attrset(A_NORMAL);
    }

    /// Print a character at current position with attribute
    pub fn addch(&mut self, attr: attr_t, ch: u32) {
        let win = self.screen.stdscr_mut();
        let _ = win.attrset(attr);
        let _ = win.addch(ch);
        let _ = win.attrset(A_NORMAL);
    }

    /// Clear from current position to end of line
    pub fn clrtoeol(&mut self) {
        let _ = self.screen.stdscr_mut().clrtoeol();
    }

    /// Move cursor and clear to end of line
    pub fn mv_clrtoeol(&mut self, y: i32, x: i32) {
        let win = self.screen.stdscr_mut();
        let _ = win.mv(y, x);
        let _ = win.clrtoeol();
    }

    /// Draw a horizontal line with attribute
    pub fn hline_attr(&mut self, y: i32, x: i32, attr: attr_t, ch: u32, n: i32) {
        let win = self.screen.stdscr_mut();
        let _ = win.mv(y, x);
        let _ = win.attrset(attr);
        let _ = win.hline(ch, n);
        let _ = win.attrset(A_NORMAL);
    }

    /// Get current cursor position
    pub fn getyx(&self) -> (i32, i32) {
        let win = self.screen.stdscr();
        (win.getcury(), win.getcurx())
    }

    /// Get mutable reference to the standard window
    /// Use this for complex drawing operations that need direct window access
    #[inline]
    pub fn stdscr_mut(&mut self) -> &mut Window {
        self.screen.stdscr_mut()
    }

    /// Set attribute for subsequent drawing operations
    /// Call attrset(A_NORMAL) when done
    pub fn attrset(&mut self, attr: attr_t) {
        let _ = self.screen.stdscr_mut().attrset(attr);
    }

    /// Move cursor to position
    pub fn mv(&mut self, y: i32, x: i32) {
        let _ = self.screen.stdscr_mut().mv(y, x);
    }

    /// Print a string at current cursor position (no attribute handling)
    /// Use attrset() before and after for attribute control
    pub fn addstr_raw(&mut self, text: &str) {
        let _ = self.screen.stdscr_mut().addstr(text);
    }

    /// Print a character at current cursor position (no attribute handling)
    /// Use attrset() before and after for attribute control
    pub fn addch_raw(&mut self, ch: u32) {
        let _ = self.screen.stdscr_mut().addch(ch);
    }

    /// Print a character at position (no attribute reset)
    pub fn mvaddch_raw(&mut self, y: i32, x: i32, ch: u32) {
        let _ = self.screen.stdscr_mut().mvaddch(y, x, ch);
    }

    /// Print up to n characters of a string (no attribute handling)
    pub fn addnstr_raw(&mut self, text: &str, n: i32) {
        let _ = self.screen.stdscr_mut().addnstr(text, n);
    }

    /// Move and print string (no attribute reset)
    pub fn mvaddstr_raw(&mut self, y: i32, x: i32, text: &str) {
        let _ = self.screen.stdscr_mut().mvaddstr(y, x, text);
    }

    /// Draw a horizontal line
    pub fn hline(&mut self, y: i32, x: i32, ch: u32, n: i32) {
        let win = self.screen.stdscr_mut();
        let _ = win.mv(y, x);
        let _ = win.hline(ch, n);
    }

    /// Draw a box
    pub fn draw_box(&mut self, y: i32, x: i32, h: i32, w: i32) {
        let win = self.screen.stdscr_mut();
        // Top border
        let _ = win.mv(y, x);
        let _ = win.hline(ACS_HLINE as u32, w);
        let _ = win.mvaddch(y, x, ACS_ULCORNER as u32);
        let _ = win.mvaddch(y, x + w - 1, ACS_URCORNER as u32);

        // Side borders
        for i in 1..h - 1 {
            let _ = win.mvaddch(y + i, x, ACS_VLINE as u32);
            let _ = win.mvaddch(y + i, x + w - 1, ACS_VLINE as u32);
        }

        // Bottom border
        let _ = win.mv(y + h - 1, x);
        let _ = win.hline(ACS_HLINE as u32, w);
        let _ = win.mvaddch(y + h - 1, x, ACS_LLCORNER as u32);
        let _ = win.mvaddch(y + h - 1, x + w - 1, ACS_LRCORNER as u32);
    }

    /// Clean up terminal
    pub fn done(&mut self) {
        let _ = self.screen.curs_set(CURSOR_VISIBLE);
        let _ = self.screen.endwin();
    }

    /// Set mouse enabled state
    pub fn set_mouse(&mut self, enabled: bool) {
        self.mouse_enabled = enabled;
        if enabled {
            let mask = BUTTON1_RELEASED
                | BUTTON1_DOUBLE_CLICKED
                | BUTTON2_RELEASED
                | BUTTON3_RELEASED
                | BUTTON4_PRESSED
                | BUTTON5_PRESSED;
            self.screen.mousemask(mask);
        } else {
            self.screen.mousemask(0);
        }
    }

    /// Check if colors are available
    pub fn has_colors(&self) -> bool {
        // We cached color state during init, use screen's has_colors
        // Note: this needs &mut self due to borrow checker, but we use the cached value from init
        // For simplicity, we return true if we have colors configured
        !self.colors.is_empty() && self.colors.iter().any(|&c| c != A_NORMAL)
    }

    /// Handle terminal resize
    pub fn handle_resize(&mut self) {
        let _ = self.screen.update_term_size();
        self.update_size();
    }

    /// Set color scheme and update display
    pub fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.set_colors(scheme);
    }

    /// Sound the terminal bell
    pub fn beep(&mut self) {
        let _ = self.screen.beep();
    }

    /// Set cursor visibility
    pub fn curs_set(&mut self, visibility: i32) {
        let _ = self.screen.curs_set(visibility);
    }

    /// Get a character (raw, returns ERR if no key available in non-blocking mode)
    pub fn getch(&mut self) -> i32 {
        self.screen.getch().unwrap_or(ERR)
    }

    /// Set blocking/non-blocking input mode
    /// When blocking is true, getch() will wait indefinitely for input
    /// When blocking is false, getch() returns ERR immediately if no input
    pub fn set_blocking(&mut self, blocking: bool) {
        if blocking {
            // Disable halfdelay mode by resetting to cbreak mode
            // halfdelay overrides nodelay, so we need to clear it first
            let _ = self.screen.nocbreak();
            let _ = self.screen.cbreak();
        }
        self.screen.nodelay(!blocking);
    }

    /// Check if input is available using select() with timeout
    /// This is more reliable than nodelay mode for some use cases
    /// Returns true if input is available, false if timeout expired
    #[cfg(unix)]
    pub fn has_input_with_timeout(&self, timeout_ms: u32) -> bool {
        use std::mem::MaybeUninit;

        unsafe {
            let mut fds = {
                let f = MaybeUninit::<libc::fd_set>::zeroed();
                f.assume_init()
            };
            libc::FD_ZERO(&mut fds);
            libc::FD_SET(libc::STDIN_FILENO, &mut fds);

            let mut timeout = libc::timeval {
                tv_sec: (timeout_ms / 1000) as _,
                tv_usec: ((timeout_ms % 1000) * 1000) as _,
            };

            let result = libc::select(
                libc::STDIN_FILENO + 1,
                &mut fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut timeout,
            );

            result > 0
        }
    }
}

impl Drop for Crt {
    fn drop(&mut self) {
        self.done();
    }
}
