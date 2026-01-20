# Cuttlefish Environment Setup

> **Work Plan:** WP-CUTTLEFISH-01
>
> **Status:** TODO
>
> **Purpose:** Document the complete environment setup for running Cuttlefish.

---

## Prerequisites

### Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 8+ cores |
| RAM | 8 GB | 16+ GB |
| Storage | 50 GB free | 100+ GB SSD |
| Virtualization | VT-x/AMD-V + nested virt | Same |

### Software Requirements

- **Operating System:** Ubuntu 20.04 LTS or 22.04 LTS (recommended)
- **Kernel:** 5.4+ with KVM support
- **Packages:** See installation section below

---

## KVM Verification

Before proceeding, verify KVM is available:

```bash
# Check CPU virtualization support
egrep -c '(vmx|svm)' /proc/cpuinfo
# Output should be > 0

# Check KVM module is loaded
lsmod | grep kvm
# Should show kvm_intel or kvm_amd

# Install and run kvm-ok
sudo apt install cpu-checker
kvm-ok
# Expected: INFO: /dev/kvm exists
#           KVM acceleration can be used
```

If KVM is not available, see [Troubleshooting: KVM Problems](06-troubleshooting.md#kvm-problems).

---

## Installation

### Ubuntu Package Installation

```bash
# Update package lists
sudo apt update

# Install required packages
sudo apt install -y \
    git \
    curl \
    wget \
    unzip \
    python3 \
    python3-pip \
    qemu-kvm \
    libvirt-daemon-system \
    libvirt-clients \
    bridge-utils \
    virtinst \
    virt-manager \
    adb \
    fastboot

# Install Cuttlefish-specific dependencies
sudo apt install -y \
    f2fs-tools \
    libc++1 \
    libc++abi1 \
    libdrm2 \
    libfdt1 \
    libgbm1 \
    libgl1-mesa-glx \
    libwayland-client0 \
    libwayland-server0 \
    libx11-xcb1 \
    libxcb-randr0 \
    libxcb-render0 \
    libxcb-shape0 \
    libxcb-shm0 \
    libxcb-xfixes0 \
    libxcb1 \
    libxml2 \
    e2fsprogs
```

### User Group Configuration

```bash
# Add current user to required groups
sudo usermod -aG kvm $USER
sudo usermod -aG cvdnetwork $USER
sudo usermod -aG libvirt $USER

# Log out and back in for group changes to take effect
# Or use: newgrp kvm
```

### Network Bridge Setup

TODO: Document network bridge configuration for remote ADB access.

```bash
# Create bridge network for Cuttlefish
# This section needs to be completed based on actual Cuttlefish requirements
```

---

## Cloud Provider Guides

### Google Cloud Platform

TODO: Document GCP setup with nested virtualization.

```bash
# Create VM with nested virtualization
# gcloud compute instances create cuttlefish-host \
#     --enable-nested-virtualization \
#     --zone=us-central1-a \
#     --machine-type=n2-standard-8 \
#     --image-family=ubuntu-2204-lts \
#     --image-project=ubuntu-os-cloud \
#     --boot-disk-size=100GB
```

### Amazon Web Services

TODO: Document AWS setup (metal instances or nested virt).

### Microsoft Azure

TODO: Document Azure setup with nested virtualization.

---

## Verification

### Testing KVM

```bash
# Verify KVM works
sudo virsh list --all
# Should not show errors

# Quick KVM test
virt-host-validate qemu
# All checks should pass or show warnings (not failures)
```

### Testing Network

TODO: Document network verification steps.

### Storage Verification

```bash
# Check available disk space
df -h /home
# Ensure at least 50GB free

# Check filesystem supports required features
# Cuttlefish works best on ext4 or f2fs
```

---

## Troubleshooting Common Issues

See [06-troubleshooting.md](06-troubleshooting.md) for detailed troubleshooting.

### Quick Fixes

| Issue | Quick Fix |
|-------|-----------|
| `/dev/kvm` permission denied | `sudo usermod -aG kvm $USER` + logout/login |
| Nested virtualization disabled | Check BIOS settings, enable VT-x |
| Network bridge not working | Check firewall rules, restart libvirtd |

---

## Next Steps

Once environment setup is verified, proceed to [02-cuttlefish-installation.md](02-cuttlefish-installation.md).
