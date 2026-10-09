RADcast Studio revision 2 — listening gate

8 October 2026, Pacific/Auckland. This revision follows human rejection of the first Studio candidate as more echoey and less clear. Status: experimental, awaiting listening approval. Existing preset IDs and defaults remain intact; nothing has been committed, pushed or promoted to the default.

The goal is to approach the reference's measured speech presentation while preserving the real lecturer. These are observed quality behaviours, not a recovered or cloned Adobe processing algorithm.

**What changed and why**

The waveform investigation found that the first candidate suppressed quiet gaps effectively but under-presented quiet words and kept a large passage-level imbalance. The reference had substantially more presence/sibilant energy and a smaller level change between quiet and louder passages. See the [initial findings](../waveform-inspection/FINDINGS.md) and [original architecture/diagnosis report](../REPORT.md). The lisp feedback belongs to Optimized, not Natural++.

Revision 2 adds source-preserving room cleanup during active speech using the already installed NARA-WPE package, protects quiet articulation with local recording-level estimates, levels speech only after cleanup, and applies a bounded adaptive presence correction. It continues to avoid generative reconstruction, a fixed low-pass, bass boosting and automatic de-essing. The old modes were not rewritten.

**Exact chain and configuration**

Decode/trim once to 48 kHz mono float32 working PCM → second-order 45 Hz high-pass → source-preserving WPE → pinned DeepFilterNet3 with compensated delay → upper-band preservation blend → guarded pause attenuation → adaptive presence correction → bounded speech leveling → light compression → constant final loudness/true-peak gain → final encode → exported-artifact QA.

WPE uses 12 s chunks, 2 s overlap, 1024-point STFT / 256-sample hop, 10 taps, delay 4, two iterations and PSD context 1. It affects 80–8000 Hz; other bins keep the observed spectrum. Main candidate: 65% WPE estimate / 35% observed spectrum. Alternate: 35% WPE / 65% observed. Overlap fades are normalized; timing and sample count are preserved. No new generative model is used. The active WPE frequency restriction is not a low-pass: upper frequencies bypass WPE and continue through the full-band pipeline.

DeepFilterNet3 remains the same pinned epoch-120 checkpoint/config, DeepFilterNet/deepfilterlib 0.5.6, postfilter off, attenuation limit 18 dB. Its lower-band blend preserves about 12.59% of its input (now the non-generative WPE output), with 87.41% filtered contribution. Extra source-preserving weight ramps from 0 at 3.5 kHz to .75 at 7 kHz, giving about 78.15% preservation of that non-generative input above 7 kHz. Above 8 kHz the WPE input itself is unchanged from the original high-passed signal. The raw/original-versus-WPE blend and the subsequent DF preservation blend are distinct stages.

Pause classification uses overlapping 4 s local energy estimates, 2 s steps, about 300 ms of articulation protection and a 360 ms guarded pause interval. This is a conservative acoustic proxy, not semantic VAD. Maximum pause attenuation is 6 dB, smoothed across seven 20 ms frames. The configuration is explicitly passed to processing and a regression test checks actual interior attenuation.

Presence EQ is a peaking correction at 2.4 kHz, Q .8, capped at 2 dB. A heuristic presence/mid energy ratio selects 0–2 dB: already-present speech is left unchanged; mid-heavy speech may receive the correction. The control threshold is -11 dB presence/mid ratio, not an inferred Adobe EQ setting. Both calibration candidates select 2 dB; that is recorded in their reports. A test confirms that an already-present signal receives no boost. There is no bass lift or blanket upper-air shelf.

Speech leveling uses centered 2 s speech-weighted energy, a geometric midpoint of robust 20th/80th percentile speech levels, a one-second gain smoothing window and +/-8 dB bounds before final mastering gain. Speech support limits its action: it does not raise long inactive gaps. Source/cleanup energy consistency rejects strongly suppressed noise from the level estimate. Centered analysis introduces lookahead but does not shift, stretch or retime samples. Fine phrase/phonetic modulation remains measurable separately from the intended broad level correction.

Compression remains light: threshold .18, ratio 1.15, attack 20 ms, release 220 ms, makeup 1. Final target remains -20.75 LUFS and -1.5 dBTP with .05 dB peak headroom; peak protection takes priority. Here the target LUFS is not reached because of peak headroom. All internal files stay uncompressed float PCM. MP3 is encoded only at final export; analysis-only decoding/resampling is not continued signal processing.

