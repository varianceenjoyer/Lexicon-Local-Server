# CI/CD Integration

> **Status:** ARCHIVED REFERENCE
>
> **Note:** This documentation is for Cuttlefish CI/CD integration, which has been replaced by Android Studio. Kept for historical reference. See [../../archive/docs/cuttlefish/](../../archive/docs/cuttlefish/_index.md) for more archived documentation.

---

## Prerequisites

> **Note:** This documentation references archived Cuttlefish setup. See [../../archive/docs/cuttlefish/](../../archive/docs/cuttlefish/_index.md).

---

## Self-Hosted Runner Setup

### GitHub Actions Runner Installation

On the Cuttlefish host machine:

```bash
# Create runner directory
mkdir ~/actions-runner && cd ~/actions-runner

# Download runner (check for latest version)
curl -o actions-runner-linux-x64-2.311.0.tar.gz -L \
    https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-linux-x64-2.311.0.tar.gz

# Extract
tar xzf ./actions-runner-linux-x64-2.311.0.tar.gz

# Configure (get token from GitHub repo settings)
./config.sh --url https://github.com/<org>/<repo> --token <REGISTRATION_TOKEN>

# Install as service
sudo ./svc.sh install
sudo ./svc.sh start
```

### Permissions Configuration

The runner needs access to:

```bash
# Add runner user to required groups
sudo usermod -aG kvm runner
sudo usermod -aG cvdnetwork runner

# Ensure ADB is accessible
which adb  # Should be in PATH

# Verify KVM access
sudo -u runner kvm-ok
```

### Concurrent Job Handling

For parallel Android testing:

```yaml
# In workflow file
jobs:
  android-test:
    runs-on: [self-hosted, cuttlefish]
    strategy:
      matrix:
        instance: [1, 2, 3]  # Run on multiple instances
      max-parallel: 3
```

---

## Workflow Patterns

### Device Lifecycle in CI

```yaml
# .github/workflows/android-test.yml
name: Android Tests

on:
  push:
    branches: [main]
  pull_request:

jobs:
  android-test:
    runs-on: [self-hosted, cuttlefish]

    steps:
      - uses: actions/checkout@v4

      - name: Start Cuttlefish
        run: |
          # Clean any previous instance
          stop_cvd || true
          sleep 2

          # Start fresh instance
          launch_cvd --daemon

          # Wait for boot
          ./scripts/wait_for_boot.sh

      - name: Run Tests
        run: |
          adb connect localhost:6520
          adb wait-for-device

          # Your test commands here
          ./scripts/run_android_tests.sh

      - name: Collect Artifacts
        if: always()
        run: |
          mkdir -p artifacts
          adb logcat -d > artifacts/logcat.txt
          adb bugreport > artifacts/bugreport.zip

      - name: Upload Artifacts
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: android-logs
          path: artifacts/

      - name: Stop Cuttlefish
        if: always()
        run: stop_cvd || true
```

### Test Execution

```bash
#!/bin/bash
# scripts/run_android_tests.sh

set -e

# Ensure connected
adb devices | grep -q "localhost:6520"

# Push test artifacts
adb push target/aarch64-linux-android/release/liblexicon.so /data/local/tmp/

# Run tests
adb shell "cd /data/local/tmp && LD_LIBRARY_PATH=. ./test_runner"

# Capture results
adb pull /data/local/tmp/test_results.xml ./
```

### Artifact Collection

```bash
#!/bin/bash
# scripts/collect_artifacts.sh

ARTIFACT_DIR="${1:-./artifacts}"
mkdir -p "$ARTIFACT_DIR"

# Logcat
adb logcat -d > "$ARTIFACT_DIR/logcat.txt"

# ANR traces
adb pull /data/anr/ "$ARTIFACT_DIR/anr/" 2>/dev/null || true

# Tombstones (crash dumps)
adb pull /data/tombstones/ "$ARTIFACT_DIR/tombstones/" 2>/dev/null || true

# Screenshots (if applicable)
adb shell screencap /sdcard/screenshot.png
adb pull /sdcard/screenshot.png "$ARTIFACT_DIR/" 2>/dev/null || true

# Bugreport
adb bugreport "$ARTIFACT_DIR/bugreport.zip"

echo "Artifacts collected in $ARTIFACT_DIR"
```

---

## Agent Automation

### Scripted Device Management

For AI agents running automated tests:

