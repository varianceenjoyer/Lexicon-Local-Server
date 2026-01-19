//! Lexicon Command-Line Interface
//!
//! Provides commands for:
//! - `lexicon transcribe <file>` - Batch transcription
//! - `lexicon serve` - Start HTTP server
//! - `lexicon models list|download|delete` - Model management
//! - `lexicon export <session> --format srt` - Export transcriptions

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lexicon")]
#[command(author, version, about = "Real-time STT, translation, and TTS", long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<std::path::PathBuf>,

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
        files: Vec<std::path::PathBuf>,

        /// Output format (srt, txt, json)
        #[arg(short, long, default_value = "srt")]
        format: String,

        /// Output file path (stdout if not specified)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
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
        output: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// List available models
    List,

    /// Download a model
    Download {
        /// Model ID to download
        model_id: String,
    },

    /// Delete a cached model
    Delete {
        /// Model ID to delete
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

fn main() {
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    let log_level = match cli.verbose {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Execute command
    match cli.command {
        Commands::Transcribe { files, format, output } => {
            tracing::info!("Transcribing {} file(s) to {} format", files.len(), format);
            // Implementation in WP-CLI
            eprintln!("Transcribe command not yet implemented");
            std::process::exit(1);
        }
        Commands::Serve { host, port } => {
            tracing::info!("Starting server on {}:{}", host, port);
            // Implementation in WP-SERVER/WP-CLI
            eprintln!("Serve command not yet implemented");
            std::process::exit(1);
        }
        Commands::Models { action } => match action {
            ModelAction::List => {
                tracing::info!("Listing models");
                eprintln!("Models list not yet implemented");
                std::process::exit(1);
            }
            ModelAction::Download { model_id } => {
                tracing::info!("Downloading model: {}", model_id);
                eprintln!("Model download not yet implemented");
                std::process::exit(1);
            }
            ModelAction::Delete { model_id } => {
                tracing::info!("Deleting model: {}", model_id);
                eprintln!("Model delete not yet implemented");
                std::process::exit(1);
            }
        },
        Commands::Sessions { action } => match action {
            SessionAction::List => {
                tracing::info!("Listing sessions");
                eprintln!("Sessions list not yet implemented");
                std::process::exit(1);
            }
            SessionAction::Show { session_id } => {
                tracing::info!("Showing session: {}", session_id);
                eprintln!("Session show not yet implemented");
                std::process::exit(1);
            }
            SessionAction::Delete { session_id } => {
                tracing::info!("Deleting session: {}", session_id);
                eprintln!("Session delete not yet implemented");
                std::process::exit(1);
            }
        },
        Commands::Export { session_id, format, output } => {
            tracing::info!("Exporting session {} to {} format", session_id, format);
            // Implementation in WP-CLI
            eprintln!("Export command not yet implemented");
            std::process::exit(1);
        }
    }
}
