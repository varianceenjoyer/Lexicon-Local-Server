//! Lexicon Command-Line Interface
//!
//! Provides commands for:
//! - `lexicon transcribe <file>` - Batch transcription
//! - `lexicon serve` - Start HTTP server
//! - `lexicon models list|download|delete` - Model management
//! - `lexicon export <session> --format srt` - Export transcriptions
//! - `lexicon tui` - Diagnostic TUI for real-time transcription

mod tui;

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use indicatif::{ProgressBar, ProgressStyle};

use lexicon_core::{
    LexiconConfig, ModelRegistry,
    models::ModelHashRegistry,
    stt::{TranscriptionConfig, TranscriptionResult, Transcriber, WhisperTranscriber},
    system_profile::SystemProfiler,
};

#[derive(Parser)]
#[command(name = "lexicon")]
#[command(author, version, about = "Real-time STT, translation, and TTS", long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Increase logging verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transcribe audio file(s)
    Transcribe {
        /// Audio file(s) to transcribe
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output format (srt, txt, json)
        #[arg(short, long, default_value = "srt")]
        format: String,

        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Model to use (tiny, base, small, medium, large)
        #[arg(short, long, default_value = "base")]
        model: String,

        /// Language code (auto-detect if not specified)
        #[arg(short, long)]
        language: Option<String>,

        /// Translate to English
        #[arg(long)]
        translate: bool,

        /// Enable GPU acceleration (enabled by default if available)
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        gpu: bool,
    },

    /// Start the HTTP server
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "8765")]
        port: u16,
    },

    /// Manage models
    Models {
        #[command(subcommand)]
        action: ModelAction,
    },

    /// Manage transcription sessions
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },

    /// Export session to file
    Export {
        /// Session ID to export
        session_id: String,

        /// Output format (srt, txt, json)
        #[arg(short, long, default_value = "srt")]
        format: String,

        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: ShellType,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// TTS quality validation for Beta exit criteria
    TtsQuality {
        #[command(subcommand)]
        action: TtsQualityAction,
    },

    /// Launch diagnostic TUI for real-time transcription
    Tui {
        /// Pre-select audio device by name
        #[arg(long)]
        device: Option<String>,

        /// Disable mouse support
        #[arg(long)]
        no_mouse: bool,

        /// Buffer size before auto-flush (default: 100000)
        #[arg(long, default_value = "100000")]
        buffer_size: usize,

        /// Path to Whisper model file (auto-detects from config if not specified)
        #[arg(long)]
        model: Option<PathBuf>,

        /// Language code for transcription (e.g., "en", "auto")
        #[arg(long, default_value = "en")]
        language: String,

        /// Translate non-English speech to English
        #[arg(long)]
        translate: bool,
    },

    /// Diagnostic TUI (alias for tui command)
    Diag {
        /// Pre-select audio device by name
        #[arg(long)]
        device: Option<String>,

        /// Disable mouse support
        #[arg(long)]
        no_mouse: bool,

        /// Buffer size before auto-flush (default: 100000)
        #[arg(long, default_value = "100000")]
        buffer_size: usize,

        /// Path to Whisper model file (auto-detects from config if not specified)
        #[arg(long)]
        model: Option<PathBuf>,

        /// Language code for transcription (e.g., "en", "auto")
        #[arg(long, default_value = "en")]
        language: String,

        /// Translate non-English speech to English
        #[arg(long)]
        translate: bool,
    },
}

/// Shell types for completion generation.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ShellType {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// PowerShell
    #[value(name = "powershell")]
    PowerShell,
    /// Elvish shell
    Elvish,
}

#[derive(Subcommand)]
enum ModelAction {
    /// List available models
    List {
        /// Show only downloaded models
        #[arg(long)]
        downloaded: bool,
    },

    /// Download a model
    Download {
        /// Model ID to download (tiny, base, small, medium, large)
        model_id: String,
    },

    /// Delete a cached model
    Delete {
        /// Model ID to delete
        model_id: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Show model information
    Info {
        /// Model ID
        model_id: String,
    },

    /// Compute SHA-256 hash for a model file (for hash verification setup)
    Hash {
        /// Model ID (e.g., "tiny", "base") or path to model file
        model_id_or_path: String,
    },

    /// Show hash verification status for all models
    HashStatus,
}

#[derive(Subcommand)]
enum SessionAction {
    /// List all sessions
    List,

    /// Show session details
    Show {
        /// Session ID
        session_id: String,
    },

    /// Delete a session
    Delete {
        /// Session ID
        session_id: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., stt.default_model, server.port)
        key: String,

        /// Value to set
        value: String,
    },