```bash
#!/bin/bash
# scripts/cuttlefish_manager.sh

ACTION="${1:-status}"
INSTANCE="${2:-1}"

case "$ACTION" in
    start)
        echo "Starting Cuttlefish instance $INSTANCE..."
        launch_cvd --daemon --base_instance_num=$INSTANCE
        ./scripts/wait_for_boot.sh
        ;;

    stop)
        echo "Stopping Cuttlefish instance $INSTANCE..."
        stop_cvd
        ;;

    restart)
        $0 stop $INSTANCE
        sleep 5
        $0 start $INSTANCE
        ;;

    status)
        if adb -s localhost:$((6519 + INSTANCE)) shell echo "ok" 2>/dev/null | grep -q "ok"; then
            echo "Instance $INSTANCE: RUNNING"
            exit 0
        else
            echo "Instance $INSTANCE: STOPPED"
            exit 1
        fi
        ;;

    *)
        echo "Usage: $0 {start|stop|restart|status} [instance_num]"
        exit 1
        ;;
esac
```

### Parallel Testing

```bash
#!/bin/bash
# scripts/parallel_test.sh

# Start multiple instances
for i in 1 2 3; do
    launch_cvd --daemon --base_instance_num=$i &
done
wait

# Wait for all to boot
for i in 1 2 3; do
    port=$((6519 + i))
    while ! adb connect localhost:$port 2>/dev/null; do
        sleep 5
    done
    adb -s localhost:$port wait-for-device
done

# Run tests in parallel
parallel -j3 ./scripts/run_test.sh ::: 1 2 3

# Collect results
for i in 1 2 3; do
    port=$((6519 + i))
    adb -s localhost:$port pull /data/local/tmp/results.xml results_$i.xml
done
```

### Result Reporting

```bash
#!/bin/bash
# scripts/report_results.sh

# Aggregate test results
cat > test_summary.md << EOF
# Android Test Results

## Environment
- Cuttlefish Version: $(launch_cvd --version 2>/dev/null || echo "unknown")
- Android Version: $(adb shell getprop ro.build.version.release)
- API Level: $(adb shell getprop ro.build.version.sdk)

## Test Results
$(cat test_results.txt)

## Artifacts
- logcat.txt
- bugreport.zip
EOF
```

---

## Example Configurations

### GitHub Actions Workflow

Complete workflow for Lexicon Android testing:

```yaml
# .github/workflows/android.yml
name: Android CI

on:
  push:
    branches: [main]
  pull_request:
    paths:
      - 'crates/lexicon-ffi/**'
      - 'crates/lexicon-core/**'
      - '.github/workflows/android.yml'

jobs:
  build-android:
    runs-on: [self-hosted, cuttlefish]

    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: aarch64-linux-android

      - name: Install cargo-ndk
        run: cargo install cargo-ndk

      - name: Build for Android
        run: |
          cargo ndk -t arm64-v8a build --release -p lexicon-ffi

      - name: Start Cuttlefish
        run: |
          stop_cvd || true
          launch_cvd --daemon
          ./scripts/wait_for_boot.sh

      - name: Deploy and Test
        run: |
          adb connect localhost:6520
          adb push target/aarch64-linux-android/release/liblexicon.so /data/local/tmp/
          # Run FFI tests
          adb shell "cd /data/local/tmp && ./run_ffi_tests"

      - name: Cleanup
        if: always()
        run: stop_cvd || true
```

### Automation Scripts

```bash
# scripts/wait_for_boot.sh
#!/bin/bash
TIMEOUT=${1:-300}
INTERVAL=5

for ((i=0; i<TIMEOUT; i+=INTERVAL)); do
    if adb shell getprop sys.boot_completed 2>/dev/null | grep -q "1"; then
        echo "Boot complete!"
        exit 0
    fi
    echo "Waiting for boot... ($i/${TIMEOUT}s)"
    sleep $INTERVAL
done

echo "Boot timeout!"
exit 1
```

### Integration with WP-BUILD

Add to existing CI matrix:

```yaml
# In ci.yml, add Android job
android:
  needs: [build]  # After main build
  runs-on: [self-hosted, cuttlefish]
  steps:
    - uses: actions/checkout@v4
    - name: Run Android Tests
      run: ./scripts/android_ci.sh
```

---

## Next Steps

- For Rust-specific testing, see [05-rust-android-testing.md](05-rust-android-testing.md)
- For troubleshooting, see [06-troubleshooting.md](06-troubleshooting.md)
