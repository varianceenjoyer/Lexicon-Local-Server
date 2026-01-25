# Environment Verification

**Date:** 2026-01-24

## KVM Verification

### CPU Virtualization Support
```bash
$ egrep -c '(vmx|svm)' /proc/cpuinfo
12
```
**Result:** PASS - 12 cores with AMD SVM support

### KVM Module Status
```bash
$ lsmod | grep kvm
kvm_amd               249856  0
kvm                  1466368  1 kvm_amd
irqbypass              16384  1 kvm
ccp                   159744  1 kvm_amd
```
**Result:** PASS - kvm_amd module loaded

### /dev/kvm Device
```bash
$ ls -la /dev/kvm
crw-rw---- 1 root kvm 10, 232 Jan 16 11:17 /dev/kvm
```
**Result:** PASS - Device exists with kvm group permissions

### User Groups
```bash
$ groups $USER
dev : dev sudo
```
**Result:** NEEDS ACTION - User not in 'kvm' group

## Troubleshooting Required

### Issue 1: User not in kvm group

**Problem:** The current user (dev) is not in the kvm group, which will cause permission denied errors when running Cuttlefish.

**Solution:**
```bash
sudo usermod -aG kvm $USER
# Then log out and back in, or:
newgrp kvm
```

### Issue 2: kvm-ok not installed

**Problem:** The `kvm-ok` tool from `cpu-checker` package is not installed.

**Solution:**
```bash
sudo apt install cpu-checker
```

## Initial Package Status

Checked packages:
- `adb` - NOT installed
- `qemu-kvm` - Status unknown
- `libvirt-daemon-system` - Status unknown
- `cpu-checker` - NOT installed

## Next Steps

1. Add user to kvm group
2. Install required packages (see 02-package-installation.md)
