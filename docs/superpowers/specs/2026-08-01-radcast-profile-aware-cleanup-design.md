# RADcast Profile-Aware Cleanup Design

## Problem

RADcast currently applies the selected enhancement profile's tuned post-filter and then applies the generic cleanup filter when `cleanup_enabled` is true. The default Studio v1.8 profile therefore receives two noise/loudness passes, changing its researched sound target and producing louder, flatter speech than the original RADcast pipeline.

## Decision

- Treat every non-`none` enhancement profile as a complete cleanup recipe.
- Apply generic cleanup only when the enhancement model is `none` and the user has enabled cleanup.
- Enforce this rule in Rust so old saved settings and direct command calls cannot cause double processing.
- Use the same rule in Svelte to choose the control shown, while sending the saved user preference to Rust so switching back to Standard processing does not silently disable cleanup.
- Record `cleanup_enabled` on an output only when the generic cleanup filter actually ran. The enhancement model metadata continues to identify profile processing.
- Preserve the researched filters, enhancement model settings, pause processing, captions, and filler-removal behavior unchanged.

## Acceptance Criteria

1. Studio v1.8, Studio, Resemble, and DeepFilterNet never request generic cleanup, even when a legacy request says it is enabled.
2. Standard processing (`none`) still applies generic cleanup when selected.
3. The interface does not imply that a second cleanup pass can be combined with an enhancement profile.
4. Focused Rust and UI tests pass.
5. A real lecture clip can be processed to playable output with the Studio v1.8 recipe and no second cleanup pass.

## Test Audio

Use `CRJU201_M01_1.1.1_Course_foundations_and_evidence_based_policy_14m31s-16m52s.wav` as the real acceptance sample. If OneDrive has not hydrated the placeholder, complete automated verification first and run the real-audio check once the file is downloaded locally.
