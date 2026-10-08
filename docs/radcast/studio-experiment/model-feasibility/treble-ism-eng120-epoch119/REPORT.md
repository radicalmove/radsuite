# Finnegan room-model feasibility trial — 9 October 2026

**Latest listening feedback:** the user reports that the local attempts are better, but Adobe still creates a special sense of voice presence. This is useful listening evidence, without a specific new-versus-B ranking or explicit identity/articulation/integration approval. The remaining target is a fuller, more immediate voice with clearer word definition. [Exact feedback](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/human-listening-result.json).

The saved target clips have nearly equal measured loudness: B −26.40 LUFS, new −26.56, Adobe −26.52. A gross playback-level mismatch does not explain the reported gap, although equal LUFS cannot prove identical subjective loudness. In the target phrase Adobe's speech-frame energy shares exceed the new trial by 2.32 dB in 80–350 Hz, 4.43 dB in 1.5–4 kHz and 5.03 dB in 4–8 kHz, with 2.14 dB less above 8 kHz. This suggests substance plus definition rather than a blanket brightness target. Differences vary across passages and are not an EQ recipe or evidence of Adobe's mechanism. [Presence descriptors](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/presence-diagnosis.json). No new audio was rendered for this diagnosis.

**One repaired candidate is ready for matched listening against B and Adobe.** Technical preservation checks pass. Audible room reduction, natural articulation and speaker identity remain unapproved; no Adobe parity is claimed. Native opt-in Studio stays on revision 2, the existing UI selection stays unchanged, and no commit, push or promotion occurred.

The user judged Adobe clearly superior to all four previous versions, with almost no echo, and B the best local version but noticeably inferior. That closed revision 3. This separate approved experiment evaluates a model trained with room simulations rather than extending that round's DSP tuning.

## Listen first

These 12-second excerpts use original 20–32 seconds and Adobe 17.761–29.761 seconds. All come from fully processed recordings. The same original speech mask and static downward gains match speech RMS to −26.53 dBFS. No enhancement, EQ or timing adjustment is added to these listening copies.

| Version | Matched target phrase |
|---|---|
| New model, 20% original preservation | [Play candidate](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/comparison/listening-triplet/model_dry20_target20-32_matched.wav) |
| B, previously preferred local trial | [Play B](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/comparison/listening-triplet/B_target20-32_matched.wav) |
| Adobe reference | [Play Adobe](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/comparison/listening-triplet/Adobe_target20-32_matched.wav) |

Listen to “reading anything to do with the criminal law a lot more simple”: does the new model reduce the room sound materially compared with B, while Finnegan's consonants and voice remain natural? Then check the quiet, louder and phrase-ending passages in the [comparison report](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/comparison/REPORT.md), including the faint beginning/end contexts. Adobe is omitted from handle comparisons where equivalent content is unverified or absent.

[Full 48 kHz float WAV](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/finnegan-repair-dry20/CRJU160_modeltrial_ISM120_best119_dry20.wav) · [Final 320 kbps MP3](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/finnegan-repair-dry20/CRJU160_modeltrial_ISM120_best119_dry20.mp3).

## Model and frozen processing

Selected Treble ISM-eng-120 **best-validation epoch 119**, repository revision `8e7998acd7e2beb2776fdcc5231731891383bbc7`. The public checkpoint is native 48 kHz, using observed-spectrum ERB gains and complex multi-frame filters. The inspectable training setup includes a partially dereverberated target plus a dry component. Historical target settings are incompletely pinned, and the improved Hybrid weights are unreleased. This supports a feasibility experiment, not an expected Adobe result. [Qualification and rejected ClearerVoice lead](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/QUALIFICATION.md).

The checkpoint is 8,715,794 bytes, SHA256 `a3cc6a842ee7674c9763dabbe16ffd562cf617aed961b797dd0bc39d1e422fd1`; config SHA256 `22aad5d5f95312051b755b61cbf050d68cf55d86bb1800931964f4b65983573b`. Pinned Git blob identities also match. All 133 state tensors, totaling 2,167,969 elements, load with exact keys/shapes and equal learned values. The included MIT licence and third-party notices are retained. Dataset rights are recorded separately.

Actual chain:

1. Original MP3 decoded once to mono 48 kHz float PCM; existing 45 Hz highpass.
2. One joint room/noise DF3 model, whole recording context, compensated 480-sample algorithmic delay, no postfilter or attenuation limit. It replaces the previous room/WPE, DF and frequency-dependent preservation block.
3. For the single repair only: 80% model output plus 20% highpassed original over the whole band and recording.
4. Existing guarded pause attenuation, at most 6 dB; adaptive 2.4 kHz presence correction, actual +2 dB.
5. Existing protected 2-second speech leveling with 1-second lookahead, smoothed and bounded ±8 dB; light 1.15:1 compression.
6. Constant final gain +6.14 dB, constrained by true peak. No time stretching, low-pass, vocoder, bandwidth synthesis or voice reconstruction.

