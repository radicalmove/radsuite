# Adaptive original preservation: full-length listening trial

The user's preference for `target_phrase_model_unblended.wav` supports keeping the cleaner model output through ordinary speech. This trial follows that direction: 91.92% of samples use no original blend at the cleanup stage. Five short recovery areas add up to 20% high-passed original audio where the model strongly attenuates protected material. This is a listening candidate; the full recording has not received human approval.

User feedback: “target_phrase_model_unblended.wav sounds better, a bit cleaner and sounds slightly closer to the mic”. This approves the direction of the target excerpt, not this full candidate, identity, articulation, or Adobe parity. The assistant has not independently listened to or claimed to hear these files.

## Listen

The main phrase covers original time 20–32 seconds. Adobe is aligned with the previously verified −2.239 second offset. These copies have static gain only; measured target loudness is within 0.06 LU, rather than exactly equal.

- [New adaptive phrase](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-lufs/target_phrase_adaptive.wav)
- [Previous continuous 20% original blend](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-lufs/target_phrase_old.wav)
- [Adobe exemplar](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-lufs/target_phrase_Adobe.wav)
- [Full candidate MP3](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/CRJU160_adaptive_original_preservation.mp3)
- [Full candidate float WAV](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/CRJU160_adaptive_original_preservation.wav)

Check the opening before choosing a full-length version. Speech recognition omitted “So some more criminal justice terminology” in the new candidate, while recognizing it in the previous candidate. Recognition is not proof that these words are physically absent. The waveform guards pass, so this remains an unresolved listening flag.

- [Previous opening, 0–12 seconds](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/intro_old.wav)
- [New opening, 0–12 seconds](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/intro_adaptive.wav)
- [Original opening, 0–12 seconds](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/intro_original.wav)

Also compare [previous internal faint material](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/internal_old.wav), [new internal faint material](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/internal_adaptive.wav), [previous ending](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/ending_old.wav), and [new ending](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/ending_adaptive.wav). These use one whole-recording gain calibration followed by a common audition gain for each group. Individual faint excerpts are not separately normalized, which would conceal loss. The 2–3.5 second [previous](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/missed_material_old.wav), [new](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/missed_material_adaptive.wav), and [original](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-faint/missed_material_original.wav) copies magnify very faint material; it has not been established to contain intelligible words.

## What changed

The former candidate added original audio continuously. This trial blends it only when protected one-second windows have a model gain at least 9 dB below the median eligible gain. Recovery increases linearly to 20% at a 12 dB relative loss. Window support expands 160 ms on each side and receives a 100 ms smoothing kernel. A post-interpolation frame-energy check bypasses any connected recovery interval that would reduce protected model energy by more than 0.1 dB. No energy bypass was needed here.

The detector uses the original and the preserved pre-EQ cleanup reference independently. Their decisions combine by maximum activation. The preserved reference helps detect faint material missed by the original-only classifier; its audio is never mixed into the output. Only the pinned raw model and high-passed original supply audio. Speech protection is based on periodicity, modulation and energy proxies; room background can trigger protection, and this is not semantic speech recognition or a direct reverb measurement.

Recovery frame intervals are 1.3–3.7, 3.8–5.7, 66.8–68.2, 104.8–106.7, and 137.3–141.2 seconds. Sample interpolation extends support by approximately 10 ms each side. Average original weight across the recording is 1.55%, with a 20% maximum. These percentages describe the blend, not the percentage of echo removed.

At the cleanup stage, the preferred phrase is bit-identical to the raw model. The existing pause policy, +2 dB presence at 2.4 kHz, saved level gain curve, and single light compressor remain fixed. There is no extra EQ from the earlier presence probe. All four old baseline replay stages and all four new downstream replay stages have zero maximum error. The final scalar gain is +6.94 dB, replacing the previous +6.14 dB; the gains are not summed. The saved leveling curve makes this a controlled offline calibration experiment, not a validated production leveling policy.

## Evidence and limits

