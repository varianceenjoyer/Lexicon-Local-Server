//! Bluetooth HFP 1.6 Audio Support
//!
//! This module provides Bluetooth audio management with HFP 1.6 codec negotiation
//! for high-quality 16kHz wideband audio from Bluetooth headsets.
//!
//! # Architecture
//!
//! ```text
//! BluetoothAudioManager
//!   │
//!   ├── Platform-specific implementation
//!   │     ├── macOS: AVAudioSession/CoreBluetooth
//!   │     ├── Windows: MMDevice API
//!   │     └── Linux: BlueZ/PulseAudio
//!   │
//!   └── State callbacks → AudioSource route change handling
//! ```
//!
//! # HFP Profiles
//!
//! | Profile | Sample Rate | Quality | Notes |
//! |---------|-------------|---------|-------|
//! | HFP 1.5 | 8 kHz | Narrowband | CVSD codec, legacy |
//! | HFP 1.6+ | 16 kHz | Wideband | mSBC codec, preferred |
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::audio::bluetooth::{BluetoothAudioManager, BluetoothState};
//!
//! // Create Bluetooth manager
//! let manager = BluetoothAudioManager::new()?;
//!
//! // Check current state
//! if let Some(info) = manager.get_audio_info() {
//!     match info.state {
//!         BluetoothState::ConnectedWideband => {
//!             println!("Using 16kHz wideband: {}", info.device_name);
//!         }
//!         BluetoothState::ConnectedNarrowband => {
//!             println!("Using 8kHz narrowband (will resample): {}", info.device_name);
//!         }
//!         _ => {}
//!     }
//! }
//! # Ok::<(), lexicon_core::error::LexiconError>(())
//! ```

use crate::error::{AudioError, LexiconError, Result};
use std::sync::{Arc, RwLock};

/// Bluetooth audio connection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothState {
    /// No Bluetooth audio device connected
    Disconnected,
    /// Connected but negotiating codec
    Connecting,
    /// Connected with narrowband (8kHz) - resampling needed for 16kHz Whisper
    ConnectedNarrowband,
    /// Connected with wideband (16kHz) - optimal quality for Whisper
    ConnectedWideband,
    /// Connection failed with error message
    Failed(String),
}

impl BluetoothState {
    /// Returns true if connected (either narrowband or wideband).
    #[must_use]
    pub fn is_connected(&self) -> bool {
        matches!(
            self,
            BluetoothState::ConnectedNarrowband | BluetoothState::ConnectedWideband
        )
    }

    /// Returns true if using optimal wideband (16kHz) connection.
    #[must_use]
    pub fn is_wideband(&self) -> bool {
        matches!(self, BluetoothState::ConnectedWideband)
    }

    /// Returns the sample rate for this state.
    #[must_use]
    pub fn sample_rate(&self) -> Option<u32> {
        match self {
            BluetoothState::ConnectedNarrowband => Some(8000),
            BluetoothState::ConnectedWideband => Some(16000),
            _ => None,
        }
    }
}

impl std::fmt::Display for BluetoothState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BluetoothState::Disconnected => write!(f, "Disconnected"),
            BluetoothState::Connecting => write!(f, "Connecting"),
            BluetoothState::ConnectedNarrowband => write!(f, "Connected (8kHz narrowband)"),
            BluetoothState::ConnectedWideband => write!(f, "Connected (16kHz wideband)"),
            BluetoothState::Failed(msg) => write!(f, "Failed: {}", msg),
        }
    }
}

/// Bluetooth audio route information.
#[derive(Debug, Clone)]
pub struct BluetoothAudioInfo {
    /// Device name (e.g., "AirPods Pro")
    pub device_name: String,
    /// Current sample rate (8000 or 16000)
    pub sample_rate: u32,
    /// Whether HFP 1.6+ wideband is supported by the device
    pub hfp_wideband_supported: bool,
    /// Connection state
    pub state: BluetoothState,
}

impl BluetoothAudioInfo {
    /// Creates a new Bluetooth audio info.
    pub fn new(device_name: impl Into<String>, state: BluetoothState) -> Self {
        let device_name = device_name.into();
        let sample_rate = state.sample_rate().unwrap_or(0);
        let hfp_wideband_supported = state.is_wideband();

        Self {
            device_name,
            sample_rate,
            hfp_wideband_supported,
            state,
        }
    }

    /// Returns true if resampling is needed (8kHz -> 16kHz).
    #[must_use]
    pub fn needs_resampling(&self) -> bool {
        self.sample_rate == 8000
    }
}

/// Audio route types.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioRoute {
    /// Built-in device microphone
    BuiltIn,
    /// USB or external wired microphone
    External,
    /// Bluetooth headset with HFP profile
    Bluetooth(BluetoothAudioInfo),
}

