RADcast booth-dryness trials — revision 3 listening result

**Human result, 9 October 2026 (Pacific/Auckland)**

The user judged Adobe clearly better than all three local versions by a fair margin, with almost no audible echo. Trial B is the best of the local versions but remains noticeably inferior. B is therefore the preferred local comparison baseline for future research, not an approved final candidate. The user did not separately approve articulation, identity or production readiness. Technical QA passing did not establish sufficient room removal.

This round is closed without integration or promotion of either trial. Its two permitted architectures have been evaluated. Further WPE/tail parameter tuning is not justified by this result. A separate focused model-feasibility plan is proposed at `docs/superpowers/plans/2026-10-09-radcast-dereverb-model-feasibility.md`; no model download, installation or third audio trial has been performed under this round. Native Studio remains revision 2.

The remainder preserves the 8 October experiment report and listening-gate evidence as recorded before this feedback.

8 October 2026, Pacific/Auckland. User feedback: revision 2 is much closer, but Adobe sounds a touch less echoey, particularly “reading anything to do with the criminal law a lot more simple” around original 23–28 s. This round follows the approved booth-dryness plan. No default promotion, commit, push or installed release update has occurred. Native opt-in Studio still uses revision 2 WPE delay 4; the new methods require explicit experimental CLI controls.

**What was frozen**

The full original was decoded once to 48 kHz float PCM for each render. Revision 2's .65 WPE blend (where applicable), pinned DF3 model/attenuation/preservation blend, high-pass, source articulation mask, 6 dB guarded pause control, adaptive presence policy, speech-leveling configuration, light compression and final mastering targets were retained. All intermediate audio remains uncompressed float WAV. No generative model, de-esser, low-pass or bass boost was introduced. Reference/original evidence hashes were checked again and remain unchanged.

A diagnostic baseline render reproduced revision 2 byte-for-byte: True. Full PCM stems before/after room control, DF cleanup, presence and leveling were saved, with source masks and exact applied level-gain curves. Those curves replay exactly in a regression test. Diagnostic folders must be fresh; stale/historical stems are not overwritten. Manifests distinguish attempted stems from the actually delivered/fallback chain.

**Trial A — shorter WPE prediction guard**

One setting changed: delay 4 → 2 frames, nominal prediction history 21.33 → 10.67 ms at the 256-sample hop / 48 kHz. All other WPE settings remain: 12 s chunks / 2 s overlap, FFT 1024, taps 10, two iterations, PSD context 1, .65 estimate / .35 observed blend, 80–8000 Hz processing region. Other frequencies remain present. This is a prediction-history change, not a sharp physical reflection cutoff or an inferred Adobe setting. Invalid delay values are rejected before decoding; the application's default remains 4.

The target 21–29 s region changes gently. Cleanup-only candidate/R2 RMS difference is -0.086 dB, while the difference signal is -25.11 dB relative to the baseline. This signal difference is not isolated reverb removed. It does establish a nonzero change beyond a simple volume boost.

Actual presence gain stayed 2 dB. Adaptive leveling responded to the changed cleanup input: target-region gain-curve RMS difference 0.173 dB, maximum 1.163 dB; full-record maximum difference 2.985 dB. Thus configuration was frozen, but adaptive gains were not literally identical. A diagnostic using the exact baseline gain curve still shows a change (difference signal -24.70 dB relative), so the result is not solely due to leveling. This frozen-gain stem is a control, not a separately promoted preset.

On a synthetic chirped harmonic signal with known 14/37 ms reflections, gain-invariant error to known clean was 12.54 dB for room input, 11.26 dB for delay 4 and 12.48 dB for delay 2. Shortening the guard did not materially improve separation over that synthetic input and strongly reduced overall signal amplitude before gain compensation. That synthetic test is not Finnegan's recording or a perceptual quality definition, but it cautions against simply pushing WPE harder.

**Trial B — bounded room-tail control**

The plan's one conditional alternative replaces WPE, rather than stacking another processor onto it. It uses a source-phase-preserving magnitude control on low/mid TF energy below its delayed running history. Maximum attenuation is 2 dB. STFT is 1024 / 256 at 48 kHz; history delay is two frames; running power decay parameter is 120 ms. This is a heuristic estimate, not measured RT60 or a clean/room separation model.

Control fades out from 3.5–4 kHz; frequencies at/above 4 kHz are untouched by this stage. Source phase is retained. Strong transients above 1.35 times the history estimate reset to unity. Otherwise attenuation has bounded release smoothing (alpha .6), so a modest rise may briefly retain attenuation; it is not a claim that every rising TF region is instantly untouched. A steady-tone test confirms there is no sustained global dulling. Actual minimum gain is about -2.000 dB. The gain limit, high-band projection, finite output, rate, timing and duration are regression-tested.

All subsequent processing remains the same revision 2 configuration. Actual adaptive presence gain was again 2 dB. Adaptive leveling differences are recorded separately (target-region RMS gain-curve difference 0.298 dB; maximum 1.155 dB). A second frozen-baseline-gain diagnostic is saved. No new package or model was installed or downloaded.

**Common-content comparison**

All comparisons use the existing locally adaptive source masks and preserve sample timing. The reference contains edits; only verified stable-offset common content is used for preservation comparisons. The target listening excerpt is original 20–32 s, reference 17.761–29.761 s, with the -2.239 s offset recorded. Local ASR places the stated phrase approximately at original 23.470–28.380 s, but ASR word timings are approximate.

