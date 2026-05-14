use radsuite_core::{
    ApaValidationStatus, ApiProjectSummary, Citation, CourseModule, Document, DocumentFileType,
    DocumentVariant, Paragraph, Project, ProjectId, ProjectRole, ReadingCategory, ReferenceEntry,
    ReferenceEntryType, UserId,
};

#[test]
fn ids_are_serializable_uuid_wrappers() {
    let id = UserId::new();
    let encoded = serde_json::to_string(&id).expect("serialize id");
    let decoded: UserId = serde_json::from_str(&encoded).expect("deserialize id");
    assert_eq!(id, decoded);
}

#[test]
fn project_owner_can_be_returned_as_api_summary() {
    let owner_id = UserId::new();
    let project = Project::new("COMS435", "Good data and how to use it", owner_id);
    let summary = ApiProjectSummary::from_project(&project, ProjectRole::Owner);

    assert_eq!(summary.id, project.id);
    assert_eq!(summary.code.as_deref(), Some("COMS435"));
    assert_eq!(summary.title, "Good data and how to use it");
    assert_eq!(summary.role, ProjectRole::Owner);
}

#[test]
fn radcite_document_contracts_are_serializable() {
    let project_id = ProjectId::new();
    let mut document = Document::new(project_id, "lesson-1.docx", DocumentFileType::Docx);
    document.doc_variant = DocumentVariant::Content;
    document.doc_number = Some(1);

    let paragraph = Paragraph::new(
        document.id,
        0,
        "Research shows that worked examples reduce cognitive load (Smith, 2020).",
    );
    let citation = Citation::new(paragraph.id, "(Smith, 2020)", 58, 71);
    let reference = ReferenceEntry::new(project_id, ReferenceEntryType::Reference);

    assert_eq!(document.project_id, project_id);
    assert_eq!(document.original_filename, "lesson-1.docx");
    assert_eq!(document.file_type, DocumentFileType::Docx);
    assert_eq!(paragraph.order_index, 0);
    assert_eq!(citation.citation_text, "(Smith, 2020)");
    assert_eq!(
        reference.apa_validation_status,
        ApaValidationStatus::Unknown
    );

    let encoded = serde_json::to_string(&document).expect("serialize document");
    let decoded: Document = serde_json::from_str(&encoded).expect("deserialize document");

    assert_eq!(decoded.id, document.id);
    assert_eq!(decoded.project_id, project_id);
    assert_eq!(decoded.doc_variant, DocumentVariant::Content);
}

#[test]
fn course_modules_and_reading_metadata_are_serializable() {
    let project_id = ProjectId::new();
    let module = CourseModule::new(project_id, "Module 1", Some(1));

    assert_eq!(module.project_id, project_id);
    assert_eq!(module.title, "Module 1");
    assert_eq!(module.order_index, Some(1));

    let mut reading = ReferenceEntry::new(project_id, ReferenceEntryType::Reading);
    reading.module_id = Some(module.id);
    reading.reading_category = Some(ReadingCategory::Optional);
    reading.lesson_code = Some("2.3".to_string());
    reading.reading_notes = Some("Read before workshop".to_string());
    reading.estimated_reading_time = Some("20 minutes".to_string());

    let encoded = serde_json::to_string(&reading).expect("serialize reading");
    assert!(encoded.contains("optional"));
    assert!(encoded.contains("2.3"));

    let decoded: ReferenceEntry = serde_json::from_str(&encoded).expect("deserialize reading");

    assert_eq!(decoded.module_id, Some(module.id));
    assert_eq!(decoded.reading_category, Some(ReadingCategory::Optional));
    assert_eq!(decoded.lesson_code.as_deref(), Some("2.3"));
    assert_eq!(
        decoded.reading_notes.as_deref(),
        Some("Read before workshop")
    );
    assert_eq!(
        decoded.estimated_reading_time.as_deref(),
        Some("20 minutes")
    );
}
