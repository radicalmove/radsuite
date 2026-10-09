# Studio revision 2 experiment

User authorized continuing after aligned waveform diagnosis. Stay in existing checkout; no commit/push/default promotion.

- [ ] Add regression tests for local masks protecting quiet speech amid recording-level changes; prove failure.
- [ ] Implement locally adaptive energy masks with conservative articulation hangover; keep QA thresholds unchanged and record mask provenance.
- [ ] Add source-preserving NARA-WPE in overlapping 12 s chunks at 48 kHz, bounded blend, no synthesis or bandwidth truncation.
- [ ] Add speech-only leveling capped at +/-8 dB, smooth gain changes and inactive-pause unity gain; small +2 dB presence correction.
- [ ] Render r2 from original, measure, compare word retention, passage levels and technical watchdog to r1/reference. At most one alternate if WPE strength remains uncertain.
- [ ] Correct at most 2–3 justified defects. Integrate embedded helper dependency/failure reporting and run relevant regressions.
- [ ] Save exact configuration/results and stop for another human listening gate.
