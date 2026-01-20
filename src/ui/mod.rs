//! UI module
//!
//! This module contains all UI-related components:
//! - CRT: Terminal abstraction using ncurses
//! - Panel: Scrollable list widget
//! - RichString: Attributed string for colored output
//! - ScreenManager: Manages panels and main loop
//! - Header: Meter display area
//! - FunctionBar: F1-F10 key labels
//! - MainPanel: Main process list panel
//! - RowPrint: Row printing utilities matching C htop
//! - SetupScreen: F2 configuration screen

mod crt;
mod function_bar;
mod header;
mod main_panel;
mod panel;
mod rich_string;
mod row_print;
mod screen_manager;
mod setup_screen;

pub use crt::*;
pub use header::*;
pub use main_panel::*;
pub use screen_manager::*;
pub use setup_screen::*;

// Re-export for internal use (used by main_panel and other modules)
#[allow(unused_imports)]
pub use rich_string::*;
#[allow(unused_imports)]
pub use row_print::*;