impl AudioRoute {
    /// Returns the sample rate for this route.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        match self {
            AudioRoute::BuiltIn | AudioRoute::External => {
                // Built-in and external typically support 44.1kHz or 48kHz
                // The audio capture layer handles resampling to 16kHz
                48000
            }
            AudioRoute::Bluetooth(info) => info.sample_rate,
        }
    }

    /// Returns true if this is a Bluetooth route.
    #[must_use]
    pub fn is_bluetooth(&self) -> bool {
        matches!(self, AudioRoute::Bluetooth(_))
    }

    /// Returns the device name for this route.
    #[must_use]
    pub fn device_name(&self) -> &str {
        match self {
            AudioRoute::BuiltIn => "Built-in Microphone",
            AudioRoute::External => "External Microphone",
            AudioRoute::Bluetooth(info) => &info.device_name,
        }
    }
}

impl PartialEq for BluetoothAudioInfo {
    fn eq(&self, other: &Self) -> bool {
        self.device_name == other.device_name && self.state == other.state
    }
}

/// Callback trait for Bluetooth state changes.
///
/// Implement this trait to receive notifications when Bluetooth audio state changes.
pub trait BluetoothStateCallback: Send + Sync {
    /// Called when Bluetooth audio state changes.
    ///
    /// This may be called from a system thread, so implementations should be fast
    /// and non-blocking. Consider using a channel to forward events to your main thread.
    fn on_state_change(&self, info: BluetoothAudioInfo);

    /// Called when the audio route changes (BT connect/disconnect).
    ///
    /// Use this to rebuild audio streams when switching between Bluetooth and built-in.
    fn on_route_change(&self, new_route: AudioRoute);
}

/// Bluetooth audio manager for HFP 1.6 codec negotiation.
///
/// This manager monitors Bluetooth audio connections and negotiates for the best
/// available codec (preferring 16kHz wideband over 8kHz narrowband).
///
/// # Platform Support
///
/// | Platform | Implementation | Status |
/// |----------|----------------|--------|
/// | macOS | AVAudioSession | Planned |
/// | Windows | MMDevice API | Planned |
/// | Linux | BlueZ/PulseAudio | Planned |
pub struct BluetoothAudioManager {
    state: Arc<RwLock<BluetoothState>>,
    current_route: Arc<RwLock<AudioRoute>>,
    callback: Option<Arc<dyn BluetoothStateCallback>>,
}

impl BluetoothAudioManager {
    /// Creates a new Bluetooth audio manager.
    ///
    /// # Errors
    ///
    /// Returns an error if platform Bluetooth audio APIs are unavailable.
    pub fn new() -> Result<Self> {
        // Platform-specific initialization will go here
        Ok(Self {
            state: Arc::new(RwLock::new(BluetoothState::Disconnected)),
            current_route: Arc::new(RwLock::new(AudioRoute::BuiltIn)),
            callback: None,
        })
    }

    /// Sets a callback for state changes.
    pub fn set_callback(&mut self, callback: impl BluetoothStateCallback + 'static) {
        self.callback = Some(Arc::new(callback));
    }

    /// Gets the current Bluetooth audio info, if connected.
    #[must_use]
    pub fn get_audio_info(&self) -> Option<BluetoothAudioInfo> {
        let state = self.state.read().ok()?;
        let route = self.current_route.read().ok()?;

        match &*route {
            AudioRoute::Bluetooth(info) => Some(info.clone()),
            _ if state.is_connected() => {
                // Connected but route not updated yet
                Some(BluetoothAudioInfo::new("Unknown Device", state.clone()))
            }
            _ => None,
        }
    }

    /// Requests HFP wideband mode if available.
    ///
    /// This attempts to negotiate 16kHz wideband audio with the connected device.
    /// Not all devices support wideband mode.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if wideband was successfully negotiated
    /// - `Ok(false)` if the device only supports narrowband
    /// - `Err(_)` if no Bluetooth device is connected or negotiation failed
    pub fn request_wideband(&self) -> Result<bool> {
        let state = self.state.read().map_err(|_| {
            LexiconError::Audio(AudioError::DeviceEnumerationFailed {
                message: "Failed to acquire state lock".to_string(),
            })
        })?;

        if !state.is_connected() {
            return Err(LexiconError::Audio(AudioError::NoInputDevice));
        }

        // Platform-specific wideband negotiation will go here
        // For now, return current state
        Ok(state.is_wideband())
    }

    /// Returns true if currently using Bluetooth audio.
    #[must_use]
    pub fn is_bluetooth_active(&self) -> bool {
        self.current_route
            .read()
            .map(|r| r.is_bluetooth())
            .unwrap_or(false)
    }

