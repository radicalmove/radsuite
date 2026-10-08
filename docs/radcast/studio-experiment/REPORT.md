RADcast Studio experiment — human listening gate

8 October 2026, Pacific/Auckland. Status: experimental and opt-in; NOT promoted to the default, committed or pushed. Your correction is incorporated: the lisp is in Optimized, not Natural++. No assertion of a Natural++ lisp is made here.

**1. Existing architecture**

The Svelte RADcast workspace selects a serialized EnhancementModel. Tauri/desktop commands pass the request into crates/radsuite-desktop/src/radcast.rs. AudioProcessor runs FFmpeg/FFprobe; EnhancementProcessor stages input and invokes the selected local Python helper. Local helper source inspected at /Users/rcd58/RADcast/src/radcast/studio_cli.py and services/studio.py/resemble_safe.py. The working runtime is /Users/rcd58/.radcast/venv311, not the incomplete venv directory.

Optimized (studio_v18): decode/trim into 16-bit PCM WAV, without a preparation filter → chunked NARA WPE (8 s chunks, 1 s overlap, 6 taps, delay 2, one iteration) → Resemble generative speech reconstruction (lambda .62, tau .45, 8/16/32 steps for Fast/Standard/High) → high-pass 65 Hz, substantial bass/body EQ (+4.05 dB at 142 Hz, +1.75 dB at 200 Hz), presence/upper-spectrum cuts, FFmpeg dynamic de-esser with fixed sensitivity/max controls (.045/.18) → loudnorm -20.75 LUFS / -1.5 dBTP / LRA 8 → 7.55 kHz low-pass → final export.

Natural++ (studio_v18_natural_double_plus): decode/trim PCM WAV → spectral late-tail suppression (reduction .90, gain floor .16, smoothing .64, transient threshold 1.28, transient floor .96), --skip-enhance → high-pass 70 Hz, +2.2 dB at 130 Hz and other corrective EQ → compressor threshold .12, ratio 1.55, makeup 1.45 → same loudness targets → 10 kHz low-pass → export. Its configured postfilter has no de-esser and its current helper does not invoke Resemble. Natural/Natural+ are separate preserved presets that do invoke reconstruction.

No MP3 intermediate occurs in the inspected existing application path: prepared/enhanced files are WAV. WAV export currently uses 16-bit PCM in legacy paths. Resemble resamples to its required 44.1 kHz. Dynamic loudnorm can oversample internally, and MP3 encoding selects a supported output rate; this introduces additional rate conversions. Natural++ skips the Resemble conversion. Neither inspected path explicitly mixes original speech back into the rendered enhanced waveform. Resemble's conditioning controls are not a dry/wet preservation blend. Loudness mastering follows cleanup, rather than preceding it, but the legacy low-pass follows loudnorm.

**2. Diagnosis, with provenance limits**

Optimized's reconstruction is the most plausible contributor to the reported garbled/drunken articulation and lisp, compounded by strong tonal shaping and de-essing. A matched excerpt has envelope correlation 0.836 and a +3.82 dB body-energy-share change; sub-band share rises 18.40 dB from a small original baseline. The configured bass boosts explain the perceived fullness. These findings do not isolate one stage as the proven cause of the lisp: the historic output contains no stage/config manifest, and listener judgement remains decisive.

Natural++ has a pronounced measured upper-frequency cliff: its 8–9 kHz speech-band share is about 26 dB below the original, with almost no retained energy above 10 kHz. Air-band share is about 30 dB lower in the matched excerpt. The current code's 10 kHz low-pass is inappropriate for Studio, but cannot alone explain the strength/location of this near-8 kHz cliff. A historical helper/configuration or earlier bandwidth limitation may also contribute; that provenance cannot be recovered from the MP3 alone. Natural++ remains available unchanged.

**3. Fresh measurements of all references**

| Version | Rate / channels | Decoded seconds | LUFS | dBTP | Sample peak dBFS | RMS dBFS | Speech/pause contrast dB |
|---|---|---:|---:|---:|---:|---:|---:|
| Original | 48000 / 1 | 143.490 | -18.54 | -1.82 | -1.83 | -20.93 | 37.28 |
| Optimized | 48000 / 1 | 131.998 | -20.29 | -1.41 | -1.43 | -21.05 | 45.75 |
| Natural++ | 48000 / 1 | 131.998 | -21.10 | -1.31 | -1.31 | -22.28 | 33.64 |
| Preferred reference | 48000 / 1 | 135.198 | -25.24 | -4.13 | -4.18 | -26.48 | 42.01 |
| Studio v1 | 48000 / 1 | 143.490 | -20.75 | -2.29 | -2.30 | -23.27 | 51.05 |
| Studio mild-tail alternate | 48000 / 1 | 143.490 | -20.75 | -2.11 | -2.12 | -23.28 | 51.21 |