    /// Reset configuration to defaults
    Reset {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Profile system and show hardware information
    Profile {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Force re-detection of system profile
        #[arg(long)]
        refresh: bool,
    },

    /// Show recommended model based on system profile
    Recommend {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show configuration file path
    Path,
}

#[derive(Subcommand)]
enum TtsQualityAction {
    /// Generate test audio files for manual quality assessment
    Generate {
        /// Output directory for audio files
        #[arg(short, long, default_value = "./tts-quality-test")]
        output_dir: PathBuf,

        /// Use platform TTS instead of Kokoro (for comparison)
        #[arg(long)]
        platform: bool,
    },

    /// Show test sentences that will be used
    Sentences,

    /// Display previous quality reports
    Report {
        /// Path to report JSON file
        report_file: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    let log_level = match cli.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Execute command
    match cli.command {
        Commands::Transcribe {
            files,
            format,
            output,
            model,
            language,
            translate,
            gpu,
        } => {
            cmd_transcribe(files, format, output, model, language, translate, gpu)
        }
        Commands::Serve { host, port } => {
            cmd_serve(host, port)
        }
        Commands::Models { action } => match action {
            ModelAction::List { downloaded } => cmd_models_list(downloaded),
            ModelAction::Download { model_id } => cmd_models_download(&model_id),
            ModelAction::Delete { model_id, force } => cmd_models_delete(&model_id, force),
            ModelAction::Info { model_id } => cmd_models_info(&model_id),
            ModelAction::Hash { model_id_or_path } => cmd_models_hash(&model_id_or_path),
            ModelAction::HashStatus => cmd_models_hash_status(),
        },
        Commands::Sessions { action } => match action {
            SessionAction::List => cmd_sessions_list(),
            SessionAction::Show { session_id } => cmd_sessions_show(&session_id),
            SessionAction::Delete { session_id } => cmd_sessions_delete(&session_id),
        },
        Commands::Export {
            session_id,
            format,
            output,
        } => cmd_export(&session_id, &format, output),
        Commands::Completions { shell } => cmd_completions(shell),
        Commands::Config { action } => match action {
            ConfigAction::Show { json } => cmd_config_show(json),
            ConfigAction::Set { key, value } => cmd_config_set(&key, &value),
            ConfigAction::Reset { yes } => cmd_config_reset(yes),
            ConfigAction::Profile { json, refresh: _ } => cmd_config_profile(json),
            ConfigAction::Recommend { json } => cmd_config_recommend(json),
            ConfigAction::Path => cmd_config_path(),
        },
        Commands::TtsQuality { action } => match action {
            TtsQualityAction::Generate { output_dir, platform } => {
                cmd_tts_quality_generate(&output_dir, platform)
            }
            TtsQualityAction::Sentences => cmd_tts_quality_sentences(),
            TtsQualityAction::Report { report_file } => cmd_tts_quality_report(report_file),
        },
        Commands::Tui { device, no_mouse, buffer_size, model, language, translate } |
        Commands::Diag { device, no_mouse, buffer_size, model, language, translate } => {
            cmd_tui(device, no_mouse, buffer_size, model, language, translate)
        }
    }
}

/// Launch the diagnostic TUI.
fn cmd_tui(
    device: Option<String>,
    no_mouse: bool,
    buffer_size: usize,
    model: Option<PathBuf>,
    language: String,
    translate: bool,
) -> Result<()> {
    use tui::{TuiApp, app::TuiConfig};

    // Load config for paths
    let core_config = LexiconConfig::builder().build()?;

    // Determine model path: use specified path, or find from config
    let model_path = if let Some(path) = model {
        Some(path)
    } else {
        // Try to find a model in the default model directory
        let model_dir = &core_config.model_dir;

        // Look for common Whisper model files
        let candidates = [
            "ggml-base.en.bin",
            "ggml-base.bin",
            "ggml-tiny.en.bin",
            "ggml-tiny.bin",
            "ggml-small.en.bin",
            "ggml-small.bin",
        ];

        candidates
            .iter()
            .map(|name| model_dir.join(name))
            .find(|path| path.exists())
    };

    // Get database path from config
    let database_path = Some(core_config.data_dir.join("lexicon.db"));

    let config = TuiConfig {
        buffer_size,
        mouse_enabled: !no_mouse,
        device_name: device,
        model_path,
        language: Some(language),
        translate,
        database_path,
        confirm_new_transcription: core_config.tui.confirm_new_transcription,
    };

    let mut app = TuiApp::with_config(config)
        .map_err(|e| anyhow::anyhow!("Failed to initialize TUI: {}", e))?;

    app.run()
        .map_err(|e| anyhow::anyhow!("TUI error: {}", e))?;

    Ok(())
}

/// Start the HTTP/WebSocket server.
fn cmd_serve(host: String, port: u16) -> Result<()> {
    use lexicon_server::{Server, ServerConfig};

    let config = LexiconConfig::builder().build()?;
    let server_config = ServerConfig::new(&host, port).with_core_config(config);

    eprintln!("Starting Lexicon server on http://{}:{}", host, port);
    eprintln!("API endpoints:");
    eprintln!("  GET  /health                     - Health check");
    eprintln!("  GET  /api/v1/models              - List models");
    eprintln!("  POST /api/v1/models/download     - Download model");
    eprintln!("  POST /api/v1/transcribe          - Batch transcribe");
    eprintln!("  GET  /api/v1/sessions            - List sessions");
    eprintln!("  WS   /api/v1/stt/stream          - STT streaming");
    eprintln!("  WS   /api/v1/tts/stream          - TTS streaming");
    eprintln!();
    eprintln!("Press Ctrl+C to stop");

    // Create runtime and run server
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let server = Server::new(server_config)?;
        server.run().await
    })?;

    Ok(())
}

/// Transcribe audio files.
fn cmd_transcribe(
    files: Vec<PathBuf>,
    format: String,
    output: Option<PathBuf>,
    model: String,
    language: Option<String>,
    translate: bool,
    gpu: bool,
) -> Result<()> {
    // Initialize configuration
    let config = LexiconConfig::builder().build()?;

    // Find model path - check multiple naming conventions
    let model_name = model.strip_prefix("whisper-").unwrap_or(&model);
    let possible_paths = [
        // Hugging Face ggml-* naming
        config.model_dir.join(format!("ggml-{model_name}.bin")),
        // Quantized variants
        config.model_dir.join(format!("ggml-{model_name}-q5_1.bin")),
        // Legacy whisper-* naming
        config.model_dir.join(format!("whisper-{model_name}.bin")),
    ];

    let model_path = possible_paths.iter().find(|p| p.exists());

    let model_path = match model_path {
        Some(path) => path.clone(),
        None => {
            eprintln!("Error: Model '{model}' not found.");
            eprintln!("Searched locations:");
            for path in &possible_paths {
                eprintln!("  - {}", path.display());
            }
            eprintln!("\nRun 'lexicon models download {model}' to download it first.");
            std::process::exit(1);
        }
    };

    // Create transcriber and load model
    eprintln!(
        "Loading model: {model}{}...",
        if gpu { " (GPU)" } else { " (CPU)" }
    );
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .expect("valid template"),
    );
    pb.set_message("Loading model...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut transcriber = WhisperTranscriber::with_gpu(gpu)?;
    transcriber.load_model(&model_path)?;
    pb.finish_with_message("Model loaded");

    // Configure transcription
    let mut tx_config = TranscriptionConfig::new();
    if let Some(lang) = language {
        tx_config = tx_config.with_language(lang);
    }
    if translate {
        tx_config = tx_config.with_translation();
    }

    // Process each file
    let mut all_results: Vec<(PathBuf, TranscriptionResult)> = Vec::new();

    for file in &files {
        eprintln!("\nTranscribing: {}", file.display());

        if !file.exists() {
            return Err(anyhow::anyhow!("File not found: {}", file.display()));
        }

        // Load audio file
        let samples = load_audio_file(file)?;
        let duration_secs = samples.len() as f32 / 16000.0;
        eprintln!("  Audio duration: {:.1}s", duration_secs);

        // Transcribe with progress
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {bar:40.cyan/blue} {percent}% {msg}")
                .expect("valid template")
                .progress_chars("##-"),
        );
        pb.set_message("Transcribing...");

        let start = Instant::now();
        let result = transcriber.transcribe(&samples, &tx_config)?;
        let elapsed = start.elapsed();

        pb.finish_with_message(format!(
            "Done in {:.1}s (RTF: {:.2}x)",
            elapsed.as_secs_f32(),
            result.real_time_factor()
        ));

        all_results.push((file.clone(), result));
    }

    // Format and output results
    let output_text = format_results(&all_results, &format)?;

    if let Some(output_path) = output {
        let mut file = BufWriter::new(
            std::fs::File::create(&output_path)
                .with_context(|| format!("Failed to create output file: {}", output_path.display()))?,
        );
        file.write_all(output_text.as_bytes())?;
        eprintln!("\nOutput written to: {}", output_path.display());
    } else {
        println!("{output_text}");
    }

