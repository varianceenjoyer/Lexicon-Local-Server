//! Tests for the models module.

use super::*;
use crate::platform::{ModelTier, ThermalState};

fn sample_manifest_json() -> &'static str {
    r#"{
        "version": "1.0.0",
        "generated_at": "2025-01-01T00:00:00Z",
        "base_url": "https://models.lexicon.dev",
        "models": {
            "whisper-tiny-q5_1": {
                "id": "whisper-tiny-q5_1",
                "name": "Whisper Tiny (Q5_1)",
                "model_type": "stt",
                "size_bytes": 42000000,
                "hash_sha256": "abc123def456789012345678901234567890123456789012345678901234",
                "download_path": "stt/whisper-tiny-q5_1.bin",
                "quantization": "q5_1",
                "languages": ["en", "multilingual"],
                "memory_mb": 150
            },
            "whisper-base-q5_1": {
                "id": "whisper-base-q5_1",
                "name": "Whisper Base (Q5_1)",
                "model_type": "stt",
                "size_bytes": 78000000,
                "hash_sha256": "def456abc789012345678901234567890123456789012345678901234567",
                "download_path": "stt/whisper-base-q5_1.bin",
                "quantization": "q5_1",
                "languages": ["en", "multilingual"],
                "memory_mb": 300
            },
            "kokoro-v1": {
                "id": "kokoro-v1",
                "name": "Kokoro TTS",
                "model_type": "tts",
                "size_bytes": 350000000,
                "hash_sha256": "789abc123def456789012345678901234567890123456789012345678901",
                "download_path": "tts/kokoro-v1.onnx",
                "memory_mb": 512
            }
        },
        "tier_recommendations": {
            "mobile-low": ["whisper-tiny-q5_1"],
            "mobile-mid": ["whisper-tiny-q5_1", "whisper-base-q5_1"],
            "desktop": ["whisper-base-q5_1", "kokoro-v1"],
            "server": ["whisper-tiny-q5_1", "whisper-base-q5_1", "kokoro-v1"]
        }
    }"#
}

#[test]
fn test_model_id() {
    let id = ModelId::new("whisper-base");
    assert_eq!(id.0, "whisper-base");
    assert_eq!(id.to_string(), "whisper-base");
    assert_eq!(id.as_str(), "whisper-base");

    let id2: ModelId = "whisper-tiny".into();
    assert_eq!(id2.0, "whisper-tiny");
}

#[test]
fn test_model_type_display() {
    assert_eq!(ModelType::Stt.to_string(), "stt");
    assert_eq!(ModelType::Tts.to_string(), "tts");
}

#[test]
fn test_download_progress() {
    let progress = DownloadProgress::new(ModelId::new("test"), 50, 100);
    assert_eq!(progress.percentage(), 50.0);

    let progress_zero = DownloadProgress::new(ModelId::new("test"), 0, 0);
    assert_eq!(progress_zero.percentage(), 0.0);

    let mut progress_speed = DownloadProgress::new(ModelId::new("test"), 1000, 10000);
    progress_speed.update_speed(1.0);
    assert_eq!(progress_speed.speed_bps, Some(1000));
    assert_eq!(progress_speed.eta_seconds, Some(9)); // 9000 bytes / 1000 bps = 9 seconds
}

#[test]
fn test_manifest_parsing() {
    let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.models.len(), 3);

    let tiny = manifest.get_model("whisper-tiny-q5_1").unwrap();
    assert_eq!(tiny.name, "Whisper Tiny (Q5_1)");
    assert_eq!(tiny.model_type, ModelType::Stt);
}

#[test]
fn test_manifest_models_by_type() {
    let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

    let stt_models = manifest.stt_models();
    assert_eq!(stt_models.len(), 2);

    let tts_models = manifest.tts_models();
    assert_eq!(tts_models.len(), 1);
    assert_eq!(tts_models[0].id, "kokoro-v1");
}

