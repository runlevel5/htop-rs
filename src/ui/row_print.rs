//! Row printing utilities - C htop compatible field formatters
//!
//! These functions match the exact formatting and coloring of C htop's Row.c:
//! - Row_printKBytes: Format memory values in KiB with unit prefixes and coloring
//! - Row_printTime: Format time values with coloring by magnitude
//! - Row_printPercentage: Format percentages with shadow/highlight coloring

#![allow(dead_code)]

use ncurses::attr_t;
use super::crt::ColorElement;
use super::rich_string::RichString;
use super::Crt;

/// Unit prefixes for memory formatting (K, M, G, T, P, E)
const UNIT_PREFIXES: [char; 6] = ['K', 'M', 'G', 'T', 'P', 'E'];

/// Format a value in KiB with appropriate unit prefix and coloring
/// Matches C htop's Row_printKBytes exactly (6 columns width)
///
/// Colors by magnitude:
/// - < 1000K: normal (PROCESS)
/// - 1K-99M: megabytes prefix in PROCESS_MEGABYTES
/// - 100M-999M: full number in PROCESS_MEGABYTES
/// - >= 1G: PROCESS_GIGABYTES / LARGE_NUMBER
pub fn print_kbytes(str: &mut RichString, number: u64, coloring: bool, crt: &Crt) {
    let process_color = crt.color(ColorElement::Process);
    let megabytes_color = crt.color(ColorElement::ProcessMegabytes);
    let gigabytes_color = crt.color(ColorElement::ProcessGigabytes);
    let large_number_color = crt.color(ColorElement::LargeNumber);
    let shadow_color = crt.color(ColorElement::ProcessShadow);

    // Handle invalid/unknown values
    if number == u64::MAX {
        let color = if coloring { shadow_color } else { process_color };
        str.append("  N/A ", color);
        return;
    }

    let colors: [attr_t; 4] = [
        process_color,
        megabytes_color,
        gigabytes_color,
        large_number_color,
    ];

    let (color, next_unit_color) = if coloring {
        (colors[0], colors[1])
    } else {
        (process_color, process_color)
    };

    // < 1000K: plain number
    if number < 1000 {
        str.append(&format!("{:>5} ", number), color);
        return;
    }

    // 1000K - 99999K: show as "NNKKK " (2 digits M prefix + 3 digits K)
    if number < 100_000 {
        let high = number / 1000;
        let low = number % 1000;
        str.append(&format!("{:>2}", high), next_unit_color);
        str.append(&format!("{:03} ", low), color);
        return;
    }

    // >= 100000K (97.6 MiB or more): need unit prefix
    // Convert KiB to (1/100) of MiB
    let mut hundredths = (number / 256) * 25 + (number % 256) * 25 / 256;
    let mut unit_index = 1usize; // Start at M
    
    let mut prev_unit_color = color;
    let mut current_color = next_unit_color;
    let mut current_next_color = if coloring && unit_index + 1 < colors.len() {
        colors[unit_index + 1]
    } else {
        current_color
    };

    // Scale up until hundredths < 1000000 (i.e., value < 10000 in current unit)
    while hundredths >= 1_000_000 {
        hundredths /= 1024;
        unit_index += 1;
        prev_unit_color = current_color;
        current_color = current_next_color;
        if coloring && unit_index + 1 < colors.len() {
            current_next_color = colors[unit_index + 1];
        }
        if unit_index >= UNIT_PREFIXES.len() {
            // Overflow - show N/A
            let color = if coloring { shadow_color } else { process_color };
            str.append("  N/A ", color);
            return;
        }
    }

    let value = hundredths / 100;
    let frac = hundredths % 100;
    let unit = UNIT_PREFIXES[unit_index];

    if value < 10 {
        // "9.76G" format: 1 digit + decimal point + 2 digits + unit
        str.append(&format!("{}", value), current_color);
        str.append(&format!(".{:02}", frac), prev_unit_color);
        str.append(&format!("{} ", unit), current_color);
    } else if value < 100 {
        // "97.6M" format: 2 digits + decimal point + 1 digit + unit
        str.append(&format!("{:>2}", value), current_color);
        str.append(&format!(".{}", frac / 10), prev_unit_color);
        str.append(&format!("{} ", unit), current_color);
    } else if value < 1000 {
        // "100M" format: 3-4 digits + unit
        str.append(&format!("{:>4}{} ", value, unit), current_color);
    } else {
        // "1000M" format: 1 digit (thousands) + 3 digits + unit
        str.append(&format!("{}", value / 1000), current_next_color);
        str.append(&format!("{:03}{} ", value % 1000, unit), current_color);
    }
}

