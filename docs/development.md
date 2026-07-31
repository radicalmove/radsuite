# RADsuite Development

## Local Checks

Run from the repository root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Real-Course Smoke Test

The RADcite desktop crate includes a local smoke test for representative files from
`course-output-system`. It is skipped unless `RADSUITE_REAL_COURSE_ROOT` is set, so
CI does not need access to private course materials.

```bash
RADSUITE_REAL_COURSE_ROOT=/Users/rcd58/course-output-system \
  cargo test -p radsuite-desktop --test real_course_smoke -- --nocapture
```

The smoke test covers project creation, DOCX review analysis, CRJU201
course_readings.csv preview/import, multi-file SCORM-style learner-reader PDF
preview, review report export, module readings export, course references export,
and project-scoped saved review lists.

RADcast also has a local smoke test for a real audio file. It is skipped unless
`RADSUITE_REAL_RADCAST_AUDIO` is set, so CI does not need access to course audio.

```bash
RADSUITE_REAL_RADCAST_AUDIO="/Users/rcd58/course-output-system/Courses/CRJU150/Cleaned up lecture audio/module8_snippets/m8_01_manaakitanga_kaitiakitanga.mp3" \
  cargo test -p radsuite-desktop --test radcast_contracts radcast_real_audio_fixture_can_process_when_available -- --nocapture
```

## Scope

This repository is the new Rust/Tauri implementation of RADsuite. The existing Python apps remain reference implementations only.
