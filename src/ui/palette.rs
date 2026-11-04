//! Semantic colour palettes.
//!
//! Each colour scheme is described once, as a small set of *roles* (label,
//! value, shadow, status colours, bar segment series, panel chrome...). Every
//! [`ColorElement`] the UI draws with is derived from those roles by
//! [`derive`], so a role can only ever have one look within a scheme: all
//! de-emphasised text uses `shadow`, all primary values use `value`, every
//! error indication uses `error`, and so on. Adding a scheme means filling in
//! one [`Palette`]; it cannot leave an element unset or assign the same kind
//! of information two different colours.
//!
//! Bars follow one rule everywhere: segment colours apply to the fill
//! characters only. Text written over a bar uses `bar_text` where the fill
//! reaches it and `shadow` beyond the fill, so a value reads the same whatever
//! segment happens to sit underneath it.

use super::crt::{
    A_BOLD, A_DIM, A_NORMAL, A_REVERSE, COLOR_BLACK, COLOR_BLUE, COLOR_CYAN, COLOR_GREEN,
    COLOR_MAGENTA, COLOR_RED, COLOR_WHITE, COLOR_YELLOW, ColorElement, attr_t, color_pair,
    color_pair_gray_black, color_pair_white_default,
};
use crate::core::ColorScheme;

/// Colours for the CPU meter's stacked segments
#[derive(Debug, Clone, Copy)]
pub struct CpuColors {
    pub normal: attr_t,
    pub system: attr_t,
    pub nice: attr_t,
    pub irq: attr_t,
    pub softirq: attr_t,
    pub steal: attr_t,
    pub guest: attr_t,
    pub iowait: attr_t,
}

/// Colours for the memory meter's stacked segments
#[derive(Debug, Clone, Copy)]
pub struct MemoryColors {
    pub used: attr_t,
    pub shared: attr_t,
    pub compressed: attr_t,
    pub buffers: attr_t,
    pub cache: attr_t,
}

/// Colours for the swap meter's stacked segments
#[derive(Debug, Clone, Copy)]
pub struct SwapColors {
    pub used: attr_t,
    pub cache: attr_t,
}

/// Colours for the file descriptor meter
#[derive(Debug, Clone, Copy)]
pub struct FdColors {
    pub used: attr_t,
    pub max: attr_t,
}

/// The roles that make up one colour scheme
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    // Text
    /// Body text: process rows, plain panel text, the reset attribute
    pub text: attr_t,
    /// Captions: meter names, "Tasks:", "Load average:", tree lines
    pub label: attr_t,
    /// Primary values: counts, totals, uptime, emphasised help text
    pub value: attr_t,
    /// The value to look at first: 1-minute load, notices
    pub value_strong: attr_t,
    /// Trailing context: 15-minute load
    pub value_weak: attr_t,
    /// De-emphasised text: idle and zero values, hidden counts, help notes, disabled items
    pub shadow: attr_t,
    /// Headings: hostname, clock, check marks, the "Load" word
    pub emphasis: attr_t,

    // Status
    pub ok: attr_t,
    pub warn: attr_t,
    /// Errors, D-state processes, failed reads and very large numbers
    pub error: attr_t,

    // Magnitude
    pub mega: attr_t,
    pub giga: attr_t,

    // Panel chrome
    pub header: attr_t,
    pub selection: attr_t,
    pub selection_unfocused: attr_t,
    pub selection_follow: attr_t,
    pub function_bar: attr_t,
    pub function_key: attr_t,
    pub edit: attr_t,
    pub tab_current: attr_t,
    pub tab_current_border: attr_t,
    pub tab_other: attr_t,
    pub tab_other_border: attr_t,
    pub failed_search: attr_t,
    pub paused: attr_t,

    // Process rows
    pub tag: attr_t,
    pub new_process: attr_t,
    pub tomb: attr_t,
    pub basename: attr_t,
    pub thread: attr_t,
    pub thread_basename: attr_t,
    pub comm: attr_t,
    pub thread_comm: attr_t,
    pub high_priority: attr_t,
    pub low_priority: attr_t,

    // Bars and graphs
    pub bar_border: attr_t,
    /// Text over the filled part of a bar
    pub bar_text: attr_t,
    pub graph_strong: attr_t,
    pub graph_weak: attr_t,

    // Series
    pub cpu: CpuColors,
    pub memory: MemoryColors,
    pub swap: SwapColors,
    pub fd: FdColors,
    pub io_read: attr_t,
    pub io_write: attr_t,
    pub led: attr_t,

    // Setup screen
    pub checkbox: attr_t,
    pub check_mark: attr_t,
}

/// A foreground colour on the scheme's background
fn fg(color: i16) -> attr_t {
    color_pair(color, COLOR_BLACK)
}