All four reference files are mono at 48 kHz. Original, Optimized and Natural++ are MP3; the reference is pcm_s16le WAV. Exact codec/bitrate, hashes, band shares, clipping counts, RMS, peaks, lag and block correlations are in comparison.json and comparison.txt. Native decoded durations, rather than guessed container padding, are used for timing QA. All four evidence hashes were checked again and are unchanged (reference-integrity.json).

Optimized/Natural++ are 132 s and the reference is 135.198 s, so they cannot be compared directly with the 143.49 s source for whole-file preservation or pause noise. First-block offsets are approximately -5.42/-5.439/-2.239 s, with later offset changes caused by edits. Whole-file correlations are therefore not quality scores. A stable-offset 30 s excerpt beginning at original 10 s is separately matched for spectral/envelope comparisons. That excerpt does not establish whole-file articulation preservation.

The preferred reference is quieter (-25.24 LUFS) than the original (-18.54 LUFS); additional loudness is not the target.

**4. Chosen Studio architecture**

studio_v1 is a distinct opt-in preset. It uses local DeepFilterNet3 filtering, not Resemble reconstruction. The simpler no-tail architecture is provisionally preferred: the alternate adds room-tail suppression but only changes the measured contrast by about 0.16 dB, slightly reduces source envelope correlation, and has more ASR transcript differences. That is insufficient evidence to justify making the extra processing the main path.

**5. Models, dependencies and runtime**

No packages or models were installed or downloaded. Existing DeepFilterNet3 weights occupy about 8.3 MB including cache files. Existing versions used: {'numpy': '1.26.2', 'scipy': '1.11.4', 'soundfile': '0.12.1', 'deepfilternet': '0.5.6', 'deepfilterlib': '0.5.6', 'torch': '2.1.1', 'torchaudio': '2.1.1'}. Python is 3.11 in venv311; CPU inference was benchmarked on Apple M1 Pro (arm64), up to four Torch threads. Main render inference: 6.59 s for 143.49 s, real-time factor 0.046. Full render/analysis time also includes startup and FFmpeg measurements, so the inference time is not an end-to-end latency promise.

