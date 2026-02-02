//! Sherpa STT Accuracy Workbench
//!
//! Tests transcription accuracy against reference transcriptions.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example sherpa_accuracy --features stt-sherpa,audio-convert
//! ```

use std::path::PathBuf;

use lexicon_core::audio::AudioConverter;
use lexicon_core::stt::{SherpaTranscriber, TranscriptionConfig, Transcriber};

/// Test case with audio file and expected transcription.
struct TestCase {
    name: &'static str,
    file: &'static str,
    expected: &'static str,
}

/// Calculate Word Error Rate (WER) between reference and hypothesis.
///
/// WER = (Substitutions + Insertions + Deletions) / Reference Words
fn calculate_wer(reference: &str, hypothesis: &str) -> (f32, usize, usize, usize, usize) {
    let ref_lower = reference.to_lowercase();
    let hyp_lower = hypothesis.to_lowercase();
    let ref_words: Vec<&str> = ref_lower.split_whitespace().collect();
    let hyp_words: Vec<&str> = hyp_lower.split_whitespace().collect();

    let ref_len = ref_words.len();
    let hyp_len = hyp_words.len();

    // Dynamic programming for Levenshtein distance
    let mut dp = vec![vec![0usize; hyp_len + 1]; ref_len + 1];

    // Initialize base cases
    for i in 0..=ref_len {
        dp[i][0] = i;
    }
    for j in 0..=hyp_len {
        dp[0][j] = j;
    }

    // Fill the DP table
    for i in 1..=ref_len {
        for j in 1..=hyp_len {
            let cost = if ref_words[i - 1] == hyp_words[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1) // deletion
                .min(dp[i][j - 1] + 1) // insertion
                .min(dp[i - 1][j - 1] + cost); // substitution
        }
    }

    let edit_distance = dp[ref_len][hyp_len];

    // Backtrack to count S, I, D
    let mut i = ref_len;
    let mut j = hyp_len;
    let mut substitutions = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && ref_words[i - 1] == hyp_words[j - 1] {
            i -= 1;
            j -= 1;
        } else if i > 0 && j > 0 && dp[i][j] == dp[i - 1][j - 1] + 1 {
            substitutions += 1;
            i -= 1;
            j -= 1;
        } else if j > 0 && dp[i][j] == dp[i][j - 1] + 1 {
            insertions += 1;
            j -= 1;
        } else if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            deletions += 1;
            i -= 1;
        } else {
            break;
        }
    }

    let wer = if ref_len > 0 {
        edit_distance as f32 / ref_len as f32
    } else {
        0.0
    };

    (wer, edit_distance, substitutions, insertions, deletions)
}

/// Normalize text for comparison (lowercase, remove punctuation).
fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    println!("=== Sherpa STT Accuracy Workbench ===\n");

    // Define test cases from accuracy_check.md
    let test_cases = vec![
        TestCase {
            name: "1089-134686-0001",
            file: "1089-134686-0001.wav",
            expected: "After early nightfall the yellow lamps would light up here and there the squalid quarter of the brothels.",
        },
        TestCase {
            name: "1221-135766-0001",
            file: "1221-135766-0001.wav",
            expected: "God, as a direct consequence of the sin which man thus punished, had given her a lovely child, whose place was on that same dishonored bosom, to connect her parent forever with the race and descent of mortals, and to be finally a blessed soul in heaven.",
        },
        TestCase {
            name: "1221-135766-0002",
            file: "1221-135766-0002.wav",
            expected: "Yet these thoughts affected Hester Prynne less with hope than apprehension.",
        },
    ];

    // Setup paths
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root")
        .to_path_buf();

    let model_dir = workspace_root.join("models/sherpa-offline/sherpa-onnx-zipformer-gigaspeech-2023-12-12");
    let fixtures_dir = workspace_root.join("tests/fixtures/audio/sherpa");

    // Check model exists
    if !model_dir.exists() {
        eprintln!("ERROR: Model not found at {:?}", model_dir);
        eprintln!("\nDownload with:");
        eprintln!("  cd models/sherpa-offline");
        eprintln!("  wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-zipformer-gigaspeech-2023-12-12.tar.bz2");
        eprintln!("  tar xjf sherpa-onnx-zipformer-gigaspeech-2023-12-12.tar.bz2");
        return Ok(());
    }

    // Load model
    println!("Loading model from {:?}...", model_dir);
    let mut transcriber = SherpaTranscriber::new()?;
    transcriber.load_model(&model_dir)?;
    println!("Model loaded successfully.\n");

    // Warm up model
    println!("Warming up model...");
    transcriber.warm_up()?;
    println!();

    // Audio converter
    let converter = AudioConverter::new();
    let config = TranscriptionConfig::new();

    // Results storage
    let mut total_wer = 0.0;
    let mut total_tests = 0;

    println!("{:-<80}", "");
    println!("{:<25} {:>10} {:>8} {:>8} {:>8} {:>8}",
             "Test Case", "WER", "Edits", "Sub", "Ins", "Del");
    println!("{:-<80}", "");

    for test in &test_cases {
        let audio_path = fixtures_dir.join(test.file);

        if !audio_path.exists() {
            println!("{:<25} SKIPPED - file not found", test.name);
            continue;
        }

        // Load and convert audio
        let samples = converter.convert_to_samples(&audio_path)?;
        let duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;

        // Transcribe
        let start = std::time::Instant::now();
        let result = transcriber.transcribe(&samples, &config)?;
        let elapsed_ms = start.elapsed().as_millis();

        // Get transcription text
        let hypothesis = result.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Calculate WER
        let normalized_expected = normalize_text(test.expected);
        let normalized_hypothesis = normalize_text(&hypothesis);
        let (wer, edits, subs, ins, dels) = calculate_wer(&normalized_expected, &normalized_hypothesis);

        total_wer += wer;
        total_tests += 1;

        println!("{:<25} {:>9.1}% {:>8} {:>8} {:>8} {:>8}",
                 test.name, wer * 100.0, edits, subs, ins, dels);

        // Print detailed comparison
        println!("  Expected:   {}", &normalized_expected);
        println!("  Got:        {}", &normalized_hypothesis);
        println!("  Audio: {}ms, Inference: {}ms, RTF: {:.3}",
                 duration_ms, elapsed_ms, result.real_time_factor());
        println!();
    }

    println!("{:-<80}", "");

    if total_tests > 0 {
        let avg_wer = total_wer / total_tests as f32;
        println!("\nAverage WER: {:.1}%", avg_wer * 100.0);

        // Accuracy assessment
        if avg_wer < 0.10 {
            println!("Assessment: EXCELLENT (WER < 10%)");
        } else if avg_wer < 0.20 {
            println!("Assessment: GOOD (WER < 20%)");
        } else if avg_wer < 0.30 {
            println!("Assessment: ACCEPTABLE (WER < 30%)");
        } else {
            println!("Assessment: NEEDS IMPROVEMENT (WER >= 30%)");
        }
    }

    Ok(())
}
