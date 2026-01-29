# Android Emulation Documentation

This directory contains documentation for Android testing and emulation approaches.

## Document Index

| Document | Status | Description |
|----------|--------|-------------|
| [01-environment-setup.md](01-environment-setup.md) | REFERENCE | Host environment requirements and setup |
| [04-ci-integration.md](04-ci-integration.md) | TODO | CI/CD pipeline integration |
| [05-rust-android-testing.md](05-rust-android-testing.md) | TODO | Testing Rust code on Android |
| [06-troubleshooting.md](06-troubleshooting.md) | REFERENCE | Common issues and maintenance |
| [08-android-build-testing.md](08-android-build-testing.md) | VERIFIED | Android build and testing workflow |
| [09-arm64-qemu-emulation.md](09-arm64-qemu-emulation.md) | **VERIFIED** | ARM64 QEMU emulation (current approach) |

> **Last Verification:** 2026-01-27 on Ubuntu 25.10 (kernel 6.17.0-8-generic)

## Current Approach: Android Studio

The project uses Android Studio for Android development and testing:

- **Android Emulator** for device testing
- **ADB** for deployment and debugging
- **Standard Android toolchain** for building

## Archived: Cuttlefish

Google Cuttlefish documentation has been archived. Cuttlefish was replaced by Android Studio due to:
- Complex setup requiring sudo access
- Kernel compatibility issues with newer kernels
- Android Studio provides a more standard development workflow

See [../../archive/docs/cuttlefish/](../../archive/docs/cuttlefish/_index.md) for archived documentation.

## Related Design Documents

- WP-ANDROID - Android platform integration work package
- WP-BUILD - CI/CD pipeline integration

## Contributing

When updating these documents:

1. Test all commands on a fresh Ubuntu VM (22.04 or 25.10)
2. Include expected output for verification steps
3. Add troubleshooting entries as issues are discovered
4. Update the document index status when complete
5. Note kernel version compatibility requirements
