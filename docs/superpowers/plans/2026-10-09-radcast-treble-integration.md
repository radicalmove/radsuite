# Treble Adaptive Studio Local Integration Plan

> **For agentic workers:** Execute inline with TDD and requesting-code-review. User explicitly selected “Treble with adaptive faint-speech protection” and requested integrating/rebuilding the app on this computer. No further permission is needed for implementation, reversible local installation or the local build.

**Goal:** Add a separate `studio_treble` option labelled “Studio — Natural voice”, install the verified cached model locally, build and install the updated macOS app.

**Architecture:** Embedded versioned Python helper, pinned epoch119 native48k CPU model, exact tested two-reference adaptive preservation. Each input derives its own pre-EQ80/20 protection reference and conservative downstream calibration. Reuse that per-input leveling curve for the adaptive branch to avoid fading quiet material; never require Finnegan evidence files or an old saved gain curve. Both original and conservative-reference watchdogs run. Failure delivers transparent source cleanup with explicit QA warning. One light compressor/scalar master, lossless workingPCM, final-only encoding and export QA. Existing presetIDs/settings/default remain compatible; no rejected TFGrid model is integrated.

## Steps

- [x] Write expected-failing Python tests: independent embedded imports, strictmodel/source/controller AST parity, gaincurve derivation/replay, coreadaptive behavior, fallback on unavailablemodel/guardfailure and finalreport metadata. Create `treble_model.py`, `treble_preservation.py`, `studio_treble.py`; no changes to historical experiment helpers.
- [x] Write Rust tests for `studio_treble` serialization/label/guarded-studio classification, embeddedmodule availability, float48k preparation/final-onlyencode andexport QA. UI test newoption pluslegacy/default preservation.
- [x] Add `StudioTreble` enum/registry/embeddedbridge and desktop guarded-studio dispatch; UI option and types. Preserve all unrelated dirty files. Relevant files: engines enhancement/studio bridge/lib/tests, desktop radcast dispatch/tests, UItypes/workspace/test.
- [x] Install pinnedconfig/epoch119checkpoint from verified local evidence into `~/.radcast/models/treble-ism-eng120-epoch119`, with MIT licence/notice. No network/runtimeupgrade needed. Availability validates cached model/runtime; helper reads stablecache or explicit override, never experiment-path defaults or autodownloads.
- [x] Real end-to-end source run reproduces the accepted adaptive master on the protected Finnegan input (or detects material difference beforeinstallation); rawmodel/reference/gaincurve hashes andoriginal/baselineguards retained. Missingmodel test shows transparent fallback andwarning. TestfinalMP3/trimmededit export. No new arbitrary sound tuning.
- [x] Run relevant/full Python/Rust/UI checks, independent code review, build TauriAppleSilicon app locally. Current installedapp is `/Applications/RADsuite.app`; noRADsuite process currentlyrunning. Back up installedbundle, atomically copy rebuilt app, verifybundle/version/binary/resources and launch ifappropriate. Preserveappdata and modelcache originals. No publicrelease/signing/notarization orgitcommit/push requested.

## Production policy and limits

Treble is mostly unblended through ordinary speech; automatic recovery adds at most20% highpassed original only in flagged faint areas. Original and derivedpre-EQ protection decisions union; non-destructive energy guard remains. Dynamic per-input dry20 calibration is a defined productionpolicy, not a promise of equal quality across all recordings. Quietwords andnaturalidentity remain priorities; source fallback is explicit. UIdoes not claim Adobe parity.

Newmodel is opt-in and has its own identifier. Existing `studio_v1` remains classicrevision2. Build uses current checkout because prior authorized Studio work is uncommitted here; isolatednewworktree would omit it. All unrelated current edits are preserved. Local installedbundle backup permitsrollback.