#[test]
fn test_manifest_tier_recommendations() {
    let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

    let mobile_low = manifest.recommended_for_tier(ModelTier::MobileLow);
    assert_eq!(mobile_low.len(), 1);
    assert_eq!(mobile_low[0].id, "whisper-tiny-q5_1");

    let desktop = manifest.recommended_for_tier(ModelTier::Desktop);
    assert_eq!(desktop.len(), 2);

    let server = manifest.recommended_for_tier(ModelTier::Server);
    assert_eq!(server.len(), 3);
}

#[test]
fn test_manifest_smallest_model() {
    let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

    let smallest = manifest.smallest_stt_model().unwrap();
    assert_eq!(smallest.id, "whisper-tiny-q5_1");
}

#[test]
fn test_model_entry_download_url() {
    let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();
    let model = manifest.get_model("whisper-tiny-q5_1").unwrap();

    let url = model.download_url(&manifest.base_url);
    assert_eq!(url, "https://models.lexicon.dev/stt/whisper-tiny-q5_1.bin");
}

#[test]
fn test_model_entry_fits_memory() {
    let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

    let tiny = manifest.get_model("whisper-tiny-q5_1").unwrap();
    assert!(tiny.fits_memory(200)); // 150 MB model fits in 200 MB
    assert!(!tiny.fits_memory(100)); // Doesn't fit in 100 MB

    let kokoro = manifest.get_model("kokoro-v1").unwrap();
    assert!(!kokoro.fits_memory(400)); // 512 MB model doesn't fit in 400 MB
    assert!(kokoro.fits_memory(600)); // Fits in 600 MB
}

#[test]
fn test_model_info() {
    let info = ModelInfo {
        id: ModelId::new("test"),
        name: "Test Model".to_string(),
        model_type: ModelType::Stt,
        size_bytes: 1000,
        hash_sha256: "abc123".to_string(),
        local_path: None,
        verified: false,
        quantization: Some("q5_1".to_string()),
        languages: vec!["en".to_string()],
        memory_mb: 100,
    };

    assert!(!info.is_downloaded());
    assert!(!info.is_ready());

    // With existing path (simulate by using temp dir)
    let temp_path = std::env::temp_dir().join("lexicon-test-model-file");
    std::fs::write(&temp_path, "test").unwrap();

    let info_with_path = ModelInfo {
        local_path: Some(temp_path.clone()),
        verified: true,
        ..info
    };

    assert!(info_with_path.is_downloaded());
    assert!(info_with_path.is_ready());

    // Cleanup
    let _ = std::fs::remove_file(&temp_path);
}

#[test]
fn test_download_state() {
    assert_eq!(DownloadState::NotDownloaded.to_string(), "not_downloaded");
    assert_eq!(DownloadState::Downloading.to_string(), "downloading");
    assert_eq!(DownloadState::Ready.to_string(), "ready");
    assert_eq!(DownloadState::Failed.to_string(), "failed");
}

#[test]
fn test_model_tier_requirements_default() {
    let req = ModelTierRequirements::default();
    assert_eq!(req.min_memory_mb, 512);
    assert!(!req.requires_gpu);
    assert!(req.platforms.is_empty());
}

#[test]
fn test_thermal_state_from_headroom() {
    assert_eq!(ThermalState::from_headroom(0.8), ThermalState::Nominal);
    assert_eq!(ThermalState::from_headroom(0.6), ThermalState::Fair);
    assert_eq!(ThermalState::from_headroom(0.4), ThermalState::Serious);
    assert_eq!(ThermalState::from_headroom(0.1), ThermalState::Critical);
}

#[test]
fn test_thermal_state_should_throttle() {
    assert!(!ThermalState::Nominal.should_throttle());
    assert!(!ThermalState::Fair.should_throttle());
    assert!(ThermalState::Serious.should_throttle());
    assert!(ThermalState::Critical.should_throttle());
}
