Aligned waveform comparison after human rejection of Studio v1

8 October 2026, Pacific/Auckland. Human feedback: Studio is more echoey and less clear than the preferred Adobe Podcast reference. Studio v1 is not accepted. No default was promoted, and no processing settings were changed in this investigation.

The original, Studio and reference were decoded for analysis only and aligned on the same speech. Original 10–40 s corresponds to reference 7.761–37.761 s; this is before the later timing edits. Waveform and spectrogram displays use identical gain matching based on the original's active speech frames. Native peaks and all inputs remain untouched. An additional original 70–80 s passage checks level variation before the known later offset change.

**What the evidence shows**

1. The reference has markedly stronger audible speech-detail bands. In the same 30 s excerpt, reference energy share exceeds Studio by about 6.43 dB in 1.5–4 kHz and 6.44 dB in 4–8 kHz. This is a change in spectral energy distribution, not an inferred EQ setting. It supports the perceived clarity difference. Above 8 kHz the reference has about 1.60 dB *less* relative energy than Studio: simply adding high-air treble is not the answer.

2. Quiet and loud passages are much more evenly presented in the reference. For original 10–40 s, common-mask speech RMS is -30.44 dBFS in the original, -32.74 dBFS in Studio and -23.89 dBFS in the reference. Thus the reference's speech in this quiet passage is about 8.85 dB stronger than Studio before gain matching for the plots. This coexists with the reference's lower whole-file LUFS. The whole-file loudness comparison hid an important local difference. See passage-levels.json for the later passage and the change between passages.

3. Studio's quiet-gap result was misleading as a readiness score. Across 27 source-defined falling energy edges, its median 40–120 ms gap level is -12.65 dB relative to preceding speech, versus -11.21 dB for the reference. Its 120–300 ms value is -34.87 dB, versus -26.31 dB for the reference. Studio is often *quieter* in these windows, yet is less clear to the listener. These windows include breath, quiet consonants and word boundaries; they are not isolated room-impulse tails or a measured RT60. Stronger attenuation can improve this number while worsening speech.

4. The current global pause mask is sensitive to the source's changing recording level. In original 10–40 s it marks 40.93% of frames as guarded pause, compared with 25.33% when the same estimator is applied locally to that passage. This discrepancy is a risk signal for quiet-speech attenuation, not proof that every discrepant frame is speech. The zoom at original 20.805 s falls near “weeks forward” according to approximate local ASR word timing. It should not be interpreted as a clean end-of-phrase reverb measurement. The reference carries substantially more of the subsequent low-level sound; Studio attenuates it. This supports protecting quiet articulation rather than increasing gating.

5. Studio retains nearly the original waveform shape: matched waveform correlation 0.981 and envelope correlation 0.992. Reference correlations are 0.095 and 0.941 respectively. Low sample correlation does not prove loss of identity: phase, filtering and reconstruction can alter samples while preserving a recognisable speaker. Conversely, strong raw-source similarity can also preserve the original room sound. These scores were useful preservation checks but insufficient quality criteria.

**What cannot be inferred from these plots**

The comparison cannot identify Adobe's proprietary model, exact EQ/compression settings, or isolate all room reflections from phonetic content. The listener's echo judgement remains valid; the gap proxy does not independently establish that Adobe removes more reverberation at every phrase ending. The useful findings are stronger speech-detail energy, more consistent passage levels, and retention of quiet articulation. Direct speech-versus-room separation during voiced passages still needs a targeted enhancement experiment.

**Consequences for the next Studio experiment**

Protect quiet words with a level-aware speech guard before pause attenuation; make cleanup work during active speech; assess source-preserving dereverberation with the existing local WPE dependency; then use restrained speech leveling and corrective presence control. Do not respond by applying a stronger global gate, a blanket treble boost, or generative reconstruction by default. Validate intelligibility and quiet-word retention alongside noise/peak/timing metrics. No new model or processing change was made in this waveform investigation.

Artifacts: waveforms-aligned.png, spectrograms-aligned.png, speech-edge-zoom.png, waveform-inspection.json (all 27 edges, including cases that do not favour the reference), passage-levels.json, and leveling.json. The latter is an exploratory 5 s window analysis and includes low-level/noise windows; use the explicitly matched speech passages for the principal level finding. Original-first45-asr.json in the parent folder records approximate word timing, not a human-verified transcript.

Reproduce: `python tools/radcast/waveform_compare.py --report docs/radcast/studio-experiment/comparison.json --out docs/radcast/studio-experiment/waveform-inspection`. Analysis-only decoding and filtering do not enter Studio's working signal.

Matched passage level change (later 70–80 s minus first 10–40 s):

| Version | First speech RMS dBFS | Later speech RMS dBFS | Change dB |
|---|---:|---:|---:|
| original | -30.44 | -13.56 | 16.89 |
| studio_v1 | -32.74 | -15.67 | 17.07 |
| reference | -23.89 | -21.12 | 2.78 |
