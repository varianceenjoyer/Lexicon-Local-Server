# Troubleshooting and Maintenance

> **Work Plan:** WP-CUTTLEFISH-06
>
> **Status:** VERIFIED
>
> **Purpose:** Document common issues and maintenance procedures.
>
> **Last Verified:** 2026-01-25 on Ubuntu 25.10 (kernel 6.17.0-8-generic)

---

## Common Issues

### KVM Problems

#### `/dev/kvm` Permission Denied

**Symptoms:**
```
Could not access KVM kernel module: Permission denied
qemu-system-x86_64: failed to initialize KVM: Permission denied
```

**Solution:**
```bash
# Add user to kvm group
sudo usermod -aG kvm $USER

# Verify group membership
groups $USER | grep kvm

# Log out and back in, or:
newgrp kvm

# Verify access
ls -la /dev/kvm
# Should show: crw-rw---- 1 root kvm ...
```

#### KVM Module Not Loaded

**Symptoms:**
```
kvm-ok: KVM acceleration cannot be used
/dev/kvm not found
```

**Solution:**
```bash
# Load KVM module
sudo modprobe kvm
sudo modprobe kvm_intel  # For Intel
# OR
sudo modprobe kvm_amd    # For AMD

# Make persistent
echo "kvm" | sudo tee /etc/modules-load.d/kvm.conf
echo "kvm_intel" | sudo tee -a /etc/modules-load.d/kvm.conf

# Verify
lsmod | grep kvm
```

#### Nested Virtualization Disabled (Cloud VMs)

**Symptoms:**
```
KVM: disabled by BIOS
nested virtualization not supported
```

**Solution:**

For GCP:
```bash
# VM must be created with --enable-nested-virtualization
# Cannot be changed on existing VM - recreate required
```

For AWS:
```bash
# Use metal instance types (e.g., c5.metal, m5.metal)
# Nested virtualization on standard instances is limited
```

For Azure:
```bash
# Use Dv3/Ev3 series or newer with nested virtualization support
# Enable via Azure portal or CLI
```

---

### Network Issues

#### Stale Network Devices (Launch Fails)

**Symptoms:**
```
Check failed: result.ok() Received `false`
Device "cvd-wtap-01" in use
```

**Cause:** Previous Cuttlefish crash left network tap devices in a stale state.

**Solution:**
```bash
# List stale Cuttlefish network devices
ip link show | grep "cvd-"

# Remove stale tap devices (requires sudo)
sudo ip link delete cvd-wtap-01
sudo ip link delete cvd-mtap-01
sudo ip link delete cvd-etap-01

# Or restart the cuttlefish host resources service
sudo systemctl restart cuttlefish-host-resources

# Then relaunch
rm -rf /home/dev/cuttlefish/instances
cd /home/dev/cuttlefish && ./bin/launch_cvd
```

#### Stale Instance Files (kernel-log-pipe exists)

**Symptoms:**
```
Unable to create named pipe at /home/dev/cuttlefish/instances/cvd-1/internal/kernel-log-pipe: File exists
```

**Solution:**
```bash
# Kill any stale processes
pkill -9 -f "launch_cvd|run_cvd|crosvm|modem_simulator"
sleep 2

# Remove the stale instance directory
rm -rf /home/dev/cuttlefish/instances

# Relaunch
cd /home/dev/cuttlefish && ./bin/launch_cvd
```

#### Port Already in Use

**Symptoms:**
```
Port 9200 Bind failed (Address already in use)
```

**Solution:**
```bash
# Find and kill the process using the port
lsof -i :9200 | grep -v PID | awk '{print $2}' | xargs -r kill -9

# Or kill the modem simulator specifically
pkill -9 -f modem_simulator

# Clean and restart
rm -rf /home/dev/cuttlefish/instances
cd /home/dev/cuttlefish && ./bin/launch_cvd
```

#### ADB Connection Refused

**Symptoms:**
```
cannot connect to localhost:6520: Connection refused
```

**Solution:**
```bash
# Check if Cuttlefish is running
ps aux | grep launch_cvd

# Check if port is listening
ss -tlnp | grep 6520

# Restart Cuttlefish
stop_cvd
launch_cvd --daemon

# Wait and retry
sleep 30
adb connect localhost:6520
```

#### Remote ADB Connection Fails

**Symptoms:**
```
cannot connect to 192.168.1.100:6520: Connection timed out
```

**Solution:**
```bash
# On Cuttlefish host, check firewall
sudo ufw status
sudo ufw allow 6520/tcp

# Check if bound to all interfaces (not just localhost)
ss -tlnp | grep 6520
# Should show 0.0.0.0:6520, not 127.0.0.1:6520

# If binding to localhost only, use port forwarding:
ssh -L 6520:localhost:6520 user@cuttlefish-host
# Then connect to localhost:6520 on local machine
```

#### Network Bridge Not Working

**Symptoms:**
```
Failed to create network bridge
virbr0 not found
```

