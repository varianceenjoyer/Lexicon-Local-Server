# Test Fixtures

This directory contains test fixtures shared across all test suites.

## Directory Structure

```
tests/fixtures/
├── audio/                  # Audio test files (generated)
│   ├── silence_10s.wav     # 10 seconds of silence (VAD testing)
│   ├── speech_like_5s.wav  # Synthetic speech-like audio
│   ├── sine_440hz_2s.wav   # 440Hz sine wave (pipeline testing)
│   └── noise_1s.wav        # White noise
├── models/                 # Model files (CI cache, not in repo)
│   └── .gitkeep
├── manifests/              # Model manifest test files
│   ├── valid_manifest.json     # Valid manifest structure
│   ├── invalid_signature.json  # Manifest with bad signature
│   └── tampered_content.json   # Modified content (signature mismatch)
└── generate_audio.rs       # Script to generate audio fixtures
```

## Generating Audio Fixtures

Audio fixtures are generated programmatically to avoid licensing issues and
keep the repository small. Run:

```bash
# From project root
cargo run --example generate_fixtures
```

Or compile and run the generator directly:

```bash
rustc tests/fixtures/generate_audio.rs -o /tmp/gen_audio && /tmp/gen_audio
```

## Manifest Fixtures

The manifest fixtures test model signature verification:

| File | Purpose |
|------|---------|
| `valid_manifest.json` | Well-formed manifest with test models |
| `invalid_signature.json` | Manifest with obviously invalid signature |
| `tampered_content.json` | Content modified after signing (checksum mismatch) |

## Notes

- Audio fixtures are synthetic (generated, not real speech recordings)
- Large audio files (>10MB) should use Git LFS if needed
- Model files are cached in CI and not committed to the repository
- Real speech recordings require proper licensing (CC0, MIT, etc.)

## Provenance

- **Audio files**: Programmatically generated using `generate_audio.rs`
- **Manifest files**: Created for testing, signatures are test values
- **Model files**: Downloaded at runtime from official sources
