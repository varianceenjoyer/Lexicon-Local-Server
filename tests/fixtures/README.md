# Test Fixtures

This directory contains test fixtures shared across all test suites.

## Directory Structure

```
tests/fixtures/
├── audio/                  # Audio test files
│   ├── 5_second_english.wav
│   ├── 30_second_multilingual.wav
│   ├── 2_speaker_conversation.wav
│   ├── 30_minute_lecture.wav
│   └── silence_10s.wav
├── models/                 # Model files (CI cache, not in repo)
│   └── tiny-q5_1.bin
└── manifests/              # Model manifest test files
    ├── valid_signed.json
    ├── invalid_signature.json
    └── tampered_content.json
```

## Notes

- Audio fixtures should be small enough to include in the repository
- Large audio files (>10MB) should use Git LFS
- Model files are cached in CI and not committed to the repository
- Manifest fixtures test signature verification logic
