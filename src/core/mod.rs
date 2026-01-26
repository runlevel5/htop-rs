//! Core module containing the main data structures
//!
//! This module contains:
//! - Process: represents a single process
//! - Machine: represents system state (CPU, memory, processes)
//! - Settings: user configuration
//! - FieldWidths: dynamic column width management

#![allow(dead_code)]

mod field_widths;
mod machine;
mod process;
mod settings;

pub use field_widths::*;
pub use machine::*;
pub use process::*;
pub use settings::*;
