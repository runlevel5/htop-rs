//! Battery Meter
//!
//! Displays battery percentage and charging status.

use super::{draw_bar, Meter, MeterMode};
use crate::core::{Machine, Settings};
use crate::ui::ColorElement;
use crate::ui::Crt;

/// Battery status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ACPresence {
    #[default]
    Unknown,
    Online,
    Offline,
}

/// Battery Meter - displays battery percentage and AC status
#[derive(Debug, Default)]
pub struct BatteryMeter {
    mode: MeterMode,
    percent: f64,
    ac_presence: ACPresence,
    available: bool,
}

impl BatteryMeter {
    pub fn new() -> Self {
        BatteryMeter::default()
    }

    /// Get battery information (platform-specific)
    #[cfg(target_os = "macos")]
    fn get_battery_info() -> Option<(f64, ACPresence)> {
        use std::process::Command;

        // Use pmset to get battery info on macOS
        let output = Command::new("pmset").arg("-g").arg("batt").output().ok()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse output like:
        // Now drawing from 'Battery Power'
        //  -InternalBattery-0 (id=...)    100%; charged; 0:00 remaining present: true
        // OR:
        // Now drawing from 'AC Power'
        //  -InternalBattery-0 (id=...)    100%; charging; ...

        let mut percent = None;
        let mut ac_presence = ACPresence::Unknown;

        for line in output_str.lines() {
            if line.contains("AC Power") {
                ac_presence = ACPresence::Online;
            } else if line.contains("Battery Power") {
                ac_presence = ACPresence::Offline;
            }

            // Look for percentage like "100%"
            if let Some(pct_pos) = line.find('%') {
                // Find the start of the number
                let before = &line[..pct_pos];
                if let Some(num_start) = before.rfind(|c: char| !c.is_ascii_digit() && c != '.') {
                    if let Ok(pct) = before[num_start + 1..].trim().parse::<f64>() {
                        percent = Some(pct);
                    }
                } else if let Ok(pct) = before.trim().parse::<f64>() {
                    percent = Some(pct);
                }
            }
        }

        percent.map(|p| (p, ac_presence))
    }

    #[cfg(target_os = "linux")]
    fn get_battery_info() -> Option<(f64, ACPresence)> {
        use std::fs;
        use std::path::Path;

        let power_supply_path = Path::new("/sys/class/power_supply");

        if !power_supply_path.exists() {
            return None;
        }

        let mut total_capacity = 0.0;
        let mut battery_count = 0;
        let mut ac_presence = ACPresence::Unknown;

        if let Ok(entries) = fs::read_dir(power_supply_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                // Check for AC adapter
                if name_str.starts_with("AC")
                    || name_str.starts_with("ACAD")
                    || name_str.contains("ADP")
                {
                    let online_path = path.join("online");
                    if let Ok(content) = fs::read_to_string(&online_path) {
                        ac_presence = if content.trim() == "1" {
                            ACPresence::Online
                        } else {
                            ACPresence::Offline
                        };
                    }
                }

                // Check for battery
                if name_str.starts_with("BAT") {
                    let capacity_path = path.join("capacity");
                    if let Ok(content) = fs::read_to_string(&capacity_path) {
                        if let Ok(cap) = content.trim().parse::<f64>() {
                            total_capacity += cap;
                            battery_count += 1;
                        }
                    }
                }
            }
        }

        if battery_count > 0 {
            Some((total_capacity / battery_count as f64, ac_presence))
        } else {
            None
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn get_battery_info() -> Option<(f64, ACPresence)> {
        None
    }

    pub(crate) fn format_battery(&self) -> String {
        if !self.available {
            return "N/A".to_string();
        }

        let ac_str = match self.ac_presence {
            ACPresence::Online => "; AC",
            ACPresence::Offline => "; BAT",
            ACPresence::Unknown => "",
        };

        format!("{:.1}%{}", self.percent, ac_str)
    }
}

impl Meter for BatteryMeter {
    fn name(&self) -> &'static str {
        "Battery"
    }

    fn caption(&self) -> &str {
        "BAT"
    }

    fn update(&mut self, _machine: &Machine) {
        if let Some((percent, ac_presence)) = Self::get_battery_info() {
            self.percent = percent;
            self.ac_presence = ac_presence;
            self.available = true;
        } else {
            self.percent = 0.0;
            self.ac_presence = ACPresence::Unknown;
            self.available = false;
        }
    }

