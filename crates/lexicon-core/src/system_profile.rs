//! System Profiling for Model Selection
//!
//! This module provides hardware detection and system tier classification
//! to recommend appropriate transcription models based on device capabilities.
//!
//! # Features
//!
//! Requires the `system-profile` feature to be enabled.
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::system_profile::{SystemProfiler, SystemTier};
//!
//! let profile = SystemProfiler::profile();
//! println!("System tier: {:?}", profile.tier);
//! println!("Recommended model: {}", profile.tier.recommended_model());
//! ```

use crate::error::{LexiconError, Result};
use crate::platform::Platform;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// System tier classification based on hardware capabilities.
///
/// Used to determine appropriate model recommendations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemTier {
    /// <4GB RAM, no GPU - tiny.en only
    Minimal,
    /// 4-6GB RAM, no GPU - base.en max
    Low,
    /// 6-8GB RAM, integrated GPU - small.en max
    Medium,
    /// 8-16GB RAM, dedicated GPU - medium.en max
    High,
    /// 16GB+ RAM, high-end GPU - large-v3 viable
    Ultra,
}

impl SystemTier {
    /// Returns the recommended model ID for this tier.
    #[must_use]
    pub fn recommended_model(&self) -> &'static str {
        match self {
            SystemTier::Minimal => "whisper-tiny-en",
            SystemTier::Low => "whisper-base-en",
            SystemTier::Medium => "whisper-small-en",
            SystemTier::High => "whisper-medium-en",
            SystemTier::Ultra => "whisper-large-v3",
        }
    }

    /// Returns the fallback model ID for this tier.
    #[must_use]
    pub fn fallback_model(&self) -> Option<&'static str> {
        match self {
            SystemTier::Minimal => None,
            SystemTier::Low => Some("whisper-tiny-en"),
            SystemTier::Medium => Some("whisper-base-en"),
            SystemTier::High => Some("whisper-small-en"),
            SystemTier::Ultra => Some("whisper-medium-en"),
        }
    }

    /// Returns all models suitable for this tier (from smallest to largest).
    #[must_use]
    pub fn suitable_models(&self) -> Vec<&'static str> {
        match self {
            SystemTier::Minimal => vec!["whisper-tiny-en"],
            SystemTier::Low => vec!["whisper-tiny-en", "whisper-base-en"],
            SystemTier::Medium => vec!["whisper-tiny-en", "whisper-base-en", "whisper-small-en"],
            SystemTier::High => vec![
                "whisper-tiny-en",
                "whisper-base-en",
                "whisper-small-en",
                "whisper-medium-en",
            ],
            SystemTier::Ultra => vec![
                "whisper-tiny-en",
                "whisper-base-en",
                "whisper-small-en",
                "whisper-medium-en",
                "whisper-large-v3",
            ],
        }
    }

    /// Returns the tier name as a string.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemTier::Minimal => "minimal",
            SystemTier::Low => "low",
            SystemTier::Medium => "medium",
            SystemTier::High => "high",
            SystemTier::Ultra => "ultra",
        }
    }

    /// Adjusts the tier for mobile platforms due to thermal constraints.
    ///
    /// Mobile devices are capped at Medium tier regardless of hardware
    /// to prevent thermal throttling issues.
    #[must_use]
    pub fn adjust_for_mobile(&self, platform: Platform) -> Self {
        if platform.is_mobile() {
            match self {
                SystemTier::High | SystemTier::Ultra => SystemTier::Medium,
                other => *other,
            }
        } else {
            *self
        }
    }
}

impl std::fmt::Display for SystemTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SystemTier {
    type Err = LexiconError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(SystemTier::Minimal),
            "low" => Ok(SystemTier::Low),
            "medium" => Ok(SystemTier::Medium),
            "high" => Ok(SystemTier::High),
            "ultra" => Ok(SystemTier::Ultra),
            _ => Err(LexiconError::Config(format!(
                "Invalid system tier: '{}'. Valid values: minimal, low, medium, high, ultra",
                s
            ))),
        }
    }
}

impl Default for SystemTier {
    fn default() -> Self {
        SystemTier::Medium
    }
}

