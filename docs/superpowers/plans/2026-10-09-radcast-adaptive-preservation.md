# RADcast adaptive original-preservation probe

> Execute in the existing checkout using executing-plans, test-first implementation and focused review. Continue the user's authorized Studio improvement and adaptive-blend investigation, now supported by their preference for the unblended target. No worktree, commit, push, package installation, model inference or preset promotion. Review uses the existing studio_review agent.

**Goal:** Preserve the preferred unblended foreground cleanup while restoring faint source material only where existing source-relative protection detects excessive model loss.

**Architecture:** Resume the verified original-derived mono48k float highpassed/model stems. Replace the global 20% preservation blend with a source-loss-triggered 0–20% time-domain blend. Freeze the accepted dry20 pause policy, actual +2dB presence filter and saved speech-level gain curve; reproduce the old full master exactly before one new render. Keep the existing light compressor and final scalar/headroom policy. Do not add the closed presence probe's extra EQ. No clip-specific decisions or handpicked time ranges.

**Tech Stack:** Existing isolated offline Python, NumPy/SciPy/SoundFile and FFmpeg; unchanged watchdog and cached Whisper small. Original-source lossless stems only.

## Design and alternatives

User: “target_phrase_model_unblended.wav sounds better, a bit cleaner and sounds slightly closer to the mic”. This updates the previous waveform-based working diagnosis: a small blend difference does matter perceptually. It approves only the diagnostic target preference, not globally unblended delivery or identity/articulation.

Recommend adaptive preservation. Globally removing the blend is already rejected for protected low-level signal loss. A smaller fixed blend does not isolate preservation to the affected material and has no demonstrated retention floor. Switching models is deferred until the evidenced blend contribution is addressed.

The existing `analysis.speech_window_gain` identifies source-periodic/modulated windows, 1s long with 0.5s hop. Measure raw-model gain relative to its median eligible-window gain. Blend weight is zero at relative gain >=−9dB, rises linearly to 0.20 at −12dB, and remains capped at 0.20 below that. These are pre-leveling detector settings, not a change to final watchdog thresholds. The 3dB onset margin protects transitions before excessive loss. No speech evidence yields zero blend.

Combine overlapping eligible windows by maximum weight over their full support. On 20ms frames, expand support by 160ms each side (8 frames), then smooth with a centered five-frame/100ms nonnegative box and interpolate to sample centers. Window analysis uses up to 1s of future support plus up to210ms expansion/smoothing; median calibration uses the whole recording. This is offline, not a real-time latency claim. Weight stays 0–0.20. Crossfade only original-derived aligned float signals: `(1-weight)*model + weight*highpassed_source`. No gain-normalized reconstruction, multiband split or learned classifier. A suppression detector can mistake room/background material for faint speech; modest source recovery is the conservative trade-off.

Actual-signal energy guard: after smoothing/interpolation, compare 20ms candidate/model RMS on unchanged source-protected frames with source RMS>1e−5 and model RMS>1e−8. If adding original decreases a frame's model RMS by more than0.1dB, bypass its entire connected nonzero blend-support interval, including any interval whose interpolated boundary overlaps that frame (one-frame neighbor margin). Zero the already-smoothed interval; do not smooth again, phase-align, normalize, exceed20% or attempt another repair. Re-interpolate once and verify the same guard. Record conflicts and bypassed intervals. Whole-support bypass retains zero endpoints without introducing a new weight discontinuity. If bypass leaves source loss, the unchanged final watchdog rejects the sole candidate. The controller must not promise recovery in adverse phase cases.

The unchanged source-relative final QA decides acceptance. The adaptive detector itself does not prove consonant preservation. Preserve the rejected unblended full master and all prior audio. One full candidate is permitted. If rejected, retain the named attempt and stop this round; do not relax guards or change thresholds/weights.

## Task 1 — tests and bounded controller

Create `tools/radcast/adaptive_preservation.py` and `tools/radcast/test_adaptive_preservation.py`; preserve existing processing helpers.

- [ ] Observe failing tests before implementation: invalid/native/count/finite contracts; silence and no-loss bypass; deterministic nonaligned duration; normal speech unchanged; recovery of severely attenuated modulated speech; anti-phase blend protection; smooth bounded transitions; onset/cap rules and provenance/history rejection.
- [ ] Implement the frozen controller and save detector window records, frame weights and active support summaries. Never hardcode Finnegan times or target exclusions.
- [ ] Run the new and existing Python suite with pinned real-model tests enabled. Save observed command/results and code/config hashes.

## Task 2 — one full source-derived probe

