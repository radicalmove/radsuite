# Finnegan presence probe — 9 October 2026

**Latest listening result — apparent microphone distance remains:** the user says Adobe sounds close to the microphone while the local versions still sound farther away in a large room. This closes the one-render tonal probe without establishing proximity, identity or articulation approval. The new [saved-stage blend investigation](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/distance-investigation/REPORT.md) points toward the cleanup model as the next component to investigate; it adds no full candidate. [Exact feedback](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/human-listening-result.json).

**One bounded tonal candidate is ready for listening.** It passes unchanged preservation checks against both the original and the previous model trial. No human presence, articulation or identity approval is implied. The existing RADcast app and presets are unchanged; no new model or dependency was added, and no commit/push/default change occurred.

The user reports improved local attempts but still prefers Adobe's voice presence. This round tests one small combined tonal correction while fixing the original-derived cleanup and leveling. It does not extend the exhausted two-render room-model experiment or launch an EQ matrix. [Approved execution plan](/Users/rcd58/Documents/RADsuite/docs/superpowers/plans/2026-10-09-radcast-presence-probe.md).

## Listen to the controlled target comparison

These 12-second clips contain original 20–32 seconds / Adobe 17.761–29.761 seconds, cut after full-file processing. Only downward constant audition gains are added. The primary set is approximately LUFS matched: previous −26.57, presence −26.57, Adobe −26.58 LUFS. LUFS equality is not proof of identical subjective loudness.

| Version | Target clip |
|---|---|
| Presence probe | [Listen](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-lufs/target_phrase_presence.wav) |
| Previous accepted-for-listening model/dry20 trial | [Listen](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-lufs/target_phrase_previous.wav) |
| Adobe reference | [Listen](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-lufs/target_phrase_Adobe.wav) |

Listen for a closer, fuller voice and clearer word edges, while checking normal S/SH/T/CH/F, familiar speaker identity, sharpness, excess body and any room sound being made more prominent. This tonal probe cannot establish further echo removal: the upstream cleanup is unchanged.

[Full float WAV](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/CRJU160_presence_body1_definition2_detail1p5.wav) · [Final 320 kbps MP3](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/CRJU160_presence_body1_definition2_detail1p5.mp3).

The following controls use identical original-derived speech masks and matched speech RMS. Their LUFS can differ, especially at phrase endings; do not assume every passage is perceptually level-matched.

| Passage, original timeline | Previous | Presence | Adobe |
|---|---|---|---|
| Quiet articulation, 19.5–22.5 s | [Before](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/quiet_articulation_previous.wav) | [Probe](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/quiet_articulation_presence.wav) | [Reference](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/quiet_articulation_Adobe.wav) |
| Louder speech, 70–80 s | [Before](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/louder_passage_previous.wav) | [Probe](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/louder_passage_presence.wav) | [Reference](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/louder_passage_Adobe.wav) |
| Phrase ending, 38.5–41.8 s | [Before](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/phrase_ending_previous.wav) | [Probe](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/phrase_ending_presence.wav) | [Reference](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-rms/phrase_ending_Adobe.wav) |

## Opening-phrase listening flag

Same-settings Whisper small changes more words after the probe: 14 edits versus the original-ASR proxy (4.78%), compared with 7 (2.39%) before. Differences include fillers and skipping “so some more criminal justice terminology,” which the before-probe recognizer retains. The target “reading anything … criminal law … a lot more simple” and core charged/convicted/prosecution/defendants/innocent terms remain recognized.

This does not prove erased speech or a lisp. Exact sample counts, timing and both preservation guards pass; the affected introductory interval's overall RMS changes only about −0.31 dB. Energy retention also does not prove intact words. Check it directly:

| Opening context, 0–12 seconds | Before | Presence |
|---|---|---|
| Same source masks and static speech-RMS matching | [Listen before](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-intro/intro_previous.wav) | [Listen after](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/listening-intro/intro_presence.wav) |

Adobe is omitted from this whole original handle because equivalent coverage is not verified. No padding or replacement reference was used. ASR has no human-verified Finnegan transcript here; edit differences are not WER, naturalness, identity or quality rankings. The human-preferred Adobe likewise differed more from the original ASR in the previous round. [Actual recognition and self-review](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/asr-comparison.json) · [word changes](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/asr-self-review.json).

## Exact pipeline and fixed variable

