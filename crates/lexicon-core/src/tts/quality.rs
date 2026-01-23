//! TTS Quality Validation
//!
//! This module provides test sentences and quality assessment infrastructure
//! for validating TTS output quality. Used for Beta exit criteria validation.
//!
//! # Usage
//!
//! ```no_run
//! use lexicon_core::tts::quality::{QUALITY_TEST_SENTENCES, QualityAssessment};
//!
//! // Generate audio for each test sentence
//! for sentence in QUALITY_TEST_SENTENCES {
//!     // Synthesize and evaluate...
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Standard test sentences for TTS quality evaluation.
///
/// These sentences are designed to test various aspects of TTS:
/// - Basic conversational speech
/// - Numbers and dates
/// - Abbreviations
/// - Punctuation and prosody
/// - Technical terms
/// - Long sentences
/// - Difficult pronunciations
pub const QUALITY_TEST_SENTENCES: &[&str] = &[
    // Basic sentences
    "Hello, how are you today?",
    "The quick brown fox jumps over the lazy dog.",

    // Numbers and dates
    "The meeting is scheduled for January 15th, 2026 at 3:30 PM.",
    "The price is $1,234.56 including tax.",
    "Call me at 555-123-4567.",

    // Abbreviations
    "Dr. Smith works at MIT in the USA.",
    "The CEO of IBM met with the FBI director.",

    // Punctuation and emphasis
    "Wait... what did you say?",
    "This is amazing! I can't believe it!",
    "Should we go left, right, or straight ahead?",

    // Technical terms
    "The API returns JSON data over HTTPS.",
    "Configure the WiFi settings in the iOS app.",

    // Long sentences
    "In order to successfully complete this task, you will need to carefully \
     follow each step of the instructions provided in the documentation.",

    // Difficult words
    "The entrepreneur scheduled a rendezvous at the boutique.",
    "The colonel analyzed the thoroughly sophisticated algorithm.",
];

/// Quality assessment criteria for a single sentence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessment {
    /// Sentence that was tested
    pub sentence: String,
    /// Intelligibility (1-5): Can words be understood?
    pub intelligibility: u8,
    /// Naturalness (1-5): Does it sound human-like?
    pub naturalness: u8,
    /// Pronunciation (1-5): Are words pronounced correctly?
    pub pronunciation: u8,
    /// Prosody (1-5): Is rhythm/intonation natural?
    pub prosody: u8,
    /// Notes on specific issues observed
    pub notes: String,
}

impl QualityAssessment {
    /// Creates a new quality assessment for a sentence.
    pub fn new(sentence: impl Into<String>) -> Self {
        Self {
            sentence: sentence.into(),
            intelligibility: 0,
            naturalness: 0,
            pronunciation: 0,
            prosody: 0,
            notes: String::new(),
        }
    }

    /// Sets the intelligibility score (1-5).
    #[must_use]
    pub fn with_intelligibility(mut self, score: u8) -> Self {
        self.intelligibility = score.clamp(1, 5);
        self
    }

    /// Sets the naturalness score (1-5).
    #[must_use]
    pub fn with_naturalness(mut self, score: u8) -> Self {
        self.naturalness = score.clamp(1, 5);
        self
    }

    /// Sets the pronunciation score (1-5).
    #[must_use]
    pub fn with_pronunciation(mut self, score: u8) -> Self {
        self.pronunciation = score.clamp(1, 5);
        self
    }

    /// Sets the prosody score (1-5).
    #[must_use]
    pub fn with_prosody(mut self, score: u8) -> Self {
        self.prosody = score.clamp(1, 5);
        self
    }

    /// Adds notes about the assessment.
    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }

    /// Calculates the Mean Opinion Score (MOS).
    ///
    /// MOS is the average of all four quality dimensions.
    /// Range: 1.0 (poor) to 5.0 (excellent).
    #[must_use]
    pub fn mos(&self) -> f32 {
        if self.intelligibility == 0 || self.naturalness == 0
            || self.pronunciation == 0 || self.prosody == 0 {
            return 0.0;
        }
        (self.intelligibility + self.naturalness + self.pronunciation + self.prosody) as f32 / 4.0
    }

    /// Checks if this assessment meets the minimum quality bar.
    ///
    /// Minimum requirements:
    /// - Intelligibility >= 4 (words must be clearly understood)
    /// - Naturalness >= 3 (at least fair synthetic quality)
    /// - Pronunciation >= 4 (words pronounced correctly)
    /// - No individual score below 2
    #[must_use]
    pub fn meets_quality_bar(&self) -> bool {
        self.intelligibility >= 4
            && self.naturalness >= 3
            && self.pronunciation >= 4
            && self.prosody >= 2
            && self.intelligibility >= 2
            && self.naturalness >= 2
            && self.pronunciation >= 2
    }

    /// Returns true if all scores have been filled in.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.intelligibility > 0
            && self.naturalness > 0
            && self.pronunciation > 0
            && self.prosody > 0
    }
}

