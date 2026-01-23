# Handoff: WP-TTS-QUALITY Implementation

> **ID**: HO-TTS-QUALITY-001 | **Work Package**: WP-TTS-QUALITY
>
> **Created**: 2026-01-23 | **Status**: 🟡 INFRASTRUCTURE COMPLETE - Awaiting Human Testing

---

## Summary

Validate Kokoro TTS produces natural-sounding speech. This is a **Beta exit criterion** requiring both automated tests and manual listening evaluation.

---

## Current Progress

### Completed (2026-01-23)
- [x] Test infrastructure: `crates/lexicon-core/src/tts/quality.rs`
  - `QUALITY_TEST_SENTENCES` - 15 standardized test sentences
  - `QualityAssessment` - Individual sentence scoring with MOS
  - `QualityReport` - Aggregate report with beta criteria validation
  - `mos_scale` module - Score descriptions
  - 8 unit tests passing
- [x] CLI commands: `lexicon tts-quality generate|sentences|report`
  - Generate WAV files for all test sentences
  - Support Kokoro and platform TTS comparison
  - Display report from JSON file

### Remaining (Requires Human)
- [ ] **HUMAN ACTION**: Generate audio files and perform listening tests
- [ ] Document quality baseline in `docs/TTS-QUALITY.md`
- [ ] Archive reference audio files

---

## Quick Start

1. **Generate test audio**:
   ```bash
   cargo run -p lexicon-cli -- tts-quality generate --output-dir ./tts-quality-test
   ```

2. **Listen and rate** each audio file on 1-5 scale

3. **Create quality report** JSON and verify `meets_beta_criteria()`

4. **Document results** in `docs/TTS-QUALITY.md`

---

## Implementation Order

### Phase 1: Test Infrastructure (1 hour)

1. Create quality test module in lexicon-core:

```rust
// crates/lexicon-core/src/tts/tests/quality_tests.rs

pub const QUALITY_TEST_SENTENCES: &[&str] = &[
    // Basic
    "Hello, how are you today?",
    "The quick brown fox jumps over the lazy dog.",

    // Numbers
    "The meeting is scheduled for January 15th, 2026 at 3:30 PM.",
    "The price is $1,234.56 including tax.",

    // Abbreviations
    "Dr. Smith works at MIT in the USA.",
    "The CEO of IBM met with the FBI director.",

    // Punctuation
    "Wait... what did you say?",
    "This is amazing! I can't believe it!",
    "Should we go left, right, or straight ahead?",

    // Technical
    "The API returns JSON data over HTTPS.",

    // Long sentence
    "In order to successfully complete this task, you will need to carefully \
     follow each step of the instructions provided in the documentation.",

    // Difficult words
    "The entrepreneur scheduled a rendezvous at the boutique.",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::{KokoroSynthesizer, Synthesizer};

    #[test]
    fn all_test_sentences_synthesize() {
        let synth = KokoroSynthesizer::new_test().expect("Failed to create synthesizer");

        for sentence in QUALITY_TEST_SENTENCES {
            let result = synth.synthesize(sentence);
            assert!(result.is_ok(), "Failed to synthesize: {}", sentence);

            let audio = result.unwrap();
            assert!(!audio.samples.is_empty(), "Empty audio for: {}", sentence);
        }
    }

    #[test]
    fn abbreviations_expanded() {
        // Verify G2P expands abbreviations
        // Check phoneme output contains expanded forms
    }

    #[test]
    fn numbers_spoken_naturally() {
        // Verify numbers converted to words
    }
}
```

### Phase 2: CLI Command (1 hour)

Add `tts-quality` subcommand to generate audio for manual review:

```rust
// crates/lexicon-cli/src/commands/tts_quality.rs

use std::path::Path;
use crate::tts::{KokoroSynthesizer, Synthesizer};
use crate::tts::tests::quality_tests::QUALITY_TEST_SENTENCES;

pub async fn generate_test_audio(output_dir: &Path) -> Result<(), CliError> {
    std::fs::create_dir_all(output_dir)?;

    let synth = KokoroSynthesizer::new()?;

    for (i, sentence) in QUALITY_TEST_SENTENCES.iter().enumerate() {
        println!("Generating {}/{}: {}", i + 1, QUALITY_TEST_SENTENCES.len(), sentence);

        let audio = synth.synthesize(sentence)?;

        let filename = format!("{:02}_{}.wav", i + 1,
            sentence.chars().take(30).collect::<String>()
                .replace(|c: char| !c.is_alphanumeric(), "_"));

        let path = output_dir.join(&filename);
        save_wav(&path, &audio.samples, 24000)?;

        println!("  Saved: {}", path.display());
    }

    Ok(())
}

pub async fn run_quality_report(output_dir: &Path, interactive: bool) -> Result<(), CliError> {
    if interactive {
        println!("\n=== TTS Quality Assessment ===\n");
        println!("Listen to each audio file and rate 1-5:");
        println!("  5 = Excellent (indistinguishable from human)");
        println!("  4 = Good (natural with minor issues)");
        println!("  3 = Fair (understandable but synthetic)");
        println!("  2 = Poor (difficult to understand)");
        println!("  1 = Bad (unintelligible)\n");

        // Interactive rating loop
        for (i, sentence) in QUALITY_TEST_SENTENCES.iter().enumerate() {
            println!("\n[{}/{}] \"{}\"", i + 1, QUALITY_TEST_SENTENCES.len(), sentence);
            println!("Rate intelligibility (1-5): ");
            // Read input...
        }
    }

    Ok(())
}
```