    /// Gets the current audio route.
    #[must_use]
    pub fn current_route(&self) -> AudioRoute {
        self.current_route
            .read()
            .map(|r| r.clone())
            .unwrap_or(AudioRoute::BuiltIn)
    }

    /// Gets the current Bluetooth state.
    #[must_use]
    pub fn state(&self) -> BluetoothState {
        self.state
            .read()
            .map(|s| s.clone())
            .unwrap_or(BluetoothState::Disconnected)
    }

    /// Simulates a Bluetooth connection (for testing).
    #[cfg(test)]
    pub fn simulate_connect(&self, device_name: &str, wideband: bool) {
        let new_state = if wideband {
            BluetoothState::ConnectedWideband
        } else {
            BluetoothState::ConnectedNarrowband
        };

        if let Ok(mut state) = self.state.write() {
            *state = new_state.clone();
        }

        let info = BluetoothAudioInfo::new(device_name, new_state);

        if let Ok(mut route) = self.current_route.write() {
            *route = AudioRoute::Bluetooth(info.clone());
        }

        if let Some(ref callback) = self.callback {
            callback.on_state_change(info.clone());
            callback.on_route_change(AudioRoute::Bluetooth(info));
        }
    }

    /// Simulates a Bluetooth disconnection (for testing).
    #[cfg(test)]
    pub fn simulate_disconnect(&self) {
        if let Ok(mut state) = self.state.write() {
            *state = BluetoothState::Disconnected;
        }

        if let Ok(mut route) = self.current_route.write() {
            *route = AudioRoute::BuiltIn;
        }

        if let Some(ref callback) = self.callback {
            callback.on_state_change(BluetoothAudioInfo::new(
                "",
                BluetoothState::Disconnected,
            ));
            callback.on_route_change(AudioRoute::BuiltIn);
        }
    }
}

impl std::fmt::Debug for BluetoothAudioManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BluetoothAudioManager")
            .field("state", &self.state())
            .field("current_route", &self.current_route())
            .field("has_callback", &self.callback.is_some())
            .finish()
    }
}

impl Default for BluetoothAudioManager {
    fn default() -> Self {
        Self::new().expect("Failed to create BluetoothAudioManager")
    }
}

