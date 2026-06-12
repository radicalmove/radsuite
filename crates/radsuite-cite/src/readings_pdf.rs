use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use flate2::read::ZlibDecoder;
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
    let mut lines = extract_pdf_literal_text_lines(&bytes);
    for stream in extract_flate_streams(&bytes) {
        lines.extend(extract_pdf_literal_text_lines(&stream));
    }
    Ok(lines)
}

fn extract_pdf_literal_text_lines(bytes: &[u8]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cursor = 0;
    let mut operators_seen = 0;

    while cursor + 2 <= bytes.len() {
        if operators_seen > 5_000 || lines.len() > 2_000 {
            break;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"TJ") {
            operators_seen += 1;
            if let Some(array) = read_pdf_array_before_operator(bytes, cursor) {
                lines.extend(extract_pdf_literals_from_array(&array));
            }
            cursor += 2;
            continue;
        }

        if bytes.get(cursor..cursor + 2) != Some(b"Tj") {
            cursor += 1;
            continue;
        }
        operators_seen += 1;

        let Some(literal) = read_pdf_literal_before_operator(bytes, cursor) else {
            cursor += 1;
            continue;
        };

        let line = unescape_pdf_literal(&literal);
        if is_plausible_pdf_text_line(&line) {
            lines.push(line);
        }
        cursor += 2;
    }

    lines
}

fn read_pdf_array_before_operator(bytes: &[u8], operator_start: usize) -> Option<Vec<u8>> {
    let mut close = operator_start.checked_sub(1)?;
    while close > 0 && bytes[close].is_ascii_whitespace() {
        close -= 1;
    }
    if bytes[close] != b']' {
        return None;
    }

    let lower_bound = close.saturating_sub(16 * 1024);
    for open in (lower_bound..close).rev() {
        if bytes[open] == b'[' {
            return Some(bytes[open + 1..close].to_vec());
        }
    }

    None
}

fn extract_pdf_literals_from_array(bytes: &[u8]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cursor = 0;

    while cursor < bytes.len() {
        if bytes[cursor] != b'(' {
            cursor += 1;
            continue;
        }
        let Some((literal, after_literal)) = read_pdf_literal(bytes, cursor + 1) else {
            cursor += 1;
            continue;
        };
        let line = unescape_pdf_literal(&literal);
        if is_plausible_pdf_text_line(&line) {
            lines.push(line);
        }
        cursor = after_literal;
    }

    lines
}

fn extract_flate_streams(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut streams = Vec::new();
    let mut cursor = 0;

    while let Some(stream_start_offset) = find_bytes(&bytes[cursor..], b"stream") {
        let stream_keyword_start = cursor + stream_start_offset;
        let header_start = stream_keyword_start.saturating_sub(512);
        let header = &bytes[header_start..stream_keyword_start];
        let mut stream_data_start = stream_keyword_start + b"stream".len();
        if bytes.get(stream_data_start) == Some(&b'\r') {
            stream_data_start += 1;
        }
        if bytes.get(stream_data_start) == Some(&b'\n') {
            stream_data_start += 1;
        }

        let Some(end_offset) = find_bytes(&bytes[stream_data_start..], b"endstream") else {
            break;
        };
        let stream_data_end = stream_data_start + end_offset;

        if header
            .windows(b"FlateDecode".len())
            .any(|part| part == b"FlateDecode")
        {
            let mut decoder = ZlibDecoder::new(&bytes[stream_data_start..stream_data_end]);
            let mut decoded = Vec::new();
            if decoder.read_to_end(&mut decoded).is_ok() && looks_like_pdf_text_stream(&decoded) {
                streams.push(decoded);
            }
        }

        cursor = stream_data_end + b"endstream".len();
    }

    streams
}

fn looks_like_pdf_text_stream(bytes: &[u8]) -> bool {
    bytes.len() <= 2 * 1024 * 1024
        && find_bytes(bytes, b"BT").is_some()
        && (find_bytes(bytes, b"Tj").is_some() || find_bytes(bytes, b"TJ").is_some())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn is_plausible_pdf_text_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.len() > 1_000 {
        return false;
    }
    let char_count = trimmed.chars().count();
    if char_count == 0 {
        return false;
    }
    let printable_count = trimmed
        .chars()
        .filter(|character| {
            character.is_alphanumeric()
                || character.is_ascii_punctuation()
                || character.is_ascii_whitespace()
                || matches!(character, '–' | '—' | '’' | '“' | '”')
        })
        .count();
    let letter_count = trimmed
        .chars()
        .filter(|character| character.is_alphabetic())
        .count();

    printable_count * 100 / char_count >= 80 && letter_count >= 3
}

fn read_pdf_literal_before_operator(bytes: &[u8], operator_start: usize) -> Option<String> {
    let mut close = operator_start.checked_sub(1)?;
    while close > 0 && bytes[close].is_ascii_whitespace() {
        close -= 1;
    }
    if bytes[close] != b')' {
        return None;
    }

    let lower_bound = close.saturating_sub(2048);
    for open in (lower_bound..close).rev() {
        if bytes[open] == b'(' && (open == 0 || bytes[open - 1] != b'\\') {
            return Some(String::from_utf8_lossy(&bytes[open + 1..close]).into_owned());
        }
    }

    None
}

#[allow(dead_code)]
fn read_pdf_literal(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let mut output = Vec::new();
    let mut escaped = false;
    let mut index = start;
    let max_literal_len = 16 * 1024;

    while index < bytes.len() && output.len() < max_literal_len {
        let byte = bytes[index];
        if escaped {
            output.push(b'\\');
            output.push(byte);
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b')' {
            return Some((String::from_utf8_lossy(&output).into_owned(), index + 1));
        } else {
            output.push(byte);
        }
        index += 1;
    }

    None
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
