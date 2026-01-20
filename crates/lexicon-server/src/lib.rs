//! Lexicon HTTP/WebSocket Server
//!
//! Provides REST and WebSocket APIs for STT, TTS, and session management.
//!
//! # Endpoints
//!
//! ## WebSocket
//! - `ws://host/api/v1/stt/stream` - Real-time STT (audio in, segments out)
//! - `ws://host/api/v1/tts/stream` - Streaming TTS (text in, audio out)
//!
//! ## REST
//! - `GET  /api/v1/models` - List available models
//! - `POST /api/v1/models/download` - Download model
//! - `POST /api/v1/transcribe` - Batch transcription
//! - `GET  /api/v1/sessions` - List sessions
//! - `GET  /api/v1/sessions/{id}` - Get session details
//! - `GET  /api/v1/sessions/{id}/export` - Export transcription
//! - `DELETE /api/v1/sessions/{id}` - Delete session
//! - `GET  /health` - Health check
//!
//! # Example
//!
//! ```no_run
//! use lexicon_server::{Server, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = ServerConfig::default();
//!     let server = Server::new(config)?;
//!     server.run().await?;
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod rate_limit;
pub mod routes;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{middleware, Router};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

pub use auth::{AuthConfig, Authenticator};
pub use rate_limit::{RateLimitConfig, RateLimiter};

use lexicon_core::{LexiconConfig, ModelRegistry};

#[cfg(feature = "stt")]
use lexicon_core::stt::{Transcriber, WhisperTranscriber};

#[cfg(feature = "tts")]
use lexicon_core::tts::TtsManager;

#[cfg(feature = "storage")]
use lexicon_core::Database;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Host to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Enable CORS for all origins.
    pub cors_permissive: bool,
    /// Path to serve static files from (optional).
    pub static_dir: Option<PathBuf>,
    /// Lexicon core configuration.
    pub core_config: LexiconConfig,
    /// Rate limiting configuration.
    pub rate_limit: RateLimitConfig,
    /// Authentication configuration.
    pub auth: AuthConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8765,
            cors_permissive: true,
            static_dir: None,
            core_config: LexiconConfig::builder().build().unwrap_or_else(|_| {
                LexiconConfig::builder()
                    .data_dir(std::env::temp_dir().join("lexicon"))
                    .build()
                    .expect("Failed to create default config")
            }),
            rate_limit: RateLimitConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}

impl ServerConfig {
    /// Creates a new server config with the given host and port.
    #[must_use]
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            ..Default::default()
        }
    }

    /// Sets the core configuration.
    #[must_use]
    pub fn with_core_config(mut self, config: LexiconConfig) -> Self {
        self.core_config = config;
        self
    }

    /// Sets the static file directory.
    #[must_use]
    pub fn with_static_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.static_dir = Some(path.into());
        self
    }

    /// Disables permissive CORS.
    #[must_use]
    pub fn with_strict_cors(mut self) -> Self {
        self.cors_permissive = false;
        self
    }

    /// Sets the rate limiting configuration.
    #[must_use]
    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit = config;
        self
    }

    /// Disables rate limiting.
    #[must_use]
    pub fn without_rate_limit(mut self) -> Self {
        self.rate_limit = RateLimitConfig::disabled();
        self
    }

    /// Sets the authentication configuration.
    #[must_use]
    pub fn with_auth(mut self, config: AuthConfig) -> Self {
        self.auth = config;
        self
    }

    /// Enables authentication with a single API key.
    #[must_use]
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.auth = AuthConfig::with_api_key(key);
        self
    }

    /// Disables authentication.
    #[must_use]
    pub fn without_auth(mut self) -> Self {
        self.auth = AuthConfig::disabled();
        self
    }
}

/// Shared application state.
pub struct AppState {
    /// Core configuration.
    pub config: LexiconConfig,
    /// Model registry.
    pub registry: RwLock<ModelRegistry>,
    /// STT transcriber (lazily loaded).
    #[cfg(feature = "stt")]
    pub transcriber: RwLock<Option<WhisperTranscriber>>,
    /// TTS manager (lazily loaded).
    #[cfg(feature = "tts")]
    pub tts: RwLock<Option<TtsManager>>,
    /// Database connection (Mutex because rusqlite::Connection is Send but not Sync).
    #[cfg(feature = "storage")]
    pub database: Mutex<Option<Database>>,
}

impl AppState {
    /// Creates new application state.
    pub fn new(config: LexiconConfig) -> anyhow::Result<Self> {
        let registry = ModelRegistry::new(&config.model_dir)?;

        Ok(Self {
            config,
            registry: RwLock::new(registry),
            #[cfg(feature = "stt")]
            transcriber: RwLock::new(None),
            #[cfg(feature = "tts")]
            tts: RwLock::new(None),
            #[cfg(feature = "storage")]
            database: Mutex::new(None),
        })
    }

