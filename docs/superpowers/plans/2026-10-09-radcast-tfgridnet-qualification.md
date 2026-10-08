# URGENT 2025 TFGridNet Qualification Plan

> **For agentic workers:** Execute this approved bounded experiment inline. Use test-driven-development and requesting-code-review. The user explicitly said “ok, please proceed” after the released TFGridNet lead was presented. No additional scope confirmation is required.

**Goal:** Qualify the actual published TFGridNetV3 weights for local native48k speech restoration, then generate at most one Finnegan listening candidate if the prerequisite tests pass.

**Architecture:** New isolated experiment folder/runtime; strict weights-only checkpoint loading and unchanged audited publisher neural classes. Native FFT1536/hop768, input variance restored once, no peak-output normalization or downsampling. Use one fixed3s window/2s hop with1s cosine overlap for long inputs, including controls and the full recording. This explicitly changes long-context execution compared with an unrestricted whole-utterance forward; it is not claimed equivalent. It respects the released native48k training cap of144000samples (nominal chunks are4s at lower rates) and bounds bidirectional-RNN/global-attention memory on the16GB Mac. Apply published math within each window and document every window. No audio stretching, model cascade, separate target-phrase processing or arbitrary candidate matrix.

**Tech stack:** Existing read-only Python3.11/Torch2.1.1/math packages, pinned ESPnet classes/source and publisher weights. No app/native/default edits, original-runtime upgrades, commits or promotion.

## Task1 — Provenance and adapter

- [ ] New artifacts under `docs/radcast/studio-experiment/tfgridnet-qualification/`; new interpreter under `/Users/rcd58/.radcast/experiments/tfgridnet-qualification/`.
- [ ] Pin publisherHF revision daad000927daa131e7692376a71c8144bcbfa6f8, model `kohei0209/tfgridnet_urgent25`, actual binary valid.loss.ave_5best.pth,34,192,512bytes/SHA8350b6f84bb5de01646b7cebe9d19d5b1fc4318cd85c31d242caccc2442d276e. Preserve CC-BY4 metadata and Apache2 code attribution. Audit publisher fork against pinned main class before selecting source; retain config, modelcard, metadata and training-target evidence.
- [ ] Test first in `tools/radcast/test_tfgrid_trial.py`: invalidaudio, source/config/hash errors, all-key/all-shape strictload, nativeFFT inverse length, one variance restoration, no peak normalization, overlap reconstruction with identity processor and arbitrary lengths. Unit tests must not imply learned voice preservation.
- [ ] Implement `tools/radcast/tfgrid_trial.py` plus unique vendor package preserving original class ASTs. Require published single-source/mic six-layer configuration. Record tensor/parameter count, versions, source hashes and actual48k spectrum size. Network-denied inference, original environments write-denied.
- [ ] One short licensed clean CPU smoke and deterministic repeat. Measure speed/RSS before controls. Fixed segmentation: process0:3s,2:5s,…until a window reaches the end; no redundant tiny terminal window. Compute variance separately per actual window, restore once before cosine overlap, then divide by accumulated weights. No RMS/phase matching between outputs. Never silently treat independently windowed output as full-context equivalent.

## Task2 — Controlled qualification (criteria fixed before inference)

- [ ] Reuse independently verified firstTWOcomplete licensed sentences for p232/p257, same direct/early/late/composite RIRs and common headroom from the preceding experiment. Do not mutate historical inputs/helpers. Eight sequential TFGridNet cases; save input/output/trace before analysis, keep all results.
- [ ] Clean-input unchanged waveform watchdog must pass and paired clean SI-SDR must be≥15dB for each speaker. This added experimental screen limits gross waveform changes; it is not identity or perceptual quality certification.
- [ ] Early-only output must be no more than1dB worse than untreated clean-reference SI-SDR. The published target retains approximately50ms early response, so do not demand training against an anechoic early target that the checkpoint did not use.
- [ ] Late-only and composite output must improve paired SI-SDR by≥1dB over untreated for each speaker; no nonfinite/clip/count/timing failure in any case. SI-SDR includes all signal error, not percentage echo.
- [ ] If those prerequisites pass, infer the current pinned Treble on the SAMEselected late/composite inputs, four calls. Require TFGridNet≥Treble+1dB in both conditions for both speakers. If any gate fails, close before a Finnegan render without changing thresholds, windows, weights or tone.

## Task3 — One conditional full candidate

- [ ] Implement `tools/radcast/tfgrid_render.py` with test-first false/incomplete-controls rejection and exclusive study-level attempt ledger. Pin the reviewed dry20 reference report SHA5855b9cac8caeed1229c094bccfaadb1a28177255b783b844685a98b5b06a4a9; prepared/source/master hashes and helper digests must match.
- [ ] One original-derived prepared48k/highpassed45Hz lossless input, processed across the complete143.49s with the same fixedwindow policy. No separately inferred phrase. Retain raw cleanup. Fixed actual+2dB presence and unchanged guarded pauses/leveling/singlelightcompressor/master policy; record newly calculated leveling curve. No original blend or automatic repair in this round.
- [ ] Apply unchanged original and reviewed-baseline watchdogs. A failed candidate stays visibly attempted; no delivery encoding. A pass receives one final MP3 export/check plus same-settings cached ASR and explicit substantive recognition flags.
- [ ] Static speech-level-matched comparisons cover original19.5–22.5s (“in the weeks forward”), Adobe−2.239s alignment, plus prior20–32s context, opening, faint/internal/ending material and the charged/convicted explanation. Record actualRMS/LUFS differences. Voice/closeness approval remains human; no Adobe-algorithm or parity claim.
- [ ] Fresh relevant/full tests with actualmodels where applicable, saved-artifact/hash/replay verification, independent review and concise report. Protected references and unrelated dirty files unchanged. No native/default change or promotion.

Budget: one model/configuration/window policy; eight prerequisite plus four conditional baseline calls; at most one full candidate, zero repair renders. A negative qualification closes this round. USES/USES2 history and previous closed budgets are preserved.

Pre-inference source audit: use exact publisher6be80495 class, native48k cap144000samples, so freeze3s/2shop/1soverlap before any learned forward. No parameter/window search.

Pre-qualification result-independent join criterion: protected20msframe mergedenergy maynotfall morethan1dB below weighted per-windowenergy. This measures phase/amplitude disagreement, not reverb; no gain/phase matching isintroduced. A failedjoinblocks Finnegan.