/// GPU information detected on the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU name/description
    pub name: String,
    /// Estimated VRAM in megabytes (if detectable)
    pub vram_mb: Option<u64>,
    /// Whether this is a discrete GPU
    pub is_discrete: bool,
    /// Whether this is Apple Silicon with unified memory
    pub is_apple_silicon: bool,
    /// Whether the device has ANE (Apple Neural Engine)
    pub has_ane: bool,
}

impl Default for GpuInfo {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            vram_mb: None,
            is_discrete: false,
            is_apple_silicon: false,
            has_ane: false,
        }
    }
}

/// Complete system hardware profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProfile {
    /// Computed system tier
    pub tier: SystemTier,
    /// Total system RAM in bytes
    pub ram_bytes: u64,
    /// Number of logical CPU cores
    pub cpu_cores: usize,
    /// CPU architecture (e.g., "x86_64", "aarch64")
    pub cpu_arch: String,
    /// CPU brand/model name
    pub cpu_brand: Option<String>,
    /// GPU information (if detected)
    pub gpu: Option<GpuInfo>,
    /// Free disk space in bytes (in data directory)
    pub disk_free_bytes: u64,
    /// Detected platform
    pub platform: Platform,
    /// Timestamp when profile was created
    pub profiled_at: DateTime<Utc>,
}

impl SystemProfile {
    /// Returns RAM in gigabytes.
    #[must_use]
    pub fn ram_gb(&self) -> f64 {
        self.ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Returns free disk space in gigabytes.
    #[must_use]
    pub fn disk_free_gb(&self) -> f64 {
        self.disk_free_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Checks if the system has a good GPU for inference.
    #[must_use]
    pub fn has_good_gpu(&self) -> bool {
        self.gpu.as_ref().map_or(false, |g| {
            g.vram_mb.map_or(false, |vram| vram >= 4096) || g.is_apple_silicon
        })
    }
}

/// Cached system profile with validity information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedProfile {
    /// The system profile
    pub profile: SystemProfile,
    /// Application version when profile was created
    pub app_version: String,
}

impl CachedProfile {
    /// Creates a new cached profile.
    #[must_use]
    pub fn new(profile: SystemProfile) -> Self {
        Self {
            profile,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Checks if the cached profile is still valid.
    ///
    /// A profile is considered stale if:
    /// - It's older than 30 days
    /// - The app version has changed (major version)
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let age = Utc::now() - self.profile.profiled_at;
        let age_valid = age < chrono::Duration::days(30);

        // Compare major version only
        let current_version = env!("CARGO_PKG_VERSION");
        let cached_major = self.app_version.split('.').next().unwrap_or("0");
        let current_major = current_version.split('.').next().unwrap_or("0");
        let version_valid = cached_major == current_major;

        age_valid && version_valid
    }
}

/// System profiler for detecting hardware capabilities.
pub struct SystemProfiler;

impl SystemProfiler {
    /// Profile the current system hardware.
    ///
    /// Uses the `sysinfo` crate for cross-platform hardware detection.
    #[must_use]
    pub fn profile() -> SystemProfile {
        use sysinfo::System;

        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();

        // Get RAM
        let ram_bytes = sys.total_memory();

        // Get CPU info
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let cpu_arch = std::env::consts::ARCH.to_string();
        let cpu_brand = sys.cpus().first().map(|c| c.brand().to_string());

        // Detect platform
        let platform = Platform::detect();

        // GPU detection (platform-specific)
        let gpu = Self::detect_gpu(&platform);

        // Disk space detection
        let disk_free_bytes = Self::detect_disk_space();

        // Calculate tier
        let tier = Self::calculate_tier(ram_bytes, &gpu, platform);

        SystemProfile {
            tier,
            ram_bytes,
            cpu_cores,
            cpu_arch,
            cpu_brand,
            gpu,
            disk_free_bytes,
            platform,
            profiled_at: Utc::now(),
        }
    }

