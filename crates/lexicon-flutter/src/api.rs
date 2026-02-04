//! Flutter Rust Bridge API for Lexicon.
//!
//! This module provides the public API surface for Flutter/Dart applications.
//! Functions annotated with `#[frb]` macros are exposed to Dart.
//!
//! # Streaming APIs
//!
//! This module provides both batch and streaming APIs:
//! - **Batch**: `transcribe()`, `synthesize()` - process complete data
//! - **Streaming**: `transcribe_stream()`, `synthesize_stream()` - real-time processing
//!
//! Streaming uses `StreamSink` to send data from Rust to Dart asynchronously.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use flutter_rust_bridge::frb;

use crate::error::LexiconFlutterError;
use crate::types::*;

// ============================================================================
// Global State
// ============================================================================

/// Global initialization state.
static INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DATA_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

// ============================================================================
// Model Cache
// ============================================================================

// Internal types - not exposed via FFI (module-private)
#[frb(ignore)]
mod cache_internal {
    use super::*;

    /// Cached model with metadata for memory management.
    #[cfg(feature = "stt-sherpa")]
    pub(super) struct CachedSherpaModel {
        pub model: Arc<Mutex<lexicon_core::stt::SherpaTranscriber>>,
        pub path: String,
        pub size_bytes: usize,
        pub last_used: RwLock<Instant>,
    }

    /// Cached TTS model with metadata.
    #[cfg(feature = "tts")]
    pub(super) struct CachedTtsModel {
        pub model: Arc<Mutex<lexicon_core::tts::TtsManager>>,
        pub path: String,
        pub size_bytes: usize,
        pub last_used: RwLock<Instant>,
    }

    /// Global model cache for session persistence.
    pub(super) struct ModelCache {
        #[cfg(feature = "stt-sherpa")]
        pub sherpa_model: Option<CachedSherpaModel>,
        #[cfg(feature = "tts")]
        pub tts_model: Option<CachedTtsModel>,
        /// Maximum cache size in bytes (default: 500MB).
        pub max_memory_bytes: usize,
    }

    impl Default for ModelCache {
        fn default() -> Self {
            Self {
                #[cfg(feature = "stt-sherpa")]
                sherpa_model: None,
                #[cfg(feature = "tts")]
                tts_model: None,
                max_memory_bytes: 500 * 1024 * 1024, // 500MB default
            }
        }
    }

    impl ModelCache {
        pub fn total_size_bytes(&self) -> usize {
            let mut total = 0;
            #[cfg(feature = "stt-sherpa")]
            if let Some(ref m) = self.sherpa_model {
                total += m.size_bytes;
            }
            #[cfg(feature = "tts")]
            if let Some(ref m) = self.tts_model {
                total += m.size_bytes;
            }
            total
        }

        /// Evict least-recently-used model if over memory limit.
        pub fn evict_if_needed(&mut self) {
            if self.total_size_bytes() <= self.max_memory_bytes {
                return;
            }

            #[cfg(all(feature = "stt-sherpa", feature = "tts"))]
            {
                let sherpa_time = self
                    .sherpa_model
                    .as_ref()
                    .and_then(|m| m.last_used.read().ok().map(|g| *g));
                let tts_time = self
                    .tts_model
                    .as_ref()
                    .and_then(|m| m.last_used.read().ok().map(|g| *g));

                match (sherpa_time, tts_time) {
                    (Some(s), Some(t)) if s < t => {
                        tracing::info!("Evicting Sherpa model from cache (LRU)");
                        self.sherpa_model = None;
                    }
                    (Some(_), Some(_)) => {
                        tracing::info!("Evicting TTS model from cache (LRU)");
                        self.tts_model = None;
                    }
                    (Some(_), None) => {
                        tracing::info!("Evicting Sherpa model from cache");
                        self.sherpa_model = None;
                    }
                    (None, Some(_)) => {
                        tracing::info!("Evicting TTS model from cache");
                        self.tts_model = None;
                    }
                    _ => {}
                }
            }

            #[cfg(all(feature = "stt-sherpa", not(feature = "tts")))]
            {
                self.sherpa_model = None;
            }

            #[cfg(all(feature = "tts", not(feature = "stt-sherpa")))]
            {
                self.tts_model = None;
            }
        }
    }
}

#[cfg(feature = "stt-sherpa")]
use cache_internal::CachedSherpaModel;
use cache_internal::ModelCache;

static MODEL_CACHE: RwLock<Option<ModelCache>> = RwLock::new(None);

/// Get or initialize the model cache.
fn get_cache() -> std::sync::RwLockWriteGuard<'static, Option<ModelCache>> {
    let mut cache = MODEL_CACHE.write().unwrap();
    if cache.is_none() {
        *cache = Some(ModelCache::default());
    }
    cache
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the flutter_rust_bridge runtime.
/// Must be called before any other functions.
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Initialize the Lexicon library with data directory.
///
/// This must be called before using transcription or synthesis features.
/// It's safe to call multiple times (subsequent calls are no-ops).
#[frb(sync)]
pub fn init_lexicon(data_dir: String) -> Result<(), LexiconFlutterError> {
    if INITIALIZED.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }

    let path = PathBuf::from(&data_dir);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&path).map_err(|e| {
        LexiconFlutterError::InitializationFailed(format!("Failed to create data directory: {e}"))
    })?;

    // Store the data directory
    {
        let mut dir = DATA_DIR.write().map_err(|_| {
            LexiconFlutterError::InitializationFailed("Failed to acquire lock".to_string())
        })?;
        *dir = Some(path);
    }

    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    INITIALIZED.store(true, std::sync::atomic::Ordering::SeqCst);
    tracing::info!("Lexicon Flutter initialized with data_dir: {}", data_dir);

    Ok(())
}

/// Get the library version.
#[frb(sync)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ============================================================================
// Platform Info
// ============================================================================

/// Get platform information including recommended model tier.
#[frb(sync)]
pub fn get_platform_info() -> PlatformInfo {
    let platform = lexicon_core::Platform::detect();
    let profile = lexicon_core::DeviceProfile::detect();

    PlatformInfo {
        platform: format!("{:?}", platform),
        model_tier: format!("{:?}", profile.recommended_tier()),
        available_memory_mb: profile.available_memory_mb,
        has_gpu: profile.has_gpu,
    }
}

// ============================================================================
// Model Cache Management
// ============================================================================

