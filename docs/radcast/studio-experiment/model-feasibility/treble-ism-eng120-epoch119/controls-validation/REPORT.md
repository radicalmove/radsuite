# Expanded validation of saved clean/room controls

9 October 2026. Evidence-only inspection of eight saved raw WAVs. The DF model was not loaded or re-run; original control hashes were verified before and after. Cached Whisper small supplied recognition evidence, with the same English/four-thread/no-fallback/no-timestamps settings used previously. Its 16 kHz inputs are analysis-only copies.

**These controls support proceeding to one Finnegan feasibility trial. They do not establish listening success, Adobe parity or promotion readiness.** The controlled room input contains no added noise, so the paired error improvement is relevant to room/clean speech rather than only background-noise removal. One shared declared synthetic reflection pattern and two clean speakers remain a narrow domain test.

## Acoustic evidence

Known-clean source masks are reused for every spectrum and speech/pause level. Local voiced-window eligibility is also fixed to the clean signal’s protected speech, periodicity and modulation, keeping the compared windows identical. All original QA thresholds remain unchanged.

| Speaker | Synthetic input SI-SDR | Model output SI-SDR | Improvement | Room-model local worst gain vs clean | Local shape vs clean | Output dBTP |
|---|---:|---:|---:|---:|---:|---:|
| p232 | 7.78 | 11.11 | +3.33 | -1.16 dB | 0.9913 | -6.22 |
| p257 | 7.55 | 11.35 | +3.80 | -1.79 dB | 0.9878 | -5.71 |

Both speakers preserve exact source sample counts and 48 kHz mono float output. All comparisons have measured lag and block drift 0, clipping 0, and no watchdog warnings against either known-clean or room input. Clean-input model preservation likewise has no warnings. Full loudness, true peak, band shifts, raw/local shape, per-window gains and comparisons are retained in the JSON.

**Interpretation:** untreated synthetic room also passes all watchdog checks. Thus zero warnings cannot be reused as evidence of dryness. Clean-reference warnings would potentially describe residual room or model coloration; room-input warnings could also describe desired room removal. Comparing both bases avoids automatically treating either set as newly introduced damage. Here the gain-invariant paired error improves while local retained speech remains bounded, supporting a cautious real-source trial rather than a perceptual conclusion.

## Official-text recognition checks

The selected author transcripts, concatenated in their recorded source order, provide a text reference here. Punctuation/case are ignored; strict word substitutions are retained separately from the explicitly recognized British/American spelling equivalence.

| Speaker / signal | Strict word error | Other error after colors/colours equivalence |
|---|---:|---:|
| p232 / clean | 0.00% | 0.00% |
| p232 / synthetic_room | 0.00% | 0.00% |
| p232 / clean_model | 1.25% | 0.00% |
| p232 / synthetic_room_model | 1.25% | 0.00% |
| p257 / clean | 1.03% | 0.00% |
| p257 / synthetic_room | 2.06% | 1.03% |
| p257 / clean_model | 1.03% | 0.00% |
| p257 / synthetic_room_model | 2.06% | 1.03% |

For p232, both model outputs write `colours` where the official reference writes `colors`; this is a spelling variant, not evidence of an acoustic articulation error. For p257 the spelling variant appears in every condition. Its room input and room-model output both recognize `you` instead of official `these` in “bring these things”; that lexical mistake already exists before enhancement and is neither repaired nor newly introduced. No other word edits were found. These outcomes do not prove preserved S/SH/T/CH/F, breath quality or speaker identity.

## Matched listening controls

Each set uses the same known-clean speech mask, target speech RMS equal to the quietest raw condition, static downward-only gains and common downward headroom if needed. No EQ, enhancement or timing adjustment enters these copies. Speech RMS matches within floating-point precision. Raw controls remain untouched.

| Speaker | Known clean | Clean processed | Room input | Room processed |
|---|---|---|---|---|
| p232 | [clean](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/matched/p232_clean_matched.wav) | [clean_model](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/matched/p232_clean_model_matched.wav) | [synthetic_room](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/matched/p232_synthetic_room_matched.wav) | [synthetic_room_model](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/matched/p232_synthetic_room_model_matched.wav) |
| p257 | [clean](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/matched/p257_clean_matched.wav) | [clean_model](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/matched/p257_clean_model_matched.wav) | [synthetic_room](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/matched/p257_synthetic_room_matched.wav) | [synthetic_room_model](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/matched/p257_synthetic_room_model_matched.wav) |

Listen especially to “fresh snow peas,” “thick slabs of blue cheese,” “sunlight strikes raindrops” and quiet word endings. The full per-speaker control lasts about 25 seconds; boundaries and transcripts are recorded. No human listening judgment was made by this validation.

## Render-path review and stop conditions

The new standalone `render_trial` path was inspected without executing it. It replaces prior room/DF cleanup with one audited joint model, keeps native defaults untouched, applies the existing downstream policies, records actual adaptive gains, and preserves a rejected master with explicit QA status. No material render-path blocker was found.

Proceed only to the permitted single original-source trial, then perform matched common-content listening against B and Adobe. Stop on an objective preservation defect, lack of material audible improvement or altered voice. A second trial requires the already agreed diagnosed preservation repair; these controls do not authorize a tuning matrix.

[Extended QA](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-extended-qa.json) · [reproducible validation script](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/controls-validation/validate_saved_controls.py). Cached ASR transcripts/logs and analysis-only WAVs are retained under `controls-validation/asr/`.