/// Complete quality report for a TTS validation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    /// When the test was conducted
    pub timestamp: String,
    /// Model version being tested
    pub model_version: String,
    /// Engine used (Kokoro, Platform, etc.)
    pub engine: String,
    /// Individual assessments for each sentence
    pub assessments: Vec<QualityAssessment>,
}

impl QualityReport {
    /// Creates a new empty quality report.
    pub fn new(model_version: impl Into<String>, engine: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            model_version: model_version.into(),
            engine: engine.into(),
            assessments: Vec::new(),
        }
    }

    /// Adds an assessment to the report.
    pub fn add_assessment(&mut self, assessment: QualityAssessment) {
        self.assessments.push(assessment);
    }

    /// Calculates the average MOS across all assessments.
    #[must_use]
    pub fn average_mos(&self) -> f32 {
        let complete: Vec<_> = self.assessments.iter()
            .filter(|a| a.is_complete())
            .collect();

        if complete.is_empty() {
            return 0.0;
        }

        complete.iter().map(|a| a.mos()).sum::<f32>() / complete.len() as f32
    }

    /// Calculates the average intelligibility score.
    #[must_use]
    pub fn average_intelligibility(&self) -> f32 {
        self.average_dimension(|a| a.intelligibility)
    }

    /// Calculates the average naturalness score.
    #[must_use]
    pub fn average_naturalness(&self) -> f32 {
        self.average_dimension(|a| a.naturalness)
    }

    /// Calculates the average pronunciation score.
    #[must_use]
    pub fn average_pronunciation(&self) -> f32 {
        self.average_dimension(|a| a.pronunciation)
    }

    /// Calculates the average prosody score.
    #[must_use]
    pub fn average_prosody(&self) -> f32 {
        self.average_dimension(|a| a.prosody)
    }

    /// Helper to calculate average for a dimension.
    fn average_dimension<F>(&self, getter: F) -> f32
    where
        F: Fn(&QualityAssessment) -> u8,
    {
        let complete: Vec<_> = self.assessments.iter()
            .filter(|a| a.is_complete())
            .collect();

        if complete.is_empty() {
            return 0.0;
        }

        complete.iter().map(|a| getter(a) as f32).sum::<f32>() / complete.len() as f32
    }

    /// Checks if the overall quality meets the beta release bar.
    ///
    /// Requirements:
    /// - Average MOS >= 3.5
    /// - All assessments complete
    /// - No individual MOS below 2.0
    #[must_use]
    pub fn meets_beta_criteria(&self) -> bool {
        if self.assessments.is_empty() {
            return false;
        }

        // All assessments must be complete
        if !self.assessments.iter().all(|a| a.is_complete()) {
            return false;
        }

        // Average MOS must be >= 3.5
        if self.average_mos() < 3.5 {
            return false;
        }

        // No individual MOS below 2.0
        if self.assessments.iter().any(|a| a.mos() < 2.0) {
            return false;
        }

        true
    }

    /// Returns assessments that don't meet the quality bar.
    #[must_use]
    pub fn failing_assessments(&self) -> Vec<&QualityAssessment> {
        self.assessments.iter()
            .filter(|a| a.is_complete() && !a.meets_quality_bar())
            .collect()
    }

    /// Generates a summary of the report.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str(&format!("TTS Quality Report - {}\n", self.timestamp));
        summary.push_str(&format!("Engine: {} ({})\n\n", self.engine, self.model_version));

        let complete_count = self.assessments.iter().filter(|a| a.is_complete()).count();
        summary.push_str(&format!("Assessments: {}/{}\n", complete_count, self.assessments.len()));

        if complete_count > 0 {
            summary.push_str(&format!("\nScores:\n"));
            summary.push_str(&format!("  Intelligibility: {:.1}\n", self.average_intelligibility()));
            summary.push_str(&format!("  Naturalness:     {:.1}\n", self.average_naturalness()));
            summary.push_str(&format!("  Pronunciation:   {:.1}\n", self.average_pronunciation()));
            summary.push_str(&format!("  Prosody:         {:.1}\n", self.average_prosody()));
            summary.push_str(&format!("  Average MOS:     {:.2}\n", self.average_mos()));

            let pass_status = if self.meets_beta_criteria() { "PASS ✓" } else { "FAIL ✗" };
            summary.push_str(&format!("\nBeta Criteria: {}\n", pass_status));

            let failing = self.failing_assessments();
            if !failing.is_empty() {
                summary.push_str(&format!("\nFailing sentences ({}):\n", failing.len()));
                for a in failing {
                    summary.push_str(&format!("  - {} (MOS: {:.1})\n",
                        if a.sentence.len() > 40 {
                            format!("{}...", &a.sentence[..40])
                        } else {
                            a.sentence.clone()
                        },
                        a.mos()
                    ));
                }
            }
        }

        summary
    }
}