/// Preload Sherpa STT model into cache for faster first use.
///
/// Call this at app startup to eliminate model load delay on first transcription.
/// Returns the load time in milliseconds, or 0 if already cached.
#[frb]
pub fn preload_sherpa_model(model_path: String) -> Result<i64, LexiconFlutterError> {
    #[cfg(feature = "stt-sherpa")]
    {
        use lexicon_core::stt::Transcriber as _;

        let start = Instant::now();

        let mut cache = get_cache();
        let cache = cache.as_mut().unwrap();

        // Check if already cached with same path
        if let Some(ref cached) = cache.sherpa_model {
            if cached.path == model_path {
                tracing::info!("Sherpa model already cached: {}", model_path);
                return Ok(0);
            }
        }

        // Load model
        let path = PathBuf::from(&model_path);
        if !path.exists() {
            return Err(LexiconFlutterError::ModelNotFound(model_path));
        }

        let mut transcriber = lexicon_core::stt::SherpaTranscriber::new()
            .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

        transcriber
            .load_model(&path)
            .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

        // Warm up to prime internal caches
        let _ = transcriber.warm_up();

        // Estimate memory size (~150MB for typical Sherpa model)
        let size_bytes = 150 * 1024 * 1024;

        cache.sherpa_model = Some(CachedSherpaModel {
            model: Arc::new(Mutex::new(transcriber)),
            path: model_path.clone(),
            size_bytes,
            last_used: RwLock::new(Instant::now()),
        });

        // Evict if over limit
        cache.evict_if_needed();

        let elapsed_ms = start.elapsed().as_millis() as i64;
        tracing::info!(
            elapsed_ms = elapsed_ms,
            path = %model_path,
            "Sherpa model preloaded into cache"
        );

        Ok(elapsed_ms)
    }

    #[cfg(not(feature = "stt-sherpa"))]
    {
        let _ = model_path;
        Err(LexiconFlutterError::ModelLoadFailed(
            "stt-sherpa feature not enabled".to_string(),
        ))
    }
}

/// Set maximum cache memory in bytes.
///
/// Default is 500MB. When exceeded, least-recently-used model is evicted.
#[frb(sync)]
pub fn set_cache_max_memory(max_bytes: u64) {
    let mut cache = get_cache();
    if let Some(ref mut c) = *cache {
        c.max_memory_bytes = max_bytes as usize;
        c.evict_if_needed();
        tracing::info!(max_bytes = max_bytes, "Cache max memory updated");
    }
}

/// Clear all cached models to free memory.
///
/// Call this when the OS signals low memory or when backgrounding the app.
#[frb(sync)]
pub fn clear_model_cache() {
    let mut cache = get_cache();
    if let Some(ref mut c) = *cache {
        #[cfg(feature = "stt-sherpa")]
        {
            c.sherpa_model = None;
        }
        #[cfg(feature = "tts")]
        {
            c.tts_model = None;
        }
        tracing::info!("Model cache cleared");
    }
}

/// Handle low memory warning from OS.
///
/// Clears the entire model cache to free RAM immediately.
/// Call this from Flutter's `didHaveMemoryPressure` callback.
#[frb(sync)]
pub fn on_low_memory() {
    clear_model_cache();
    tracing::warn!("Model cache cleared due to low memory warning");
}

/// Get current cache statistics.
#[frb(sync)]
pub fn get_cache_stats() -> CacheStats {
    let guard = match MODEL_CACHE.read() {
        Ok(g) => g,
        Err(_) => return CacheStats::default(),
    };

    let cache = match guard.as_ref() {
        Some(c) => c,
        None => return CacheStats::default(),
    };

    let mut stats = CacheStats {
        total_bytes: cache.total_size_bytes() as u64,
        max_bytes: cache.max_memory_bytes as u64,
        ..Default::default()
    };

    #[cfg(feature = "stt-sherpa")]
    if let Some(ref m) = cache.sherpa_model {
        stats.sherpa_cached = true;
        stats.sherpa_bytes = m.size_bytes as u64;
    }

    #[cfg(feature = "tts")]
    if let Some(ref m) = cache.tts_model {
        stats.tts_cached = true;
        stats.tts_bytes = m.size_bytes as u64;
    }

    stats
}

/// Check if a specific model type is cached.
#[frb(sync)]
pub fn is_model_cached(model_type: String) -> bool {
    let guard = match MODEL_CACHE.read() {
        Ok(g) => g,
        Err(_) => return false,
    };

    let cache = match guard.as_ref() {
        Some(c) => c,
        None => return false,
    };

    match model_type.as_str() {
        #[cfg(feature = "stt-sherpa")]
        "sherpa" => cache.sherpa_model.is_some(),
        #[cfg(feature = "tts")]
        "tts" => cache.tts_model.is_some(),
        _ => false,
    }
}

// ============================================================================
// Model Management
// ============================================================================

/// List available models.
pub fn list_models(model_dir: String) -> Result<Vec<ModelInfo>, LexiconFlutterError> {
    let model_path = PathBuf::from(&model_dir);

    let models = [
        ("whisper-tiny", "Whisper Tiny", "stt", 75_000_000u64),
        ("whisper-base", "Whisper Base", "stt", 142_000_000),
        ("whisper-small", "Whisper Small", "stt", 466_000_000),
        ("whisper-medium", "Whisper Medium", "stt", 1_500_000_000),
        ("whisper-large", "Whisper Large", "stt", 2_900_000_000),
    ];

    let results: Vec<ModelInfo> = models
        .iter()
        .map(|(id, name, model_type, size)| {
            let path = model_path.join(format!("{id}.bin"));
            let downloaded = path.exists();
            ModelInfo {
                id: id.to_string(),
                name: name.to_string(),
                model_type: model_type.to_string(),
                size_bytes: *size,
                downloaded,
                local_path: if downloaded {
                    Some(path.display().to_string())
                } else {
                    None
                },
            }
        })
        .collect();

    Ok(results)
}

/// Check if a model is downloaded.
#[frb(sync)]
pub fn is_model_downloaded(model_dir: String, model_id: String) -> bool {
    let model_path = PathBuf::from(&model_dir).join(format!("{model_id}.bin"));
    model_path.exists()
}

/// Delete a downloaded model.
pub fn delete_model(model_dir: String, model_id: String) -> Result<(), LexiconFlutterError> {
    let model_path = PathBuf::from(&model_dir).join(format!("{model_id}.bin"));

    if !model_path.exists() {
        return Err(LexiconFlutterError::ModelNotFound(model_id));
    }

    std::fs::remove_file(&model_path)?;
    tracing::info!("Deleted model: {}", model_id);

    Ok(())
}

// ============================================================================
// Transcriber
// ============================================================================

/// Transcriber state held in Dart via opaque handle.
#[frb(opaque)]
pub struct Transcriber {
    #[cfg(feature = "stt")]
    inner: Mutex<Option<lexicon_core::stt::WhisperTranscriber>>,
    #[cfg(not(feature = "stt"))]
    _marker: std::marker::PhantomData<()>,
}

