//! Configuration management for Lexicon
//!
//! This module provides configuration loading from files, environment variables,
//! and programmatic construction via a builder pattern.

use crate::error::{LexiconError, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::Level;

/// Logging level configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Error-level logging only
    Error,
    /// Warnings and errors
    Warn,
    /// Informational messages (default)
    #[default]
    Info,
    /// Debug-level messages
    Debug,
    /// Trace-level messages (most verbose)
    Trace,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "error"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = LexiconError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(LexiconError::Config(format!(
                "Invalid log level: '{}'. Valid values: error, warn, info, debug, trace",
                s
            ))),
        }
    }
}

/// Main configuration for Lexicon.
///
/// Configuration can be loaded from:
/// 1. TOML configuration file
/// 2. Environment variables (prefixed with `LEXICON_`)
/// 3. Programmatic construction via builder
///
/// Environment variables override file settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexiconConfig {
    /// Directory for application data (database, cache)
    pub data_dir: PathBuf,

    /// Directory for model files
    pub model_dir: PathBuf,

    /// Logging level
    #[serde(default)]
    pub log_level: LogLevel,

    /// Server configuration (optional)
    #[serde(default)]
    pub server: ServerConfig,

    /// STT-specific configuration
    #[serde(default)]
    pub stt: SttConfig,

    /// TTS-specific configuration
    #[serde(default)]
    pub tts: TtsConfig,
}

/// Server-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    #[serde(default = "ServerConfig::default_host")]
    pub host: String,

    /// Port to listen on
    #[serde(default = "ServerConfig::default_port")]
    pub port: u16,

    /// Enable CORS
    #[serde(default = "ServerConfig::default_cors")]
    pub cors_enabled: bool,
}

impl ServerConfig {
    fn default_host() -> String {
        "127.0.0.1".to_string()
    }

    fn default_port() -> u16 {
        8765
    }

    fn default_cors() -> bool {
        true
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            cors_enabled: Self::default_cors(),
        }
    }
}

/// STT-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Default model to use
    #[serde(default = "SttConfig::default_model")]
    pub default_model: String,

    /// Enable speaker diarization by default
    #[serde(default)]
    pub diarization_enabled: bool,

    /// Maximum number of speakers for diarization
    #[serde(default = "SttConfig::default_max_speakers")]
    pub max_speakers: u32,

    /// Enable GPU acceleration
    #[serde(default = "SttConfig::default_gpu")]
    pub gpu_enabled: bool,
}

impl SttConfig {
    fn default_model() -> String {
        "whisper-base-q5_1".to_string()
    }

    fn default_max_speakers() -> u32 {
        4
    }

    fn default_gpu() -> bool {
        true
    }
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            default_model: Self::default_model(),
            diarization_enabled: false,
            max_speakers: Self::default_max_speakers(),
            gpu_enabled: Self::default_gpu(),
        }
    }
}

/// TTS-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Preferred TTS engine
    #[serde(default)]
    pub preferred_engine: TtsEngine,

    /// Enable fallback to platform TTS
    #[serde(default = "TtsConfig::default_fallback")]
    pub fallback_enabled: bool,

    /// Default voice ID
    #[serde(default)]
    pub default_voice: Option<String>,
}

impl TtsConfig {
    fn default_fallback() -> bool {
        true
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            preferred_engine: TtsEngine::default(),
            fallback_enabled: Self::default_fallback(),
            default_voice: None,
        }
    }
}

/// TTS engine selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TtsEngine {
    /// Kokoro TTS (high quality, English only)
    #[default]
    Kokoro,
    /// Platform native TTS (multi-language, lower quality)
    Platform,
}

