#!/bin/bash
# build.sh
# Build the Cuttlefish Docker image

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

IMAGE_NAME="${IMAGE_NAME:-cuttlefish-emulator}"
IMAGE_TAG="${IMAGE_TAG:-ubuntu22.04}"

echo "=== Building Cuttlefish Docker Image ==="
echo "Image: $IMAGE_NAME:$IMAGE_TAG"
echo ""

docker build -t "$IMAGE_NAME:$IMAGE_TAG" -t "$IMAGE_NAME:latest" .

echo ""
echo "=== Build Complete ==="
echo "Image: $IMAGE_NAME:$IMAGE_TAG"
echo ""
echo "Next steps:"
echo "  1. Download Cuttlefish host package"
echo "  2. Download Android system images"
echo "  3. Run: ./setup.sh"
