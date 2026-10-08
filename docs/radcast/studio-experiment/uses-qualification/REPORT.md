# USES qualification: rejected before a Finnegan trial

The verified legacy USES checkpoint runs locally at48kHz, but fails the predeclared room-control gate. All six synthetic-room cases give a worse match to their paired clean reference than untreated input. No new Finnegan audio was generated, and the microphone-distance gap with Adobe remains unresolved.

## USES2 correction

The user correctly pointed out that ESPnet includes USES2-Comp and USES2-Swin. The author paper is from2024; implementation entered ESPnet in2025. The bounded release audit established code and recipes, but no provenance-established pretrained binary. The user also found no official checkpoint and required any future weights to be verified separately. [Release audit](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/USES2-RELEASE-AUDIT.md).

The downloaded and tested model is explicitly **legacy USES**, not USES2. Its publisher revision is927a9ecea245120a6f2d88c2552864b937ec5ab9;12,279,888byte20epoch.pth matches publisherSHA2566b2a0c78b2eea566fcfd39b0d77ffc0102b0baf6af60a4a1f4dee93995081121. Weights are publisher-labelledCCBY4.0; code isApache2.0 with preserved attribution. [Pinned artifact inventory](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/artifact-manifest.json).

## What the local integration establishes

All261 tensors and3,052,492 elements load strictly. Actual dereverberation selects memory group1. Processing uses full-band native48k complex spectra, FFT1536/hop768/769bins, input variance restored once, and no output peak normalization, resampling, model cascade or original blend.

The unmodified full-utterance feature allocation would be unsafe on this16GB Mac. A bounded-intermediate execution path keeps one continuous STFT, the original64-frame schedule and20-frame memory, and exact convolution neighbors. It does not independently enhance audio chunks or crossfade them. Eleven published class/function ASTs remain unchanged. Separate-process actual trained2.2second comparisons spanning three segments agreed within maximumPCMerror1.0431×10⁻⁷ andRMS1.4263×10⁻⁸, below fixed3×10⁻⁶/3×10⁻⁷ limits. Bounded control processing peaked at2.805GB. These validate this execution path; they do not establish voice quality. [Numerical evidence](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/bounded-equivalence.json).

## Controlled room result

The initial set was fixed before inference: two complete licensed sentences from each of two speakers, including “Six spoons of fresh snow peas…”. Original full source/stage hashes are verified before selecting prefixes. Each speaker receives identical direct speech plus either the existing14/37ms reflections, the existing50ms-onset late tail, or the existing combined response. Headroom gain is common across the four conditions. These are synthetic added-room controls, not measurements of Finnegan's room.

| Speaker / condition | Untreated SI-SDR | USES SI-SDR | Change |
| --- | ---: | ---: | ---: |
| p232 / early |11.12dB|7.48dB|−3.64dB|
| p232 / late |10.48dB|8.82dB|−1.66dB|
| p232 / combined |7.88dB|7.03dB|−0.85dB|
| p257 / early |11.68dB|6.63dB|−5.06dB|
| p257 / late |9.99dB|7.44dB|−2.55dB|
| p257 / combined |7.79dB|6.17dB|−1.62dB|

The prerequisite was at least+1dB paired benefit for both early and late conditions. It fails for both voices. Broad clean-input waveform guards returned no warnings; clean-reference SI-SDR was9.66/8.41dB. A zero watchdog count therefore does not establish a good cleanup result. SI-SDR includes all waveform error, including phase and timbre differences; it is not percentage echo removal, speaker identity, or perceived microphone distance. The assistant has not listened to or claimed to hear these controls.

The four fresh Treble comparison calls were conditional on passing the prerequisites and were correctly skipped. Their absence is recorded as an incomplete final comparison, caused by the earlier room-gate rejection; no head-to-head improvement over Treble is claimed. All eight planned USES cases completed, with finite native48k float outputs, exact sample counts and no processing/timing failures. [Full results and event log](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/controls/controls.json).

Optional diagnostic listening uses different calibration voices, not Finnegan:

- [p232 original clean sentences](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/controls/p232_clean_input.wav) and [USES output](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/controls/p232_clean_uses_output.wav).
- [p257 original clean sentences](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/controls/p257_clean_input.wav) and [USES output](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/controls/p257_clean_uses_output.wav).

These are unmastered saved stages and have different levels. They are evidence files, not matched-loudness quality comparisons or delivery candidates.

## Decision and remaining direction

Close this qualification round without threshold changes or a Finnegan render. It remains plausible that a different model trained explicitly for native48k dereverberation could perform better, but this trial does not establish that. Historical training provenance is incomplete and released48k VoiceBank denoising results do not certify mono48k room-removal transfer in this path.

The [official URGENT2025 TFGridNetV3 baseline](https://huggingface.co/kohei0209/tfgridnet_urgent25) is a publicly released checkpoint with verified publisher metadata and48k reverberant training coverage; its published training preparation nevertheless retains roughly50ms of early response. It is not a guaranteed booth/dry-target solution, and no weights or candidates for it were created here. USES2 would require provenance-established weights or a separately scoped training project. Stronger assistant reasoning cannot replace either missing trained weights or a matching training target.

The user's key phrase remains “in the weeks forward”, nominally19.98–21.56s from approximate non-DTW ASR. Existing contextual [adaptive](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-rms/quiet_adaptive.wav) and [Adobe](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/adaptive-preservation-repair/listening-rms/quiet_Adobe.wav) copies cover original19.5–22.5s with verifiedAdobe−2.239s alignment. No actual word-onset clipping was established. The prior “reading anything…” phrase and its ASR-presence flag are a different criterion.

## Verification and scope

Fresh complete Python suite: **106passed, no skips,24.124seconds**, including real checkpoint tests. The first106-test run retained one test-fixture failure: an absence-of-proof test assumed the actual experiment still lacked its now-completed proof. It was corrected to use isolated temporary artifacts, without changing model code or numerical evidence. Both logs are retained.

Independent checks reproduce all control inputs exactly, recompute their paired scores, verify all source/output hashes, match all eleven vendor ASTs, and rehash34 original-runtime files, four protected audio references and unrelated tracked changes. Everything protected remains unchanged. [Verification](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/verification.json), [test log](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/uses-qualification/final-tests.log), [plan](/Users/rcd58/Documents/RADsuite/docs/superpowers/plans/2026-10-09-radcast-uses-qualification.md).

Eight qualification calls and three setup/equivalence calls used the learned USES checkpoint. Each of two complete regression-suite runs also made two learned USES test calls, separate from the qualification. No candidate inference, full Finnegan render, final MP3 export, app/default change, commit or promotion occurred. Runtime references existing packages read-only; no package installation or upgrade was performed. Code cross-review and direct artifact verification were completed; the first design-review agent's credit failure is retained in the work log rather than presented as a successful review.