    /// Calculate system tier from hardware profile.
    fn calculate_tier(ram_bytes: u64, gpu: &Option<GpuInfo>, platform: Platform) -> SystemTier {
        let ram_gb = ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let has_good_gpu = gpu.as_ref().map_or(false, |g| {
            g.vram_mb.map_or(false, |vram| vram >= 4096) || g.is_apple_silicon
        });

        let base_tier = match (ram_gb, has_good_gpu) {
            (r, _) if r < 4.0 => SystemTier::Minimal,
            (r, false) if r < 6.0 => SystemTier::Low,
            (r, false) if r < 8.0 => SystemTier::Medium,
            (r, true) if r < 8.0 => SystemTier::Medium,
            (r, false) if r < 16.0 => SystemTier::High,
            (r, true) if r < 16.0 => SystemTier::High,
            (_, true) => SystemTier::Ultra,
            (_, false) => SystemTier::High,
        };

        // Adjust for mobile
        base_tier.adjust_for_mobile(platform)
    }

    /// Detect GPU information (platform-specific).
    fn detect_gpu(platform: &Platform) -> Option<GpuInfo> {
        match platform {
            // Apple platforms - assume Metal is available
            Platform::MacOsArm64 => Some(GpuInfo {
                name: "Apple Silicon GPU".to_string(),
                vram_mb: None, // Unified memory, shares with RAM
                is_discrete: false,
                is_apple_silicon: true,
                has_ane: true,
            }),
            Platform::MacOsX86 => Some(GpuInfo {
                name: "Intel/AMD GPU (Metal)".to_string(),
                vram_mb: None,
                is_discrete: false, // Could be either, conservative default
                is_apple_silicon: false,
                has_ane: false,
            }),
            Platform::Ios => Some(GpuInfo {
                name: "Apple GPU".to_string(),
                vram_mb: None,
                is_discrete: false,
                is_apple_silicon: true,
                has_ane: true, // A14+ chips have ANE
            }),
            Platform::IosSimulator => Some(GpuInfo {
                name: "Simulator GPU".to_string(),
                vram_mb: None,
                is_discrete: false,
                is_apple_silicon: cfg!(target_arch = "aarch64"),
                has_ane: false,
            }),
            // Android - assume OpenGL ES is available
            Platform::AndroidArm64 | Platform::AndroidArm32 => Some(GpuInfo {
                name: "Mobile GPU".to_string(),
                vram_mb: None,
                is_discrete: false,
                is_apple_silicon: false,
                has_ane: false,
            }),
            // Desktop platforms - conservative defaults
            // Real GPU detection would require platform-specific APIs
            Platform::LinuxX86 | Platform::LinuxArm64 => {
                // Could potentially detect NVIDIA via nvidia-smi
                // For now, return None to indicate unknown
                None
            }
            Platform::Windows => {
                // Could potentially detect via DXGI
                // For now, return None to indicate unknown
                None
            }
            Platform::Unknown => None,
        }
    }

    /// Detect available disk space.
    fn detect_disk_space() -> u64 {
        use sysinfo::Disks;

        let disks = Disks::new_with_refreshed_list();

        // Find the disk with the most available space
        // (typically the main disk for data storage)
        disks
            .iter()
            .map(|d| d.available_space())
            .max()
            .unwrap_or(0)
    }

    /// Recommend a model based on system profile.
    #[must_use]
    pub fn recommend_model(profile: &SystemProfile) -> ModelRecommendation {
        let model_id = profile.tier.recommended_model();

        let rationale = match profile.tier {
            SystemTier::Minimal => {
                "Limited RAM detected. Using tiny model for best stability.".to_string()
            }
            SystemTier::Low => "Moderate RAM available. Base model provides good balance.".to_string(),
            SystemTier::Medium => {
                "Good system resources. Small model recommended for quality.".to_string()
            }
            SystemTier::High => "Strong hardware detected. Medium model for high accuracy.".to_string(),
            SystemTier::Ultra => {
                "High-end system. Large model for maximum transcription quality.".to_string()
            }
        };

        let alternatives: Vec<RecommendedModelInfo> = profile
            .tier
            .suitable_models()
            .iter()
            .filter(|&&m| m != model_id)
            .map(|&m| RecommendedModelInfo {
                model_id: m.to_string(),
                display_name: model_display_name(m),
                size_bytes: model_size_bytes(m),
            })
            .collect();

        ModelRecommendation {
            model_id: model_id.to_string(),
            display_name: model_display_name(model_id),
            size_bytes: model_size_bytes(model_id),
            rationale,
            alternatives,
        }
    }

