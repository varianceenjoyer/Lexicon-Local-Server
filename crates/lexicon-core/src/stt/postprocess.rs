//! Rule-based post-processing for STT output.
//!
//! This module provides text corrections to reduce Word Error Rate (WER)
//! with minimal latency impact (~5-10ms for typical utterances).
//!
//! # Rules Applied
//!
//! 1. **Capitalization**: Sentence starts, standalone "I"
//! 2. **Homophones**: Context-aware corrections (your/you're, their/there/they're)
//! 3. **Filler removal**: Remove "um", "uh", "er", "ah"
//! 4. **Punctuation inference**: Question marks for interrogative patterns
//!
//! # Example
//!
//! ```
//! use lexicon_core::stt::{PostProcessConfig, PostProcessor};
//!
//! let processor = PostProcessor::new(PostProcessConfig::default());
//! let result = processor.process("um i think your going to the store");
//! assert_eq!(result, "I think you're going to the store.");
//! ```

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

/// Configuration for post-processing rules.
#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    /// Enable capitalization corrections.
    pub capitalize: bool,

    /// Enable homophone corrections.
    pub correct_homophones: bool,

    /// Remove filler words (um, uh, etc.).
    pub remove_fillers: bool,

    /// Enable punctuation inference.
    pub infer_punctuation: bool,

    /// Expand contractions (gonna -> going to).
    pub expand_contractions: bool,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            capitalize: true,
            correct_homophones: true,
            remove_fillers: true,
            infer_punctuation: true,
            expand_contractions: false, // Off by default - preserves speaker style
        }
    }
}

impl PostProcessConfig {
    /// Creates a new config with all rules enabled.
    #[must_use]
    pub fn all_enabled() -> Self {
        Self {
            capitalize: true,
            correct_homophones: true,
            remove_fillers: true,
            infer_punctuation: true,
            expand_contractions: true,
        }
    }

    /// Creates a new config with all rules disabled.
    #[must_use]
    pub fn none() -> Self {
        Self {
            capitalize: false,
            correct_homophones: false,
            remove_fillers: false,
            infer_punctuation: false,
            expand_contractions: false,
        }
    }
}

/// Context for homophone correction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomophoneContext {
    /// Correct when followed by a verb (you're going, they're coming)
    VerbFollows,
    /// Correct in possessive context (their car, its wheels)
    Possessive,
    /// Correct in comparison context (better than, rather than)
    Comparison,
    /// Correct for degree/extent (too much, too many)
    Degree,
}

/// Rule for homophone correction.
#[derive(Debug, Clone)]
struct HomophoneRule {
    /// The corrected form
    correction: &'static str,
    /// Context that triggers this correction
    context: HomophoneContext,
}

/// Post-processor for STT output.
pub struct PostProcessor {
    config: PostProcessConfig,
    /// Homophone rules: (wrong_word, rule)
    homophones: HashMap<&'static str, Vec<HomophoneRule>>,
    /// Common contractions to expand: (contraction, expansion)
    contractions: HashMap<&'static str, &'static str>,
    /// Filler words to remove
    fillers: HashSet<&'static str>,
    /// Words that typically follow verbs (for homophone detection)
    verb_indicators: HashSet<&'static str>,
    /// Question words for punctuation inference
    question_words: HashSet<&'static str>,
}

impl Default for PostProcessor {
    fn default() -> Self {
        Self::new(PostProcessConfig::default())
    }
}

