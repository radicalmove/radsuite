use radsuite_desktop::{AppPaths, DesktopState, get_app_status};

#[test]
fn app_paths_resolve_platform_data_directory_for_radsuite() {
    let paths = AppPaths::for_app("RADsuite").expect("resolve app paths");
    let data_dir = paths.data_dir.to_string_lossy();

    assert!(paths.data_dir.is_absolute());
    assert!(data_dir.to_lowercase().contains("radsuite"));
}

#[test]
fn app_status_exposes_database_sync_and_engine_state() {
    let state = DesktopState::for_tests();
    let status = get_app_status(&state);

    assert_eq!(status.app_name, "RADsuite");
    assert!(status.database_ready);
    assert!(!status.sync_configured);
    assert_eq!(status.engines.len(), 4);
}
