# TFGridNet: short listening diagnostic ready, full trial not qualified

The published URGENT2025 checkpoint works locally at48kHz and preserves clean speech well. It improves synthetic late-tail/composite cases over untreated audio, but it does not meet the predeclared improvement over current Treble. No full Finnegan replacement was rendered or approved.

## Listen to the requested phrase

These three-second copies cover original19.5–22.5s, including “in the weeks forward”. They measure−23.22/−23.24/−23.25LUFS, a0.03LUspread, using static audition gain only.

- [TFGridNet diagnostic](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/phrase-diagnostic/lufs/weeks_forward_TFGrid.wav)
- [Current unblended Treble](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/phrase-diagnostic/lufs/weeks_forward_Treble.wav)
- [Adobe exemplar](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/phrase-diagnostic/lufs/weeks_forward_Adobe.wav)

TFGridNet was independently run on this single original-derived high-passed window. Treble comes from the saved full original-derived raw cleanup. Neither has additional EQ/compression/mastering here. Adobe processing is unknown. The diagnostic has a different inference context from the proposed full windowed pipeline: it cannot establish full-length preservation, superiority or safety. It is an explicitly labelled listening investigation after the full qualification failed, not an approved candidate. [Context and provenance](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/phrase-diagnostic/listening.json).

The local source watchdog returns no flags. Cached short-context ASR recognizes “them in the weeks forward just because it makes like”. This is a proxy, not verified ground truth, identity, articulation or lisp certification. The assistant has not personally listened to these files. Human feedback on clearer/closer voice and natural Finnegan identity is pending.

## Provenance and runtime

Actual34192512byte weights match publisherSHA8350b6f84bb5de01646b7cebe9d19d5b1fc4318cd85c31d242caccc2442d276e, revisiondaad000927daa131e7692376a71c8144bcbfa6f8. Strict270tensors/8524352elements loaded. Four publisher classASTs remain unchanged from source6be80495bca76d8406ca11e5971a9954949ac9c1. Modelcard/config and CCBY4/Apache2 attribution retained. No USES2 weights or labels are substituted.

NativeFFT1536/hop768 covers all769bins without resampling. The actual training cap at48k is144000samples, so the fixed inference policy is3s windows/2s hop/1s cosine overlap. It is independent window inference, not equivalent to unrestricted full-context processing. Per-window input variance is restored once; model peak normalization, phase matching and between-window gain matching are disabled.

The full-window/join CPUprobe took32.89s; local MPS18.50s. Saved outputs agree within max1.49×10⁻⁷/RMS2.83×10⁻⁸, below fixed3×10⁻⁵/3×10⁻⁶limits. ComplexFFT remains CPU, real neural features run MPS with automatic im2col CPUfallback. Actual fallback environment was unset, not forced. The original runtime is read-only; no package upgrades or installations. [Device evidence](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/device-selection.json).

## Qualification outcome

Two licensed voices, two complete sentences each, were fixed before inference. Eight logical TFGrid signals used36learned windows, with the same clean/direct,14/37ms early taps,50ms-onset late tail and combined synthetic response. Clean SI-SDR was36.76/23.79dB, with no clean waveform warnings; early-only changes were approximately0/−0.27dB, consistent with the model's retained early-response target.

| Case | TFGrid SI-SDR | Fresh Treble SI-SDR | TFGrid minus Treble |
| --- | ---: | ---: | ---: |
| p232 late |15.33dB|14.91dB|+0.42dB|
| p232 combined |10.48dB|11.24dB|−0.76dB|
| p257 late |14.67dB|15.39dB|−0.73dB|
| p257 combined |10.47dB|11.92dB|−1.44dB|

All four fail the predeclared+1dB-over-Treble criterion. SI-SDR measures all waveform error, not pure echo or perceived proximity. These scores neither prove the diagnostic sounds worse nor justify a full replacement. The publisher target retains roughly50ms early response; fully dry/booth speech is not established.

## Overlap-reference correction, transparently retained

The initial weighted-energy monitor flagged speech frames, not merely room-only masks. A diagnostic replay confirmed the original NNoutputs bit-identically. Further math showed that this monitor also penalizes unequal levels with identical polarity through Jensen's inequality, even without destructive cancellation.

A corrected coherent-sign reference was defined and frozen before a second replay: `RMS(merged) / RMS(sum(w*abs(prediction))/sum(w))`. It separates sign-conflict cancellation from amplitude-only variation, but is not pure phase/reverb or perceptual certification. Same-sign unequal-level and opposing-signal tests validate the distinction. The scalar−1dBlimit, model, weights, windows and raw audio remained fixed; the metric definition did change after results and is explicitly labelled as such. The original failed gate is preserved.

The three previously failing cases measured−0.168/−0.817/−0.560dB with the corrected reference and passed. The other five satisfy a conservative lower bound from the original stronger arithmetic-energy check. Four fresh same-input Treble comparisons then ran, and the comparative gate still failed. [Original rejection](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/controls/controls.json), [corrected assessment and baseline](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/corrected-assessment/controls.json).

Six diagnostic signal replays added26learned-window forwards, all bit-identical to their saved outputs. Setup/device probes added6windows; the teacher listening diagnostic1window; real regression tests separately2windows. No new weights/configuration/window matrix, full teacher render or repair candidate was created.

## Verification and remaining gate

Fresh full suite:126passed, no skips,48.296s, including actual checkpoint tests. Original protected audio,34original runtime files and unrelated tracked changes rehashed unchanged. All three listening clips are finite mono48k float,144000samples, nonclipping and independently remeasured at0.03LUspread. [Verification](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/verification.json), [tests](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/tfgridnet-qualification/final-tests.log).

No native/default change, commit, promotion or full-candidate MP3 export occurred. The Adobe microphone-distance objective remains unresolved. The current useful next evidence is the user's judgment of this short diagnostic; a positive impression would not itself remove the full qualification failure or approve production use.
