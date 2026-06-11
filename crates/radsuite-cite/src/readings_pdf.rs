use std::{
    fs,
    path::{Path, PathBuf},
};

use radsuite_core::ReadingCategory;
use regex::Regex;
use thiserror::Error;

use crate::{ReadingImportCandidate, docx::extract_reading_candidates_from_paragraphs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfReadingExtractionRequest {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Error)]
pub enum PdfReadingExtractionError {
    #[error("choose at least one PDF file before previewing module readings")]
    EmptyPaths,
    #[error("expected a .pdf file: {path}")]
    UnsupportedExtension { path: PathBuf },
    #[error("failed to read PDF file")]
    Io(#[from] std::io::Error),
}

pub fn extract_pdf_reading_candidates(
    request: PdfReadingExtractionRequest,
) -> Result<Vec<ReadingImportCandidate>, PdfReadingExtractionError> {
    let mut all_candidates = Vec::new();

    if request.paths.is_empty() {
        return Err(PdfReadingExtractionError::EmptyPaths);
    }

    for path in request.paths {
        if !has_pdf_extension(&path) {
            return Err(PdfReadingExtractionError::UnsupportedExtension { path });
        }

        let source = infer_pdf_source_context(&path);
        let paragraphs = extract_pdf_text_lines(&path)?;
        let mut candidates = extract_reading_candidates_from_paragraphs(paragraphs);

        for candidate in &mut candidates {
            if candidate.module_order.is_none() {
                candidate.module_order = source.module_order;
            }
            if candidate.module_title.is_none() {
                candidate.module_title.clone_from(&source.module_title);
            }
            if candidate.lesson_code.is_none() {
                candidate.lesson_code.clone_from(&source.lesson_code);
            }
        }

        merge_candidates(&mut all_candidates, candidates);
    }

    Ok(all_candidates)
}

fn extract_pdf_text_lines(path: &Path) -> Result<Vec<String>, PdfReadingExtractionError> {
    let bytes = fs::read(path)?;
    let content = String::from_utf8_lossy(&bytes);
    let literal_string = Regex::new(r"(?s)\(((?:\\.|[^\\)])*)\)\s*Tj").expect("pdf text regex");
    let mut lines = Vec::new();

    for captures in literal_string.captures_iter(&content) {
        if let Some(text) = captures.get(1) {
            let line = unescape_pdf_literal(text.as_str());
            if !line.trim().is_empty() {
                lines.push(line);
            }
        }
    }

    if lines.is_empty() {
        let fallback = normalize_binary_text(&bytes);
        lines.extend(
            fallback
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string),
        );
    }

    Ok(lines)
}

fn unescape_pdf_literal(text: &str) -> String {
    let mut output = String::new();
    let mut escaped = false;

    for character in text.chars() {
        if escaped {
            output.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'b' => '\u{0008}',
                'f' => '\u{000C}',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }

    output
}

fn normalize_binary_text(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| {
            let character = *byte as char;
            if character.is_ascii_graphic() || character.is_ascii_whitespace() {
                character
            } else {
                '\n'
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PdfSourceContext {
    module_order: Option<i32>,
    module_title: Option<String>,
    lesson_code: Option<String>,
}

fn infer_pdf_source_context(path: &Path) -> PdfSourceContext {
    let filename = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let parent = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let source = format!("{parent} {filename}");

    let module = Regex::new(r"(?i)\b(module|week)[\s_-]+(\d{1,2})\b").expect("module source regex");
    let module_match = module.captures(&source);
    let module_order = module_match
        .as_ref()
        .and_then(|captures| captures.get(2))
        .and_then(|value| value.as_str().parse::<i32>().ok());
    let module_title = module_match.as_ref().and_then(|captures| {
        let kind = captures.get(1)?.as_str();
        let order = captures.get(2)?.as_str();
        Some(format!("{kind} {order}"))
    });

    let lesson = Regex::new(r"(?i)\b(microlearning|lesson|topic)[\s_-]+(\d{1,2})\b")
        .expect("lesson source regex");
    let lesson_code = lesson.captures(&source).and_then(|captures| {
        let kind = captures.get(1)?.as_str();
        let order = captures.get(2)?.as_str();
        Some(format!("{kind} {order}"))
    });

    PdfSourceContext {
        module_order,
        module_title,
        lesson_code,
    }
}

fn merge_candidates(
    existing: &mut Vec<ReadingImportCandidate>,
    incoming: Vec<ReadingImportCandidate>,
) {
    for candidate in incoming {
        if let Some(index) = existing
            .iter()
            .position(|current| same_reading_candidate(current, &candidate))
        {
            if candidate.reading_category == ReadingCategory::Compulsory
                && existing[index].reading_category == ReadingCategory::Optional
            {
                existing[index] = candidate;
            }
            continue;
        }
        existing.push(candidate);
    }
}

fn same_reading_candidate(left: &ReadingImportCandidate, right: &ReadingImportCandidate) -> bool {
    left.module_order == right.module_order
        && normalize_key(left.module_title.as_deref())
            == normalize_key(right.module_title.as_deref())
        && normalize_key(Some(&left.apa_citation)) == normalize_key(Some(&right.apa_citation))
}

fn normalize_key(value: Option<&str>) -> String {
    value
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn has_pdf_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}
