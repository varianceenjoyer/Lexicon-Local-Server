# Android Termux Development Environment

This directory contains documentation for setting up a physical Android device as a remote development and testing environment using Termux.

## Document Index

| Document | Description |
|----------|-------------|
| [01-termux-installation.md](01-termux-installation.md) | Installing Termux and initial setup |
| [02-ssh-configuration.md](02-ssh-configuration.md) | SSH server setup for remote access |
| [03-dev-environment.md](03-dev-environment.md) | Git, Rust, and build dependencies |
| [04-build-deployment.md](04-build-deployment.md) | Automated build scripts and deployment |

## Why Termux on Physical Hardware?

Testing on physical Android devices provides advantages over emulation:

- **Real Hardware Performance**: Accurate CPU/GPU/thermal profiling
- **Native Audio Stack**: Test actual audio capture and playback
- **Battery/Thermal Constraints**: Real-world power consumption testing
- **ARM Architecture**: Native aarch64 execution, no emulation overhead
- **Peripheral Access**: Bluetooth, USB, actual microphone hardware

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│  Development Machine (Windows/Linux/macOS)                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  SSH Client                                          │   │
│  │  git push → triggers build on device                │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ SSH (port 8022)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Android Device (Termux)                                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  sshd (port 8022)                                    │   │
│  │  └── dev user session                               │   │
│  │      ├── git pull                                   │   │
│  │      ├── cargo build --target aarch64-linux-android │   │
│  │      └── ./target/release/lexicon-server            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### On Android Device

1. Install Termux from F-Droid (not Play Store)
2. Run initial setup:
   ```bash
   pkg update && pkg upgrade -y
   pkg install openssh git
   passwd  # Set a password
   sshd    # Start SSH server
   ```
3. Get device IP: `ip addr show wlan0 | grep inet`

### On Development Machine

```bash
# Connect to device
ssh -p 8022 <device-ip>

# Or with key authentication (after setup)
ssh -p 8022 dev@<device-ip>
```

## Requirements

### Android Device

- Android 7.0+ (API 24+)
- ARM64 processor (aarch64)
- 4GB+ RAM recommended
- 8GB+ free storage
- WiFi connectivity to development network

### Network

- Device and development machine on same network
- Port 8022 accessible (no firewall blocking)
- Static IP or DHCP reservation recommended

## Related Documentation

- [Android Emulation](../android-emulation/README.md) - Cuttlefish emulator setup
- [BUILD.md](../../BUILD.md) - General build instructions
- [WP-ANDROID](../../work/WP-ANDROID.md) - Android platform work package
