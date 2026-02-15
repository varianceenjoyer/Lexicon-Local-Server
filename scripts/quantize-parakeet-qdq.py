#!/usr/bin/env python3
"""Re-quantize Parakeet CTC fp32 model to int8 for CPU inference.

The onnx-community int8 models use QOperator format (ConvInteger nodes) which
are not supported by the ONNX Runtime CPU execution provider. This script
re-quantizes using approaches that work on CPU:

1. Dynamic quantization targeting only MatMul ops (avoids ConvInteger entirely)
2. Static quantization with QDQ format (wraps ops with QuantizeLinear/DequantizeLinear)

Usage:
    python scripts/quantize-parakeet-qdq.py [model_dir] [--static]

Requires: pip install onnxruntime onnx numpy
"""

import sys
from pathlib import Path


def quantize_dynamic_matmul(input_model: Path, output_model: Path, output_data: Path):
    """Dynamic quantization targeting only MatMul ops (no Conv → no ConvInteger)."""
    import onnx
    from onnx.external_data_helper import convert_model_to_external_data
    from onnxruntime.quantization import quantize_dynamic, QuantType

    tmp_model = input_model.parent / "_quant_tmp.onnx"

    print("Quantizing MatMul ops only (dynamic int8)...")
    quantize_dynamic(
        model_input=str(input_model),
        model_output=str(tmp_model),
        weight_type=QuantType.QInt8,
        use_external_data_format=True,
        op_types_to_quantize=["MatMul"],
        extra_options={"MatMulConstBOnly": True},
    )

    # Consolidate external data into a single file
    print("Consolidating external data...")
    model = onnx.load(str(tmp_model), load_external_data=True)
    convert_model_to_external_data(
        model,
        all_tensors_to_one_file=True,
        location=output_data.name,
        size_threshold=1024,
        convert_attribute=False,
    )
    onnx.save_model(model, str(output_model))

    for f in input_model.parent.glob("_quant_tmp*"):
        f.unlink()


