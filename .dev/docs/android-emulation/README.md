# Android Emulation Documentation

This directory contains documentation for setting up and using Google Cuttlefish for automated Android testing.

## Document Index

| Document | Status | Description |
|----------|--------|-------------|
| [01-environment-setup.md](01-environment-setup.md) | TODO | Host environment requirements and setup |
| [02-cuttlefish-installation.md](02-cuttlefish-installation.md) | TODO | Installing Cuttlefish and Android images |
| [03-running-cuttlefish.md](03-running-cuttlefish.md) | **VERIFIED** | Day-to-day operation and management |
| [04-ci-integration.md](04-ci-integration.md) | TODO | CI/CD pipeline integration |
| [05-rust-android-testing.md](05-rust-android-testing.md) | TODO | Testing Rust code on Android |
| [06-troubleshooting.md](06-troubleshooting.md) | **VERIFIED** | Common issues and maintenance |
| [07-docker-cuttlefish.md](07-docker-cuttlefish.md) | ACTIVE | Docker-based Cuttlefish (Ubuntu 22.04) |

> **Last Verification:** 2026-01-25 on Ubuntu 25.10 (kernel 6.17.0-8-generic)

## Why Cuttlefish?

Cuttlefish is Google's reference virtual Android device, designed specifically for CI/CD environments:

- **CI-Optimized**: Headless operation, fast boot times
- **Cloud-Ready**: Works on GCP, AWS, Azure, or bare metal with KVM
- **Automated Testing**: ADB-accessible, scriptable lifecycle
- **Scalable**: Run parallel tests on multiple virtual devices

## Quick Start

### Verified on Ubuntu 25.10 / Kernel 6.17+ (2026-01-25)

```bash
# 1. Clean any stale state
pkill -9 -f "launch_cvd|run_cvd|crosvm|modem_simulator" 2>/dev/null
rm -rf /home/dev/cuttlefish/instances

# 2. Launch with sandbox disabled (required for kernel 6.17+)
cd /home/dev/cuttlefish && ./bin/launch_cvd --enable_sandbox=false --daemon

# 3. Wait for boot and verify
sleep 30
/home/dev/cuttlefish/bin/adb devices
# Expected: 0.0.0.0:6520    device

# 4. Run shell commands
/home/dev/cuttlefish/bin/adb -s 0.0.0.0:6520 shell getprop ro.build.version.release
# Expected: 13

# 5. Access WebRTC UI (optional)
# Open https://localhost:8443 in browser

# 6. Stop when done
/home/dev/cuttlefish/bin/stop_cvd
```

### For older kernels (< 6.17)

```bash
# Standard launch (no sandbox workaround needed)
launch_cvd --daemon
adb connect localhost:6520
```

## Related Design Documents

- [design-plan.md](../../design/design-plan.md) - Work plans WP-CUTTLEFISH-01 through WP-CUTTLEFISH-06
- WP-ANDROID - Android platform integration work package
- WP-BUILD - CI/CD pipeline integration

## Contributing

When updating these documents:

1. Test all commands on a fresh Ubuntu VM (22.04 or 25.10)
2. Include expected output for verification steps
3. Add troubleshooting entries as issues are discovered
4. Update the document index status when complete
5. Note kernel version compatibility requirements (especially for 6.17+)
