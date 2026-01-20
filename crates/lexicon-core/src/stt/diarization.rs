//! Speaker Diarization
//!
//! Provides speaker identification and clustering for transcribed segments.
//! This is an MVP implementation using simple audio feature extraction and
//! K-means clustering for speaker assignment.
//!
//! # Approach
//!
//! 1. Extract simple audio features (energy, zero-crossing rate) for each segment
//! 2. Use K-means clustering to group segments by speaker
//! 3. Assign speaker IDs to segments based on cluster membership
//!
//! # Limitations
//!
//! This MVP implementation uses basic features and may not be as accurate as
//! dedicated speaker embedding models (e.g., ECAPA-TDNN, wespeaker). Future
//! improvements could integrate such models via ONNX Runtime.

use tracing::{debug, info};

use super::types::Segment;

/// Default maximum number of speakers for diarization.
pub const DEFAULT_MAX_SPEAKERS: u32 = 4;

/// Minimum segment duration (ms) for diarization to be meaningful.
pub const MIN_SEGMENT_DURATION_MS: i64 = 200;

/// Audio features extracted from a segment for speaker clustering.
#[derive(Debug, Clone)]
struct SegmentFeatures {
    /// Index of the segment in the original list.
    segment_idx: usize,

    /// Mean energy (RMS) of the segment.
    mean_energy: f32,

    /// Standard deviation of energy.
    energy_std: f32,

    /// Zero-crossing rate.
    zcr: f32,

    /// Spectral centroid estimate (simplified).
    spectral_centroid: f32,
}

impl SegmentFeatures {
    /// Computes features from raw audio samples for a segment.
    fn from_samples(segment_idx: usize, samples: &[f32]) -> Option<Self> {
        if samples.len() < 160 { // Less than 10ms at 16kHz
            return None;
        }

        // Mean energy (RMS)
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        let mean_energy = (sum_sq / samples.len() as f32).sqrt();

        // Energy standard deviation
        let energy_samples: Vec<f32> = samples
            .chunks(160) // 10ms frames
            .filter(|chunk| chunk.len() >= 80)
            .map(|chunk| {
                let sq_sum: f32 = chunk.iter().map(|s| s * s).sum();
                (sq_sum / chunk.len() as f32).sqrt()
            })
            .collect();

        let energy_std = if energy_samples.len() > 1 {
            let mean: f32 = energy_samples.iter().sum::<f32>() / energy_samples.len() as f32;
            let variance: f32 = energy_samples.iter().map(|e| (e - mean).powi(2)).sum::<f32>() / energy_samples.len() as f32;
            variance.sqrt()
        } else {
            0.0
        };

        // Zero-crossing rate
        let zero_crossings = samples
            .windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();
        let zcr = zero_crossings as f32 / samples.len() as f32;

        // Simplified spectral centroid using first-order difference (rough approximation)
        let diff_sum: f32 = samples.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let spectral_centroid = diff_sum / samples.len() as f32;

        Some(Self {
            segment_idx,
            mean_energy,
            energy_std,
            zcr,
            spectral_centroid,
        })
    }

    /// Returns a normalized feature vector for clustering.
    fn as_vector(&self) -> [f32; 4] {
        [self.mean_energy, self.energy_std, self.zcr, self.spectral_centroid]
    }
}

/// Configuration for speaker diarization.
#[derive(Debug, Clone)]
pub struct DiarizationConfig {
    /// Maximum number of speakers to identify.
    pub max_speakers: u32,

    /// Number of K-means iterations.
    pub kmeans_iterations: u32,

    /// Minimum energy threshold to consider a segment voiced.
    pub min_energy_threshold: f32,
}

impl Default for DiarizationConfig {
    fn default() -> Self {
        Self {
            max_speakers: DEFAULT_MAX_SPEAKERS,
            kmeans_iterations: 20,
            min_energy_threshold: 0.001,
        }
    }
}

impl DiarizationConfig {
    /// Creates a new diarization config with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of speakers.
    #[must_use]
    pub fn with_max_speakers(mut self, max_speakers: u32) -> Self {
        self.max_speakers = max_speakers.clamp(1, 10);
        self
    }
}