DeepFilterNet is a full-band 48 kHz filtering model. Its repository is dual licensed MIT/Apache-2.0: [official project and license](https://github.com/Rikorose/DeepFilterNet); [primary DeepFilterNet3 paper](https://arxiv.org/abs/2305.08227). The existing Torch runtime is the large dependency; this experiment adds no new model family or Rust library dependency. A clean-machine install and Windows performance were not benchmarked. The embedded helper travels with the Rust build; processing uses the app's resolved FFmpeg/FFprobe paths.

For reproducibility, Studio checks DeepFilterNet/deepfilterlib 0.5.6 and the exact cached config/checkpoint hashes. It copies only verified artifacts to an isolated temporary directory and loads epoch 120, so another cached checkpoint cannot be selected. Cached config and weights are not modified. Model/config hashes: {'checkpoints/model_120.ckpt.best': '23b92884f63ccf54bb026014604625ab231657b6480df65db4095c4c171e6003', 'config.ini': '415eb925d44990d938fb739f514aa3662c1ec0ea836cff044fa1291b82cb4290'}. Processing never downloads a missing model. Readiness checks validate imports, cached files and local tools. Missing/invalid models after selection or failed preservation QA select a transparent source-cleanup fallback with an explicit UI warning; missing core runtime/tools or a failing fallback/export produce an error and no delivered output.

**6. Exact stage order**

Decode/trim once to 48 kHz float32 PCM → 45 Hz second-order high-pass → delay-compensated DeepFilterNet3, postfilter disabled → upper-band original preservation blend → guarded pause attenuation → light compression → measured constant loudness gain constrained by true-peak headroom → final WAV or MP3 encode → final exported-artifact QA.

All internal files are uncompressed float32 WAV. There is no lossy intermediate and no time-stretch. Input-rate conversion occurs at initial decode only when required. Float PCM reads and unchanged 48 kHz stages do not resample. Loudness measurement's analysis-only oversampling/ASR conversion never enters the processing signal. Studio WAV export stays float32; final MP3 uses the existing application's quality-2 export (the convenience listening file uses quality 0). Float WAV PEAK timestamps are canonicalized in generated candidates for stable file hashes; PCM is untouched.

**7. Configuration**

Model attenuation limit 18 dB: approximately 12.59% source / 87.41% filtered signal in lower bands. Additional dry weight ramps linearly from 0 at 3.5 kHz to .75 at 7 kHz; above 7 kHz effective source retention is approximately 78.15% and filtered contribution 21.85%. This is a frequency-dependent linear preservation blend, not generative restoration.

No bass boost, additional EQ, fixed low-pass or de-essing is applied. De-essing is intentionally inactive for this calibration because consonant preservation takes precedence; no claim of a new adaptive de-esser is made. Guarded pause attenuation is capped at 12 dB: source-derived 20 ms energy frames, at least 180 ms of low-energy interior, smoothed across seven frames to avoid abrupt gating. Light compression: threshold .18 (about -14.9 dBFS), ratio 1.15, attack 20 ms, release 220 ms, makeup 1. Final target -20.75 LUFS, true peak -1.5 dBTP with .05 dB headroom in the gain calculation. Peak headroom takes precedence if exact LUFS cannot be achieved. Constant final gain preserves phrase dynamics instead of enforcing legacy LRA 8 through dynamic normalization.

The alternate only adds spectral late-tail reduction .30 with power floor .70, smoothed delayed estimate (four 256-sample frames at 48 kHz); frequencies at/above 4 kHz are excluded from that extra suppression. It is not enabled by the application preset.

**8. Watchdog and limitations**

Shared analysis/watchdog implementation: tools/radcast/analysis.py; processing: tools/radcast/studio.py. Limits: duration difference >25 ms; lag >15 ms; block-lag spread >25 ms; any clipping; true peak above -1.3 dBTP (measurement tolerance for -1.5 target); envelope correlation <.90; pause increase >3 dB; low-frequency share increase >3 dB; sibilant loss >6 dB; air loss >10 dB; broad balance change >6 dB; missing speech energy/below -65 dBFS. Spectral checks ignore negligible bands below -50 dB relative share to avoid false alarms from codec noise on pure-tone tests.

A failed model path falls back to high-pass source cleanup, bounded pause attenuation and constant final gain without compression, then is checked again. A failed fallback is rejected. Final export is measured after encoding, with final path/hash and original source identity retained. Intentional pause/filler edits are distinguished from enhancer drift: expected retained duration and usable speech/peaks are checked, and pre-edit master preservation measurements remain available. Fallback warnings appear beside the audio in the UI; QA JSON is downloadable. QA report read/parse/copy failures remove unregistered output and temporary files.

Both WAV candidates and the listening MP3 have zero watchdog defects and no fallback. Speech/pause masks are energy proxies, not semantic VAD; cleaner measured pauses can partly reflect bounded pause attenuation rather than de-reverberation during voiced speech. Spectra/envelopes do not prove consonant or speaker identity preservation. The retained PCM-hash field in comparable analysis can describe aligned overlap; final file SHA256 is the authoritative full artifact identity.

An additional existing whisper.cpp small-model comparison ran offline with the same settings and no temperature fallback. Transcript difference rates relative to the original ASR transcript: Optimized 23.2%, Natural++ 10.6%, Preferred reference 11.9%, Studio v1 10.9%, Studio mild-tail alternate 15.4%. These are not WER against a verified transcript and do not measure a lisp or speaker identity. Original ASR itself can be wrong. ASR is report-only, not an automatic acceptance/fallback criterion. No new speaker-embedding, PESQ or STOI dependency was added; those packages were absent from the inspected environment.

**9. Tests and self-review**

15 deterministic Python tests passed; 171 Rust tests reported passed; 177 frontend tests passed (363 reported tests across suites). Svelte/TypeScript diagnostics: zero errors/warnings. Production UI build, Rust formatting and git diff whitespace checks passed. The real Studio desktop end-to-end test was explicitly enabled and exercised both WAV and MP3 exports, source identity, final metrics/report propagation, duration, cleanup handling and visible warning data.

Some historical real-audio fixture tests early-return when their external fixture environment is absent; they are not evidence of fresh legacy-model renders. Legacy preset IDs, filters/helper arguments, existing defaults and stored settings remain covered by regressions. No Optimized/Natural++ filter was rewritten. Test commands/results and implementation hashes are in validation.json; complete Rust/frontend logs are retained here.

Self-review rendered two meaningful architectures only, analyzed them against all references, added matched excerpts and offline ASR, inspected warnings, repaired QA/export/provenance handling and a negligible-band false alarm, then verified again. Processing parameters were not swept. Re-renders retained the same measured audio metrics; volatile WAV metadata was canonicalized. Independent review performed three passes; no remaining material blockers were found. Further tuning now primarily needs human listening.

**10. Listening artifacts and hashes**

[CRJU160_studio_v1_df18_air75](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/CRJU160_studio_v1_df18_air75.wav)

SHA256: `e76418a638fd1f5501c62d9659933d89faff2e558c3a194c09075f293b5e4257`

[CRJU160_studio_v1_df18_air75_tail30](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/CRJU160_studio_v1_df18_air75_tail30.wav)

SHA256: `f287c040ff206b47a1863b8102394db5e16db800fb15d9d927e2bb536054b3c6`

[CRJU160_studio_v1_df18_air75_listening](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/CRJU160_studio_v1_df18_air75_listening.mp3)

SHA256: `c400a16b1d3a93ce377faa5f851366029dae483ff61c438d398b3f687821d685`

Primary metrics: 143.490 s, 48 kHz mono float WAV, -20.75 LUFS, -2.29 dBTP, speech -19.40 dBFS / pause -70.45 dBFS / contrast 51.05 dB, zero clipping, zero measured lag and block drift, envelope correlation 0.993.

**11. Comparison and provisional choice**

Matched-excerpt band-share changes from original, in dB (sub 20–80 Hz; body 80–350; mid 350–1500; presence 1500–4000; sibilants 4000–8000; air 8000–16000):

| Matched 10–40 s excerpt | Sub | Body | Mid | Presence | Sibilants | Air | Envelope correlation |
|---|---:|---:|---:|---:|---:|---:|---:|
| Optimized | +18.40 | +3.82 | -1.41 | +1.83 | +3.01 | -1.77 | 0.836 |
| Natural++ | +10.16 | +1.13 | -0.66 | +8.26 | +9.17 | -30.31 | 0.949 |
| Preferred reference | +13.00 | +1.94 | -0.75 | +6.74 | +5.67 | -2.74 | 0.941 |
| Studio v1 | -3.07 | -0.64 | +0.10 | +0.32 | -0.77 | -1.14 | 0.992 |
| Studio mild-tail alternate | -3.45 | -0.81 | +0.13 | +0.40 | -0.45 | -0.82 | 0.991 |

Studio closely preserves the original tonal balance and envelope; there is no unnecessary bass inflation or upper-frequency cliff. Its matched-excerpt sibilant/air shares fall only about .77/1.14 dB. Compared with the preferred reference, Studio has roughly 6.4 dB less presence-band energy share in this excerpt and preserves the original's more restrained articulation/presence balance. The reference's overall processing sounds preferred to you, but these measurements cannot establish whether Studio now approaches its perceived room cleanup.

Studio is about 4.49 LU louder than the reference; level-match for judging quality. Whole-file Studio pause contrast is about 13.8 dB greater than original, but the reference/legacy files contain edits, so their whole-file pause values are not directly comparable quality rankings. No claim that Studio is cleaner than the reference is made. The simpler main candidate is provisionally preferred over the tail alternate because additional objective cleanup is tiny, envelope preservation is better, and its ASR difference rate is lower. This is a technical choice awaiting listening, not final acceptance.

[Spectral figure](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/spectral-comparison.png) · [Machine comparison](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/comparison.json) · [Readable measurements](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/comparison.txt) · [ASR comparison](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/asr-comparison.json) · [Validation](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/validation.json)

**12. Human judgement and promotion gate**

Listen first to the main WAV from the original source. Check S/SH/T/CH/F consonants for a lisp or dulling; drunken/warbling reconstruction; recognisable speaker identity; low-frequency excess; room/noise during speech; breaths, pauses and phrase endings for gating or unnatural truncation. Compare at matched loudness with the original and preferred reference. The main subjective risk is that strong original preservation leaves more room sound during voiced passages and less presence than the preferred reference. The optional mild-tail WAV tests whether its modest extra dryness is worth the small additional alteration.

Studio appears as “Studio — Recommended” with explicit experimental/listening-approval information, as requested. Existing UI/project defaults remain Optimized and existing saved IDs resolve unchanged; requests missing a model retain the existing `none` model default. The new default will not be promoted until you listen and issue a separate promotion instruction. No commit, push, deletion of historical evidence or change to unrelated applications was made. Pre-existing uncommitted checkout changes were preserved.

Reproduce measurements with the existing venv311 Python: `python tools/radcast/analysis.py --manifest docs/radcast/studio-experiment/comparison-paths.json --out docs/radcast/studio-experiment/comparison`. Render the primary with `python tools/radcast/studio.py ORIGINAL_PATH NEW_OUTPUT.wav`; use `--model-dir` for the pinned cached model and `--tail` only for the alternate. Use a fresh output path for future experiments. The paths manifest is machine-local and can be replaced for future calibration sets.

Human-feedback addendum, 8 October 2026: Studio v1 was rejected as more echoey and less clear than the reference. It has not been promoted. The aligned waveform investigation is recorded in [waveform findings](waveform-inspection/FINDINGS.md); quiet-word protection and passage leveling are now identified as weaknesses that the original pause/envelope scores did not establish.

Revision 2, 8 October 2026: the user authorized continuing from the waveform findings. New source-preserving WPE, local articulation guard, bounded leveling and adaptive presence correction are implemented in the experimental Studio path. [Revision 2 listening report](revision2/REPORT.md) contains new candidates and validation. No default promotion or commit has occurred.