Adaptive policies remain frozen, but their measured gains depend on model output. Actual leveling target is −28.07 dBFS, with 4,664 eligible frames and the existing ±8 dB bounds. A replay of the exact saved B gain curve is included as a diagnostic, along with matched cleanup/presence stems. It removes the leveling-choice difference partially; preceding cleanup, pause and adaptive presence still differ. Effects cannot be attributed solely to the new checkpoint. [Trial QA and stage/hash manifest](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/finnegan-repair-dry20/trial.qa.json).

## Why one preservation repair was necessary

The unblended full-source render rejected under unchanged `quiet_speech_loss`: its worst protected window lost 23.08 dB relative to typical final gain. Saved stems localize the loss to raw model inference, whose worst relative attenuation is 27.81 dB; highpass worst is only 0.20 dB. Faint periodic/modulated material near 4–5.5 and 138–141 seconds is suppressed. Context-limited ASR suggests speech, but these are not verified human words. Guard thresholds and window eligibility were not relaxed.

The plan's one bounded repair adds 20% original for signal retention. It restores some original room sound as well. The same raw model output is exactly reproduced in both renders; only the explicit preservation blend changes. The repaired final worst relative window loss is **5.08 dB**, passing the existing 12 dB guard. Both attempts, raw stems, masks and gains remain available; the rejected master is clearly named `master_attempt.wav`. [Diagnosis and repair basis](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/PRESERVATION-REPAIR.md).

No additional model, strength matrix or repair was evaluated. If this candidate fails listening, that is the outcome of this experiment.

## Preservation, controls and resource evidence

| Measure | Repaired candidate |
|---|---:|
| Duration / exact samples | 143.490 s / 6,887,520 |
| Working and WAV format | Mono 48 kHz float PCM |
| Integrated loudness / true peak | −22.46 LUFS / −1.55 dBTP |
| Loudness range | 6.7 LU |
| Measured lag / block drift | 0 / 0 ms |
| Clipped samples / watchdog warnings | 0 / 0 |
| Locally normalized articulation envelope correlation | 0.9743 |
| Full model CPU inference | 7.36 s; 0.0513 × recording time |
| Peak process memory, complete render/QA | 2,319,810,560 bytes, about 2.16 GiB |

The −20.75 LUFS target yields to the same true-peak constraint as Studio. The final MP3 independently passes preservation/export QA at 143.490 seconds, −22.46 LUFS and −1.55 dBTP. Only that final convenience export is lossy. Final spectrum shares differ from B by about −0.13 dB body, +0.04 dB presence, −0.56 dB sibilants and −0.78 dB air; no broad high-frequency-collapse guard triggers. These are descriptors, not estimates of isolated reverb or clarity.

Licensed University of Edinburgh native-48k clean speech from two speakers, about 25 seconds each, was evaluated unchanged and with one declared synthetic reflection pattern without added noise. Gain-invariant room/clean SI-SDR improves **+3.33 and +3.80 dB**. Clean-input and room-input preservation checks pass against both known-clean and untreated-room bases. Official-text ASR introduces no substantive lexical errors after disclosed `colors/colours` equivalence; a room-condition “these”/“you” mistake already exists before the model. Original raw controls remain unchanged. [Control evidence, attribution and matched clips](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/REPORT.md).

Controls have only two speakers and one synthetic room. Published clean speech is not asserted perfectly anechoic. Their positive results did not predict the original-source faint-content suppression. Untreated synthetic room also passes the watchdog, so zero warnings is never reused as proof of dryness. No human control-listening result is claimed; the candidate's subsequent partial listening feedback is recorded above.

The corrected full-source ASR uses cached Whisper small and the exact previous English/four-thread/no-fallback/no-timestamp settings. New candidate differs by 7 words out of the original ASR's 293 (**2.39%**), B by 20 (**6.83%**). All four versions retain the target phrase. These are unverified original-transcript edit proxies, not measured WER or naturalness. Adobe has 11.95% proxy difference yet is the user's clear listening winner; its edits and recognition differences demonstrate why this metric cannot rank quality. An earlier timestamp-enabled analysis is retained separately; the exact-settings comparison is [asr-prior-settings.json](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/comparison/asr-prior-settings.json).

## Reproducibility and next gate

Python 3.11.15 and cached Torch/Torchaudio 2.1.1, DeepFilterNet/lib 0.5.6, NumPy 1.26.2, SciPy 1.11.4 and SoundFile 0.12.1 are pinned. A separate interpreter/cache references existing packages without installing or upgrading them. It is not a fully copied environment. Model inference and tests run with OS network denial and original-runtime write denial. Resolved inference source/native-library hashes and the interpreter identity are retained; this is not a complete wheel hash claim.

**42 Python tests passed**, including both real pinned-model tests. All four protected reference hashes, model/config/licences, clean-source hashes, raw controls, runtime snapshot files and saved trial stems match their recorded identities. Adapter AST, JSON validity and diff whitespace were checked. [Verification commands and scope](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/VERIFICATION.md).

The remaining gate is the user's matched listening result: material active-speech dryness improvement over B with acceptable Finnegan identity, consonants, breaths, quiet words and dynamics. A useful listening result still needs a separate integration/promotion instruction. Otherwise close this experiment honestly; technical QA and paired control scores alone cannot settle the remaining Adobe gap.
