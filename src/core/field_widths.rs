//! Dynamic field width management for process columns
//!
//! This module handles dynamic column widths similar to C htop's Row.c:
//! - PID columns: width based on max PID value (min 5 digits)
//! - UID column: width based on max UID value (min 5 digits)
//! - Auto-width columns: width expands based on content (e.g., PERCENT_CPU)

use super::process::ProcessField;

/// Minimum width for PID columns (matches C htop's ROW_MIN_PID_DIGITS)
pub const MIN_PID_DIGITS: usize = 5;
/// Maximum width for PID columns (matches C htop's ROW_MAX_PID_DIGITS)
pub const MAX_PID_DIGITS: usize = 19;

/// Minimum width for UID column
pub const MIN_UID_DIGITS: usize = 5;
/// Maximum width for UID column
pub const MAX_UID_DIGITS: usize = 10;

/// Default width for PERCENT_CPU (title " CPU%" is 5 chars, so width=5)
/// This matches C htop's Row_resetFieldWidths which sets width = strlen(title)
const DEFAULT_PERCENT_CPU_WIDTH: usize = 5;

/// Manages dynamic column widths for process fields
#[derive(Debug, Clone)]
pub struct FieldWidths {
    /// Width for PID-type columns (PID, PPID, PGRP, SID, TPGID, TGID)
    pub pid_digits: usize,
    /// Width for UID column
    pub uid_digits: usize,
    /// Width for PERCENT_CPU column (auto-width based on max CPU%)
    pub percent_cpu_width: usize,
    /// Width for PERCENT_NORM_CPU column
    pub percent_norm_cpu_width: usize,
}

impl Default for FieldWidths {
    fn default() -> Self {
        Self {
            pid_digits: MIN_PID_DIGITS,
            uid_digits: MIN_UID_DIGITS,
            percent_cpu_width: DEFAULT_PERCENT_CPU_WIDTH,
            percent_norm_cpu_width: DEFAULT_PERCENT_CPU_WIDTH,
        }
    }
}

impl FieldWidths {
    /// Create a new FieldWidths with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Update PID column width based on max PID value
    /// Returns true if the width changed
    pub fn set_pid_width(&mut self, max_pid: i32) -> bool {
        let old = self.pid_digits;
        let digits = count_digits(max_pid as u64);
        self.pid_digits = digits.clamp(MIN_PID_DIGITS, MAX_PID_DIGITS);
        self.pid_digits != old
    }

    /// Update UID column width based on max UID value
    /// Returns true if the width changed
    pub fn set_uid_width(&mut self, max_uid: u32) -> bool {
        let old = self.uid_digits;
        let digits = count_digits(max_uid as u64);
        self.uid_digits = digits.clamp(MIN_UID_DIGITS, MAX_UID_DIGITS);
        self.uid_digits != old
    }

    /// Update PERCENT_CPU width based on max CPU percentage
    /// Width needs to accommodate values like "100.0" or "1234.5" (for multi-core totals)
    /// The width is the number of characters for the value itself (not including trailing space)
    pub fn update_percent_cpu_width(&mut self, max_percent: f32) {
        // Width calculation: we need space for the number + 1 decimal place
        // Examples: "99.9" (4), "100.0" (5), "999.9" (5), "1000.0" (6), "9999.9" (6)
        // C htop initializes to strlen(" CPU%") = 5, and Row_printPercentage needs min 4
        let width = if max_percent < 100.0 {
            5 // "XX.X " or "X.X  " - minimum is title width
        } else if max_percent < 1000.0 {
            5 // "XXX.X"
        } else if max_percent < 10000.0 {
            6 // "XXXX.X"
        } else {
            7 // "XXXXX.X"
        };
        
        // Only expand, never shrink (matches C htop behavior)
        if width > self.percent_cpu_width {
            self.percent_cpu_width = width;
        }
    }

    /// Update PERCENT_NORM_CPU width (same logic as PERCENT_CPU)
    pub fn update_percent_norm_cpu_width(&mut self, max_percent: f32) {
        // Same logic as PERCENT_CPU - min width is 5 (strlen("NCPU%"))
        let width = if max_percent < 100.0 {
            5 // Minimum is title width
        } else if max_percent < 1000.0 {
            5 // "XXX.X"
        } else if max_percent < 10000.0 {
            6 // "XXXX.X"
        } else {
            7 // "XXXXX.X"
        };
        
        if width > self.percent_norm_cpu_width {
            self.percent_norm_cpu_width = width;
        }
    }

