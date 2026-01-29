//! Lexicon Flutter - flutter_rust_bridge bindings for Dart/Flutter
//!
//! This crate provides cross-platform bindings for Flutter/Dart applications
//! using the flutter_rust_bridge framework.
//!
//! # Usage (Dart)
//!
//! ```dart
//! import 'package:lexicon_flutter/lexicon_flutter.dart';
//!
//! // Initialize
//! await initLexicon(dataDir: documentsDir);
//!
//! // Create transcriber
//! final transcriber = await Transcriber.new_();
//! await transcriber.loadModel(modelPath: modelPath);
//!
//! // Transcribe
//! final config = TranscriptionConfig(language: 'en', translate: false);
//! final result = await transcriber.transcribe(audioData: data, config: config);
//! for (final segment in result.segments) {
//!   print('[${segment.startMs}-${segment.endMs}] ${segment.text}');
//! }
//! ```
//!
//! # Features
//!
//! - **stt** (default): Speech-to-text via Whisper
//! - **tts** (default): Text-to-speech via Kokoro/platform TTS

mod frb_generated;

pub mod api;
pub mod error;
pub mod types;

// Re-exports for flutter_rust_bridge
pub use api::*;
pub use error::LexiconFlutterError;
pub use types::*;