/// An explicit foreground/background pair
fn on(fg: i16, bg: i16) -> attr_t {
    color_pair(fg, bg)
}

/// Dark gray on the scheme's background (bright black where available)
fn gray() -> attr_t {
    color_pair_gray_black()
}

impl Palette {
    /// The palette for a colour scheme
    pub fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Default => Self::default_dark(),
            ColorScheme::Monochrome => Self::monochrome(),
            ColorScheme::BlackOnWhite => Self::black_on_white(),
            ColorScheme::LightTerminal => Self::light_terminal(),
            ColorScheme::Midnight => Self::midnight(),
            ColorScheme::BlackNight => Self::black_night(),
            ColorScheme::BrokenGray => Self::broken_gray(),
            ColorScheme::Nord => Self::nord(),
        }
    }

    /// The default scheme: coloured text on the terminal's own background
    fn default_dark() -> Self {
        Self {
            text: A_NORMAL,
            label: fg(COLOR_CYAN),
            value: fg(COLOR_CYAN) | A_BOLD,
            value_strong: A_BOLD,
            value_weak: fg(COLOR_CYAN) | A_DIM,
            shadow: gray() | A_BOLD,
            emphasis: A_BOLD,

            ok: fg(COLOR_GREEN),
            warn: fg(COLOR_YELLOW) | A_BOLD,
            error: fg(COLOR_RED) | A_BOLD,

            mega: fg(COLOR_CYAN),
            giga: fg(COLOR_GREEN),

            header: on(COLOR_BLACK, COLOR_GREEN),
            selection: on(COLOR_BLACK, COLOR_CYAN),
            selection_unfocused: on(COLOR_BLACK, COLOR_WHITE),
            selection_follow: on(COLOR_BLACK, COLOR_YELLOW),
            function_bar: on(COLOR_BLACK, COLOR_CYAN),
            function_key: A_NORMAL,
            edit: on(COLOR_WHITE, COLOR_BLUE),
            tab_current: on(COLOR_BLACK, COLOR_GREEN),
            tab_current_border: on(COLOR_GREEN, COLOR_GREEN),
            tab_other: on(COLOR_BLACK, COLOR_BLUE),
            tab_other_border: on(COLOR_BLUE, COLOR_BLUE),
            failed_search: on(COLOR_RED, COLOR_CYAN),
            paused: on(COLOR_YELLOW, COLOR_CYAN) | A_BOLD,

            tag: fg(COLOR_YELLOW) | A_BOLD,
            new_process: on(COLOR_BLACK, COLOR_GREEN),
            tomb: on(COLOR_BLACK, COLOR_RED),
            basename: fg(COLOR_CYAN) | A_BOLD,
            thread: fg(COLOR_GREEN),
            thread_basename: fg(COLOR_GREEN) | A_BOLD,
            comm: fg(COLOR_MAGENTA),
            thread_comm: fg(COLOR_YELLOW),
            high_priority: fg(COLOR_RED),
            low_priority: fg(COLOR_GREEN),

            bar_border: A_BOLD,
            bar_text: A_BOLD,
            graph_strong: fg(COLOR_CYAN) | A_BOLD,
            graph_weak: fg(COLOR_CYAN),

            cpu: CpuColors {
                normal: fg(COLOR_GREEN),
                system: fg(COLOR_RED),
                nice: fg(COLOR_BLUE) | A_BOLD,
                irq: fg(COLOR_YELLOW),
                softirq: fg(COLOR_MAGENTA),
                steal: fg(COLOR_CYAN),
                guest: fg(COLOR_CYAN) | A_BOLD,
                iowait: gray() | A_BOLD,
            },
            memory: MemoryColors {
                used: fg(COLOR_GREEN),
                shared: fg(COLOR_MAGENTA),
                compressed: gray() | A_BOLD,
                buffers: fg(COLOR_BLUE),
                cache: fg(COLOR_YELLOW),
            },
            swap: SwapColors {
                used: fg(COLOR_RED),
                cache: fg(COLOR_YELLOW),
            },
            fd: FdColors {
                used: fg(COLOR_GREEN),
                max: fg(COLOR_BLUE) | A_BOLD,
            },
            io_read: fg(COLOR_GREEN),
            io_write: fg(COLOR_BLUE) | A_BOLD,
            led: fg(COLOR_GREEN),

            checkbox: fg(COLOR_CYAN),
            check_mark: A_BOLD,
        }
    }

    /// No colours: bold, dim and reverse video only. Bar segments are told
    /// apart by their characters rather than their colours.
    fn monochrome() -> Self {
        Self {
            text: A_NORMAL,
            label: A_NORMAL,
            value: A_BOLD,
            value_strong: A_BOLD,
            value_weak: A_DIM,
            shadow: A_DIM,
            emphasis: A_BOLD,

            ok: A_BOLD,
            warn: A_BOLD,
            error: A_BOLD,

            mega: A_BOLD,
            giga: A_BOLD,

            header: A_REVERSE,
            selection: A_REVERSE,
            selection_unfocused: A_BOLD,
            selection_follow: A_REVERSE,
            function_bar: A_REVERSE,
            function_key: A_NORMAL,
            edit: A_BOLD,
            tab_current: A_REVERSE,
            tab_current_border: A_REVERSE,
            tab_other: A_DIM,
            tab_other_border: A_DIM,
            failed_search: A_REVERSE | A_BOLD,
            paused: A_BOLD | A_REVERSE,

            tag: A_BOLD,
            new_process: A_BOLD,
            tomb: A_DIM,
            basename: A_BOLD,
            thread: A_BOLD,
            thread_basename: A_REVERSE,
            comm: A_BOLD,
            thread_comm: A_REVERSE,
            high_priority: A_BOLD,
            low_priority: A_DIM,

            bar_border: A_BOLD,
            bar_text: A_BOLD,
            graph_strong: A_BOLD,
            graph_weak: A_NORMAL,

            cpu: CpuColors {
                normal: A_BOLD,
                system: A_BOLD,
                nice: A_NORMAL,
                irq: A_BOLD,
                softirq: A_BOLD,
                steal: A_DIM,
                guest: A_DIM,
                iowait: A_NORMAL,
            },
            memory: MemoryColors {
                used: A_BOLD,
                shared: A_NORMAL,
                compressed: A_DIM,
                buffers: A_NORMAL,
                cache: A_NORMAL,
            },
            swap: SwapColors {
                used: A_BOLD,
                cache: A_NORMAL,
            },
            fd: FdColors {
                used: A_BOLD,
                max: A_BOLD,
            },
            io_read: A_NORMAL,
            io_write: A_NORMAL,
            led: A_NORMAL,

            checkbox: A_BOLD,
            check_mark: A_NORMAL,
        }
    }

    /// Dark text on an explicit white background
    fn black_on_white() -> Self {
        let w = COLOR_WHITE;
        Self {
            text: on(COLOR_BLACK, w),
            label: on(COLOR_BLUE, w),
            value: on(COLOR_BLACK, w),
            value_strong: on(COLOR_BLACK, w) | A_BOLD,
            value_weak: on(COLOR_BLACK, w),
            shadow: on(COLOR_BLACK, w) | A_DIM,
            emphasis: on(COLOR_BLACK, w) | A_BOLD,

            ok: on(COLOR_GREEN, w),
            warn: on(COLOR_YELLOW, w),
            error: on(COLOR_RED, w),

            mega: on(COLOR_BLUE, w),
            giga: on(COLOR_GREEN, w),

            header: on(COLOR_BLACK, COLOR_GREEN),
            selection: on(COLOR_BLACK, COLOR_CYAN),
            selection_unfocused: on(COLOR_BLUE, w),
            selection_follow: on(COLOR_BLACK, COLOR_YELLOW),
            function_bar: on(COLOR_BLACK, COLOR_CYAN),
            function_key: on(COLOR_BLACK, w),
            edit: on(COLOR_WHITE, COLOR_BLUE),
            tab_current: on(COLOR_BLACK, COLOR_GREEN),
            tab_current_border: on(COLOR_GREEN, COLOR_GREEN),
            tab_other: on(COLOR_BLACK, w),
            tab_other_border: on(COLOR_BLACK, w),
            failed_search: on(COLOR_RED, COLOR_CYAN),
            paused: on(COLOR_YELLOW, COLOR_CYAN),

            tag: on(COLOR_WHITE, COLOR_BLUE),
            new_process: on(COLOR_WHITE, COLOR_GREEN),
            tomb: on(COLOR_WHITE, COLOR_RED),
            basename: on(COLOR_BLUE, w),
            thread: on(COLOR_BLUE, w),
            thread_basename: on(COLOR_BLUE, w),
            comm: on(COLOR_MAGENTA, w),
            thread_comm: on(COLOR_GREEN, w),
            high_priority: on(COLOR_RED, w),
            low_priority: on(COLOR_GREEN, w),

            bar_border: on(COLOR_BLUE, w),
            bar_text: on(COLOR_BLACK, w) | A_BOLD,
            graph_strong: on(COLOR_BLUE, w),
            graph_weak: on(COLOR_BLUE, w),

            cpu: CpuColors {
                normal: on(COLOR_GREEN, w),
                system: on(COLOR_RED, w),
                nice: on(COLOR_CYAN, w),
                irq: on(COLOR_BLUE, w),
                softirq: on(COLOR_BLUE, w),
                steal: on(COLOR_CYAN, w),
                guest: on(COLOR_CYAN, w),
                iowait: on(COLOR_BLACK, w),
            },
            memory: MemoryColors {
                used: on(COLOR_GREEN, w),
                shared: on(COLOR_MAGENTA, w),
                compressed: on(COLOR_BLACK, w),
                buffers: on(COLOR_CYAN, w),
                cache: on(COLOR_YELLOW, w),
            },
            swap: SwapColors {
                used: on(COLOR_RED, w),
                cache: on(COLOR_YELLOW, w),
            },
            fd: FdColors {
                used: on(COLOR_GREEN, w),
                max: on(COLOR_BLUE, w),
            },
            io_read: on(COLOR_GREEN, w),
            io_write: on(COLOR_YELLOW, w),
            led: on(COLOR_GREEN, w),

            checkbox: on(COLOR_BLUE, w),
            check_mark: on(COLOR_BLACK, w),
        }
    }

    /// Dark text on the terminal's own light background
    fn light_terminal() -> Self {
        Self {
            text: fg(COLOR_BLACK),
            label: fg(COLOR_BLUE),
            value: fg(COLOR_BLACK),
            value_strong: fg(COLOR_BLACK) | A_BOLD,
            value_weak: fg(COLOR_BLACK),
            shadow: gray() | A_BOLD,
            emphasis: A_BOLD,

            ok: fg(COLOR_GREEN),
            warn: fg(COLOR_YELLOW) | A_BOLD,
            error: fg(COLOR_RED) | A_BOLD,

            mega: fg(COLOR_BLUE),
            giga: fg(COLOR_GREEN),

            header: on(COLOR_BLACK, COLOR_GREEN),
            selection: on(COLOR_BLACK, COLOR_CYAN),
            selection_unfocused: fg(COLOR_BLUE),
            selection_follow: on(COLOR_BLACK, COLOR_YELLOW),
            function_bar: on(COLOR_BLACK, COLOR_CYAN),
            function_key: fg(COLOR_BLACK),
            edit: on(COLOR_WHITE, COLOR_BLUE),
            tab_current: on(COLOR_BLACK, COLOR_GREEN),
            tab_current_border: on(COLOR_GREEN, COLOR_GREEN),
            tab_other: fg(COLOR_BLUE),
            tab_other_border: fg(COLOR_BLUE),
            failed_search: on(COLOR_RED, COLOR_CYAN),
            paused: on(COLOR_YELLOW, COLOR_CYAN),

            tag: on(COLOR_WHITE, COLOR_BLUE),
            new_process: on(COLOR_BLACK, COLOR_GREEN),
            tomb: on(COLOR_BLACK, COLOR_RED),
            basename: fg(COLOR_GREEN),
            thread: fg(COLOR_BLUE),
            thread_basename: fg(COLOR_BLUE) | A_BOLD,
            comm: fg(COLOR_MAGENTA),
            thread_comm: fg(COLOR_YELLOW),
            high_priority: fg(COLOR_RED),
            low_priority: fg(COLOR_GREEN),

            bar_border: fg(COLOR_BLUE),
            bar_text: fg(COLOR_BLACK) | A_BOLD,
            graph_strong: fg(COLOR_CYAN) | A_BOLD,
            graph_weak: fg(COLOR_CYAN),

            cpu: CpuColors {
                normal: fg(COLOR_GREEN),
                system: fg(COLOR_RED),
                nice: fg(COLOR_CYAN),
                irq: fg(COLOR_BLUE) | A_BOLD,
                softirq: fg(COLOR_BLUE),
                steal: fg(COLOR_BLACK),
                guest: fg(COLOR_BLACK),
                iowait: fg(COLOR_BLACK) | A_BOLD,
            },
            memory: MemoryColors {
                used: fg(COLOR_GREEN),
                shared: fg(COLOR_MAGENTA),
                compressed: gray(),
                buffers: fg(COLOR_CYAN),
                cache: fg(COLOR_YELLOW),
            },
            swap: SwapColors {
                used: fg(COLOR_RED),
                cache: fg(COLOR_YELLOW),
            },
            fd: FdColors {
                used: fg(COLOR_GREEN),
                max: fg(COLOR_BLUE) | A_BOLD,
            },
            io_read: fg(COLOR_GREEN),
            io_write: fg(COLOR_YELLOW),
            led: fg(COLOR_GREEN),

            checkbox: fg(COLOR_BLUE),
            check_mark: fg(COLOR_BLACK),
        }
    }

    /// Light text on an explicit blue background
    fn midnight() -> Self {
        let b = COLOR_BLUE;
        Self {
            text: on(COLOR_WHITE, b),
            label: on(COLOR_CYAN, b),
            value: on(COLOR_CYAN, b) | A_BOLD,
            value_strong: on(COLOR_WHITE, b) | A_BOLD,
            value_weak: on(COLOR_BLACK, b),
            shadow: on(COLOR_BLACK, b),
            emphasis: on(COLOR_WHITE, b) | A_BOLD,

            ok: on(COLOR_GREEN, b),
            warn: on(COLOR_YELLOW, b) | A_BOLD,
            error: on(COLOR_RED, b) | A_BOLD,

            mega: on(COLOR_CYAN, b),
            giga: on(COLOR_GREEN, b),

            header: on(COLOR_BLACK, COLOR_CYAN),
            selection: on(COLOR_BLACK, COLOR_WHITE),
            selection_unfocused: on(COLOR_YELLOW, b),
            selection_follow: on(COLOR_BLACK, COLOR_YELLOW),
            function_bar: on(COLOR_BLACK, COLOR_CYAN),
            function_key: A_NORMAL,
            edit: on(COLOR_WHITE, COLOR_BLUE),
            tab_current: on(COLOR_BLACK, COLOR_CYAN),
            tab_current_border: on(COLOR_CYAN, COLOR_CYAN),
            tab_other: on(COLOR_CYAN, b),
            tab_other_border: on(COLOR_YELLOW, b),
            failed_search: on(COLOR_RED, COLOR_CYAN),
            paused: on(COLOR_BLACK, COLOR_CYAN),

            tag: on(COLOR_YELLOW, b),
            new_process: on(COLOR_BLUE, COLOR_GREEN),
            tomb: on(COLOR_BLUE, COLOR_RED),
            basename: on(COLOR_CYAN, b),
            thread: on(COLOR_GREEN, b),
            thread_basename: on(COLOR_GREEN, b),
            comm: on(COLOR_MAGENTA, b),
            thread_comm: on(COLOR_BLACK, b),
            high_priority: on(COLOR_RED, b),
            low_priority: on(COLOR_GREEN, b),

            bar_border: on(COLOR_YELLOW, b),
            bar_text: on(COLOR_WHITE, b) | A_BOLD,
            graph_strong: on(COLOR_CYAN, b),
            graph_weak: on(COLOR_CYAN, b),

            cpu: CpuColors {
                normal: on(COLOR_GREEN, b),
                system: on(COLOR_RED, b) | A_BOLD,
                nice: on(COLOR_CYAN, b),
                irq: on(COLOR_BLACK, b),
                softirq: on(COLOR_BLACK, b),
                steal: on(COLOR_WHITE, b),
                guest: on(COLOR_WHITE, b),
                iowait: on(COLOR_BLACK, b),
            },
            memory: MemoryColors {
                used: on(COLOR_GREEN, b),
                shared: on(COLOR_MAGENTA, b),
                compressed: on(COLOR_BLACK, b),
                buffers: on(COLOR_CYAN, b),
                cache: on(COLOR_YELLOW, b),
            },
            swap: SwapColors {
                used: on(COLOR_RED, b),
                cache: on(COLOR_YELLOW, b),
            },
            fd: FdColors {
                used: on(COLOR_GREEN, b),
                max: on(COLOR_RED, b) | A_BOLD,
            },
            io_read: on(COLOR_GREEN, b),
            io_write: on(COLOR_BLACK, b),
            led: on(COLOR_GREEN, b),

            checkbox: on(COLOR_CYAN, b),
            check_mark: on(COLOR_WHITE, b),
        }
    }

    /// Green-on-black console look with an explicit black background
    fn black_night() -> Self {
        Self {
            text: fg(COLOR_CYAN),
            label: fg(COLOR_CYAN),
            value: fg(COLOR_GREEN),
            value_strong: fg(COLOR_GREEN) | A_BOLD,
            value_weak: fg(COLOR_GREEN),
            shadow: gray() | A_BOLD,
            emphasis: fg(COLOR_WHITE) | A_BOLD,

            ok: fg(COLOR_GREEN),
            warn: fg(COLOR_YELLOW) | A_BOLD,
            error: fg(COLOR_RED) | A_BOLD,

            mega: fg(COLOR_GREEN) | A_BOLD,
            giga: fg(COLOR_YELLOW) | A_BOLD,

            header: on(COLOR_BLACK, COLOR_GREEN),
            selection: on(COLOR_BLACK, COLOR_CYAN),
            selection_unfocused: on(COLOR_BLACK, COLOR_WHITE),
            selection_follow: on(COLOR_BLACK, COLOR_YELLOW),
            function_bar: on(COLOR_BLACK, COLOR_GREEN),
            function_key: fg(COLOR_CYAN),
            edit: on(COLOR_WHITE, COLOR_CYAN),
            tab_current: fg(COLOR_GREEN) | A_BOLD,
            tab_current_border: fg(COLOR_WHITE) | A_BOLD,
            tab_other: fg(COLOR_CYAN),
            tab_other_border: fg(COLOR_WHITE),
            failed_search: on(COLOR_RED, COLOR_GREEN),
            paused: on(COLOR_YELLOW, COLOR_GREEN),

            tag: fg(COLOR_YELLOW) | A_BOLD,
            new_process: on(COLOR_BLACK, COLOR_GREEN),
            tomb: on(COLOR_BLACK, COLOR_RED),
            basename: fg(COLOR_GREEN) | A_BOLD,
            thread: fg(COLOR_GREEN),
            thread_basename: fg(COLOR_BLUE) | A_BOLD,
            comm: fg(COLOR_MAGENTA),
            thread_comm: fg(COLOR_YELLOW),
            high_priority: fg(COLOR_RED),
            low_priority: fg(COLOR_GREEN),

            bar_border: fg(COLOR_GREEN) | A_BOLD,
            bar_text: fg(COLOR_GREEN) | A_BOLD,
            graph_strong: fg(COLOR_GREEN) | A_BOLD,
            graph_weak: fg(COLOR_GREEN),

            cpu: CpuColors {
                normal: fg(COLOR_GREEN),
                system: fg(COLOR_RED),
                nice: fg(COLOR_BLUE),
                irq: fg(COLOR_BLUE) | A_BOLD,
                softirq: fg(COLOR_BLUE),
                steal: fg(COLOR_CYAN),
                guest: fg(COLOR_CYAN),
                iowait: fg(COLOR_YELLOW),
            },
            memory: MemoryColors {
                used: fg(COLOR_GREEN),
                shared: fg(COLOR_MAGENTA),
                compressed: gray() | A_BOLD,
                buffers: fg(COLOR_BLUE),
                cache: fg(COLOR_YELLOW),
            },
            swap: SwapColors {
                used: fg(COLOR_RED),
                cache: fg(COLOR_YELLOW),
            },
            fd: FdColors {
                used: fg(COLOR_GREEN),
                max: fg(COLOR_BLUE) | A_BOLD,
            },
            io_read: fg(COLOR_GREEN),
            io_write: fg(COLOR_BLUE),
            led: fg(COLOR_GREEN),

            checkbox: fg(COLOR_GREEN),
            check_mark: fg(COLOR_GREEN) | A_BOLD,
        }
    }

    /// The default scheme for terminals whose "bright black" is invisible:
    /// de-emphasised text is plain white instead of gray
    fn broken_gray() -> Self {
        let white = color_pair_white_default();
        let base = Self::default_dark();
        Self {
            shadow: white,
            memory: MemoryColors {
                compressed: white,
                ..base.memory
            },
            cpu: CpuColors {
                iowait: white,
                ..base.cpu
            },
            ..base
        }
    }

    /// A restrained scheme in the spirit of the Nord palette: mostly the
    /// terminal's own text colour with bold for emphasis, cyan and yellow as
    /// the only accents
    fn nord() -> Self {
        let cyan = fg(COLOR_CYAN);
        let yellow = fg(COLOR_YELLOW);
        let white = color_pair_white_default();
        let black_on_cyan = on(COLOR_BLACK, COLOR_CYAN);
        Self {
            text: A_NORMAL,
            label: A_NORMAL,
            value: A_BOLD,
            value_strong: cyan | A_BOLD,
            value_weak: gray() | A_BOLD,
            shadow: gray() | A_BOLD,
            emphasis: A_BOLD,

            ok: A_NORMAL,
            warn: A_BOLD,
            error: yellow | A_BOLD,

            mega: white | A_BOLD,
            giga: cyan | A_BOLD,

            header: black_on_cyan,
            selection: black_on_cyan,
            selection_unfocused: A_BOLD,
            selection_follow: A_REVERSE,
            function_bar: black_on_cyan,
            function_key: A_NORMAL,
            edit: A_BOLD,
            tab_current: black_on_cyan,
            tab_current_border: black_on_cyan,
            tab_other: gray() | A_BOLD,
            tab_other_border: gray() | A_BOLD,
            failed_search: yellow | A_REVERSE | A_BOLD,
            paused: black_on_cyan | A_BOLD,

            tag: cyan | A_BOLD,
            new_process: A_BOLD,
            tomb: gray() | A_BOLD,
            basename: A_BOLD,
            thread: A_NORMAL,
            thread_basename: A_BOLD,
            comm: A_BOLD,
            thread_comm: A_BOLD,
            high_priority: A_BOLD,
            low_priority: gray() | A_BOLD,

            bar_border: A_BOLD,
            bar_text: A_BOLD,
            graph_strong: A_BOLD,
            graph_weak: A_NORMAL,

            cpu: CpuColors {
                normal: A_BOLD,
                system: yellow | A_BOLD,
                nice: A_NORMAL,
                irq: A_BOLD,
                softirq: A_BOLD,
                steal: gray() | A_BOLD,
                guest: gray() | A_BOLD,
                iowait: A_NORMAL,
            },
            memory: MemoryColors {
                used: yellow | A_BOLD,
                shared: A_NORMAL,
                compressed: gray() | A_BOLD,
                buffers: A_NORMAL,
                cache: A_NORMAL,
            },
            swap: SwapColors {
                used: A_BOLD,
                cache: A_NORMAL,
            },
            fd: FdColors {
                used: A_BOLD,
                max: yellow | A_BOLD,
            },
            io_read: A_NORMAL,
            io_write: A_NORMAL,
            led: A_NORMAL,

            checkbox: A_BOLD,
            check_mark: A_NORMAL,
        }
    }
}

