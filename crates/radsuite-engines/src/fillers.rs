//! Timing and context rules ported from the Python RADcast speech cleanup service.
use crate::{AudioTimeInterval, CaptionWord, FillerRemovalMode};

struct Heuristics {
    probability: f64,
    strong_probability: f64,
    context_gap: f64,
    strong_side: f64,
    single_side: f64,
    two_sided_gap: f64,
    two_sided_min: f64,
    internal_gap: f64,
    lead_cap: f64,
    tail_cap: f64,
    pad_ratio: f64,
    always_accept: bool,
}

impl Heuristics {
    fn for_mode(mode: FillerRemovalMode) -> Self {
        match mode {
            FillerRemovalMode::Normal => Self {
                probability: 0.28,
                strong_probability: 0.34,
                context_gap: 0.14,
                strong_side: 0.06,
                single_side: 0.15,
                two_sided_gap: 0.1,
                two_sided_min: 0.04,
                internal_gap: 0.18,
                lead_cap: 0.025,
                tail_cap: 0.05,
                pad_ratio: 0.5,
                always_accept: false,
            },
            FillerRemovalMode::Aggressive => Self {
                probability: 0.08,
                strong_probability: 0.22,
                context_gap: 0.11,
                strong_side: 0.04,
                single_side: 0.1,
                two_sided_gap: 0.06,
                two_sided_min: 0.018,
                internal_gap: 0.32,
                lead_cap: 0.035,
                tail_cap: 0.07,
                pad_ratio: 0.55,
                always_accept: true,
            },
        }
    }
}

pub(crate) fn normalized_token(text: &str) -> String {
    text.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || *c == '\'')
        .collect::<String>()
        .trim_matches('\'')
        .to_string()
}

pub(crate) fn is_filler_token(text: &str) -> bool {
    let token = normalized_token(text);
    if matches!(
        token.as_str(),
        "ah" | "ahh" | "erm" | "er" | "uh" | "uhh" | "uhm" | "um" | "umm"
    ) {
        return true;
    }
    // Equivalent to u+m+, u+h+, u+h+m+, e+r+m+, a+h+ without a regex dependency.
    ["um", "uh", "uhm", "erm", "ah"].iter().any(|pattern| {
        let mut rest = token.as_str();
        for letter in pattern.chars() {
            let trimmed = rest.trim_start_matches(letter);
            if trimmed.len() == rest.len() {
                return false;
            }
            rest = trimmed;
        }
        rest.is_empty()
    })
}

pub(crate) struct FillerDetection {
    pub intervals: Vec<AudioTimeInterval>,
    pub accepted_words: Vec<bool>,
    pub count: usize,
}

pub(crate) fn detect(words: &[CaptionWord], mode: FillerRemovalMode) -> FillerDetection {
    let h = Heuristics::for_mode(mode);
    let mut result = FillerDetection {
        intervals: Vec::new(),
        accepted_words: vec![false; words.len()],
        count: 0,
    };
    let mut index = 0;
    while index < words.len() {
        if !is_filler_token(&words[index].text) {
            index += 1;
            continue;
        }
        let start_index = index;
        index += 1;
        while index < words.len()
            && is_filler_token(&words[index].text)
            && (words[index].start_seconds - words[index - 1].end_seconds).max(0.0)
                <= h.internal_gap
        {
            index += 1;
        }
        let start = words[start_index].start_seconds;
        let end = words[index - 1].end_seconds;
        let duration = end - start;
        // Millisecond timestamps at the exact limits can differ by floating-point noise.
        if !(0.08 - 1e-9..=1.35 + 1e-9).contains(&duration) {
            continue;
        }
        let probabilities: Vec<f64> = words[start_index..index]
            .iter()
            .map(|w| w.probability)
            .filter(|p| p.is_finite())
            .collect();
        if !h.always_accept
            && !probabilities.is_empty()
            && probabilities.iter().sum::<f64>() / (probabilities.len() as f64) < h.probability
            && probabilities.iter().copied().fold(0.0, f64::max) < h.strong_probability
        {
            continue;
        }
        let previous_end = if start_index > 0 {
            words[start_index - 1].end_seconds
        } else {
            0.0
        };
        let next_start = words.get(index).map_or(end, |w| w.start_seconds);
        let before = (start - previous_end).max(0.0);
        let after = (next_start - end).max(0.0);
        let total = before + after;
        let strongest = before.max(after);
        if !((total >= h.context_gap && strongest >= h.strong_side)
            || (total >= h.two_sided_gap && before >= h.two_sided_min && after >= h.two_sided_min)
            || strongest >= h.single_side)
        {
            continue;
        }
        result.intervals.push(AudioTimeInterval {
            start_seconds: round_timing((start - h.lead_cap.min(before * h.pad_ratio)).max(0.0)),
            end_seconds: round_timing(end + h.tail_cap.min(after * h.pad_ratio)),
        });
        result.accepted_words[start_index..index].fill(true);
        result.count += index - start_index;
    }
    result
}

fn round_timing(seconds: f64) -> f64 {
    (seconds * 1_000_000.0).round() / 1_000_000.0
}
