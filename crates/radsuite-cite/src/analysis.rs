use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedCitation {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub full_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CitationAnalysis {
    pub citations: Vec<DetectedCitation>,
    pub needs_citation: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CitationAnalyzer;

impl CitationAnalyzer {
    pub fn analyse_paragraph(&self, text: &str) -> CitationAnalysis {
        let citations = self.detect_citations(text);
        let needs_citation = self.needs_citation(text, &citations);

        CitationAnalysis {
            citations,
            needs_citation,
        }
    }

    pub fn detect_citations(&self, text: &str) -> Vec<DetectedCitation> {
        let mut citations = Vec::new();
        let mut seen_positions: Vec<(usize, usize)> = Vec::new();

        let parenthetical = Regex::new(r"\([^()]{1,500}?(?:19|20)\d{2}[^()]{0,500}?\)").unwrap();
        let author = Regex::new(r"[A-Z][a-z]+").unwrap();
        let year = Regex::new(r"(?:19|20)\d{2}").unwrap();

        for hit in parenthetical.find_iter(text) {
            let full_text = hit.as_str();
            let start = hit.start();
            let end = hit.end();

            if overlaps(&seen_positions, start, end) {
                continue;
            }

            let inner = full_text.trim_matches(['(', ')']);
            if !year.is_match(inner) || !(inner.contains("et al.") || author.is_match(inner)) {
                continue;
            }

            seen_positions.push((start, end));

            if inner.contains(';') {
                for part in inner.split(';').map(str::trim) {
                    if year.is_match(part) {
                        citations.push(DetectedCitation {
                            text: part.to_string(),
                            start,
                            end,
                            full_text: Some(full_text.to_string()),
                        });
                    }
                }
            } else {
                citations.push(DetectedCitation {
                    text: full_text.to_string(),
                    start,
                    end,
                    full_text: None,
                });
            }
        }

        let narrative = Regex::new(
            r"\b[A-Z][A-Za-z\-']+(?:\s+(?:et\s+al\.|(?:&|and)\s+[A-Z][A-Za-z\-']+(?:\s+et\s+al\.)?))?\s*'?s?\s+\((?:19|20)\d{2}[a-z]?\)",
        )
        .unwrap();
        let title_context = Regex::new(r"(?i)(?:Comment\s+on|Reply\s+to)\s*$").unwrap();

        for hit in narrative.find_iter(text) {
            let start = hit.start();
            let end = hit.end();

            if overlaps(&seen_positions, start, end) {
                continue;
            }

            let context_start = start.saturating_sub(15);
            if title_context.is_match(&text[context_start..start]) {
                continue;
            }

            seen_positions.push((start, end));
            citations.push(DetectedCitation {
                text: hit.as_str().to_string(),
                start,
                end,
                full_text: None,
            });
        }

        let et_al = Regex::new(r"\b[A-Z][A-Za-z]+\s+et\s+al\.\s*'?s?").unwrap();
        for hit in et_al.find_iter(text) {
            let start = hit.start();
            let end = hit.end();

            if overlaps(&seen_positions, start, end) {
                continue;
            }

            seen_positions.push((start, end));
            citations.push(DetectedCitation {
                text: hit.as_str().trim().to_string(),
                start,
                end,
                full_text: None,
            });
        }

        citations.sort_by_key(|citation| citation.start);
        deduplicate_by_text(citations)
    }

    pub fn extract_keywords(&self, text: &str, limit: usize) -> Vec<String> {
        let stop_words = [
            "the",
            "a",
            "an",
            "and",
            "or",
            "but",
            "in",
            "on",
            "at",
            "to",
            "for",
            "of",
            "with",
            "by",
            "from",
            "as",
            "is",
            "was",
            "are",
            "were",
            "been",
            "have",
            "has",
            "had",
            "do",
            "does",
            "did",
            "will",
            "would",
            "could",
            "should",
            "may",
            "might",
            "must",
            "can",
            "this",
            "that",
            "these",
            "those",
            "it",
            "its",
            "they",
            "their",
            "them",
            "we",
            "our",
            "us",
            "he",
            "she",
            "his",
            "her",
            "him",
            "which",
            "what",
            "when",
            "where",
            "who",
            "why",
            "how",
            "all",
            "each",
            "every",
            "some",
            "any",
            "few",
            "more",
            "most",
            "other",
            "such",
            "only",
            "same",
            "so",
            "than",
            "too",
            "very",
            "just",
            "now",
            "also",
            "about",
            "after",
            "before",
            "because",
            "between",
            "both",
            "during",
            "either",
            "however",
            "into",
            "like",
            "many",
            "not",
            "often",
            "once",
            "over",
            "since",
            "then",
            "there",
            "therefore",
            "through",
            "under",
            "until",
            "upon",
            "while",
            "whether",
            "within",
            "without",
        ];
        let token = Regex::new(r"[a-z0-9]+").unwrap();
        let mut counts: Vec<(String, usize)> = Vec::new();

        for hit in token.find_iter(&text.to_lowercase()) {
            let word = hit.as_str();
            if word.len() <= 3 || stop_words.contains(&word) {
                continue;
            }

            if let Some((_, count)) = counts.iter_mut().find(|(known, _)| known == word) {
                *count += 1;
            } else {
                counts.push((word.to_string(), 1));
            }
        }

        counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        counts
            .into_iter()
            .take(limit)
            .map(|(word, _)| word)
            .collect()
    }

    fn needs_citation(&self, text: &str, citations: &[DetectedCitation]) -> bool {
        let text_lower = text.to_lowercase();
        let statistics = [
            r"\b\d+(?:\.\d+)?\s*(?:%|percent|per\s+cent)\b",
            r"\b\d+(?:\.\d+)?\s*(?:million|billion|thousand)\b",
            r"\b\d+\s+out\s+of\s+\d+\b",
        ];
        let quantitative = [
            r"\bn\s*=\s*\d+",
            r"\bsample\s+of\s+\d+",
            r"\bsurvey(?:ed)?\s+\d+",
            r"\brespondents?\s+\d+",
            r"\bp\s*(?:<|=)\s*0\.\d+",
            r"\bconfidence\s+interval",
            r"\bcontrol\s+group",
            r"\btreatment\s+group",
        ];
        let factual = [
            r"[A-Z][a-z]+\s+(?:discovered|invented|developed|created|found|demonstrated)",
            r"study\s+(?:by|from|conducted)",
            r"research\s+(?:by|from|conducted)",
            r"according to (?:the\s+)?(?:study|research|investigation|analysis)",
            r"has been (?:shown|demonstrated|proven|found)",
            r"(?:shows?|demonstrates?|proves?|reveals?|indicates?)\s+that",
        ];
        let claim_indicators = [
            "research shows",
            "studies indicate",
            "according to",
            "evidence suggests",
            "data reveals",
            "analysis found",
            "experiments demonstrate",
            "surveys show",
            "statistics indicate",
            "scholars argue",
            "researchers found",
            "scientists discovered",
            "findings suggest",
            "results indicate",
            "studies have shown",
            "research indicates",
            "empirical evidence",
            "literature suggests",
            "meta-analysis",
            "systematic review",
            "longitudinal study",
            "cross-sectional study",
        ];

        let has_statistics = matches_any(&statistics, &text_lower);
        let has_quantitative = matches_any(&quantitative, &text_lower);
        let has_factual = matches_any(&factual, text);
        let has_claim = claim_indicators
            .iter()
            .any(|indicator| text_lower.contains(indicator));
        let has_comparison = Regex::new(
            r"\b(?:more|less|better|worse|higher|lower|greater|fewer|most|least|best|worst)\s+than\b",
        )
        .unwrap()
        .is_match(&text_lower);
        let research_context = Regex::new(
            r"\b(?:participants?|respondents?|clinical|trial|longitudinal|randomi[sz]ed|experiment|dataset|surveyed)\b",
        )
        .unwrap()
        .is_match(&text_lower);
        let has_uncited_years = self.has_uncited_years(text, citations);

        if !citations.is_empty() {
            let claim_count = [
                has_statistics,
                has_quantitative,
                has_factual,
                has_claim,
                has_comparison,
                research_context,
                has_uncited_years,
            ]
            .into_iter()
            .filter(|matched| *matched)
            .count();

            return has_uncited_years || claim_count > citations.len();
        }

        has_statistics
            || has_quantitative
            || has_factual
            || has_claim
            || has_comparison
            || research_context
            || has_uncited_years
    }

    fn has_uncited_years(&self, text: &str, citations: &[DetectedCitation]) -> bool {
        let year = Regex::new(r"\b((?:19|20)\d{2})\b").unwrap();

        year.captures_iter(text).any(|capture| {
            let Some(found_year) = capture.get(1).map(|hit| hit.as_str()) else {
                return false;
            };

            citations
                .iter()
                .all(|citation| !citation.text.contains(found_year))
        })
    }
}

fn overlaps(seen_positions: &[(usize, usize)], start: usize, end: usize) -> bool {
    seen_positions.iter().any(|(seen_start, seen_end)| {
        (start >= *seen_start && start < *seen_end)
            || (end > *seen_start && end <= *seen_end)
            || (start <= *seen_start && end >= *seen_end)
    })
}

fn deduplicate_by_text(citations: Vec<DetectedCitation>) -> Vec<DetectedCitation> {
    let mut unique = Vec::new();

    for citation in citations {
        if unique
            .iter()
            .any(|known: &DetectedCitation| known.text == citation.text)
        {
            continue;
        }
        unique.push(citation);
    }

    unique
}

fn matches_any(patterns: &[&str], text: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| Regex::new(pattern).unwrap().is_match(text))
}
