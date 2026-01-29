# Cuttlefish Setup Process Log

This directory documents the actual steps taken to set up the Android Cuttlefish development environment on this machine, including troubleshooting steps encountered.

**Date Started:** 2026-01-24
**Last Updated:** 2026-01-24

## System Information

- **OS:** Ubuntu (Linux 6.17.0-8-generic)
- **CPU:** AMD with SVM virtualization (12 cores)
- **KVM Status:** Module loaded (kvm_amd)
- **Cuttlefish Home:** `~/cuttlefish`

## Document Index

| Document | Description |
|----------|-------------|
| [01-environment-verification.md](01-environment-verification.md) | Initial environment checks |
| [02-package-installation.md](02-package-installation.md) | Package installation steps |
| [03-cuttlefish-install.md](03-cuttlefish-install.md) | Cuttlefish host tools and images |
| [04-first-launch.md](04-first-launch.md) | First launch and verification |
| [05-rust-android-toolchain.md](05-rust-android-toolchain.md) | Rust cross-compilation setup |
| [06-summary.md](06-summary.md) | Complete summary and next steps |

## Quick Status

- [x] Environment verified (KVM works, user in kvm/libvirt groups)
- [x] Packages installed (qemu-kvm, libvirt, bridge-utils, f2fs-tools)
- [x] User groups configured (kvm, libvirt)
- [x] Cuttlefish host tools installed (cvd-host_package.tar.gz extracted)
- [x] Android images downloaded (Android 13 with GApps, from hipexscape/cuttlefish_releases)
- [ ] First successful launch - **BLOCKED** (needs sudo)
- [x] Rust Android targets installed (aarch64, x86_64)
- [x] cargo-ndk installed (v4.1.2)
- [x] Android NDK installed (r26d at ~/android-ndk-r26d)
- [x] Android x86_64 build working (lexicon-core without STT)
- [x] Android arm64-v8a build working (lexicon-core without STT)
- [x] whisper.cpp pre-built for Android (arm64 + x86_64)
- [x] lexicon-ffi Android build working (without STT/TTS - 140MB .so)
- [ ] Full lexicon-ffi with STT (whisper-rs-sys CMake conflicts)
- [ ] Full lexicon-ffi with TTS (ort-sys needs Android binaries)

## Current Blocker

**Missing /etc/cvd_config directory (requires sudo)**

The tarball installation method cannot proceed because Cuttlefish requires the system directory `/etc/cvd_config` which is normally created by the Debian packages.

### Resolution Required

Run one of:

```bash
# Option 1: Symlink to existing config (EASIEST - single command)
sudo ln -s /home/dev/cuttlefish/etc/cvd_config /etc/cvd_config

# Option 2: Install Debian packages (recommended for full setup)
sudo /home/dev/projects/Lexicon-Local-Server/scripts/cuttlefish/setup-cuttlefish-debian.sh
```

See [04-first-launch.md](04-first-launch.md) for detailed error logs.

## Helper Scripts

Scripts are available in `/scripts/`:

| Script | Purpose |
|--------|---------|
| `cuttlefish/setup-cuttlefish-debian.sh` | Full Debian package installation (requires sudo) |
| `cuttlefish/download-images.sh` | Instructions for downloading images |
| `cuttlefish/launch-cuttlefish.sh` | Launch Cuttlefish with proper HOME workaround |
| `android/build-android.sh` | Build lexicon-ffi for Android targets |

### Build for Android

```bash
# Build for x86_64 (Cuttlefish emulator)
./scripts/android/build-android.sh x86_64

# Build for ARM64 (real devices)
./scripts/android/build-android.sh arm64

# Build release mode
./scripts/android/build-android.sh arm64 --release

# Output: target/{target}/{mode}/liblexicon_ffi.so (140MB debug, smaller in release)
```

**Note:** Currently builds without STT/TTS features due to native library cross-compilation issues. See [05-rust-android-toolchain.md](05-rust-android-toolchain.md) for details.

## Two Installation Approaches

### Option A: Tarball Method (Current)

Uses `cvd-host_package.tar.gz` extracted to `~/cuttlefish`. Requires `HOME=$PWD` workaround when launching.

**Pros:** No sudo required, portable
**Cons:** No cvdnetwork group, no cvd fetch command

### Option B: Debian Package Method (Recommended for CI)

Builds and installs cuttlefish-base and cuttlefish-user packages from source.

**Pros:** Creates cvdnetwork group, provides cvd fetch, system integration
**Cons:** Requires sudo, longer setup time

Run `scripts/cuttlefish/setup-cuttlefish-debian.sh` to use this method.
