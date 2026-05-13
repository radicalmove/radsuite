# RADcite Visual Identity Pass Design

## Goal

Make the new Svelte/Tauri RADcite shell feel visually continuous with the previous RADcite app while keeping the new project-first RADsuite workflow.

## Context

The current shell proves the workflow: left project navigation, central document review, and right citation actions. Its green/cream palette was temporary. The previous RADcite app has a stronger visual identity in `/Users/rcd58/citation-checker/app/static/css/wysiwyg.css` and `/Users/rcd58/citation-checker/app/static/img/radcite-logo.svg`:

- UC/RADcite red `#ce3e2e` as the primary brand/action colour.
- Black, white, and neutral greys for the app frame and surfaces.
- Green reserved for positive citation states.
- Red/orange reserved for warnings and missing citation states.
- A document/check RADcite logo that should carry forward.

The colleague Rise course builder app at `https://github.com/kieran-williamson-staff/rise-course-builder-app` is also a Svelte 5 + Tauri 2 app. Its useful alignment points are:

- A shared token layer (`app/src/tokens.css`) for colour, spacing, type, shadows, and motion.
- Poppins as the main UI font, with local font files.
- A dark navy/black left sidebar, compact course-first navigation, and simple top crumbs.
- Course-level navigation with expandable course children and view tabs.
- White card surfaces on a pale wash background, small radii, subtle borders, and restrained shadows.
- UC red used as a semantic/destructive/accent colour, not as a full-page theme.

## Design

Keep the three-column product structure from the Rust rebuild. Restyle it with a RADcite token layer in `apps/desktop-ui/src/styles.css` so future UI work uses named variables instead of scattered one-off colours. Mirror the colleague app’s token vocabulary where practical: font, spacing, radius, surfaces, line colours, semantic success/warning/danger, and a dark sidebar shell.

The left navigation should become a neutral dark RADsuite/RADcite frame, not a green course card. The main workspace should use clean white document surfaces on a light grey background. Primary actions should use UC red for RADcite review actions. Citation badges should use green. Missing citation states should use red-tinted surfaces and a red left edge, matching the old citation review pattern.

Add the RADcite logo asset to the Svelte app and use it in the sidebar brand block. The brand should still read as RADsuite overall, with RADcite as the active tool identity inside the project.

The RADsuite shell should not copy CUBE’s blue Necker-cube brand. It should align structurally: tokenized CSS, Poppins, dark sidebar, course-first hierarchy, compact controls, and neutral card surfaces. Product colour remains RADcite/UC red rather than CUBE blue.

## Scope

Included:

- Add RADcite logo asset to the desktop UI.
- Update Svelte sidebar branding markup enough to display the logo.
- Replace temporary green/cream CSS values with RADcite/UC tokens.
- Align shell spacing, type, navigation density, and card treatment with the Rise course builder where it fits.
- Preserve the current workflow and command behaviour.
- Add a small CSS contract test that protects the palette and major status semantics.

Excluded:

- Native file picker.
- Dark mode.
- Full old WYSIWYG document viewer recreation.
- Any changes to Rust analysis behaviour.

## Testing

Run a CSS contract test before and after styling work. Then run the Svelte production build. Manual functional testing should reuse `/tmp/radsuite-radcite-smoke.docx` and confirm the existing workflow still loads four paragraphs and shows citation/missing-citation states.

## Review Note

The normal brainstorming workflow asks for a spec-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so the subagent review step is intentionally skipped.
