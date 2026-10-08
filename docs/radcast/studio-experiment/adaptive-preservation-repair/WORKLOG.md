# 2026-10-09 adaptive preservation coverage repair

The original-only trial was rejected and retained. The pre-EQ cleanup reference coverage correction was designed and independently reviewed before its one render. Two regression tests failed before the correction; the resulting suite passed 60 tests. The final saved-artifact audit and fresh complete suite also passed (60 tests in 19.891 seconds; no skips). Model and local calibration tests were enabled.

All candidate generation used verified original-derived 48 kHz lossless stems, with no new candidate model inference. The two-reference controller adds protection decisions only, keeps 20–32 seconds identical to raw model at cleanup, and passes unchanged original and baseline guards. Final 320 kbps MP3 was encoded from the accepted master and verified separately without mutating WAV QA.

The single cached ASR call completed, then report finalization raised a historical-field KeyError. The saved analysis input was regenerated solely in a temporary verification directory and matched byte-for-byte; the existing transcript/log were preserved and the missing JSON report finalized without rerunning recognition. The intro omission and increased proxy edits are disclosed and retained as human listening flags.

All 30 static audition clips replay exactly. Comparison ending metadata collision did not change audio; independent verification indexes the three short-ending RMS clips omitted from the original comparison metadata. Future script uses a distinct short_ending key. Original metadata retained.

No further processing candidate, threshold tuning, extra EQ, native/default change, commit, or promotion. Render budgets zero. Final independent review found no material blocker to presenting the trial; substantive ASR examples and absolute report links were added. Full-length human listening remains pending.