/// Value used to detect an element the derivation forgot to assign
const UNSET: attr_t = attr_t::MAX;

/// Derive the attribute for every [`ColorElement`] from a palette.
///
/// The result is indexed by `ColorElement as usize`.
pub fn derive(p: &Palette) -> Vec<attr_t> {
    use ColorElement as E;

    let mut colors = vec![UNSET; E::Last as usize];
    let mut set = |element: E, attr: attr_t| colors[element as usize] = attr;

    // Text
    set(E::ResetColor, p.text);
    set(E::DefaultColor, p.text);
    set(E::Process, p.text);
    set(E::CheckText, p.text);
    set(E::MeterText, p.label);
    set(E::ProcessTree, p.label);
    set(E::CheckBox, p.checkbox);
    set(E::MeterValue, p.value);
    set(E::Uptime, p.value);
    set(E::Battery, p.value);
    set(E::HelpBold, p.value);
    set(E::LoadAverageFive, p.value);
    set(E::MeterValueNotice, p.value_strong);
    set(E::LoadAverageOne, p.value_strong);
    set(E::LoadAverageFifteen, p.value_weak);
    set(E::MeterShadow, p.shadow);
    set(E::ProcessShadow, p.shadow);
    set(E::BarShadow, p.shadow);
    set(E::HelpShadow, p.shadow);
    set(E::Disabled, p.shadow);
    set(E::Load, p.emphasis);
    set(E::Hostname, p.emphasis);
    set(E::Clock, p.emphasis);
    set(E::Date, p.emphasis);
    set(E::DateTime, p.emphasis);
    set(E::CheckMark, p.check_mark);

    // Status
    set(E::MeterValueOk, p.ok);
    set(E::ProcessRunState, p.ok);
    set(E::TasksRunning, p.ok | A_BOLD);
    set(E::MeterValueWarn, p.warn);
    set(E::MeterValueError, p.error);
    set(E::LargeNumber, p.error);
    set(E::FailedRead, p.error);
    set(E::ProcessDState, p.error);

    // Magnitude
    set(E::ProcessMegabytes, p.mega);
    set(E::ProcessGigabytes, p.giga);

    // Panel chrome
    set(E::PanelHeaderFocus, p.header);
    set(E::PanelHeaderUnfocus, p.header);
    set(E::PanelSelectionFocus, p.selection);
    set(E::PanelSelectionUnfocus, p.selection_unfocused);
    set(E::PanelSelectionFollow, p.selection_follow);
    set(E::FunctionBar, p.function_bar);
    set(E::FunctionKey, p.function_key);
    set(E::PanelEdit, p.edit);
    set(E::ScreensCurText, p.tab_current);
    set(E::ScreensCurBorder, p.tab_current_border);
    set(E::ScreensOthText, p.tab_other);
    set(E::ScreensOthBorder, p.tab_other_border);
    set(E::FailedSearch, p.failed_search);
    set(E::Paused, p.paused);

    // Process rows
    set(E::ProcessTag, p.tag);
    set(E::ProcessNew, p.new_process);
    set(E::ProcessTomb, p.tomb);
    set(E::ProcessBasename, p.basename);
    set(E::ProcessThread, p.thread);
    set(E::ProcessThreadBasename, p.thread_basename);
    set(E::ProcessComm, p.comm);
    set(E::ProcessThreadComm, p.thread_comm);
    set(E::ProcessHighPriority, p.high_priority);
    set(E::ProcessLowPriority, p.low_priority);

    // Bars and graphs
    set(E::BarBorder, p.bar_border);
    set(E::BarText, p.bar_text);
    set(E::Graph1, p.graph_strong);
    set(E::Graph2, p.graph_weak);

    // Series
    set(E::CpuNormal, p.cpu.normal);
    set(E::CpuSystem, p.cpu.system);
    set(E::CpuNice, p.cpu.nice);
    set(E::CpuNiceText, p.cpu.nice);
    set(E::CpuIrq, p.cpu.irq);
    set(E::CpuSoftIrq, p.cpu.softirq);
    set(E::CpuSteal, p.cpu.steal);
    set(E::CpuGuest, p.cpu.guest);
    set(E::CpuIOWait, p.cpu.iowait);
    set(E::MemoryUsed, p.memory.used);
    set(E::MemoryShared, p.memory.shared);
    set(E::MemoryCompressed, p.memory.compressed);
    set(E::MemoryBuffers, p.memory.buffers);
    set(E::MemoryBuffersText, p.memory.buffers);
    set(E::MemoryCache, p.memory.cache);
    set(E::Swap, p.swap.used);
    set(E::SwapCache, p.swap.cache);
    set(E::FileDescriptorUsed, p.fd.used);
    set(E::FileDescriptorMax, p.fd.max);
    set(E::MeterValueIORead, p.io_read);
    set(E::MeterValueIOWrite, p.io_write);
    set(E::LedColor, p.led);

    debug_assert!(
        !colors.contains(&UNSET),
        "a ColorElement has no palette role assigned"
    );
    colors
}