impl PostProcessor {
    /// Creates a new post-processor with the given configuration.
    #[must_use]
    pub fn new(config: PostProcessConfig) -> Self {
        let mut homophones: HashMap<&'static str, Vec<HomophoneRule>> = HashMap::new();

        // your -> you're (when followed by verb)
        homophones.insert(
            "your",
            vec![HomophoneRule {
                correction: "you're",
                context: HomophoneContext::VerbFollows,
            }],
        );

        // there -> they're (when followed by verb) or their (possessive)
        homophones.insert(
            "there",
            vec![
                HomophoneRule {
                    correction: "they're",
                    context: HomophoneContext::VerbFollows,
                },
            ],
        );

        // its -> it's (when followed by verb)
        homophones.insert(
            "its",
            vec![HomophoneRule {
                correction: "it's",
                context: HomophoneContext::VerbFollows,
            }],
        );

        // then -> than (in comparisons)
        homophones.insert(
            "then",
            vec![HomophoneRule {
                correction: "than",
                context: HomophoneContext::Comparison,
            }],
        );

        // to -> too (for degree)
        homophones.insert(
            "to",
            vec![HomophoneRule {
                correction: "too",
                context: HomophoneContext::Degree,
            }],
        );

        let mut contractions = HashMap::new();
        contractions.insert("gonna", "going to");
        contractions.insert("wanna", "want to");
        contractions.insert("gotta", "got to");
        contractions.insert("kinda", "kind of");
        contractions.insert("dunno", "don't know");
        contractions.insert("lemme", "let me");
        contractions.insert("gimme", "give me");

        let mut fillers = HashSet::new();
        fillers.insert("um");
        fillers.insert("uh");
        fillers.insert("er");
        fillers.insert("ah");
        fillers.insert("hmm");
        fillers.insert("hm");

        // Words that indicate the previous word should be a verb form
        let mut verb_indicators = HashSet::new();
        // Common verbs and verb forms
        verb_indicators.insert("going");
        verb_indicators.insert("coming");
        verb_indicators.insert("doing");
        verb_indicators.insert("being");
        verb_indicators.insert("having");
        verb_indicators.insert("making");
        verb_indicators.insert("taking");
        verb_indicators.insert("getting");
        verb_indicators.insert("trying");
        verb_indicators.insert("looking");
        verb_indicators.insert("working");
        verb_indicators.insert("running");
        verb_indicators.insert("walking");
        verb_indicators.insert("talking");
        verb_indicators.insert("saying");
        verb_indicators.insert("thinking");
        verb_indicators.insert("feeling");
        verb_indicators.insert("playing");
        verb_indicators.insert("watching");
        verb_indicators.insert("waiting");
        verb_indicators.insert("starting");
        verb_indicators.insert("leaving");
        verb_indicators.insert("moving");
        // Auxiliary/modal verbs
        verb_indicators.insert("not");
        verb_indicators.insert("so");
        verb_indicators.insert("really");
        verb_indicators.insert("just");
        verb_indicators.insert("always");
        verb_indicators.insert("never");
        verb_indicators.insert("still");
        verb_indicators.insert("already");
        // Adjectives that follow "it's" or "you're"
        verb_indicators.insert("good");
        verb_indicators.insert("great");
        verb_indicators.insert("nice");
        verb_indicators.insert("fine");
        verb_indicators.insert("okay");
        verb_indicators.insert("ok");
        verb_indicators.insert("right");
        verb_indicators.insert("true");
        verb_indicators.insert("important");
        verb_indicators.insert("possible");
        verb_indicators.insert("hard");
        verb_indicators.insert("easy");

        let mut question_words = HashSet::new();
        question_words.insert("what");
        question_words.insert("when");
        question_words.insert("where");
        question_words.insert("why");
        question_words.insert("how");
        question_words.insert("who");
        question_words.insert("which");
        question_words.insert("is");
        question_words.insert("are");
        question_words.insert("do");
        question_words.insert("does");
        question_words.insert("did");
        question_words.insert("can");
        question_words.insert("could");
        question_words.insert("would");
        question_words.insert("should");
        question_words.insert("will");
        question_words.insert("shall");
        question_words.insert("have");
        question_words.insert("has");
        question_words.insert("was");
        question_words.insert("were");

        Self {
            config,
            homophones,
            contractions,
            fillers,
            verb_indicators,
            question_words,
        }
    }

    /// Process transcription text with all enabled rules.
    ///
    /// Handles both lowercase and UPPERCASE input from STT models.
    /// Target latency: <10ms for typical utterance (20-50 words)
    #[must_use]
    pub fn process(&self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }

