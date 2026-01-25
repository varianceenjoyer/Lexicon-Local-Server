# Package Installation

**Date:** 2026-01-24

## Required Packages Already Installed

Verified the following packages are installed:

```bash
$ dpkg -l | grep -E "qemu-kvm|libvirt-daemon|bridge-utils|f2fs-tools"
ii  bridge-utils        1.7.1-4ubuntu2          amd64    Utilities for configuring the Linux Ethernet bridge
ii  f2fs-tools          1.16.0-2                amd64    Tools for Flash-Friendly File System
ii  libvirt-daemon      11.6.0-1ubuntu3.2       amd64    virtualization daemon
```

## User Groups Configuration

### Current Status
```bash
$ groups $USER
dev : dev sudo kvm libvirt
```

**Result:** PASS - User is in kvm and libvirt groups

### cvdnetwork Group

The `cvdnetwork` group is needed for Cuttlefish networking. It doesn't exist by default:

```bash
$ getent group cvdnetwork
(no output - group does not exist)
```

**Action Required:** Create the cvdnetwork group and add user:
```bash
sudo groupadd cvdnetwork
sudo usermod -aG cvdnetwork $USER
```

## Additional Dependencies Check

```bash
$ which kvm-ok adb
/usr/sbin/kvm-ok
/home/dev/cuttlefish/bin/adb
```

**Result:** PASS - kvm-ok installed, adb available from Cuttlefish package

## Next Steps

1. Create cvdnetwork group
2. Proceed to Cuttlefish installation (03-cuttlefish-install.md)