    /// Reset auto-width fields to their minimum values (called on screen refresh)
    pub fn reset_auto_widths(&mut self) {
        self.percent_cpu_width = DEFAULT_PERCENT_CPU_WIDTH;
        self.percent_norm_cpu_width = DEFAULT_PERCENT_CPU_WIDTH;
    }

    /// Check if a field is a PID-type column
    pub fn is_pid_column(field: ProcessField) -> bool {
        matches!(
            field,
            ProcessField::Pid
                | ProcessField::Ppid
                | ProcessField::Pgrp
                | ProcessField::Session
                | ProcessField::Tpgid
                | ProcessField::Tgid
        )
    }

    /// Check if a field has auto-width
    pub fn is_auto_width(field: ProcessField) -> bool {
        matches!(
            field,
            ProcessField::PercentCpu | ProcessField::PercentNormCpu
        )
    }

    /// Get the render width for a field (not including trailing space)
    pub fn get_width(&self, field: ProcessField) -> usize {
        if Self::is_pid_column(field) {
            self.pid_digits
        } else if field == ProcessField::StUid {
            self.uid_digits
        } else if field == ProcessField::PercentCpu {
            self.percent_cpu_width
        } else if field == ProcessField::PercentNormCpu {
            self.percent_norm_cpu_width
        } else {
            // For non-dynamic fields, return 0 (caller should use static width)
            0
        }
    }

    /// Get the title for a field with proper width padding
    /// Returns a String because PID columns need dynamic padding
    pub fn get_title(&self, field: ProcessField) -> String {
        let base_title = field.base_title();
        
        if Self::is_pid_column(field) {
            // Right-align PID column titles
            format!("{:>width$} ", base_title, width = self.pid_digits)
        } else if field == ProcessField::StUid {
            // Right-align UID title
            format!("{:>width$} ", base_title, width = self.uid_digits)
        } else if field == ProcessField::PercentCpu {
            // Right-align CPU% title (autoTitleRightAlign = true in C htop)
            format!("{:>width$} ", base_title, width = self.percent_cpu_width)
        } else if field == ProcessField::PercentNormCpu {
            // Left-align NCPU% title (autoTitleRightAlign = false in C htop)
            format!("{:<width$} ", base_title, width = self.percent_norm_cpu_width)
        } else {
            // Static width fields - return base title as-is
            base_title.to_string()
        }
    }
}

/// Count the number of decimal digits needed to represent a number
fn count_digits(n: u64) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    let mut val = n;
    while val > 0 {
        count += 1;
        val /= 10;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_digits() {
        assert_eq!(count_digits(0), 1);
        assert_eq!(count_digits(1), 1);
        assert_eq!(count_digits(9), 1);
        assert_eq!(count_digits(10), 2);
        assert_eq!(count_digits(99), 2);
        assert_eq!(count_digits(100), 3);
        assert_eq!(count_digits(99999), 5);
        assert_eq!(count_digits(100000), 6);
    }

    #[test]
    fn test_pid_width() {
        let mut fw = FieldWidths::new();
        assert_eq!(fw.pid_digits, 5);
        
        // Small PIDs should keep min width
        fw.set_pid_width(1000);
        assert_eq!(fw.pid_digits, 5);
        
        // Large PIDs should expand
        fw.set_pid_width(100000);
        assert_eq!(fw.pid_digits, 6);
        
        fw.set_pid_width(1000000);
        assert_eq!(fw.pid_digits, 7);
    }

    #[test]
    fn test_percent_cpu_width() {
        let mut fw = FieldWidths::new();
        assert_eq!(fw.percent_cpu_width, 5); // Default is strlen(" CPU%") = 5
        
        // Small values keep default (min is 5)
        fw.update_percent_cpu_width(50.0);
        assert_eq!(fw.percent_cpu_width, 5);
        
        // Values < 1000 stay at 5
        fw.update_percent_cpu_width(150.0);
        assert_eq!(fw.percent_cpu_width, 5);
        
        // Values >= 1000 expand to 6
        fw.update_percent_cpu_width(1234.5);
        assert_eq!(fw.percent_cpu_width, 6);
        
        // Never shrinks
        fw.update_percent_cpu_width(10.0);
        assert_eq!(fw.percent_cpu_width, 6);
    }
}