// Platform-specific implementations will be added in submodules:
// - macos.rs: AVAudioSession/CoreBluetooth
// - windows.rs: MMDevice API
// - linux.rs: BlueZ/PulseAudio

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    #[test]
    fn test_bluetooth_state_is_connected() {
        assert!(!BluetoothState::Disconnected.is_connected());
        assert!(!BluetoothState::Connecting.is_connected());
        assert!(BluetoothState::ConnectedNarrowband.is_connected());
        assert!(BluetoothState::ConnectedWideband.is_connected());
        assert!(!BluetoothState::Failed("test".to_string()).is_connected());
    }

    #[test]
    fn test_bluetooth_state_is_wideband() {
        assert!(!BluetoothState::Disconnected.is_wideband());
        assert!(!BluetoothState::ConnectedNarrowband.is_wideband());
        assert!(BluetoothState::ConnectedWideband.is_wideband());
    }

    #[test]
    fn test_bluetooth_state_sample_rate() {
        assert_eq!(BluetoothState::Disconnected.sample_rate(), None);
        assert_eq!(BluetoothState::ConnectedNarrowband.sample_rate(), Some(8000));
        assert_eq!(BluetoothState::ConnectedWideband.sample_rate(), Some(16000));
    }

    #[test]
    fn test_bluetooth_state_display() {
        assert_eq!(
            format!("{}", BluetoothState::Disconnected),
            "Disconnected"
        );
        assert_eq!(
            format!("{}", BluetoothState::ConnectedWideband),
            "Connected (16kHz wideband)"
        );
        assert_eq!(
            format!("{}", BluetoothState::ConnectedNarrowband),
            "Connected (8kHz narrowband)"
        );
    }

    #[test]
    fn test_bluetooth_audio_info_new() {
        let info = BluetoothAudioInfo::new("AirPods Pro", BluetoothState::ConnectedWideband);

        assert_eq!(info.device_name, "AirPods Pro");
        assert_eq!(info.sample_rate, 16000);
        assert!(info.hfp_wideband_supported);
        assert!(!info.needs_resampling());
    }

    #[test]
    fn test_bluetooth_audio_info_narrowband() {
        let info = BluetoothAudioInfo::new("Old Headset", BluetoothState::ConnectedNarrowband);

        assert_eq!(info.sample_rate, 8000);
        assert!(!info.hfp_wideband_supported);
        assert!(info.needs_resampling());
    }

    #[test]
    fn test_audio_route_sample_rate() {
        assert_eq!(AudioRoute::BuiltIn.sample_rate(), 48000);
        assert_eq!(AudioRoute::External.sample_rate(), 48000);

        let info = BluetoothAudioInfo::new("Test", BluetoothState::ConnectedWideband);
        assert_eq!(AudioRoute::Bluetooth(info).sample_rate(), 16000);
    }

    #[test]
    fn test_audio_route_is_bluetooth() {
        assert!(!AudioRoute::BuiltIn.is_bluetooth());
        assert!(!AudioRoute::External.is_bluetooth());

        let info = BluetoothAudioInfo::new("Test", BluetoothState::ConnectedWideband);
        assert!(AudioRoute::Bluetooth(info).is_bluetooth());
    }

    #[test]
    fn test_audio_route_device_name() {
        assert_eq!(AudioRoute::BuiltIn.device_name(), "Built-in Microphone");
        assert_eq!(AudioRoute::External.device_name(), "External Microphone");

        let info = BluetoothAudioInfo::new("My Headphones", BluetoothState::ConnectedWideband);
        assert_eq!(AudioRoute::Bluetooth(info).device_name(), "My Headphones");
    }

    #[test]
    fn test_bluetooth_manager_new() {
        let manager = BluetoothAudioManager::new().expect("Should create manager");
        assert_eq!(manager.state(), BluetoothState::Disconnected);
        assert_eq!(manager.current_route(), AudioRoute::BuiltIn);
        assert!(!manager.is_bluetooth_active());
    }

    #[test]
    fn test_bluetooth_manager_simulate_connect() {
        let manager = BluetoothAudioManager::new().expect("Should create manager");

        // Simulate wideband connection
        manager.simulate_connect("Test Headset", true);

        assert!(manager.is_bluetooth_active());
        assert_eq!(manager.state(), BluetoothState::ConnectedWideband);

        let info = manager.get_audio_info().expect("Should have info");
        assert_eq!(info.device_name, "Test Headset");
        assert_eq!(info.sample_rate, 16000);
    }

    #[test]
    fn test_bluetooth_manager_simulate_disconnect() {
        let manager = BluetoothAudioManager::new().expect("Should create manager");

        manager.simulate_connect("Test Headset", true);
        assert!(manager.is_bluetooth_active());

        manager.simulate_disconnect();
        assert!(!manager.is_bluetooth_active());
        assert_eq!(manager.state(), BluetoothState::Disconnected);
        assert!(manager.get_audio_info().is_none());
    }

    #[test]
    fn test_bluetooth_manager_callback() {
        struct TestCallback {
            state_changed: Arc<AtomicBool>,
            route_changed: Arc<AtomicBool>,
            last_sample_rate: Arc<AtomicU32>,
        }

        impl BluetoothStateCallback for TestCallback {
            fn on_state_change(&self, info: BluetoothAudioInfo) {
                self.state_changed.store(true, Ordering::SeqCst);
                self.last_sample_rate.store(info.sample_rate, Ordering::SeqCst);
            }

            fn on_route_change(&self, _new_route: AudioRoute) {
                self.route_changed.store(true, Ordering::SeqCst);
            }
        }

        let state_changed = Arc::new(AtomicBool::new(false));
        let route_changed = Arc::new(AtomicBool::new(false));
        let last_sample_rate = Arc::new(AtomicU32::new(0));

        let callback = TestCallback {
            state_changed: state_changed.clone(),
            route_changed: route_changed.clone(),
            last_sample_rate: last_sample_rate.clone(),
        };

        let mut manager = BluetoothAudioManager::new().expect("Should create manager");
        manager.set_callback(callback);

        manager.simulate_connect("Test", true);

        assert!(state_changed.load(Ordering::SeqCst));
        assert!(route_changed.load(Ordering::SeqCst));
        assert_eq!(last_sample_rate.load(Ordering::SeqCst), 16000);
    }

    #[test]
    fn test_bluetooth_manager_request_wideband_not_connected() {
        let manager = BluetoothAudioManager::new().expect("Should create manager");

        // Should error when not connected
        assert!(manager.request_wideband().is_err());
    }

    #[test]
    fn test_bluetooth_manager_request_wideband_connected() {
        let manager = BluetoothAudioManager::new().expect("Should create manager");

        manager.simulate_connect("Test", true);
        assert!(manager.request_wideband().unwrap());

        // Reconnect as narrowband
        manager.simulate_connect("Test", false);
        assert!(!manager.request_wideband().unwrap());
    }

    #[test]
    fn test_bluetooth_manager_debug() {
        let manager = BluetoothAudioManager::new().expect("Should create manager");
        let debug = format!("{:?}", manager);

        assert!(debug.contains("BluetoothAudioManager"));
        assert!(debug.contains("Disconnected"));
    }
}
