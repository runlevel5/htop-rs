//! Stub meters for unimplemented meter types
//!
//! These meters display "Not implemented" text and serve as placeholders
//! until the actual implementation is complete.

use super::{Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::{ColorElement, Crt};

/// Helper function to draw "Not implemented" text for stub meters
fn draw_not_implemented(crt: &Crt, x: i32, y: i32, caption: &str) {
    use ncurses::*;

    let caption_attr = crt.color(ColorElement::MeterText);
    let text_attr = crt.color(ColorElement::MeterValueError);

    mv(y, x);
    attrset(caption_attr);
    let _ = addstr(caption);
    attrset(text_attr);
    let _ = addstr("Not implemented");
    attrset(crt.color(ColorElement::ResetColor));
}

// Bitmask constants for supported modes
const TEXT_ONLY_MODES: u32 = 1 << (MeterMode::Text as u32);
const ALL_MODES: u32 = (1 << MeterMode::Bar as u32)
    | (1 << MeterMode::Text as u32)
    | (1 << MeterMode::Graph as u32)
    | (1 << MeterMode::Led as u32);

/// Macro to generate a stub meter implementation
macro_rules! stub_meter {
    ($name:ident, $internal_name:expr, $caption:expr, $default_mode:expr, $supported_modes:expr) => {
        #[derive(Debug, Default)]
        pub struct $name {
            mode: MeterMode,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    mode: $default_mode,
                }
            }
        }

        impl Meter for $name {
            fn name(&self) -> &'static str {
                $internal_name
            }

            fn caption(&self) -> &str {
                $caption
            }

            fn update(&mut self, _machine: &Machine) {
                // Not implemented
            }

            fn draw(
                &self,
                crt: &Crt,
                _machine: &Machine,
                _settings: &Settings,
                x: i32,
                y: i32,
                _width: i32,
            ) {
                draw_not_implemented(crt, x, y, $caption);
            }

            fn mode(&self) -> MeterMode {
                self.mode
            }

            fn set_mode(&mut self, mode: MeterMode) {
                self.mode = mode;
            }

            fn default_mode(&self) -> MeterMode {
                $default_mode
            }

            fn supported_modes(&self) -> u32 {
                $supported_modes
            }
        }
    };
}

// ============================================================================
// Cross-platform meters
// ============================================================================

// MemorySwap Meter - Combined memory and swap usage
stub_meter!(
    MemorySwapMeter,
    "MemorySwap",
    "M&S: ",
    MeterMode::Bar,
    ALL_MODES
);

// System Meter - System information
stub_meter!(
    SystemMeter,
    "System",
    "System: ",
    MeterMode::Text,
    TEXT_ONLY_MODES
);

// DiskIORate Meter - Disk IO read & write bytes per second
stub_meter!(
    DiskIORateMeter,
    "DiskIORate",
    "Dsk: ",
    MeterMode::Text,
    ALL_MODES
);

// DiskIOTime Meter - Disk percent time busy
stub_meter!(
    DiskIOTimeMeter,
    "DiskIOTime",
    "Dsk: ",
    MeterMode::Text,
    ALL_MODES
);

// FileDescriptors Meter - Number of allocated/available file descriptors
stub_meter!(
    FileDescriptorsMeter,
    "FileDescriptors",
    "FDs: ",
    MeterMode::Text,
    ALL_MODES
);

// ============================================================================
// Linux-specific meters
// ============================================================================

// HugePages Meter
stub_meter!(
    HugePagesMeter,
    "HugePages",
    "HP: ",
    MeterMode::Bar,
    ALL_MODES
);

// PressureStallCPUSome Meter
stub_meter!(
    PressureStallCPUSomeMeter,
    "PressureStallCPUSome",
    "PSI some CPU:    ",
    MeterMode::Text,
    ALL_MODES
);

// PressureStallIOSome Meter
stub_meter!(
    PressureStallIOSomeMeter,
    "PressureStallIOSome",
    "PSI some IO:     ",
    MeterMode::Text,
    ALL_MODES
);

// PressureStallIOFull Meter
stub_meter!(
    PressureStallIOFullMeter,
    "PressureStallIOFull",
    "PSI full IO:     ",
    MeterMode::Text,
    ALL_MODES
);

// PressureStallIRQFull Meter
stub_meter!(
    PressureStallIRQFullMeter,
    "PressureStallIRQFull",
    "PSI full IRQ:    ",
    MeterMode::Text,
    ALL_MODES
);

// PressureStallMemorySome Meter
stub_meter!(
    PressureStallMemorySomeMeter,
    "PressureStallMemorySome",
    "PSI some memory: ",
    MeterMode::Text,
    ALL_MODES
);

// PressureStallMemoryFull Meter
stub_meter!(
    PressureStallMemoryFullMeter,
    "PressureStallMemoryFull",
    "PSI full memory: ",
    MeterMode::Text,
    ALL_MODES
);

// Zram Meter
stub_meter!(ZramMeter, "Zram", "zrm: ", MeterMode::Bar, ALL_MODES);

// SELinux Meter
stub_meter!(
    SELinuxMeter,
    "SELinux",
    "SELinux: ",
    MeterMode::Text,
    TEXT_ONLY_MODES
);

// Systemd Meter
stub_meter!(
    SystemdMeter,
    "Systemd",
    "Systemd: ",
    MeterMode::Text,
    TEXT_ONLY_MODES
);

// SystemdUser Meter
stub_meter!(
    SystemdUserMeter,
    "SystemdUser",
    "Systemd User: ",
    MeterMode::Text,
    TEXT_ONLY_MODES
);

// ============================================================================
// ZFS meters (multi-platform where ZFS is available)
// ============================================================================

// ZFSARC Meter
stub_meter!(ZfsArcMeter, "ZFSARC", "ARC: ", MeterMode::Bar, ALL_MODES);

// ZFSCARC Meter - Compressed ARC
stub_meter!(
    ZfsCompressedArcMeter,
    "ZFSCARC",
    "ARC: ",
    MeterMode::Bar,
    ALL_MODES
);
