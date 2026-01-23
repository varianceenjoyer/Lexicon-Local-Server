//! Custom TUI Widgets
//!
//! Reusable widget components for the diagnostic TUI.

mod level_meter;
mod device_dropdown;

pub use level_meter::LevelMeter;

// DeviceDropdown is reserved for future use in standalone dropdown widget
#[allow(unused_imports)]
pub use device_dropdown::DeviceDropdown;
