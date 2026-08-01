# RADcast Trim Rail

## Goal

Bring the common trim workflow from the original RADcast into RADsuite while
keeping the existing numeric fields for precise keyboard and screen-reader
editing.

## User Experience

- The existing trim section gains a horizontal rail spanning the selected
  recording.
- Two accessible range controls represent the start and end handles. They are
  keyboard-operable and expose labels such as `Trim start` and `Trim end`.
- Moving either handle updates the corresponding seconds field, selected
  duration, and the saved per-source trim range.
- The start handle cannot move past the end handle and the end handle cannot
  move before the start handle. A small minimum output duration prevents an
  empty or unusably short result.
- A `Reset` action restores the complete source range and removes the saved
  trim override for that source.
- The current numeric fields remain the precise editing path and use the same
  clamping rules as the rail.
- The rail is disabled when no source or duration is available, and remains
  usable on narrow screens without changing the processing layout.

## Data Flow

The existing `clipStart`, `clipEnd`, `trimRangesBySourceId`, and processing
request remain the source of truth. A small pure helper module will normalize a
range against a source duration, enforce a minimum output duration, determine
whether a range is the full source, and format rail metrics. The component will
call these helpers from both range inputs and numeric inputs so both editing
paths have identical behaviour.

The saved range continues to be project-scoped and keyed by source ID. Full
source ranges are treated as the default and are not required to be persisted.

## Error Handling

- Invalid or non-finite numeric input falls back to the nearest valid boundary.
- A source shorter than the minimum output duration always uses the full source
  range.
- Processing continues to use the existing `processDisabled` validation, so an
  invalid transient edit cannot be sent to the Rust command.

## Testing

- Add failing unit tests for range normalization, minimum-duration enforcement,
  full-range detection, and metric formatting.
- Run the focused settings tests, then the full desktop UI checks and Rust
  contract suite.
- Verify the final layout with the existing style contract and production
  build.