/// Performs speaker diarization on segments and their corresponding audio.
///
/// # Arguments
///
/// * `segments` - Transcribed segments to assign speaker IDs
/// * `audio_samples` - Full audio at 16kHz mono (must correspond to segment timestamps)
/// * `config` - Diarization configuration
///
/// # Returns
///
/// Updated segments with `speaker_id` set based on clustering.
pub fn assign_speakers(
    segments: Vec<Segment>,
    audio_samples: &[f32],
    config: &DiarizationConfig,
) -> Vec<Segment> {
    if segments.is_empty() {
        return segments;
    }

    // If only one segment or max_speakers is 1, assign all to speaker 0
    if segments.len() == 1 || config.max_speakers == 1 {
        return segments
            .into_iter()
            .map(|s| s.with_speaker(0))
            .collect();
    }

    info!("Starting diarization for {} segments", segments.len());

    // Extract features for each segment
    let sample_rate = 16000.0;
    let features: Vec<SegmentFeatures> = segments
        .iter()
        .enumerate()
        .filter_map(|(idx, seg)| {
            // Skip very short segments
            if seg.duration_ms() < MIN_SEGMENT_DURATION_MS {
                return None;
            }

            // Convert timestamps to sample indices
            let start_sample = ((seg.start_ms as f64 / 1000.0) * sample_rate) as usize;
            let end_sample = ((seg.end_ms as f64 / 1000.0) * sample_rate) as usize;

            // Bounds check
            if start_sample >= audio_samples.len() || end_sample > audio_samples.len() {
                return None;
            }

            let segment_audio = &audio_samples[start_sample..end_sample];
            SegmentFeatures::from_samples(idx, segment_audio)
        })
        .collect();

    if features.len() <= 1 {
        debug!("Too few segments for diarization, assigning all to speaker 0");
        return segments
            .into_iter()
            .map(|s| s.with_speaker(0))
            .collect();
    }

    // Determine optimal number of speakers (up to max_speakers, but not more than features)
    let num_speakers = (config.max_speakers as usize).min(features.len()).max(2);

    // Normalize features
    let normalized = normalize_features(&features);

    // Run K-means clustering
    let assignments = kmeans_cluster(&normalized, num_speakers, config.kmeans_iterations);

    // Create a map from segment index to speaker ID
    let mut speaker_map = vec![0u32; segments.len()];
    for (feat, &cluster) in features.iter().zip(assignments.iter()) {
        speaker_map[feat.segment_idx] = cluster as u32;
    }

    // Update segments with speaker IDs
    let result: Vec<Segment> = segments
        .into_iter()
        .enumerate()
        .map(|(idx, seg)| seg.with_speaker(speaker_map[idx]))
        .collect();

    // Count speakers actually used
    let unique_speakers: std::collections::HashSet<_> = result.iter().filter_map(|s| s.speaker_id).collect();
    info!("Diarization complete: {} unique speakers identified", unique_speakers.len());

    result
}

/// Normalizes feature vectors to zero mean and unit variance.
fn normalize_features(features: &[SegmentFeatures]) -> Vec<[f32; 4]> {
    if features.is_empty() {
        return vec![];
    }

    // Compute mean and std for each dimension
    let n = features.len() as f32;
    let mut means = [0.0f32; 4];
    let mut stds = [0.0f32; 4];

    for feat in features {
        let vec = feat.as_vector();
        for (i, v) in vec.iter().enumerate() {
            means[i] += v;
        }
    }
    for m in &mut means {
        *m /= n;
    }

    for feat in features {
        let vec = feat.as_vector();
        for (i, v) in vec.iter().enumerate() {
            stds[i] += (v - means[i]).powi(2);
        }
    }
    for s in &mut stds {
        *s = (*s / n).sqrt().max(0.001); // Avoid division by zero
    }

    // Normalize
    features
        .iter()
        .map(|feat| {
            let vec = feat.as_vector();
            [
                (vec[0] - means[0]) / stds[0],
                (vec[1] - means[1]) / stds[1],
                (vec[2] - means[2]) / stds[2],
                (vec[3] - means[3]) / stds[3],
            ]
        })
        .collect()
}

