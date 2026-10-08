# RADcast dereverberation model feasibility plan

> **For agentic workers:** Use superpowers:executing-plans for the separately agreed evaluation, with test-first adapter work and focused review. User constraints override generic workflow defaults: use this checkout, preserve existing evidence and presets, and make no commit, push or default promotion. This is a proposed new experiment, not an extension of revision 3's exhausted trial budget.

**Goal:** Establish whether one reproducible local, non-generative, full-band model can make Finnegan sound materially drier than trial B while preserving his voice and articulation.

**Architecture:** Qualify a checkpoint for actual active-speech dereverberation before installing or integrating it. Evaluate one qualified model in an isolated runtime against B and Adobe, first without mastering and then through the frozen Studio mastering configuration. Integrate only after a clearly useful listening result and the user's subsequent instruction; generic noise removal alone is insufficient.

**Tech Stack:** Existing 48 kHz float PCM, FFmpeg, Python/NumPy/SciPy, analysis/watchdog, offline ASR, matched-excerpt tools and an isolated CPU-capable model runtime if justified. Apple Silicon M1 Pro is the target machine; CUDA cannot be required. No new dependency or weight has been downloaded for this plan.

## Evidence and scope

On 9 October 2026 the user judged Adobe clearly superior by a fair margin, with almost no echo. B was best of the local versions but noticeably inferior. Neither r3 trial is approved for production. B becomes the local comparison baseline; native opt-in Studio remains revision 2. The lisp complaint applies to Optimized, per the user's correction.

Passing technical QA is necessary but did not predict the remaining audible room sound. Adobe is a perceptual reference, not an anechoic recording or paired clean ground truth. Its waveform cannot identify its proprietary architecture, training targets or exact settings. Do not claim recovered RT60, direct/reverberant ratio or Adobe equivalence.

Keep the original request's order: natural voice > intelligibility > cleanup > loudness. No vocoder, diffusion, bandwidth synthesis or voice reconstruction in this proposed experiment. Learned masks/filters may still distort phonetics and must be tested. Native processing at 48 kHz is preferred; a 16 kHz model followed by upsampling does not meet full-band preservation. A separate multiband/resampling architecture would need its own design, rather than being silently introduced here.

## Initial research leads — neither is qualified yet

| Lead | Verified primary evidence | Unresolved reason not to select it yet |
|---|---|---|
| ClearerVoice `MossFormer2_SE_48K` | Author documentation describes native 48 kHz phase-sensitive masking and inverse STFT; official weight repository reports about 222 MB and Apache-2.0. Source licence is Apache-2.0. | The model card describes background-noise removal, without demonstrating the required dereverberation. Apple Silicon cost and long-file correctness need verification. An upstream issue reports length/indexing defects in direct NumPy segmented inference. |
| Room-trained DeepFilterNet checkpoints from the 2026 Treble reproducibility package | Authors release ISM-trained checkpoints and code for evaluating speech enhancement with room simulations. Existing RADcast DF runtime makes this worth inspecting. | The improved Hybrid checkpoints are explicitly not released. Inspect the available model's rate, target definition, checkpoint licence and dry-speech performance; reverberant training input alone does not establish dereverberation. Do not promise the paper's Hybrid results from public ISM weights. |

Sources checked 9 October 2026:

- [ClearerVoice enhancement architecture](https://github.com/modelscope/ClearerVoice-Studio/blob/main/train/speech_enhancement/README.md), [model card](https://huggingface.co/alibabasglab/MossFormer2_SE_48K), [weight listing](https://huggingface.co/alibabasglab/MossFormer2_SE_48K/tree/main), [source licence](https://github.com/modelscope/ClearerVoice-Studio/blob/main/LICENSE), [48 kHz inference configuration](https://github.com/modelscope/ClearerVoice-Studio/blob/main/clearvoice/clearvoice/config/inference/MossFormer2_SE_48K.yaml), [reported segmented-inference issue](https://github.com/modelscope/ClearerVoice-Studio/issues/169).
- [Treble reproducibility repository](https://github.com/TrebleTechnologies/iwaenc2026milo), [authors' paper](https://arxiv.org/abs/2608.20971).

These are leads for qualification, not assertions that either approaches Adobe. If neither qualifies, report that result rather than downloading a convenient noise model or launching a broad search matrix. Training a custom model, purchasing datasets and contacting model authors are outside this plan.

## Task 1 — qualify one checkpoint before a runtime experiment

**Files:** Create `docs/radcast/studio-experiment/model-feasibility/QUALIFICATION.md` and `qualification.json` in a future separately agreed round. Read upstream inference/training code, checkpoint metadata, licence files and this plan.

- [ ] Inspect at most the two leads above. Record immutable code and model revisions, code/weights licences, size, dependencies, native bandwidth, single-channel inference and actual output construction. Inspect training target: dry speech, early-reverberant speech or unchanged reverberant speech.
- [ ] Require published paired reverberant/clean evidence or an inspectable dereverberation training/evaluation setup for the available checkpoint. A generic enhancement label or noise-only benchmark cannot qualify it. Record uncertainty explicitly.
- [ ] Audit padding, normalization, windowing, chunk/context/overlap, sample count, model delay, hidden resampling and input scale. Treat the reported ClearerVoice bug as an unverified issue to inspect at the pinned revision, not a pre-written patch to apply blindly.
- [ ] Map a minimal CPU inference dependency set and expected disk/RAM/time costs. Do not install training, video, separation or super-resolution dependencies just because the upstream umbrella package includes them.
- [ ] Select one model only if its downloadable checkpoint and inference path meet the constraints. Otherwise stop with a concise qualification failure. No app change is needed to reach this decision.

**Output:** a justified one-model experiment specification, or evidence that neither lead is suitable. Resolve the model and its exact configuration before writing an adapter.

## Task 2 — isolated runtime and meaningful preservation controls

**Files:** If one model qualifies, create `tools/radcast/model_trial.py` (48 kHz array adapter and experiment runner), `tools/radcast/test_model_trial.py` (contract/boundary tests), and a fresh `docs/radcast/studio-experiment/model-feasibility/<model-revision>/` evidence folder. Reuse `analysis.py`, `speech_cleanup.py` and `studio.py`; do not change native preset defaults.

- [ ] Use a separate environment/cache with pinned versions and hashes; leave `.radcast/venv311` intact. Record actual install footprint and downloaded weights. Download only the selected checkpoint. After installation, verify inference with network access disabled and log CPU device, wall time and peak memory.
- [ ] Write failing tests for exact rate/count, finite floats, documented latency compensation, deterministic re-runs, chunk joins and explicit dependency/checkpoint failure. Test a file longer than the model's one-pass limit and a non-window-aligned final fragment. Reject unexplained length changes; do not trim away speech simply to pass duration QA.
- [ ] Implement the smallest adapter matching audited preprocessing and output scale. Disable dither or fix its seed if upstream preprocessing randomizes it. Save requested/actual stages, code/config/weight hashes, source hash and raw float output. Do not return fallback audio labelled as a successful model result.
- [ ] Use an appropriately licensed known-clean speech excerpt at the model's full bandwidth, plus declared synthetic reflections, to compare clean-input preservation and reverberant-input error at matched gain. Retain vowels, S/SH/T/CH/F, quiet words and phrase endings. Record provenance/licence; do not substitute Adobe as clean ground truth. A chirped tone alone cannot qualify natural-speech quality.
- [ ] Demonstrate useful room reduction on the paired control without substantial direct-speech damage. Record gain-invariant metrics as descriptors and inspect/listen to audio; do not select using a single score. If the clean control is unavailable, postpone this qualification instead of silently omitting it.

Run: `<isolated-python> -m unittest discover -s tools/radcast -p 'test_model_trial.py'`. Expected: audited adapter contracts pass; known-clean/control results and runtime are recorded, not converted into an unsupported pass score.

## Task 3 — one full-source trial against the best local result

**Files:** Create a traceably named full float WAV, QA JSON, stem/gain manifest, matched listening excerpts and report in the fresh model evidence folder. Reuse `analysis.py`, `asr_compare.py`, `compare_trial.py` and `audition_set.py` patterns; parameterize filenames rather than overwrite r3 evidence.

- [ ] Start from the original MP3; decode once to 48 kHz float PCM and retain sample timing. Replace the current room/cleanup block with the selected model for the first trial. A joint model replaces both WPE and DF3; avoid automatic double enhancement. For a genuinely dereverb-only model, decide and freeze downstream denoise before rendering.
- [ ] Save isolated cleanup output, then apply the same guarded pause policy, adaptive presence policy, speech-leveling configuration, light compression and final targets as Studio r2. Record the adaptive gains actually used. Replay frozen baseline gains as a diagnostic if a claimed clarity/dryness advantage may instead be leveling.
- [ ] Process full-source context before cutting excerpts. Compare original, B, new candidate and Adobe using identical original-derived speech masks and static downward RMS matching. Reuse original 20–32 s / Adobe 17.761–29.761 s for the target phrase; verify that stable offset. Include quiet consonants, louder speech and the previously examined phrase-ending context.
- [ ] Run unchanged timing, spectral, local voiced-loss, pause, clipping and true-peak guards; run the same offline ASR. Inspect warnings, important words and cleanup-only spectra. No warning-threshold relaxation to save a model. Only actual clean ground truth permits intrusive clean-reference scoring.
- [ ] Deliver one meaningful model candidate. A second render is permitted only for a diagnosed preservation defect addressed by one bounded blend/configuration change. No architecture or EQ matrix, no repeated strength escalation. Limit repairs to two focused attempts; stop if benefit remains weak or voice becomes altered.

**Human success criterion:** Clearly audible active-speech dryness improvement over B in the target phrase and other passages, with acceptable identity, natural consonants, breaths, quiet words and phrase dynamics. Report the remaining gap to Adobe plainly. Neither quieter pauses nor zero QA warnings establishes this result.

## Task 4 — evidence gate and possible later integration

- [ ] Report source/model/code hashes, licence/runtime/dependency implications, stages and preservation blend, duration/rate, LUFS/true peak, meaningful spectral/timing/ASR findings, paired controls and remaining subjective defects. Provide full WAV and matched audio clips; optional MP3 is final-export only.
- [ ] Stop for the user's listening result. If it is still noticeably inferior without a material improvement over B, close the experiment and present the feasibility limit. Do not claim that more opaque reconstruction must therefore be accepted.
- [ ] Only after listening approval and separate integration/promotion instruction, embed a qualified adapter in `crates/radsuite-engines/src/studio.rs` and make the requested opt-in change. Preserve all legacy IDs, saved selections, QA/fallback provenance and unrelated applications. No default promotion by implication.
- [ ] For integration, run Python tests with the qualified runtime; `RADSUITE_TEST_STUDIO=1 cargo test -p radsuite-engines -p radsuite-desktop`; `cargo fmt --all -- --check`; `git diff --check`. Run frontend tests/build only if UI/contracts change, and request focused code review. Report external-fixture skips honestly.

Keep Sol 6.1 Medium as requested. If a genuinely difficult integration/debugging issue remains after one focused investigation, stop and recommend Sol 6.1 High for that part only. No unexplained dependency expansion or indefinite tuning.