**Dependencies and reproducibility**

No package/model was installed or downloaded. NARA-WPE 0.0.11 was already installed for legacy RADcast. It is MIT licensed; see the [authors' paper](https://groups.uni-paderborn.de/nt/pubs/2018/ITG_2018_Drude_Paper.pdf) and [project](https://github.com/fgnt/nara_wpe). DeepFilterNet's existing licensing, pinned model/config hashes and runtime versions remain documented in the initial report and candidate JSON. NARA-WPE is validated on import/use. The Rust binary embeds the new speech_cleanup.py module alongside the renderer and analysis code, and uses the same FFmpeg/FFprobe resolution as the app.

On Apple M1 Pro with four configured threads, this primary render's WPE time was 2.66 s and DF inference 4.80 s for 143.49 s. Full pipeline time includes Python startup and repeated technical measurements. Fresh-machine installation and Windows performance were not tested. The two candidate WAV file hashes matched on repeat renders. Implementation hashes and complete validation logs are retained in validation.json.

**Fresh comparison on a common analysis basis**

The pause/speech masks changed to protect quiet articulation, so historical contrast values must not be compared as though the method were identical. All four references, the rejected first candidate, and both new candidates were re-measured under the revised harness. Reference timing edits still prevent direct whole-file preservation/noise comparisons; the stable-offset 10–40 s excerpt provides common-content spectral evidence.

| Version | Decoded s | LUFS | dBTP | LRA LU | Speech/pause contrast dB |
|---|---:|---:|---:|---:|---:|
| Original | 143.490 | -18.54 | -1.82 | 19.4 | 35.40 |
| Optimized | 131.998 | -20.29 | -1.41 | 9.6 | 53.68 |
| Natural++ | 131.998 | -21.10 | -1.31 | 9.9 | 33.84 |
| Adobe reference | 135.198 | -25.24 | -4.13 | 7.1 | 41.02 |
| Rejected initial Studio | 143.490 | -20.75 | -2.29 | 19.0 | 56.85 |
| Studio r2, WPE 65% | 143.490 | -21.63 | -1.55 | 7.4 | 44.22 |
| Studio r2, WPE 35% | 143.490 | -21.65 | -1.55 | 7.5 | 44.22 |

Matched speech passage levels, same original-derived masks for each version. These differ numerically from the initial report because the mask now retains more quiet articulation.

| Version | 10–40 s speech dBFS | 70–80 s speech dBFS | Difference dB |
|---|---:|---:|---:|
| Original | -32.57 | -15.15 | 17.42 |
| Rejected initial Studio | -34.87 | -17.27 | 17.61 |
| Adobe reference | -26.01 | -22.71 | 3.31 |
| Studio r2, WPE 65% | -23.45 | -18.54 | 4.91 |
| Studio r2, WPE 35% | -23.46 | -18.56 | 4.90 |

The rejected candidate retained a 17.61 dB imbalance. The revised candidate reduces it to 4.91 dB, closer to the reference's 3.31 dB. This is a measured level correction, not proof of equivalent room cleanup. Its matched-excerpt presence and sibilant energy shares rise about 2.27/2.32 dB from original, compared with about 6.72/5.53 dB for the reference. The reference therefore still has more speech-detail emphasis. Revised air-band share is about 2.48 dB higher than original; no upper-frequency cliff is present. Whether breath/air or residual room sound is excessive needs listening. Broad body share is about .21 dB below original in that excerpt: no bass inflation.

Same offline ASR model/settings, no temperature fallback. Word difference relative to original ASR: Optimized 23.21%, Natural++ 10.58%, Adobe reference 11.95%, Rejected initial Studio 10.92%, Studio r2, WPE 65% 3.41%, Studio r2, WPE 35% 3.07%. These are not verified WER or identity/quality scores. The main and alternate differ by one ASR word; this is too small to establish a perceptual winner.

**Watchdog and repair review**

Both new WAVs and the listening MP3 pass final QA with zero clipping, measured lag or block timing drift, no spectral cliff, no excessive bass/balance change, no pause increase violation, and no fallback. Source/reference evidence files were rehashed and remain untouched. The delivered file path/hash, original source identity and temporary prepared-clip identity are recorded separately.

The original global envelope correlation remains reported; primary raw correlation is 0.908. An additional locally normalized articulation-envelope correlation (primary 0.968) separates intended slow leveling from phonetic shape changes. A 0.90 warning threshold is used for that local shape; the raw measure is retained for review, not hidden. Existing timing/peak/spectral limits remain.

A new local-loss guard compares 1 s modulated voiced-speech windows at .5 s steps, with original protected speech mask plus periodicity/modulation proxies. A window more than 12 dB below the typical source/output gain is flagged. A regression demonstrates that it catches a quiet voiced section attenuated an additional 20 dB even when global correlations remain near 1. Initial false alarms were at source windows 43.5/89.5/90 s, which the original mask classifies entirely as pause; requiring protected original speech prevents room-tail suppression from being mislabeled as speech erasure. Primary worst eligible relative window gain is -4.21 dB; alternate -4.37 dB.

These proxies can miss unvoiced phonemes, unusual low-pitched/very quiet speech or semantic damage. They remain QA signals. No automatic metric proves natural S/SH/T/CH/F articulation or speaker identity. Model/dependency failure or preservation failure uses an explicit conservative fallback and visible warning. Fallback compression is bypassed and effective-stage reporting reflects the actual fallback chain. A failing fallback or final export is rejected.

Two meaningful settings were rendered, differing only in WPE strength. Technical self-review corrected the local loss guard's pause eligibility and fallback reporting; adaptive EQ was bounded and tested. No arbitrary model/EQ matrix was generated. Independent review found no remaining material correctness blockers; listening limitations remain disclosed. The stronger WPE candidate is the main listening reference, with the gentler one retained for voice-character comparison; no final perceptual selection has been made.

**Tests**

23 Python tests, 171 reported Rust tests, 177 frontend tests: 371 reported tests passed. Production UI build, type/Svelte diagnostics, formatting and diff whitespace checks passed. The real Studio WAV/MP3 end-to-end test was explicitly enabled and passed with embedded helper modules. Historical external real-audio fixture tests may early-return without their fixture environment; counts do not imply fresh renders of every legacy model. Existing preset IDs, default/settings contracts, filters and helper arguments remain regression-protected. No unrelated application was modified as part of this revision.

**Listening artifacts**

[Main WAV, WPE 65%](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision2/CRJU160_studio_r2_wpe65_level8_presence2.wav)

SHA256: `f6c8eb34fa24b4dc7a36df04dc8430177ec47d20eb708301d076a6a627eebdd1`

48 kHz mono; 143.490 s; -21.63 LUFS; -1.55 dBTP; contrast 44.22 dB; zero watchdog defects.

[Gentler WAV, WPE 35%](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision2/CRJU160_studio_r2_wpe35_level8_presence2.wav)

SHA256: `2a129c401efdff49c8fa7e4326b4685b1dcbd20e927c8226afd06880a417515e`

48 kHz mono; 143.490 s; -21.65 LUFS; -1.55 dBTP; contrast 44.22 dB; zero watchdog defects.

[Main listening MP3](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision2/CRJU160_studio_r2_wpe65_level8_presence2_listening.mp3)

SHA256: `3d8585c40437781c0bd2cc52df565b3c4e8c4a957a26c23aeac1aca3fa58968d`

48 kHz mono; 143.490 s; -21.63 LUFS; -1.59 dBTP; contrast 44.23 dB; zero watchdog defects.

[Machine comparison](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision2/comparison.json) · [ASR comparison](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision2/asr-comparison.json) · [Aligned spectrogram](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision2/waveform-inspection/spectrograms-aligned.png) · [Validation](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision2/validation.json)

**Listening gate**

Listen especially to the quieter opening, “weeks forward,” quiet word endings, and then the louder later passage. Compare at matched playback loudness against the reference. Check room/echo during voiced speech; S/SH/T/CH/F without a lisp; recognisable speaker identity and absence of warbling; bass/body; breaths and phrase endings; and whether leveling sounds natural or pumps the room. The reference's remaining presence advantage and subjective early-room reflections are not resolved by the technical tests. The gentler candidate tests whether reducing WPE strength preserves more desirable voice character without giving back too much room sound.

The experimental Studio repository path now uses revision 2; the installed release has not been promoted or distributed. Existing Optimized/Natural++ and defaults remain. No commit or push was made. Human listening approval and a separate promotion instruction are still required.