#[cfg(test)]
mod tests {
    use super::*;
    use ColorElement as E;

    const SCHEMES: [ColorScheme; 8] = [
        ColorScheme::Default,
        ColorScheme::Monochrome,
        ColorScheme::BlackOnWhite,
        ColorScheme::LightTerminal,
        ColorScheme::Midnight,
        ColorScheme::BlackNight,
        ColorScheme::BrokenGray,
        ColorScheme::Nord,
    ];

    fn colors(scheme: ColorScheme) -> Vec<attr_t> {
        derive(&Palette::for_scheme(scheme))
    }

    fn same(colors: &[attr_t], elements: &[E]) -> bool {
        let first = colors[elements[0] as usize];
        elements.iter().all(|e| colors[*e as usize] == first)
    }

    #[test]
    fn every_element_is_assigned_in_every_scheme() {
        for scheme in SCHEMES {
            let c = colors(scheme);
            assert_eq!(c.len(), E::Last as usize);
            assert!(!c.contains(&UNSET), "{scheme:?} leaves an element unset");
        }
    }

    #[test]
    fn one_role_has_one_look_within_a_scheme() {
        for scheme in SCHEMES {
            let c = colors(scheme);
            assert!(
                same(
                    &c,
                    &[
                        E::MeterShadow,
                        E::ProcessShadow,
                        E::BarShadow,
                        E::HelpShadow,
                        E::Disabled
                    ]
                ),
                "{scheme:?}: shadow differs between components"
            );
            assert!(
                same(
                    &c,
                    &[
                        E::MeterValue,
                        E::Uptime,
                        E::Battery,
                        E::HelpBold,
                        E::LoadAverageFive
                    ]
                ),
                "{scheme:?}: value differs between components"
            );
            assert!(
                same(
                    &c,
                    &[
                        E::MeterValueError,
                        E::LargeNumber,
                        E::FailedRead,
                        E::ProcessDState
                    ]
                ),
                "{scheme:?}: error differs between components"
            );
            assert!(
                same(
                    &c,
                    &[E::ResetColor, E::DefaultColor, E::Process, E::CheckText]
                ),
                "{scheme:?}: text differs between components"
            );
            assert!(
                same(&c, &[E::PanelHeaderFocus, E::PanelHeaderUnfocus]),
                "{scheme:?}: header differs with focus"
            );
            assert!(
                same(&c, &[E::Clock, E::Date, E::DateTime, E::Hostname]),
                "{scheme:?}: headings differ between components"
            );
        }
    }

    #[test]
    fn bar_text_is_distinct_from_the_unfilled_part() {
        for scheme in SCHEMES {
            let c = colors(scheme);
            assert_ne!(
                c[E::BarText as usize],
                c[E::BarShadow as usize],
                "{scheme:?}: text over the fill would look like text beyond it"
            );
        }
    }

    #[test]
    fn broken_gray_only_replaces_gray() {
        let base = colors(ColorScheme::Default);
        let broken = colors(ColorScheme::BrokenGray);
        let white = color_pair_white_default();
        for (idx, (a, b)) in base.iter().zip(&broken).enumerate() {
            if a != b {
                assert_eq!(
                    *b, white,
                    "element {idx} changed to something other than white"
                );
            }
        }
        assert_eq!(broken[E::ProcessShadow as usize], white);
    }
}