Fresh evidence folder: `docs/radcast/studio-experiment/adaptive-preservation/`.

- [ ] Verify protected references, baseline manifest/stage/source identities and saved gain curve. Recompute baseline pause/presence/level/compressor/master with recorded final scalar; require max float error <=3e−7. Record saved gain SHA and exact stage reproduction. Refuse nonempty history folders.
- [ ] Apply the controller once to the original-derived model and highpassed source, then frozen downstream stages. Save weights, masks, detector windows and float stems. Record original recovery support and gain-frozen experimental scope.
- [ ] Confirm the target cleanup region remains bit-identical to raw model as a measured outcome, not by hardcoding an exclusion. If adaptive recovery reaches20–32s, disclose loss of this isolation and stop without tuning exclusions. The new full-master scalar may still differ and is removed in matched audition.
- [ ] Apply unchanged full original-source timing, spectrum, protected loss, pause, true-peak and clipping guards; compare against baseline as well. Retain rejection without fallback substitution. Only if accepted, export final MP3 once and independently verify it; do not mutate the original trial report during export.
- [ ] Evaluate the controller on the saved licensed clean/room controls without new model inference. Record activation and unchanged/changed paired scores; no human control result inferred.

## Task 3 — listening and review

- [ ] Cut equal-original-mask static RMS target20–32, quiet19.5–22.5, loud70–80 and ending38.5–41.8 copies from full masters. Keep Adobe's verified −2.239s offset. Make a target LUFS-matched set with static down-only adjustments. No per-clip enhancement.
- [ ] Add matched original/old/new comparisons around faint start0–8, internal66–69 and end137–143.49, without Adobe's missing handles. Include before/after intro0–12 for the prior tonal probe's ASR flag; no claim that the new variant fixes it from energy alone.
- [ ] Use the cached Whisper small binary/model and exact `-l en -t 4 -nf -nt -otxt` settings; save proxy edits and word differences. ASR does not prove natural identity or lack of lisp.
- [ ] Independent saved-stem/weights/gain/hash/sample-format/clip-level verification and focused code/evidence review. Document relevant limitations and the human target preference.
- [ ] Deliver one candidate and matched target versus old dry20/Adobe, with faint-context links. Leave technical acceptance distinct from human voice/articulation approval and later integration/promotion.

## Completion boundary

This is a separate human-feedback-driven adaptive-preservation round. It does not extend the prior model or EQ budgets. A useful calibration result would still need broader source-loss/false-positive validation and a production adaptive-leveling decision before changing Studio. Native opt-in Studio remains r2; existing Optimized, Natural++ and UI defaults stay unchanged.

## Diagnosed coverage correction after the rejected first probe

The first probe is rejected and retained: original-source guards pass, baseline-relative quiet-loss fails at2.0–3.5s by up to20.88dB. No delivery export or ASR approval is issued. All four replay errors are0, and target cleanup isolation is exact. Read-only comparison localizes a detector-eligibility difference: saved global20 cleanup recognizes source material at1.5/2.0/2.5s that the original reference does not. It also identifies105.0/105.5s and slightly broader ending support. Semantic speech classification remains unverified.

Within the ongoing authorized source-preservation investigation, make ONE separately recorded coverage repair, without changing thresholds, cap, smoothing, tone, gain curve, master policy or final guards. Preserve a copy of the first adapter/tests and their hashes alongside its rejected evidence before edits. This is not another strength trial or permission to relax acceptance.

Add a second optional protection reference to the controller: the pinned original-derived saved baseline `cleanup.wav` (before pause/EQ/level/compression). Compute the same existing periodic/modulated loss detector independently for original and preserved-cleanup references; calibrate each median independently and union window weights by maximum. Both references use identical thresholds/cap. Union their protected masks for the post-interpolation energy guard, retaining the original RMS eligibility floor. The preserved cleanup is a decision reference only: new audio remains the original highpassed source mixed with the same raw model, never a delivered or previously enhanced master. This intentionally errs toward preserving ambiguous faint material and can restore some room/background sound in those intervals.

Test the observed failure first, with the verified calibration stems: assert the second reference protects2.0–3.5s without activating20–32s, while all native/count/phase/weight/provenance contracts remain active. Also reject malformed protection references. Run the entire suite with this local regression and real-model tests actually enabled. Then render ONCE into fresh `adaptive-preservation-repair/` from the original baseline, not the rejected attempt. If either unchanged final guard rejects, retain and stop without another correction. If accepted, complete final-only MP3, exact-settings ASR, matched comparisons and independent/reviewer verification. Both rounds' budgets and outcomes remain explicit.
