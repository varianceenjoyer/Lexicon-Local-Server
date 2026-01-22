# Research Plan: Speaker Diarization Approaches

**ID:** R-DIA-01
**Status:** Complete (Research Phase)
**Category:** Algorithm Selection
**Created:** 2026-01-23
**Last Updated:** 2026-01-23
**Output:** Algorithm comparison and upgrade path

---

## Executive Summary

### Current Implementation

Lexicon uses **energy-based clustering** for speaker diarization (MVP approach):
- Computes average energy (RMS) per audio segment
- Clusters segments by energy threshold
- Assigns speaker IDs based on cluster membership

**Limitations:**
- Only distinguishes loud vs. quiet speakers
- Fails when speakers have similar volume levels
- No speaker embedding/voice characteristic analysis

### Recommended Upgrade Path

| Phase | Approach | Accuracy | Latency | Notes |
|-------|----------|----------|---------|-------|
| MVP (current) | Energy clustering | ~50-60% | <10ms | Simple, fast |
| Beta | Whisper timestamps + pyannote-style VAD | ~70-75% | ~50ms | Moderate complexity |
| Release | Speaker embedding (ECAPA-TDNN) | ~85-95% | ~200ms | State-of-the-art |

### Recommendation: ECAPA-TDNN via ONNX

For Release phase, integrate **ECAPA-TDNN speaker embedding** model:
- Available as ONNX model (~20MB)
- Can use `ort` crate (already a dependency)
- State-of-the-art accuracy for speaker identification
- Works offline, cross-platform

---

## Problem Statement

Speaker diarization ("who spoke when") is essential for:
- Multi-party meeting transcription
- Interview transcription
- Conversation analysis
- Accessibility features

The current energy-based approach is a placeholder that provides basic functionality but cannot reliably distinguish speakers with similar volume levels.

---

## Research Questions

### RQ1: What are the state-of-the-art diarization approaches?

**Findings:**

| Approach | Description | DER (Diarization Error Rate) |
|----------|-------------|------------------------------|
| **Spectral clustering** | Cluster speaker embeddings with spectral methods | 8-15% |
| **UIS-RNN (Google)** | Unbounded interleaved-state RNN | 7-10% |
| **pyannote.audio** | End-to-end neural pipeline | 5-10% |
| **ECAPA-TDNN embeddings** | Extract embeddings, cluster | 5-8% |
| **Whisper X diarization** | Whisper + pyannote fusion | 6-12% |

**Key Finding:** All state-of-the-art approaches use neural speaker embeddings rather than acoustic features like energy.

### RQ2: What speaker embedding models are available?

**Findings:**

| Model | Size | License | ONNX Available | Notes |
|-------|------|---------|----------------|-------|
| **ECAPA-TDNN** | 20MB | Apache 2.0 | Yes | SpeechBrain implementation |
| **ResNet-based** | 25MB | MIT | Yes | Resemblyzer-style |
| **x-vector** | 15MB | Apache 2.0 | Yes | Kaldi-based |
| **TitaNet (NVIDIA)** | 85MB | CC-BY-4.0 | Yes | High accuracy |
| **pyannote embedding** | 17MB | MIT | Partial | PyTorch native |

**Recommendation:** ECAPA-TDNN is the best balance of size, accuracy, and license.

### RQ3: What Rust integration options exist?

**Findings:**

| Option | Pros | Cons | Feasibility |
|--------|------|------|-------------|
| `ort` crate + ONNX model | Already used for Kokoro TTS | Need ONNX model conversion | High |
| `tract` crate | Pure Rust | Less model support | Medium |
| C FFI to speechbrain | Full feature set | Complex build | Low |
| HTTP to pyannote server | Easy integration | Not offline | Not recommended |

**Recommendation:** Use `ort` crate with ECAPA-TDNN ONNX model.

### RQ4: What is the pipeline architecture?

**Findings:**

Standard diarization pipeline:
```
Audio → VAD → Speaker Embedding → Clustering → Alignment → Segments
```

1. **VAD (Voice Activity Detection):** Identify speech regions
2. **Speaker Embedding:** Extract fixed-size vector per speech segment
3. **Clustering:** Group embeddings by speaker (spectral, agglomerative, or HDBSCAN)
4. **Alignment:** Assign speaker IDs to transcription segments

**Integration with Whisper:**
- Whisper provides word/segment timestamps
- Apply speaker embedding to each segment
- Cluster embeddings to assign speaker IDs
- No need for separate VAD (Whisper handles this)

---

## Implementation Approaches

### Approach 1: Energy-Based Clustering (Current)

