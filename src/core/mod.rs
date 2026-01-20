//! Core module containing the main data structures
//!
//! This module contains:
//! - Process: represents a single process
//! - Machine: represents system state (CPU, memory, processes)
//! - Settings: user configuration

#![allow(dead_code)]

mod machine;
mod process;
mod settings;

pub use machine::*;
pub use process::*;
pub use settings::*;
