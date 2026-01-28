//! Header - Meter display area at the top of the screen

#![allow(dead_code)]

use rayon::prelude::*;

use super::crt::{ColorElement, A_NORMAL};
use super::Crt;
use crate::core::{HeaderLayout, Machine, Settings};
use crate::meters::{Meter, MeterType};

/// Header containing meters
pub struct Header {
    /// Columns of meters
    columns: Vec<Vec<Box<dyn Meter>>>,

    /// Header layout
    pub layout: HeaderLayout,

    /// Calculated height
    pub height: i32,

    /// Vertical padding (top margin) - derived from header_margin setting
    /// When header_margin is true, this is 2 (1 line top + 1 line bottom padding)
    /// When header_margin is false, this is 0
    pub pad: i32,

    /// Whether header margin is enabled
    header_margin: bool,
}

impl Header {
    /// Create a new header
    pub fn new(_machine: &Machine, layout: HeaderLayout, header_margin: bool) -> Self {
        let num_cols = layout.num_columns();
        let mut columns = Vec::with_capacity(num_cols);
        for _ in 0..num_cols {
            columns.push(Vec::new());
        }
        let pad = if header_margin { 2 } else { 0 };
        Header {
            columns,
            layout,
            height: 0,
            pad,
            header_margin,
        }
    }

    /// Set the layout
    pub fn set_layout(&mut self, layout: HeaderLayout) {
        // Resize columns if needed
        let num_cols = layout.num_columns();
        while self.columns.len() < num_cols {
            self.columns.push(Vec::new());
        }
        while self.columns.len() > num_cols {
            self.columns.pop();
        }
        self.layout = layout;
    }

    /// Add a meter to a column
    pub fn add_meter(&mut self, column: usize, meter: Box<dyn Meter>) {
        if column < self.columns.len() {
            self.columns[column].push(meter);
        }
    }

    /// Clear all meters
    pub fn clear(&mut self) {
        for column in &mut self.columns {
            column.clear();
        }
    }

    /// Populate meters from settings
    pub fn populate_from_settings(&mut self, settings: &Settings) {
        self.clear();
        self.set_layout(settings.header_layout);

        // Update header margin
        self.header_margin = settings.header_margin;
        self.pad = if settings.header_margin { 2 } else { 0 };

        for (col_idx, column_config) in settings.header_columns.iter().enumerate() {
            for meter_config in column_config {
                if let Some(mut meter) =
                    MeterType::create_from_name(&meter_config.name, meter_config.param)
                {
                    // Apply the mode from config
                    meter.set_mode(meter_config.mode.into());
                    if col_idx < self.columns.len() {
                        self.columns[col_idx].push(meter);
                    }
                }
            }
        }

        self.calculate_height();
    }

    /// Calculate the header height (matches C htop Header_calculateHeight)
    pub fn calculate_height(&mut self) -> i32 {
        // Restore pad from header_margin setting before calculation
        let pad = if self.header_margin { 2 } else { 0 };
        self.pad = pad;

        let mut max_height = pad;

        for column in &self.columns {
            // Each column starts at pad (accounts for top margin)
            let mut col_height = pad;
            for meter in column {
                col_height += meter.height();
            }
            max_height = max_height.max(col_height);
        }

        // If no meters, no height needed (and no padding)
        if max_height == pad {
            max_height = 0;
            self.pad = 0;
        }

        self.height = max_height;
        self.height
    }

    /// Update meter data (parallel using rayon)
    ///
    /// Meters are updated in parallel since some (like BatteryMeter) may do
    /// expensive I/O operations. Most meters just copy data from Machine,
    /// but parallelizing ensures expensive meters don't block others.
    pub fn update(&mut self, machine: &Machine) {
        // Update all meters in parallel across all columns
        self.columns.par_iter_mut().for_each(|column| {
            column.par_iter_mut().for_each(|meter| {
                meter.update(machine);
            });
        });
        // Recalculate height since CPU meter height depends on CPU count
        self.calculate_height();
    }

    /// Draw the header (matches C htop Header_draw)
    pub fn draw(&self, crt: &mut Crt, machine: &Machine, settings: &Settings) {
        let screen_width = crt.width();
        let height = self.height;
        let pad = self.pad;

        // Clear header area with spaces (like C htop)
        let reset_attr = crt.color(ColorElement::ResetColor);
        crt.attrset(reset_attr);
        for y in 0..height {
            crt.hline(y, 0, ' ' as u32, screen_width);
        }
        crt.attrset(A_NORMAL);

        let num_cols = self.columns.len();
        if num_cols == 0 {
            return;
        }

        // Calculate usable width: screen width minus padding on both sides and column separators
        let usable_width = screen_width - 2 * pad - (num_cols as i32 - 1);
        let widths = self.layout.column_widths();

        let mut x = pad; // Start after left padding
        let mut rounding_loss: f32 = 0.0;

        for (col_idx, column) in self.columns.iter().enumerate() {
            // Calculate column width as proportion of usable width
            let width_percent = widths.get(col_idx).copied().unwrap_or(0.5);
            let mut col_width = (usable_width as f32 * width_percent as f32).floor();

            // Accumulate rounding loss and add back when >= 1
            rounding_loss += (usable_width as f32 * width_percent as f32) - col_width;
            if rounding_loss >= 1.0 {
                col_width += 1.0;
                rounding_loss -= 1.0;
            }

            let col_width = col_width as i32;

            // Draw meters in this column, starting at y = pad / 2 (top padding)
            let mut y = pad / 2;
            for meter in column {
                meter.draw(crt, machine, settings, x, y, col_width);
                y += meter.height();
            }

            x += col_width;
            x += 1; // Column separator
        }
    }

    /// Get the header height
    pub fn get_height(&self) -> i32 {
        self.height
    }

    /// Update header margin setting
    pub fn set_header_margin(&mut self, header_margin: bool) {
        self.header_margin = header_margin;
        self.pad = if header_margin { 2 } else { 0 };
    }

    /// Reinitialize meters (after settings change)
    pub fn reinit(&mut self) {
        for column in &mut self.columns {
            for meter in column {
                meter.init();
            }
        }
        self.calculate_height();
    }

    /// Get the supported modes for a meter at a specific position
    pub fn get_meter_supported_modes(&self, column: usize, index: usize) -> Option<u32> {
        self.columns
            .get(column)
            .and_then(|col| col.get(index))
            .map(|meter| meter.supported_modes())
    }
}