impl Transcriber {
    /// Create a new transcriber.
    pub fn new() -> Result<Transcriber, LexiconFlutterError> {
        Ok(Self {
            #[cfg(feature = "stt")]
            inner: Mutex::new(None),
            #[cfg(not(feature = "stt"))]
            _marker: std::marker::PhantomData,
        })
    }

    /// Load a Whisper model.
    pub fn load_model(&self, model_path: String) -> Result<(), LexiconFlutterError> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::Transcriber as _;

            let path = PathBuf::from(&model_path);
            if !path.exists() {
                return Err(LexiconFlutterError::ModelNotFound(model_path));
            }

            let mut transcriber = lexicon_core::stt::WhisperTranscriber::new()
                .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

            transcriber
                .load_model(&path)
                .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

            let mut inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;
            *inner = Some(transcriber);

            tracing::info!("Model loaded: {}", model_path);
            Ok(())
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "STT feature not enabled".to_string(),
        ))
    }

    /// Unload the current model.
    #[frb(sync)]
    pub fn unload_model(&self) {
        #[cfg(feature = "stt")]
        {
            if let Ok(mut inner) = self.inner.lock() {
                *inner = None;
                tracing::info!("Model unloaded");
            }
        }
    }

    /// Check if a model is loaded.
    #[frb(sync)]
    pub fn is_model_loaded(&self) -> bool {
        #[cfg(feature = "stt")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "stt"))]
        false
    }

    /// Get the loaded model name.
    #[frb(sync)]
    pub fn get_model_name(&self) -> Option<String> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::Transcriber as _;

            self.inner
                .lock()
                .ok()
                .and_then(|inner| inner.as_ref().and_then(|t| t.model_name().map(String::from)))
        }

        #[cfg(not(feature = "stt"))]
        None
    }

    /// Warms up the model by running a short inference on silence.
    ///
    /// **Latency Optimization (WP-LATENCY-STT)**: This primes the model's
    /// internal caches and eliminates the "first inference" latency penalty
    /// that can add 100-500ms to the first transcription.
    ///
    /// Call this method after loading a model and before real transcription
    /// for optimal latency on the first request.
    ///
    /// Returns the warm-up time in milliseconds for diagnostics.
    pub fn warm_up(&self) -> Result<i64, LexiconFlutterError> {
        #[cfg(feature = "stt")]
        {
            let start = Instant::now();

            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;

            let transcriber = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::TranscriptionFailed("No model loaded".to_string())
            })?;

            transcriber
                .warm_up()
                .map_err(|e| LexiconFlutterError::TranscriptionFailed(e.to_string()))?;

            let elapsed_ms = start.elapsed().as_millis() as i64;
            tracing::info!(elapsed_ms = elapsed_ms, "Model warm-up complete");

            Ok(elapsed_ms)
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "STT feature not enabled".to_string(),
        ))
    }

    /// Transcribe audio data (batch mode).
    ///
    /// Audio should be raw PCM f32 samples at 16kHz mono, packed as bytes.
    pub fn transcribe(
        &self,
        audio_data: Vec<u8>,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult, LexiconFlutterError> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::{PostProcessor, Transcriber as _};

            // Convert bytes to f32 samples
            let samples: Vec<f32> = audio_data
                .chunks(4)
                .filter_map(|chunk| {
                    if chunk.len() == 4 {
                        Some(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    } else {
                        None
                    }
                })
                .collect();

            if samples.is_empty() {
                return Err(LexiconFlutterError::InvalidInput(
                    "Empty audio data".to_string(),
                ));
            }

            // Build config
            let mut tx_config = lexicon_core::stt::TranscriptionConfig::new();
            if let Some(lang) = config.language {
                tx_config = tx_config.with_language(lang);
            }
            if config.translate {
                tx_config = tx_config.with_translation();
            }
            if config.num_threads > 0 {
                tx_config = tx_config.with_threads(config.num_threads);
            }
            if let Some(prompt) = config.initial_prompt {
                tx_config = tx_config.with_prompt(prompt);
            }
            tx_config = tx_config.with_temperature(config.temperature);

            // Transcribe
            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;

            let transcriber = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::TranscriptionFailed("No model loaded".to_string())
            })?;

            let result = transcriber
                .transcribe(&samples, &tx_config)
                .map_err(|e| LexiconFlutterError::TranscriptionFailed(e.to_string()))?;

            // Apply post-processing to improve transcription quality
            let post_processor = PostProcessor::new(Default::default());

            // Convert to FFI result
            let segments: Vec<TranscriptionSegment> = result
                .segments
                .iter()
                .map(|s| TranscriptionSegment {
                    start_ms: s.start_ms,
                    end_ms: s.end_ms,
                    text: post_processor.process(&s.text),
                    confidence: s.confidence,
                    language: s.language.clone(),
                    speaker_id: s.speaker_id,
                })
                .collect();

            Ok(TranscriptionResult {
                segments,
                detected_language: result.detected_language,
                audio_duration_ms: result.audio_duration_ms,
                processing_time_ms: result.processing_time_ms,
            })
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "STT feature not enabled".to_string(),
        ))
    }

    /// Translate audio to English (batch mode).
    pub fn translate(
        &self,
        audio_data: Vec<u8>,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult, LexiconFlutterError> {
        let mut translate_config = config;
        translate_config.translate = true;
        self.transcribe(audio_data, translate_config)
    }
}

// ============================================================================
// Sherpa Transcriber (Low-Latency STT)
// ============================================================================

/// Sherpa-based transcriber for low-latency speech-to-text.
///
/// Uses sherpa-onnx with Zipformer architecture for faster inference.
/// Recommended for real-time transcription where latency is critical.
///
/// # Latency Comparison
///
/// On ARM64 physical device:
/// - Whisper (tiny-q5_1): ~3250ms per 3s audio
/// - Sherpa (Zipformer-20M): ~300-500ms per 3s audio (expected)
///
/// # Model Format
///
/// Unlike WhisperTranscriber which uses a single .bin file, SherpaTranscriber
/// requires a model directory containing:
/// - `encoder-*.onnx` - Audio encoder
/// - `decoder-*.onnx` - Token decoder
/// - `joiner-*.onnx` - Encoder-decoder connector
/// - `tokens.txt` - Vocabulary
///
/// Use `scripts/download-sherpa-model.sh` to download models.
#[frb(opaque)]
pub struct SherpaTranscriber {
    #[cfg(feature = "stt-sherpa")]
    inner: Mutex<Option<Arc<Mutex<lexicon_core::stt::SherpaTranscriber>>>>,
    #[cfg(feature = "stt-sherpa")]
    model_path: Mutex<Option<String>>,
    #[cfg(not(feature = "stt-sherpa"))]
    _marker: std::marker::PhantomData<()>,
}