**Solution:**
```bash
# Ensure libvirt is running
sudo systemctl start libvirtd
sudo systemctl enable libvirtd

# Create default network
sudo virsh net-start default
sudo virsh net-autostart default

# Verify
sudo virsh net-list
```

---

### crosvm Crashes

#### virtio-console Crash (SIGSEGV)

**Symptoms:**
```
[ERROR:external/crosvm/src/linux/mod.rs:1878] child pcivirtio-console (pid XXXXX) died: signo 17, status 11, code 2
failed write to child device process serial: failed to send packet: Broken pipe
```

**Cause:** Incompatibility between crosvm version and newer kernels (6.x+), specifically with seccomp sandbox policies.

**Verified Solution (Ubuntu 25.10 / Kernel 6.17+):**

Disable the seccomp sandbox:

```bash
cd /home/dev/cuttlefish && ./bin/launch_cvd --enable_sandbox=false
```

This was verified working on 2026-01-25 with:
- Ubuntu 25.10 (Questing Quokka)
- Kernel 6.17.0-8-generic
- AMD Ryzen 5 3600 (SVM virtualization)
- Cuttlefish Android 13

**Alternative Workarounds:**

1. Try running with console disabled:
```bash
cvd start --daemon --console=false
```

2. Try with different kernel settings:
```bash
cvd start --daemon --enable_minimal_mode=true
```

3. Download a newer cuttlefish release or build from source with matching kernel headers.

**Alternative: Docker-based Cuttlefish**

Use Docker-based Cuttlefish with Ubuntu 22.04. See [07-docker-cuttlefish.md](07-docker-cuttlefish.md) for complete setup instructions.

```bash
cd scripts/cuttlefish/docker
./setup.sh
./run.sh start
```

This runs Cuttlefish inside an Ubuntu 22.04 container with compatible kernel interfaces while using KVM passthrough for hardware acceleration.

---

### Boot Failures

#### Cuttlefish Won't Start

**Symptoms:**
```
launch_cvd exits immediately
No devices/emulators found
```

**Solution:**
```bash
# Check launcher logs
cat ~/cuttlefish_runtime/cuttlefish_runtime.1/launcher.log

# Common issues:
# - Missing dependencies (run apt install again)
# - Corrupted images (re-download)
# - Insufficient permissions (check groups)

# Clean start
rm -rf ~/cuttlefish_runtime
launch_cvd --daemon
```

#### Boot Loop / Hangs

**Symptoms:**
```
Device never reaches boot_completed=1
kernel.log shows repeating errors
```

**Solution:**
```bash
# Check kernel log
tail -100 ~/cuttlefish_runtime/cuttlefish_runtime.1/kernel.log

# Try with more resources
stop_cvd
launch_cvd --daemon --cpus=4 --memory_mb=8192

# Try different GPU mode
launch_cvd --daemon --gpu_mode=guest_swiftshader

# Re-download images if corrupted
rm -rf ~/cuttlefish/images
# Download fresh images
```

#### Insufficient Disk Space

**Symptoms:**
```
No space left on device
Failed to create virtual disk
```

**Solution:**
```bash
# Check disk space
df -h ~

# Clean old runtime data
rm -rf ~/cuttlefish_runtime/cuttlefish_runtime.*/

# Clean old images
rm -rf ~/cuttlefish/images-old/

# Consider moving to larger disk
ln -s /mnt/larger-disk/cuttlefish ~/cuttlefish
```

---

### ADB Connectivity

#### Device Offline

**Symptoms:**
```
localhost:6520    offline
```

**Solution:**
```bash
# Kill and restart ADB server
adb kill-server
adb start-server

# Reconnect
adb connect localhost:6520

# If still offline, restart Cuttlefish
stop_cvd
launch_cvd --daemon
```

#### Multiple Device Confusion

**Symptoms:**
```
error: more than one device/emulator
```

**Solution:**
```bash
# List all devices
adb devices -l

# Specify device explicitly
adb -s localhost:6520 shell ls

# Or set default
export ANDROID_SERIAL=localhost:6520
```

---

## Performance Tuning

### CPU Configuration

```bash
# Pin Cuttlefish to specific CPUs (reduces context switching)
# Edit launch_cvd wrapper or use taskset:
taskset -c 0-3 launch_cvd --daemon

# Disable CPU frequency scaling for consistent performance
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

### Memory Optimization

```bash
# Increase memory allocation
launch_cvd --daemon --memory_mb=8192

# Monitor memory usage
watch -n 1 'ps aux | grep crosvm | grep -v grep'

# Enable memory ballooning (if supported)
launch_cvd --daemon --enable_balloon=true
```

### I/O Tuning

```bash
# Use SSD for cuttlefish_runtime
# If on HDD, consider:
echo mq-deadline | sudo tee /sys/block/sda/queue/scheduler

