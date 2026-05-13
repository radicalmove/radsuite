# RADcite Shell Polish Design

## Goal

Address the visual review notes from the May 4 screenshots before PR #6 is marked ready.

## Design

Keep the current three-column RADsuite shell and RADcite identity. Refine the parts that still feel off:

- Convert `Local DB` and `Sync` from rounded pill-like controls into compact status chips with small dots. They should read as passive system state, not clickable buttons.
- Add a light/dark theme toggle using the old RADcite `moon.png` asset. The toggle should sit near the status chips and persist the chosen theme in `localStorage`.
- Rename disabled left-nav product entries from internal product names to task-oriented names:
  - `Audio cleanup` with `RADcast` as the small product label.
  - `Voice generation` with `RADTTS` as the small product label.
- Make active summary cards more neutral by default. Use red emphasis primarily for the `Needs citations` state.
- Tighten the empty `Citation Actions` state so it feels intentional instead of sparse.

## Out Of Scope

- No native file picker in this pass.
- No functional RADcast/RADTTS routing.
- No full replication of the old top-bar UI.
- No Rust command changes.

## Alignment Notes

This pass keeps structural alignment with the CUBE/Rise course builder app: compact app shell, dark left navigation, tokenized styling, and task-first course workflows. It keeps RADcite distinct through the UC/RADcite red, citation review states, and document/check logo.

## Review Note

The normal brainstorming workflow asks for a spec-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so the subagent review step is intentionally skipped.