```rust
// Current implementation in diarization.rs
fn cluster_by_energy(segments: &[Segment]) -> Vec<SpeakerId> {
    let energies: Vec<f32> = segments.iter()
        .map(|s| calculate_rms(&s.audio))
        .collect();

    let threshold = median(&energies);
    energies.iter()
        .map(|e| if *e > threshold { 0 } else { 1 })
        .collect()
}
```

**Pros:**
- Very fast (<10ms)
- No additional models needed
- Simple implementation

**Cons:**
- Low accuracy (~50-60%)
- Fails with similar-volume speakers
- Not suitable for production use

### Approach 2: Whisper-Based Timestamp Clustering (Beta Candidate)

Use Whisper's word-level timestamps to improve segment boundaries:

```rust
fn cluster_by_timestamps(segments: &[Segment]) -> Vec<SpeakerId> {
    // Group segments by silence gaps
    // Assume speaker change on long pauses (>500ms)
    // Combine with energy for within-pause assignments
}
```

**Pros:**
- Leverages existing Whisper integration
- Better segment boundaries
- Moderate complexity

**Cons:**
- Still no voice characteristic analysis
- Accuracy limited to ~70-75%

### Approach 3: ECAPA-TDNN Speaker Embedding (Release Target)

Full neural speaker diarization:

```rust
use ort::{Session, Value};

struct SpeakerEmbedding {
    session: Session,
}

impl SpeakerEmbedding {
    fn new(model_path: &Path) -> Result<Self> {
        let session = Session::builder()?
            .with_model_from_file(model_path)?;
        Ok(Self { session })
    }

    fn embed(&self, audio: &[f32]) -> Result<Vec<f32>> {
        // Input: 16kHz mono audio chunk (1-5 seconds)
        // Output: 192-dimensional embedding vector
        let input = Value::from_array((1, audio.len()), audio)?;
        let outputs = self.session.run(vec![input])?;
        Ok(outputs[0].try_extract()?.into_owned())
    }
}

fn cluster_embeddings(embeddings: Vec<Vec<f32>>, max_speakers: usize) -> Vec<SpeakerId> {
    // Spectral clustering or agglomerative clustering
    // Use cosine similarity between embeddings
}
```

**Pros:**
- State-of-the-art accuracy (85-95%)
- Works with any speaker combination
- Consistent with Kokoro TTS architecture (ONNX)

**Cons:**
- Additional model download (~20MB)
- Higher latency (~200ms per segment)
- More complex implementation

---

## Model Acquisition

### ECAPA-TDNN ONNX Model

**Source:** SpeechBrain model zoo or Hugging Face

**Conversion from PyTorch:**
```python
import torch
import speechbrain
from speechbrain.pretrained import EncoderClassifier

# Load pretrained model
classifier = EncoderClassifier.from_hparams(
    source="speechbrain/spkrec-ecapa-voxceleb"
)

# Export to ONNX
dummy_input = torch.randn(1, 16000)  # 1 second of audio
torch.onnx.export(
    classifier.mods.embedding_model,
    dummy_input,
    "ecapa_tdnn.onnx",
    input_names=["audio"],
    output_names=["embedding"],
    dynamic_axes={"audio": {1: "audio_length"}}
)
```

**Pre-converted models available:**
- https://huggingface.co/speechbrain/spkrec-ecapa-voxceleb
- ONNX exports in community repos

### Model Manifest Integration

Add to model registry alongside Whisper and Kokoro models:
```json
{
  "id": "speaker-embedding-ecapa",
  "type": "diarization",
  "size_bytes": 20000000,
  "sha256": "...",
  "files": ["ecapa_tdnn.onnx"]
}
```

---

## Clustering Algorithms

### Agglomerative Clustering

```rust
fn agglomerative_cluster(
    embeddings: &[Vec<f32>],
    threshold: f32,
    max_speakers: usize
) -> Vec<usize> {
    // 1. Compute pairwise cosine distances
    // 2. Build linkage tree (average linkage)
    // 3. Cut tree at threshold or max_speakers
}
```

**Pros:**
- Deterministic
- Works well with unknown speaker count
- Standard in diarization

**Cons:**
- O(n²) complexity
- Fixed threshold sensitivity

### Spectral Clustering

```rust
fn spectral_cluster(
    embeddings: &[Vec<f32>],
    n_clusters: usize
) -> Vec<usize> {
    // 1. Build affinity matrix (cosine similarity)
    // 2. Compute graph Laplacian
    // 3. Find eigenvectors
    // 4. K-means on eigenvector space
}
```

**Pros:**
- Handles complex cluster shapes
- Used in pyannote.audio

**Cons:**
- Requires known cluster count or estimation
- More complex implementation

