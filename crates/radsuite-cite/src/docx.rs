use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use quick_xml::{Reader, events::Event};
use radsuite_core::{Citation, Document, DocumentFileType, Paragraph, ProjectId, ReadingCategory};
use regex::Regex;
use thiserror::Error;
use zip::{ZipArchive, result::ZipError};

use crate::CitationAnalyzer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocxIngestionRequest {
    pub project_id: ProjectId,
    pub path: PathBuf,
    pub original_filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysedDocument {
    pub document: Document,
    pub paragraphs: Vec<Paragraph>,
    pub citations: Vec<Citation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocxReadingExtractionRequest {
    pub path: PathBuf,
    pub original_filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingImportCandidate {
    pub source_path: Option<String>,
    pub source_filename: Option<String>,
    pub module_order: Option<i32>,
    pub module_title: Option<String>,
    pub reading_category: ReadingCategory,
    pub lesson_code: Option<String>,
    pub apa_citation: String,
    pub citation_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Error)]
pub enum DocxIngestionError {
    #[error("expected a .docx file: {path}")]
    UnsupportedExtension { path: PathBuf },
    #[error("failed to read DOCX file")]
    Io(#[from] std::io::Error),
    #[error("failed to read DOCX package")]
    Zip(#[from] ZipError),
    #[error("DOCX package is missing word/document.xml")]
    MissingDocumentXml,
    #[error("failed to parse DOCX XML")]
    Xml(#[from] quick_xml::Error),
}

pub fn ingest_docx(request: DocxIngestionRequest) -> Result<AnalysedDocument, DocxIngestionError> {
    let extracted_paragraphs = extract_docx_plain_paragraphs(&request.path)?;
    Ok(analyse_extracted_paragraphs(
        request.project_id,
        request.original_filename,
        DocumentFileType::Docx,
        extracted_paragraphs,
    ))
}

pub(crate) fn analyse_extracted_paragraphs(
    project_id: ProjectId,
    original_filename: impl Into<String>,
    file_type: DocumentFileType,
    extracted_paragraphs: impl IntoIterator<Item = ExtractedParagraph>,
) -> AnalysedDocument {
    let document = Document::new(project_id, original_filename, file_type);
    let analyzer = CitationAnalyzer;
    let mut paragraphs = Vec::new();
    let mut citations = Vec::new();

    for (order_index, extracted) in extracted_paragraphs.into_iter().enumerate() {
        let mut paragraph = Paragraph::new(document.id, order_index as i32, extracted.text);
        paragraph.is_table = extracted.is_table;

        let analysis = analyzer.analyse_paragraph(&paragraph.text);
        paragraph.needs_citation = analysis.needs_citation;

        citations.extend(analysis.citations.into_iter().map(|citation| {
            Citation::new(
                paragraph.id,
                citation.text,
                citation.start as i32,
                citation.end as i32,
            )
        }));
        paragraphs.push(paragraph);
    }

    AnalysedDocument {
        document,
        paragraphs,
        citations,
    }
}

pub fn extract_docx_reading_candidates(
    request: DocxReadingExtractionRequest,
) -> Result<Vec<ReadingImportCandidate>, DocxIngestionError> {
    let extracted_paragraphs = extract_docx_plain_paragraphs(&request.path)?;
    Ok(extract_reading_candidates_from_paragraphs(
        extracted_paragraphs
            .into_iter()
            .map(|paragraph| paragraph.text),
    ))
}

fn extract_docx_plain_paragraphs(
    path: &Path,
) -> Result<Vec<ExtractedParagraph>, DocxIngestionError> {
    if !has_docx_extension(path) {
        return Err(DocxIngestionError::UnsupportedExtension {
            path: path.to_path_buf(),
        });
    }

    let mut archive = ZipArchive::new(File::open(path)?)?;
    let document_xml = read_required_zip_file(&mut archive, "word/document.xml")?;
    let relationships_xml = read_optional_zip_file(&mut archive, "word/_rels/document.xml.rels")?;
    let relationships = relationships_xml
        .as_deref()
        .map(parse_relationships)
        .transpose()?
        .unwrap_or_default();

    Ok(extract_paragraphs(&document_xml, &relationships)?)
}

pub(crate) fn extract_reading_candidates_from_paragraphs(
    paragraphs: impl IntoIterator<Item = String>,
) -> Vec<ReadingImportCandidate> {
    let mut current_category = ReadingCategory::Compulsory;
    let mut current_module_order = None;
    let mut current_module_title = None;
    let mut ignoring_bibliography = false;
    let mut in_reading_section = false;
    let mut seen = Vec::new();
    let mut candidates: Vec<ReadingImportCandidate> = Vec::new();

    for paragraph in paragraphs {
        let plain = normalize_plain_text(&paragraph);
        if plain.is_empty() {
            continue;
        }

        if let Some(category) = detect_reading_category(&plain) {
            current_category = category;
            ignoring_bibliography = false;
            in_reading_section = true;
            continue;
        }

        if let Some(category) = detect_generic_reading_heading(&plain) {
            current_category = category;
            ignoring_bibliography = false;
            in_reading_section = true;
            continue;
        }

        if is_bibliography_heading(&plain) {
            ignoring_bibliography = true;
            in_reading_section = false;
            continue;
        }

        let (lesson_code, body) = split_lesson_prefix(&plain);
        let reference_text = body.unwrap_or(&plain);

        if body.is_none()
            && let Some((order, title)) = detect_module_heading(&plain)
        {
            current_module_order = Some(order);
            current_module_title = Some(title);
            continue;
        }

        if ignoring_bibliography {
            continue;
        }

        let url = extract_first_url(reference_text).or_else(|| extract_first_url(&plain));
        let doi = extract_doi(reference_text).or_else(|| url.as_deref().and_then(extract_doi));
        if !(looks_like_reference(reference_text)
            || in_reading_section
                && looks_like_standalone_url_reference(reference_text, url.as_deref()))
        {
            continue;
        }

        let apa_citation = clean_reference_plain_text(reference_text);
        if apa_citation.is_empty() {
            continue;
        }

        let dedupe_key = reading_candidate_dedupe_key(
            current_module_order,
            current_module_title.as_deref(),
            &apa_citation,
            doi.as_deref(),
            url.as_deref(),
        );
        let candidate = ReadingImportCandidate {
            source_path: None,
            source_filename: None,
            module_order: current_module_order,
            module_title: current_module_title.clone(),
            reading_category: current_category,
            lesson_code,
            apa_citation,
            citation_text: (body.is_some()).then_some(plain.clone()),
            doi,
            url,
        };

        if let Some(index) = seen.iter().position(|key| key == &dedupe_key) {
            if should_prefer_reading_candidate(
                candidate.reading_category,
                candidates[index].reading_category,
            ) {
                candidates[index] = candidate;
            }
            continue;
        }
        seen.push(dedupe_key);
        candidates.push(candidate);
    }

    candidates
}

fn normalize_plain_text(text: &str) -> String {
    normalize_whitespace(text)
        .trim_matches(|character: char| character == '•' || character == '*' || character == '-')
        .trim()
        .to_string()
}

fn detect_reading_category(text: &str) -> Option<ReadingCategory> {
    let lowered = text.to_lowercase();
    if lowered.contains("compulsory reading") || lowered.contains("required reading") {
        return Some(ReadingCategory::Compulsory);
    }
    if lowered.contains("optional reading") || lowered.contains("recommended reading") {
        return Some(ReadingCategory::Optional);
    }
    None
}

fn detect_generic_reading_heading(text: &str) -> Option<ReadingCategory> {
    let normalized = text.trim().trim_end_matches(':').to_lowercase();
    match normalized.as_str() {
        "reading" | "readings" | "reading list" | "reading lists" | "course readings"
        | "module readings" | "essential reading" | "essential readings" => {
            Some(ReadingCategory::Compulsory)
        }
        "further reading" | "further readings" => Some(ReadingCategory::Optional),
        _ => None,
    }
}

fn is_bibliography_heading(text: &str) -> bool {
    let normalized = text.trim().trim_end_matches(':').to_lowercase();
    matches!(
        normalized.as_str(),
        "references" | "reference list" | "bibliography"
    )
}

fn detect_module_heading(text: &str) -> Option<(i32, String)> {
    let heading = Regex::new(r"(?i)^\s*(module|week)\s+(\d{1,2})(?:[\s:–—-]+(.+))?\s*$")
        .expect("module heading regex");
    let captures = heading.captures(text)?;
    let order = captures.get(2)?.as_str().parse().ok()?;
    Some((order, text.to_string()))
}

fn looks_like_reference(text: &str) -> bool {
    let normalized = text
        .trim_start_matches(|character: char| {
            character.is_ascii_digit()
                || character == '.'
                || character == '-'
                || character == '–'
                || character == '—'
                || character.is_whitespace()
        })
        .trim_start_matches(['•', '*', '-', '–', '—', ' ']);

    if detect_reading_category(text).is_some() || detect_reading_category(normalized).is_some() {
        return false;
    }
    if normalized.len() < 20 {
        return false;
    }

    let date_marker =
        Regex::new(r"(?i)\((?:(?:19|20)\d{2}[a-z]?|n\.d\.?)\)|\b(?:19|20)\d{2}[a-z]?\b")
            .expect("date marker regex");
    if !date_marker.is_match(normalized) {
        return false;
    }

    let author_with_initial =
        Regex::new(r"^[A-Z][A-Za-z'’`.\- ]{1,80},\s*[A-Z]").expect("author regex");
    let author_with_year =
        Regex::new(r"^[A-Z][A-Za-z&'’`\- ]{1,80}\s*\(\s*\d{4}\s*\)").expect("author-year regex");

    author_with_initial.is_match(normalized) || author_with_year.is_match(normalized)
}

fn looks_like_standalone_url_reference(text: &str, url: Option<&str>) -> bool {
    let Some(url) = url else {
        return false;
    };

    let normalized = text
        .trim_start_matches(|character: char| {
            character == '•'
                || character == '*'
                || character == '-'
                || character == '–'
                || character == '—'
                || character.is_whitespace()
        })
        .trim_end_matches(['.', ',', ')', ';'])
        .trim();

    normalized == url
}

pub(crate) fn reading_candidate_identity(candidate: &ReadingImportCandidate) -> String {
    normalised_reading_identity(
        &candidate.apa_citation,
        candidate.doi.as_deref(),
        candidate.url.as_deref(),
    )
}

fn reading_candidate_dedupe_key(
    module_order: Option<i32>,
    module_title: Option<&str>,
    apa_citation: &str,
    doi: Option<&str>,
    url: Option<&str>,
) -> (Option<i32>, String, String) {
    (
        module_order,
        module_title
            .map(|title| title.trim().to_lowercase())
            .unwrap_or_default(),
        normalised_reading_identity(apa_citation, doi, url),
    )
}

fn normalised_reading_identity(apa_citation: &str, doi: Option<&str>, url: Option<&str>) -> String {
    if let Some(doi) = doi.map(canonical_doi).filter(|doi| !doi.is_empty()) {
        return format!("doi:{doi}");
    }
    let citation = normalize_whitespace(apa_citation).to_lowercase();
    if !citation.is_empty() {
        return citation;
    }
    if let Some(url) = url.map(canonical_url).filter(|url| !url.is_empty()) {
        return format!("url:{url}");
    }
    String::new()
}

fn canonical_doi(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(['.', ',', ';', ')', ']'])
        .to_lowercase()
        .trim_start_matches("https://doi.org/")
        .trim_start_matches("http://doi.org/")
        .trim_start_matches("doi:")
        .to_string()
}

fn canonical_url(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(['.', ',', ';', ')', ']'])
        .to_lowercase()
        .trim_end_matches('/')
        .to_string()
}

fn should_prefer_reading_candidate(incoming: ReadingCategory, existing: ReadingCategory) -> bool {
    matches!(
        (incoming, existing),
        (ReadingCategory::Compulsory, ReadingCategory::Optional)
    )
}

fn split_lesson_prefix(text: &str) -> (Option<String>, Option<&str>) {
    let composite = Regex::new(
        r"(?i)^\s*module\s+(\d+)\s+(?:topic|lesson)\s+(\d+)(?:\s+lesson\s+(\d+))?[\s\-–—:]+(.+)$",
    )
    .expect("composite lesson prefix regex");
    if let Some(captures) = composite.captures(text) {
        let mut lesson = vec![
            captures
                .get(1)
                .map(|part| part.as_str())
                .unwrap_or_default(),
            captures
                .get(2)
                .map(|part| part.as_str())
                .unwrap_or_default(),
        ];
        if let Some(third) = captures.get(3) {
            lesson.push(third.as_str());
        }
        return (
            Some(lesson.join(".")),
            captures.get(4).map(|body| body.as_str().trim()),
        );
    }

    let prefix =
        Regex::new(r"^\s*(\d+(?:\.\d+){1,3})[\s\-–—:]+(.+)$").expect("lesson prefix regex");
    if let Some(captures) = prefix.captures(text) {
        return (
            captures.get(1).map(|lesson| lesson.as_str().to_string()),
            captures.get(2).map(|body| body.as_str().trim()),
        );
    }

    (None, None)
}

fn clean_reference_plain_text(text: &str) -> String {
    let cleaned = text.trim_start_matches(['•', '*', '-', '–', '—', ' ']);
    Regex::new(r"(?i)(?:Lesson\s+\d|Reflection activity|Practice activity).*")
        .expect("trailing activity regex")
        .replace(cleaned, "")
        .trim()
        .to_string()
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

pub(crate) fn extract_doi(text: &str) -> Option<String> {
    let doi =
        Regex::new(r#"(?i)(?:https?://(?:dx\.)?doi\.org/|doi:\s*|\b)(10\.\d{4,9}/[^\s<>"\]]+)"#)
            .expect("doi regex");

    doi.captures(text).and_then(|captures| {
        captures.get(1).map(|matched| {
            matched
                .as_str()
                .trim_end_matches(|character| {
                    matches!(
                        character,
                        '.' | ',' | ';' | ':' | ')' | ']' | '}' | '"' | '\''
                    )
                })
                .to_string()
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtractedParagraph {
    pub(crate) text: String,
    pub(crate) is_table: bool,
}

fn has_docx_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("docx"))
}

fn read_required_zip_file(
    archive: &mut ZipArchive<File>,
    path: &str,
) -> Result<String, DocxIngestionError> {
    match read_zip_file(archive, path) {
        Ok(content) => Ok(content),
        Err(ZipError::FileNotFound) => Err(DocxIngestionError::MissingDocumentXml),
        Err(error) => Err(error.into()),
    }
}

fn read_optional_zip_file(
    archive: &mut ZipArchive<File>,
    path: &str,
) -> Result<Option<String>, DocxIngestionError> {
    match read_zip_file(archive, path) {
        Ok(content) => Ok(Some(content)),
        Err(ZipError::FileNotFound) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn read_zip_file(archive: &mut ZipArchive<File>, path: &str) -> Result<String, ZipError> {
    let mut file = archive.by_name(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn parse_relationships(xml: &str) -> Result<HashMap<String, String>, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut relationships = HashMap::new();

    loop {
        match reader.read_event()? {
            Event::Start(element) | Event::Empty(element)
                if local_name(element.name().as_ref()) == b"Relationship" =>
            {
                let mut id = None;
                let mut target = None;

                for attribute in element.attributes().with_checks(false).flatten() {
                    match local_name(attribute.key.as_ref()) {
                        b"Id" => id = Some(attribute_value(attribute.value.as_ref())),
                        b"Target" => target = Some(attribute_value(attribute.value.as_ref())),
                        _ => {}
                    }
                }

                if let (Some(id), Some(target)) = (id, target) {
                    relationships.insert(id, target);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(relationships)
}

fn extract_paragraphs(
    xml: &str,
    relationships: &HashMap<String, String>,
) -> Result<Vec<ExtractedParagraph>, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut paragraphs = Vec::new();
    let mut active_paragraph: Option<ParagraphBuilder> = None;
    let mut table_builder: Option<ParagraphBuilder> = None;
    let mut table_depth = 0usize;
    let mut paragraph_depth = 0usize;
    let mut hyperlink_target: Option<String> = None;

    loop {
        match reader.read_event()? {
            Event::Start(element) => match local_name(element.name().as_ref()) {
                b"tbl" => {
                    table_depth += 1;
                    if table_builder.is_none() {
                        table_builder = Some(ParagraphBuilder::new(true));
                    }
                }
                b"p" => {
                    paragraph_depth += 1;
                    if table_depth == 0 {
                        active_paragraph = Some(ParagraphBuilder::new(false));
                    }
                }
                b"hyperlink" => {
                    hyperlink_target = relationship_target(element.attributes(), relationships);
                }
                _ => {}
            },
            Event::Empty(element) => match local_name(element.name().as_ref()) {
                b"tab" => append_text(
                    active_builder(table_depth, &mut active_paragraph, &mut table_builder),
                    " ",
                ),
                b"br" => append_text(
                    active_builder(table_depth, &mut active_paragraph, &mut table_builder),
                    " ",
                ),
                _ => {}
            },
            Event::Text(text) => {
                let text = text.decode()?;
                append_text(
                    active_builder(table_depth, &mut active_paragraph, &mut table_builder),
                    text.as_ref(),
                );
            }
            Event::CData(text) => {
                let text = text.decode()?;
                append_text(
                    active_builder(table_depth, &mut active_paragraph, &mut table_builder),
                    text.as_ref(),
                );
            }
            Event::GeneralRef(reference) => {
                let text = decode_general_ref(&reference)?;
                append_text(
                    active_builder(table_depth, &mut active_paragraph, &mut table_builder),
                    &text,
                );
            }
            Event::End(element) => match local_name(element.name().as_ref()) {
                b"hyperlink" => {
                    if let Some(target) = hyperlink_target.take() {
                        append_hyperlink(
                            active_builder(table_depth, &mut active_paragraph, &mut table_builder),
                            &target,
                        );
                    }
                }
                b"p" => {
                    paragraph_depth = paragraph_depth.saturating_sub(1);
                    if table_depth == 0 {
                        if let Some(paragraph) =
                            active_paragraph.take().and_then(|builder| builder.finish())
                        {
                            push_extracted_paragraph(&mut paragraphs, paragraph);
                        }
                    } else if paragraph_depth == 0 {
                        append_text(table_builder.as_mut(), " ");
                    }
                }
                b"tbl" => {
                    table_depth = table_depth.saturating_sub(1);
                    if table_depth == 0
                        && let Some(paragraph) =
                            table_builder.take().and_then(|builder| builder.finish())
                    {
                        push_extracted_paragraph(&mut paragraphs, paragraph);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(paragraphs)
}

fn push_extracted_paragraph(
    paragraphs: &mut Vec<ExtractedParagraph>,
    paragraph: ExtractedParagraph,
) {
    let url = extract_first_url(&paragraph.text);
    if !paragraph.is_table
        && looks_like_standalone_url_reference(&paragraph.text, url.as_deref())
        && let Some(previous) = paragraphs.last_mut()
        && !previous.is_table
        && looks_like_reference(&previous.text)
    {
        previous.text = normalize_whitespace(&format!("{} {}", previous.text, paragraph.text));
        return;
    }

    paragraphs.push(paragraph);
}

fn active_builder<'a>(
    table_depth: usize,
    active_paragraph: &'a mut Option<ParagraphBuilder>,
    table_builder: &'a mut Option<ParagraphBuilder>,
) -> Option<&'a mut ParagraphBuilder> {
    if table_depth > 0 {
        table_builder.as_mut()
    } else {
        active_paragraph.as_mut()
    }
}

fn relationship_target<'a>(
    mut attributes: quick_xml::events::attributes::Attributes<'a>,
    relationships: &HashMap<String, String>,
) -> Option<String> {
    attributes
        .with_checks(false)
        .flatten()
        .find_map(|attribute| {
            let key = local_name(attribute.key.as_ref());
            if key != b"id" {
                return None;
            }

            let id = attribute_value(attribute.value.as_ref());
            relationships.get(&id).cloned()
        })
}

fn append_text(builder: Option<&mut ParagraphBuilder>, text: &str) {
    if let Some(builder) = builder {
        builder.push_text(text);
    }
}

fn append_hyperlink(builder: Option<&mut ParagraphBuilder>, target: &str) {
    if let Some(builder) = builder {
        builder.push_hyperlink(target);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParagraphBuilder {
    is_table: bool,
    text: String,
}

impl ParagraphBuilder {
    fn new(is_table: bool) -> Self {
        Self {
            is_table,
            text: String::new(),
        }
    }

    fn push_text(&mut self, text: &str) {
        self.text.push_str(text);
    }

    fn push_hyperlink(&mut self, target: &str) {
        if !self.text.split_whitespace().any(|word| word == target) {
            self.text.push(' ');
            self.text.push_str(target);
        }
    }

    fn finish(self) -> Option<ExtractedParagraph> {
        let text = normalize_whitespace(&self.text);
        (!text.is_empty()).then_some(ExtractedParagraph {
            text,
            is_table: self.is_table,
        })
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn local_name(name: &[u8]) -> &[u8] {
    name.split(|byte| *byte == b':').next_back().unwrap_or(name)
}

fn attribute_value(value: &[u8]) -> String {
    String::from_utf8_lossy(value).into_owned()
}

fn decode_general_ref(
    reference: &quick_xml::events::BytesRef<'_>,
) -> Result<String, quick_xml::Error> {
    if let Some(character) = reference.resolve_char_ref()? {
        return Ok(character.to_string());
    }

    let name = reference.decode()?;
    Ok(match name.as_ref() {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "apos" => "'".to_string(),
        "quot" => "\"".to_string(),
        other => format!("&{other};"),
    })
}
