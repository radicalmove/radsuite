# USES Close-Microphone Qualification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox syntax. The user said “please continue where you left off” after the concrete USES design was presented. Execute in this session; no further scope permission is needed for this bounded experiment.

**Goal:** Determine whether the released USES dereverberation mode is a viable local replacement cleanup block and, only if it passes qualification, present one original-derived full candidate focused on “in the weeks forward”.

**Architecture:** Strictly load the single pinned checkpoint into audited upstream USES layers in an isolated experiment runtime. Explicitly select dereverb memory group1, scale STFT to native48k and retain full-band complex prediction. Keep application code/defaults and existing experiment environment untouched. Replace the cleanup model alone; do not cascade another enhancer or feed a delivered master into inference.

**Tech stack:** Existing Python3.11/Torch2.1.1/NumPy/SciPy/SoundFile read-only packages; only minimal separately pinned dependencies if required; ESPnet source revision bfc13cecfd0a07ed8e21d733b0ce130a1c69211a and HF revision927a9ecea245120a6f2d88c2552864b937ec5ab9. CPU4threads, deterministic eval/inference mode, network-denied inference.

**Working area:** `/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/`; isolated runtime `/Users/rcd58/.radcast/experiments/uses-qualification/`. Preserve dirty unrelated native/UI files and do not commit/push.

## Task1 — Artifact/runtime qualification

- [ ] Save publisher metadata, exact config/meta/licences and required audited upstream source files with hashes. Acquire only20epoch.pth, expected12,279,888bytes and SHA6b2a0c78b2eea566fcfd39b0d77ffc0102b0baf6af60a4a1f4dee93995081121. Never load the11-byte aliases.
- [ ] Read all inference dependencies before executing them. Build a separate interpreter referencing existing packages read-only; additional dependencies stay inside that experiment, fully pinned. Use a sandbox that denies network and original-runtime writes.
- [ ] Create tests in `/Users/rcd58/Documents/RADsuite/tools/radcast/test_uses_trial.py` before adapter implementation. Observe expected failures for absent behavior: invalid audio/sample-rate rejection; checkpoint missing/extra/shape errors; explicit dereverb path; native48k STFT inverse exactlength; normalization/de-normalization; deterministic real strict-load smoke when pinned artifacts are enabled.
- [ ] Implement minimal adapter `/Users/rcd58/Documents/RADsuite/tools/radcast/uses_trial.py`; preserve upstream model math. Assert every key/shape matches, config one speaker/two memory groups; record tensors/parameters and actual dereverb invocation. Compute configured FFT/hop256/128 ×48000/8000 =1536/768. Restore input variance exactly once; no unrelated wrapper output RMS normalization or clipping.
- [ ] Run a short native48k CPU smoke before long inference. Recordfinite samples,length, timing, RSS and repeatability. Stop on load, mode, sample-rate, incompatibility or unreasonable resource failure; do not secretly swap models or resample.

## Task2 — Voice/reflection controls

- [ ] Reuse the two saved licensed clean sources and all pinned hashes from Treble controls. Construct early-only and late-only variants by splitting the existing `model_trial.synthetic_room` direct/taps/tail, not changing its values. Keep a common down-only headroom gain across each speaker's conditions.
- [ ] Test the new control construction before implementing a runner: clean unchanged; early14/37ms taps.25/.12, no late tail; late begins50ms with same seed20261009, normalized L2.35 and.65s decay parameter; source samplecount preserved and no anticipatory energy.
- [ ] Run one USES configuration on clean, early, late and prior composite signals. Record strictmodel identity, runtime, original/paired masks, waveformwatchdog and paired clean-reference SI-SDR alongside current model controls. Run current pinned Treble on new early/late controls only if needed for a fair comparison; label control inference separately from candidate inference.
- [ ] Qualification gate: clean watchdogs must be clear; no nonfinite/clipping/count/drift issues; paired early/late measurements must show meaningful room benefit without claiming they measure Finnegan proximity. Provide matched clean voice examples for listening. If the early condition is worse than untreated, or clean source is materially damaged, close the experiment before a Finnegan candidate. Do not adjust modes or thresholds to rescue failing results.

## Task3 — At most one full source candidate

