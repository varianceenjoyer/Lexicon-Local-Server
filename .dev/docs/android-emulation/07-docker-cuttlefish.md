# Docker-Based Cuttlefish Setup

> **Work Plan:** WP-CUTTLEFISH-07
>
> **Status:** ACTIVE
>
> **Purpose:** Run Cuttlefish in Docker with Ubuntu 22.04 to resolve kernel compatibility issues.

---

## Current Status: Work in Progress

The Docker-based Cuttlefish setup is partially implemented. The container builds and runs, crosvm starts correctly (OpenWRT network VM boots), but the Android kernel doesn't produce output in either qemu_cli or crosvm mode.

**Root cause:** Likely a version mismatch between the cvd-host_package and Android images from the GitHub mirror.

**Recommended fix:** Download matching cvd-host_package AND Android images from the same Android CI build at https://ci.android.com/builds/branches/aosp-android-latest-release/grid

---

## Overview

This guide documents running Cuttlefish inside a Docker container with Ubuntu 22.04 LTS. This approach solves compatibility issues between newer host kernels (6.x+) and Cuttlefish's crosvm component.

### Why Docker?

| Issue | Native (Host) | Docker Solution |
|-------|---------------|-----------------|
| Kernel 6.x+ crosvm crashes | SIGSEGV errors | Ubuntu 22.04 container with compatible libraries |
| Ubuntu 24.04+ dependency issues | Package version mismatches | Isolated Ubuntu 22.04 environment |
| System pollution | Cuttlefish packages modify system | Contained in Docker volumes |
| Reproducibility | Host-dependent setup | Consistent across any Docker host |

### Requirements

- Docker 20.10+ with Docker Compose
- KVM-enabled host (same as native Cuttlefish)
- 8GB+ RAM available for container
- 20GB+ disk space for images and container

---

## Quick Start

### 1. Run the Setup Script

```bash
cd scripts/cuttlefish/docker
chmod +x *.sh
./setup.sh
```

This script will:
1. Create Docker volumes for Cuttlefish data
2. Download cvd-host_package (~500MB)
3. Download Android system images (~1.2GB)
4. Build the Docker image
5. Verify the installation

### 2. Start Cuttlefish

```bash
./run.sh start
# Or with docker-compose:
docker-compose up -d
```

### 3. Connect via ADB

```bash
# Wait for boot (~60-120 seconds)
adb connect localhost:6520
adb devices
```

### 4. Stop Cuttlefish

```bash
./run.sh stop
# Or:
docker-compose down
```

---

## Manual Setup

If you prefer to set up manually or need customization:

### Step 1: Build the Docker Image

```bash
cd scripts/cuttlefish/docker
docker build -t cuttlefish-emulator:ubuntu22.04 .
```

### Step 2: Create Docker Volumes

```bash
docker volume create cuttlefish-host
docker volume create cuttlefish-images
docker volume create cuttlefish-runtime
```

### Step 3: Download Cuttlefish Host Package

Option A: From GitHub mirror (recommended)
```bash
# Download
curl -L -o cvd-host_package.tar.gz \
    https://github.com/hipexscape/cuttlefish_releases/releases/download/20250619/cvd-host_package.tar.gz

# Extract into Docker volume
docker run --rm \
    -v cuttlefish-host:/opt/cuttlefish \
    -v $(pwd):/tmp/setup:ro \
    ubuntu:22.04 \
    bash -c "cd /opt/cuttlefish && tar -xzf /tmp/setup/cvd-host_package.tar.gz"
```

Option B: From Android CI
```bash
# Visit: https://ci.android.com/builds/branches/aosp-android-latest-release/grid
# Download cvd-host_package.tar.gz from a recent build
# Then extract as shown above
```

### Step 4: Download Android System Images

Option A: From GitHub mirror (includes GApps)
```bash
curl -L -o aosp_cf_x86_64_phone-img.zip \
    https://github.com/hipexscape/cuttlefish_releases/releases/download/20250619/aosp_cf_x86_64_phone-img-eng.cvd.zip

docker run --rm \
    -v cuttlefish-host:/opt/cuttlefish \
    -v $(pwd):/tmp/setup:ro \
    ubuntu:22.04 \
    bash -c "cd /opt/cuttlefish && unzip -o /tmp/setup/aosp_cf_x86_64_phone-img.zip"
```

Option B: From Android CI (for specific versions)
```bash
# Visit: https://ci.android.com/builds/branches/aosp-android-latest-release/grid
# Download aosp_cf_x86_64_phone-img-XXXXXX.zip
# Extract as shown above
```

