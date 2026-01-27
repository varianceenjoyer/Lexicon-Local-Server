#!/bin/bash
# Download Android ARM64 Cuttlefish images
#
# Usage: ./download-arm64.sh [DEST_DIR]
#
# This script downloads ARM64 Android images from Android CI.
# Note: ci.android.com requires browser-based download for some artifacts.
# This script uses the GitHub API to get download URLs when available.

set -e

DEST_DIR="${1:-$HOME/cuttlefish-arm64}"
X86_HOST_DIR="${2:-$HOME/cuttlefish}"

# Build IDs - update these as needed
# Check: https://ci.android.com/builds/branches/aosp-android-latest-release/grid
# Target: aosp_cf_arm64_only_phone-userdebug
BUILD_ID="14781626"

echo "=== Android ARM64 Cuttlefish Setup ==="
echo "ARM64 images: $DEST_DIR"
echo "x86_64 host tools: $X86_HOST_DIR"
echo ""

# Create directories
mkdir -p "$DEST_DIR"
mkdir -p "$X86_HOST_DIR"

# Step 1: Download x86_64 host tools (needed for bootloader)
echo "=== Step 1: x86_64 Host Tools ==="
if [ ! -f "$X86_HOST_DIR/etc/bootloader_aarch64/bootloader.qemu" ]; then
    echo "Downloading x86_64 host package from GitHub mirror..."
    cd "$X86_HOST_DIR"

    # Use hipexscape/cuttlefish_releases GitHub mirror
    GITHUB_URL=$(curl -sL "https://api.github.com/repos/hipexscape/cuttlefish_releases/releases/tags/20250619" | \
        grep -o '"browser_download_url": "[^"]*cvd-host_package.tar.gz"' | \
        cut -d'"' -f4)

    if [ -z "$GITHUB_URL" ]; then
        echo "ERROR: Could not find download URL from GitHub API"
        echo "Please download manually from:"
        echo "  https://github.com/hipexscape/cuttlefish_releases/releases"
        exit 1
    fi

    curl -L -o cvd-host_package.tar.gz "$GITHUB_URL" --progress-bar
    tar -xzf cvd-host_package.tar.gz
    echo "x86_64 host tools installed"
else
    echo "x86_64 host tools already present"
fi

# Step 2: Download ARM64 images
echo ""
echo "=== Step 2: ARM64 System Images ==="
cd "$DEST_DIR"

if [ ! -f "$DEST_DIR/super.img" ]; then
    echo ""
    echo "ARM64 images must be downloaded from Android CI:"
    echo ""
    echo "  1. Open: https://ci.android.com/builds/branches/aosp-android-latest-release/grid"
    echo "  2. Find: aosp_cf_arm64_only_phone-userdebug"
    echo "  3. Click a green build, then 'Artifacts'"
    echo "  4. Download: aosp_cf_arm64_only_phone-img-XXXXXX.zip"
    echo "  5. Extract to: $DEST_DIR"
    echo ""
    echo "     unzip aosp_cf_arm64_only_phone-img-*.zip -d $DEST_DIR"
    echo ""

    # Try automated download (may not work without auth)
    echo "Attempting automated download (build $BUILD_ID)..."
    TARGET="aosp_cf_arm64_only_phone-userdebug"
    IMG_ZIP="aosp_cf_arm64_only_phone-img-${BUILD_ID}.zip"
    URL="https://ci.android.com/builds/submitted/${BUILD_ID}/${TARGET}/latest/raw/${IMG_ZIP}"

    if curl -L -o "$IMG_ZIP" "$URL" --progress-bar 2>/dev/null; then
        # Check if we got HTML instead of zip
        if file "$IMG_ZIP" | grep -q "HTML"; then
            rm -f "$IMG_ZIP"
            echo ""
            echo "ERROR: Automated download returned HTML (auth required)"
            echo "Please download manually using the instructions above."
            exit 1
        fi
        unzip -o "$IMG_ZIP"
        rm -f "$IMG_ZIP"
    else
        echo "Automated download failed. Please download manually."
        exit 1
    fi
else
    echo "ARM64 images already present"
fi

# Verify installation
echo ""
echo "=== Verifying Installation ==="
echo ""
echo "ARM64 images:"
ls -la "$DEST_DIR"/*.img 2>/dev/null | head -5 || echo "  (none found)"

echo ""
echo "Bootloader:"
ls -la "$X86_HOST_DIR/etc/bootloader_aarch64/bootloader.qemu" 2>/dev/null || echo "  (not found)"

echo ""
echo "=== Setup Complete ==="
echo ""
echo "To run ARM64 Android on x86_64:"
echo "  ./scripts/cuttlefish/run-arm64-qemu.sh --daemon"
echo ""
echo "To wait for boot:"
echo "  ./scripts/cuttlefish/wait-for-arm64.sh"
echo ""
echo "Note: ARM64 on x86_64 uses QEMU TCG emulation (5-30 min boot time)"
