//! Platform detection and capabilities
//!
//! This module provides platform detection and capability querying for
//! cross-platform compatibility.

use serde::{Deserialize, Serialize};

/// Supported platforms for Lexicon deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// macOS on Apple Silicon (ARM64)
    MacOsArm64,
    /// macOS on Intel (x86_64)
    MacOsX86,
    /// Windows on x86_64
    Windows,
    /// Linux on x86_64
    LinuxX86,
    /// Linux on ARM64
    LinuxArm64,
    /// iOS (ARM64)
    Ios,
    /// iOS Simulator (ARM64 or x86_64)
    IosSimulator,
    /// Android (ARM64)
    AndroidArm64,
    /// Android (ARM32)
    AndroidArm32,
    /// Unknown/unsupported platform
    Unknown,
}

impl Platform {
    /// Detects the current platform at runtime.
    #[must_use]
    pub fn detect() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Platform::MacOsArm64
        }

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            Platform::MacOsX86
        }

        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            Platform::LinuxX86
        }

        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            Platform::LinuxArm64
        }

        #[cfg(target_os = "ios")]
        {
            // Distinguish simulator from device
            #[cfg(target_os = "ios")]
            if cfg!(target_arch = "x86_64") || std::env::var("SIMULATOR_DEVICE_NAME").is_ok() {
                Platform::IosSimulator
            } else {
                Platform::Ios
            }
        }

        #[cfg(all(target_os = "android", target_arch = "aarch64"))]
        {
            Platform::AndroidArm64
        }

        #[cfg(all(target_os = "android", target_arch = "arm"))]
        {
            Platform::AndroidArm32
        }

        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "ios",
            target_os = "android"
        )))]
        {
            Platform::Unknown
        }
    }

    /// Returns whether this platform is a desktop platform.
    #[must_use]
    pub fn is_desktop(&self) -> bool {
        matches!(
            self,
            Platform::MacOsArm64
                | Platform::MacOsX86
                | Platform::Windows
                | Platform::LinuxX86
                | Platform::LinuxArm64
        )
    }

    /// Returns whether this platform is a mobile platform.
    #[must_use]
    pub fn is_mobile(&self) -> bool {
        matches!(
            self,
            Platform::Ios | Platform::IosSimulator | Platform::AndroidArm64 | Platform::AndroidArm32
        )
    }

    /// Returns whether this platform is iOS (device or simulator).
    #[must_use]
    pub fn is_ios(&self) -> bool {
        matches!(self, Platform::Ios | Platform::IosSimulator)
    }

    /// Returns whether this platform is Android.
    #[must_use]
    pub fn is_android(&self) -> bool {
        matches!(self, Platform::AndroidArm64 | Platform::AndroidArm32)
    }

    /// Returns whether this platform supports Metal GPU acceleration.
    #[must_use]
    pub fn supports_metal(&self) -> bool {
        matches!(
            self,
            Platform::MacOsArm64 | Platform::MacOsX86 | Platform::Ios | Platform::IosSimulator
        )
    }

    /// Returns whether this platform supports CUDA GPU acceleration.
    #[must_use]
    pub fn supports_cuda(&self) -> bool {
        // CUDA is available on desktop Linux and Windows with NVIDIA GPUs
        // Actual GPU presence should be checked at runtime
        matches!(
            self,
            Platform::LinuxX86 | Platform::LinuxArm64 | Platform::Windows
        )
    }

    /// Returns whether this platform supports Vulkan GPU acceleration.
    #[must_use]
    pub fn supports_vulkan(&self) -> bool {
        matches!(
            self,
            Platform::LinuxX86
                | Platform::LinuxArm64
                | Platform::Windows
                | Platform::AndroidArm64
                | Platform::AndroidArm32
        )
    }

    /// Returns the platform name as a string.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::MacOsArm64 => "macos-arm64",
            Platform::MacOsX86 => "macos-x86_64",
            Platform::Windows => "windows-x86_64",
            Platform::LinuxX86 => "linux-x86_64",
            Platform::LinuxArm64 => "linux-arm64",
            Platform::Ios => "ios-arm64",
            Platform::IosSimulator => "ios-simulator",
            Platform::AndroidArm64 => "android-arm64",
            Platform::AndroidArm32 => "android-arm32",
            Platform::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Device capability profile for model selection.
///
/// This struct captures device characteristics that influence
/// which models are appropriate for the device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// Platform identifier
    pub platform: Platform,
    /// Available system memory in megabytes
    pub available_memory_mb: u64,
    /// Whether the device has GPU acceleration available
    pub has_gpu: bool,
    /// Thermal headroom (0.0-1.0), if available
    /// iOS: from ProcessInfo.thermalState
    /// Android: from PowerManager.getThermalHeadroom()
    pub thermal_headroom: Option<f32>,
}

impl DeviceProfile {
    /// Creates a new device profile with the given parameters.
    #[must_use]
    pub fn new(
        platform: Platform,
        available_memory_mb: u64,
        has_gpu: bool,
        thermal_headroom: Option<f32>,
    ) -> Self {
        Self {
            platform,
            available_memory_mb,
            has_gpu,
            thermal_headroom,
        }
    }

