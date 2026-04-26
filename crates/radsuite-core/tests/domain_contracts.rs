use radsuite_core::{
    ApaValidationStatus, ApiProjectSummary, Citation, Document, DocumentFileType, DocumentVariant,
    Paragraph, Project, ProjectId, ProjectRole, ReferenceEntry, ReferenceEntryType, UserId,
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
