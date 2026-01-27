//! Audio Device Manager for TUI
//!
//! Handles device enumeration, selection, and real-time level monitoring.
//!
//! NOTE: On Android, cpal/oboe requires an Android app context (JNI Activity)
//! that doesn't exist for standalone CLI binaries. The TUI gracefully degrades
//! with audio features disabled on Android.

#![allow(dead_code)] // Some fields/methods reserved for future use

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[cfg(not(target_os = "android"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::{Result, TuiError};

/// Gain multiplier for level meter display.
///
/// Scales RMS values so typical speech levels display prominently.
/// Typical speech RMS is 0.02-0.1, we want that to show as 0.2-0.8 on the meter.
const METER_GAIN: f32 = 8.0;

/// Information about an audio input device.
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    /// Device name for display.
    pub name: String,
    /// Unique device identifier.
    pub id: String,
    /// Whether this is the system default.
    pub is_default: bool,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u16,
}

/// Manages audio device enumeration and selection for the TUI.
///
/// On Android, audio functionality is disabled because cpal/oboe requires
/// an Android app context that doesn't exist for standalone CLI binaries.
pub struct AudioDeviceManager {
    /// Available input devices.
    devices: Vec<AudioDeviceInfo>,
    /// Currently selected device index.
    selected: usize,
    /// Current audio level (0.0 - 1.0), stored as u32 bits of f32.
    level: Arc<AtomicU32>,
    /// Level monitoring stream (not available on Android).
    #[cfg(not(target_os = "android"))]
    level_stream: Option<cpal::Stream>,
    /// Host handle (not available on Android).
    #[cfg(not(target_os = "android"))]
    _host: cpal::Host,
}

impl AudioDeviceManager {
    /// Creates a new audio device manager.
    ///
    /// Enumerates available devices and selects the default.
    ///
    /// On Android, this returns an error because cpal/oboe requires an Android
    /// app context (JNI Activity) that doesn't exist for standalone CLI binaries.
    #[cfg(target_os = "android")]
    pub fn new() -> Result<Self> {
        Err(TuiError::Audio(
            "Audio unavailable on Android (requires app context)".to_string(),
        ))
    }

    /// Creates a new audio device manager.
    ///
    /// Enumerates available devices and selects the default.
    #[cfg(not(target_os = "android"))]
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let devices = Self::enumerate_devices_internal(&host)?;
        let default_idx = devices.iter().position(|d| d.is_default).unwrap_or(0);

        let mut manager = Self {
            devices,
            selected: default_idx,
            level: Arc::new(AtomicU32::new(0)),
            level_stream: None,
            _host: host,
        };

        // Start level monitoring on default device
        if !manager.devices.is_empty() {
            let _ = manager.start_level_monitoring();
        }

        Ok(manager)
    }

    /// Enumerates all available input devices.
    #[cfg(not(target_os = "android"))]
    fn enumerate_devices_internal(host: &cpal::Host) -> Result<Vec<AudioDeviceInfo>> {
        let default_name = host.default_input_device().and_then(|d| d.name().ok());

        let mut devices = Vec::new();

        let input_devices = host
            .input_devices()
            .map_err(|e| TuiError::Audio(format!("Failed to enumerate devices: {}", e)))?;

        for device in input_devices {
            if let Ok(name) = device.name() {
                let (sample_rate, channels) = device
                    .default_input_config()
                    .map(|c| (c.sample_rate().0, c.channels()))
                    .unwrap_or((48000, 1));

                devices.push(AudioDeviceInfo {
                    id: name.clone(),
                    is_default: Some(&name) == default_name.as_ref(),
                    name,
                    sample_rate,
                    channels,
                });
            }
        }

        Ok(devices)
    }

    /// Returns the list of available devices.
    pub fn devices(&self) -> &[AudioDeviceInfo] {
        &self.devices
    }

    /// Returns the number of devices.
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Returns the currently selected device index.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Returns the currently selected device info.
    pub fn selected_device(&self) -> Option<&AudioDeviceInfo> {
        self.devices.get(self.selected)
    }

    /// Selects a device by index.
    #[cfg(target_os = "android")]
    pub fn select_device(&mut self, _index: usize) -> Result<()> {
        Err(TuiError::Audio(
            "Audio unavailable on Android".to_string(),
        ))
    }

    /// Selects a device by index.
    #[cfg(not(target_os = "android"))]
    pub fn select_device(&mut self, index: usize) -> Result<()> {
        if index >= self.devices.len() {
            return Err(TuiError::Audio("Invalid device index".to_string()));
        }

        if index != self.selected {
            self.selected = index;
            // Restart level monitoring on new device
            self.stop_level_monitoring();
            self.start_level_monitoring()?;
        }

        Ok(())
    }

    /// Gets the current audio level (0.0 - 1.0).
    pub fn get_level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// Starts level monitoring on the selected device.
    #[cfg(not(target_os = "android"))]
    fn start_level_monitoring(&mut self) -> Result<()> {
        let host = cpal::default_host();

        let device = if let Some(info) = self.selected_device() {
            host.input_devices()
                .map_err(|e| TuiError::Audio(format!("Device enumeration failed: {}", e)))?
                .find(|d| d.name().map(|n| n == info.name).unwrap_or(false))
                .ok_or_else(|| TuiError::Audio("Selected device not found".to_string()))?
        } else {
            return Err(TuiError::Audio("No device selected".to_string()));
        };

        let config = device
            .default_input_config()
            .map_err(|e| TuiError::Audio(format!("Failed to get device config: {}", e)))?;

        let level = self.level.clone();
        let channels = config.channels() as usize;

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Calculate RMS level
                    if data.is_empty() {
                        return;
                    }

                    // Convert to mono and calculate RMS
                    let mono_sum: f32 = data
                        .chunks(channels)
                        .map(|chunk| {
                            let sample = chunk.iter().sum::<f32>() / channels as f32;
                            sample * sample
                        })
                        .sum();

                    let sample_count = data.len() / channels;
                    let rms = (mono_sum / sample_count as f32).sqrt();

                    // Apply gain to make meter more responsive to typical speech levels
                    let scaled_rms = rms * METER_GAIN;

                    // Apply smoothing (exponential moving average)
                    let current = f32::from_bits(level.load(Ordering::Relaxed));
                    let alpha = 0.3; // Smoothing factor
                    let smoothed = current * (1.0 - alpha) + scaled_rms * alpha;

                    // Clamp and store
                    let clamped = smoothed.clamp(0.0, 1.0);
                    level.store(clamped.to_bits(), Ordering::Relaxed);
                },
                |err| {
                    tracing::error!("Level monitoring error: {}", err);
                },
                None,
            )
            .map_err(|e| TuiError::Audio(format!("Failed to build stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| TuiError::Audio(format!("Failed to start stream: {}", e)))?;

        self.level_stream = Some(stream);
        Ok(())
    }

    /// Stops level monitoring.
    #[cfg(not(target_os = "android"))]
    fn stop_level_monitoring(&mut self) {
        self.level_stream = None;
        self.level.store(0_f32.to_bits(), Ordering::Relaxed);
    }

    /// Refreshes the device list.
    #[cfg(target_os = "android")]
    pub fn refresh_devices(&mut self) -> Result<()> {
        Err(TuiError::Audio(
            "Audio unavailable on Android".to_string(),
        ))
    }

    /// Refreshes the device list.
    #[cfg(not(target_os = "android"))]
    pub fn refresh_devices(&mut self) -> Result<()> {
        let host = cpal::default_host();
        self.devices = Self::enumerate_devices_internal(&host)?;

        // Ensure selected index is still valid
        if self.selected >= self.devices.len() {
            self.selected = self.devices.iter().position(|d| d.is_default).unwrap_or(0);
        }

        // Restart level monitoring
        self.stop_level_monitoring();
        if !self.devices.is_empty() {
            self.start_level_monitoring()?;
        }

        Ok(())
    }
}

#[cfg(not(target_os = "android"))]
impl Drop for AudioDeviceManager {
    fn drop(&mut self) {
        self.stop_level_monitoring();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_device_info() {
        let info = AudioDeviceInfo {
            name: "Test Device".to_string(),
            id: "test-device".to_string(),
            is_default: true,
            sample_rate: 48000,
            channels: 2,
        };

        assert_eq!(info.name, "Test Device");
        assert!(info.is_default);
    }

    #[test]
    fn test_audio_device_manager_creation() {
        // This test may fail in CI without audio devices
        let result = AudioDeviceManager::new();
        match result {
            Ok(manager) => {
                assert!(manager.device_count() == 0 || manager.selected_device().is_some());
            }
            Err(_) => {
                // Expected in CI/headless environments
            }
        }
    }
}
