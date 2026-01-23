# WP-DIARIZATION: Speaker Diarization Enhancement

> **Status**: Blocked (Release Phase) | **Priority**: MEDIUM | **Phase**: Release
>
> **Research**: [R-DIA-01](../design/research/R-DIA-01-speaker-diarization.md) (Complete)
>
> **Created**: 2026-01-23 | **Last Updated**: 2026-01-23

---

## Overview

Upgrade speaker diarization from energy-based clustering (MVP) to neural speaker embedding using ECAPA-TDNN. This enables accurate multi-speaker transcription for meetings, interviews, and conversations.

**Current State**: Energy-based clustering provides ~50-60% accuracy, suitable only for speakers with very different volume levels.

**Target State**: ECAPA-TDNN embeddings provide ~85-95% accuracy, distinguishing speakers by voice characteristics.

---

## Scope

### Included (Release Phase)
- ECAPA-TDNN ONNX model integration via `ort` crate
- Speaker embedding extraction (~192 dimensions)
- Agglomerative clustering for speaker assignment
- Model download and verification (extend existing model registry)
- Configurable diarization (enable/disable, max speakers)
- GPU acceleration for embedding extraction

### Beta Enhancement (Optional)
- Improved segment boundaries using Whisper timestamps
- Configurable `min_speaker_gap` parameter
- Combined energy + silence gap clustering

### Excluded
- Real-time streaming diarization (future work)
- Speaker identification/verification (different use case)
- Custom model training

---

## Interface Contract

### Speaker Embedding Module

```rust
// crates/lexicon-core/src/stt/embedding.rs

use ort::Session;
use std::path::Path;

/// ECAPA-TDNN speaker embedding extractor
pub struct SpeakerEmbedding {
    session: Session,
}

impl SpeakerEmbedding {
    /// Load ECAPA-TDNN model from path
    pub fn new(model_path: &Path) -> Result<Self, DiarizationError>;

    /// Extract 192-dimensional embedding from audio chunk
    ///
    /// Input: 16kHz mono audio (1-5 seconds recommended)
    /// Output: 192-dimensional embedding vector
    pub fn embed(&self, audio: &[f32]) -> Result<Vec<f32>, DiarizationError>;

    /// Extract embeddings for multiple segments
    pub fn embed_batch(&self, segments: &[&[f32]]) -> Result<Vec<Vec<f32>>, DiarizationError>;
}

/// Cosine similarity between embeddings (0.0 to 1.0)
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32;
```

### Clustering Module

```rust
// crates/lexicon-core/src/stt/clustering.rs

/// Agglomerative clustering for speaker assignment
pub struct SpeakerClusterer {
    threshold: f32,
    max_speakers: Option<usize>,
}

impl SpeakerClusterer {
    /// Create clusterer with similarity threshold
    ///
    /// threshold: Minimum cosine similarity for same speaker (default: 0.7)
    pub fn new(threshold: f32) -> Self;

    /// Set maximum number of speakers to detect
    pub fn with_max_speakers(self, max: usize) -> Self;

    /// Cluster embeddings and return speaker IDs
    pub fn cluster(&self, embeddings: &[Vec<f32>]) -> Vec<u32>;
}
```

### Enhanced Transcription Config

```rust
// crates/lexicon-core/src/stt/mod.rs

/// Diarization model selection
#[derive(Debug, Clone, Default)]
pub enum DiarizationModel {
    /// Energy-based clustering (fast, low accuracy)
    #[default]
    Energy,
    /// ECAPA-TDNN neural embeddings (accurate, requires model)
    EcapaTdnn,
}

/// Enhanced transcription configuration
impl TranscriptionConfig {
    /// Enable diarization with model selection
    pub fn with_diarization_model(self, model: DiarizationModel) -> Self;
}
```

### Model Registry Extension

```rust
// crates/lexicon-core/src/models/registry.rs

/// Extended model types
pub enum ModelType {
    Whisper,
    Kokoro,
    SpeakerEmbedding,  // NEW
}

// Model manifest entry
// {
//   "id": "speaker-embedding-ecapa",
//   "type": "speaker_embedding",
//   "size_bytes": 20000000,
//   "sha256": "...",
//   "url": "https://huggingface.co/.../ecapa_tdnn.onnx"
// }
```

---

## Acceptance Criteria

### Model Integration
- [ ] ECAPA-TDNN ONNX model added to model registry
- [ ] Model downloads via existing `models download` command
- [ ] Model verified with SHA-256 hash
- [ ] Model loads via `ort` crate without errors