/// Simple K-means clustering implementation.
fn kmeans_cluster(features: &[[f32; 4]], k: usize, iterations: u32) -> Vec<usize> {
    if features.is_empty() || k == 0 {
        return vec![];
    }

    let n = features.len();
    if k >= n {
        // Each point is its own cluster
        return (0..n).collect();
    }

    // Initialize centroids using K-means++ style initialization
    let mut centroids = initialize_centroids(features, k);
    let mut assignments = vec![0usize; n];

    for iteration in 0..iterations {
        // Assign points to nearest centroid
        let mut changed = false;
        for (i, feat) in features.iter().enumerate() {
            let nearest = find_nearest_centroid(feat, &centroids);
            if assignments[i] != nearest {
                assignments[i] = nearest;
                changed = true;
            }
        }

        if !changed && iteration > 0 {
            debug!("K-means converged after {} iterations", iteration);
            break;
        }

        // Update centroids
        centroids = update_centroids(features, &assignments, k);
    }

    assignments
}

/// Initializes centroids using K-means++ style selection.
fn initialize_centroids(features: &[[f32; 4]], k: usize) -> Vec<[f32; 4]> {
    let mut centroids = Vec::with_capacity(k);

    // First centroid: first point
    centroids.push(features[0]);

    // Remaining centroids: pick points far from existing centroids
    while centroids.len() < k {
        let mut max_dist = 0.0f32;
        let mut best_idx = 0;

        for (i, feat) in features.iter().enumerate() {
            let min_dist = centroids
                .iter()
                .map(|c| euclidean_distance(feat, c))
                .fold(f32::INFINITY, f32::min);

            if min_dist > max_dist {
                max_dist = min_dist;
                best_idx = i;
            }
        }

        centroids.push(features[best_idx]);
    }

    centroids
}

