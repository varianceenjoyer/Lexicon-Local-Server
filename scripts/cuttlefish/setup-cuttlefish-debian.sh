#!/bin/bash
# setup-cuttlefish-debian.sh
# Builds and installs Cuttlefish Debian packages from google/android-cuttlefish
# This creates the cvdnetwork group and properly sets up system integration
#
# Run with: sudo ./setup-cuttlefish-debian.sh

set -e

echo "=== Cuttlefish Debian Package Setup ==="
echo "This script builds and installs Cuttlefish system packages."
echo "It requires sudo access and will:"
echo "  - Install build dependencies"
echo "  - Clone google/android-cuttlefish"
echo "  - Build Debian packages"
echo "  - Install cuttlefish-base and cuttlefish-user"
echo "  - Add current user to required groups"
echo ""
read -p "Continue? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

WORKDIR="${WORKDIR:-/tmp/cuttlefish-build}"
mkdir -p "$WORKDIR"
cd "$WORKDIR"

echo ""
echo "=== Step 1: Install build dependencies ==="
sudo apt update
sudo apt install -y \
    git \
    devscripts \
    equivs \
    config-package-dev \
    debhelper-compat \
    golang \
    curl

echo ""
echo "=== Step 2: Clone android-cuttlefish repository ==="
if [ -d "$WORKDIR/android-cuttlefish" ]; then
    echo "Repository already exists, pulling latest..."
    cd "$WORKDIR/android-cuttlefish"
    git pull
else
    git clone https://github.com/google/android-cuttlefish
    cd "$WORKDIR/android-cuttlefish"
fi

echo ""
echo "=== Step 3: Build Debian packages ==="
echo "This may take several minutes..."
./tools/buildutils/build_packages.sh

echo ""
echo "=== Step 4: Install packages ==="
sudo dpkg -i ./cuttlefish-base_*_*64.deb || sudo apt-get install -f -y
sudo dpkg -i ./cuttlefish-user_*_*64.deb || sudo apt-get install -f -y

echo ""
echo "=== Step 5: Configure user groups ==="
sudo usermod -aG kvm,cvdnetwork,render "$USER"

echo ""
echo "=== Installation Complete ==="
echo ""
echo "IMPORTANT: You must log out and log back in (or reboot) for group"
echo "changes to take effect."
echo ""
echo "After re-login, verify with:"
echo "  groups \$USER"
echo "  # Should include: kvm cvdnetwork render"
echo ""
echo "Then download Android images and run:"
echo "  cvd fetch --default_build aosp-android-latest-release/aosp_cf_x86_64_phone-userdebug"
echo "  launch_cvd --daemon"
