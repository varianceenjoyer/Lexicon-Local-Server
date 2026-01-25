#!/bin/bash
# docker-entrypoint.sh
# Entrypoint script for Cuttlefish Docker container

set -e

CUTTLEFISH_HOME="${CUTTLEFISH_HOME:-/opt/cuttlefish}"
IMAGES_DIR="${IMAGES_DIR:-/opt/android-images}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_kvm() {
    if [ ! -c /dev/kvm ]; then
        log_error "/dev/kvm not found. KVM is required for Cuttlefish."
        log_error "Run container with: --device=/dev/kvm"
        return 1
    fi

    if [ ! -r /dev/kvm ] || [ ! -w /dev/kvm ]; then
        log_error "Cannot access /dev/kvm. Check permissions."
        log_error "Run container with: --privileged or fix /dev/kvm permissions"
        return 1
    fi

    log_info "KVM access: OK"
    return 0
}

check_images() {
    # Check for images in CUTTLEFISH_HOME first, then IMAGES_DIR
    if ls "$CUTTLEFISH_HOME"/*.img 2>/dev/null | head -1 > /dev/null; then
        log_info "Android images found in $CUTTLEFISH_HOME"
        return 0
    fi

    if ls "$IMAGES_DIR"/*.img 2>/dev/null | head -1 > /dev/null; then
        log_info "Android images found in $IMAGES_DIR"
        # Create symlinks to images
        for img in "$IMAGES_DIR"/*.img; do
            ln -sf "$img" "$CUTTLEFISH_HOME/$(basename "$img")" 2>/dev/null || true
        done
        return 0
    fi

    log_error "No Android system images found."
    log_error "Mount images to /opt/android-images or /opt/cuttlefish"
    return 1
}

check_host_package() {
    if [ ! -x "$CUTTLEFISH_HOME/bin/launch_cvd" ]; then
        log_warn "Cuttlefish host package not found at $CUTTLEFISH_HOME/bin/launch_cvd"
        return 1
    fi
    log_info "Cuttlefish host package: OK"
    return 0
}

setup_cuttlefish() {
    log_info "Setting up Cuttlefish environment..."

    # Start rsyslog for crosvm syslog support
    sudo rsyslogd 2>/dev/null || true

    # Fix ownership of cuttlefish home directory if needed
    # This handles cases where volumes were created with different ownership
    if [ ! -w "$CUTTLEFISH_HOME" ]; then
        log_warn "Fixing ownership of $CUTTLEFISH_HOME..."
        sudo chown -R cuttlefish:cuttlefish "$CUTTLEFISH_HOME" 2>/dev/null || true
    fi

    # Ensure required directories exist and are writable
    mkdir -p "$CUTTLEFISH_HOME/cuttlefish_runtime"
    mkdir -p "$CUTTLEFISH_HOME/cuttlefish"
    mkdir -p "$CUTTLEFISH_HOME/cuttlefish/assembly"
    mkdir -p "$CUTTLEFISH_HOME/cuttlefish/instances"

    # Ensure /tmp is writable for O_TMPFILE operations
    mkdir -p /tmp/cvd

    # Create /etc/cvd_config symlink if needed
    if [ -d "$CUTTLEFISH_HOME/etc/cvd_config" ] && [ ! -e "/etc/cvd_config" ]; then
        sudo ln -sf "$CUTTLEFISH_HOME/etc/cvd_config" /etc/cvd_config 2>/dev/null || true
    fi

    # Clear any stale config files that might cause issues
    rm -f "$CUTTLEFISH_HOME/launcher_pseudo_fetcher_config.json" 2>/dev/null || true
}

launch_cuttlefish() {
    local extra_args="$@"

    log_info "Launching Cuttlefish..."

    cd "$CUTTLEFISH_HOME"

    # Default launch options
    local launch_opts=(
        "--daemon"
        "--cpus=${CVD_CPUS:-4}"
        "--memory_mb=${CVD_MEMORY_MB:-4096}"
    )

    # Headless mode (no WebRTC)
    if [ "${CVD_HEADLESS:-true}" = "true" ]; then
        launch_opts+=("--start_webrtc=false")
    fi

    # GPU mode
    if [ -n "${CVD_GPU_MODE:-}" ]; then
        launch_opts+=("--gpu_mode=$CVD_GPU_MODE")
    else
        launch_opts+=("--gpu_mode=guest_swiftshader")
    fi

    # VM manager (qemu_cli for better compatibility)
    if [ -n "${CVD_VM_MANAGER:-}" ]; then
        launch_opts+=("--vm_manager=$CVD_VM_MANAGER")
    fi

    # Add extra args
    if [ -n "$extra_args" ]; then
        launch_opts+=($extra_args)
    fi

    log_info "Launch command: launch_cvd ${launch_opts[*]}"

    HOME="$CUTTLEFISH_HOME" ./bin/launch_cvd "${launch_opts[@]}"
}

wait_for_boot() {
    local timeout="${BOOT_TIMEOUT:-300}"
    local interval=5
    local elapsed=0

    log_info "Waiting for device boot (timeout: ${timeout}s)..."

    while [ $elapsed -lt $timeout ]; do
        if HOME="$CUTTLEFISH_HOME" "$CUTTLEFISH_HOME/bin/adb" shell getprop sys.boot_completed 2>/dev/null | grep -q "1"; then
            log_info "Device booted successfully!"
            return 0
        fi
        sleep $interval
        elapsed=$((elapsed + interval))
        echo -n "."
    done

    echo ""
    log_error "Boot timeout after ${timeout}s"
    return 1
}

show_status() {
    log_info "=== Cuttlefish Status ==="

    HOME="$CUTTLEFISH_HOME" "$CUTTLEFISH_HOME/bin/adb" devices 2>/dev/null || true

    if HOME="$CUTTLEFISH_HOME" "$CUTTLEFISH_HOME/bin/adb" shell getprop sys.boot_completed 2>/dev/null | grep -q "1"; then
        log_info "Boot status: BOOTED"
        log_info "Android version: $(HOME="$CUTTLEFISH_HOME" "$CUTTLEFISH_HOME/bin/adb" shell getprop ro.build.version.release 2>/dev/null)"
    else
        log_warn "Boot status: NOT READY"
    fi
}

stop_cuttlefish() {
    log_info "Stopping Cuttlefish..."
    cd "$CUTTLEFISH_HOME"
    HOME="$CUTTLEFISH_HOME" ./bin/stop_cvd 2>/dev/null || true
}

# Main command handler
case "${1:-}" in
    launch)
        shift
        check_kvm || exit 1
        check_host_package || exit 1
        check_images || exit 1
        setup_cuttlefish
        launch_cuttlefish "$@"
        wait_for_boot
        show_status

        # Keep container running
        log_info "Container ready. ADB available at localhost:6520"
        log_info "Press Ctrl+C to stop"

        # Tail logs to keep container alive and show output
        tail -f "$CUTTLEFISH_HOME/cuttlefish_runtime/cuttlefish_runtime.1/kernel.log" 2>/dev/null || \
            tail -f /dev/null
        ;;

    shell)
        exec /bin/bash
        ;;

    adb)
        shift
        HOME="$CUTTLEFISH_HOME" "$CUTTLEFISH_HOME/bin/adb" "$@"
        ;;

    stop)
        stop_cuttlefish
        ;;

    status)
        show_status
        ;;

    check)
        check_kvm || exit 1
        check_host_package || exit 1
        check_images || exit 1
        log_info "All checks passed!"
        ;;

    *)
        echo "Usage: docker-entrypoint.sh {launch|shell|adb|stop|status|check}"
        echo ""
        echo "Commands:"
        echo "  launch [opts]  - Start Cuttlefish (default)"
        echo "  shell          - Open bash shell"
        echo "  adb [args]     - Run ADB command"
        echo "  stop           - Stop Cuttlefish"
        echo "  status         - Show device status"
        echo "  check          - Pre-flight checks"
        echo ""
        echo "Environment variables:"
        echo "  CVD_CPUS       - Number of CPUs (default: 4)"
        echo "  CVD_MEMORY_MB  - Memory in MB (default: 4096)"
        echo "  CVD_HEADLESS   - Disable WebRTC (default: true)"
        echo "  CVD_GPU_MODE   - GPU mode (default: guest_swiftshader)"
        echo "  BOOT_TIMEOUT   - Boot timeout in seconds (default: 300)"
        exit 1
        ;;
esac