    Ok(())
}

/// Load and convert audio file to 16kHz mono f32 samples.
fn load_audio_file(path: &Path) -> Result<Vec<f32>> {
    // For now, we expect raw 16kHz mono f32 PCM files
    // In a full implementation, we'd use symphonia or similar for format detection
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "wav" => load_wav_file(path),
        "pcm" | "raw" => load_raw_pcm(path),
        _ => {
            anyhow::bail!(
                "Unsupported audio format: {extension}. Supported formats: wav, pcm, raw"
            );
        }
    }
}

/// Load a WAV file and convert to 16kHz mono f32.
fn load_wav_file(path: &Path) -> Result<Vec<f32>> {
    use std::io::BufReader;

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    // Simple WAV parser - read header and data
    let mut wav_reader = hound::WavReader::new(reader)
        .with_context(|| format!("Failed to read WAV file: {}", path.display()))?;

    let spec = wav_reader.spec();
    tracing::debug!(
        "WAV: {}Hz, {} channels, {:?}",
        spec.sample_rate,
        spec.channels,
        spec.sample_format
    );

    // Read samples based on format
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            wav_reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max_val))
                .collect::<std::result::Result<Vec<_>, _>>()?
        }
        hound::SampleFormat::Float => wav_reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()?,
    };

    // Convert to mono if stereo
    let mono = if spec.channels == 2 {
        samples
            .chunks(2)
            .map(|c| (c[0] + c.get(1).copied().unwrap_or(c[0])) * 0.5)
            .collect()
    } else {
        samples
    };

    // Resample to 16kHz if needed
    if spec.sample_rate != 16000 {
        resample_to_16k(&mono, spec.sample_rate)
    } else {
        Ok(mono)
    }
}

