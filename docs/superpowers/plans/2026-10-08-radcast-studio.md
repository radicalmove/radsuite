# RADcast Studio experiment implementation plan

Goal: implement an opt-in studio_v1 experiment and stop at human listening approval, with existing defaults and presets preserved.
Architecture: source decode to 48 kHz float PCM once, cached DeepFilterNet3 CPU inference with compensated delay, source-preserving upper-band blend, bounded pause attenuation, optional light compression, final measured linear loudness gain and true-peak guard. No generative restoration, fixed low-pass, bass boost or mandatory de-essing. Share analysis and watchdog code with the repeatable calibration CLI.

User's explicit instructions supersede workflow commit/worktree/design-approval gates: work here, continue autonomously, no commit or push, listening approval before promotion.

- [ ] Add deterministic Python analysis/watchdog tests; verify failure, then implement native-rate metadata/loudness/spectra, source-derived frame masks, lag and block alignment, JSON and text reports.
- [ ] Analyze all four supplied files and record hashes. Treat edited duration/reference alignment as a limitation, not enhancer drift.
- [ ] Benchmark cached full-band DeepFilterNet3 with float PCM and no runtime model download; record versions, hashes, CPU performance/license.
- [ ] Implement float Studio renderer, bounded source preservation, final mastering and watchdog. Reject destructive results; transparently fall back to conservative source cleanup with QA report.
- [ ] Add Rust preset/serialization/format/ordering tests; run red, integrate embedded Python helper with explicit interpreter/model paths and retained QA sidecar; preserve legacy arguments and defaults.
- [ ] Add UI opt-in Studio Recommended, dependency checks and QA link, retaining saved defaults and IDs; verify frontend.
- [ ] Render <=2 meaningful candidates from original; analyze and make <=3 evidence-justified repairs.
- [ ] Run Python/Rust/frontend regression tests, inspect diff, write complete experimental report and stop for listening approval.
