use std::path::PathBuf;

use radsuite_engines::EngineRegistry;

#[test]
fn registry_reports_available_engines_with_actionable_details() {
    let executable = std::env::current_exe().expect("test executable");
    let engines = EngineRegistry::from_commands(
        Some(executable.clone()),
        Some(executable.clone()),
        Some(executable.clone()),
        Some(executable),
    )
    .list();
    let ids = engines
        .iter()
        .map(|engine| engine.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, ["ffmpeg", "asr", "audio_cleanup", "tts"]);
    assert!(engines.iter().all(|engine| engine.available));
    assert!(
        engines
            .iter()
            .all(|engine| engine.detail.contains("Available"))
    );
    assert!(
        engines
            .iter()
            .all(|engine| !engine.detail.contains("not implemented"))
    );
}

#[test]
fn registry_explains_missing_engines_without_calling_them_broken() {
    let missing = PathBuf::from("/definitely/missing/radsuite-engine");
    let engines = EngineRegistry::from_commands(None, None, Some(missing.clone()), None).list();

    assert!(engines.iter().all(|engine| !engine.available));
    assert!(
        engines
            .iter()
            .all(|engine| engine.detail.contains("Install") || engine.detail.contains("set "))
    );
}