/// Format time in hundredths of seconds with coloring by magnitude
/// Matches C htop's Row_printTime exactly (9 columns width)
///
/// Format varies by magnitude:
/// - < 60m: " m:ss.cc " (minutes:seconds.centiseconds)
/// - < 24h: "NNhMM:SS " (hours + minutes:seconds)
/// - < 10d: "NdHHhMMm " (days + hours + minutes)
/// - < 365d: "NNNNdHHh " (days + hours)
/// - >= 1y: "NNNyDDDd " or "NNNNNNy " (years + days)
pub fn print_time(str: &mut RichString, total_hundredths: u64, coloring: bool, crt: &Crt) {
    let base_color = crt.color(ColorElement::Process);
    let shadow_color = crt.color(ColorElement::ProcessShadow);
    let hour_color = if coloring { crt.color(ColorElement::ProcessMegabytes) } else { base_color };
    let day_color = if coloring { crt.color(ColorElement::ProcessGigabytes) } else { base_color };
    let year_color = if coloring { crt.color(ColorElement::LargeNumber) } else { base_color };

    // Zero time
    if total_hundredths == 0 {
        let color = if coloring { shadow_color } else { base_color };
        str.append(" 0:00.00 ", color);
        return;
    }

    let total_seconds = total_hundredths / 100;
    let total_minutes = total_seconds / 60;
    let total_hours = total_minutes / 60;
    let seconds = (total_seconds % 60) as u32;
    let minutes = (total_minutes % 60) as u32;

    // < 60 minutes: "mm:ss.cc "
    if total_minutes < 60 {
        let hundredths = (total_hundredths % 100) as u32;
        str.append(&format!("{:>2}:{:02}.{:02} ", total_minutes, seconds, hundredths), base_color);
        return;
    }

    // < 24 hours: "NNhMM:SS "
    if total_hours < 24 {
        str.append(&format!("{:>2}h", total_hours), hour_color);
        str.append(&format!("{:02}:{:02} ", minutes, seconds), base_color);
        return;
    }

    let total_days = total_hours / 24;
    let hours = (total_hours % 24) as u32;

    // < 10 days: "NdHHhMMm "
    if total_days < 10 {
        str.append(&format!("{}d", total_days), day_color);
        str.append(&format!("{:02}h", hours), hour_color);
        str.append(&format!("{:02}m ", minutes), base_color);
        return;
    }

    // < 365 days: "NNNNdHHh "
    if total_days < 365 {
        str.append(&format!("{:>4}d", total_days), day_color);
        str.append(&format!("{:02}h ", hours), hour_color);
        return;
    }

    // >= 1 year
    let years = total_days / 365;
    let days = (total_days % 365) as u32;

    if years < 1000 {
        str.append(&format!("{:>3}y", years), year_color);
        str.append(&format!("{:03}d ", days), day_color);
    } else if years < 10_000_000 {
        str.append(&format!("{:>7}y ", years), year_color);
    } else {
        str.append("eternity ", year_color);
    }
}

/// Format a percentage with appropriate coloring
/// Matches C htop's Row_printPercentage
///
/// Colors:
/// - < 0.05: PROCESS_SHADOW (dim)
/// - >= 99.9: PROCESS_MEGABYTES (highlight)
/// - else: DEFAULT_COLOR
///
/// Width is typically 4-5 characters + trailing space
pub fn print_percentage(str: &mut RichString, val: f32, width: usize, crt: &Crt) -> attr_t {
    let mut attr = crt.color(ColorElement::DefaultColor);
    let shadow_color = crt.color(ColorElement::ProcessShadow);
    let megabytes_color = crt.color(ColorElement::ProcessMegabytes);

    if !val.is_finite() || val < 0.0 {
        attr = shadow_color;
        str.append(&format!("{:>width$} ", "N/A", width = width), attr);
        return attr;
    }

    if val < 0.05 {
        attr = shadow_color;
    } else if val >= 99.9 {
        attr = megabytes_color;
    }

    let precision = if width == 4 && val > 99.9 {
        // Display as "100" for narrow columns like "MEM%"
        str.append(&format!("{:>width$} ", 100, width = width), attr);
        return attr;
    } else {
        1
    };

    str.append(&format!("{:>width$.prec$} ", val, width = width, prec = precision), attr);
    attr
}

/// Format a left-aligned field with given width
pub fn print_left_aligned(str: &mut RichString, attr: attr_t, content: &str, width: usize) {
    let content_len = content.chars().count();
    if content_len >= width {
        // Truncate
        str.append_n(content, attr, width);
        str.append_char(' ', attr);
    } else {
        // Pad
        str.append(content, attr);
        str.append_chr(' ', attr, width - content_len + 1);
    }
}

/// Format a right-aligned integer field
pub fn print_right_aligned_int(str: &mut RichString, attr: attr_t, value: i64, width: usize) {
    str.append(&format!("{:>width$} ", value, width = width), attr);
}

/// Format a right-aligned unsigned integer field
pub fn print_right_aligned_uint(str: &mut RichString, attr: attr_t, value: u64, width: usize) {
    str.append(&format!("{:>width$} ", value, width = width), attr);
}