    fn draw(
        &self,
        crt: &Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        use crate::ncurses_compat::*;

        match self.mode {
            MeterMode::Bar => {
                if !self.available {
                    // Draw N/A in text mode
                    let caption_attr = crt.color(ColorElement::MeterText);
                    let value_attr = crt.color(ColorElement::MeterValueError);

                    mv(y, x);
                    attrset(caption_attr);
                    let _ = addstr("BAT");

                    attrset(value_attr);
                    let _ = addstr(" N/A");
                    return;
                }

                // Draw caption
                let caption_attr = crt.color(ColorElement::MeterText);
                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("BAT");

                // Determine bar color based on percentage
                let bar_color = if self.percent > 50.0 {
                    ColorElement::Battery as i32
                } else if self.percent > 20.0 {
                    ColorElement::MeterValueWarn as i32
                } else {
                    ColorElement::MeterValueError as i32
                };

                let bar_width = width - 3; // Account for "BAT"
                draw_bar(
                    crt,
                    x + 3,
                    y,
                    bar_width,
                    &[(self.percent, bar_color)],
                    100.0,
                );

                // Overlay percentage text
                let text = self.format_battery();
                let text_x = x + width - text.len() as i32 - 2;
                if text_x > x + 4 {
                    let value_attr = crt.color(ColorElement::MeterValue);
                    mv(y, text_x);
                    attrset(value_attr);
                    let _ = addstr(&text);
                }
            }
            _ => {
                // Text mode
                let caption_attr = crt.color(ColorElement::MeterText);
                let value_attr = if self.available {
                    crt.color(ColorElement::Battery)
                } else {
                    crt.color(ColorElement::MeterValueError)
                };

                mv(y, x);
                attrset(caption_attr);
                let _ = addstr("Battery: ");

                attrset(value_attr);
                let _ = addstr(&self.format_battery());
            }
        }
    }

    fn mode(&self) -> MeterMode {
        self.mode
    }

    fn set_mode(&mut self, mode: MeterMode) {
        self.mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Constructor Tests ====================

    #[test]
    fn test_battery_meter_new() {
        let meter = BatteryMeter::new();
        assert_eq!(meter.mode, MeterMode::Bar);
        assert_eq!(meter.percent, 0.0);
        assert_eq!(meter.ac_presence, ACPresence::Unknown);
        assert!(!meter.available);
    }

    #[test]
    fn test_battery_meter_default() {
        let meter = BatteryMeter::default();
        assert_eq!(meter.mode, MeterMode::Bar);
        assert!(!meter.available);
    }

    // ==================== ACPresence Tests ====================

    #[test]
    fn test_ac_presence_default() {
        let presence: ACPresence = ACPresence::default();
        assert_eq!(presence, ACPresence::Unknown);
    }

    #[test]
    fn test_ac_presence_variants() {
        assert_eq!(ACPresence::Unknown, ACPresence::Unknown);
        assert_eq!(ACPresence::Online, ACPresence::Online);
        assert_eq!(ACPresence::Offline, ACPresence::Offline);
        
        assert_ne!(ACPresence::Online, ACPresence::Offline);
        assert_ne!(ACPresence::Online, ACPresence::Unknown);
    }

    // ==================== format_battery Tests ====================

    #[test]
    fn test_format_battery_not_available() {
        let meter = BatteryMeter::new();
        assert_eq!(meter.format_battery(), "N/A");
    }

    #[test]
    fn test_format_battery_unknown_ac() {
        let mut meter = BatteryMeter::new();
        meter.available = true;
        meter.percent = 75.5;
        meter.ac_presence = ACPresence::Unknown;
        
        assert_eq!(meter.format_battery(), "75.5%");
    }

    #[test]
    fn test_format_battery_on_ac() {
        let mut meter = BatteryMeter::new();
        meter.available = true;
        meter.percent = 100.0;
        meter.ac_presence = ACPresence::Online;
        
        assert_eq!(meter.format_battery(), "100.0%; AC");
    }

    #[test]
    fn test_format_battery_on_battery() {
        let mut meter = BatteryMeter::new();
        meter.available = true;
        meter.percent = 45.3;
        meter.ac_presence = ACPresence::Offline;
        
        assert_eq!(meter.format_battery(), "45.3%; BAT");
    }

    #[test]
    fn test_format_battery_low() {
        let mut meter = BatteryMeter::new();
        meter.available = true;
        meter.percent = 5.0;
        meter.ac_presence = ACPresence::Offline;
        
        assert_eq!(meter.format_battery(), "5.0%; BAT");
    }

    #[test]
    fn test_format_battery_zero() {
        let mut meter = BatteryMeter::new();
        meter.available = true;
        meter.percent = 0.0;
        meter.ac_presence = ACPresence::Offline;
        
        assert_eq!(meter.format_battery(), "0.0%; BAT");
    }

    // ==================== Meter Trait Tests ====================

    #[test]
    fn test_battery_meter_name() {
        let meter = BatteryMeter::new();
        assert_eq!(meter.name(), "Battery");
    }

    #[test]
    fn test_battery_meter_caption() {
        let meter = BatteryMeter::new();
        assert_eq!(meter.caption(), "BAT");
    }

    #[test]
    fn test_battery_meter_mode() {
        let mut meter = BatteryMeter::new();
        assert_eq!(meter.mode(), MeterMode::Bar);
        
        meter.set_mode(MeterMode::Text);
        assert_eq!(meter.mode(), MeterMode::Text);
        
        meter.set_mode(MeterMode::Led);
        assert_eq!(meter.mode(), MeterMode::Led);
    }

    #[test]
    fn test_battery_meter_default_mode() {
        let meter = BatteryMeter::new();
        assert_eq!(meter.default_mode(), MeterMode::Bar);
    }
}