/// Load raw PCM file (assumed 16kHz mono f32).
fn load_raw_pcm(path: &Path) -> Result<Vec<f32>> {
    let bytes = std::fs::read(path)?;
    let samples: Vec<f32> = bytes
        .chunks(4)
        .filter_map(|chunk| {
            if chunk.len() == 4 {
                Some(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            } else {
                None
            }
        })
        .collect();
    Ok(samples)
}

/// Resample audio to 16kHz.
fn resample_to_16k(samples: &[f32], source_rate: u32) -> Result<Vec<f32>> {
    use lexicon_core::Resampler;

    // Use 1 second chunks for resampling (or less if file is shorter)
    let chunk_size = samples.len().min(source_rate as usize);
    let mut resampler = Resampler::new(source_rate, 1, chunk_size)?;

    let mut output = Vec::new();
    let chunks: Vec<&[f32]> = samples.chunks(chunk_size).collect();
    let num_chunks = chunks.len();

    for (i, chunk) in chunks.into_iter().enumerate() {
        let is_last_chunk = i == num_chunks - 1;
        let is_partial = chunk.len() < chunk_size;

        let resampled = if is_last_chunk && is_partial {
            // Use process_partial for the final chunk if it's smaller than chunk_size
            resampler.process_partial(chunk)?
        } else {
            resampler.process(chunk)?
        };
        output.extend(resampled);
    }

    Ok(output)
}

/// Format transcription results for output.
fn format_results(results: &[(PathBuf, TranscriptionResult)], format: &str) -> Result<String> {
    match format {
        "srt" => format_srt(results),
        "txt" => format_txt(results),
        "json" => format_json(results),
        _ => anyhow::bail!("Unsupported output format: {format}. Use srt, txt, or json."),
    }
}

/// Format results as SRT subtitles.
fn format_srt(results: &[(PathBuf, TranscriptionResult)]) -> Result<String> {
    let mut output = String::new();
    let mut segment_num = 1;

    for (_path, result) in results {
        for segment in &result.segments {
            // SRT format: index, timecodes, text, blank line
            output.push_str(&format!("{segment_num}\n"));
            output.push_str(&format!(
                "{} --> {}\n",
                format_srt_time(segment.start_ms),
                format_srt_time(segment.end_ms)
            ));
            output.push_str(&segment.text);
            output.push_str("\n\n");
            segment_num += 1;
        }
    }

    Ok(output)
}

/// Format milliseconds as SRT timestamp (HH:MM:SS,mmm).
fn format_srt_time(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

/// Format results as plain text.
fn format_txt(results: &[(PathBuf, TranscriptionResult)]) -> Result<String> {
    let mut output = String::new();

    for (path, result) in results {
        if results.len() > 1 {
            output.push_str(&format!("=== {} ===\n\n", path.display()));
        }
        output.push_str(&result.full_text());
        output.push_str("\n\n");
    }

    Ok(output)
}

/// Format results as JSON.
fn format_json(results: &[(PathBuf, TranscriptionResult)]) -> Result<String> {
    #[derive(serde::Serialize)]
    struct JsonOutput {
        files: Vec<FileResult>,
    }

    #[derive(serde::Serialize)]
    struct FileResult {
        path: String,
        duration_ms: i64,
        processing_time_ms: i64,
        detected_language: Option<String>,
        segments: Vec<JsonSegment>,
    }

    #[derive(serde::Serialize)]
    struct JsonSegment {
        start_ms: i64,
        end_ms: i64,
        text: String,
        confidence: Option<f32>,
    }

    let output = JsonOutput {
        files: results
            .iter()
            .map(|(path, result)| FileResult {
                path: path.display().to_string(),
                duration_ms: result.audio_duration_ms,
                processing_time_ms: result.processing_time_ms,
                detected_language: result.detected_language.clone(),
                segments: result
                    .segments
                    .iter()
                    .map(|s| JsonSegment {
                        start_ms: s.start_ms,
                        end_ms: s.end_ms,
                        text: s.text.clone(),
                        confidence: s.confidence,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(serde_json::to_string_pretty(&output)?)
}

/// Model definition with display name, size, and possible filenames.
struct ModelDef {
    /// Display name (e.g., "base-q5_1")
    name: &'static str,
    /// Human-readable size
    size: &'static str,
    /// Possible filenames to check (in order of preference)
    /// Supports both legacy whisper-*.bin and ggml-*.bin naming conventions
    filenames: &'static [&'static str],
}

impl ModelDef {
    /// Find the first existing file path for this model in the given directory.
    fn find_path(&self, model_dir: &Path) -> Option<PathBuf> {
        for filename in self.filenames {
            let path = model_dir.join(filename);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }
}

/// Find an existing model file by name, checking all possible filename patterns.
/// Returns the path if found, otherwise returns the expected ggml-*.bin path.
fn find_model_path(model_dir: &Path, model_name: &str) -> (Option<PathBuf>, PathBuf) {
    // Determine the base name and whether it's quantized
    let base_name = model_name
        .strip_prefix("whisper-")
        .unwrap_or(model_name);

    // Build list of possible filenames to check
    let filenames: Vec<String> = vec![
        format!("ggml-{base_name}.bin"),
        format!("whisper-{base_name}.bin"),
    ];

    // Find existing file
    for filename in &filenames {
        let path = model_dir.join(filename);
        if path.exists() {
            return (Some(path.clone()), path);
        }
    }

    // Return expected path (ggml naming preferred)
    let expected = model_dir.join(format!("ggml-{base_name}.bin"));
    (None, expected)
}

/// List available models.
fn cmd_models_list(downloaded_only: bool) -> Result<()> {
    let config = LexiconConfig::builder().build()?;
    let _registry = ModelRegistry::new(&config.model_dir)?;

    println!("Available Whisper models:\n");
    println!("{:<16} {:<10} {:<14} {}", "Model", "Size", "Status", "Path");
    println!("{}", "-".repeat(70));

    // Models with both ggml-*.bin (HuggingFace) and whisper-*.bin (legacy) filename patterns
    let models: &[ModelDef] = &[
        // Quantized models (smaller, faster)
        ModelDef {
            name: "tiny-q5_1",
            size: "31 MB",
            filenames: &["ggml-tiny-q5_1.bin", "whisper-tiny-q5_1.bin"],
        },
        ModelDef {
            name: "base-q5_1",
            size: "57 MB",
            filenames: &["ggml-base-q5_1.bin", "whisper-base-q5_1.bin"],
        },
        ModelDef {
            name: "small-q5_1",
            size: "182 MB",
            filenames: &["ggml-small-q5_1.bin", "whisper-small-q5_1.bin"],
        },
        // Non-quantized models (original quality)
        ModelDef {
            name: "tiny",
            size: "75 MB",
            filenames: &["ggml-tiny.bin", "whisper-tiny.bin"],
        },
        ModelDef {
            name: "base",
            size: "142 MB",
            filenames: &["ggml-base.bin", "whisper-base.bin"],
        },
        ModelDef {
            name: "small",
            size: "466 MB",
            filenames: &["ggml-small.bin", "whisper-small.bin"],
        },
        ModelDef {
            name: "medium",
            size: "1.5 GB",
            filenames: &["ggml-medium.bin", "whisper-medium.bin"],
        },
        ModelDef {
            name: "large-v3",
            size: "2.9 GB",
            filenames: &["ggml-large-v3.bin", "whisper-large-v3.bin", "ggml-large.bin", "whisper-large.bin"],
        },
        ModelDef {
            name: "large-v3-turbo",
            size: "1.5 GB",
            filenames: &["ggml-large-v3-turbo.bin", "whisper-large-v3-turbo.bin"],
        },
    ];

    for model in models {
        let found_path = model.find_path(&config.model_dir);
        let is_downloaded = found_path.is_some();

        if downloaded_only && !is_downloaded {
            continue;
        }

        let status = if is_downloaded { "Downloaded" } else { "Not downloaded" };
        let path_str = found_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "-".to_string());

        println!("{:<16} {:<10} {:<14} {}", model.name, model.size, status, path_str);
    }

    println!("\nModel directory: {}", config.model_dir.display());

    Ok(())
}

/// Download a model.
fn cmd_models_download(model_id: &str) -> Result<()> {
    let config = LexiconConfig::builder().build()?;
    let mut registry = ModelRegistry::new(&config.model_dir)?;

    // Normalize model ID
    let model_name = model_id.strip_prefix("whisper-").unwrap_or(model_id);
    let full_model_id = format!("whisper-{model_name}");

    // Check if model already exists (any filename variant)
    let (existing_path, _) = find_model_path(&config.model_dir, model_name);
    if let Some(path) = existing_path {
        println!("Model '{model_name}' is already downloaded at:");
        println!("  {}", path.display());
        return Ok(());
    }

    // Create progress bar
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("valid template")
            .progress_chars("#>-"),
    );
    let pb_clone = pb.clone();

    // Create a runtime for async download
    let rt = tokio::runtime::Runtime::new()?;

    println!("Fetching model manifest...");
    let download_path = rt.block_on(async {
        // Fetch manifest first
        registry.fetch_manifest().await?;

        println!("Downloading model: {model_name}...");

        // Download with progress callback
        registry
            .download_model(&full_model_id, move |progress| {
                pb_clone.set_length(progress.total_bytes);
                pb_clone.set_position(progress.downloaded_bytes);
            })
            .await
    })?;

    pb.finish_with_message("Download complete");
    // Use the actual path returned by the registry, falling back to expected path
    println!("\nModel downloaded to: {}", download_path.display());

    Ok(())
}

/// Delete a cached model.
fn cmd_models_delete(model_id: &str, force: bool) -> Result<()> {
    let config = LexiconConfig::builder().build()?;

    // Normalize model ID
    let model_name = model_id.strip_prefix("whisper-").unwrap_or(model_id);

    // Find existing model file (any filename variant)
    let (existing_path, _) = find_model_path(&config.model_dir, model_name);

    let model_path = match existing_path {
        Some(path) => path,
        None => {
            println!("Model '{model_name}' is not downloaded.");
            return Ok(());
        }
    };

    if !force {
        eprint!("Delete model '{model_name}'? [y/N] ");
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    std::fs::remove_file(&model_path)?;
    println!("Deleted model: {}", model_path.display());

    Ok(())
}

/// Show model information.
fn cmd_models_info(model_id: &str) -> Result<()> {
    let config = LexiconConfig::builder().build()?;

    // Normalize model ID
    let model_name = model_id.strip_prefix("whisper-").unwrap_or(model_id);
    let full_model_id = format!("whisper-{model_name}");

    // Find existing model file (any filename variant)
    let (existing_path, expected_path) = find_model_path(&config.model_dir, model_name);

    println!("Model: {model_name}");
    println!("ID: {full_model_id}");

    if let Some(ref path) = existing_path {
        println!("Path: {}", path.display());
        let metadata = std::fs::metadata(path)?;
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        println!("Status: Downloaded");
        println!("Size: {:.1} MB", size_mb);
    } else {
        println!("Path: {} (expected)", expected_path.display());
        println!("Status: Not downloaded");
    }

    // Model capabilities - strip quantization suffix for language detection
    let base_model = model_name
        .strip_suffix("-q5_1")
        .or_else(|| model_name.strip_suffix("-q8_0"))
        .unwrap_or(model_name);

    let (_languages, multilingual) = match base_model {
        "tiny" | "base" | "small" | "medium" | "large" | "large-v3" | "large-v3-turbo" => (99, true),
        "tiny.en" | "base.en" | "small.en" | "medium.en" => (1, false),
        _ => (99, true),
    };

    println!("Languages: {}", if multilingual { "99 (multilingual)" } else { "English only" });

    Ok(())
}

/// Compute SHA-256 hash for a model file.
fn cmd_models_hash(model_id_or_path: &str) -> Result<()> {
    let path = if model_id_or_path.contains('/') || model_id_or_path.contains('\\') || model_id_or_path.ends_with(".bin") || model_id_or_path.ends_with(".onnx") {
        // Treat as a direct file path
        PathBuf::from(model_id_or_path)
    } else {
        // Treat as a model ID, find it in the model directory
        let config = LexiconConfig::builder().build()?;
        let model_name = model_id_or_path.strip_prefix("whisper-").unwrap_or(model_id_or_path);

        // Try various naming conventions
        let possible_paths = [
            config.model_dir.join(format!("ggml-{model_name}.bin")),
            config.model_dir.join(format!("ggml-{model_name}-q5_1.bin")),
            config.model_dir.join(format!("whisper-{model_name}.bin")),
            config.model_dir.join(format!("{model_id_or_path}.bin")),
            config.model_dir.join(format!("{model_id_or_path}.onnx")),
        ];

        possible_paths
            .into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Model '{}' not found in {}.\nRun 'lexicon models download {}' first.",
                    model_id_or_path,
                    config.model_dir.display(),
                    model_name
                )
            })?
    };

    if !path.exists() {
        anyhow::bail!("File not found: {}", path.display());
    }

    // Show file info
    let metadata = std::fs::metadata(&path)?;
    let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

    eprintln!("Computing SHA-256 hash...");
    eprintln!("  File: {}", path.display());
    eprintln!("  Size: {:.1} MB", size_mb);

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .expect("valid template"),
    );
    pb.set_message("Hashing...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let hash = ModelHashRegistry::compute_hash(&path)
        .context("Failed to compute hash")?;

    pb.finish_and_clear();

    println!();
    println!("SHA-256 Hash:");
    println!("  {}", hash);
    println!();

    // Extract model ID from filename for convenience
    let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let model_id = if filename.starts_with("ggml-") {
        let base = filename.strip_prefix("ggml-").unwrap_or(filename);
        format!("whisper-{}", base)
    } else if filename.starts_with("whisper-") {
        filename.to_string()
    } else {
        filename.to_string()
    };

    // Provide copy-paste ready format for hashes.rs
    println!("To add to hashes.rs:");
    println!("  pub const HASH_{}: &str = \"{}\";",
        model_id.to_uppercase().replace('-', "_"),
        hash
    );

    Ok(())
}

/// Show hash verification status for all models.
fn cmd_models_hash_status() -> Result<()> {
    let registry = ModelHashRegistry::new();
    let config = LexiconConfig::builder().build()?;

    println!("Model Hash Verification Status");
    println!("{}", "=".repeat(60));
    println!();

    let pending = registry.pending_models();
    let total_models = 10; // Number of models in the registry

    if registry.is_production_ready() {
        println!("Status: PRODUCTION READY ✓");
        println!("All {} model hashes have been verified.", total_models);
    } else {
        println!("Status: AWAITING VERIFICATION");
        println!("Pending: {} of {} models need hash verification.", pending.len(), total_models);
        println!();
        println!("Models pending verification:");
        for model_id in &pending {
            // Check if model is downloaded
            let model_name = model_id.strip_prefix("whisper-").unwrap_or(model_id);
            let possible_paths = [
                config.model_dir.join(format!("ggml-{model_name}.bin")),
                config.model_dir.join(format!("{model_id}.bin")),
                config.model_dir.join(format!("{model_id}.onnx")),
            ];

            let is_downloaded = possible_paths.iter().any(|p| p.exists());
            let status = if is_downloaded { "downloaded" } else { "not downloaded" };

            println!("  - {} ({})", model_id, status);
        }
    }

    println!();
    println!("To compute a hash for a downloaded model:");
    println!("  lexicon models hash <model-id>");
    println!();
    println!("Example:");
    println!("  lexicon models hash tiny");
    println!("  lexicon models hash /path/to/model.bin");
    println!();
    println!("See .dev/human.md for full verification instructions.");

    Ok(())
}

// ============================================================================
// Session Commands
// ============================================================================

/// List all sessions.
fn cmd_sessions_list() -> Result<()> {
    #[allow(unused_imports)]
    use lexicon_core::storage::{Database, SessionId};

    let config = LexiconConfig::builder().build()?;
    let db_path = config.data_dir.join("lexicon.db");

    if !db_path.exists() {
        println!("No sessions found. Database has not been created yet.");
        return Ok(());
    }

    let db = Database::open(&db_path)?;
    let sessions = db.list_sessions()?;

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    println!("{:<8} {:<24} {:<12} {}", "ID", "Created", "Language", "Name");
    println!("{}", "-".repeat(60));

    for session in sessions {
        let created = chrono::DateTime::from_timestamp_millis(session.created_at)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let language = session.language.as_deref().unwrap_or("-");
        let name = session.name.as_deref().unwrap_or("-");

        println!("{:<8} {:<24} {:<12} {}", session.id.0, created, language, name);
    }

    Ok(())
}

/// Show session details.
fn cmd_sessions_show(session_id_str: &str) -> Result<()> {
    use lexicon_core::storage::{Database, SessionId};

    let session_id: i64 = session_id_str
        .parse()
        .context("Invalid session ID - must be a number")?;

    let config = LexiconConfig::builder().build()?;
    let db_path = config.data_dir.join("lexicon.db");

    if !db_path.exists() {
        anyhow::bail!("Database not found at {}", db_path.display());
    }

    let db = Database::open(&db_path)?;
    let session = db.get_session(SessionId(session_id))?;
    let segments = db.get_session_segments(SessionId(session_id))?;

    // Format timestamps
    let created = chrono::DateTime::from_timestamp_millis(session.created_at)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let updated = chrono::DateTime::from_timestamp_millis(session.updated_at)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Calculate duration
    let duration_ms = segments.iter().map(|s| s.end_ms).max().unwrap_or(0);
    let duration_secs = duration_ms as f64 / 1000.0;

    println!("Session: {}", session.id.0);
    println!("  Name:     {}", session.name.as_deref().unwrap_or("-"));
    println!("  Language: {}", session.language.as_deref().unwrap_or("auto"));
    println!("  Created:  {}", created);
    println!("  Updated:  {}", updated);
    println!("  Duration: {:.1}s", duration_secs);
    println!("  Segments: {}", segments.len());

    if !segments.is_empty() {
        println!("\nTranscription:");
        println!("{}", "-".repeat(60));

        for segment in &segments {
            let start_time = format_srt_time(segment.start_ms);
            let end_time = format_srt_time(segment.end_ms);
            let confidence = segment
                .confidence
                .map(|c| format!(" ({:.0}%)", c * 100.0))
                .unwrap_or_default();

            println!("[{} --> {}]{}", start_time, end_time, confidence);
            println!("  {}", segment.text);
        }
    }

    Ok(())
}

/// Delete a session.
fn cmd_sessions_delete(session_id_str: &str) -> Result<()> {
    use lexicon_core::storage::{Database, SessionId};

    let session_id: i64 = session_id_str
        .parse()
        .context("Invalid session ID - must be a number")?;

    let config = LexiconConfig::builder().build()?;
    let db_path = config.data_dir.join("lexicon.db");

    if !db_path.exists() {
        anyhow::bail!("Database not found at {}", db_path.display());
    }

    let db = Database::open(&db_path)?;

    // Verify session exists
    let session = db.get_session(SessionId(session_id))?;

    // Ask for confirmation
    eprint!(
        "Delete session {} ({})? [y/N] ",
        session.id.0,
        session.name.as_deref().unwrap_or("unnamed")
    );
    std::io::stderr().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Cancelled.");
        return Ok(());
    }

    db.delete_session(SessionId(session_id))?;
    println!("Session {} deleted.", session_id);

    Ok(())
}

// ============================================================================
// Export Command
// ============================================================================

/// Export session to file.
fn cmd_export(session_id_str: &str, format: &str, output: Option<PathBuf>) -> Result<()> {
    use lexicon_core::storage::{Database, SessionId};

    let session_id: i64 = session_id_str
        .parse()
        .context("Invalid session ID - must be a number")?;

    let config = LexiconConfig::builder().build()?;
    let db_path = config.data_dir.join("lexicon.db");

    if !db_path.exists() {
        anyhow::bail!("Database not found at {}", db_path.display());
    }

    let db = Database::open(&db_path)?;
    let session = db.get_session(SessionId(session_id))?;
    let segments = db.get_session_segments(SessionId(session_id))?;

    let content = match format {
        "srt" => {
            // Use database's built-in SRT export
            db.export_srt(SessionId(session_id))?
        }
        "txt" => {
            // Plain text - just the transcription
            segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        }
        "json" => {
            // Full JSON export
            let export = serde_json::json!({
                "session": {
                    "id": session.id.0,
                    "name": session.name,
                    "language": session.language,
                    "created_at": chrono::DateTime::from_timestamp_millis(session.created_at)
                        .map(|dt| dt.to_rfc3339()),
                    "updated_at": chrono::DateTime::from_timestamp_millis(session.updated_at)
                        .map(|dt| dt.to_rfc3339()),
                },
                "segments": segments.iter().map(|s| serde_json::json!({
                    "id": s.id.0,
                    "start_ms": s.start_ms,
                    "end_ms": s.end_ms,
                    "text": s.text,
                    "confidence": s.confidence,
                    "language": s.language,
                    "speaker_id": s.speaker_id
                })).collect::<Vec<_>>(),
                "duration_ms": segments.iter().map(|s| s.end_ms).max().unwrap_or(0)
            });
            serde_json::to_string_pretty(&export)?
        }
        _ => {
            anyhow::bail!("Unsupported export format: {format}. Use srt, txt, or json.");
        }
    };

    if let Some(output_path) = output {
        let mut file = BufWriter::new(
            std::fs::File::create(&output_path)
                .with_context(|| format!("Failed to create output file: {}", output_path.display()))?,
        );
        file.write_all(content.as_bytes())?;
        eprintln!("Exported to: {}", output_path.display());
    } else {
        println!("{content}");
    }

    Ok(())
}

// ============================================================================
// Completions Command
// ============================================================================

/// Generate shell completion scripts.
fn cmd_completions(shell_type: ShellType) -> Result<()> {
    let mut cmd = Cli::command();
    let shell = match shell_type {
        ShellType::Bash => Shell::Bash,
        ShellType::Zsh => Shell::Zsh,
        ShellType::Fish => Shell::Fish,
        ShellType::PowerShell => Shell::PowerShell,
        ShellType::Elvish => Shell::Elvish,
    };

    generate(shell, &mut cmd, "lexicon", &mut std::io::stdout());

    // Print usage instructions to stderr
    let install_hint = match shell_type {
        ShellType::Bash => {
            "# Add to ~/.bashrc:\n#   eval \"$(lexicon completions bash)\"\n# Or save to file:\n#   lexicon completions bash > ~/.local/share/bash-completion/completions/lexicon"
        }
        ShellType::Zsh => {
            "# Add to ~/.zshrc (before compinit):\n#   eval \"$(lexicon completions zsh)\"\n# Or save to fpath:\n#   lexicon completions zsh > ~/.zfunc/_lexicon"
        }
        ShellType::Fish => {
            "# Save to completions directory:\n#   lexicon completions fish > ~/.config/fish/completions/lexicon.fish"
        }
        ShellType::PowerShell => {
            "# Add to PowerShell profile:\n#   lexicon completions powershell | Out-String | Invoke-Expression"
        }
        ShellType::Elvish => {
            "# Add to ~/.elvish/rc.elv:\n#   eval (lexicon completions elvish | slurp)"
        }
    };

    eprintln!("\n{install_hint}");

    Ok(())
}

// ============================================================================
// Config Commands
// ============================================================================

/// Show current configuration.
fn cmd_config_show(json: bool) -> Result<()> {
    // Try to load existing config, or show defaults
    let (config, is_default) = if let Some(cfg) = LexiconConfig::load_if_exists()? {
        (cfg, false)
    } else {
        (LexiconConfig::builder().build()?, true)
    };

    if json {
        // JSON output
        let output = serde_json::json!({
            "config": config,
            "is_default": is_default,
            "config_path": LexiconConfig::default_config_path()?.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if is_default {
            eprintln!("No configuration file found. Showing defaults.\n");
        }
        // Display configuration
        println!("{}", config.to_toml()?);
    }

    Ok(())
}

/// Set a configuration value.
fn cmd_config_set(key: &str, value: &str) -> Result<()> {
    // Load existing config or create default
    let mut config = LexiconConfig::load_if_exists()?
        .unwrap_or_else(|| LexiconConfig::builder().build().unwrap());

    // Parse the key and update the appropriate field
    match key {
        "stt.default_model" => {
            config.stt.default_model = value.to_string();
        }
        "stt.diarization_enabled" => {
            config.stt.diarization_enabled = value.parse()
                .with_context(|| format!("Invalid boolean value: {value}"))?;
        }
        "stt.max_speakers" => {
            config.stt.max_speakers = value.parse()
                .with_context(|| format!("Invalid number: {value}"))?;
        }
        "stt.gpu_enabled" => {
            config.stt.gpu_enabled = value.parse()
                .with_context(|| format!("Invalid boolean value: {value}"))?;
        }
        "tts.fallback_enabled" => {
            config.tts.fallback_enabled = value.parse()
                .with_context(|| format!("Invalid boolean value: {value}"))?;
        }
        "tts.default_voice" => {
            config.tts.default_voice = if value.is_empty() || value == "none" {
                None
            } else {
                Some(value.to_string())
            };
        }
        "server.host" => {
            config.server.host = value.to_string();
        }
        "server.port" => {
            config.server.port = value.parse()
                .with_context(|| format!("Invalid port number: {value}"))?;
        }
        "server.cors_enabled" => {
            config.server.cors_enabled = value.parse()
                .with_context(|| format!("Invalid boolean value: {value}"))?;
        }
        "log_level" => {
            config.log_level = value.parse()
                .with_context(|| format!("Invalid log level: {value}"))?;
        }
        _ => {
            anyhow::bail!(
                "Unknown configuration key: {key}\n\
                Valid keys:\n  \
                stt.default_model\n  \
                stt.diarization_enabled\n  \
                stt.max_speakers\n  \
                stt.gpu_enabled\n  \
                tts.fallback_enabled\n  \
                tts.default_voice\n  \
                server.host\n  \
                server.port\n  \
                server.cors_enabled\n  \
                log_level"
            );
        }
    }

    // Save the updated config
    config.save_default()?;

    println!("Configuration updated: {key} = {value}");
    println!("Saved to: {}", LexiconConfig::default_config_path()?.display());

    Ok(())
}

/// Reset configuration to defaults.
fn cmd_config_reset(skip_confirm: bool) -> Result<()> {
    let config_path = LexiconConfig::default_config_path()?;

    if !config_path.exists() {
        println!("No configuration file exists at:");
        println!("  {}", config_path.display());
        println!("\nNothing to reset - already using defaults.");
        return Ok(());
    }

    // Ask for confirmation unless -y flag is provided
    if !skip_confirm {
        eprint!("Reset configuration to defaults? This will remove your custom settings. [y/N] ");
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Load existing config to preserve directory paths
    let old_config = LexiconConfig::load_from_file(&config_path)?;

    // Create new default config but preserve data_dir and model_dir
    let new_config = LexiconConfig::builder()
        .data_dir(old_config.data_dir)
        .model_dir(old_config.model_dir)
        .build()?;

    // Save the reset config
    new_config.save_to_file(&config_path)?;

    println!("Configuration reset to defaults.");
    println!("Preserved directories:");
    println!("  data_dir:  {}", new_config.data_dir.display());
    println!("  model_dir: {}", new_config.model_dir.display());
    println!("\nConfig file: {}", config_path.display());

    Ok(())
}

/// Profile system hardware and display results.
fn cmd_config_profile(json: bool) -> Result<()> {
    let profile = SystemProfiler::profile();

    if json {
        // JSON output
        let output = serde_json::json!({
            "tier": profile.tier.as_str(),
            "ram_bytes": profile.ram_bytes,
            "ram_gb": profile.ram_gb(),
            "cpu_cores": profile.cpu_cores,
            "cpu_arch": profile.cpu_arch,
            "cpu_brand": profile.cpu_brand,
            "gpu": profile.gpu.as_ref().map(|g| serde_json::json!({
                "name": g.name,
                "vram_mb": g.vram_mb,
                "is_discrete": g.is_discrete,
                "is_apple_silicon": g.is_apple_silicon,
                "has_ane": g.has_ane
            })),
            "disk_free_bytes": profile.disk_free_bytes,
            "disk_free_gb": profile.disk_free_gb(),
            "platform": format!("{:?}", profile.platform),
            "profiled_at": profile.profiled_at.to_rfc3339(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        eprintln!("Profiling system...\n");

        println!("System Profile");
        println!("{}", "=".repeat(50));
        println!();

        // RAM
        println!("Memory:");
        println!("  Total RAM:     {:.1} GB", profile.ram_gb());
        println!("  CPU Cores:     {}", profile.cpu_cores);
        println!("  Architecture:  {}", profile.cpu_arch);
        if let Some(brand) = &profile.cpu_brand {
            println!("  CPU:           {}", brand);
        }
        println!();

        // GPU
        println!("GPU:");
        if let Some(gpu) = &profile.gpu {
            println!("  Name:          {}", gpu.name);
            if let Some(vram) = gpu.vram_mb {
                println!("  VRAM:          {} MB", vram);
            }
            if gpu.is_apple_silicon {
                println!("  Apple Silicon: Yes (unified memory)");
            }
            if gpu.has_ane {
                println!("  Neural Engine: Yes");
            }
        } else {
            println!("  Status:        Not detected");
        }
        println!();

        // Storage
        println!("Storage:");
        println!("  Free space:    {:.1} GB", profile.disk_free_gb());
        println!();

        // Platform
        println!("Platform:");
        println!("  OS:            {:?}", profile.platform);
        println!();

        // Tier classification
        println!("Classification:");
        println!("  System Tier:   {} ({:?})", profile.tier, profile.tier);
        println!();
    }

    Ok(())
}

/// Show recommended model based on system profile.
fn cmd_config_recommend(json: bool) -> Result<()> {
    let profile = SystemProfiler::profile();
    let recommendation = SystemProfiler::recommend_model(&profile);

    // Check if recommended model is downloaded
    let config = LexiconConfig::builder().build()?;
    let model_path = config.model_dir.join(format!("{}.bin", recommendation.model_id));
    let is_downloaded = model_path.exists();

    if json {
        // JSON output
        let output = serde_json::json!({
            "system_tier": profile.tier.as_str(),
            "recommended": {
                "model_id": recommendation.model_id,
                "display_name": recommendation.display_name,
                "size_bytes": recommendation.size_bytes,
                "rationale": recommendation.rationale,
                "is_downloaded": is_downloaded
            },
            "alternatives": recommendation.alternatives.iter().map(|alt| serde_json::json!({
                "model_id": alt.model_id,
                "display_name": alt.display_name,
                "size_bytes": alt.size_bytes
            })).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Model Recommendation");
        println!("{}", "=".repeat(50));
        println!();

        println!("System Tier: {:?}", profile.tier);
        println!();

        println!("Recommended Model:");
        println!("  Name:     {}", recommendation.display_name);
        println!("  ID:       {}", recommendation.model_id);
        println!("  Size:     {:.1} MB", recommendation.size_bytes as f64 / 1_000_000.0);
        println!("  Reason:   {}", recommendation.rationale);
        println!();

        if !recommendation.alternatives.is_empty() {
            println!("Alternative Models:");
            for alt in &recommendation.alternatives {
                println!(
                    "  - {} ({:.1} MB)",
                    alt.display_name,
                    alt.size_bytes as f64 / 1_000_000.0
                );
            }
            println!();
        }

        if is_downloaded {
            println!("Status: Model is already downloaded");
        } else {
            println!("Status: Model not downloaded");
            println!("        Run: lexicon models download {}",
                recommendation.model_id.strip_prefix("whisper-").unwrap_or(&recommendation.model_id));
        }
    }

    Ok(())
}

/// Show configuration file path.
fn cmd_config_path() -> Result<()> {
    let config_path = LexiconConfig::default_config_path()?;
    let data_dir = LexiconConfig::default_data_dir()?;
    let model_dir = LexiconConfig::default_model_dir()?;

    println!("Configuration Paths");
    println!("{}", "=".repeat(50));
    println!();
    println!("Config file:   {}", config_path.display());
    println!("Data directory: {}", data_dir.display());
    println!("Model directory: {}", model_dir.display());
    println!();

    if config_path.exists() {
        println!("Config status: exists");
    } else {
        println!("Config status: not created (using defaults)");
    }

    if LexiconConfig::is_first_run() {
        println!("First run:     yes");
    } else {
        println!("First run:     no");
    }

    Ok(())
}

// ============================================================================
// TTS Quality Commands
// ============================================================================

/// Generate test audio files for manual quality assessment.
fn cmd_tts_quality_generate(output_dir: &Path, use_platform: bool) -> Result<()> {
    use lexicon_core::tts::{QUALITY_TEST_SENTENCES, TtsManager, TtsEngine, TtsConfig};
    use std::fs;

    println!("TTS Quality Test - Audio Generation");
    println!("{}", "=".repeat(50));
    println!();

    // Create output directory
    fs::create_dir_all(output_dir)
        .context("Failed to create output directory")?;

    let engine_name = if use_platform { "platform" } else { "kokoro" };
    println!("Engine: {}", engine_name);
    println!("Output: {}", output_dir.display());
    println!("Sentences: {}", QUALITY_TEST_SENTENCES.len());
    println!();

    // Initialize TTS manager
    let config = TtsConfig::new()
        .with_engine(if use_platform { TtsEngine::Platform } else { TtsEngine::Kokoro });

    let tts = TtsManager::new()
        .context("Failed to initialize TTS engine")?;

    let pb = ProgressBar::new(QUALITY_TEST_SENTENCES.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")?
            .progress_chars("##-"),
    );

    for (i, sentence) in QUALITY_TEST_SENTENCES.iter().enumerate() {
        let filename = format!("{:02}_{}.wav", i + 1, engine_name);
        let output_path = output_dir.join(&filename);

        pb.set_message(format!("Generating: {}", filename));

        // Synthesize audio
        match tts.synthesize(sentence, &config) {
            Ok(result) => {
                // Write WAV file
                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: result.audio.sample_rate,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };

                let mut writer = hound::WavWriter::create(&output_path, spec)
                    .context("Failed to create WAV file")?;

                for sample in &result.audio.samples {
                    let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    writer.write_sample(sample_i16)?;
                }

                writer.finalize()?;
            }
            Err(e) => {
                eprintln!("\nError synthesizing sentence {}: {}", i + 1, e);
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    println!();

    // Write sentence manifest
    let manifest_path = output_dir.join("sentences.txt");
    let mut manifest = BufWriter::new(
        std::fs::File::create(&manifest_path)
            .context("Failed to create manifest")?
    );

    writeln!(manifest, "TTS Quality Test Sentences")?;
    writeln!(manifest, "Engine: {}", engine_name)?;
    writeln!(manifest, "Generated: {}", chrono::Utc::now().to_rfc3339())?;
    writeln!(manifest)?;

    for (i, sentence) in QUALITY_TEST_SENTENCES.iter().enumerate() {
        writeln!(manifest, "{:02}. {}", i + 1, sentence)?;
    }

    manifest.flush()?;

    println!("Generated {} audio files in: {}", QUALITY_TEST_SENTENCES.len(), output_dir.display());
    println!("Sentence manifest: {}", manifest_path.display());
    println!();
    println!("Next steps:");
    println!("  1. Listen to each audio file");
    println!("  2. Rate on 1-5 scale: intelligibility, naturalness, pronunciation, prosody");
    println!("  3. Record results (minimum: avg MOS >= 3.5 for Beta)");

    Ok(())
}

/// Show test sentences that will be used for quality assessment.
fn cmd_tts_quality_sentences() -> Result<()> {
    use lexicon_core::tts::QUALITY_TEST_SENTENCES;

    println!("TTS Quality Test Sentences");
    println!("{}", "=".repeat(50));
    println!();
    println!("These {} sentences test various TTS capabilities:", QUALITY_TEST_SENTENCES.len());
    println!();

    for (i, sentence) in QUALITY_TEST_SENTENCES.iter().enumerate() {
        println!("{:2}. {}", i + 1, sentence);
    }

    println!();
    println!("Categories tested:");
    println!("  - Basic conversational speech");
    println!("  - Numbers and dates");
    println!("  - Abbreviations (Dr., Mr., USA, etc.)");
    println!("  - Punctuation and prosody (?, !, ...)");
    println!("  - Technical terms");
    println!("  - Long sentences");
    println!("  - Difficult pronunciations");

    Ok(())
}

/// Display a quality report from a JSON file.
fn cmd_tts_quality_report(report_file: Option<PathBuf>) -> Result<()> {
    use lexicon_core::tts::QualityReport;

    match report_file {
        Some(path) => {
            let content = std::fs::read_to_string(&path)
                .context("Failed to read report file")?;

            let report: QualityReport = serde_json::from_str(&content)
                .context("Failed to parse report JSON")?;

            println!("{}", report.summary());
        }
        None => {
            println!("TTS Quality Report");
            println!("{}", "=".repeat(50));
            println!();
            println!("No report file specified.");
            println!();
            println!("To create a report:");
            println!("  1. Run: lexicon tts-quality generate");
            println!("  2. Listen to audio files and rate each");
            println!("  3. Create a JSON report with QualityReport structure");
            println!("  4. Run: lexicon tts-quality report <report.json>");
            println!();
            println!("Report JSON structure:");
            println!("  {{");
            println!("    \"timestamp\": \"2026-01-23T12:00:00Z\",");
            println!("    \"model_version\": \"v0.19\",");
            println!("    \"engine\": \"Kokoro\",");
            println!("    \"assessments\": [");
            println!("      {{");
            println!("        \"sentence\": \"Hello, how are you?\",");
            println!("        \"intelligibility\": 4,");
            println!("        \"naturalness\": 3,");
            println!("        \"pronunciation\": 4,");
            println!("        \"prosody\": 3,");
            println!("        \"notes\": \"Clear but slightly robotic\"");
            println!("      }}");
            println!("    ]");
            println!("  }}");
            println!();
            println!("Beta criteria: Average MOS >= 3.5, no score below 2.0");
        }
    }

    Ok(())
}