### Embedding Extraction
- [ ] Extracts 192-dimensional embeddings from audio
- [ ] Handles variable-length audio (1-30 seconds)
- [ ] GPU acceleration works on CUDA/Metal when available
- [ ] Embedding extraction <200ms per segment (CPU)

### Clustering
- [ ] Agglomerative clustering assigns speaker IDs
- [ ] Respects `max_speakers` constraint
- [ ] Same-speaker segments get consistent IDs
- [ ] Different-speaker segments get different IDs

### Integration
- [ ] Existing API unchanged for `DiarizationModel::Energy`
- [ ] New `DiarizationModel::EcapaTdnn` opt-in
- [ ] Diarization results in `Segment.speaker_id` field
- [ ] 2-speaker test case achieves >80% accuracy

### Performance
- [ ] <500ms total diarization for 10-minute audio
- [ ] <100MB additional memory for model
- [ ] Works offline (no network after model download)

---

## Dependencies

### Requires
- `ort` crate (already used for Kokoro TTS)
- ECAPA-TDNN ONNX model (~20MB)
- Clustering implementation (pure Rust)
- WP-HASH-VERIFY (for secure model download)

### Blocks
- Release phase completion
- Multi-speaker meeting transcription feature

---

## Files

### New Files
- `crates/lexicon-core/src/stt/embedding.rs` - ECAPA-TDNN integration
- `crates/lexicon-core/src/stt/clustering.rs` - Agglomerative clustering
- `crates/lexicon-core/src/stt/tests/embedding_tests.rs`
- `crates/lexicon-core/src/stt/tests/clustering_tests.rs`

### Modified Files
- `crates/lexicon-core/src/stt/mod.rs` - Add DiarizationModel enum
- `crates/lexicon-core/src/stt/diarization.rs` - Integrate neural approach
- `crates/lexicon-core/src/models/registry.rs` - Add speaker model type

---

## Implementation Notes

### ECAPA-TDNN Model Source

Pre-converted ONNX model from SpeechBrain:
```
https://huggingface.co/speechbrain/spkrec-ecapa-voxceleb
```

Or convert from PyTorch:
```python
# See R-DIA-01 for conversion script
torch.onnx.export(model, dummy_input, "ecapa_tdnn.onnx", ...)
```

### Embedding Workflow

```
Audio Segments → SpeakerEmbedding.embed_batch() → [Vec<f32>; N]
                                                       ↓
                                          SpeakerClusterer.cluster()
                                                       ↓
                                                  [u32; N] (speaker IDs)
```

### Clustering Algorithm

Agglomerative clustering with average linkage:
1. Compute pairwise cosine similarities
2. Build linkage tree (bottom-up merging)
3. Cut tree at similarity threshold or max_speakers
4. Assign cluster labels as speaker IDs

---

## Test Plan

### Unit Tests

| Test | Input | Expected Output |
|------|-------|-----------------|
| `embedding_dimensions` | 1s audio | 192-element vector |
| `same_speaker_high_sim` | Two clips, same speaker | similarity > 0.8 |
| `diff_speaker_low_sim` | Two clips, different speakers | similarity < 0.5 |
| `cluster_two_speakers` | Embeddings from 2 speakers | 2 unique IDs |
| `cluster_max_speakers` | 5 speakers, max=3 | 3 unique IDs |

### Integration Tests

| Test | Scenario | Pass Criteria |
|------|----------|---------------|
| `two_speaker_conversation` | 2-speaker audio | Correct speaker assignment >80% |
| `single_speaker` | Monologue | All segments speaker_id=0 |
| `diarization_disabled` | Config disabled | speaker_id=None for all |
| `energy_vs_neural` | Same audio | Neural more accurate |

### Accuracy Benchmarks

| Test Set | Energy-Based | ECAPA-TDNN Target |
|----------|--------------|-------------------|
| 2-speaker interview | ~60% | >85% |
| 3-speaker meeting | ~40% | >75% |
| Similar-voice speakers | ~50% | >80% |

---

## Research References

- **R-DIA-01**: Speaker diarization algorithm research
- [ECAPA-TDNN Paper](https://arxiv.org/abs/2005.07143)
- [SpeechBrain](https://speechbrain.github.io/)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work package created from R-DIA-01 research |

---

*Blocked until Release phase. Beta enhancement (timestamp-based) can be done independently.*