    /// Initializes the database connection.
    #[cfg(feature = "storage")]
    pub async fn init_database(&self) -> anyhow::Result<()> {
        let db_path = self.config.data_dir.join("lexicon.db");
        let db = Database::open(&db_path)?;
        *self.database.lock().await = Some(db);
        info!("Database initialized at {:?}", db_path);
        Ok(())
    }

    /// Loads an STT model.
    #[cfg(feature = "stt")]
    pub async fn load_stt_model(&self, model_name: &str) -> anyhow::Result<()> {
        let model_path = self.config.model_dir.join(format!("whisper-{model_name}.bin"));

        if !model_path.exists() {
            anyhow::bail!("Model not found: {}", model_path.display());
        }

        let mut transcriber = WhisperTranscriber::new()?;
        transcriber.load_model(&model_path)?;
        *self.transcriber.write().await = Some(transcriber);
        info!("STT model loaded: {}", model_name);
        Ok(())
    }

    /// Initializes TTS.
    #[cfg(feature = "tts")]
    pub async fn init_tts(&self) -> anyhow::Result<()> {
        let tts = TtsManager::new()?;
        *self.tts.write().await = Some(tts);
        info!("TTS manager initialized");
        Ok(())
    }
}

/// The Lexicon HTTP/WebSocket server.
pub struct Server {
    config: ServerConfig,
    state: Arc<AppState>,
    rate_limiter: Arc<RateLimiter>,
    authenticator: Arc<Authenticator>,
}

impl Server {
    /// Creates a new server instance.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn new(config: ServerConfig) -> anyhow::Result<Self> {
        let state = AppState::new(config.core_config.clone())?;
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
        let authenticator = Arc::new(Authenticator::new(config.auth.clone()));

        Ok(Self {
            config,
            state: Arc::new(state),
            rate_limiter,
            authenticator,
        })
    }

    /// Returns a reference to the application state.
    #[must_use]
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }

    /// Returns the Axum router for testing.
    #[must_use]
    pub fn router(&self) -> Router {
        self.build_router()
    }

    /// Builds the Axum router.
    fn build_router(&self) -> Router {
        let mut router = Router::new()
            .nest("/api/v1", routes::api_routes())
            .route("/health", axum::routing::get(routes::health_check))
            .with_state(Arc::clone(&self.state));

        // Add authentication middleware (only if enabled)
        if self.authenticator.is_enabled() {
            router = router
                .layer(axum::Extension(Arc::clone(&self.authenticator)))
                .layer(middleware::from_fn(auth::auth_middleware));
        }

        // Add rate limiting middleware (only if enabled)
        if self.config.rate_limit.enabled {
            router = router
                .layer(axum::Extension(Arc::clone(&self.rate_limiter)))
                .layer(middleware::from_fn(rate_limit::rate_limit_middleware_forwarded));
        }

        // Add CORS layer
        if self.config.cors_permissive {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);
            router = router.layer(cors);
        }

        // Add tracing layer
        router = router.layer(TraceLayer::new_for_http());

        // Add static file serving if configured
        if let Some(ref static_dir) = self.config.static_dir {
            use tower_http::services::ServeDir;
            router = router.fallback_service(ServeDir::new(static_dir));
        }

        router
    }

    /// Runs the server.
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to start or encounters a runtime error.
    pub async fn run(self) -> anyhow::Result<()> {
        // Initialize database
        #[cfg(feature = "storage")]
        self.state.init_database().await?;

        // Initialize TTS
        #[cfg(feature = "tts")]
        self.state.init_tts().await.ok(); // Non-fatal if TTS fails

        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

        let router = self.build_router();

        info!("Lexicon server starting on http://{}", addr);
        if self.authenticator.is_enabled() {
            info!(
                "Authentication enabled with {} API key(s)",
                self.config.auth.api_keys.len()
            );
        } else {
            info!("Authentication disabled");
        }
        if self.config.rate_limit.enabled {
            info!(
                "Rate limiting enabled: {} req/min (general), {} req/min (heavy), {} req/min (websocket)",
                self.config.rate_limit.default_rpm,
                self.config.rate_limit.heavy_rpm,
                self.config.rate_limit.websocket_rpm
            );
        } else {
            info!("Rate limiting disabled");
        }

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        info!("Server shutdown complete");
        Ok(())
    }
}

/// Signal handler for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("Shutdown signal received");
}