| Check | Result |
| --- | --- |
| Full WAV | 143.49 seconds; mono 48 kHz float; 6,887,520 samples |
| WAV / final MP3 | −22.60 LUFS; −1.55 dB true peak; no clipped samples |
| Original-source watchdog | No flags; worst protected relative window gain −3.99 dB |
| Previous-candidate watchdog | No flags; worst protected relative window gain −1.50 dB |
| Protected speech envelope shape against original | 0.9536 |
| Timing | No detected global lag or block drift |
| Target listening copies | Previous −26.56, new −26.51, Adobe −26.57 LUFS |
| Fresh complete test suite | 60 passed, no skips; 19.891 seconds |
| Independent artifact audit | 30 audition WAVs replay exactly; 34 pinned runtime files and 4 protected references unchanged |

Two saved speakers, each clean and in one synthetic room, exercise the same two-reference controller. All four outputs remain bit-identical to their saved raw model because no recovery activates. The decision-only protection reference is analytically derived as 0.8 × saved model + 0.2 × the same input, before downstream processing. [Comparison controls](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/comparison.json) retain the four results, [control input manifest](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls/controls.json) retains the paired input hashes, and [fresh control verification](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/controls-verification.json) checks those hashes and repeats the controller evaluation without inference. These controls establish preservation on this small set, not successful recovery coverage, general dereverberation performance or human naturalness. No new candidate model inference was needed; the enabled model regression test does run real inference.

Same-settings cached speech recognition gives 35 edits against the 293-word original recognition proxy (11.95%), versus 7 for the previous candidate (2.39%). The target phrase is recognized, but the opening is omitted and additional wording changes occur. Some changes are spelling variants; substantive examples include “read the entire” becoming “very much higher”, “against them” becoming “incident”, and omission of the trailing “I've put” recognized in the previous candidate. These are material intelligibility listening flags. The original transcript is not verified ground truth, and this proxy does not rank dryness, identity, lisp, or voice quality. The earlier Adobe proxy also had 35 edits despite the user's strong preference for its sound. [Original-proxy differences](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/asr/differences-against-original.json) and [previous-candidate differences](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/asr/differences-against-model_dry20.json) retain the wording for checking.

Recognition completed once. JSON finalization then failed because a historical report used a different field name. [ASR reporting](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/asr-comparison.json) was recovered from the retained transcript and log after independently verifying the analysis input against the unchanged master. Recognition was not rerun. The 16 kHz copy is analysis-only.

The original [comparison metadata](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/comparison.json) reused an ending key for two different excerpts. The audio was unaffected. The [independent audit](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/verification.json) verifies all three short ending RMS clips as well as the faint ending. Future comparison code uses a distinct key; the original metadata is retained.

## History and listening gate

The first [original-only adaptive trial](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation/REPORT.md) passed the original watchdog but failed the previous-candidate guard on faint 2–3.5 second material, with a −20.88 dB relative loss. It was rejected and retained, without a delivery export. Its exact implementation and test snapshots are preserved.

The separately reviewed [coverage correction](/Users/rcd58/Documents/RADsuite/docs/superpowers/plans/2026-10-09-radcast-adaptive-preservation.md) adds the pre-EQ preservation reference. Activation thresholds, blend cap, smoothing, energy guard, downstream processing and watchdogs were not relaxed. One repaired full candidate was rendered. Across these two rounds there are two attempts, one rejected and one technically accepted; both render budgets are now zero.

[WAV QA](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/trial.qa.json), [MP3 QA](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/export.qa.json), [controller decisions](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/controller.json), [verification](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/verification.json), [test log](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/final-tests.log), and [listening gate](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-gate.json) retain the evidence. The original references, previous candidates, model checkpoint and runtime remain intact. No RADsuite native/UI code, preset default, or model installation was changed in this trial.

Full-length human listening remains pending, particularly the opening, faint material, and the charged/convicted explanation. Production integration and a preferred default remain separate work after this gate. No Adobe algorithm or Adobe parity is claimed.