        // Tokenize
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return String::new();
        }

        // Convert all words to lowercase first (handles UPPERCASE model output)
        let lower_words: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();

        let mut result = Vec::with_capacity(words.len());
        let mut is_sentence_start = true;

        for (i, lower) in lower_words.iter().enumerate() {
            let next_word = lower_words.get(i + 1).map(|s| s.as_str());
            let prev_word = if i > 0 {
                Some(lower_words[i - 1].as_str())
            } else {
                None
            };

            // Skip filler words
            if self.config.remove_fillers && self.fillers.contains(lower.as_str()) {
                continue;
            }

            // Start with lowercase word
            let mut processed: Cow<'_, str> = Cow::Borrowed(lower.as_str());

            // Expand contractions
            if self.config.expand_contractions {
                if let Some(expansion) = self.contractions.get(lower.as_str()) {
                    processed = Cow::Borrowed(*expansion);
                }
            }

            // Correct homophones
            if self.config.correct_homophones {
                if let Some(rules) = self.homophones.get(lower.as_str()) {
                    for rule in rules {
                        if self.context_matches(rule.context, prev_word, next_word) {
                            processed = Cow::Borrowed(rule.correction);
                            break;
                        }
                    }
                }
            }

            // Apply capitalization (now works since text is lowercase)
            if self.config.capitalize {
                processed = self.apply_capitalization(processed, is_sentence_start);
            }

            // Check if this word ends a sentence (for next iteration)
            is_sentence_start = processed.ends_with('.')
                || processed.ends_with('!')
                || processed.ends_with('?');

            result.push(processed.into_owned());
        }

        if result.is_empty() {
            return String::new();
        }

        let mut output = result.join(" ");

        // Add final punctuation if missing
        if self.config.infer_punctuation {
            output = self.infer_final_punctuation(output, &lower_words);
        }

        output
    }

    /// Check if context matches for homophone correction.
    fn context_matches(&self, context: HomophoneContext, prev: Option<&str>, next: Option<&str>) -> bool {
        match context {
            HomophoneContext::VerbFollows => {
                if let Some(next_word) = next {
                    self.verb_indicators.contains(next_word)
                } else {
                    false
                }
            }
            HomophoneContext::Possessive => {
                // This is harder to detect without a full parser
                // For now, don't auto-correct to possessive form
                false
            }
            HomophoneContext::Comparison => {
                // "better than", "rather than", "more than", "less than"
                if let Some(prev_word) = prev {
                    matches!(prev_word, "better" | "rather" | "more" | "less" | "other" | "greater" | "fewer")
                } else {
                    false
                }
            }
            HomophoneContext::Degree => {
                // "too much", "too many", "too big", "too small", etc.
                if let Some(next_word) = next {
                    matches!(next_word, "much" | "many" | "big" | "small" | "fast" | "slow" | "late" | "early" | "long" | "short" | "far" | "hard" | "easy" | "bad" | "good")
                } else {
                    false
                }
            }
        }
    }

    /// Apply capitalization rules.
    fn apply_capitalization<'a>(&self, word: Cow<'a, str>, is_sentence_start: bool) -> Cow<'a, str> {
        // Always capitalize "I" as standalone word
        if word.eq_ignore_ascii_case("i") && word.len() == 1 {
            return Cow::Borrowed("I");
        }

        // Capitalize sentence starts
        if is_sentence_start && !word.is_empty() {
            let mut chars: Vec<char> = word.chars().collect();
            if let Some(first) = chars.first_mut() {
                if first.is_ascii_lowercase() {
                    *first = first.to_ascii_uppercase();
                    return Cow::Owned(chars.into_iter().collect());
                }
            }
        }

        word
    }

    /// Infer final punctuation based on sentence structure.
    fn infer_final_punctuation(&self, mut text: String, words: &[String]) -> String {
        // Don't add if already has ending punctuation
        if text.ends_with('.') || text.ends_with('!') || text.ends_with('?') {
            return text;
        }

        // Check if it's a question (words are already lowercase)
        if let Some(first_word) = words.first() {
            if self.question_words.contains(first_word.as_str()) {
                text.push('?');
                return text;
            }
        }

        // Default to period
        text.push('.');
        text
    }

    /// Process a slice of segments, applying post-processing to each.
    pub fn process_segments(&self, segments: Vec<super::Segment>) -> Vec<super::Segment> {
        segments
            .into_iter()
            .map(|mut seg| {
                seg.text = self.process(&seg.text);
                seg
            })
            .filter(|seg| !seg.text.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalization_sentence_start() {
        let processor = PostProcessor::default();
        assert!(processor.process("hello world").starts_with("Hello"));
    }

    #[test]
    fn test_capitalization_i() {
        let processor = PostProcessor::default();
        let result = processor.process("i think i can do it");
        assert!(result.contains("I think"));
        assert!(result.contains("I can"));
    }

    #[test]
    fn test_homophone_your_youre() {
        let processor = PostProcessor::default();
        // "your going" should become "you're going" (capitalized at sentence start)
        let result = processor.process("your going home");
        assert!(result.contains("You're going") || result.contains("you're going"), "Got: {}", result);

        // "your house" should stay as "your house" (not verb context)
        let result2 = processor.process("your house is nice");
        assert!(result2.contains("your house") || result2.contains("Your house"), "Got: {}", result2);
    }

    #[test]
    fn test_homophone_its_its() {
        let processor = PostProcessor::default();
        let result = processor.process("its going to rain");
        // Capitalized at sentence start
        assert!(result.contains("It's going") || result.contains("it's going"), "Got: {}", result);
    }

    #[test]
    fn test_homophone_then_than() {
        let processor = PostProcessor::default();
        let result = processor.process("better then expected");
        // Capitalized at sentence start
        assert!(result.contains("Better than") || result.contains("better than"), "Got: {}", result);
    }

    #[test]
    fn test_homophone_to_too() {
        let processor = PostProcessor::default();
        let result = processor.process("it was to much");
        assert!(result.contains("too much"), "Got: {}", result);
    }

    #[test]
    fn test_filler_removal() {
        let processor = PostProcessor::default();
        let result = processor.process("um i think uh maybe");
        assert!(!result.contains("um"), "Got: {}", result);
        assert!(!result.contains("uh"), "Got: {}", result);
        assert!(result.contains("I think"), "Got: {}", result);
    }

    #[test]
    fn test_contraction_expansion() {
        let processor = PostProcessor::new(PostProcessConfig::all_enabled());
        let result = processor.process("i gonna do it");
        assert!(result.contains("going to"), "Got: {}", result);
    }

    #[test]
    fn test_contraction_expansion_disabled() {
        let processor = PostProcessor::default(); // expand_contractions is false by default
        let result = processor.process("i gonna do it");
        assert!(result.contains("gonna"), "Got: {}", result);
    }

    #[test]
    fn test_question_punctuation() {
        let processor = PostProcessor::default();
        let result = processor.process("what time is it");
        assert!(result.ends_with('?'), "Got: {}", result);
    }

    #[test]
    fn test_statement_punctuation() {
        let processor = PostProcessor::default();
        let result = processor.process("the weather is nice");
        assert!(result.ends_with('.'), "Got: {}", result);
    }

    #[test]
    fn test_empty_input() {
        let processor = PostProcessor::default();
        assert_eq!(processor.process(""), "");
    }

    #[test]
    fn test_only_fillers() {
        let processor = PostProcessor::default();
        assert_eq!(processor.process("um uh er"), "");
    }

    #[test]
    fn test_config_none() {
        let processor = PostProcessor::new(PostProcessConfig::none());
        let result = processor.process("um your going");
        // Nothing should be changed except joining words
        assert!(result.contains("um"), "Got: {}", result);
        assert!(result.contains("your"), "Got: {}", result);
    }

    #[test]
    fn test_processing_latency() {
        let processor = PostProcessor::default();
        let long_text = "this is a test sentence with some words ".repeat(20); // ~160 words

        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = processor.process(&long_text);
        }
        let elapsed = start.elapsed();
        let per_call = elapsed.as_micros() / 100;

        // Should be well under 10ms (10000 microseconds) per call
        assert!(
            per_call < 10000,
            "Processing took {}us per call, expected <10000us",
            per_call
        );
    }

    #[test]
    fn test_uppercase_input() {
        let processor = PostProcessor::default();
        // Sherpa-onnx outputs ALL CAPS - verify we handle it
        let result = processor.process("LO THIS IS THE TARANTULAS DEN");
        assert!(result.starts_with("Lo this"), "Should lowercase and capitalize first: {}", result);
        assert!(result.ends_with('.'), "Should add period: {}", result);
    }

    #[test]
    fn test_uppercase_with_homophones() {
        let processor = PostProcessor::default();
        let result = processor.process("YOUR GOING TO THE STORE");
        assert!(result.contains("You're going"), "Should fix homophone: {}", result);
    }

    #[test]
    fn test_uppercase_question() {
        let processor = PostProcessor::default();
        let result = processor.process("WHAT TIME IS IT");
        assert!(result.ends_with('?'), "Should add question mark: {}", result);
        assert!(result.starts_with("What"), "Should capitalize first word: {}", result);
    }

    #[test]
    fn test_real_world_example() {
        let processor = PostProcessor::default();
        let input = "um i think your going to the store and its going to rain so you should bring an umbrella";
        let result = processor.process(input);

        // Should have:
        // - Removed "um"
        // - Capitalized "I"
        // - Changed "your going" to "you're going"
        // - Changed "its going" to "it's going"
        // - Added final punctuation

        assert!(!result.contains("um "), "Filler not removed: {}", result);
        assert!(result.starts_with("I think"), "Not capitalized: {}", result);
        assert!(result.contains("you're going"), "Homophone not corrected: {}", result);
        assert!(result.contains("it's going"), "Homophone not corrected: {}", result);
        assert!(result.ends_with('.') || result.ends_with('?'), "No punctuation: {}", result);
    }
}
