#!/bin/bash
# setup.sh
# Complete setup script for Docker-based Cuttlefish
#
# This script:
# 1. Creates Docker volumes
# 2. Downloads and extracts cvd-host_package
# 3. Downloads Android system images
# 4. Builds the Docker image

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Configuration
HOST_VOLUME="cuttlefish-host"
IMAGES_VOLUME="cuttlefish-images"
TEMP_DIR="/tmp/cuttlefish-setup"

# URLs - update these as needed
# GitHub mirror with GApps (recommended for quick setup)
GITHUB_RELEASE="https://github.com/hipexscape/cuttlefish_releases/releases/download/20250619"
GITHUB_HOST_PACKAGE="cvd-host_package.tar.gz"
GITHUB_IMAGES="aosp_cf_x86_64_phone-img-eng.cvd.zip"

echo "=== Cuttlefish Docker Setup ==="
echo ""
echo "This script will:"
echo "  1. Create Docker volumes for Cuttlefish"
echo "  2. Download cvd-host_package (~500MB)"
echo "  3. Download Android system images (~1.2GB)"
echo "  4. Build the Docker image"
echo ""
read -p "Continue? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Step 1: Create temp directory
echo ""
echo "=== Step 1: Preparing workspace ==="
mkdir -p "$TEMP_DIR"
cd "$TEMP_DIR"

# Step 2: Create Docker volumes
echo ""
echo "=== Step 2: Creating Docker volumes ==="
docker volume create "$HOST_VOLUME" || true
docker volume create "$IMAGES_VOLUME" || true
docker volume create "cuttlefish-runtime" || true

# Get volume mount paths
HOST_MOUNT=$(docker volume inspect "$HOST_VOLUME" --format '{{ .Mountpoint }}')
IMAGES_MOUNT=$(docker volume inspect "$IMAGES_VOLUME" --format '{{ .Mountpoint }}')

echo "Host volume mount: $HOST_MOUNT"
echo "Images volume mount: $IMAGES_MOUNT"

# Step 3: Download and extract host package
echo ""
echo "=== Step 3: Downloading cvd-host_package ==="

if [ ! -f "$TEMP_DIR/$GITHUB_HOST_PACKAGE" ]; then
    echo "Downloading from GitHub mirror..."
    curl -L -o "$TEMP_DIR/$GITHUB_HOST_PACKAGE" "$GITHUB_RELEASE/$GITHUB_HOST_PACKAGE"
else
    echo "Using cached host package"
fi

echo "Extracting to Docker volume..."
# Use a temporary container to extract into the volume
docker run --rm \
    -v "$HOST_VOLUME:/opt/cuttlefish" \
    -v "$TEMP_DIR:/tmp/setup:ro" \
    ubuntu:22.04 \
    bash -c "cd /opt/cuttlefish && tar -xzf /tmp/setup/$GITHUB_HOST_PACKAGE"

echo "Host package installed."

# Step 4: Download and extract Android images
echo ""
echo "=== Step 4: Downloading Android system images ==="

if [ ! -f "$TEMP_DIR/$GITHUB_IMAGES" ]; then
    echo "Downloading from GitHub mirror (~1.2GB)..."
    curl -L -o "$TEMP_DIR/$GITHUB_IMAGES" "$GITHUB_RELEASE/$GITHUB_IMAGES"
else
    echo "Using cached images"
fi

echo "Extracting to Docker volume..."
docker run --rm \
    -v "$HOST_VOLUME:/opt/cuttlefish" \
    -v "$TEMP_DIR:/tmp/setup:ro" \
    ubuntu:22.04 \
    bash -c "cd /opt/cuttlefish && unzip -o /tmp/setup/$GITHUB_IMAGES"

echo "Android images installed."

# Step 5: Build Docker image
echo ""
echo "=== Step 5: Building Docker image ==="
cd "$SCRIPT_DIR"
./build.sh

# Step 6: Verify installation
echo ""
echo "=== Step 6: Verifying installation ==="
docker run --rm \
    --device=/dev/kvm \
    -v "$HOST_VOLUME:/opt/cuttlefish" \
    cuttlefish-emulator:latest \
    check

echo ""
echo "=== Setup Complete ==="
echo ""
echo "To start Cuttlefish:"
echo "  cd $SCRIPT_DIR"
echo "  docker-compose up -d"
echo ""
echo "To view logs:"
echo "  docker-compose logs -f"
echo ""
echo "To connect via ADB:"
echo "  adb connect localhost:6520"
echo ""
echo "To stop:"
echo "  docker-compose down"
echo ""
echo "Downloaded files cached in: $TEMP_DIR"
echo "You can delete this directory to reclaim ~2GB of space."