### Step 5: Start the Container

```bash
docker-compose up -d
```

---

## Configuration

### Environment Variables

Configure Cuttlefish behavior via environment variables in `docker-compose.yml`:

| Variable | Default | Description |
|----------|---------|-------------|
| `CVD_CPUS` | 4 | Number of virtual CPUs |
| `CVD_MEMORY_MB` | 4096 | Memory allocation in MB |
| `CVD_HEADLESS` | true | Disable WebRTC display |
| `CVD_GPU_MODE` | guest_swiftshader | GPU rendering mode |
| `BOOT_TIMEOUT` | 300 | Boot timeout in seconds |

### Resource Limits

Adjust in `docker-compose.yml`:

```yaml
deploy:
  resources:
    limits:
      cpus: '6'
      memory: 8G
    reservations:
      cpus: '4'
      memory: 4G
```

### Network Mode

The default configuration uses `network_mode: host` for simplest ADB access. To use bridge networking with port mapping:

```yaml
services:
  cuttlefish:
    # Remove: network_mode: host
    ports:
      - "6520:6520"   # ADB
      - "8443:8443"   # WebRTC (if enabled)
```

---

## Usage

### Helper Script Commands

```bash
./run.sh start    # Start container
./run.sh stop     # Stop container
./run.sh restart  # Restart container
./run.sh logs     # View logs
./run.sh shell    # Open bash in container
./run.sh adb ...  # Run ADB command
./run.sh status   # Show device status
./run.sh clean    # Remove all data
```

### Direct Docker Commands

```bash
# Start
docker-compose up -d

# View logs
docker-compose logs -f

# Execute command in container
docker-compose exec cuttlefish <command>

# Stop
docker-compose down

# Remove everything including volumes
docker-compose down -v
```

### ADB Access

From the host:
```bash
# Connect (after container is running and device booted)
adb connect localhost:6520

# Verify
adb devices
# Output: localhost:6520    device

# Run commands
adb shell getprop ro.build.version.release
adb install myapp.apk
```

From inside the container:
```bash
docker-compose exec cuttlefish docker-entrypoint.sh adb devices
docker-compose exec cuttlefish docker-entrypoint.sh adb shell
```

---

## Directory Structure

```
scripts/cuttlefish/docker/
├── Dockerfile              # Ubuntu 22.04 with Cuttlefish deps
├── docker-compose.yml      # Container orchestration
├── docker-entrypoint.sh    # Container entrypoint script
├── build.sh               # Build image script
├── setup.sh               # Complete setup script
└── run.sh                 # Helper script for common operations
```

### Docker Volumes

| Volume | Mount Point | Purpose |
|--------|-------------|---------|
| `cuttlefish-host` | `/opt/cuttlefish` | Host package (launch_cvd, etc.) |
| `cuttlefish-images` | `/opt/android-images` | Android system images |
| `cuttlefish-runtime` | `/opt/cuttlefish/cuttlefish_runtime` | Instance runtime data |

---

## Troubleshooting

### KVM Access Denied

**Symptoms:**
```
[ERROR] Cannot access /dev/kvm. Check permissions.
```

**Solutions:**
1. Ensure KVM is available on host:
   ```bash
   ls -la /dev/kvm
   kvm-ok
   ```

2. Run container with privileged mode (default in docker-compose.yml):
   ```yaml
   privileged: true
   devices:
     - /dev/kvm:/dev/kvm
   ```

3. Check host user is in kvm group:
   ```bash
   sudo usermod -aG kvm $USER
   # Log out and back in
   ```

### Boot Timeout

**Symptoms:**
```
[ERROR] Boot timeout after 300s
```

**Solutions:**
1. Increase memory:
   ```yaml
   environment:
     - CVD_MEMORY_MB=8192
   ```

2. Increase timeout:
   ```yaml
   environment:
     - BOOT_TIMEOUT=600
   ```

3. Check container logs:
   ```bash
   ./run.sh logs
   docker-compose exec cuttlefish cat /opt/cuttlefish/cuttlefish_runtime/cuttlefish_runtime.1/launcher.log
   ```

### Container Won't Start

**Symptoms:**
Container exits immediately.

**Solutions:**
1. Check for existing container:
   ```bash
   docker ps -a | grep cuttlefish
   docker rm cuttlefish
   ```