impl SherpaTranscriber {
    /// Create a new Sherpa transcriber.
    pub fn new() -> Result<SherpaTranscriber, LexiconFlutterError> {
        Ok(Self {
            #[cfg(feature = "stt-sherpa")]
            inner: Mutex::new(None),
            #[cfg(feature = "stt-sherpa")]
            model_path: Mutex::new(None),
            #[cfg(not(feature = "stt-sherpa"))]
            _marker: std::marker::PhantomData,
        })
    }

    /// Load a Sherpa model from a directory.
    ///
    /// If the model is already cached, returns immediately (0ms load time).
    /// The directory should contain encoder, decoder, joiner, and tokens files.
    pub fn load_model(&self, model_dir: String) -> Result<i64, LexiconFlutterError> {
        #[cfg(feature = "stt-sherpa")]
        {
            use lexicon_core::stt::Transcriber as _;

            let start = Instant::now();

            // Check cache first
            {
                let cache = MODEL_CACHE.read().map_err(|_| {
                    LexiconFlutterError::TranscriptionFailed("Cache lock poisoned".to_string())
                })?;

                if let Some(ref c) = *cache {
                    if let Some(ref cached) = c.sherpa_model {
                        if cached.path == model_dir {
                            // Cache hit - use cached model
                            let mut inner = self.inner.lock().map_err(|_| {
                                LexiconFlutterError::TranscriptionFailed(
                                    "Lock poisoned".to_string(),
                                )
                            })?;
                            *inner = Some(Arc::clone(&cached.model));

                            let mut path = self.model_path.lock().map_err(|_| {
                                LexiconFlutterError::TranscriptionFailed(
                                    "Lock poisoned".to_string(),
                                )
                            })?;
                            *path = Some(model_dir.clone());

                            // Update last used time
                            if let Ok(mut last_used) = cached.last_used.write() {
                                *last_used = Instant::now();
                            }

                            tracing::info!("Sherpa model loaded from cache: {}", model_dir);
                            return Ok(0);
                        }
                    }
                }
            }

            // Cache miss - load from disk
            let path = PathBuf::from(&model_dir);
            if !path.exists() {
                return Err(LexiconFlutterError::ModelNotFound(model_dir));
            }

            let mut transcriber = lexicon_core::stt::SherpaTranscriber::new()
                .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

            transcriber
                .load_model(&path)
                .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

            let model_arc = Arc::new(Mutex::new(transcriber));

            // Store in instance
            {
                let mut inner = self.inner.lock().map_err(|_| {
                    LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
                })?;
                *inner = Some(Arc::clone(&model_arc));
            }

            {
                let mut model_path = self.model_path.lock().map_err(|_| {
                    LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
                })?;
                *model_path = Some(model_dir.clone());
            }

            // Also store in cache for future use
            {
                let mut cache = get_cache();
                let cache = cache.as_mut().unwrap();

                // Estimate memory size (~150MB for typical Sherpa model)
                let size_bytes = 150 * 1024 * 1024;

                cache.sherpa_model = Some(CachedSherpaModel {
                    model: model_arc,
                    path: model_dir.clone(),
                    size_bytes,
                    last_used: RwLock::new(Instant::now()),
                });

                cache.evict_if_needed();
            }

            let elapsed_ms = start.elapsed().as_millis() as i64;
            tracing::info!(
                elapsed_ms = elapsed_ms,
                "Sherpa model loaded and cached: {}",
                model_dir
            );
            Ok(elapsed_ms)
        }

        #[cfg(not(feature = "stt-sherpa"))]
        {
            let _ = model_dir;
            Err(LexiconFlutterError::TranscriptionFailed(
                "stt-sherpa feature not enabled".to_string(),
            ))
        }
    }

    /// Unload the current model from this instance.
    ///
    /// Note: The model may still be cached for future use by other instances.
    #[frb(sync)]
    pub fn unload_model(&self) {
        #[cfg(feature = "stt-sherpa")]
        {
            if let Ok(mut inner) = self.inner.lock() {
                // Just drop the Arc reference; model stays in cache
                *inner = None;
                tracing::info!("Sherpa model unloaded from instance");
            }
            if let Ok(mut path) = self.model_path.lock() {
                *path = None;
            }
        }
    }

    /// Check if a model is loaded.
    #[frb(sync)]
    pub fn is_model_loaded(&self) -> bool {
        #[cfg(feature = "stt-sherpa")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "stt-sherpa"))]
        false
    }

    /// Warm up the model for optimal first-inference latency.
    pub fn warm_up(&self) -> Result<i64, LexiconFlutterError> {
        #[cfg(feature = "stt-sherpa")]
        {
            let start = Instant::now();

            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;

            let model_arc = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::TranscriptionFailed("No model loaded".to_string())
            })?;

