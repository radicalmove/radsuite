use radsuite_engines::EngineRegistry;

#[test]
fn registry_reports_expected_engine_slots_as_unavailable_stubs() {
    let engines = EngineRegistry::default().list();
    let ids = engines
        .iter()
        .map(|engine| engine.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, ["ffmpeg", "asr", "audio_cleanup", "tts"]);
    assert!(engines.iter().all(|engine| !engine.available));
    assert!(engines.iter().all(|engine| !engine.detail.is_empty()));
}
