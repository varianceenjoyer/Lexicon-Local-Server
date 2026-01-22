# Research Report: Cross-Platform System Profiling and GPU Detection

**ID:** R-SYS-01
**Status:** Complete (Research Phase)
**Category:** Infrastructure / Configuration
**Created:** 2026-01-23
**Last Updated:** 2026-01-23
**Related:** WP-CONFIG (Local Configuration Management)
**Output:** Implementation guide for system profiling and hardware detection

---

## Executive Summary

### Platform Support Matrix

| Platform | RAM Detection | CPU Detection | GPU Detection | Disk Space | Complexity |
|----------|--------------|---------------|---------------|------------|------------|
| **Linux** | ✅ sysinfo | ✅ sysinfo | ⚠️ Limited | ✅ sysinfo | LOW |
| **macOS** | ✅ sysinfo | ✅ sysinfo | ✅ Metal API | ✅ sysinfo | MEDIUM |
| **Windows** | ✅ sysinfo | ✅ sysinfo | ✅ WMI/DXGI | ✅ sysinfo | MEDIUM |
| **iOS** | ✅ ProcessInfo | ✅ ProcessInfo | ✅ Metal API | ⚠️ Limited | MEDIUM |
| **Android** | ✅ ActivityManager | ✅ Build.VERSION | ⚠️ Limited | ✅ StatFs | MEDIUM |

### Key Findings

1. **`sysinfo` crate covers desktop platforms well** — RAM, CPU, and disk detection work out-of-the-box
2. **GPU detection requires platform-specific code** — No single crate covers all platforms
3. **Mobile platforms need FFI wrappers** — Swift/Kotlin code required for accurate profiling
4. **VRAM detection is the hardest** — Only available via vendor-specific APIs (NVML, Metal, DXGI)

### Recommendation: TIERED IMPLEMENTATION

| Component | Approach | Priority |
|-----------|----------|----------|
| RAM/CPU/Disk | `sysinfo` crate | HIGH (easy) |
| GPU presence | Platform-specific APIs | MEDIUM |
| GPU VRAM | Optional, best-effort | LOW |
| Mobile profiling | FFI wrapper functions | MEDIUM |

---

## Problem Statement

WP-CONFIG requires implementing system profiling to automatically recommend appropriate Whisper models based on hardware capabilities. The profiler needs to detect:

1. Available RAM
2. CPU cores and architecture
3. GPU availability and capabilities
4. Available disk space
5. Platform-specific constraints

This research evaluates available Rust crates and platform APIs for cross-platform hardware detection.

---

## Desktop Platform Analysis

### The `sysinfo` Crate

