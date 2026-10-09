# Remaining microphone-distance investigation — 9 October 2026

The user's latest listening feedback is that Adobe sounds close to the microphone while the local versions still sound farther away in a large room. The tonal probe has therefore not achieved the proximity target. Identity and articulation have not received explicit approval. [Exact feedback](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/distance-investigation/human-listening-result.json).

**The evidence does not support the 20% original blend as the dominant cause of the remaining gap.** Its removal makes a small gain-normalized change in foreground passages. The current cleanup model is the next component to investigate. This is a working diagnosis, not proof of isolated room energy or an explanation of Adobe's proprietary algorithm. Human listening must determine whether the small change matters perceptually.

## Isolated listening pair

These are the saved model and blended cleanup stages, before pause control, EQ, adaptive speech leveling, compression and final mastering. They are diagnostic copies, not new full candidates. Both retain original 20–32 seconds and measure **−26.61 LUFS**, with static audition gains only. The unblended full recording remains rejected for suppressing faint protected material; these foreground excerpts do not override that rejection.

- [Model alone](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/distance-investigation/listening-lufs/target_phrase_model_unblended.wav)
- [Same model plus 20% original](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/distance-investigation/listening-lufs/target_phrase_original20.wav)

Use “reading anything to do with the criminal law a lot more simple” to assess whether the blend brings back a meaningful impression of distance. Other quiet, loud and phrase-ending controls are included in the [analysis manifest](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/distance-investigation/analysis.json). Adobe is omitted from this pair to isolate one variable; the prior verified Adobe listening comparison remains available in the [presence report](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/presence-probe/REPORT.md).

## What the saved stages establish

The recorded blended cleanup exactly equals 0.8 × model + 0.2 × highpassed original: **maximum sample error 0**. Both full runs produced the same raw model hash. This checks that the original blend is being measured without a model, configuration or downstream change.

For each region, fit one scalar to the unblended waveform using identical original-derived protected speech frames. Measure the remainder of the blended waveform relative to that scaled waveform:

| Original timeline | Correlation | Difference RMS after matching scalar |
|---|---:|---:|
| Target 20–32 s | 0.99912 | 4.20% / −27.53 dB |
| Exact words 23.47–28.38 s | 0.99929 | 3.78% / −28.45 dB |
| Quiet 19.5–22.5 s | 0.99903 | 4.42% / −27.10 dB |
| Loud 70–80 s | 0.99944 | 3.34% / −29.52 dB |
| Ending 38.5–41.8 s | 0.99939 | 3.48% / −29.17 dB |

These percentages describe **all remaining waveform differences**, including room, voice, noise and phase. They are neither percentage echo nor an estimate of direct-to-reverberant ratio. A perceptually important reflection can have low energy. Target blend spectral-share changes are small: body +0.125 dB, presence +0.065 dB, sibilants −0.003 dB and air +0.032 dB. Raw model versus original itself retains high gain-normalized correlation (0.99097); this also cannot identify which component remains.

On the two existing paired clean/synthetic-room controls, analytically adding the same 20% room input decreases clean-reference SI-SDR from 11.11 to 10.71 dB and from 11.35 to 11.00 dB: losses of **0.40 and 0.36 dB**. No model is rerun. The blend does reduce this model's measured benefit, but modestly on these controls. Two speakers and one synthetic reflection pattern cannot predict Finnegan's perceived distance. SI-SDR includes all errors and is not a pure reverb measure.

The existing faint-content failure still matters. Unblended output suppressed protected source windows by as much as 27.81 dB relative to typical raw gain. Removing the preservation blend globally would reopen that failure. An adaptive blend might reduce this trade-off, but these results do not justify presenting it as the solution to the larger proximity gap.

## Why the cleanup target deserves attention

The current model's paper specifies a shortened 0.05 s target decay with a 5 ms late offset; audited upstream training code retains a partially dereverberated component plus dry speech. Historical training settings are incompletely pinned. This is compatible with residual room response, without proving that it causes this recording's distance impression. [Current model paper](https://arxiv.org/html/2608.20971v1), [prior qualification](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/QUALIFICATION.md).

A separate study found that training toward aligned anechoic speech improved results over early-reflection targets in its experiments. This supports testing a genuinely dry target, without establishing Adobe's method or predicting success on Finnegan. [Primary research](https://arxiv.org/html/2603.02641v2).

Two new released-model leads were checked, with ten source files pinned and saved, no weights downloaded:

- **NVIDIA RE-USE:** the card identifies a GAN-trained release; inspected source predicts magnitude and phase. The stock inference explicitly rejects CPU and requires NVIDIA/CUDA dependencies. The card declares research/development use and a noncommercial licence. It is not a drop-in Apple Silicon preservation model. A regression-only checkpoint or supported bypass was not established. [Publisher card](https://huggingface.co/nvidia/RE-USE), [pinned inference](https://huggingface.co/nvidia/RE-USE/blob/022e920d727347a64d6c21fbf0628f2a5f37ad78/inference.py).
- **SGMSE+ EARS-Reverb:** a native-48k dereverberation checkpoint is published, but it uses diffusion-based generative reconstruction. It would require a separate experimental restoration design and voice-preservation evaluation. No runtime, checkpoint load or Apple Silicon performance was tested. [Author repository](https://github.com/sp-uhh/sgmse), [author dataset and demonstrations](https://sp-uhh.github.io/ears_dataset/).

Neither inspected lead qualifies as a straightforward replacement under the current Studio design. This limited audit does not establish that no suitable model exists. [Pinned source inventory and decisions](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/distance-investigation/research/qualification.json).

## Decision and verification

Close the tonal probe with the proximity gap still open. Do not add more brightness or silently remove preservation globally. First use the isolated pair to determine whether removing the blend produces an audible benefit. If it does not, the next substantive experiment should replace the room-removal model with one trained toward genuinely dry speech; it must satisfy native-rate, licence, local-runtime and identity/articulation checks before an original-derived trial. Generative reconstruction would be a distinct rescue experiment, never an automatic promotion into Studio.

This investigation adds **zero full candidates and zero model inferences**. Ten static diagnostic WAVs were generated from verified original-derived processing stems. Four protected references and all consumed stage/control hashes were rechecked. Native/application presets, dependencies and historical audio remain unchanged. The first low-volume diagnostic export stopped because gated loudness matching differed by 0.04 LU; its copies are retained in `diagnostic-attempt1`. The successful copies use comfortable static audition gain, exact speech-RMS matching within 0.00001 dB and independently measured target LUFS equality. [Verification](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/distance-investigation/verification.json).

Reproduce in a fresh evidence directory with the saved [analysis script](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/distance-investigation/analyse_saved.py), using the existing offline experiment runtime. It refuses to overwrite a completed run. No hearing, new full-source preservation approval, identity approval or Adobe parity is claimed.
