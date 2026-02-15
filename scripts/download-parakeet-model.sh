#!/bin/bash
# Download NVIDIA Parakeet CTC 0.6B ONNX model for STT
#
# Model: onnx-community/parakeet-ctc-0.6b-ONNX
# Size: ~2.3GB (fp32, recommended) or ~583MB (int8, requires GPU)
# Languages: English
# Architecture: FastConformer CTC
# Source: https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX
#
# NOTE: fp32 is the only variant verified to work on CPU.
# fp16/int8 require GPU execution providers (CUDA, TensorRT, etc.).

set -e

REPO="onnx-community/parakeet-ctc-0.6b-ONNX"
BASE_URL="https://huggingface.co/${REPO}/resolve/main"
DEST="${1:-models/parakeet-ctc-0.6b}"
VARIANT="${2:-fp32}"

echo "=== Parakeet CTC Model Downloader ==="
echo "Model: ${REPO}"
echo "Variant: ${VARIANT}"
echo "Destination: ${DEST}"
echo ""

# Determine which model files to download based on variant
case "${VARIANT}" in
    int8)
        MODEL_FILE="model_int8.onnx"
        DATA_FILE="model_int8.onnx_data"
        EXPECTED_SIZE="~583MB"
        ;;
    fp16)
        MODEL_FILE="model_fp16.onnx"
        DATA_FILE="model_fp16.onnx_data"
        EXPECTED_SIZE="~1.1GB"
        ;;
    q4)
        MODEL_FILE="model_q4.onnx"
        DATA_FILE="model_q4.onnx_data"
        EXPECTED_SIZE="~613MB"
        ;;
    fp32)
        MODEL_FILE="model.onnx"
        DATA_FILE="model.onnx_data"
        EXPECTED_SIZE="~2.3GB"
        ;;
    *)
        echo "Unknown variant: ${VARIANT}"
        echo "Supported: fp32 (default, CPU), fp16 (GPU), int8 (GPU), q4 (GPU)"
        exit 1
        ;;
esac

echo "Model file: onnx/${MODEL_FILE} + onnx/${DATA_FILE} (${EXPECTED_SIZE})"
echo ""

# Create destination directory
mkdir -p "${DEST}"

# Check if already downloaded
if [ -f "${DEST}/${MODEL_FILE}" ] && [ -f "${DEST}/${DATA_FILE}" ] && [ -f "${DEST}/tokenizer.json" ]; then
    echo "Model already downloaded:"
    ls -lh "${DEST}/${MODEL_FILE}" "${DEST}/${DATA_FILE}" "${DEST}/tokenizer.json"
    exit 0
fi

# Download tokenizer.json (required by parakeet-rs)
if [ ! -f "${DEST}/tokenizer.json" ]; then
    echo "Downloading tokenizer.json..."
    curl -L -o "${DEST}/tokenizer.json" "${BASE_URL}/tokenizer.json" --progress-bar
    echo "  Downloaded tokenizer.json"
fi

# Download model ONNX file
if [ ! -f "${DEST}/${MODEL_FILE}" ]; then
    echo "Downloading onnx/${MODEL_FILE}..."
    curl -L -o "${DEST}/${MODEL_FILE}" "${BASE_URL}/onnx/${MODEL_FILE}" --progress-bar
    size=$(du -h "${DEST}/${MODEL_FILE}" | cut -f1)
    echo "  Downloaded ${MODEL_FILE} (${size})"
fi

# Download model data file
if [ ! -f "${DEST}/${DATA_FILE}" ]; then
    echo "Downloading onnx/${DATA_FILE}..."
    curl -L -o "${DEST}/${DATA_FILE}" "${BASE_URL}/onnx/${DATA_FILE}" --progress-bar
    size=$(du -h "${DEST}/${DATA_FILE}" | cut -f1)
    echo "  Downloaded ${DATA_FILE} (${size})"
fi

# Verify all files exist
echo ""
echo "=== Download Complete ==="
echo "Model files in: ${DEST}"
echo ""
echo "Directory contents:"
ls -lh "${DEST}"
echo ""
echo "To use with ParakeetTranscriber:"
echo "  transcriber.load_model(Path::new(\"${DEST}\"))"
echo ""
echo "Variants available: fp32 (default, CPU), fp16 (GPU), int8 (GPU), q4 (GPU)"
echo "  ./scripts/download-parakeet-model.sh models/parakeet-ctc-0.6b fp16"