| Version | Duration s | LUFS | dBTP | Speech/pause contrast dB |
|---|---:|---:|---:|---:|
| Original | 143.490 | -18.54 | -1.82 | 35.40 |
| Adobe reference | 135.198 | -25.24 | -4.13 | 41.02 |
| Revision 2 | 143.490 | -21.63 | -1.55 | 44.22 |
| Trial A: WPE delay 2 | 143.490 | -21.60 | -1.55 | 44.04 |
| Trial B: bounded tail | 143.490 | -21.70 | -1.55 | 44.46 |

ASR word differences relative to original ASR: Adobe reference 11.95%, Revision 2 3.41%, Trial A: WPE delay 2 3.07%, Trial B: bounded tail 6.83%. Both trials retain the specified sentence in the offline transcript. The bounded-tail result has less favourable whole-file ASR agreement; source ASR is not verified ground truth, so this is a warning/inspection signal rather than proof of lost articulation. ASR cannot prove speaker identity or absence of lisp.

No audible dryness advantage is asserted as confirmed. The changes are small and both methods have single-channel/proxy limitations. The first method does not demonstrate clean/room separation on the simple synthetic test; the second can attenuate desired falling phonetic energy as well as room tail, despite its conservative cap. There is no room-only ground truth for this recording. Human listening to the matched clips decides whether either improves the recording-booth impression.

**Artifact watchdog and test results**

Both full WAVs and optional listening MP3s pass final exported-artifact QA: zero clipping, measured lag and block drift; no HF collapse or bass-inflation warning; no excessive broad spectral change, pause gain or eligible voiced-window loss. No fallback was used. Stage order, uncompressed working format, encode-once export, dependency failure and old preset IDs/defaults remain protected. Raw and locally normalized envelope correlations and local gain-loss checks remain available; thresholds were not relaxed.

31 Python tests and 171 reported Rust tests passed, including real Studio WAV/MP3 end-to-end processing. The frontend/UI was unchanged; the previous 177 passing frontend tests and production build remain applicable. Total reported test count across these suites is 379. Some historical fixture tests early-return without their external fixture environment; these counts are not fresh renders of every legacy model. Formatting and diff whitespace checks passed. Code review addressed diagnostic fallback provenance and verified new delay/method metadata and preserved defaults.

The waveform plotting utility's custom-preset spectral metadata selector was corrected; no rendering signal was changed by that diagnostic fix. All requested/actual processing metadata and source/output hashes are retained in the candidate QA files. Exact implementation hashes and logs are in validation.json.

**Start with the matched 12-second clips**

These clips were extracted from full PCM renders, not separately enhanced as short excerpts. An identical original-derived speech mask is used to match RMS; static gains only reduce level, and a common downward headroom gain is applied if needed. No time stretch or tonal matching was performed. Separate sets include the quiet word boundary, louder passage and approximate phrase-ending context near “time.” Energy falls inside words are not presented as isolated room decay.

[Existing revision 2, matched phrase](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/listening-all/target_phrase_r2_matched.wav) — 12.000 s; gain -3.16 dB.

[Trial A: shorter WPE, matched phrase](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/listening-all/target_phrase_shorter_wpe_matched.wav) — 12.000 s; gain -3.17 dB.

[Trial B: bounded tail, matched phrase](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/listening-all/target_phrase_bounded_tail_matched.wav) — 12.000 s; gain -3.02 dB.

[Adobe reference, matched phrase](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/listening-all/target_phrase_reference_matched.wav) — 12.000 s; gain 0.00 dB.

**Full lossless candidates and hashes**

[Trial A full WAV](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/CRJU160_studio_r3_wpe65_delay2.wav)

SHA256: `5415c71be2da00ef03fcac0ffad46fb95f100290fbf465db034c7930190fd583`

[Trial B full WAV](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/CRJU160_studio_r3_bounded_tail2db.wav)

SHA256: `dafce9a9f2b65f93fd8c0c9055ee510d38ce88ee8d924733fcdfe8551a2d8016`

[Trial A listening MP3](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/CRJU160_studio_r3_wpe65_delay2_listening.mp3)

[Trial B listening MP3](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/CRJU160_studio_r3_bounded_tail2db_listening.mp3)

[Machine comparison](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/comparison-all.json) · [Controlled WPE diagnostics](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/controlled-diagnostics.json) · [Controlled bounded-tail diagnostics](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/controlled-tail-diagnostics.json) · [ASR](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/asr-all.json) · [Listening manifest](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/listening-all-manifest.json) · [Validation](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/revision3/validation.json)

**Listening gate and stopping point**

Compare the halo around the words, especially “reading anything … criminal law … more simple.” Check whether Finnegan feels closer to the microphone while keeping the same voice, natural S/SH/T/CH/F, vowel body, breaths and phrase dynamics. Also check the quieter words for flattening or missing detail and the louder excerpt for pumping. Judge dryness at the matched playback level, not by the quietest pause score or waveform density.

Neither new trial is selected or promoted. Native Studio stays at revision 2 WPE/default delay 4; Optimized, Natural++ and stored defaults remain intact. No commit, push or overwrite of original/reference/historical audio evidence was made. This round has used its two permitted meaningful trial architectures. If neither provides the desired improvement, stop DSP tuning and propose the deferred dedicated-model research plan rather than generating a third matrix of settings.