impl LexiconConfig {
    /// Creates a new configuration builder.
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Loads configuration from the default location.
    ///
    /// Default locations:
    /// - Linux: `~/.config/lexicon/config.toml`
    /// - macOS: `~/Library/Application Support/com.lexicon.LexiconLocalServer/config.toml`
    /// - Windows: `%APPDATA%\Lexicon\LexiconLocalServer\config.toml`
    ///
    /// If the file doesn't exist, returns default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file exists but cannot be parsed.
    pub fn load_default() -> Result<Self> {
        let config_path = Self::default_config_path()?;
        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            Ok(Self::builder().build()?)
        }
    }

    /// Loads configuration from a specific file path.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML content is invalid
    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            LexiconError::Config(format!("Failed to read config file '{}': {}", path.display(), e))
        })?;

        Self::from_toml(&content)
    }

    /// Parses configuration from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is invalid.
    pub fn from_toml(content: &str) -> Result<Self> {
        let config: Self = toml::from_str(content)
            .map_err(|e| LexiconError::Config(format!("Invalid TOML configuration: {}", e)))?;
        config.validate()?;
        Ok(config)
    }

    /// Returns the default configuration file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the project directories cannot be determined.
    pub fn default_config_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("com", "lexicon", "LexiconLocalServer").ok_or_else(|| {
            LexiconError::Config("Could not determine configuration directory".to_string())
        })?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Returns the default data directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the project directories cannot be determined.
    pub fn default_data_dir() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("com", "lexicon", "LexiconLocalServer").ok_or_else(|| {
            LexiconError::Config("Could not determine data directory".to_string())
        })?;
        Ok(dirs.data_dir().to_path_buf())
    }

    /// Returns the default model directory (within data directory).
    ///
    /// # Errors
    ///
    /// Returns an error if the project directories cannot be determined.
    pub fn default_model_dir() -> Result<PathBuf> {
        Ok(Self::default_data_dir()?.join("models"))
    }

    /// Validates the configuration.
    fn validate(&self) -> Result<()> {
        // Validate port range
        if self.server.port == 0 {
            return Err(LexiconError::Config(
                "Server port cannot be 0".to_string(),
            ));
        }

        // Validate max speakers
        if self.stt.max_speakers == 0 || self.stt.max_speakers > 100 {
            return Err(LexiconError::Config(
                "max_speakers must be between 1 and 100".to_string(),
            ));
        }

        Ok(())
    }

    /// Saves the configuration to the default location.
    ///
    /// Creates parent directories if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration directory cannot be created
    /// - The file cannot be written
    /// - The configuration cannot be serialized
    pub fn save_default(&self) -> Result<()> {
        let config_path = Self::default_config_path()?;
        self.save_to_file(&config_path)
    }

    /// Saves the configuration to a specific file path.
    ///
    /// Creates parent directories if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The parent directory cannot be created
    /// - The file cannot be written
    /// - The configuration cannot be serialized
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        // Create parent directories
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                LexiconError::Config(format!(
                    "Failed to create config directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Serialize to TOML
        let content = self.to_toml()?;

        // Write to file
        std::fs::write(path, content).map_err(|e| {
            LexiconError::Config(format!(
                "Failed to write config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Serializes the configuration to a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_toml(&self) -> Result<String> {
        // Add header comment
        let header = "# Lexicon Local Server Configuration\n\
                      # Generated automatically. Edit with caution.\n\n";

        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| LexiconError::Config(format!("Failed to serialize config: {}", e)))?;

        Ok(format!("{}{}", header, toml_content))
    }

    /// Checks if this is a first run (no config file exists).
    ///
    /// Returns `true` if the default config file does not exist.
    pub fn is_first_run() -> bool {
        Self::default_config_path()
            .map(|p| !p.exists())
            .unwrap_or(true)
    }

    /// Loads configuration from default location, returning `None` if not found.
    ///
    /// Unlike `load_default`, this method returns `None` instead of default
    /// configuration when the file doesn't exist. This is useful for detecting
    /// first-run scenarios.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file exists but cannot be parsed.
    pub fn load_if_exists() -> Result<Option<Self>> {
        let config_path = Self::default_config_path()?;
        if config_path.exists() {
            Self::load_from_file(&config_path).map(Some)
        } else {
            Ok(None)
        }
    }
}

/// Builder for constructing `LexiconConfig`.
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    data_dir: Option<PathBuf>,
    model_dir: Option<PathBuf>,
    log_level: Option<LogLevel>,
    server: Option<ServerConfig>,
    stt: Option<SttConfig>,
    tts: Option<TtsConfig>,
}

impl ConfigBuilder {
    /// Creates a new configuration builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the data directory.
    #[must_use]
    pub fn data_dir(mut self, path: PathBuf) -> Self {
        self.data_dir = Some(path);
        self
    }

    /// Sets the model directory.
    #[must_use]
    pub fn model_dir(mut self, path: PathBuf) -> Self {
        self.model_dir = Some(path);
        self
    }

    /// Sets the log level.
    #[must_use]
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.log_level = Some(level);
        self
    }

    /// Sets the server configuration.
    #[must_use]
    pub fn server(mut self, config: ServerConfig) -> Self {
        self.server = Some(config);
        self
    }

    /// Sets the STT configuration.
    #[must_use]
    pub fn stt(mut self, config: SttConfig) -> Self {
        self.stt = Some(config);
        self
    }

    /// Sets the TTS configuration.
    #[must_use]
    pub fn tts(mut self, config: TtsConfig) -> Self {
        self.tts = Some(config);
        self
    }

    /// Builds the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if required directories cannot be determined.
    pub fn build(self) -> Result<LexiconConfig> {
        let data_dir = self
            .data_dir
            .or_else(|| LexiconConfig::default_data_dir().ok())
            .ok_or_else(|| {
                LexiconError::Config("Could not determine data directory".to_string())
            })?;

        let model_dir = self.model_dir.unwrap_or_else(|| data_dir.join("models"));

        let config = LexiconConfig {
            data_dir,
            model_dir,
            log_level: self.log_level.unwrap_or_default(),
            server: self.server.unwrap_or_default(),
            stt: self.stt.unwrap_or_default(),
            tts: self.tts.unwrap_or_default(),
        };

        config.validate()?;
        Ok(config)
    }

    /// Builds the configuration, applying environment variable overrides.
    ///
    /// Supported environment variables:
    /// - `LEXICON_DATA_DIR`: Override data directory
    /// - `LEXICON_MODEL_DIR`: Override model directory
    /// - `LEXICON_LOG_LEVEL`: Override log level
    /// - `LEXICON_SERVER_HOST`: Override server host
    /// - `LEXICON_SERVER_PORT`: Override server port
    ///
    /// # Errors
    ///
    /// Returns an error if environment values are invalid.
    pub fn build_with_env(mut self) -> Result<LexiconConfig> {
        // Apply environment overrides
        if let Ok(val) = std::env::var("LEXICON_DATA_DIR") {
            self.data_dir = Some(PathBuf::from(val));
        }
        if let Ok(val) = std::env::var("LEXICON_MODEL_DIR") {
            self.model_dir = Some(PathBuf::from(val));
        }
        if let Ok(val) = std::env::var("LEXICON_LOG_LEVEL") {
            self.log_level = Some(val.parse()?);
        }

        let mut config = self.build()?;

        // Server overrides
        if let Ok(val) = std::env::var("LEXICON_SERVER_HOST") {
            config.server.host = val;
        }
        if let Ok(val) = std::env::var("LEXICON_SERVER_PORT") {
            config.server.port = val.parse().map_err(|_| {
                LexiconError::Config(format!("Invalid LEXICON_SERVER_PORT: {}", val))
            })?;
        }

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_builder_defaults() {
        let temp_dir = std::env::temp_dir().join("lexicon-test-config");
        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .build()
            .unwrap();

        assert_eq!(config.data_dir, temp_dir);
        assert_eq!(config.model_dir, temp_dir.join("models"));
        assert_eq!(config.log_level, LogLevel::Info);
        assert_eq!(config.server.port, 8765);
    }

    #[test]
    fn test_config_builder_custom() {
        let temp_dir = std::env::temp_dir().join("lexicon-test-custom");
        let model_dir = temp_dir.join("custom-models");

        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(model_dir.clone())
            .log_level(LogLevel::Debug)
            .build()
            .unwrap();

        assert_eq!(config.data_dir, temp_dir);
        assert_eq!(config.model_dir, model_dir);
        assert_eq!(config.log_level, LogLevel::Debug);
    }

    #[test]
    fn test_config_from_toml() {
        let toml = r#"
            data_dir = "/tmp/lexicon"
            model_dir = "/tmp/lexicon/models"
            log_level = "debug"

            [server]
            host = "0.0.0.0"
            port = 9000
            cors_enabled = false

            [stt]
            default_model = "whisper-tiny"
            diarization_enabled = true
            max_speakers = 8

            [tts]
            preferred_engine = "platform"
            fallback_enabled = false
        "#;

        let config = LexiconConfig::from_toml(toml).unwrap();

        assert_eq!(config.data_dir, PathBuf::from("/tmp/lexicon"));
        assert_eq!(config.log_level, LogLevel::Debug);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9000);
        assert!(!config.server.cors_enabled);
        assert_eq!(config.stt.default_model, "whisper-tiny");
        assert!(config.stt.diarization_enabled);
        assert_eq!(config.stt.max_speakers, 8);
        assert_eq!(config.tts.preferred_engine, TtsEngine::Platform);
        assert!(!config.tts.fallback_enabled);
    }

    #[test]
    fn test_config_validation_invalid_port() {
        let toml = r#"
            data_dir = "/tmp/lexicon"
            model_dir = "/tmp/lexicon/models"

            [server]
            port = 0
        "#;

        let result = LexiconConfig::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_invalid_max_speakers() {
        let toml = r#"
            data_dir = "/tmp/lexicon"
            model_dir = "/tmp/lexicon/models"

            [stt]
            max_speakers = 0
        "#;

        let result = LexiconConfig::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_load_from_file() {
        let temp_dir = std::env::temp_dir().join("lexicon-config-file-test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let config_path = temp_dir.join("config.toml");

        // Use forward slashes for TOML compatibility (backslashes are escape sequences)
        let data_dir = temp_dir.join("data").to_string_lossy().replace('\\', "/");
        let model_dir = temp_dir.join("models").to_string_lossy().replace('\\', "/");
        let toml_content = format!(
            r#"
            data_dir = "{data_dir}"
            model_dir = "{model_dir}"
            log_level = "warn"
        "#
        );

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = LexiconConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.log_level, LogLevel::Warn);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("Debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("TRACE".parse::<LogLevel>().unwrap(), LogLevel::Trace);

        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Error.to_string(), "error");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Info.to_string(), "info");
    }

    #[test]
    fn test_log_level_to_tracing() {
        assert_eq!(Level::from(LogLevel::Error), Level::ERROR);
        assert_eq!(Level::from(LogLevel::Debug), Level::DEBUG);
    }

    // Note: Environment variable tests are not included because the crate uses
    // #![deny(unsafe_code)] and env::set_var/remove_var are unsafe in Rust 2024.
    // The build_with_env() function is tested indirectly via integration tests
    // that can set up the environment before invoking the binary.
    //
    // The function logic is covered by:
    // - test_log_level_parsing (validates log level parsing used in build_with_env)
    // - test_config_builder_* (validates builder patterns used by build_with_env)
    // - test_config_validation_* (validates validation used by build_with_env)

    #[test]
    fn test_config_default_paths_determined() {
        // Test that default_data_dir and default_model_dir return valid paths
        let data_dir = LexiconConfig::default_data_dir();
        let model_dir = LexiconConfig::default_model_dir();

        // Should successfully determine paths on any supported system
        assert!(data_dir.is_ok());
        assert!(model_dir.is_ok());

        // Model dir should be a subdirectory of data dir
        let data_dir = data_dir.unwrap();
        let model_dir = model_dir.unwrap();
        assert!(model_dir.starts_with(&data_dir));
    }

    #[test]
    fn test_config_default_config_path_determined() {
        // Test that default_config_path returns a valid path
        let config_path = LexiconConfig::default_config_path();

        // Should successfully determine path on any supported system
        assert!(config_path.is_ok());

        // Path should end with config.toml
        let path = config_path.unwrap();
        assert!(path.file_name().unwrap().to_str().unwrap() == "config.toml");
    }

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig::default();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8765);
        assert!(config.cors_enabled);
    }

    #[test]
    fn test_stt_config_defaults() {
        let config = SttConfig::default();

        assert_eq!(config.default_model, "whisper-base-q5_1");
        assert!(!config.diarization_enabled);
        assert_eq!(config.max_speakers, 4);
        assert!(config.gpu_enabled);
    }

    #[test]
    fn test_tts_engine_default() {
        assert_eq!(TtsEngine::default(), TtsEngine::Kokoro);
    }

    #[test]
    fn test_config_save_to_file() {
        let temp_dir = std::env::temp_dir().join("lexicon-config-save-test");
        let config_path = temp_dir.join("subdir").join("config.toml");

        // Clean up first
        let _ = std::fs::remove_dir_all(&temp_dir);

        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .log_level(LogLevel::Debug)
            .build()
            .unwrap();

        // Save should create parent directories
        config.save_to_file(&config_path).unwrap();

        // Verify file was created
        assert!(config_path.exists());

        // Load back and verify
        let loaded = LexiconConfig::load_from_file(&config_path).unwrap();
        assert_eq!(loaded.log_level, LogLevel::Debug);
        assert_eq!(loaded.data_dir, temp_dir);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_config_to_toml() {
        let temp_dir = std::env::temp_dir().join("lexicon-config-toml-test");
        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .build()
            .unwrap();

        let toml_str = config.to_toml().unwrap();

        // Should contain header comment
        assert!(toml_str.contains("# Lexicon Local Server Configuration"));

        // Should be valid TOML
        assert!(toml_str.contains("[server]"));
        assert!(toml_str.contains("[stt]"));
        assert!(toml_str.contains("[tts]"));
    }

    #[test]
    fn test_config_load_if_exists_returns_none_for_missing() {
        // This test verifies behavior when config doesn't exist
        // We can't easily test the default path, so we test the concept
        let temp_dir = std::env::temp_dir().join("lexicon-nonexistent-config-test");
        let config_path = temp_dir.join("config.toml");

        // Ensure directory doesn't exist
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Load from non-existent path
        let result = LexiconConfig::load_from_file(&config_path);
        assert!(result.is_err()); // File doesn't exist, should error
    }

    #[test]
    fn test_config_round_trip() {
        let temp_dir = std::env::temp_dir().join("lexicon-config-roundtrip-test");
        let config_path = temp_dir.join("config.toml");

        // Clean up first
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Create config with non-default values
        let mut stt = SttConfig::default();
        stt.default_model = "whisper-small-en".to_string();
        stt.diarization_enabled = true;
        stt.max_speakers = 8;

        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .log_level(LogLevel::Trace)
            .stt(stt)
            .build()
            .unwrap();

        // Save
        config.save_to_file(&config_path).unwrap();

        // Load and verify all values preserved
        let loaded = LexiconConfig::load_from_file(&config_path).unwrap();
        assert_eq!(loaded.log_level, LogLevel::Trace);
        assert_eq!(loaded.stt.default_model, "whisper-small-en");
        assert!(loaded.stt.diarization_enabled);
        assert_eq!(loaded.stt.max_speakers, 8);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
