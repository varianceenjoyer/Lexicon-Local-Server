#!/bin/bash
# Download sherpa-onnx streaming Zipformer model for low-latency STT
# See WP-SHERPA-STT for details

set -e

# Model: sherpa-onnx-streaming-zipformer-en-2023-06-26
# Size: ~68MB (int8 quantized)
# Languages: English
# Architecture: Streaming Zipformer (transducer)

MODEL_NAME="sherpa-onnx-streaming-zipformer-en-2023-06-26"
MODEL_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/${MODEL_NAME}.tar.bz2"
DEST="${1:-models/sherpa}"
ARCHIVE="${DEST}/${MODEL_NAME}.tar.bz2"

echo "=== Sherpa Model Downloader ==="
echo "Model: ${MODEL_NAME}"
echo "Destination: ${DEST}"
echo ""

# Create destination directory
mkdir -p "${DEST}"

# Check if already extracted
if [ -d "${DEST}/${MODEL_NAME}" ]; then
    echo "✓ Model already downloaded: ${DEST}/${MODEL_NAME}"
    ls -lh "${DEST}/${MODEL_NAME}"
    exit 0
fi

# Download archive
if [ ! -f "${ARCHIVE}" ]; then
    echo "Downloading ${MODEL_NAME}.tar.bz2..."
    curl -L -o "${ARCHIVE}" "${MODEL_URL}" --progress-bar

    if [ ! -f "${ARCHIVE}" ]; then
        echo "✗ Failed to download archive"
        exit 1
    fi

    size=$(du -h "${ARCHIVE}" | cut -f1)
    echo "✓ Downloaded archive (${size})"
else
    echo "✓ Archive already exists: ${ARCHIVE}"
fi

# Extract archive
echo "Extracting ${MODEL_NAME}.tar.bz2..."
cd "${DEST}"
tar xjf "${MODEL_NAME}.tar.bz2"

# Verify extraction
if [ -d "${MODEL_NAME}" ]; then
    echo ""
    echo "=== Download Complete ==="
    echo "Model files are in: ${DEST}/${MODEL_NAME}"
    echo ""
    echo "Directory contents:"
    ls -lh "${MODEL_NAME}"
    echo ""
    echo "To use with SherpaTranscriber:"
    echo "  transcriber.load_model(Path::new(\"${DEST}/${MODEL_NAME}\"))"

    # Optionally remove archive to save space
    # rm -f "${MODEL_NAME}.tar.bz2"
else
    echo "✗ Extraction failed"
    exit 1
fi