    /// Get all models suitable for the given system profile.
    #[must_use]
    pub fn suitable_models(profile: &SystemProfile) -> Vec<RecommendedModelInfo> {
        profile
            .tier
            .suitable_models()
            .iter()
            .map(|&m| RecommendedModelInfo {
                model_id: m.to_string(),
                display_name: model_display_name(m),
                size_bytes: model_size_bytes(m),
            })
            .collect()
    }
}

/// Model recommendation with rationale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    /// Recommended model identifier
    pub model_id: String,
    /// Human-readable model name
    pub display_name: String,
    /// Model file size in bytes
    pub size_bytes: u64,
    /// Explanation of why this model was recommended
    pub rationale: String,
    /// Alternative models suitable for this system
    pub alternatives: Vec<RecommendedModelInfo>,
}

/// Basic model information for recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedModelInfo {
    /// Model identifier
    pub model_id: String,
    /// Human-readable model name
    pub display_name: String,
    /// Model file size in bytes
    pub size_bytes: u64,
}

/// Get display name for a model ID.
fn model_display_name(model_id: &str) -> String {
    match model_id {
        "whisper-tiny-en" => "Whisper Tiny (English)".to_string(),
        "whisper-base-en" => "Whisper Base (English)".to_string(),
        "whisper-small-en" => "Whisper Small (English)".to_string(),
        "whisper-medium-en" => "Whisper Medium (English)".to_string(),
        "whisper-large-v3" => "Whisper Large v3 (Multilingual)".to_string(),
        _ => model_id.to_string(),
    }
}