/// Finds the index of the nearest centroid to a point.
fn find_nearest_centroid(point: &[f32; 4], centroids: &[[f32; 4]]) -> usize {
    centroids
        .iter()
        .enumerate()
        .map(|(i, c)| (i, euclidean_distance(point, c)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Updates centroids based on current assignments.
fn update_centroids(features: &[[f32; 4]], assignments: &[usize], k: usize) -> Vec<[f32; 4]> {
    let mut sums = vec![[0.0f32; 4]; k];
    let mut counts = vec![0usize; k];

    for (feat, &cluster) in features.iter().zip(assignments) {
        for i in 0..4 {
            sums[cluster][i] += feat[i];
        }
        counts[cluster] += 1;
    }

    sums.into_iter()
        .zip(counts)
        .map(|(sum, count)| {
            if count > 0 {
                let c = count as f32;
                [sum[0] / c, sum[1] / c, sum[2] / c, sum[3] / c]
            } else {
                sum // Empty cluster keeps its position
            }
        })
        .collect()
}

/// Euclidean distance between two 4D points.
fn euclidean_distance(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..4 {
        sum += (a[i] - b[i]).powi(2);
    }
    sum.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_features_extraction() {
        // Generate a simple test signal (sine wave)
        let samples: Vec<f32> = (0..1600)
            .map(|i| (i as f32 * 0.1).sin())
            .collect();

        let features = SegmentFeatures::from_samples(0, &samples);
        assert!(features.is_some());

        let feat = features.unwrap();
        assert!(feat.mean_energy > 0.0);
        assert!(feat.zcr > 0.0);
    }

    #[test]
    fn test_segment_features_too_short() {
        let samples = vec![0.0f32; 100]; // Too short
        let features = SegmentFeatures::from_samples(0, &samples);
        assert!(features.is_none());
    }

    #[test]
    fn test_diarization_single_segment() {
        let segments = vec![Segment::new(0, 1000, "Hello")];
        let audio = vec![0.0f32; 16000]; // 1 second of silence

        let config = DiarizationConfig::new().with_max_speakers(2);
        let result = assign_speakers(segments, &audio, &config);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].speaker_id, Some(0));
    }

    #[test]
    fn test_diarization_empty_segments() {
        let segments: Vec<Segment> = vec![];
        let audio = vec![0.0f32; 16000];

        let config = DiarizationConfig::default();
        let result = assign_speakers(segments, &audio, &config);

        assert!(result.is_empty());
    }

    #[test]
    fn test_diarization_multiple_segments() {
        // Create segments that should cluster into 2 speakers
        // Speaker 1: louder segments
        // Speaker 2: quieter segments
        let mut audio = Vec::with_capacity(80000); // 5 seconds

        // Generate audio with varying energy for different segments
        // Segment 1: 0-1s, loud
        for i in 0..16000 {
            audio.push((i as f32 * 0.1).sin() * 0.5);
        }
        // Segment 2: 1-2s, quiet
        for i in 0..16000 {
            audio.push((i as f32 * 0.1).sin() * 0.1);
        }
        // Segment 3: 2-3s, loud
        for i in 0..16000 {
            audio.push((i as f32 * 0.1).sin() * 0.5);
        }
        // Segment 4: 3-4s, quiet
        for i in 0..16000 {
            audio.push((i as f32 * 0.1).sin() * 0.1);
        }
        // Segment 5: 4-5s, loud
        for i in 0..16000 {
            audio.push((i as f32 * 0.1).sin() * 0.5);
        }

        let segments = vec![
            Segment::new(0, 1000, "Hello"),      // Loud
            Segment::new(1000, 2000, "Hi"),       // Quiet
            Segment::new(2000, 3000, "World"),    // Loud
            Segment::new(3000, 4000, "There"),    // Quiet
            Segment::new(4000, 5000, "Test"),     // Loud
        ];

        let config = DiarizationConfig::new().with_max_speakers(2);
        let result = assign_speakers(segments, &audio, &config);

        assert_eq!(result.len(), 5);

        // All segments should have speaker IDs
        for seg in &result {
            assert!(seg.speaker_id.is_some());
        }

        // Loud segments (0, 2, 4) should be same speaker
        assert_eq!(result[0].speaker_id, result[2].speaker_id);
        assert_eq!(result[2].speaker_id, result[4].speaker_id);

        // Quiet segments (1, 3) should be same speaker
        assert_eq!(result[1].speaker_id, result[3].speaker_id);

        // Loud and quiet should be different speakers
        assert_ne!(result[0].speaker_id, result[1].speaker_id);
    }

    #[test]
    fn test_kmeans_clustering() {
        // Two distinct clusters
        let features = vec![
            [0.0, 0.0, 0.0, 0.0],
            [0.1, 0.1, 0.1, 0.1],
            [10.0, 10.0, 10.0, 10.0],
            [10.1, 10.1, 10.1, 10.1],
        ];

        let assignments = kmeans_cluster(&features, 2, 20);

        // Points should be grouped correctly
        assert_eq!(assignments[0], assignments[1]);
        assert_eq!(assignments[2], assignments[3]);
        assert_ne!(assignments[0], assignments[2]);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = [0.0, 0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0, 0.0];

        assert!((euclidean_distance(&a, &b) - 1.0).abs() < 0.001);

        let c = [1.0, 1.0, 1.0, 1.0];
        assert!((euclidean_distance(&a, &c) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_diarization_config() {
        let config = DiarizationConfig::new()
            .with_max_speakers(3);

        assert_eq!(config.max_speakers, 3);
        assert_eq!(config.kmeans_iterations, 20);
    }

    #[test]
    fn test_diarization_config_clamping() {
        let config = DiarizationConfig::new()
            .with_max_speakers(100); // Should be clamped to 10

        assert_eq!(config.max_speakers, 10);

        let config2 = DiarizationConfig::new()
            .with_max_speakers(0); // Should be clamped to 1

        assert_eq!(config2.max_speakers, 1);
    }

    #[test]
    fn test_normalize_features() {
        let features = vec![
            SegmentFeatures {
                segment_idx: 0,
                mean_energy: 0.5,
                energy_std: 0.1,
                zcr: 0.1,
                spectral_centroid: 0.2,
            },
            SegmentFeatures {
                segment_idx: 1,
                mean_energy: 1.5,
                energy_std: 0.3,
                zcr: 0.3,
                spectral_centroid: 0.6,
            },
        ];

        let normalized = normalize_features(&features);
        assert_eq!(normalized.len(), 2);

        // Mean of normalized features should be near 0
        let mean: f32 = normalized.iter().map(|f| f[0]).sum::<f32>() / 2.0;
        assert!(mean.abs() < 0.001);
    }
}