The pipeline still derives from the original protected MP3, decoded once to 48 kHz float in the accepted room-model experiment: HP45 → pinned ISM best119 joint room/noise model → 80% model / 20% highpassed original → guarded pauses → baseline +2 dB/2.4 kHz presence control → original-derived protected speech leveling. This probe resumes its verified lossless `levelled.wav` processing stage; it does not process a delivered enhanced MP3/WAV as its input.

The exact existing compressor is applied once to that precompression stem: threshold .18, ratio 1.15, attack 20 ms, release 220 ms, makeup 1. Reapplying the baseline's recorded scalar +6.14 dB to this compressed signal reproduces the previous float master **exactly, maximum absolute error 0**. The previous gain is used for this verification only, not summed into the new output.

Then apply these extra bell filters and final constant gain:

| Extra bell | Q | Gain |
|---|---:|---:|
| 180 Hz body | 0.8 | +1.0 dB |
| 3,000 Hz definition | 0.8 | +2.0 dB |
| 5,200 Hz consonant detail | 1.0 | +1.5 dB |

The EXTRA response peaks at **2.76 dB** and stays below **0.33 dB below 80 Hz**. No fixed low-pass, shelf, de-essing, exciter, synthesis, dynamic EQ or additional compressor is used. The nominal sum with the existing baseline presence filter peaks at 4.38 dB; that is not an exact complete-chain transfer function because model/dynamics/compression intervene. Full extra/nominal response arrays are in [response.json](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/response.json).

Final constant gain is **+5.41 dB**, replacing the baseline's +6.14 dB, a difference of −0.73 dB. The same −20.75 LUFS / −1.5 dBTP policy prioritizes peak headroom; no limiter, time stretching or new adaptive gain is introduced. Only the final MP3 is lossy. [Complete provenance, stages, gains, paths and hashes](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/trial.qa.json).

This is a fixed calibration correction, not production adaptive EQ or an identified Adobe processing recipe. A useful result would support this combined correction on this source, without identifying which bell helped or justifying applying it to every speaker.

## Technical evidence

| Measure | Presence probe |
|---|---:|
| Duration / samples / working rate | 143.490 s / 6,887,520 / mono 48 kHz FLOAT |
| Integrated loudness / true peak | −22.60 LUFS / −1.55 dBTP |
| Loudness range | 7.1 LU |
| Fixed-original-mask speech / pause RMS | −23.08 / −65.42 dBFS |
| Speech/pause contrast | 42.34 dB |
| Measured lag / block drift / clips | 0 / 0 / 0 |
| Locally normalized envelope correlation | 0.9731 |
| Worst relative protected voiced-window gain | −4.93 dB |
| Original-source / baseline watchdog warnings | 0 / 0 |

Final MP3 independently passes export QA with the same duration, loudness and true peak. Compared with the original, full-file speech-frame band shares change by −0.57 dB body, +2.69 dB presence, +1.83 dB sibilants and +0.74 dB air; no collapse, excessive bass, drift, pause or clipping guard triggers. Against the previous trial, the target phrase adds only about +0.36 dB body share, +1.45 dB presence and +1.88 dB sibilants. Adobe still has roughly +1.96 dB body, +2.98 dB presence and +3.15 dB sibilant share relative to this probe in that phrase. In the louder passage these relationships differ substantially. These are spectral descriptors, not isolated-room estimates, EQ inversion gains or a quality ranking.

All **50 Python tests pass**, including 8 new signal/provenance tests and both real cached-model tests. New tests were first observed failing for the missing implementation, then passed. Native/UI code did not change, so prior native/UI results are not claimed as fresh tests of this probe. Focused review found no material blocker to presenting the candidate with the explicit intro listening flag. Model inference and new filters are never run in the same new render; the full suite's real-model tests are separate contract checks.

The cached runtime remains unchanged: no model/package/download/licence change. Existing room-model/checkpoint licensing and runtime qualification remain [documented](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/REPORT.md). Rendering, tests, comparisons and ASR run under existing OS network denial and original-runtime write denial. Reference/model/runtime/code identities, FFmpeg and Whisper identities, comparison gain/count/format checks and test totals are recorded in [verification.json](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/verification.json).

The next decision is human: does this sound fuller and more immediate without sharp consonants, excess body, more audible room, lost quiet words or changed speaker identity? If benefit is weak or articulation suffers, retain the previous trial and close this probe. No strength escalation, new model or app/default integration is authorized by a numerical improvement.
