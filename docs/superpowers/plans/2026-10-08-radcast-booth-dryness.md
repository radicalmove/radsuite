# RADcast booth-dryness implementation plan

> For agentic workers: use the existing Superpowers test-first and verification workflows. Execute in this checkout after planning discussion; request focused review of changes using requesting-code-review. User instructions override workflow defaults: no new worktree, commit, push or default promotion during this experiment.

**Goal:** Reduce the residual lecture-theatre room sound around Finnegan's words so the result approaches the preferred reference's close, dry presentation, retaining the lecturer's identity and natural S/SH/T/CH/F.

**Architecture:** Freeze Studio revision 2's successful noise cleanup, articulation protection, tonal control and speech-leveling configuration. First compare its existing WPE prediction guard with a shorter guard, then run at most one conditional fallback architecture if the existing method cannot reduce the remaining room effect transparently. Use full lossless renders, common-content listening excerpts and preservation QA; stop at human listening approval.

**Tech stack:** Existing 48 kHz float PCM / FFmpeg, pinned DeepFilterNet3, NARA-WPE 0.0.11, NumPy/SciPy, existing analysis/ASR/watchdog and plotting tools. No new dependency/model is assumed or installed by this plan.

## Evidence and limits

The reference's remaining advantage is audible room reduction during speech, not just silence cleanliness. Revision 2 already reduced the matched passage-level difference from 17.61 dB to 4.91 dB (reference 3.31 dB), and improves quiet-word presentation. User feedback now says it is much closer but slightly more echoey.

The specific original-timeline phrase is approximately 23.470–28.380 s: “reading anything to do with the criminal law a lot more simple.” Word timing is approximate local ASR. The reference offset in this stable region is about -2.239 s, so the corresponding reference phrase is approximately 21.231–26.141 s. Prepare broader original 20–32 s / reference 17.761–29.761 s listening excerpts for context; confirm alignment against speech envelopes and words rather than trusting ASR timing alone. No timing adjustment enters the enhancement signal.

A direct check of the current 65% WPE output before DF/mastering found a source/output waveform correlation of .99923 for original 21–29 s; the difference signal RMS is about -28.01 dB relative to the source. This establishes that the current stage changes this passage gently. It does **not** measure reverb removed, direct-to-reverberant ratio or RT60, and does not prove why the residual room sound remains.

Current WPE delay is 4 frames at a 256-sample hop / 48 kHz, nominally 21.33 ms. The first trial is 2 frames, nominally 10.67 ms. A shorter prediction guard may target some earlier correlations, but may also predict/remove periodic direct speech. This is a hypothesis to test, not a claim about Adobe's algorithm or a guaranteed room improvement. Prediction delay controls STFT history; it is not a sharp physical reflection cutoff or an early-reflection detector.

## Approach selection

1. **Recommended first:** shorten the WPE prediction guard while retaining the existing 65% filtered / 35% observed blend and all other WPE settings. This uses installed dependencies and isolates a meaningful architectural variable.
2. **Conditional second:** if WPE does not provide useful audible dryness, test one conservative source-preserving room-suppression method aimed at active speech. Establish that it preserves articulation before integration. Candidate choice must follow analysis of the first trial; do not stack it automatically onto ineffective WPE.
3. **Deferred research:** if both permitted transparent trials fail, stop this experiment and report the limits. Propose a future focused model-evaluation plan covering actual licences, sample rate/bandwidth, dependency size, Apple Silicon performance, offline operation and reproducibility. That is not a third candidate in this round. Generative reconstruction or restoration rescue trials are outside this plan; any future use needs its own explicitly agreed scope.

Do not build an arbitrary EQ/model parameter matrix. Do not infer Adobe's proprietary model or exact processing settings from output waveforms. Do not equate more signal removed, a darker spectrum, quieter gaps or higher source correlation with better sound.

## Task 1 — freeze the baseline and comparison method

