---
type: index
status: archived
archived: 2026-01-29
reason: Replaced by Android Studio for Android development and testing
---

# Archived: Cuttlefish Documentation

> This documentation has been archived. Cuttlefish has been replaced by Android Studio for Android development and testing.

## Archived Contents

### Setup Process Log
- `setup-process/` - Original setup process documentation from 2026-01-24

### Emulation Guides
- `02-cuttlefish-installation.md` - Installation guide
- `03-running-cuttlefish.md` - Day-to-day operation
- `07-docker-cuttlefish.md` - Docker-based Cuttlefish setup

## Why Archived

Cuttlefish presented several challenges:
- Required sudo access for `/etc/cvd_config` directory
- Complex setup with Debian packages
- Kernel compatibility issues with newer kernels (6.17+)

Android Studio provides:
- Standard Android development workflow
- Built-in Android Emulator
- Better tooling integration (ADB, Gradle, etc.)

## Original Purpose

Cuttlefish was Google's reference virtual Android device for CI/CD environments. It was evaluated for automated Android testing of the Lexicon library.