**Crate:** [sysinfo](https://crates.io/crates/sysinfo) (v0.32+)

**Supported Platforms:** Linux, macOS, Windows, FreeBSD, Android, iOS (limited)

**License:** MIT

**Features:**
- System memory (total, used, available)
- CPU information (cores, frequency, usage)
- Disk information (total, free space per mount)
- Network interfaces
- Running processes

**Code Example:**
```rust
use sysinfo::System;

pub struct BasicSystemInfo {
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub cpu_count: usize,
    pub cpu_arch: String,
}

pub fn detect_basic_system() -> BasicSystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    BasicSystemInfo {
        total_ram_bytes: sys.total_memory(),
        available_ram_bytes: sys.available_memory(),
        cpu_count: sys.cpus().len(),
        cpu_arch: std::env::consts::ARCH.to_string(),
    }
}

pub fn get_disk_free_bytes(path: &std::path::Path) -> Option<u64> {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        if path.starts_with(disk.mount_point()) {
            return Some(disk.available_space());
        }
    }
    None
}
```

**Limitations:**
- No GPU detection
- No VRAM information
- iOS/Android support is limited (requires native wrapper for accuracy)

---

### GPU Detection by Platform

#### Linux: OpenGL/Vulkan Query

**Primary Method:** Parse `/proc` or use Vulkan/OpenGL context

```rust
#[cfg(target_os = "linux")]
pub fn detect_linux_gpu() -> Option<GpuInfo> {
    // Method 1: Check for common GPU paths
    let nvidia_present = std::path::Path::new("/dev/nvidia0").exists();
    let amd_present = std::path::Path::new("/dev/dri/renderD128").exists();

    if nvidia_present {
        // Try NVML for detailed info
        return detect_nvidia_gpu();
    }

    if amd_present {
        return Some(GpuInfo {
            name: "AMD GPU (detected)".into(),
            vram_bytes: None,
            vendor: GpuVendor::AMD,
        });
    }

    // Method 2: Parse lspci output (fallback)
    // Not recommended - requires subprocess

    None
}
```

**NVIDIA-Specific (NVML):**
```rust
// Using nvml-wrapper crate (optional dependency)
#[cfg(feature = "nvml")]
pub fn detect_nvidia_gpu() -> Option<GpuInfo> {
    use nvml_wrapper::Nvml;

    let nvml = Nvml::init().ok()?;
    let device = nvml.device_by_index(0).ok()?;

    Some(GpuInfo {
        name: device.name().ok()?,
        vram_bytes: Some(device.memory_info().ok()?.total),
        vendor: GpuVendor::NVIDIA,
    })
}
```

#### macOS: Metal API

**Primary Method:** Query Metal devices

```rust
#[cfg(target_os = "macos")]
pub fn detect_macos_gpu() -> Option<GpuInfo> {
    // Requires metal-rs or objc crate
    use metal::Device;

    let device = Device::system_default()?;

    Some(GpuInfo {
        name: device.name().to_string(),
        // Note: Metal doesn't expose VRAM directly
        // Use recommendedMaxWorkingSetSize as approximation
        vram_bytes: Some(device.recommended_max_working_set_size()),
        vendor: detect_vendor_from_name(&device.name()),
    })
}
```

**Alternative (System Profiler):**
```rust
// Fallback: Parse system_profiler output
pub fn detect_macos_gpu_fallback() -> Option<GpuInfo> {
    let output = std::process::Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output()
        .ok()?;

    // Parse JSON output for GPU info
    // More reliable for VRAM but requires subprocess
}
```

#### Windows: DXGI or WMI

**Primary Method:** DXGI (DirectX Graphics Infrastructure)

```rust
#[cfg(target_os = "windows")]
pub fn detect_windows_gpu() -> Option<GpuInfo> {
    use windows::Win32::Graphics::Dxgi::*;

    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
        let adapter: IDXGIAdapter1 = factory.EnumAdapters1(0).ok()?;

        let mut desc = DXGI_ADAPTER_DESC1::default();
        adapter.GetDesc1(&mut desc).ok()?;

        // Convert wide string to Rust string
        let name = String::from_utf16_lossy(
            &desc.Description[..desc.Description.iter().position(|&c| c == 0)?]
        );

        Some(GpuInfo {
            name,
            vram_bytes: Some(desc.DedicatedVideoMemory as u64),
            vendor: detect_vendor_from_id(desc.VendorId),
        })
    }
}
```

**Alternative (WMI):**
```rust
// Using wmi crate for broader compatibility
#[cfg(target_os = "windows")]
pub fn detect_windows_gpu_wmi() -> Option<GpuInfo> {
    use wmi::{COMLibrary, WMIConnection};

    let com_con = COMLibrary::new().ok()?;
    let wmi_con = WMIConnection::new(com_con.into()).ok()?;

    #[derive(Deserialize)]
    struct Win32_VideoController {
        Name: String,
        AdapterRAM: Option<u64>,
    }

    let results: Vec<Win32_VideoController> =
        wmi_con.query().ok()?;

    results.first().map(|gpu| GpuInfo {
        name: gpu.Name.clone(),
        vram_bytes: gpu.AdapterRAM,
        vendor: detect_vendor_from_name(&gpu.Name),
    })
}
```

---

## Mobile Platform Analysis

### iOS (Swift FFI)

iOS requires native Swift code for accurate system profiling.

**Swift Implementation:**
```swift
import Foundation
import Metal

@objc public class SystemProfiler: NSObject {
    @objc public static func totalMemoryBytes() -> UInt64 {
        return ProcessInfo.processInfo.physicalMemory
    }

    @objc public static func availableMemoryBytes() -> UInt64 {
        var info = mach_task_basic_info()
        var count = mach_msg_type_number_t(MemoryLayout<mach_task_basic_info>.size) / 4

        let result = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: 1) {
                task_info(mach_task_self_, task_flavor_t(MACH_TASK_BASIC_INFO), $0, &count)
            }
        }

        guard result == KERN_SUCCESS else { return 0 }

        // os_proc_available_memory() is more accurate on iOS 13+
        if #available(iOS 13.0, *) {
            return UInt64(os_proc_available_memory())
        }

        return ProcessInfo.processInfo.physicalMemory - info.resident_size
    }

    @objc public static func cpuCoreCount() -> Int {
        return ProcessInfo.processInfo.processorCount
    }

    @objc public static func gpuName() -> String? {
        return MTLCreateSystemDefaultDevice()?.name
    }

    @objc public static func gpuRecommendedMemory() -> UInt64 {
        guard let device = MTLCreateSystemDefaultDevice() else { return 0 }
        return UInt64(device.recommendedMaxWorkingSetSize)
    }

    @objc public static func thermalState() -> Int {
        return ProcessInfo.processInfo.thermalState.rawValue
    }
}
```

**Rust FFI Binding:**
```rust
#[cfg(target_os = "ios")]
extern "C" {
    fn SystemProfiler_totalMemoryBytes() -> u64;
    fn SystemProfiler_availableMemoryBytes() -> u64;
    fn SystemProfiler_cpuCoreCount() -> i32;
    fn SystemProfiler_gpuRecommendedMemory() -> u64;
    fn SystemProfiler_thermalState() -> i32;
}

#[cfg(target_os = "ios")]
pub fn detect_ios_system() -> SystemProfile {
    unsafe {
        let ram_total = SystemProfiler_totalMemoryBytes();
        let ram_available = SystemProfiler_availableMemoryBytes();
        let cpu_cores = SystemProfiler_cpuCoreCount() as usize;
        let gpu_mem = SystemProfiler_gpuRecommendedMemory();
        let thermal = SystemProfiler_thermalState();

        SystemProfile {
            ram_gb: ram_total as f32 / 1_073_741_824.0,
            cpu_cores,
            gpu_available: gpu_mem > 0,
            gpu_vram_gb: if gpu_mem > 0 { Some(gpu_mem as f32 / 1_073_741_824.0) } else { None },
            thermal_state: ThermalState::from_ios_raw(thermal),
            ..Default::default()
        }
    }
}
```

### Android (Kotlin FFI)

**Kotlin Implementation:**
```kotlin
package com.lexicon.profiler

import android.app.ActivityManager
import android.content.Context
import android.os.Build
import android.os.StatFs

object SystemProfiler {
    @JvmStatic
    fun getTotalMemoryBytes(context: Context): Long {
        val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val memInfo = ActivityManager.MemoryInfo()
        activityManager.getMemoryInfo(memInfo)
        return memInfo.totalMem
    }

    @JvmStatic
    fun getAvailableMemoryBytes(context: Context): Long {
        val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val memInfo = ActivityManager.MemoryInfo()
        activityManager.getMemoryInfo(memInfo)
        return memInfo.availMem
    }

    @JvmStatic
    fun getCpuCoreCount(): Int {
        return Runtime.getRuntime().availableProcessors()
    }

    @JvmStatic
    fun getCpuArchitecture(): String {
        return Build.SUPPORTED_ABIS.firstOrNull() ?: "unknown"
    }

    @JvmStatic
    fun getFreeDiskSpaceBytes(path: String): Long {
        val stat = StatFs(path)
        return stat.availableBlocksLong * stat.blockSizeLong
    }

    @JvmStatic
    fun getThermalHeadroom(context: Context): Float {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val powerManager = context.getSystemService(Context.POWER_SERVICE) as android.os.PowerManager
            return powerManager.getThermalHeadroom(10) // 10 second forecast
        }
        return 1.0f // No thermal data available
    }

    @JvmStatic
    fun hasVulkanSupport(): Boolean {
        // Check for Vulkan support
        return Build.VERSION.SDK_INT >= Build.VERSION_CODES.N
    }
}
```

**JNI Binding Pattern:**
```rust
#[cfg(target_os = "android")]
use jni::{JNIEnv, objects::JObject};

#[cfg(target_os = "android")]
pub fn detect_android_system(env: JNIEnv, context: JObject) -> SystemProfile {
    let profiler_class = env.find_class("com/lexicon/profiler/SystemProfiler").unwrap();

    let total_mem: i64 = env.call_static_method(
        profiler_class,
        "getTotalMemoryBytes",
        "(Landroid/content/Context;)J",
        &[context.into()]
    ).unwrap().j().unwrap();

    let available_mem: i64 = env.call_static_method(
        profiler_class,
        "getAvailableMemoryBytes",
        "(Landroid/content/Context;)J",
        &[context.into()]
    ).unwrap().j().unwrap();

    let cpu_cores: i32 = env.call_static_method(
        profiler_class,
        "getCpuCoreCount",
        "()I",
        &[]
    ).unwrap().i().unwrap();

    SystemProfile {
        ram_gb: total_mem as f32 / 1_073_741_824.0,
        available_ram_gb: available_mem as f32 / 1_073_741_824.0,
        cpu_cores: cpu_cores as usize,
        platform: "android".into(),
        ..Default::default()
    }
}
```

---

## Implementation Strategy

### Phase 1: Desktop Profiling (sysinfo)

```rust
use sysinfo::System;

#[derive(Debug, Clone)]
pub enum SystemTier {
    Minimal,  // < 4GB RAM
    Low,      // 4-6GB RAM
    Medium,   // 6-8GB RAM
    High,     // 8-16GB RAM
    Ultra,    // 16GB+ RAM
}

#[derive(Debug, Clone)]
pub struct SystemProfile {
    pub tier: SystemTier,
    pub ram_gb: f32,
    pub available_ram_gb: f32,
    pub cpu_cores: usize,
    pub cpu_arch: String,
    pub gpu_available: bool,
    pub gpu_name: Option<String>,
    pub gpu_vram_gb: Option<f32>,
    pub disk_free_gb: f32,
    pub platform: String,
}

impl SystemProfile {
    pub fn detect() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let ram_gb = sys.total_memory() as f32 / 1_073_741_824.0;
        let available_ram_gb = sys.available_memory() as f32 / 1_073_741_824.0;
        let cpu_cores = sys.cpus().len();
        let cpu_arch = std::env::consts::ARCH.to_string();
        let platform = std::env::consts::OS.to_string();

        // GPU detection (platform-specific)
        let (gpu_available, gpu_name, gpu_vram_gb) = detect_gpu();

        // Disk detection (model storage path)
        let disk_free_gb = detect_disk_free().unwrap_or(0.0);

        // Calculate tier
        let tier = Self::calculate_tier(ram_gb, gpu_available);

        Self {
            tier,
            ram_gb,
            available_ram_gb,
            cpu_cores,
            cpu_arch,
            gpu_available,
            gpu_name,
            gpu_vram_gb,
            disk_free_gb,
            platform,
        }
    }

    fn calculate_tier(ram_gb: f32, has_gpu: bool) -> SystemTier {
        match (ram_gb, has_gpu) {
            (r, _) if r < 4.0 => SystemTier::Minimal,
            (r, false) if r < 6.0 => SystemTier::Low,
            (r, _) if r < 8.0 => SystemTier::Medium,
            (r, true) if r < 16.0 => SystemTier::High,
            (r, _) if r >= 16.0 => SystemTier::Ultra,
            _ => SystemTier::Low,
        }
    }
}
```

### Phase 2: GPU Detection (Platform-Specific)

```rust
#[derive(Debug, Clone)]
pub enum GpuVendor {
    NVIDIA,
    AMD,
    Intel,
    Apple,
    Unknown,
}

fn detect_gpu() -> (bool, Option<String>, Option<f32>) {
    #[cfg(target_os = "macos")]
    {
        detect_metal_gpu()
    }

    #[cfg(target_os = "windows")]
    {
        detect_dxgi_gpu()
    }

    #[cfg(target_os = "linux")]
    {
        detect_linux_gpu_simple()
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        (false, None, None)
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_gpu_simple() -> (bool, Option<String>, Option<f32>) {
    // Simple presence check
    let nvidia = std::path::Path::new("/dev/nvidia0").exists();
    let amd = std::path::Path::new("/dev/dri/renderD128").exists();

    if nvidia {
        (true, Some("NVIDIA GPU".into()), None)
    } else if amd {
        (true, Some("AMD/Intel GPU".into()), None)
    } else {
        (false, None, None)
    }
}
```

### Phase 3: Model Recommendation

```rust
impl SystemProfile {
    pub fn recommend_model(&self) -> ModelRecommendation {
        let (model_id, rationale) = match self.tier {
            SystemTier::Minimal => (
                "whisper-tiny.en",
                "Lowest memory footprint for systems with limited RAM"
            ),
            SystemTier::Low => (
                "whisper-base.en",
                "Good balance of quality and speed for modest hardware"
            ),
            SystemTier::Medium => (
                "whisper-small.en",
                "Better accuracy while maintaining reasonable speed"
            ),
            SystemTier::High => (
                "whisper-medium.en",
                "High accuracy with GPU acceleration support"
            ),
            SystemTier::Ultra => (
                "whisper-large-v3",
                "Maximum accuracy for high-end systems"
            ),
        };

        ModelRecommendation {
            model_id: model_id.into(),
            rationale: rationale.into(),
            alternatives: self.suitable_models(),
        }
    }

    pub fn suitable_models(&self) -> Vec<String> {
        match self.tier {
            SystemTier::Minimal => vec!["whisper-tiny.en".into()],
            SystemTier::Low => vec![
                "whisper-tiny.en".into(),
                "whisper-base.en".into(),
            ],
            SystemTier::Medium => vec![
                "whisper-tiny.en".into(),
                "whisper-base.en".into(),
                "whisper-small.en".into(),
            ],
            SystemTier::High => vec![
                "whisper-base.en".into(),
                "whisper-small.en".into(),
                "whisper-medium.en".into(),
            ],
            SystemTier::Ultra => vec![
                "whisper-small.en".into(),
                "whisper-medium.en".into(),
                "whisper-large-v3".into(),
            ],
        }
    }
}
```

---

## Crate Recommendations

| Crate | Purpose | License | Platforms | Notes |
|-------|---------|---------|-----------|-------|
| `sysinfo` | RAM, CPU, Disk | MIT | All | Primary choice |
| `metal` | macOS/iOS GPU | MIT/Apache 2.0 | Apple | For Metal detection |
| `windows` | Windows APIs | MIT/Apache 2.0 | Windows | For DXGI access |
| `nvml-wrapper` | NVIDIA VRAM | MIT | Linux/Windows | Optional, detailed NVIDIA info |
| `wmi` | Windows WMI | MIT/Apache 2.0 | Windows | Alternative to DXGI |

**Cargo.toml additions:**
```toml
[dependencies]
sysinfo = "0.32"

[target.'cfg(target_os = "macos")'.dependencies]
metal = "0.29"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Win32_Graphics_Dxgi"] }

[target.'cfg(all(target_os = "linux", feature = "nvml"))'.dependencies]
nvml-wrapper = "0.10"
```

---

## Testing Requirements

### Test Matrix

| Test | Linux | macOS | Windows | iOS | Android |
|------|-------|-------|---------|-----|---------|
| RAM detection | Required | Required | Required | Required | Required |
| CPU core count | Required | Required | Required | Required | Required |
| CPU architecture | Required | Required | Required | Required | Required |
| GPU presence | Required | Required | Required | Required | Best-effort |
| GPU name | Best-effort | Required | Required | Required | N/A |
| GPU VRAM | Best-effort | Best-effort | Required | N/A | N/A |
| Disk free space | Required | Required | Required | N/A | Required |
| Tier calculation | Required | Required | Required | Required | Required |

### Mock Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_calculation() {
        assert_eq!(SystemProfile::calculate_tier(2.0, false), SystemTier::Minimal);
        assert_eq!(SystemProfile::calculate_tier(5.0, false), SystemTier::Low);
        assert_eq!(SystemProfile::calculate_tier(7.0, true), SystemTier::Medium);
        assert_eq!(SystemProfile::calculate_tier(12.0, true), SystemTier::High);
        assert_eq!(SystemProfile::calculate_tier(32.0, true), SystemTier::Ultra);
    }

    #[test]
    fn test_model_recommendation() {
        let profile = SystemProfile {
            tier: SystemTier::Medium,
            ..Default::default()
        };

        let rec = profile.recommend_model();
        assert_eq!(rec.model_id, "whisper-small.en");
        assert!(rec.alternatives.contains(&"whisper-base.en".to_string()));
    }
}
```

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| GPU detection fails | Falls back to CPU-only tier | Graceful degradation, tier recalculation |
| sysinfo inaccurate on mobile | Wrong recommendations | Use native FFI wrappers |
| VRAM not available | Can't verify GPU model fits | Use conservative estimates |
| Disk space calculation wrong | Model download fails | Check before download, retry |
| Thermal state changes | Model becomes unsuitable | Monitor during transcription |

---

## Dependencies

- **sysinfo** — Cross-platform system information (new dependency)
- **metal** — macOS GPU detection (optional, build-time feature)
- **windows** — Windows GPU detection (optional, build-time feature)
- **R-THR-01** — Thermal profiling patterns for mobile
- **WP-CONFIG** — Primary consumer of this research

---

## Completion Criteria

- [x] Document cross-platform RAM/CPU detection approaches
- [x] Document GPU detection strategies per platform
- [x] Provide Rust code samples for desktop profiling
- [x] Provide Swift/Kotlin code samples for mobile profiling
- [x] Define tier calculation algorithm
- [x] Recommend crate dependencies
- [x] Define test matrix

---

## Recommendations for WP-CONFIG

Based on this research:

1. **Add `sysinfo` crate immediately** — Covers desktop RAM/CPU/disk detection
2. **Implement tier-based model recommendations** — Use the algorithm in this report
3. **Make GPU detection optional/best-effort** — Don't block on GPU detection failure
4. **Plan for mobile FFI** — Native wrappers needed for accurate iOS/Android profiling
5. **Cache system profile** — Re-profile only on explicit request or config reset

### Implementation Priority

| Component | Effort | Priority |
|-----------|--------|----------|
| sysinfo integration | LOW | HIGH |
| Tier calculation | LOW | HIGH |
| Model recommendation | LOW | HIGH |
| macOS Metal GPU | MEDIUM | MEDIUM |
| Windows DXGI GPU | MEDIUM | MEDIUM |
| Linux GPU (basic) | LOW | MEDIUM |
| NVIDIA VRAM (nvml) | MEDIUM | LOW |
| iOS Swift wrapper | HIGH | MEDIUM (deferred) |
| Android Kotlin wrapper | HIGH | MEDIUM (deferred) |

---

## References

- [sysinfo crate documentation](https://docs.rs/sysinfo)
- [Metal GPU Device](https://developer.apple.com/documentation/metal/mtldevice)
- [DXGI Overview](https://docs.microsoft.com/en-us/windows/win32/direct3ddxgi/dxgi-overview)
- [NVML Reference](https://docs.nvidia.com/deploy/nvml-api/)
- [Android MemoryInfo](https://developer.android.com/reference/android/app/ActivityManager.MemoryInfo)
- [iOS ProcessInfo](https://developer.apple.com/documentation/foundation/processinfo)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Initial research report created |
| 2026-01-23 | Desktop profiling strategy documented |
| 2026-01-23 | Mobile FFI patterns documented |
| 2026-01-23 | Tier calculation algorithm defined |
