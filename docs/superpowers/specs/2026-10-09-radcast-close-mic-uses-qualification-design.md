# Close-microphone gap: USES qualification design

Status: proposed next experiment, following the user's 9 October feedback. Read-only architecture research and phrase audit are complete. No new model, dependency, inference, or processing candidate has been created for this design. Existing experiment budgets remain closed.

## Problem and success criterion

The user is assessing “in the weeks forward”: the adaptive recording sounds slightly more forward, while Adobe remains clearer and sounds immediately next to the listener. The earlier “reading anything … criminal law … more simple” listening criterion is a separate phrase and must not substitute for this feedback.

The proposed success criterion is materially reduced perceived microphone distance on that phrase, with natural Finnegan identity and intact consonants, words, quiet speech and breaths. Objective guards cannot establish this perceptual result. The current full adaptive version has unresolved ASR intelligibility flags and is not a human-approved full baseline.

## Evidence motivating a different cleanup block

The adaptive cleanup has zero original blend through the contextual 19.5–22.5 second excerpt. Therefore a further blend reduction cannot change the cleanup there. Fixed downstream tone, leveling and light compression remain possible contributors; no evidence isolates them as the dominant cause.

The loaded Treble/DF3 configuration and source predict complex filtering in the first 96 frequency bins, approximately below 4.8 kHz; higher bins use a real magnitude mask. Its paper describes a partially dereverberated target, not guaranteed anechoic speech. This provides a plausible architectural/training limitation for overlapping room cues. It does not measure the Finnegan room response, establish the precise reflection times, or reveal Adobe's method. Recurrent model context also means the five filter frames are not a simple maximum cancellable echo duration.

The nominal ASR phrase interval is 19.980–21.560 seconds, with no DTW alignment. These are approximate decoder boundaries. The existing contextual listening clips cover 19.5–22.5 seconds, with Adobe at 17.261–20.261 seconds using the verified −2.239 offset. Local energy-envelope alignment supports that offset. The old 20-second onset has no preceding context, but there is no evidence of audible onset clipping.

## Options

| Approach | Assessment |
| --- | --- |
| Continue the current blend/EQ path | Useful preservation improvement already demonstrated; it cannot alter the unblended cleanup in the key phrase. No new arbitrary tonal candidate is proposed. |
| Qualify ESPnet USES with explicit dereverberation | Recommended. Released compact checkpoint, full-band complex prediction, an explicit room-removal memory group, and sample-rate-independent processing. These are materially different capabilities, with local performance and voice preservation still untested. |
| Diffusion/vocoder restoration | A separate rescue design would be needed. It raises larger identity/articulation uncertainty and is not the proposed Studio replacement or default. |

## Pinned lead and important limitations

Model: `espnet/Wangyou_Zhang_universal_train_enh_uses_refch0_2mem_raw`.

- Publisher revision: `927a9ecea245120a6f2d88c2552864b937ec5ab9`.
- Actual checkpoint: `exp/enh_train_enh_uses_refch0_2mem_raw/20epoch.pth`, 12,279,888 bytes.
- Publisher LFS-declared SHA-256: `6b2a0c78b2eea566fcfd39b0d77ffc0102b0baf6af60a4a1f4dee93995081121`. This is metadata, not a locally verified binary hash yet.
- `meta.yaml` selects that checkpoint. The 11-byte `valid.loss.*` files are alias text, not checkpoint binaries.
- Publisher weights licence: CC BY 4.0; ESPnet code licence: Apache 2.0. Preserve attribution and the exact source licences during qualification.
- Audited ESPnet source revision: `bfc13cecfd0a07ed8e21d733b0ce130a1c69211a`.
- Saved configuration: two memory groups, one speaker; FFT 256 / hop 128 at default 8 kHz. Explicit 48 kHz encoder/decoder operation scales those to FFT 1536 / hop 768; no 16 kHz intermediate is proposed.
- Call the separator explicitly with `additional={"mode":"dereverb"}` or a verified equivalent. The default is `no_dereverb`; the saved `_r` categories also differ from the current public wrapper's `_reverb` convention. Do not infer successful room removal from a generic quickstart.

This is direct learned complex-spectrum prediction, without a diffusion sampler or vocoder. That distinction does not guarantee unchanged voice or phase. The training corpus is mixed: author recipe maps WHAMR room targets to anechoic speech, while REVERB uses an early-reflection target. Historical training Git identity is absent from the model card. Published 48 kHz VoiceBank results establish denoising, not demonstrated mono 48 kHz dereverberation on lecture speech. CPU code paths exist; Apple Silicon compatibility, speed, memory and strict checkpoint loading remain untested. ESPnet and einops are absent from the current experiment runtime.

Primary sources: [publisher model card](https://huggingface.co/espnet/Wangyou_Zhang_universal_train_enh_uses_refch0_2mem_raw), [pinned separator](https://github.com/espnet/espnet/blob/bfc13cecfd0a07ed8e21d733b0ce130a1c69211a/espnet2/enh/separator/uses_separator.py), [author paper](https://audiocc.sjtu.edu.cn/user/pages/05.members/wangyou.zhang/publications/Toward%40Universal%40Speech%40Enhancement%40For%40Diverse%40Input%40Conditions/paper.pdf).

## Bounded experiment

1. Pin the released metadata, model/config/source identities and licences. Acquire only the single actual checkpoint and audited required sources in a new experiment area. Keep the current RADcast environment and all saved audio intact. Use an isolated pinned CPU runtime; no GPU or paid service assumption.
2. Require strict matching of every checkpoint tensor and shape, explicit `dereverb` execution, native 48 kHz FFT/hop and unchanged output length. A small CPU smoke test must record wall time and memory before any full-recording work. Stop on missing tensors, incompatible sample-rate handling, unavailable runtime or impractical local resource use. Do not silently resample, fall back to denoising, or substitute another model.
3. Run the two existing licensed clean speakers and separate known-direct controls for early reflections and late tails. Derive the two room conditions by splitting the existing pinned synthetic RIR construction, retaining the direct path and coefficients. Keep the existing composite control as context. Check clean-input preservation with the unchanged waveform watchdog, paired clean-reference measures, and matched voice/consonant listening examples. Require improvement over the current model on the early-reflection controls rather than treating quiet gaps or a composite score as proof of proximity.
4. If qualification passes, generate one whole original-derived lossless cleanup trial, with the cleanup block as the only architectural change. Use unchanged pause/tone/level/mastering policy for comparison, retain unmastered cleanup separately, and record any recalculated gain curves explicitly. Do not cascade USES onto an enhanced delivery file or bake Adobe reference audio into processing. The contextual phrase and all preservation material must come from the single full run, not a phrase-specific model run.
5. Apply the unchanged original and baseline waveform guards, final-export check and same-settings cached ASR. Name substantive wording changes, and compare the opening, faint sections, ending and charged/convicted explanation. Reject a failed candidate; no automatic repair or extra parameter matrix is included in this round.
6. Present speech-level-matched USES, current adaptive and Adobe excerpts of 19.5–22.5 seconds, plus their unmastered diagnostic counterparts where available. Report actual RMS/LUFS differences. Human judgment of closer voice, natural identity and articulation is the final listening gate. A technical pass does not promote a native preset or change the default.

Budget: one model family, one qualification configuration, and at most one full Finnegan candidate after qualification. If the model fails the prerequisite gates or offers no material perceived benefit, retain the evidence and close this round. Do not consume the previous rounds' exhausted budgets or claim that more assistant reasoning changes the capability of the speech model itself.