Files:
- Read: `tools/radcast/studio.py`, `speech_cleanup.py`, `analysis.py`, `waveform_compare.py`.
- Read: `docs/radcast/studio-experiment/revision2/REPORT.md` and candidate QA files.
- Create: `docs/radcast/studio-experiment/revision3/manifest.json` and diagnostic report.
- If instrumentation is required, test in `tools/radcast/test_revision3.py` before changing the renderer.

- [ ] Record source/reference/r2 hashes and implementation/configuration hashes; preserve every existing audio artifact.
- [ ] Save the exact baseline parameters, protected original speech masks, applied EQ gain and the leveling/gain information needed for a fair comparison.
- [ ] Prepare listening excerpts from full PCM masters, not as separately processed short clips. WPE chunk context affects its estimates.
- [ ] Use identical active-speech masks and matched speech RMS for listening A/B. Apply only a common downward gain if peak headroom requires it. Keep native masters untouched.
- [ ] Compare cleanup-only stems before downstream EQ/leveling to isolate the dereverb change. Then render the complete pipeline with the same downstream configuration. Record any changes in adaptive gain so clarity/level differences are not misattributed to room removal; use a diagnostic frozen-gain A/B if those changes confound the result.
- [ ] Include the supplied phrase, a quiet word/consonant boundary such as “weeks forward,” a louder passage, and an actual phrase ending confirmed by waveform/word inspection. Earlier source energy falls were sometimes inside words; do not call them isolated room decay.

## Task 2 — make the WPE delay testable without changing the current default

Files:
- Modify: `tools/radcast/speech_cleanup.py`, `tools/radcast/studio.py`.
- Test: `tools/radcast/test_revision3.py`.
- Verify: `crates/radsuite-engines/src/studio.rs` still embeds every helper module.

- [ ] Write failing tests for validated delay selection, retained default delay 4, exact reported delay, native rate, finite output, unchanged sample count and no timing drift across chunk overlaps.
- [ ] Run: `/Users/rcd58/.radcast/venv311/bin/python -m unittest discover -s tools/radcast -p 'test_revision3.py'`; confirm the new behaviour fails before implementation.
- [ ] Add a delay argument with default 4. Proposed interface: `dereverb(x, sr, strength=.65, delay=4)`. Validate the supported conservative interval and integer type; reject zero/invalid delay.
- [ ] Pass that value to `wpe_v8` and record it in returned WPE metadata. Do not let static WPE_CONFIG metadata advertise delay 4 while executing delay 2.
- [ ] Expose an explicit experimental CLI flag, retaining delay 4 for the current application preset until the new candidate is reviewed. Record a distinct trial/revision ID in candidate metadata and filename.
- [ ] Hold blend .65, chunk 12 s, overlap 2 s, FFT 1024/hop 256, taps 10, iterations 2, PSD context 1 and the 80–8000 Hz WPE processing region fixed. Frequencies outside that region remain present; no low-pass is added.
- [ ] Run new and existing Python tests. Use a synthetic known-clean voiced/transient signal convolved with a simple reflection pattern as a diagnostic for direct-speech damage. Synthetic results do not certify the real recording.

## Task 3 — render one candidate and establish whether it helps

Files:
- Create: `docs/radcast/studio-experiment/revision3/CRJU160_studio_r3_wpe65_delay2.wav`, QA JSON and comparison artifacts.
- Reuse: `tools/radcast/analysis.py`, `asr_compare.py`, `waveform_compare.py`.

