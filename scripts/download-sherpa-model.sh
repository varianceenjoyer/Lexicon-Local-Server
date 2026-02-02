#!/bin/bash
# Download sherpa-onnx streaming Zipformer model for low-latency STT
# See WP-SHERPA-STT for details

set -e

# Model: sherpa-onnx-streaming-zipformer-en-20M-2023-06-26
# Size: ~60MB total
# Languages: English
# Architecture: Streaming Zipformer (transducer)

MODEL_NAME="sherpa-onnx-streaming-zipformer-en-20M-2023-06-26"
MODEL_URL="https://huggingface.co/csukuangfj/${MODEL_NAME}/resolve/main"
DEST="${1:-models/sherpa}"

echo "=== Sherpa Model Downloader ==="
echo "Model: ${MODEL_NAME}"
echo "Destination: ${DEST}"
echo ""

# Create destination directory
mkdir -p "${DEST}"

# Files to download
FILES=(
    "encoder-epoch-99-avg-1.int8.onnx"
    "decoder-epoch-99-avg-1.onnx"
    "joiner-epoch-99-avg-1.onnx"
    "tokens.txt"
)

# Download each file
for file in "${FILES[@]}"; do
    dest_file="${DEST}/${file}"

    if [ -f "${dest_file}" ]; then
        echo "✓ ${file} already exists, skipping"
        continue
    fi

    echo "Downloading ${file}..."
    curl -L -o "${dest_file}" "${MODEL_URL}/${file}" --progress-bar

    if [ -f "${dest_file}" ]; then
        size=$(du -h "${dest_file}" | cut -f1)
        echo "✓ Downloaded ${file} (${size})"
    else
        echo "✗ Failed to download ${file}"
        exit 1
    fi
done

echo ""
echo "=== Download Complete ==="
echo "Model files are in: ${DEST}"
echo ""
echo "Directory contents:"
ls -lh "${DEST}"
echo ""
echo "To use with SherpaTranscriber:"
echo "  transcriber.load_model(Path::new(\"${DEST}\"))"