### Recommendation: Agglomerative Clustering

For the initial ECAPA-TDNN integration, use agglomerative clustering:
- Simpler to implement
- Handles unknown speaker count naturally
- Good enough accuracy for most use cases

---

## Performance Considerations

### Latency Budget

| Operation | Target Latency | Notes |
|-----------|----------------|-------|
| Audio segmentation | <10ms | Already done by Whisper |
| Speaker embedding | <200ms per segment | GPU: <50ms |
| Clustering | <50ms for 100 segments | O(n²) acceptable |
| Total diarization | <500ms for 10-min audio | Acceptable for batch |

### Memory Usage

| Component | Memory | Notes |
|-----------|--------|-------|
| ECAPA-TDNN model | ~80MB loaded | Shared across segments |
| Embeddings (100 segments) | ~80KB | 192 floats × 100 |
| Distance matrix | ~40KB | 100×100 floats |
| Total additional | ~80MB | Dominated by model |

### Streaming Considerations

For real-time diarization:
1. Buffer 3-5 seconds of audio per speaker embedding
2. Incrementally update clustering
3. May need to re-assign early segments as more data arrives

---

## Test Strategy

### Unit Tests

```rust
#[test]
fn test_embedding_dimensions() {
    let embedder = SpeakerEmbedding::new("ecapa_tdnn.onnx")?;
    let audio = vec![0.0f32; 16000]; // 1 second
    let embedding = embedder.embed(&audio)?;
    assert_eq!(embedding.len(), 192);
}

#[test]
fn test_same_speaker_similarity() {
    // Two clips from same speaker should have high similarity
    let emb1 = embedder.embed(&speaker_a_clip_1)?;
    let emb2 = embedder.embed(&speaker_a_clip_2)?;
    let similarity = cosine_similarity(&emb1, &emb2);
    assert!(similarity > 0.8);
}

#[test]
fn test_different_speaker_dissimilarity() {
    // Clips from different speakers should have low similarity
    let emb1 = embedder.embed(&speaker_a_clip)?;
    let emb2 = embedder.embed(&speaker_b_clip)?;
    let similarity = cosine_similarity(&emb1, &emb2);
    assert!(similarity < 0.5);
}
```

### Integration Tests

```rust
#[test]
fn test_two_speaker_conversation() {
    let audio = load_fixture("2_speaker_conversation.wav");
    let segments = transcribe_with_diarization(&audio);

    let speaker_ids: HashSet<_> = segments.iter()
        .filter_map(|s| s.speaker_id)
        .collect();

    assert_eq!(speaker_ids.len(), 2);
}
```

---

## Upgrade Path

### Phase 1: MVP (Current)
- Energy-based clustering
- Suitable for very different speakers
- No additional dependencies

### Phase 2: Beta Enhancement
- Improve segment boundaries using Whisper timestamps
- Add configurable `min_speaker_gap` parameter
- Optional: Basic energy + silence clustering

### Phase 3: Release (ECAPA-TDNN)
1. Add ECAPA-TDNN ONNX model to model registry
2. Implement `SpeakerEmbedding` module
3. Add agglomerative clustering
4. Integrate with existing diarization interface
5. Make neural diarization opt-in (for battery/performance)

### Migration Guide

```rust
// Current API (unchanged)
let config = TranscriptionConfig::default()
    .with_diarization(true)
    .with_max_speakers(2);

let result = transcriber.transcribe(&audio, &config)?;

// New: Optional neural diarization (Release)
let config = TranscriptionConfig::default()
    .with_diarization(true)
    .with_diarization_model(DiarizationModel::EcapaTdnn)  // New
    .with_max_speakers(2);
```

---

## Dependencies

### Existing
- `ort` crate (ONNX Runtime) - already used for Kokoro TTS

### New (Release phase)
- ECAPA-TDNN ONNX model (~20MB download)
- Clustering implementation (pure Rust, no external crate needed)

---

## References

- [ECAPA-TDNN Paper](https://arxiv.org/abs/2005.07143)
- [SpeechBrain Speaker Recognition](https://speechbrain.github.io/)
- [pyannote.audio](https://github.com/pyannote/pyannote-audio)
- [Whisper X](https://github.com/m-bain/whisperX) - Whisper + diarization
- [Speaker Diarization Overview (Google)](https://arxiv.org/abs/2101.09624)
- [VoxCeleb Dataset](https://www.robots.ox.ac.uk/~vgg/data/voxceleb/)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Initial research plan created |
| 2026-01-23 | Documented current energy-based approach |
| 2026-01-23 | Researched ECAPA-TDNN and clustering options |
| 2026-01-23 | Defined upgrade path for Release phase |