# Use tmpfs for temporary data (if RAM permits)
sudo mount -t tmpfs -o size=10G tmpfs /tmp/cuttlefish_tmp
export CF_RUNTIME=/tmp/cuttlefish_tmp
```

### Boot Time Optimization

```bash
# Skip unnecessary features for CI
launch_cvd --daemon \
    --start_webrtc=false \
    --start_gnss_proxy=false \
    --enable_audio=false

# Use snapshot for faster subsequent boots (if available)
# Note: snapshot support varies by Cuttlefish version
```

---

## Maintenance

### Updating Images

```bash
# Backup current images
mv ~/cuttlefish/images ~/cuttlefish/images-backup-$(date +%Y%m%d)

# Download new images
cd ~/cuttlefish
wget <new-image-url>
unzip aosp_cf_x86_64_phone-img-*.zip -d images/

# Test new images
stop_cvd
rm -rf ~/cuttlefish_runtime
launch_cvd --daemon

# If successful, remove backup
rm -rf ~/cuttlefish/images-backup-*
```

### Log Management

```bash
# Log rotation (add to crontab)
# crontab -e
0 0 * * * find ~/cuttlefish_runtime -name "*.log" -mtime +7 -delete

# Manual cleanup
find ~/cuttlefish_runtime -name "*.log" -size +100M -delete

# Compress old logs
find ~/cuttlefish_runtime -name "*.log" -mtime +1 -exec gzip {} \;
```

### Disk Cleanup

```bash
#!/bin/bash
# scripts/cleanup_cuttlefish.sh

echo "Cleaning Cuttlefish..."

# Stop if running
stop_cvd 2>/dev/null

# Remove runtime data
rm -rf ~/cuttlefish_runtime/*/

# Remove old images (keep current)
find ~/cuttlefish -name "*.img.bak" -delete
find ~/cuttlefish -name "*.zip" -mtime +30 -delete

# Clear ADB cache
rm -rf ~/.android/adbkey.pub ~/.android/adbkey

echo "Cleanup complete"
df -h ~
```

---

## Recovery

### Instance Recovery

```bash
# If instance is corrupted:
stop_cvd
rm -rf ~/cuttlefish_runtime/cuttlefish_runtime.1

# Start fresh (userdata will be reset)
launch_cvd --daemon
```

### Clean Reinstall

```bash
#!/bin/bash
# scripts/reinstall_cuttlefish.sh

echo "Full Cuttlefish reinstall..."

# Stop everything
stop_cvd 2>/dev/null
pkill -9 -f crosvm
pkill -9 -f launch_cvd

# Remove all data
rm -rf ~/cuttlefish_runtime
rm -rf ~/cuttlefish

# Reinstall (adjust URLs as needed)
mkdir -p ~/cuttlefish
cd ~/cuttlefish

echo "Download host package and images manually, then re-run setup"
echo "See: 02-cuttlefish-installation.md"
```

### Network Reset

```bash
# Reset network configuration
sudo systemctl restart libvirtd
sudo virsh net-destroy default
sudo virsh net-start default

# Remove stale bridge interfaces
sudo ip link delete cvd-ebr0 2>/dev/null
sudo ip link delete cvd-mobile-tap0 2>/dev/null

# Restart Cuttlefish
stop_cvd
launch_cvd --daemon
```

---

## Quick Reference

### Common Commands

| Command | Description |
|---------|-------------|
| `launch_cvd --daemon` | Start Cuttlefish in background |
| `stop_cvd` | Stop Cuttlefish gracefully |
| `adb connect localhost:6520` | Connect ADB |
| `adb shell getprop sys.boot_completed` | Check boot status |
| `pkill -9 -f crosvm` | Force kill (last resort) |

### Log Locations

| Log | Path |
|-----|------|
| Launcher | `~/cuttlefish_runtime/cuttlefish_runtime.1/launcher.log` |
| Kernel | `~/cuttlefish_runtime/cuttlefish_runtime.1/kernel.log` |
| Logcat | `adb logcat -d` |
| ADB | `~/.android/adb.log` (if enabled) |

### Health Check Script

```bash
#!/bin/bash
# scripts/health_check.sh

echo "=== Cuttlefish Health Check ==="

echo -n "KVM: "
[ -c /dev/kvm ] && echo "OK" || echo "FAIL"

echo -n "Cuttlefish running: "
pgrep -f launch_cvd > /dev/null && echo "YES" || echo "NO"

echo -n "ADB connection: "
adb devices 2>/dev/null | grep -q "localhost:6520" && echo "OK" || echo "FAIL"

echo -n "Boot completed: "
[ "$(adb shell getprop sys.boot_completed 2>/dev/null)" = "1" ] && echo "YES" || echo "NO"

echo -n "Disk space: "
df -h ~ | tail -1 | awk '{print $4 " free"}'

echo "==========================="
```

---

## Additional Resources

- [Cuttlefish Google Source](https://android.googlesource.com/device/google/cuttlefish/)
- [Android CI](https://ci.android.com)
- [AOSP Cuttlefish Documentation](https://source.android.com/docs/setup/create/cuttlefish)