- [ ] Render from the supplied original MP3, not an enhanced recording, using float PCM throughout and final-only export.
- [ ] Compare the unmastered room-cleanup stems first, then the whole-pipeline candidate at matched speech level against r2 and the Adobe reference.
- [ ] Check voiced harmonic structure, spectral balance, quiet-word energy, onset/offset shape, pitch/timbre consistency where practical, clipping, true peak, lag and block drift. Report waveform correlation as a descriptor, not a target to maximize.
- [ ] Keep the existing watchdog thresholds. Run the locally normalized articulation-envelope and modulated voiced-window loss guard; retain raw envelope metrics too. Do not relax a threshold simply to pass the candidate.
- [ ] Run the same offline ASR settings. Inspect the phrase's actual transcript and key words alongside difference rates. ASR cannot certify absence of lisp, warbling or speaker change.
- [ ] Examine genuine low-speech intervals for noise/room gain; avoid describing consonant/breath energy as unwanted reverb. Do not report RT60 or direct-to-reverberant ratio unless a defensible method and its limitations are established.
- [ ] A candidate with erased consonants, hollow/changed voice, increased noise, timing drift or failed preservation QA is rejected or falls back visibly. A technically passing candidate must still have an audible dryness benefit to be selected.

## Task 4 — conditional method change, not endless WPE tuning

- [ ] If the shorter guard provides useful dryness without damage, proceed to the listening gate; do not add more processing.
- [ ] If it changes too little or damages direct speech, stop increasing WPE strength or shortening its guard. Save the failed evidence and explain the limit.
- [ ] Based on the measured failure, select at most one alternative source-preserving room-suppression approach. Keep it bounded during voiced/transient frames and explicitly preserve sibilants/air. Do not use heavy de-essing, bass inflation or a fixed low-pass to conceal artifacts.
- [ ] If a dedicated model becomes necessary after these two candidates, document why and propose the deferred research plan. Do not install, benchmark or integrate a third model/architecture under this plan.
- [ ] Honour the user's effort rule: use Sol 6.1 Medium. If a genuinely difficult audio/model integration or debugging problem remains unresolved after one focused investigation, stop and recommend Sol 6.1 High for that specific part only.

At most two new meaningful listening candidates in this round. At most two to three evidence-justified repair iterations. Stop when objective defects are resolved and remaining differences require human preference.

## Task 5 — integration and regression checks

- [ ] Integrate the selected experiment into the opt-in Studio path only after evidence supports it; continue to require human approval before default promotion.
- [ ] Preserve `studio_v18`, `studio_v18_natural_double_plus`, other existing IDs/behaviours, existing saved settings and unrelated RadSuite applications.
- [ ] Retain lossless working format, encode-once export, timing preservation, dependency failure reporting and explicit fallback warning/actual-stage metadata.
- [ ] Run `/Users/rcd58/.radcast/venv311/bin/python -m unittest discover -s tools/radcast -p 'test*.py'`.
- [ ] Run `RADSUITE_TEST_STUDIO=1 cargo test -p radsuite-engines -p radsuite-desktop`, including actual Studio WAV/MP3 processing.
- [ ] Run `npm --prefix apps/desktop-ui test -- --run` and `npm --prefix apps/desktop-ui run build` if UI/selection/contracts change; otherwise retain the latest verified frontend results and document that the UI was unchanged.
- [ ] Run `cargo fmt --all -- --check` and `git diff --check` for affected source.
- [ ] Request focused code review of stage order, metadata, fallback/error cleanup and preservation checks. Do not turn this into unrelated refactoring.

## Task 6 — listening approval gate

- [ ] Deliver the lossless WAV, optional listening MP3, aligned matched-level excerpts and exact paths/hashes.
- [ ] Report exact stages, WPE delay/blend, any actual adaptive EQ/gain changes, model/dependency versions, sample rate, duration, LUFS, true peak, speech/pause metrics with method provenance, spectral changes, watchdog/ASR results and regression totals.
- [ ] State honestly whether the remaining lecture-theatre sound has been reduced according to the listening comparison; zero QA warnings alone do not establish that result.
- [ ] Ask the user to judge the specified phrase for a closer/drier booth impression while Finnegan's voice, consonants, breaths and phrase dynamics remain recognisable and natural.
- [ ] Stop there. No default promotion, commit, push, deletion or overwrite of original/reference/historical audio evidence. Promotion still requires the user's separate instruction.