/// Get approximate file size for a model ID.
fn model_size_bytes(model_id: &str) -> u64 {
    match model_id {
        "whisper-tiny-en" => 75_000_000,     // ~75 MB
        "whisper-base-en" => 147_000_000,    // ~140 MB
        "whisper-small-en" => 483_000_000,   // ~460 MB
        "whisper-medium-en" => 1_533_000_000, // ~1.5 GB
        "whisper-large-v3" => 3_086_000_000,  // ~3 GB
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_tier_recommended_model() {
        assert_eq!(SystemTier::Minimal.recommended_model(), "whisper-tiny-en");
        assert_eq!(SystemTier::Low.recommended_model(), "whisper-base-en");
        assert_eq!(SystemTier::Medium.recommended_model(), "whisper-small-en");
        assert_eq!(SystemTier::High.recommended_model(), "whisper-medium-en");
        assert_eq!(SystemTier::Ultra.recommended_model(), "whisper-large-v3");
    }

    #[test]
    fn test_system_tier_suitable_models() {
        let minimal = SystemTier::Minimal.suitable_models();
        assert_eq!(minimal.len(), 1);
        assert!(minimal.contains(&"whisper-tiny-en"));

        let ultra = SystemTier::Ultra.suitable_models();
        assert_eq!(ultra.len(), 5);
        assert!(ultra.contains(&"whisper-large-v3"));
    }

    #[test]
    fn test_system_tier_mobile_adjustment() {
        // Desktop platforms should not be affected
        assert_eq!(
            SystemTier::Ultra.adjust_for_mobile(Platform::LinuxX86),
            SystemTier::Ultra
        );
        assert_eq!(
            SystemTier::High.adjust_for_mobile(Platform::Windows),
            SystemTier::High
        );

        // Mobile platforms cap at Medium
        assert_eq!(
            SystemTier::Ultra.adjust_for_mobile(Platform::Ios),
            SystemTier::Medium
        );
        assert_eq!(
            SystemTier::High.adjust_for_mobile(Platform::AndroidArm64),
            SystemTier::Medium
        );

        // Lower tiers unchanged on mobile
        assert_eq!(
            SystemTier::Low.adjust_for_mobile(Platform::Ios),
            SystemTier::Low
        );
    }

    #[test]
    fn test_system_tier_parsing() {
        assert_eq!("minimal".parse::<SystemTier>().unwrap(), SystemTier::Minimal);
        assert_eq!("LOW".parse::<SystemTier>().unwrap(), SystemTier::Low);
        assert_eq!("Medium".parse::<SystemTier>().unwrap(), SystemTier::Medium);
        assert!("invalid".parse::<SystemTier>().is_err());
    }

    #[test]
    fn test_system_profiler_profile() {
        let profile = SystemProfiler::profile();

        // Should have non-zero RAM
        assert!(profile.ram_bytes > 0);
        // Should have at least 1 CPU core
        assert!(profile.cpu_cores >= 1);
        // Should have a valid architecture
        assert!(!profile.cpu_arch.is_empty());
        // Tier should be valid
        assert!(profile.tier.suitable_models().len() >= 1);
    }

    #[test]
    fn test_system_profiler_recommend_model() {
        let profile = SystemProfiler::profile();
        let recommendation = SystemProfiler::recommend_model(&profile);

        // Should have a model ID
        assert!(!recommendation.model_id.is_empty());
        // Should have a display name
        assert!(!recommendation.display_name.is_empty());
        // Should have a rationale
        assert!(!recommendation.rationale.is_empty());
    }

    #[test]
    fn test_cached_profile_validity() {
        let profile = SystemProfiler::profile();
        let cached = CachedProfile::new(profile);

        // Fresh cache should be valid
        assert!(cached.is_valid());
    }

    #[test]
    fn test_tier_calculation_minimal() {
        // <4GB, no GPU = Minimal
        let tier = SystemProfiler::calculate_tier(
            3 * 1024 * 1024 * 1024, // 3GB
            &None,
            Platform::LinuxX86,
        );
        assert_eq!(tier, SystemTier::Minimal);
    }

    #[test]
    fn test_tier_calculation_low() {
        // 5GB, no GPU = Low
        let tier = SystemProfiler::calculate_tier(
            5 * 1024 * 1024 * 1024, // 5GB
            &None,
            Platform::LinuxX86,
        );
        assert_eq!(tier, SystemTier::Low);
    }

    #[test]
    fn test_tier_calculation_ultra_with_gpu() {
        // 32GB + good GPU = Ultra
        let gpu = Some(GpuInfo {
            name: "NVIDIA RTX 4090".to_string(),
            vram_mb: Some(24576),
            is_discrete: true,
            is_apple_silicon: false,
            has_ane: false,
        });
        let tier = SystemProfiler::calculate_tier(
            32 * 1024 * 1024 * 1024, // 32GB
            &gpu,
            Platform::LinuxX86,
        );
        assert_eq!(tier, SystemTier::Ultra);
    }

    #[test]
    fn test_tier_calculation_mobile_cap() {
        // Even with 32GB + good GPU, mobile caps at Medium
        let gpu = Some(GpuInfo {
            name: "Apple A17 Pro".to_string(),
            vram_mb: None,
            is_discrete: false,
            is_apple_silicon: true,
            has_ane: true,
        });
        let tier = SystemProfiler::calculate_tier(
            32 * 1024 * 1024 * 1024, // 32GB
            &gpu,
            Platform::Ios,
        );
        assert_eq!(tier, SystemTier::Medium);
    }

    #[test]
    fn test_model_info() {
        assert_eq!(model_display_name("whisper-tiny-en"), "Whisper Tiny (English)");
        assert_eq!(model_size_bytes("whisper-tiny-en"), 75_000_000);
        assert_eq!(model_size_bytes("unknown-model"), 0);
    }

    #[test]
    fn test_system_profile_helpers() {
        let profile = SystemProfile {
            tier: SystemTier::Medium,
            ram_bytes: 8 * 1024 * 1024 * 1024, // 8GB
            cpu_cores: 4,
            cpu_arch: "x86_64".to_string(),
            cpu_brand: Some("Intel Core i5".to_string()),
            gpu: Some(GpuInfo {
                name: "NVIDIA RTX 3060".to_string(),
                vram_mb: Some(12288),
                is_discrete: true,
                is_apple_silicon: false,
                has_ane: false,
            }),
            disk_free_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            platform: Platform::LinuxX86,
            profiled_at: Utc::now(),
        };

        assert!((profile.ram_gb() - 8.0).abs() < 0.01);
        assert!((profile.disk_free_gb() - 100.0).abs() < 0.01);
        assert!(profile.has_good_gpu());
    }
}