/// MOS (Mean Opinion Score) scale descriptions.
pub mod mos_scale {
    /// Excellent - Indistinguishable from human speech
    pub const EXCELLENT: u8 = 5;
    /// Good - Natural with only minor issues
    pub const GOOD: u8 = 4;
    /// Fair - Clearly synthetic but understandable
    pub const FAIR: u8 = 3;
    /// Poor - Some difficulty understanding
    pub const POOR: u8 = 2;
    /// Bad - Unintelligible
    pub const BAD: u8 = 1;

    /// Returns a description for a MOS score.
    #[must_use]
    pub fn description(score: u8) -> &'static str {
        match score {
            5 => "Excellent - Indistinguishable from human",
            4 => "Good - Natural with minor issues",
            3 => "Fair - Understandable but synthetic",
            2 => "Poor - Difficult to understand",
            1 => "Bad - Unintelligible",
            _ => "Invalid score",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_test_sentences_not_empty() {
        assert!(!QUALITY_TEST_SENTENCES.is_empty());
        assert!(QUALITY_TEST_SENTENCES.len() >= 10, "Should have at least 10 test sentences");
    }

    #[test]
    fn test_quality_assessment_mos() {
        let assessment = QualityAssessment::new("Hello")
            .with_intelligibility(4)
            .with_naturalness(3)
            .with_pronunciation(4)
            .with_prosody(3);

        assert!((assessment.mos() - 3.5).abs() < 0.001);
    }

    #[test]
    fn test_quality_assessment_incomplete() {
        let assessment = QualityAssessment::new("Hello")
            .with_intelligibility(4);

        assert!(!assessment.is_complete());
        assert_eq!(assessment.mos(), 0.0);
    }

    #[test]
    fn test_quality_assessment_meets_bar() {
        // Passing assessment
        let passing = QualityAssessment::new("Hello")
            .with_intelligibility(4)
            .with_naturalness(3)
            .with_pronunciation(4)
            .with_prosody(3);
        assert!(passing.meets_quality_bar());

        // Failing on intelligibility
        let failing_intel = QualityAssessment::new("Hello")
            .with_intelligibility(3) // Too low
            .with_naturalness(4)
            .with_pronunciation(4)
            .with_prosody(4);
        assert!(!failing_intel.meets_quality_bar());

        // Failing on naturalness
        let failing_natural = QualityAssessment::new("Hello")
            .with_intelligibility(4)
            .with_naturalness(2) // Too low
            .with_pronunciation(4)
            .with_prosody(4);
        assert!(!failing_natural.meets_quality_bar());
    }

    #[test]
    fn test_quality_report_average_mos() {
        let mut report = QualityReport::new("v0.19", "Kokoro");

        report.add_assessment(
            QualityAssessment::new("Sentence 1")
                .with_intelligibility(4)
                .with_naturalness(4)
                .with_pronunciation(4)
                .with_prosody(4)
        );

        report.add_assessment(
            QualityAssessment::new("Sentence 2")
                .with_intelligibility(3)
                .with_naturalness(3)
                .with_pronunciation(3)
                .with_prosody(3)
        );

        assert!((report.average_mos() - 3.5).abs() < 0.001);
    }

    #[test]
    fn test_quality_report_meets_beta_criteria() {
        let mut passing_report = QualityReport::new("v0.19", "Kokoro");

        // Add passing assessments
        for sentence in &QUALITY_TEST_SENTENCES[..3] {
            passing_report.add_assessment(
                QualityAssessment::new(*sentence)
                    .with_intelligibility(4)
                    .with_naturalness(4)
                    .with_pronunciation(4)
                    .with_prosody(4)
            );
        }

        assert!(passing_report.meets_beta_criteria());

        // Create failing report (low MOS)
        let mut failing_report = QualityReport::new("v0.19", "Kokoro");
        failing_report.add_assessment(
            QualityAssessment::new("Test")
                .with_intelligibility(2)
                .with_naturalness(2)
                .with_pronunciation(2)
                .with_prosody(2)
        );

        assert!(!failing_report.meets_beta_criteria());
    }

    #[test]
    fn test_quality_report_summary() {
        let mut report = QualityReport::new("v0.19", "Kokoro");
        report.add_assessment(
            QualityAssessment::new("Hello")
                .with_intelligibility(4)
                .with_naturalness(4)
                .with_pronunciation(4)
                .with_prosody(4)
        );

        let summary = report.summary();
        assert!(summary.contains("Kokoro"));
        assert!(summary.contains("Average MOS"));
        assert!(summary.contains("4.0"));
    }

    #[test]
    fn test_mos_scale_descriptions() {
        assert!(mos_scale::description(5).contains("Excellent"));
        assert!(mos_scale::description(4).contains("Good"));
        assert!(mos_scale::description(3).contains("Fair"));
        assert!(mos_scale::description(2).contains("Poor"));
        assert!(mos_scale::description(1).contains("Bad"));
        assert!(mos_scale::description(0).contains("Invalid"));
    }
}
