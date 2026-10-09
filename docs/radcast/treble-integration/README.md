# RADsuite 0.2.11 — local Treble integration

Select **Studio — Treble (recommended)** in RADcast. This separately stored `studio_treble` preset uses the accepted epoch119 Treble model with adaptive faint-speech protection. Existing settings retain their previous preset.

The local model lives in `~/.radcast/models/treble-ism-eng120-epoch119/`, with its MIT licence, third-party notices and download provenance. The app embeds its processing helpers; it does not read the experiment folders at runtime or download models. Override the model with `RADSUITE_STUDIO_TREBLE_MODEL` and Python with `RADSUITE_STUDIO_TREBLE_PYTHON`. The local pinned Python runtime is `~/.radcast/venv311/bin/python`.

Ordinary speech is unblended. Quiet passages can receive up to 20% highpassed original, with an energy guard. Presence and leveling calibrate from each recording's conservative source-derived reference, then reuse those gains in the adaptive branch. Preservation checks compare the delivered master with the original and calibrated reference. Failure produces conservative source cleanup and records a warning. Final exports keep a QA report alongside the audio.

The full Finnegan production render reproduced the accepted adaptive candidate exactly (6,887,520 samples, 48 kHz). See `reproduction.json` and `CRJU160_Treble_app.qa.json`. This confirms integration fidelity, not parity with Adobe or approval of every future recording. Listen for natural articulation and quiet words.

This is a local build, with updater artifacts disabled only for that build invocation. No public release or repository commit was made.

Installed in `/Applications/RADsuite.app`, with the original 0.2.10 bundle backed up under `~/Library/Application Support/RADsuite/local-app-backups/`. The local app is signed ad hoc and verified.
