# WP-TTS-QUALITY: TTS Quality Validation

> **Status**: Ready for Implementation | **Priority**: MEDIUM | **Phase**: Beta
>
> **Research**: [R-TTS-01](../design/research/R-TTS-01-alternative-tts-engines.md)
>
> **Created**: 2026-01-23 | **Last Updated**: 2026-01-23

---

## Overview

Validate that Kokoro TTS produces natural-sounding speech for common English sentences. This is a Beta exit criterion that requires subjective quality assessment and objective metrics validation.

**Current State**: Kokoro TTS integrated with misaki-rs G2P, but quality validation not formally documented.

---

## Scope

### Included
- Subjective listening tests with standardized sentences
- MOS (Mean Opinion Score) estimation methodology
- Phoneme coverage verification
- Edge case handling (numbers, abbreviations, punctuation)
- Comparison with platform TTS fallback
- Documentation of quality baseline

### Excluded
- Automated MOS scoring (requires external services)
- Multi-language TTS quality (English only for Kokoro)
- Voice cloning/customization
- Real-time performance benchmarks (covered by existing benches)

---

## Interface Contract

### Quality Test Suite

```rust
// crates/lexicon-core/src/tts/tests/quality_tests.rs

/// Standard test sentences for TTS quality evaluation
pub const QUALITY_TEST_SENTENCES: &[&str] = &[
    // Basic sentences
    "Hello, how are you today?",
    "The quick brown fox jumps over the lazy dog.",

    // Numbers and dates
    "The meeting is scheduled for January 15th, 2026 at 3:30 PM.",
    "The price is $1,234.56 including tax.",
    "Call me at 555-123-4567.",

    // Abbreviations
    "Dr. Smith works at MIT in the USA.",
    "The CEO of IBM met with the FBI director.",

    // Punctuation and emphasis
    "Wait... what did you say?",
    "This is amazing! I can't believe it!",
    "Should we go left, right, or straight ahead?",

    // Technical terms
    "The API returns JSON data over HTTPS.",
    "Configure the WiFi settings in the iOS app.",

    // Long sentences
    "In order to successfully complete this task, you will need to carefully \
     follow each step of the instructions provided in the documentation.",

    // Difficult words
    "The entrepreneur scheduled a rendezvous at the boutique.",
    "The colonel analyzed the thoroughly sophisticated algorithm.",
];

/// Quality assessment criteria
pub struct QualityAssessment {
    /// Sentence tested
    pub sentence: String,
    /// Intelligibility (1-5): Can words be understood?
    pub intelligibility: u8,
    /// Naturalness (1-5): Does it sound human-like?
    pub naturalness: u8,
    /// Pronunciation (1-5): Are words pronounced correctly?
    pub pronunciation: u8,
    /// Prosody (1-5): Is rhythm/intonation natural?
    pub prosody: u8,
    /// Notes on specific issues
    pub notes: String,
}

impl QualityAssessment {
    /// Calculate overall MOS (Mean Opinion Score)
    pub fn mos(&self) -> f32 {
        (self.intelligibility + self.naturalness + self.pronunciation + self.prosody) as f32 / 4.0
    }

    /// Check if meets minimum quality bar
    pub fn meets_quality_bar(&self) -> bool {
        self.intelligibility >= 4 && self.naturalness >= 3 && self.pronunciation >= 4
    }
}
```

### Quality Test Runner

```rust
// crates/lexicon-cli/src/commands/tts_quality.rs

/// Run TTS quality test suite
pub async fn run_quality_tests(
    output_dir: &Path,
    interactive: bool,
) -> Result<QualityReport, CliError>;

/// Generate audio files for all test sentences
pub async fn generate_test_audio(
    output_dir: &Path,
    synthesizer: &impl Synthesizer,
) -> Result<Vec<PathBuf>, CliError>;

/// Quality report structure
pub struct QualityReport {
    pub timestamp: DateTime<Utc>,
    pub model_version: String,
    pub assessments: Vec<QualityAssessment>,
    pub average_mos: f32,
    pub pass: bool,
}
```

### CLI Command