def quantize_static_qdq(input_model: Path, output_model: Path, output_data: Path,
                        audio_dir: Path):
    """Static quantization with QDQ format using calibration data from WAV files."""
    import numpy as np
    import onnx
    from onnx.external_data_helper import convert_model_to_external_data
    from onnxruntime.quantization import (
        quantize_static, QuantType, QuantFormat, CalibrationDataReader,
    )

    class ParakeetCalibrationReader(CalibrationDataReader):
        """Generates mel-spectrogram features from WAV files for calibration."""
        def __init__(self, model_path: str, wav_dir: Path):
            import onnxruntime as ort
            sess = ort.InferenceSession(model_path)
            self.input_names = [i.name for i in sess.get_inputs()]
            del sess

            # Collect WAV files
            self.wav_files = sorted(wav_dir.glob("*.wav"))
            if not self.wav_files:
                raise ValueError(f"No WAV files found in {wav_dir}")
            self.idx = 0
            print(f"  Calibration: {len(self.wav_files)} WAV files from {wav_dir}")

        def get_next(self):
            if self.idx >= len(self.wav_files):
                return None
            wav_path = self.wav_files[self.idx]
            self.idx += 1

            # Read WAV and create mel features
            samples = self._read_wav(wav_path)
            features = self._extract_features(samples)
            feed = {
                "input_features": features,
                "attention_mask": np.ones(
                    (1, features.shape[1]), dtype=np.int64
                ),
            }
            return feed

        @staticmethod
        def _read_wav(path: Path) -> np.ndarray:
            import wave
            with wave.open(str(path), 'rb') as w:
                frames = w.readframes(w.getnframes())
                samples = np.frombuffer(frames, dtype=np.int16).astype(np.float32) / 32768.0
            return samples

        @staticmethod
        def _extract_features(samples: np.ndarray) -> np.ndarray:
            """Simple mel-spectrogram extraction matching parakeet-rs defaults."""
            n_fft = 512
            hop_length = 160
            win_length = 400
            n_mels = 80
            preemphasis = 0.97
            sr = 16000

            # Preemphasis
            emphasized = np.append(samples[0], samples[1:] - preemphasis * samples[:-1])

            # STFT
            from numpy.lib.stride_tricks import sliding_window_view
            # Pad signal
            pad_len = n_fft // 2
            padded = np.pad(emphasized, (pad_len, pad_len), mode='reflect')
            # Frame
            n_frames = 1 + (len(padded) - n_fft) // hop_length
            window = np.hanning(win_length)
            padded_window = np.zeros(n_fft)
            padded_window[:win_length] = window

            frames = np.stack([
                padded[i * hop_length:i * hop_length + n_fft]
                for i in range(n_frames)
            ])
            frames = frames * padded_window

            # FFT
            spectrum = np.abs(np.fft.rfft(frames, n=n_fft))
            power = spectrum ** 2

            # Mel filterbank
            mel_basis = _mel_filterbank(sr, n_fft, n_mels)
            mel_spec = np.dot(power, mel_basis.T)
            mel_spec = np.maximum(mel_spec, 1e-10)
            log_mel = np.log(mel_spec)

            # Normalize per feature
            mean = log_mel.mean(axis=0, keepdims=True)
            std = log_mel.std(axis=0, keepdims=True) + 1e-5
            log_mel = (log_mel - mean) / std

            return log_mel[np.newaxis, :, :].astype(np.float32)

    def _mel_filterbank(sr, n_fft, n_mels):
        """Create mel filterbank matrix."""
        low_freq = 0.0
        high_freq = sr / 2.0
        low_mel = 2595.0 * np.log10(1.0 + low_freq / 700.0)
        high_mel = 2595.0 * np.log10(1.0 + high_freq / 700.0)
        mel_points = np.linspace(low_mel, high_mel, n_mels + 2)
        hz_points = 700.0 * (10.0 ** (mel_points / 2595.0) - 1.0)
        bin_points = np.floor((n_fft + 1) * hz_points / sr).astype(int)

        filterbank = np.zeros((n_mels, n_fft // 2 + 1))
        for i in range(n_mels):
            for j in range(bin_points[i], bin_points[i + 1]):
                filterbank[i, j] = (j - bin_points[i]) / (bin_points[i + 1] - bin_points[i])
            for j in range(bin_points[i + 1], bin_points[i + 2]):
                filterbank[i, j] = (bin_points[i + 2] - j) / (bin_points[i + 2] - bin_points[i + 1])
        return filterbank

    tmp_model = input_model.parent / "_quant_tmp.onnx"

    print("Quantizing with static int8 QDQ format...")
    reader = ParakeetCalibrationReader(str(input_model), audio_dir)
    quantize_static(
        model_input=str(input_model),
        model_output=str(tmp_model),
        calibration_data_reader=reader,
        quant_format=QuantFormat.QDQ,
        weight_type=QuantType.QInt8,
        activation_type=QuantType.QInt8,
        use_external_data_format=True,
        extra_options={"ActivationSymmetric": True},
    )

    # Consolidate external data
    print("Consolidating external data...")
    model = onnx.load(str(tmp_model), load_external_data=True)
    convert_model_to_external_data(
        model,
        all_tensors_to_one_file=True,
        location=output_data.name,
        size_threshold=1024,
        convert_attribute=False,
    )
    onnx.save_model(model, str(output_model))

    for f in input_model.parent.glob("_quant_tmp*"):
        f.unlink()


def main():
    model_dir = Path(sys.argv[1]) if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else Path("models/parakeet-ctc-0.6b")
    use_static = "--static" in sys.argv
    input_model = model_dir / "model.onnx"
    output_model = model_dir / "model_int8_qdq.onnx"
    output_data = model_dir / "model_int8_qdq.onnx_data"

    if not input_model.exists():
        print(f"Error: {input_model} not found. Download fp32 model first.")
        sys.exit(1)

    print(f"Input:  {input_model}")
    print(f"Output: {output_model}")
    print(f"Method: {'static QDQ' if use_static else 'dynamic (MatMul only)'}")
    print()

    # Clean previous output
    for f in [output_model, output_data]:
        if f.exists():
            f.unlink()

    if use_static:
        audio_dir = Path("tests/fixtures/audio/sherpa")
        if not audio_dir.exists():
            print(f"Error: calibration audio dir not found: {audio_dir}")
            sys.exit(1)
        quantize_static_qdq(input_model, output_model, output_data, audio_dir)
    else:
        quantize_dynamic_matmul(input_model, output_model, output_data)

    # Report
    if output_model.exists():
        model_size = output_model.stat().st_size
        data_size = output_data.stat().st_size if output_data.exists() else 0
        total_mb = (model_size + data_size) / (1024 * 1024)
        print(f"\nQuantized model: {output_model} ({model_size / 1024:.0f}K)")
        if output_data.exists():
            print(f"External data:  {output_data} ({data_size / (1024*1024):.0f}MB)")
        print(f"Total size: {total_mb:.0f}MB")

        # Verify it loads
        print("\nVerifying model loads on CPU...")
        import onnxruntime as ort
        sess = ort.InferenceSession(str(output_model))
        inputs = {i.name: i for i in sess.get_inputs()}
        print(f"  Inputs: {list(inputs.keys())}")
        print(f"  OK - model loads on CPU")
    else:
        print("Error: quantization produced no output")
        sys.exit(1)


if __name__ == "__main__":
    main()
