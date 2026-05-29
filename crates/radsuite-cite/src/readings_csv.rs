use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use csv::StringRecord;
use radsuite_core::ReadingCategory;
use regex::Regex;
use thiserror::Error;

use crate::ReadingImportCandidate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvReadingExtractionRequest {
    pub path: PathBuf,
    pub original_filename: String,
}

#[derive(Debug, Error)]
pub enum CsvReadingImportError {
    #[error("expected a .csv file: {path}")]
    UnsupportedExtension { path: PathBuf },
    #[error("failed to parse CSV file")]
    Csv(#[from] csv::Error),
}

pub fn extract_csv_reading_candidates(
    request: CsvReadingExtractionRequest,
) -> Result<Vec<ReadingImportCandidate>, CsvReadingImportError> {
    if !has_csv_extension(&request.path) {
        return Err(CsvReadingImportError::UnsupportedExtension { path: request.path });
    }

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(&request.path)?;
    let headers = CsvHeaderMap::from_headers(reader.headers()?);
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for record in reader.records() {
        let record = record?;
        let apa_citation = normalize_plain_text(field(&record, headers.citation));
        if apa_citation.is_empty() {
            continue;
        }

        let lesson_code = optional_plain_text(field(&record, headers.lesson));
        let module_title = optional_plain_text(field(&record, headers.module_title));
        let url = optional_plain_text(field(&record, headers.url))
            .or_else(|| extract_first_url(&apa_citation));
        let reading_category = field(&record, headers.category)
            .and_then(parse_reading_category)
            .unwrap_or(ReadingCategory::Compulsory);
        let module_order = parse_module_order(lesson_code.as_deref())
            .or_else(|| parse_module_order(field(&record, headers.module_order)));

        let dedupe_key = (
            reading_category_label(reading_category),
            lesson_code.clone().unwrap_or_default(),
            module_title.clone().unwrap_or_default(),
            apa_citation.clone(),
            url.clone().unwrap_or_default(),
        );
        if !seen.insert(dedupe_key) {
            continue;
        }

        candidates.push(ReadingImportCandidate {
            module_order,
            module_title,
            reading_category,
            lesson_code,
            apa_citation,
            citation_text: None,
            url,
        });
    }

    Ok(candidates)
}

#[derive(Debug, Clone, Copy, Default)]
struct CsvHeaderMap {
    citation: Option<usize>,
    lesson: Option<usize>,
    module_order: Option<usize>,
    module_title: Option<usize>,
    category: Option<usize>,
    url: Option<usize>,
}

impl CsvHeaderMap {
    fn from_headers(headers: &StringRecord) -> Self {
        let mut map = Self::default();
        for (index, header) in headers.iter().enumerate() {
            match normalize_header(header).as_str() {
                "citation" | "reading" | "reference" | "apacitation" | "apareference" => {
                    map.citation = map.citation.or(Some(index));
                }
                "week" | "lesson" | "lessoncode" => {
                    map.lesson = map.lesson.or(Some(index));
                }
                "sectionseq" | "moduleorder" | "order" => {
                    map.module_order = map.module_order.or(Some(index));
                }
                "sectiontitle" | "moduletitle" | "module" => {
                    map.module_title = map.module_title.or(Some(index));
                }
                "readingcategory" | "category" => {
                    map.category = map.category.or(Some(index));
                }
                "url" | "link" => {
                    map.url = map.url.or(Some(index));
                }
                _ => {}
            }
        }
        map
    }
}

fn field(record: &StringRecord, index: Option<usize>) -> Option<&str> {
    index.and_then(|index| record.get(index))
}

fn optional_plain_text(text: Option<&str>) -> Option<String> {
    let text = normalize_plain_text(text);
    (!text.is_empty()).then_some(text)
}

fn normalize_plain_text(text: Option<&str>) -> String {
    text.unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn normalize_header(header: &str) -> String {
    header
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_reading_category(value: &str) -> Option<ReadingCategory> {
    let lowered = value.trim().to_lowercase();
    if lowered.contains("optional") || lowered.contains("recommended") {
        return Some(ReadingCategory::Optional);
    }
    if lowered.contains("compulsory") || lowered.contains("required") {
        return Some(ReadingCategory::Compulsory);
    }
    None
}

fn parse_module_order(value: Option<&str>) -> Option<i32> {
    let value = value?.trim();
    let leading_digits = Regex::new(r"^\D*(\d{1,3})").expect("module order regex");
    leading_digits
        .captures(value)
        .and_then(|captures| captures.get(1))
        .and_then(|matched| matched.as_str().parse().ok())
}

fn extract_first_url(text: &str) -> Option<String> {
    let url = Regex::new(r#"https?://[^\s<>"\)\]]+"#).expect("url regex");
    url.find(text).map(|matched| {
        matched
            .as_str()
            .trim_end_matches(['.', ',', ')', ';'])
            .to_string()
    })
}

fn reading_category_label(reading_category: ReadingCategory) -> &'static str {
    match reading_category {
        ReadingCategory::Compulsory => "compulsory",
        ReadingCategory::Optional => "optional",
    }
}

fn has_csv_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
}