Add to CLI main:

```rust
// crates/lexicon-cli/src/main.rs

#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...

    /// TTS quality assessment
    TtsQuality {
        #[command(subcommand)]
        action: TtsQualityAction,
    },
}

#[derive(Subcommand)]
enum TtsQualityAction {
    /// Generate test audio files
    Generate {
        /// Output directory
        #[arg(short, long, default_value = "tts_quality_test")]
        output: PathBuf,
    },
    /// Run interactive quality assessment
    Assess {
        /// Audio files directory
        #[arg(short, long, default_value = "tts_quality_test")]
        input: PathBuf,
    },
}
```

### Phase 3: Manual Testing (1-2 hours)

1. Generate test audio:
```bash
cargo run -p lexicon-cli -- tts-quality generate --output ./tts_test
```

2. Listen to each file and rate using the MOS scale:

| File | Intelligibility | Naturalness | Pronunciation | Prosody | Notes |
|------|-----------------|-------------|---------------|---------|-------|
| 01_Hello_how_are.wav | ? | ? | ? | ? | |
| 02_The_quick_brown.wav | ? | ? | ? | ? | |
| ... | | | | | |

3. Calculate averages and verify >= 3.5 MOS

### Phase 4: Documentation (30 min)

Create quality documentation:

```markdown
// docs/TTS-QUALITY.md

# TTS Quality Baseline

## Test Date: 2026-01-XX

## Model: Kokoro v0.19

## Results Summary

| Metric | Score |
|--------|-------|
| Average MOS | X.X |
| Intelligibility | X.X |
| Naturalness | X.X |
| Pronunciation | X.X |
| Prosody | X.X |

## Known Limitations

- [List any pronunciation issues]
- [List prosody issues]

## Sample Audio

Reference audio files archived at: tests/fixtures/tts/quality_baseline/
```

---

## Key Files

| File | Purpose |
|------|---------|
| `crates/lexicon-core/src/tts/tests/quality_tests.rs` | Test sentences and unit tests |
| `crates/lexicon-cli/src/commands/tts_quality.rs` | CLI command |
| `docs/TTS-QUALITY.md` | Quality documentation |

---

## Testing Commands

```bash
# View test sentences
cargo run -p lexicon-cli -- tts-quality sentences

# Generate test audio (Kokoro)
cargo run -p lexicon-cli -- tts-quality generate --output-dir ./tts-quality-test

# Generate test audio (platform TTS for comparison)
cargo run -p lexicon-cli -- tts-quality generate --output-dir ./tts-quality-platform --platform

# Run unit tests
cargo test -p lexicon-core tts::quality

# View a quality report
cargo run -p lexicon-cli -- tts-quality report my-report.json

# List generated files
ls -la ./tts-quality-test/
```

---

## Acceptance Checklist

- [x] Quality infrastructure implemented (`quality.rs`)
- [x] CLI commands implemented (`tts-quality generate|sentences|report`)
- [x] 8 unit tests passing
- [ ] **HUMAN**: Audio files generated for manual review
- [ ] **HUMAN**: Manual listening test completed
- [ ] **HUMAN**: Average MOS >= 3.5 verified
- [ ] **HUMAN**: No sentence scores below 2.0 verified
- [ ] **HUMAN**: Quality documentation created (docs/TTS-QUALITY.md)
- [ ] **HUMAN**: Known limitations documented
- [ ] **HUMAN**: Reference audio archived

---

## MOS Rating Guide

| Score | Label | Description |
|-------|-------|-------------|
| 5 | Excellent | Indistinguishable from human speech |
| 4 | Good | Natural with only minor issues |
| 3 | Fair | Clearly synthetic but understandable |
| 2 | Poor | Some difficulty understanding |
| 1 | Bad | Unintelligible |

**Target**: Average MOS >= 3.5, no individual score < 2.0

---

## Notes

- This requires subjective human evaluation
- Keep test audio files for regression testing
- Compare with platform TTS if scores are borderline
- Document any sentences that need improvement

---

*This handoff includes manual listening tests. Allow 1-2 hours for full evaluation.*
