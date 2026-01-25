#!/bin/bash
# download-images.sh
# Downloads Android system images for Cuttlefish
#
# Usage: ./download-images.sh [--github|--manual]
#   --github : Download from hipexscape/cuttlefish_releases (includes GApps)
#   --manual : Show instructions for downloading from ci.android.com

set -e

CUTTLEFISH_HOME="${CUTTLEFISH_HOME:-$HOME/cuttlefish}"
SOURCE="${1:-}"

# GitHub mirror URL (includes GApps, Android 13)
GITHUB_RELEASE="https://github.com/hipexscape/cuttlefish_releases/releases/download/20250619"
GITHUB_IMAGES="aosp_cf_x86_64_phone-img-eng.cvd.zip"

echo "=== Cuttlefish Image Downloader ==="
echo "Cuttlefish home: $CUTTLEFISH_HOME"
echo ""

# Check if images already exist
if ls "$CUTTLEFISH_HOME"/*.img 2>/dev/null | head -1 > /dev/null; then
    echo "WARNING: Images already exist in $CUTTLEFISH_HOME"
    echo "Existing images:"
    ls -la "$CUTTLEFISH_HOME"/*.img 2>/dev/null | head -5
    echo ""
    read -p "Continue and overwrite? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 1
    fi
fi

if [ "$SOURCE" = "--github" ]; then
    echo "=== Downloading from GitHub Mirror ==="
    echo "Source: hipexscape/cuttlefish_releases"
    echo "Note: This includes GApps (Google Play Services)"
    echo ""

    cd "$CUTTLEFISH_HOME"

    echo "Downloading images (~1.2GB)..."
    curl -L -o "$GITHUB_IMAGES" "$GITHUB_RELEASE/$GITHUB_IMAGES"

    echo "Extracting images..."
    unzip -o "$GITHUB_IMAGES"

    echo "Cleaning up..."
    rm -f "$GITHUB_IMAGES"

    echo ""
    echo "=== Download Complete ==="
    echo "Images installed to: $CUTTLEFISH_HOME"
    ls -la "$CUTTLEFISH_HOME"/*.img 2>/dev/null | head -10

    exit 0
fi

echo "=== Download Options ==="
echo ""
echo "Option 1: GitHub Mirror (recommended for quick setup)"
echo "  ./download-images.sh --github"
echo "  - Pre-built Android 13 with GApps"
echo "  - ~1.2GB download"
echo "  - From: hipexscape/cuttlefish_releases"
echo ""
echo "Option 2: Official Android CI (for specific versions)"
echo ""
echo "  1. Open: https://ci.android.com/builds/branches/aosp-android-latest-release/grid"
echo ""
echo "  2. Find target: aosp_cf_x86_64_phone or aosp_cf_x86_64_only_phone"
echo ""
echo "  3. Click 'userdebug' under a recent green build"
echo ""
echo "  4. In the Details panel, download:"
echo "     - aosp_cf_x86_64_phone-img-XXXXXX.zip (~2GB)"
echo ""
echo "  5. Extract to $CUTTLEFISH_HOME:"
echo "     cd $CUTTLEFISH_HOME"
echo "     unzip ~/Downloads/aosp_cf_x86_64_phone-img-*.zip"
echo ""
echo "Option 3: Using cvd fetch (requires Debian packages)"
echo "  cvd fetch --default_build aosp-android-latest-release/aosp_cf_x86_64_phone-userdebug"