            let transcriber = model_arc.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Model lock poisoned".to_string())
            })?;

            transcriber
                .warm_up()
                .map_err(|e| LexiconFlutterError::TranscriptionFailed(e.to_string()))?;

            let elapsed_ms = start.elapsed().as_millis() as i64;
            tracing::info!(elapsed_ms = elapsed_ms, "Sherpa model warm-up complete");

            Ok(elapsed_ms)
        }

        #[cfg(not(feature = "stt-sherpa"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "stt-sherpa feature not enabled".to_string(),
        ))
    }

    /// Transcribe audio data (batch mode).
    ///
    /// Audio should be raw PCM f32 samples at 16kHz mono, packed as bytes.
    pub fn transcribe(
        &self,
        audio_data: Vec<u8>,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult, LexiconFlutterError> {
        #[cfg(feature = "stt-sherpa")]
        {
            use lexicon_core::stt::{PostProcessor, Transcriber as _};

            // Convert bytes to f32 samples
            let samples: Vec<f32> = audio_data
                .chunks(4)
                .filter_map(|chunk| {
                    if chunk.len() == 4 {
                        Some(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    } else {
                        None
                    }
                })
                .collect();

            if samples.is_empty() {
                return Err(LexiconFlutterError::InvalidInput(
                    "Empty audio data".to_string(),
                ));
            }

            // Build config
            let mut tx_config = lexicon_core::stt::TranscriptionConfig::new();
            if let Some(lang) = config.language {
                tx_config = tx_config.with_language(lang);
            }
            if config.num_threads > 0 {
                tx_config = tx_config.with_threads(config.num_threads);
            }

            // Get the model Arc
            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;

            let model_arc = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::TranscriptionFailed("No model loaded".to_string())
            })?;

            // Lock the actual transcriber
            let transcriber = model_arc.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Model lock poisoned".to_string())
            })?;

            let result = transcriber
                .transcribe(&samples, &tx_config)
                .map_err(|e| LexiconFlutterError::TranscriptionFailed(e.to_string()))?;

            // Apply post-processing to improve transcription quality
            let post_processor = PostProcessor::new(Default::default());

            // Convert to FFI result
            let segments: Vec<TranscriptionSegment> = result
                .segments
                .iter()
                .map(|s| TranscriptionSegment {
                    start_ms: s.start_ms,
                    end_ms: s.end_ms,
                    text: post_processor.process(&s.text),
                    confidence: s.confidence,
                    language: s.language.clone(),
                    speaker_id: s.speaker_id,
                })
                .collect();

            // Update last_used time in cache
            if let Ok(path) = self.model_path.lock() {
                if let Some(ref model_path) = *path {
                    if let Ok(cache) = MODEL_CACHE.read() {
                        if let Some(ref c) = *cache {
                            if let Some(ref cached) = c.sherpa_model {
                                if &cached.path == model_path {
                                    if let Ok(mut last_used) = cached.last_used.write() {
                                        *last_used = Instant::now();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(TranscriptionResult {
                segments,
                detected_language: result.detected_language,
                audio_duration_ms: result.audio_duration_ms,
                processing_time_ms: result.processing_time_ms,
            })
        }

        #[cfg(not(feature = "stt-sherpa"))]
        {
            let _ = (audio_data, config);
            Err(LexiconFlutterError::TranscriptionFailed(
                "stt-sherpa feature not enabled".to_string(),
            ))
        }
    }
}

// ============================================================================
// Synthesizer
// ============================================================================

/// Synthesizer state held in Dart via opaque handle.
#[frb(opaque)]
pub struct Synthesizer {
    #[cfg(feature = "tts")]
    inner: Mutex<Option<lexicon_core::tts::TtsManager>>,
    #[cfg(not(feature = "tts"))]
    _marker: std::marker::PhantomData<()>,
}

impl Synthesizer {
    /// Create a new synthesizer.
    pub fn new() -> Result<Synthesizer, LexiconFlutterError> {
        #[cfg(feature = "tts")]
        {
            let manager = lexicon_core::tts::TtsManager::new()
                .map_err(|e| LexiconFlutterError::SynthesisFailed(e.to_string()))?;

            Ok(Self {
                inner: Mutex::new(Some(manager)),
            })
        }

        #[cfg(not(feature = "tts"))]
        {
            Ok(Self {
                _marker: std::marker::PhantomData,
            })
        }
    }

    /// Check if the synthesizer is ready.
    #[frb(sync)]
    pub fn is_ready(&self) -> bool {
        #[cfg(feature = "tts")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "tts"))]
        false
    }

    /// List available voices.
    #[frb(sync)]
    pub fn list_voices(&self) -> Vec<String> {
        #[cfg(feature = "tts")]
        {
            self.inner
                .lock()
                .ok()
                .and_then(|inner| {
                    inner
                        .as_ref()
                        .map(|manager| manager.list_voices().iter().map(|v| v.id.clone()).collect())
                })
                .unwrap_or_default()
        }

        #[cfg(not(feature = "tts"))]
        Vec::new()
    }

    /// Synthesize text to audio (batch mode).
    pub fn synthesize(
        &self,
        text: String,
        config: TtsConfig,
    ) -> Result<SynthesisResult, LexiconFlutterError> {
        #[cfg(feature = "tts")]
        {
            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::SynthesisFailed("Lock poisoned".to_string())
            })?;

            let manager = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::SynthesisFailed("TTS not initialized".to_string())
            })?;

            // Build config
            let mut tts_config = lexicon_core::tts::TtsConfig::new();
            if let Some(lang) = config.language {
                tts_config = tts_config.with_language(lang);
            }
            tts_config = tts_config
                .with_rate(config.rate)
                .with_pitch(config.pitch)
                .with_volume(config.volume);

            if config.use_kokoro {
                tts_config = tts_config.with_engine(lexicon_core::tts::TtsEngine::Kokoro);
            }

            // Synthesize
            let result = manager
                .synthesize(&text, &tts_config)
                .map_err(|e| LexiconFlutterError::SynthesisFailed(e.to_string()))?;

            // Convert audio to bytes
            let audio_bytes: Vec<u8> = result
                .audio
                .samples
                .iter()
                .flat_map(|&s| s.to_le_bytes())
                .collect();

            Ok(SynthesisResult {
                audio_data: audio_bytes,
                sample_rate: result.audio.sample_rate,
                duration_ms: result.audio.duration_ms() as i64,
                engine: format!("{:?}", result.engine_used),
            })
        }

        #[cfg(not(feature = "tts"))]
        Err(LexiconFlutterError::SynthesisFailed(
            "TTS feature not enabled".to_string(),
        ))
    }
}

// ============================================================================
// Streaming Transcription
// ============================================================================

/// Audio chunk for streaming transcription.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// Raw PCM f32 samples at 16kHz mono, packed as bytes.
    pub data: Vec<u8>,
    /// Timestamp in milliseconds from start of stream.
    pub timestamp_ms: i64,
    /// Whether this is the final chunk (signals end of stream).
    pub is_final: bool,
}

/// Streaming transcription event sent to Dart.
#[derive(Debug, Clone)]
pub enum TranscriptionEvent {
    /// A segment was transcribed.
    Segment(TranscriptionSegment),
    /// Partial/interim result (may change).
    Partial { text: String, timestamp_ms: i64 },
    /// Stream completed with final result.
    Complete(TranscriptionResult),
    /// An error occurred.
    Error(String),
}

/// Streaming transcription session.
#[frb(opaque)]
pub struct StreamingTranscriber {
    #[cfg(feature = "stt")]
    inner: Arc<Mutex<Option<lexicon_core::stt::WhisperTranscriber>>>,
    #[cfg(feature = "stt")]
    running: Arc<AtomicBool>,
    #[cfg(not(feature = "stt"))]
    _marker: std::marker::PhantomData<()>,
}

impl StreamingTranscriber {
    /// Create a new streaming transcriber.
    pub fn new() -> Result<StreamingTranscriber, LexiconFlutterError> {
        Ok(Self {
            #[cfg(feature = "stt")]
            inner: Arc::new(Mutex::new(None)),
            #[cfg(feature = "stt")]
            running: Arc::new(AtomicBool::new(false)),
            #[cfg(not(feature = "stt"))]
            _marker: std::marker::PhantomData,
        })
    }

    /// Load a model for streaming transcription.
    pub fn load_model(&self, model_path: String) -> Result<(), LexiconFlutterError> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::Transcriber as _;

