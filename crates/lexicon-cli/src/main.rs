//! Lexicon Command-Line Interface
//!
//! Provides commands for:
//! - `lexicon transcribe <file>` - Batch transcription
//! - `lexicon serve` - Start HTTP server
//! - `lexicon models list|download|delete` - Model management
//! - `lexicon export <session> --format srt` - Export transcriptions

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use indicatif::{ProgressBar, ProgressStyle};

use lexicon_core::{
    LexiconConfig, ModelRegistry,
    stt::{TranscriptionConfig, TranscriptionResult, Transcriber, WhisperTranscriber},
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
        } => {
            cmd_transcribe(files, format, output, model, language, translate)
        }
        Commands::Serve { host, port } => {
            cmd_serve(host, port)
        }
        Commands::Models { action } => match action {
            ModelAction::List { downloaded } => cmd_models_list(downloaded),
            ModelAction::Download { model_id } => cmd_models_download(&model_id),
            ModelAction::Delete { model_id, force } => cmd_models_delete(&model_id, force),
            ModelAction::Info { model_id } => cmd_models_info(&model_id),
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
    }
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
    eprintln!("Loading model: {model}...");
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .expect("valid template"),
    );
    pb.set_message("Loading model...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut transcriber = WhisperTranscriber::new()?;
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
            eprintln!("Warning: File not found: {}", file.display());
            continue;
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

    // Calculate chunk size for resampling
    let chunk_size = samples.len().min(source_rate as usize); // Up to 1 second chunks
    let mut resampler = Resampler::new(source_rate, 1, chunk_size)?;

    let mut output = Vec::new();
    for chunk in samples.chunks(chunk_size) {
        let resampled = resampler.process(chunk)?;
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

/// List available models.
fn cmd_models_list(downloaded_only: bool) -> Result<()> {
    let config = LexiconConfig::builder().build()?;
    let _registry = ModelRegistry::new(&config.model_dir)?;

    println!("Available Whisper models:\n");
    println!("{:<12} {:<10} {:<12} {}", "Model", "Size", "Status", "Path");
    println!("{}", "-".repeat(60));

    let models = [
        ("tiny", "75 MB"),
        ("base", "142 MB"),
        ("small", "466 MB"),
        ("medium", "1.5 GB"),
        ("large", "2.9 GB"),
    ];

    for (name, size) in models {
        let model_id = format!("whisper-{name}");
        let model_path = config.model_dir.join(format!("{model_id}.bin"));
        let status = if model_path.exists() {
            "Downloaded"
        } else {
            "Not downloaded"
        };

        if downloaded_only && !model_path.exists() {
            continue;
        }

        let path_str = if model_path.exists() {
            model_path.display().to_string()
        } else {
            "-".to_string()
        };

        println!("{:<12} {:<10} {:<12} {}", name, size, status, path_str);
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

    let model_path = config.model_dir.join(format!("{full_model_id}.bin"));
    if model_path.exists() {
        println!("Model '{model_name}' is already downloaded at:");
        println!("  {}", model_path.display());
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
    rt.block_on(async {
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
    println!("\nModel downloaded to: {}", model_path.display());

    Ok(())
}

/// Delete a cached model.
fn cmd_models_delete(model_id: &str, force: bool) -> Result<()> {
    let config = LexiconConfig::builder().build()?;

    // Normalize model ID
    let model_name = model_id.strip_prefix("whisper-").unwrap_or(model_id);
    let full_model_id = format!("whisper-{model_name}");

    let model_path = config.model_dir.join(format!("{full_model_id}.bin"));

    if !model_path.exists() {
        println!("Model '{model_name}' is not downloaded.");
        return Ok(());
    }

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

    let model_path = config.model_dir.join(format!("{full_model_id}.bin"));

    println!("Model: {model_name}");
    println!("ID: {full_model_id}");
    println!("Path: {}", model_path.display());

    if model_path.exists() {
        let metadata = std::fs::metadata(&model_path)?;
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        println!("Status: Downloaded");
        println!("Size: {:.1} MB", size_mb);
    } else {
        println!("Status: Not downloaded");
    }

    // Model capabilities
    let (_languages, multilingual) = match model_name {
        "tiny" | "base" | "small" | "medium" | "large" => (99, true),
        "tiny.en" | "base.en" | "small.en" | "medium.en" => (1, false),
        _ => (99, true),
    };

    println!("Languages: {}", if multilingual { "99 (multilingual)" } else { "English only" });

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
