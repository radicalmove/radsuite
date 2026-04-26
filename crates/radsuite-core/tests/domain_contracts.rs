use radsuite_core::{ApiProjectSummary, Project, ProjectRole, UserId};

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