            let path = PathBuf::from(&model_path);
            if !path.exists() {
                return Err(LexiconFlutterError::ModelNotFound(model_path));
            }

            let mut transcriber = lexicon_core::stt::WhisperTranscriber::new()
                .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

            transcriber
                .load_model(&path)
                .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

            let mut inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;
            *inner = Some(transcriber);

            Ok(())
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "STT feature not enabled".to_string(),
        ))
    }

    /// Check if model is loaded.
    #[frb(sync)]
    pub fn is_ready(&self) -> bool {
        #[cfg(feature = "stt")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "stt"))]
        false
    }

    /// Check if streaming is active.
    #[frb(sync)]
    pub fn is_streaming(&self) -> bool {
        #[cfg(feature = "stt")]
        {
            self.running.load(Ordering::SeqCst)
        }

        #[cfg(not(feature = "stt"))]
        false
    }

    /// Stop the streaming session.
    #[frb(sync)]
    pub fn stop(&self) {
        #[cfg(feature = "stt")]
        {
            self.running.store(false, Ordering::SeqCst);
        }
    }

    /// Warms up the model by running a short inference on silence.
    ///
    /// **Latency Optimization (WP-LATENCY-STT)**: This primes the model's
    /// internal caches and eliminates the "first inference" latency penalty.
    ///
    /// Returns the warm-up time in milliseconds for diagnostics.
    pub fn warm_up(&self) -> Result<i64, LexiconFlutterError> {
        #[cfg(feature = "stt")]
        {
            let start = Instant::now();

            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;

            let transcriber = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::TranscriptionFailed("No model loaded".to_string())
            })?;

            transcriber
                .warm_up()
                .map_err(|e| LexiconFlutterError::TranscriptionFailed(e.to_string()))?;

            let elapsed_ms = start.elapsed().as_millis() as i64;
            tracing::info!(elapsed_ms = elapsed_ms, "Streaming model warm-up complete");

            Ok(elapsed_ms)
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "STT feature not enabled".to_string(),
        ))
    }

    /// Process a single audio chunk and return segments.
    ///
    /// This is the core streaming method. Call it repeatedly with audio chunks
    /// to get real-time transcription results.
    ///
    /// **Latency Optimization (WP-LATENCY-STT)**: Uses VAD-based early exit
    /// to skip silent/noise chunks before expensive Whisper inference.
    /// Silent chunks return empty segments instantly.
    pub fn process_chunk(
        &self,
        chunk: AudioChunk,
        config: TranscriptionConfig,
    ) -> Result<Vec<TranscriptionSegment>, LexiconFlutterError> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::audio::{should_process, ProcessingDecision};
            use lexicon_core::stt::{PostProcessor, Transcriber as _};

            // Convert bytes to f32 samples
            let samples: Vec<f32> = chunk
                .data
                .chunks(4)
                .filter_map(|c| {
                    if c.len() == 4 {
                        Some(f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    } else {
                        None
                    }
                })
                .collect();

            if samples.is_empty() {
                return Ok(Vec::new());
            }

            // VAD early exit: skip silent/noise chunks to save compute
            if should_process(&samples) == ProcessingDecision::Skip {
                tracing::debug!(
                    timestamp_ms = chunk.timestamp_ms,
                    samples = samples.len(),
                    "Skipping chunk (silence/noise detected)"
                );
                return Ok(Vec::new());
            }

            // Build config
            let mut tx_config = lexicon_core::stt::TranscriptionConfig::new();
            if let Some(lang) = config.language {
                tx_config = tx_config.with_language(lang);
            }
            if config.translate {
                tx_config = tx_config.with_translation();
            }

            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;

            let transcriber = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::TranscriptionFailed("No model loaded".to_string())
            })?;

            let result = transcriber
                .transcribe(&samples, &tx_config)
                .map_err(|e| LexiconFlutterError::TranscriptionFailed(e.to_string()))?;

            // Apply post-processing to improve transcription quality
            let post_processor = PostProcessor::new(Default::default());

            // Convert segments with timestamp offset
            let segments: Vec<TranscriptionSegment> = result
                .segments
                .iter()
                .map(|s| TranscriptionSegment {
                    start_ms: s.start_ms + chunk.timestamp_ms,
                    end_ms: s.end_ms + chunk.timestamp_ms,
                    text: post_processor.process(&s.text),
                    confidence: s.confidence,
                    language: s.language.clone(),
                    speaker_id: s.speaker_id,
                })
                .collect();

            Ok(segments)
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "STT feature not enabled".to_string(),
        ))
    }
}

// ============================================================================
// Streaming TTS
// ============================================================================

/// TTS audio chunk sent to Dart during streaming synthesis.
#[derive(Debug, Clone)]
pub struct TtsAudioChunk {
    /// Raw PCM f32 audio data as bytes.
    pub data: Vec<u8>,
    /// Sample rate of this chunk.
    pub sample_rate: u32,
    /// Sequence number (0-indexed).
    pub sequence: u32,
    /// Whether this is the final chunk.
    pub is_final: bool,
}

/// Streaming TTS synthesizer.
#[frb(opaque)]
pub struct StreamingSynthesizer {
    #[cfg(feature = "tts")]
    inner: Arc<Mutex<Option<lexicon_core::tts::TtsManager>>>,
    #[cfg(feature = "tts")]
    running: Arc<AtomicBool>,
    #[cfg(not(feature = "tts"))]
    _marker: std::marker::PhantomData<()>,
}

impl StreamingSynthesizer {
    /// Create a new streaming synthesizer.
    pub fn new() -> Result<StreamingSynthesizer, LexiconFlutterError> {
        #[cfg(feature = "tts")]
        {
            let manager = lexicon_core::tts::TtsManager::new()
                .map_err(|e| LexiconFlutterError::SynthesisFailed(e.to_string()))?;

            Ok(Self {
                inner: Arc::new(Mutex::new(Some(manager))),
                running: Arc::new(AtomicBool::new(false)),
            })
        }

        #[cfg(not(feature = "tts"))]
        {
            Ok(Self {
                _marker: std::marker::PhantomData,
            })
        }
    }

    /// Check if ready.
    #[frb(sync)]
    pub fn is_ready(&self) -> bool {
        #[cfg(feature = "tts")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "tts"))]
        false
    }

    /// Check if synthesis is active.
    #[frb(sync)]
    pub fn is_synthesizing(&self) -> bool {
        #[cfg(feature = "tts")]
        {
            self.running.load(Ordering::SeqCst)
        }

        #[cfg(not(feature = "tts"))]
        false
    }

    /// Stop synthesis.
    #[frb(sync)]
    pub fn stop(&self) {
        #[cfg(feature = "tts")]
        {
            self.running.store(false, Ordering::SeqCst);
        }
    }