2. Verify volumes have data:
   ```bash
   docker run --rm -v cuttlefish-host:/opt/cuttlefish ubuntu:22.04 ls -la /opt/cuttlefish
   ```

3. Run setup again:
   ```bash
   ./setup.sh
   ```

### ADB Connection Refused

**Symptoms:**
```
cannot connect to localhost:6520: Connection refused
```

**Solutions:**
1. Wait for device to boot (check logs):
   ```bash
   ./run.sh logs
   ```

2. Verify container is running:
   ```bash
   docker-compose ps
   ```

3. Check port is listening:
   ```bash
   ss -tlnp | grep 6520
   ```

### Images Not Found

**Symptoms:**
```
[ERROR] No Android system images found.
```

**Solutions:**
1. Re-run setup:
   ```bash
   ./setup.sh
   ```

2. Manually verify images in volume:
   ```bash
   docker run --rm -v cuttlefish-host:/opt/cuttlefish ubuntu:22.04 ls -la /opt/cuttlefish/*.img
   ```

---

## Updating Cuttlefish

### Update Host Package

```bash
# Download new host package
curl -L -o cvd-host_package.tar.gz <NEW_URL>

# Stop container
./run.sh stop

# Extract to volume (overwrites existing)
docker run --rm \
    -v cuttlefish-host:/opt/cuttlefish \
    -v $(pwd):/tmp/setup:ro \
    ubuntu:22.04 \
    bash -c "cd /opt/cuttlefish && tar -xzf /tmp/setup/cvd-host_package.tar.gz"

# Start container
./run.sh start
```

### Update Android Images

```bash
# Download new images
curl -L -o aosp_cf_x86_64_phone-img.zip <NEW_URL>

# Stop container
./run.sh stop

# Remove old images
docker run --rm -v cuttlefish-host:/opt/cuttlefish ubuntu:22.04 rm -f /opt/cuttlefish/*.img

# Extract new images
docker run --rm \
    -v cuttlefish-host:/opt/cuttlefish \
    -v $(pwd):/tmp/setup:ro \
    ubuntu:22.04 \
    bash -c "cd /opt/cuttlefish && unzip -o /tmp/setup/aosp_cf_x86_64_phone-img.zip"

# Start container
./run.sh start
```

### Rebuild Docker Image

```bash
./run.sh stop
./build.sh
./run.sh start
```

---

## CI Integration

### GitHub Actions Example

```yaml
jobs:
  android-test:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4

      - name: Setup Cuttlefish
        run: |
          cd scripts/cuttlefish/docker
          ./setup.sh

      - name: Start Cuttlefish
        run: |
          cd scripts/cuttlefish/docker
          docker-compose up -d
          # Wait for boot
          timeout 300 bash -c 'until adb connect localhost:6520 && adb -s localhost:6520 shell getprop sys.boot_completed | grep -q 1; do sleep 5; done'

      - name: Run Tests
        run: |
          adb -s localhost:6520 install app-debug.apk
          # Run your tests...

      - name: Stop Cuttlefish
        if: always()
        run: |
          cd scripts/cuttlefish/docker
          docker-compose down
```

---

## Known Issues

### Kernel Not Booting (Empty kernel.log)

**Status:** Under investigation

When using qemu_cli mode, the kernel may not boot properly, resulting in:
- Empty `/opt/cuttlefish/cuttlefish/instances/cvd-1/logs/kernel.log`
- ADB device shows as "offline"

**Possible causes:**
1. Bootloader incompatibility with QEMU
2. Missing QEMU machine configuration options
3. cvd-host_package version mismatch

**Workarounds to try:**
- Use a different cvd-host_package version from Android CI
- Try crosvm mode if running on Ubuntu 22.04 host kernel
- Enable console output: add `--console=true` to launch options

### TAP Device Conflicts

If you have a native Cuttlefish installation, clean up TAP devices before starting Docker:
```bash
for dev in cvd-mtap-01 cvd-mtap-02 cvd-etap-01 cvd-etap-02 cvd-ebr cvd-mbr; do
    sudo ip link delete "$dev" 2>/dev/null
done
```

---

## Related Documentation

- [01-environment-setup.md](01-environment-setup.md) - Host requirements
- [02-cuttlefish-installation.md](02-cuttlefish-installation.md) - Native installation (for comparison)
- [03-running-cuttlefish.md](03-running-cuttlefish.md) - General Cuttlefish operation
- [06-troubleshooting.md](06-troubleshooting.md) - General troubleshooting