```
lexicon tts-quality
    Run TTS quality assessment

    OPTIONS:
        --output-dir <DIR>    Directory for generated audio files
        --interactive         Prompt for subjective ratings
        --compare-platform    Also generate with platform TTS for comparison
        --report <FILE>       Output report file (JSON)

lexicon tts-quality generate
    Generate audio files for manual review

lexicon tts-quality report
    Display previous quality reports
```

---

## Acceptance Criteria

### Phoneme Coverage
- [ ] All English phonemes correctly synthesized
- [ ] G2P handles common words without errors
- [ ] Abbreviations expanded correctly (Dr., Mr., USA, etc.)
- [ ] Numbers spoken naturally (ordinals, cardinals, phone numbers)
- [ ] Punctuation affects prosody appropriately

### Quality Metrics
- [ ] Average MOS >= 3.5 across test sentences
- [ ] Intelligibility >= 4.0 for all sentences
- [ ] No sentence scores below 2.0 on any metric
- [ ] Natural-sounding for conversational sentences

### Edge Cases
- [ ] Long sentences don't have unnatural pauses
- [ ] Questions have rising intonation
- [ ] Exclamations have appropriate emphasis
- [ ] Technical terms pronounced reasonably

### Comparison
- [ ] Kokoro quality meets or exceeds platform TTS for English
- [ ] Fallback to platform TTS works when Kokoro fails
- [ ] Quality difference documented

### Documentation
- [ ] Quality baseline documented in BENCHMARKS.md
- [ ] Known limitations listed
- [ ] Sample audio files archived for regression testing

---

## Dependencies

### Requires
- WP-TTS (Kokoro + fallback implemented)
- Audio playback capability for listening tests

### Blocks
- Beta exit criteria completion

---

## Files

### New Files
- `crates/lexicon-core/src/tts/tests/quality_tests.rs` - Test infrastructure
- `crates/lexicon-cli/src/commands/tts_quality.rs` - CLI command
- `docs/TTS-QUALITY.md` - Quality documentation
- `tests/fixtures/tts/quality_baseline/` - Reference audio files

### Modified Files
- `crates/lexicon-cli/src/main.rs` - Add tts-quality subcommand
- `docs/BENCHMARKS.md` - Add TTS quality section

---

## Test Plan

### Automated Tests

| Test | Input | Expected Output |
|------|-------|-----------------|
| `all_test_sentences_synthesize` | Test sentences | Audio files generated, no errors |
| `phoneme_coverage` | Phoneme test words | All phonemes present in output |
| `abbreviation_expansion` | "Dr. Smith" | "Doctor Smith" spoken |
| `number_handling` | "$1,234.56" | Natural number speech |
| `punctuation_prosody` | "Hello?" | Rising intonation |

### Manual Tests

| Test | Procedure | Pass Criteria |
|------|-----------|---------------|
| `listening_test` | Play each audio, rate 1-5 | Average MOS >= 3.5 |
| `comparison_test` | Compare Kokoro vs platform | Kokoro comparable or better |
| `long_text_test` | Synthesize paragraph | No unnatural breaks |

### Quality Checklist

For each test sentence:
- [ ] Words clearly intelligible
- [ ] Pronunciation correct
- [ ] Natural rhythm
- [ ] Appropriate intonation
- [ ] No audio artifacts

---

## Implementation Notes

### MOS Scale

| Score | Description |
|-------|-------------|
| 5 | Excellent - Indistinguishable from human |
| 4 | Good - Natural with minor issues |
| 3 | Fair - Understandable but noticeably synthetic |
| 2 | Poor - Difficult to understand in places |
| 1 | Bad - Unintelligible |

### Quality Targets

| Metric | Minimum | Target |
|--------|---------|--------|
| Intelligibility | 4.0 | 4.5 |
| Naturalness | 3.0 | 3.5 |
| Pronunciation | 4.0 | 4.5 |
| Prosody | 3.0 | 3.5 |
| **Overall MOS** | **3.5** | **4.0** |

### Known Limitations (Document)

- Kokoro is English-only (fallback handles other languages)
- Some technical terms may be mispronounced
- Very long sentences may have prosody issues
- Emotional expression is limited

---

## Research References

- **R-TTS-01**: TTS engine selection and evaluation
- **R-LIC-01**: License audit (Kokoro Apache 2.0)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work package created for Beta exit criteria |

---

*Ready for implementation. Requires manual listening tests for subjective evaluation.*