    /// Synthesize text and return audio in chunks.
    ///
    /// This method synthesizes the full text but returns it in chunks
    /// for progressive playback. Each chunk is ~100ms of audio.
    pub fn synthesize_chunked(
        &self,
        text: String,
        config: TtsConfig,
        chunk_ms: u32,
    ) -> Result<Vec<TtsAudioChunk>, LexiconFlutterError> {
        #[cfg(feature = "tts")]
        {
            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::SynthesisFailed("Lock poisoned".to_string())
            })?;

            let manager = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::SynthesisFailed("TTS not initialized".to_string())
            })?;

            // Build config
            let mut tts_config = lexicon_core::tts::TtsConfig::new();
            if let Some(lang) = config.language {
                tts_config = tts_config.with_language(lang);
            }
            tts_config = tts_config
                .with_rate(config.rate)
                .with_pitch(config.pitch)
                .with_volume(config.volume);

            if config.use_kokoro {
                tts_config = tts_config.with_engine(lexicon_core::tts::TtsEngine::Kokoro);
            }

            // Synthesize full audio
            let result = manager
                .synthesize(&text, &tts_config)
                .map_err(|e| LexiconFlutterError::SynthesisFailed(e.to_string()))?;

            // Calculate chunk size in samples
            let samples_per_chunk = (result.audio.sample_rate * chunk_ms / 1000) as usize;
            let total_samples = result.audio.samples.len();

            // Split into chunks
            let mut chunks = Vec::new();
            let mut offset = 0;
            let mut sequence = 0u32;

            while offset < total_samples {
                let end = (offset + samples_per_chunk).min(total_samples);
                let chunk_samples = &result.audio.samples[offset..end];
                let is_final = end >= total_samples;

                // Convert to bytes
                let data: Vec<u8> = chunk_samples
                    .iter()
                    .flat_map(|&s| s.to_le_bytes())
                    .collect();

                chunks.push(TtsAudioChunk {
                    data,
                    sample_rate: result.audio.sample_rate,
                    sequence,
                    is_final,
                });

                offset = end;
                sequence += 1;
            }

            Ok(chunks)
        }

        #[cfg(not(feature = "tts"))]
        Err(LexiconFlutterError::SynthesisFailed(
            "TTS feature not enabled".to_string(),
        ))
    }
}

// ============================================================================
// Model Download with Progress
// ============================================================================

/// Download progress event.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// Download progress update.
    Progress(DownloadProgress),
    /// Download completed successfully.
    Complete { model_id: String, path: String },
    /// Download failed.
    Error { model_id: String, error: String },
    /// Download was cancelled.
    Cancelled { model_id: String },
}

/// Model downloader with progress reporting.
#[frb(opaque)]
pub struct ModelDownloader {
    model_dir: PathBuf,
    cancelled: Arc<AtomicBool>,
}

