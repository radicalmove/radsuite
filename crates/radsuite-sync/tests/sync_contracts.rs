use radsuite_core::ProjectId;
use radsuite_sync::{AssetManifest, AssetSyncPolicy, LocalChange, SyncConflict, SyncOperation};
use serde_json::json;

#[test]
fn local_change_serializes_contract_fields() {
    let change = LocalChange {
        project_id: ProjectId::new(),
        entity_type: "project".to_string(),
        entity_id: "project-1".to_string(),
        operation: SyncOperation::Update,
        payload: json!({ "title": "Updated title" }),
    };

    let value = serde_json::to_value(change).expect("serialize change");

    assert_eq!(value["entity_type"], "project");
    assert_eq!(value["entity_id"], "project-1");
    assert_eq!(value["operation"], "update");
}

#[test]
fn asset_manifest_exposes_hash_size_type_and_policy() {
    let manifest = AssetManifest {
        project_id: ProjectId::new(),
        sha256: "a".repeat(64),
        byte_size: 1024,
        mime_type: "application/pdf".to_string(),
        original_name: "module.pdf".to_string(),
        sync_policy: AssetSyncPolicy::CollaborativeSource,
    };

    assert_eq!(manifest.byte_size, 1024);
    assert_eq!(manifest.mime_type, "application/pdf");
    assert_eq!(manifest.sync_policy, AssetSyncPolicy::CollaborativeSource);
    assert_eq!(manifest.sha256.len(), 64);
}

#[test]
fn conflict_preserves_local_and_remote_payloads() {
    let conflict = SyncConflict {
        project_id: ProjectId::new(),
        entity_type: "script".to_string(),
        entity_id: "script-1".to_string(),
        local_payload: json!({ "body": "local" }),
        remote_payload: json!({ "body": "remote" }),
    };

    assert_eq!(conflict.local_payload["body"], "local");
    assert_eq!(conflict.remote_payload["body"], "remote");
}
