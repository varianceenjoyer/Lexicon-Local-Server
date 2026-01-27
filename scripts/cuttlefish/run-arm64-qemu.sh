#!/bin/bash
# run-arm64-qemu.sh
# Boot ARM64 Android Cuttlefish on x86_64 using QEMU TCG emulation
#
# Usage: ./run-arm64-qemu.sh [--daemon]
#
# Note: This uses software emulation (TCG) which is slow but functional.
# Boot time may be 5-15 minutes.

set -e

CUTTLEFISH_ARM64="${CUTTLEFISH_ARM64:-$HOME/cuttlefish-arm64}"
CUTTLEFISH_X86="${CUTTLEFISH_X86:-$HOME/cuttlefish}"
INSTANCE_DIR="${INSTANCE_DIR:-$HOME/cuttlefish-arm64-runtime}"

DAEMON=false
if [ "$1" = "--daemon" ]; then
    DAEMON=true
fi

echo "=== ARM64 Android on x86_64 (QEMU TCG) ==="
echo "Images: $CUTTLEFISH_ARM64"
echo "Instance: $INSTANCE_DIR"
echo ""

# Create instance directory
mkdir -p "$INSTANCE_DIR"

# Check required files
for img in super.img boot.img vendor_boot.img; do
    if [ ! -f "$CUTTLEFISH_ARM64/$img" ]; then
        echo "ERROR: Missing $CUTTLEFISH_ARM64/$img"
        exit 1
    fi
done

# Check for bootloader
BOOTLOADER="$CUTTLEFISH_X86/etc/bootloader_aarch64/bootloader.qemu"
if [ ! -f "$BOOTLOADER" ]; then
    echo "ERROR: Missing bootloader at $BOOTLOADER"
    exit 1
fi

# Create overlay for userdata if needed
USERDATA="$INSTANCE_DIR/userdata.qcow2"
if [ ! -f "$USERDATA" ]; then
    echo "Creating userdata image (4GB)..."
    qemu-img create -f qcow2 "$USERDATA" 4G
fi

echo ""
echo "Starting QEMU ARM64 emulation..."
echo "This will take several minutes to boot (software emulation)."
echo ""
echo "Console output will appear below. Use Ctrl+A X to exit."
echo ""

QEMU_CMD=(
    qemu-system-aarch64
    -name guest=android-arm64
    -machine virt,gic-version=2
    -cpu max,pauth-impdef=on  # Faster pointer auth
    -accel tcg,thread=multi   # Multi-threaded TCG
    -m 4096
    -smp 4

    # BIOS/bootloader
    -bios "$BOOTLOADER"

    # Console
    -serial mon:stdio
    -display none

    # Storage - using virtio-blk (without deprecated scsi option)
    -drive file="$CUTTLEFISH_ARM64/super.img",format=raw,if=none,id=super
    -device virtio-blk-pci,drive=super,bootindex=1

    -drive file="$CUTTLEFISH_ARM64/boot.img",format=raw,if=none,id=boot
    -device virtio-blk-pci,drive=boot

    -drive file="$CUTTLEFISH_ARM64/vendor_boot.img",format=raw,if=none,id=vboot
    -device virtio-blk-pci,drive=vboot

    -drive file="$USERDATA",format=qcow2,if=none,id=data
    -device virtio-blk-pci,drive=data

    # Networking for ADB
    -netdev user,id=net0,hostfwd=tcp::5555-:5555
    -device virtio-net-pci,netdev=net0

    # RNG
    -object rng-random,id=rng0,filename=/dev/urandom
    -device virtio-rng-pci,rng=rng0
)

if $DAEMON; then
    echo "Running in daemon mode..."
    echo "Logs: $INSTANCE_DIR/qemu.log"
    "${QEMU_CMD[@]}" > "$INSTANCE_DIR/qemu.log" 2>&1 &
    QEMU_PID=$!
    echo $QEMU_PID > "$INSTANCE_DIR/qemu.pid"
    echo "QEMU started with PID $QEMU_PID"
    echo ""
    echo "To monitor: tail -f $INSTANCE_DIR/qemu.log"
    echo "To connect ADB: adb connect localhost:5555"
    echo "To stop: kill $QEMU_PID"
else
    exec "${QEMU_CMD[@]}"
fi