- [ ] If prerequisites pass, decode the protected original once or reuse its independently hash-verified prepared float stage. Apply existing45Hz source highpass then one USES cleanup. Retain raw cleanup separately. Apply unchanged pause/presence/speech-leveling/compressor/master policy; record any newly calculated level gain curve explicitly. Do not treat the old model-specific saved gain curve as a production invariant.
- [ ] Use unchanged original-source and previously accepted dry20 baseline watchdogs. Adaptive candidate remains an unapproved reference withASR flags. Reject a failed full candidate; this plan has no repair render budget. Retain attempted master/evidence, do not encode a failed delivery.
- [ ] For a pass, encode final MP3 once and verify separately while preserving WAVQA. Save same-settings cachedASR model/binary/input hashes and substantive recognition changes; output is a proxy, not verifiedWER or a voice-quality ranking.
- [ ] Create static-gain listening copies with original19.5–22.5s/Adobe17.261–20.261s, compare USES/currentadaptive/Adobe at matched speechRMS and record actualLUFS. Include the opening/faint/ending and charged/convicted material from the single full run.
- [ ] Independently rehash protectedfiles/runtime/checkpoint and replay downstream artifacts, verifyallclips/reports/linkpaths, run relevanttest suite with real model enabled and record gate/report. No additional candidates, native preset integration, defaults, or promotion. Human closer-voice/identity/articulation result remains decisive.

Budget: one released model configuration; one full candidate after qualification; zero automatic repairs. Stop rather than relax a guard. Review-agent unavailability does not waive read-through, strict loading, independent artifact checks or honest reporting; record any unavailable independent review.

## Pre-inference audit updates

The user identified USES2 and confirmed that officialcode/recipes must not be mistaken for a verifiedreleasedcheckpoint. The completed boundedreleaseaudit did not establish USES2weights. Continue with explicitlylabelledlegacyUSES; no silentweightsubstitution. USES2reviewsource is archived as textonly.

Predeclare before controls: early/late pairedSI-SDR benefit must be≥1.0dB overuntreated. Ifclean and theseinitialroomgates pass, run pinnedTreble on the same newearly/late inputs sequentially. Require earlyUSES benefit≥1.0dB overTreble forbothspeakers and no late degradation>1.0dB. These are experimentalqualificationcriteria, not relaxed existingwaveformguards or perceptualcertification. Historicalcompositecontrols alone cannotmeet thisnew comparison.

Isolatedruntime created withoutpip/newdependencies, referencing existingtorch/mathpackages read-only. Minimal audited upstreamsource subset will preserve modelclass ASTs and imports only will be rewritten; no modelmath changes or applicationcode changes. Independent design reviewer hit workspacecreditlimit; root continues direct audit and separate implementation/artifact checks without claiming that review succeeded.

## Measured smoke and locked control subset

The first1slicensedCSTRclean smoke passed native48k/769bins/exactsamplecount/finiteoutput and actualdereverbmemory1; wall8.701seconds, peakRSS2.663GB. No Finnegan inference. Before anycontrol result, fix the initialqualification subset to firstTWOcompleteutterances per speaker (p2320:475203samples; p2570:398282), including a consonant-rich snowpeas/spoons sentence. Fullsource/controlhashes remainverified, selection/transcripts retained, no midwordboundary or result-dependentselection. Fourconditions per speaker and fixed+1dB comparativegates unchanged. This smallset is a limited qualification, not generalizationproof.

Full143.49s upstreampostencoder activation alone is roughly7GB; additionalnormtemporaries canexceed16GB device capacity. Assess compatibility of boundedintermediateexecution preserving exactly the publishedNN operations/convhalos and persistentmemory, before whole-source work. Never silently switch to independentlychunked/crossfaded enhancement or resetmodelmemory. Any execution optimization requires observed red/green equivalence tests and recorded max/RMSdifferences against original unmodifiedforward on short multi-segment inputs; no weight/target/processing-strength change. Ifmemory remainsimpractical, stop at qualification and disclose it.

## Final execution outcome

Task1complete: strict pinnedload/localCPU/native48k proof andboundedintermediatemath verified. Task2complete: all8USEScontrols measured; bothvoices fail predeclared early/latebenefit. NewTreblecallsconditionalongate were skipped; no missingruntimeerror. Task3noteligible:0Finneganrenders/0deliveryexports/0ledgerclaims, roundclosedremainingrenderbudget0. Protectedreferences/runtime/unrelatedtrackedfiles unchanged;106testsPASSnoskips24.124s andindependent savedartifactverificationPASS. PerceptualAdobecloseness objective remainsopen. See uses-qualification/REPORT.md andround-closure.json.