impl ModelDownloader {
    /// Create a new model downloader.
    pub fn new(model_dir: String) -> Result<ModelDownloader, LexiconFlutterError> {
        let path = PathBuf::from(&model_dir);
        std::fs::create_dir_all(&path)?;

        Ok(Self {
            model_dir: path,
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get the model directory.
    #[frb(sync)]
    pub fn get_model_dir(&self) -> String {
        self.model_dir.display().to_string()
    }

    /// Cancel any active download.
    #[frb(sync)]
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if download is cancelled.
    #[frb(sync)]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Reset cancellation flag.
    #[frb(sync)]
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    /// Download a model with progress tracking.
    ///
    /// Returns a list of progress events. In a real streaming scenario,
    /// you would use StreamSink, but for simplicity this returns all events.
    ///
    /// For real-time progress, call this in an isolate and poll `is_cancelled()`.
    pub async fn download_model(
        &self,
        model_id: String,
    ) -> Result<Vec<DownloadEvent>, LexiconFlutterError> {
        self.cancelled.store(false, Ordering::SeqCst);

        // Use Arc<Mutex<Vec>> to share events between closure and main function
        let events = Arc::new(Mutex::new(Vec::new()));
        let model_dir = self.model_dir.clone();
        let cancelled = self.cancelled.clone();

        // Create registry
        let mut registry = lexicon_core::ModelRegistry::new(&model_dir)
            .map_err(|e| LexiconFlutterError::DownloadFailed(e.to_string()))?;

        // Fetch manifest
        registry
            .fetch_manifest()
            .await
            .map_err(|e| LexiconFlutterError::DownloadFailed(format!("Manifest fetch failed: {e}")))?;

        // Check if cancelled
        if cancelled.load(Ordering::SeqCst) {
            let mut ev = events.lock().unwrap();
            ev.push(DownloadEvent::Cancelled {
                model_id: model_id.clone(),
            });
            return Ok(std::mem::take(&mut *ev));
        }

        // Track progress
        let mid = model_id.clone();
        let start_time = Instant::now();
        let events_clone = events.clone();
        let cancelled_clone = cancelled.clone();

        // Download with progress callback
        let result = registry
            .download_model_cancellable(&model_id, move |progress| {
                // Calculate speed
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    (progress.downloaded_bytes as f64 / elapsed) as u64
                } else {
                    0
                };

                // Calculate ETA
                let remaining = progress.total_bytes.saturating_sub(progress.downloaded_bytes);
                let eta = if speed > 0 {
                    Some(remaining / speed)
                } else {
                    None
                };

                if let Ok(mut ev) = events_clone.lock() {
                    ev.push(DownloadEvent::Progress(DownloadProgress {
                        model_id: mid.clone(),
                        downloaded_bytes: progress.downloaded_bytes,
                        total_bytes: progress.total_bytes,
                        speed_bps: speed,
                        eta_seconds: eta,
                    }));
                }

                // Return true to continue, false to cancel
                !cancelled_clone.load(Ordering::SeqCst)
            })
            .await;

        // Get the events back
        let mut ev = events.lock().unwrap();

        match result {
            Ok(path) => {
                ev.push(DownloadEvent::Complete {
                    model_id,
                    path: path.display().to_string(),
                });
            }
            Err(e) => {
                let error_str = e.to_string();
                if cancelled.load(Ordering::SeqCst) || error_str.contains("cancelled") {
                    ev.push(DownloadEvent::Cancelled { model_id });
                } else {
                    ev.push(DownloadEvent::Error {
                        model_id,
                        error: error_str,
                    });
                }
            }
        }

        Ok(std::mem::take(&mut *ev))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_platform_info() {
        let info = get_platform_info();
        assert!(!info.platform.is_empty());
        assert!(info.available_memory_mb > 0);
    }

    #[test]
    fn test_transcriber_creation() {
        let transcriber = Transcriber::new();
        assert!(transcriber.is_ok());
        let t = transcriber.unwrap();
        assert!(!t.is_model_loaded());
    }

    #[test]
    fn test_config_defaults() {
        let config = TranscriptionConfig::default();
        assert!(!config.translate);
        assert_eq!(config.num_threads, 4);

        let tts = TtsConfig::default();
        assert!(tts.use_kokoro);
        assert!((tts.rate - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_streaming_transcriber_creation() {
        let transcriber = StreamingTranscriber::new();
        assert!(transcriber.is_ok());
        let t = transcriber.unwrap();
        assert!(!t.is_ready());
        assert!(!t.is_streaming());
    }

    #[test]
    fn test_streaming_synthesizer_creation() {
        let synth = StreamingSynthesizer::new();
        // Note: This may fail if TTS manager can't initialize on this system
        if let Ok(s) = synth {
            assert!(s.is_ready());
            assert!(!s.is_synthesizing());
        }
    }

    #[test]
    fn test_model_downloader_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path().display().to_string());
        assert!(downloader.is_ok());
        let d = downloader.unwrap();
        assert!(!d.is_cancelled());
        assert_eq!(d.get_model_dir(), temp_dir.path().display().to_string());
    }

    #[test]
    fn test_audio_chunk_creation() {
        let chunk = AudioChunk {
            data: vec![0u8; 6400], // 100ms at 16kHz mono f32
            timestamp_ms: 0,
            is_final: false,
        };
        assert_eq!(chunk.data.len(), 6400);
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_tts_chunk_creation() {
        let chunk = TtsAudioChunk {
            data: vec![0u8; 3200],
            sample_rate: 22050,
            sequence: 0,
            is_final: true,
        };
        assert_eq!(chunk.sample_rate, 22050);
        assert!(chunk.is_final);
    }

    #[test]
    fn test_download_event_variants() {
        let progress = DownloadEvent::Progress(DownloadProgress {
            model_id: "test".to_string(),
            downloaded_bytes: 100,
            total_bytes: 1000,
            speed_bps: 100,
            eta_seconds: Some(9),
        });
        assert!(matches!(progress, DownloadEvent::Progress(_)));

        let complete = DownloadEvent::Complete {
            model_id: "test".to_string(),
            path: "/tmp/model.bin".to_string(),
        };
        assert!(matches!(complete, DownloadEvent::Complete { .. }));

        let error = DownloadEvent::Error {
            model_id: "test".to_string(),
            error: "Network error".to_string(),
        };
        assert!(matches!(error, DownloadEvent::Error { .. }));

        let cancelled = DownloadEvent::Cancelled {
            model_id: "test".to_string(),
        };
        assert!(matches!(cancelled, DownloadEvent::Cancelled { .. }));
    }

    #[test]
    fn test_transcription_event_variants() {
        let segment = TranscriptionEvent::Segment(TranscriptionSegment {
            start_ms: 0,
            end_ms: 1000,
            text: "Hello".to_string(),
            confidence: Some(0.95),
            language: Some("en".to_string()),
            speaker_id: None,
        });
        assert!(matches!(segment, TranscriptionEvent::Segment(_)));

        let partial = TranscriptionEvent::Partial {
            text: "Hel".to_string(),
            timestamp_ms: 500,
        };
        assert!(matches!(partial, TranscriptionEvent::Partial { .. }));

        let error = TranscriptionEvent::Error("Test error".to_string());
        assert!(matches!(error, TranscriptionEvent::Error(_)));
    }

    // --- Tests for WP-LATENCY-STT optimizations ---

    #[test]
    fn test_transcriber_warm_up_requires_model() {
        let transcriber = Transcriber::new().unwrap();
        // warm_up should fail without a loaded model
        let result = transcriber.warm_up();
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_transcriber_warm_up_requires_model() {
        let transcriber = StreamingTranscriber::new().unwrap();
        // warm_up should fail without a loaded model
        let result = transcriber.warm_up();
        assert!(result.is_err());
    }

    #[test]
    fn test_process_chunk_skips_silence() {
        let transcriber = StreamingTranscriber::new().unwrap();

        // Create a silent chunk (all zeros)
        let silence_samples = vec![0.0f32; 1600]; // 100ms at 16kHz
        let silence_bytes: Vec<u8> = silence_samples
            .iter()
            .flat_map(|&s| s.to_le_bytes())
            .collect();

        let chunk = AudioChunk {
            data: silence_bytes,
            timestamp_ms: 0,
            is_final: false,
        };

        // VAD early exit should return empty segments for silence
        // even without a model loaded (because it skips before model check)
        let result = transcriber.process_chunk(chunk, TranscriptionConfig::default());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_process_chunk_requires_model_for_speech() {
        let transcriber = StreamingTranscriber::new().unwrap();

        // Create a chunk with high energy (simulated speech)
        let speech_samples = vec![0.1f32; 1600]; // 100ms at 16kHz with energy
        let speech_bytes: Vec<u8> = speech_samples
            .iter()
            .flat_map(|&s| s.to_le_bytes())
            .collect();

        let chunk = AudioChunk {
            data: speech_bytes,
            timestamp_ms: 0,
            is_final: false,
        };

        // Non-silent chunk should fail without a model loaded
        let result = transcriber.process_chunk(chunk, TranscriptionConfig::default());
        assert!(result.is_err());
    }

    // --- Tests for model cache ---

    #[test]
    fn test_cache_stats_default() {
        // Clear any existing cache first
        clear_model_cache();

        let stats = get_cache_stats();
        assert_eq!(stats.total_bytes, 0);
        assert!(!stats.sherpa_cached);
        assert!(!stats.tts_cached);
    }

    #[test]
    fn test_set_cache_max_memory() {
        let custom_max = 200 * 1024 * 1024; // 200MB
        set_cache_max_memory(custom_max);

        let stats = get_cache_stats();
        assert_eq!(stats.max_bytes, custom_max);
    }

    #[test]
    fn test_is_model_cached_when_empty() {
        clear_model_cache();
        assert!(!is_model_cached("sherpa".to_string()));
        assert!(!is_model_cached("tts".to_string()));
        assert!(!is_model_cached("unknown".to_string()));
    }

    #[test]
    fn test_clear_model_cache() {
        // Should not panic even if cache is empty
        clear_model_cache();
        let stats = get_cache_stats();
        assert_eq!(stats.total_bytes, 0);
    }

    #[test]
    fn test_on_low_memory() {
        // Should not panic and should clear cache
        on_low_memory();
        let stats = get_cache_stats();
        assert_eq!(stats.total_bytes, 0);
    }
}