    /// Profiles the current device.
    ///
    /// Note: Actual memory detection and GPU detection require platform-specific
    /// implementations. This method provides reasonable defaults.
    #[must_use]
    pub fn detect() -> Self {
        let platform = Platform::detect();

        // Default memory estimates based on platform
        // Real implementation should use sysinfo or platform APIs
        let available_memory_mb = match platform {
            Platform::Ios | Platform::IosSimulator => 2048,       // Conservative iOS estimate
            Platform::AndroidArm64 | Platform::AndroidArm32 => 2048, // Conservative Android estimate
            _ => 8192,                                            // Desktop default
        };

        // GPU availability (conservative defaults)
        let has_gpu = platform.supports_metal() || platform.supports_cuda();

        Self {
            platform,
            available_memory_mb,
            has_gpu,
            thermal_headroom: None,
        }
    }

    /// Returns the recommended model tier for this device.
    #[must_use]
    pub fn recommended_tier(&self) -> ModelTier {
        if self.platform.is_mobile() {
            if self.available_memory_mb < 2048 {
                ModelTier::MobileLow
            } else {
                ModelTier::MobileMid
            }
        } else if self.available_memory_mb >= 16384 && self.has_gpu {
            ModelTier::Server
        } else {
            ModelTier::Desktop
        }
    }
}

/// Model tier for device capability matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelTier {
    /// Low-end mobile devices (<2GB RAM): tiny model only
    MobileLow,
    /// Mid-range mobile devices (2-4GB RAM): tiny + base models
    MobileMid,
    /// Desktop devices: base + small models
    Desktop,
    /// Server/high-end devices: all models including medium/large
    Server,
}

impl ModelTier {
    /// Returns the tier name as a string.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelTier::MobileLow => "mobile-low",
            ModelTier::MobileMid => "mobile-mid",
            ModelTier::Desktop => "desktop",
            ModelTier::Server => "server",
        }
    }
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Thermal state enumeration matching iOS ProcessInfo.ThermalState.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThermalState {
    /// System is in a normal thermal state
    Nominal,
    /// System is slightly elevated but still manageable
    Fair,
    /// System is warm; consider reducing workload
    Serious,
    /// System is critically hot; must reduce workload
    Critical,
}

impl ThermalState {
    /// Converts thermal headroom (0.0-1.0) to thermal state.
    ///
    /// Thresholds based on typical thermal management:
    /// - Nominal: headroom >= 0.7
    /// - Fair: headroom >= 0.5
    /// - Serious: headroom >= 0.3
    /// - Critical: headroom < 0.3
    #[must_use]
    pub fn from_headroom(headroom: f32) -> Self {
        if headroom >= 0.7 {
            ThermalState::Nominal
        } else if headroom >= 0.5 {
            ThermalState::Fair
        } else if headroom >= 0.3 {
            ThermalState::Serious
        } else {
            ThermalState::Critical
        }
    }

    /// Returns whether the system should consider reducing workload.
    #[must_use]
    pub fn should_throttle(&self) -> bool {
        matches!(self, ThermalState::Serious | ThermalState::Critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detect() {
        let platform = Platform::detect();
        // Should not be unknown on any supported platform
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "ios",
            target_os = "android"
        ))]
        {
            assert_ne!(platform, Platform::Unknown);
        }
    }

    #[test]
    fn test_platform_categories() {
        assert!(Platform::MacOsArm64.is_desktop());
        assert!(Platform::Windows.is_desktop());
        assert!(!Platform::Ios.is_desktop());

        assert!(Platform::Ios.is_mobile());
        assert!(Platform::AndroidArm64.is_mobile());
        assert!(!Platform::LinuxX86.is_mobile());
    }

    #[test]
    fn test_platform_gpu_support() {
        assert!(Platform::MacOsArm64.supports_metal());
        assert!(Platform::Ios.supports_metal());
        assert!(!Platform::LinuxX86.supports_metal());

        assert!(Platform::LinuxX86.supports_cuda());
        assert!(Platform::Windows.supports_cuda());
        assert!(!Platform::Ios.supports_cuda());
    }

    #[test]
    fn test_device_profile_tier() {
        let low_mobile = DeviceProfile::new(Platform::AndroidArm64, 1500, false, None);
        assert_eq!(low_mobile.recommended_tier(), ModelTier::MobileLow);

        let mid_mobile = DeviceProfile::new(Platform::Ios, 4000, true, None);
        assert_eq!(mid_mobile.recommended_tier(), ModelTier::MobileMid);

        let desktop = DeviceProfile::new(Platform::LinuxX86, 8000, true, None);
        assert_eq!(desktop.recommended_tier(), ModelTier::Desktop);

        let server = DeviceProfile::new(Platform::LinuxX86, 32000, true, None);
        assert_eq!(server.recommended_tier(), ModelTier::Server);
    }

    #[test]
    fn test_thermal_state() {
        assert_eq!(ThermalState::from_headroom(0.8), ThermalState::Nominal);
        assert_eq!(ThermalState::from_headroom(0.6), ThermalState::Fair);
        assert_eq!(ThermalState::from_headroom(0.4), ThermalState::Serious);
        assert_eq!(ThermalState::from_headroom(0.1), ThermalState::Critical);

        assert!(!ThermalState::Nominal.should_throttle());
        assert!(!ThermalState::Fair.should_throttle());
        assert!(ThermalState::Serious.should_throttle());
        assert!(ThermalState::Critical.should_throttle());
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(Platform::MacOsArm64.to_string(), "macos-arm64");
        assert_eq!(Platform::Windows.to_string(), "windows-x86_64");
    }
}
