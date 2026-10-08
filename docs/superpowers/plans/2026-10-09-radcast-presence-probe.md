# RADcast bounded presence probe implementation plan

> Execute in the existing checkout, with test-first work and focused review. User's 9 October “ok, proceed” authorizes continuing the described presence/body/word-definition investigation. Preserve historical evidence and existing presets. No worktree, commit, push, new model, package install or default change.

**Goal:** Produce one controlled listening comparison to determine whether modest tonal correction helps the remaining Adobe presence gap while preserving Finnegan's articulation and existing cleanup.

**Architecture:** Resume the accepted original-source model/dry20 pipeline at its saved 48 kHz FLOAT pre-compression `levelled.wav` stage. Reproduce the baseline light compression once, verify its output against the baseline master before its recorded scalar gain, then add three small minimum-phase bell filters and the existing final constant gain policy. No delivered enhanced file feeds the new candidate, no signal gets a second compression pass, and no model or adaptive leveling is rerun. This is a fixed calibration probe, not production adaptive EQ or an inferred Adobe recipe.

**Tech Stack:** Cached NumPy/SciPy/SoundFile, FFmpeg, unchanged analysis/watchdog and optional cached Whisper small. Offline existing experiment runtime; no dependency changes.

## Evidence and decisions

The user reports improved attempts but still prefers Adobe's voice presence. Target clips have nearly equal measured LUFS. Adobe shows more body and 1.5–8 kHz energy, with less very high air; contrast varies across passages. No fixed EQ inversion is justified.

Recommended first approach: one conservative static tonal probe with fixed upstream processing. Alternatives considered: speech-adaptive tonal shaping (adds another moving variable and requires dry/noisy-speech validation before production), or another cleanup/model trial (does not isolate tonal contribution and would reopen model qualification). Do not pursue those alternatives in this bounded round.

Frozen extra filter configuration:

- Bell at 180 Hz, Q 0.8, +1.0 dB, to modestly restore body without boosting sub-bass.
- Bell at 3,000 Hz, Q 0.8, +2.0 dB, for a bounded speech-definition probe.
- Bell at 5,200 Hz, Q 1.0, +1.5 dB, for a small consonant-detail probe.
- No shelf, fixed low-pass, de-esser, exciter, saturation, generator or added compressor. No time-varying EQ, target-region splice or strength matrix.
- Validate the EXTRA three-bell response peak <=3.5 dB, sub-80-Hz gain <=0.5 dB, no response cliff/high-frequency attenuation. The baseline already has a +2 dB/2.4 kHz presence filter. Record extra and nominal summed EQ responses separately; compression between the old and new filters prevents calling the sum an exact full-chain transfer function.

Only one full candidate is permitted in this round. If original-source preservation QA fails, retain the rejected attempt and stop. Do not relax thresholds or escalate strength. Human listening decides whether the tonal probe improves presence or instead sounds boomy, sharp or more room-coloured.

## Task 1 — test and implement bounded filtering

**Files:** Create `tools/radcast/presence_probe.py` and `tools/radcast/test_presence_probe.py`. Preserve `studio.py`, `speech_cleanup.py` and `model_trial.py`.

- [ ] Write failing tests for finite mono native-48k/count contracts, deterministic nonaligned output, silence preservation, combined-response bounds/sub preservation/upper-band retention, and preserved impulse timing. Refuse invalid input and modification of frozen configuration.
- [ ] Run the new tests and observe the expected missing-implementation failure.
- [ ] Implement the three bell filters with cached SciPy; retain float PCM and exact samples. Treat phase/timing explicitly; no zero-phase acausal smoothing or resampling.
- [ ] Run new tests and the existing Python suite; real cached-model tests remain enabled. Save code/config hashes.

## Task 2 — one full preserved-upstream render

**Files:** Fresh `docs/radcast/studio-experiment/presence-probe/` directory only. Base: accepted model-feasibility `finnegan-repair-dry20/trial.qa.json`, final FLOAT master and prepared original.

- [ ] Verify accepted/nonfallback baseline, current master/source/hash identities and unchanged processing format before creating a fresh evidence folder. Refuse stale/nonempty output directories.
- [ ] Verify saved original-derived `levelled.wav` identity, reproduce the SAME single compressor pass with FFmpeg, and check it against baseline master divided by its recorded scalar gain within float rounding. Save compressed and filtered pre-final-gain stems plus resulting full float master. Use `studio.master(..., compression=False)` for final constant gain. Record the existing chain plus extra probe, actual final scalar gain and its difference from baseline +6.14 dB, response curve, code and baseline manifest hashes.
- [ ] Apply unchanged original-derived masks, timing, spectrum, local voiced-loss, pause, true-peak and clipping guards. Independently compare against baseline as well; pause level or spectral balance can worsen even when original-source QA passes. Report descriptive changes, without treating Adobe as clean ground truth.
- [ ] If accepted, export MP3 only from final WAV and verify against original. Save actual model-side original blend .20 and baseline model/checkpoint information by reference; no claim of new model inference.

## Task 3 — controlled matched audition and review

**Files:** New comparison manifest/report and listening gate in the fresh round.

- [ ] Cut target20–32, quiet19.5–22.5, loud70–80 and ending38.5–41.8 from full processed recordings. Compare before-probe/after-probe/Adobe using identical fixed original masks, static downward speech-RMS matching, and common peak headroom. Adobe offset −2.239 remains fixed for those verified common-content excerpts.
- [ ] Measure matched excerpt LUFS; also make one target three-way LUFS-matched set with static downward gains if spectral correction changes measured loudness. These are separate audition controls, not new candidates. No separate dynamic leveling per excerpt.
- [ ] Run exact cached ASR settings `-l en -t 4 -nf -nt -otxt` for baseline and probe, using analysis-only 16k conversions; compare to the already reproduced original-ASR proxy. Record model/binary hashes and distinguish edit differences from WER/identity.
- [ ] Review frozen gains, no repeated compression, full original-source derivation, response and rejected/accepted provenance; verify all generated hashes/durations/matching independently.
- [ ] Deliver one meaningful candidate and concise evidence. No app/default change until explicit human approval and later integration instruction. Stop at listening; do not conclude presence, identity or Adobe parity from QA/spectra/ASR.

## Stop and interpretation

This new authorized tonal probe does not extend or erase the exhausted two-render model trial. Even success would only establish that this bounded tonal change is useful on the calibration source. Before later application integration, derive conservative adaptive eligibility from broader speakers/rooms rather than applying this calibration EQ to every voice.
