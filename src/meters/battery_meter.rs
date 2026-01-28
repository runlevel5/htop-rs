//! Battery Meter
//!
//! Displays battery percentage and charging status.
//!
//! Battery information is collected via the background scanner since it can be
//! expensive (especially on macOS where it spawns an external process).
//! The meter receives updates via `merge_expensive_data()`.

use super::meter_bg_scanner::{MeterDataId, MeterExpensiveData};
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
#[derive(Debug)]
pub struct BatteryMeter {
    mode: MeterMode,
    percent: f64,
    ac_presence: ACPresence,
    available: bool,
}

impl Default for BatteryMeter {
    fn default() -> Self {
        Self {
            mode: MeterMode::Bar,
            percent: 0.0,
            ac_presence: ACPresence::Unknown,
            available: false,
        }
    }
}

impl BatteryMeter {
    pub fn new() -> Self {
        BatteryMeter::default()
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
        // Battery data comes from background scanner via merge_expensive_data()
        // Nothing to do here - we just display whatever data we have
    }

    fn expensive_data_id(&self) -> Option<MeterDataId> {
        Some(MeterDataId::Battery)
    }

    fn merge_expensive_data(&mut self, data: &MeterExpensiveData) {
        // Use match to be future-proof when more variants are added
        match data {
            MeterExpensiveData::Battery {
                percent,
                ac_presence,
                available,
            } => {
                self.percent = *percent;
                self.ac_presence = *ac_presence;
                self.available = *available;
            }
        }
    }

    fn draw(
        &self,
        crt: &mut Crt,
        _machine: &Machine,
        _settings: &Settings,
        x: i32,
        y: i32,
        width: i32,
    ) {
        match self.mode {
            MeterMode::Bar => {
                if !self.available {
                    // Draw N/A in text mode
                    let caption_attr = crt.color(ColorElement::MeterText);
                    let value_attr = crt.color(ColorElement::MeterValueError);
                    crt.with_window(|win| {
                        let _ = win.mv(y, x);
                        let _ = win.attrset(caption_attr);
                        let _ = win.addstr("BAT");

                        let _ = win.attrset(value_attr);
                        let _ = win.addstr(" N/A");
                    });
                    return;
                }

                // Draw caption
                let caption_attr = crt.color(ColorElement::MeterText);
                crt.with_window(|win| {
                    let _ = win.mv(y, x);
                    let _ = win.attrset(caption_attr);
                    let _ = win.addstr("BAT");
                });

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
                    crt.with_window(|win| {
                        let _ = win.mv(y, text_x);
                        let _ = win.attrset(value_attr);
                        let _ = win.addstr(&text);
                    });
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
                let battery_text = self.format_battery();
                crt.with_window(|win| {
                    let _ = win.mv(y, x);
                    let _ = win.attrset(caption_attr);
                    let _ = win.addstr("Battery: ");

                    let _ = win.attrset(value_attr);
                    let _ = win.addstr(&battery_text);
                });
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

    // ==================== Background Scanner Integration Tests ====================

    #[test]
    fn test_battery_meter_expensive_data_id() {
        let meter = BatteryMeter::new();
        assert_eq!(meter.expensive_data_id(), Some(MeterDataId::Battery));
    }

    #[test]
    fn test_battery_meter_merge_expensive_data() {
        let mut meter = BatteryMeter::new();
        assert!(!meter.available);

        // Simulate background scanner providing data
        meter.merge_expensive_data(&MeterExpensiveData::Battery {
            percent: 75.5,
            ac_presence: ACPresence::Online,
            available: true,
        });

        assert!(meter.available);
        assert_eq!(meter.percent, 75.5);
        assert_eq!(meter.ac_presence, ACPresence::Online);
    }

    #[test]
    fn test_battery_meter_merge_unavailable() {
        let mut meter = BatteryMeter::new();

        // Simulate no battery found
        meter.merge_expensive_data(&MeterExpensiveData::Battery {
            percent: 0.0,
            ac_presence: ACPresence::Unknown,
            available: false,
        });

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
