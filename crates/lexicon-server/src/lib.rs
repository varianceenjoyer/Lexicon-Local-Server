//! Lexicon HTTP/WebSocket Server
//!
//! Provides REST and WebSocket APIs for STT, TTS, and session management.
//! This crate depends on `lexicon-core` for the underlying functionality.

pub mod routes;

/// Server configuration and startup.
pub struct Server {
    // Configuration and state will be added in WP-SERVER
}

impl Server {
    /// Creates a new server instance.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}
