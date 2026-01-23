# Handoff: WP-CONFIG Implementation

> **ID**: HO-CONFIG-001 | **Work Package**: WP-CONFIG
>
> **Created**: 2026-01-23 | **Status**: ✅ COMPLETE (2026-01-23)

---

## Summary

Implement local configuration management with system profiling and CLI config commands. This is a **HIGH PRIORITY** Beta requirement that enables automatic model selection based on hardware.

---

## Quick Start

1. **Read the work package**: [.dev/work/WP-CONFIG.md](../work/WP-CONFIG.md)
2. **Read the research**: [.dev/design/research/R-SYS-01-system-profiling.md](../design/research/R-SYS-01-system-profiling.md)
3. **Start with sysinfo integration** (lowest risk, highest value)

---

## Implementation Order

### Phase 1: Core Types and sysinfo Integration (1-2 hours)

1. Add `sysinfo` to `crates/lexicon-core/Cargo.toml`:
   ```toml
   [dependencies]
   sysinfo = "0.32"
   ```

2. Create `crates/lexicon-core/src/config/mod.rs`:
   ```rust
   mod profile;
   mod manager;

   pub use profile::*;
   pub use manager::*;
   ```

3. Create `crates/lexicon-core/src/config/profile.rs` with `SystemProfile` and `SystemTier`

4. Implement `SystemProfile::detect()` using `sysinfo`:
   - RAM: `sys.total_memory()`, `sys.available_memory()`
   - CPU: `sys.cpus().len()`, `std::env::consts::ARCH`
   - Disk: `Disks::new_with_refreshed_list()`

5. Implement `SystemProfile::calculate_tier()`

6. Write unit tests for tier calculation

### Phase 2: GPU Detection (1-2 hours)

Platform-specific code behind `#[cfg]` attributes:

**macOS** (requires `metal` crate):
```rust
#[cfg(target_os = "macos")]
fn detect_metal_gpu() -> (bool, Option<String>, Option<f32>) {
    use metal::Device;
    if let Some(device) = Device::system_default() {
        (true, Some(device.name().to_string()), Some(device.recommended_max_working_set_size() as f32 / 1e9))
    } else {
        (false, None, None)
    }
}
```

**Windows** (requires `windows` crate):
```rust
#[cfg(target_os = "windows")]
fn detect_dxgi_gpu() -> (bool, Option<String>, Option<f32>) {
    // Use CreateDXGIFactory1 -> EnumAdapters1
    // See R-SYS-01 for full implementation
}
```

**Linux** (simple presence check):
```rust
#[cfg(target_os = "linux")]
fn detect_linux_gpu() -> (bool, Option<String>, Option<f32>) {
    let nvidia = Path::new("/dev/nvidia0").exists();
    let amd = Path::new("/dev/dri/renderD128").exists();
    (nvidia || amd, None, None)
}
```

### Phase 3: Model Recommendation (30 min)

Implement `SystemProfile::recommend_model()` and `suitable_models()`:

```rust
pub fn recommend_model(&self) -> ModelRecommendation {
    let (model_id, rationale) = match self.tier {
        SystemTier::Minimal => ("whisper-tiny.en", "Lowest memory for limited RAM"),
        SystemTier::Low => ("whisper-base.en", "Balance of quality and speed"),
        SystemTier::Medium => ("whisper-small.en", "Better accuracy"),
        SystemTier::High => ("whisper-medium.en", "High accuracy with GPU"),
        SystemTier::Ultra => ("whisper-large-v3", "Maximum accuracy"),
    };
    ModelRecommendation { model_id: model_id.into(), rationale: rationale.into(), ... }
}
```

### Phase 4: Configuration Manager (1 hour)

1. Create `crates/lexicon-core/src/config/manager.rs`
2. Add `toml` crate for config file format
3. Implement `LexiconLocalConfig` with load/save
4. Use platform-appropriate paths:
   - Linux: `~/.config/lexicon/config.toml`
   - macOS: `~/Library/Application Support/lexicon/config.toml`
   - Windows: `%APPDATA%\lexicon\config.toml`

### Phase 5: CLI Commands (1-2 hours)

1. Create `crates/lexicon-cli/src/commands/config.rs`
2. Add subcommands to main.rs:
   - `config show [--json]`
   - `config set <key> <value>`
   - `config reset [-y]`
   - `config profile [--json] [--refresh]`
   - `config recommend [--json]`

---

## Key Files

| File | Purpose |
|------|---------|
| `crates/lexicon-core/src/config/profile.rs` | System detection types |
| `crates/lexicon-core/src/config/manager.rs` | Config persistence |
| `crates/lexicon-cli/src/commands/config.rs` | CLI interface |

---

## Dependencies to Add

```toml
# crates/lexicon-core/Cargo.toml
[dependencies]
sysinfo = "0.32"
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
dirs = "5.0"  # For platform config paths

[target.'cfg(target_os = "macos")'.dependencies]
metal = "0.29"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Win32_Graphics_Dxgi"] }
```

---

## Testing Commands

```bash
# Run unit tests
cargo test -p lexicon-core config::

# Run specific tests
cargo test -p lexicon-core test_tier_calculation
cargo test -p lexicon-core test_model_recommendation

# Test CLI (after implementation)
cargo run -p lexicon-cli -- config show
cargo run -p lexicon-cli -- config profile --json
cargo run -p lexicon-cli -- config recommend
```

---

## Acceptance Checklist

- [x] `SystemProfile::detect()` returns valid data on Linux/macOS/Windows
- [x] `SystemTier` correctly calculated from RAM and GPU
- [x] Model recommendations match tier
- [x] Config file created on first run
- [x] Config persists across sessions
- [x] CLI `config show` displays config
- [x] CLI `config set` updates config
- [x] CLI `config profile` shows system info
- [x] CLI `config recommend` shows model recommendation
- [x] All tests pass

---

## Implementation Summary

**Completed 2026-01-23**

Implementation files:
- `crates/lexicon-core/src/system_profile.rs` - Full system profiling with `SystemProfiler`, `SystemTier`, `GpuInfo`, `SystemProfile`, `ModelRecommendation`
- `crates/lexicon-core/src/config.rs` - Configuration management with `LexiconConfig`, `ConfigBuilder`, TOML persistence
- `crates/lexicon-cli/src/main.rs` - CLI commands: `config show`, `config set`, `config reset`, `config profile`, `config recommend`, `config path`

Key features implemented:
- Cross-platform system detection via `sysinfo` crate
- GPU detection with Apple Silicon, discrete GPU support
- Mobile tier adjustment for thermal management
- TOML configuration with environment variable overrides
- Comprehensive unit tests

---

*This handoff is complete. Refer to the implementation files for details.*
