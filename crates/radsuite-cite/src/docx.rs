use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use quick_xml::{Reader, events::Event};
use radsuite_core::{Citation, Document, DocumentFileType, Paragraph, ProjectId};
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
    if !has_docx_extension(&request.path) {
        return Err(DocxIngestionError::UnsupportedExtension { path: request.path });
    }

    let mut archive = ZipArchive::new(File::open(&request.path)?)?;
    let document_xml = read_required_zip_file(&mut archive, "word/document.xml")?;
    let relationships_xml = read_optional_zip_file(&mut archive, "word/_rels/document.xml.rels")?;
    let relationships = relationships_xml
        .as_deref()
        .map(parse_relationships)
        .transpose()?
        .unwrap_or_default();
    let extracted_paragraphs = extract_paragraphs(&document_xml, &relationships)?;

    let document = Document::new(
        request.project_id,
        request.original_filename,
        DocumentFileType::Docx,
    );
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

    Ok(AnalysedDocument {
        document,
        paragraphs,
        citations,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExtractedParagraph {
    text: String,
    is_table: bool,
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
                            paragraphs.push(paragraph);
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
                        paragraphs.push(paragraph);
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
