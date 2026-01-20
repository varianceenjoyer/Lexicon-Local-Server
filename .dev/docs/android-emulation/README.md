# Android Emulation Documentation

This directory contains documentation for setting up and using Google Cuttlefish for automated Android testing.

## Document Index

| Document | Status | Description |
|----------|--------|-------------|
| [01-environment-setup.md](01-environment-setup.md) | TODO | Host environment requirements and setup |
| [02-cuttlefish-installation.md](02-cuttlefish-installation.md) | TODO | Installing Cuttlefish and Android images |
| [03-running-cuttlefish.md](03-running-cuttlefish.md) | TODO | Day-to-day operation and management |
| [04-ci-integration.md](04-ci-integration.md) | TODO | CI/CD pipeline integration |
| [05-rust-android-testing.md](05-rust-android-testing.md) | TODO | Testing Rust code on Android |
| [06-troubleshooting.md](06-troubleshooting.md) | TODO | Common issues and maintenance |

## Why Cuttlefish?

Cuttlefish is Google's reference virtual Android device, designed specifically for CI/CD environments:

- **CI-Optimized**: Headless operation, fast boot times
- **Cloud-Ready**: Works on GCP, AWS, Azure, or bare metal with KVM
- **Automated Testing**: ADB-accessible, scriptable lifecycle
- **Scalable**: Run parallel tests on multiple virtual devices

## Quick Start

Once all documentation is complete, the typical workflow will be:

```bash
# 1. Verify environment (see 01-environment-setup.md)
kvm-ok

# 2. Install Cuttlefish (see 02-cuttlefish-installation.md)
# Download and extract cvd-host-package

# 3. Launch device (see 03-running-cuttlefish.md)
launch_cvd --daemon

# 4. Connect via ADB
adb connect localhost:6520

# 5. Run tests (see 05-rust-android-testing.md)
cargo ndk -t arm64-v8a test
```

## Related Design Documents

- [design-plan.md](../../design/design-plan.md) - Work plans WP-CUTTLEFISH-01 through WP-CUTTLEFISH-06
- WP-ANDROID - Android platform integration work package
- WP-BUILD - CI/CD pipeline integration

## Contributing

When updating these documents:

1. Test all commands on a fresh Ubuntu 22.04 VM
2. Include expected output for verification steps
3. Add troubleshooting entries as issues are discovered
4. Update the document index status when complete
