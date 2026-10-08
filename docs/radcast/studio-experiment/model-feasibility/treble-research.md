# Treble ISM checkpoint qualification

9 October 2026. Research only: no checkpoint bytes downloaded, no packages installed, no model loaded, no audio rendered, and no application/source changes.

**Conclusion: qualified for one isolated feasibility evaluation, not qualified for deployment or an audible-dryness claim.** The available ISM checkpoint has inspectable joint denoising/dereverberation training and paired clean/reverberant evaluation evidence. It is a native 48 kHz, single-channel, non-generative filtering model. Its actual behavior, complete weight loading in the selected runtime, resource cost and clean-input preservation remain untested.

## Immutable author evidence

Author repository revision: `8e7998acd7e2beb2776fdcc5231731891383bbc7` (8 May 2026). [Pinned repository](https://github.com/TrebleTechnologies/iwaenc2026milo/tree/8e7998acd7e2beb2776fdcc5231731891383bbc7).

The [`ISM-eng-120` configuration](https://github.com/TrebleTechnologies/iwaenc2026milo/blob/8e7998acd7e2beb2776fdcc5231731891383bbc7/evaluation/models/ISM-eng-120/config.ini) specifies DeepFilterNet3, 48,000 Hz, FFT 960/hop 480, 32 ERB bands, 96 complex-filter bins, order 5 and two-frame lookahead. Reverb probability is 1.0; bandwidth extension, clipping, air absorption, zeroing and interfering speech augmentation probabilities are zero. Postfilter is disabled.

The public best-validation artifact is **epoch 119**, not epoch 120:

| Artifact | Git blob identity | Git API size |
|---|---|---:|
| `checkpoints/model_119.ckpt.best` | `3d0167bbc893f33f1358d2047ad5a2adedb8d8f4` | 8,715,794 bytes |
| `checkpoints/model_120.ckpt` | `7a11b1d1d66e5cddade92b91dddfa19fea368ab0` | 8,715,194 bytes |
| `config.ini` | `a1fc064626d46fb94a32e4f41de85388cf6776d6` | 2,144 bytes |

[Pinned Git tree metadata](https://api.github.com/repos/TrebleTechnologies/iwaenc2026milo/git/trees/8e7998acd7e2beb2776fdcc5231731891383bbc7?recursive=1). These are Git blob identities, **not downloaded-file SHA256 hashes**. Binary tensor contents were not inspected. A later authorized runtime must download exactly one selected checkpoint, compute its SHA256 and constrain loading to that artifact.

## Training target and evidence limits

The [authors' paper, version 1](https://arxiv.org/html/2608.20971v1), describes DF3 joint denoising/dereverberation with 48 kHz training, a dry-component factor of .3, shortened target decay of .05 s and a 5 ms late-reflection offset. These define a partially dereverberated target, not simply unchanged clean-reverberant speech and not a guaranteed anechoic output. Its paired tests use unseen speech, measured room responses and noise. Available ISM results improve objective metrics; the stronger Hybrid results belong to unreleased checkpoints and cannot be promised for ISM. Perceptual listening is acknowledged as future work.

The [pinned evaluation code](https://github.com/TrebleTechnologies/iwaenc2026milo/blob/8e7998acd7e2beb2776fdcc5231731891383bbc7/evaluation/eval_objective.py) convolves dry speech and noise with measured RIRs, retains dry speech separately, runs DF enhancement and compares enhanced audio with dry speech. Noise removal can contribute to its improvements, so this is not proof of room-only benefit on Finnegan.

The upstream augmentation implementation explicitly constructs a shortened-tail speech target, optionally mixes original dry speech into it, and uses the more-reverberant speech for the noisy mixture. [Audited upstream augmentation code](https://github.com/Rikorose/DeepFilterNet/blob/d375b2d8309e0935d165700c91da9de862a99c31/libDF/src/augmentations.rs#L973).

**Reproducibility caveat:** the author README identifies training version `0.5.7pre`/main but does not pin its DeepFilterNet commit. The released INI does not encode the paper's target-decay/offset environment settings. The audited current upstream [dataset defaults](https://github.com/Rikorose/DeepFilterNet/blob/d375b2d8309e0935d165700c91da9de862a99c31/libDF/src/dataset.rs#L715) differ from those paper settings. Thus the exact historical target parameters are author-reported, not independently reconstructed from the released INI. The [decay-disable patch](https://github.com/TrebleTechnologies/iwaenc2026milo/blob/8e7998acd7e2beb2776fdcc5231731891383bbc7/training/disable_late_suppression.patch) modifies stochastic input-RIR decay augmentation; it does not remove the separate shortened-target construction.

## Output construction and runtime

[Upstream DF3 inference](https://github.com/Rikorose/DeepFilterNet/blob/d375b2d8309e0935d165700c91da9de862a99c31/DeepFilterNet/df/deepfilternet3.py#L430) predicts ERB gains and complex multi-frame coefficients, applies them to the observed spectrum, then synthesizes audio. No vocoder, diffusion or bandwidth synthesis is involved in this path. Deterministic repeatability still needs actual runtime tests.

The author evaluator supports CPU with `--no_gpu`; CUDA is not an inference requirement. Its broad requirements include ASR and metric tooling unnecessary for a minimal DF adapter. Native DF `enhance(..., pad=True)` compensates STFT delay and promises input-length output; this must be verified on long/non-aligned inputs. The evaluator itself downsamples saved enhanced examples to 16 kHz and writes ordinary PCM WAV; **do not reuse that export path** for the lossless 48 kHz experiment.

Installed RADcast uses DF/deepfilterlib 0.5.6, while training states 0.5.7pre. The architecture/config look compatible, but no binary loading test was authorized here. Both audited upstream and installed checkpoint loaders permit non-strict loading and can discard size-mismatched tensors. An isolated evaluation must reject unexplained missing, unexpected or mismatched learned keys; successful initialization alone does not establish compatibility. Preserve the installed environment and use a separately pinned runtime if necessary. CPU wall time, RAM and Apple Silicon performance for these weights are unknown.

## Licensing and next gate

The [pinned root licence](https://github.com/TrebleTechnologies/iwaenc2026milo/blob/8e7998acd7e2beb2776fdcc5231731891383bbc7/LICENSE) is MIT, copyright Treble Technologies 2026. The checkpoints are included in that repository; no separate checkpoint restriction or weight-specific licence declaration was found. This supports an MIT repository-distribution interpretation, not an independent training-data-rights audit. Preserve the licence and [third-party notices](https://github.com/TrebleTechnologies/iwaenc2026milo/blob/8e7998acd7e2beb2776fdcc5231731891383bbc7/THIRD_PARTY_LICENSES.md); evaluation datasets have their own terms and are unnecessary for fetching the one checkpoint.

A justified next specification is one joint DF3 ISM checkpoint replacing both existing WPE and DF cleanup, with full-band float output, explicit artifact/runtime identity and strict load audit. Proceed only under the parent plan's isolated-runtime/control gate. Require natural-speech paired controls, clean-input preservation, actual CPU cost and matched listening against B/Adobe before any integration. This research does not establish Adobe equivalence or approval for promotion.
