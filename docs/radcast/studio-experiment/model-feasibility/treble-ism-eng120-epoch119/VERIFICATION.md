# Verification — 9 October 2026

The complete Python suite passed: **42 tests, 18.887 seconds, no failures and no skips**. Both real-model tests were enabled, including exact checkpoint loading and repeatable full-context inference on a 20.2-second signal with a nonaligned final fragment. This proves the tested contracts, not natural speech quality.

Executed from `/Users/rcd58/Documents/RADsuite`:

```sh
/usr/bin/sandbox-exec -f docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/offline.sb /usr/bin/env PYTHONDONTWRITEBYTECODE=1 XDG_CACHE_HOME=/Users/rcd58/.radcast/experiments/treble-ism-eng120-epoch119/cache RADSUITE_TEST_MODEL_TRIAL=1 RADSUITE_TRIAL_MODEL=/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/model /Users/rcd58/.radcast/experiments/treble-ism-eng120-epoch119/runtime/bin/python -m unittest discover -s tools/radcast -p 'test_*.py'
```

Actual model inference, controls, tests and both Finnegan renders ran under OS network denial and write denial for the original cached runtime. No dependency was installed or upgraded. This experiment has a separate interpreter/cache that references existing packages; it is not a fully copied environment. Resolved inference source/native-library identities are recorded, rather than claiming complete wheel hashes.

The first Finnegan render rejected under `quiet_speech_loss`; the one explicit 20% original preservation repair passed the unchanged full-source watchdog. Exact model-raw hashes match between the two renders. The rejected master and every processing stem remain available. A QA pass is permission to compare audio, not integration or promotion approval.

The final 320 kbps MP3 independently passes export QA with the exact 143.490-second decoded length, zero measured lag and no warnings. The float master and its trial report are retained unchanged.

[Integrity verification](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/integrity-verification.json) independently re-hashes all four protected references, the selected model/config/licences, runtime snapshot files, licensed clean sources, raw controls and both sets of full Finnegan stems. Every recorded identity matches. It also confirms that the offline sandbox profile has not changed.

[Independent comparison verification](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/comparison-independent-verification.json) confirms 56 recorded comparison-path hashes, B's historical hash and the render code hashes. All six audition groups retain exact durations and matched speech RMS within 0.000001 dB; the practical three-way target set retains 576,000 samples per clip and also matches within that tolerance. Generated experiment WAVs are finite mono 48 kHz FLOAT; supplied originals retain their original encoding. Comparison-script AST and JSON checks pass. `git diff --check` passes; the new adapter/test files also have no trailing whitespace.

No native or frontend source changed during this model-feasibility execution. Previous native/UI verification belongs to its earlier Studio rounds; those results are not represented as fresh tests of this experimental adapter. The opt-in native Studio remains revision 2 and the UI's initial default remains `studio_v18`. No commit, push or default promotion occurred.
