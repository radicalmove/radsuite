# Application Menu and Manual Update Check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the noisy header status strip with a compact hamburger application menu and add a user-triggered update check, then prepare RADsuite 0.2.8 for public release.

**Architecture:** A focused Svelte `ApplicationMenu` component owns menu presentation, focus, and dismissal. `App.svelte` retains updater and Help orchestration, while a small testable update-check helper distinguishes automatic checks from manual checks and produces the five-second “up to date” confirmation. Existing signed Tauri update installation remains unchanged.

**Tech Stack:** Svelte 5, TypeScript, Vitest, Happy DOM for component interaction tests, Tauri updater, CSS, Rust/Cargo release manifests, GitHub Actions.

**Release authorization:** After approving the UI design and specification, the user explicitly requested the full update, GitHub publication, version bump, and continued monitoring until all release work is complete. Task 4 implements that subsequent authorized release request in addition to the UI specification.

---

### Task 1: Accessible Application Menu Component

**Files:**
- Create: `apps/desktop-ui/src/components/ApplicationMenu.svelte`
- Create: `apps/desktop-ui/src/components/ApplicationMenu.test.ts`
- Modify: `apps/desktop-ui/package.json`
- Modify: `apps/desktop-ui/package-lock.json`

- [ ] **Step 1: Add the component-test environment**

Add `happy-dom` as a development dependency with `npm install --save-dev happy-dom`. Keep all existing dependency ranges unchanged.

- [ ] **Step 2: Write failing interaction tests**

Mount `ApplicationMenu.svelte` into Happy DOM and assert:

```ts
expect(button.getAttribute("aria-expanded")).toBe("false");
button.click();
await tick();
expect(button.getAttribute("aria-expanded")).toBe("true");
expect(document.activeElement?.textContent).toContain("Check for updates");
```

Cover opening, toggle-close, outside-click close, Escape close with focus returned to the hamburger, Help callback, update callback, disabled “Checking for updates…” state, and rendered version text.

- [ ] **Step 3: Run the focused test and verify RED**

Run: `cd apps/desktop-ui && npm test -- --run src/components/ApplicationMenu.test.ts`

Expected: FAIL because `ApplicationMenu.svelte` does not exist.

- [ ] **Step 4: Implement the minimal component**

Create a component with props:

```ts
type Props = {
  version: string;
  checkingForUpdate: boolean;
  onCheckForUpdates: () => void;
  onOpenHelp: () => void;
};
```

Use a native hamburger button with `aria-label="Open application menu"`, `aria-haspopup="menu"`, `aria-expanded`, and `aria-controls`. Render Check for updates, Help, and the version footer. Use Svelte actions/effects or document listeners for outside-click and Escape dismissal, clean listeners up when closed/unmounted, focus the first action on open, and return focus only after Escape.

- [ ] **Step 5: Run the focused test and verify GREEN**

Run: `cd apps/desktop-ui && npm test -- --run src/components/ApplicationMenu.test.ts`

Expected: all component tests PASS.

- [ ] **Step 6: Commit the component**

```bash
git add apps/desktop-ui/package.json apps/desktop-ui/package-lock.json apps/desktop-ui/src/components/ApplicationMenu.svelte apps/desktop-ui/src/components/ApplicationMenu.test.ts
git commit -m "feat: add accessible application menu"
```

### Task 2: Manual Update Check State and Feedback

**Files:**
- Create: `apps/desktop-ui/src/lib/updateCheck.ts`
- Create: `apps/desktop-ui/src/lib/updateCheck.test.ts`
- Create: `apps/desktop-ui/src/lib/updatePresentation.ts`
- Create: `apps/desktop-ui/src/lib/updatePresentation.test.ts`
- Create: `apps/desktop-ui/src/lib/applicationHeader.test.ts`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/lib/updateState.test.ts`

- [ ] **Step 1: Write failing update-check tests**

Define the intended helper API through tests:

```ts
const result = await checkForStableUpdate({
  force: true,
  now: () => 20_000,
  storage,
  check: async () => null,
});
expect(result).toEqual({ kind: "current" });
```

Also test that a non-forced check returns `{ kind: "skipped" }` inside 24 hours, a forced check bypasses the throttle, a returned update becomes `{ kind: "available", update }` even when its version was dismissed, and failures reject without recording a successful check.

Write presentation tests around a pure `presentStableUpdateCheck(result, manual, installedVersion)` helper and exported `UPDATE_CURRENT_NOTICE_MS = 5_000`. Assert that `{ kind: "current" }` produces the up-to-date message only when `manual` is true, `{ kind: "available" }` preserves the updater object and clears the current message, and `formatUpdateCheckError(error)` preserves the existing visible “Could not check for updates” error wording.

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cd apps/desktop-ui && npm test -- --run src/lib/updateCheck.test.ts src/lib/updatePresentation.test.ts`

Expected: FAIL because the update-check and presentation modules do not exist.

- [ ] **Step 3: Implement the helper**

Move the storage/throttle/check orchestration from `App.svelte` into `updateCheck.ts`. Keep `updateState.ts` as the storage-rule owner. Return a discriminated union:

```ts
type StableUpdateCheckResult =
  | { kind: "skipped" }
  | { kind: "current" }
  | { kind: "available"; update: Update };
```

Record the check timestamp only after the updater API resolves successfully. Pass `force` to both the throttle bypass and `shouldShowUpdateVersion(..., allowDismissed)`.

Implement `updatePresentation.ts` as a pure mapping layer consumed by `App.svelte`; it must not own timers or call the updater API. This makes the manual-only current confirmation, update-available preservation, five-second duration, and error wording directly testable without mounting the full application shell.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run: `cd apps/desktop-ui && npm test -- --run src/lib/updateCheck.test.ts src/lib/updatePresentation.test.ts src/lib/updateState.test.ts`

Expected: all update-state tests PASS.

- [ ] **Step 5: Integrate manual feedback into App.svelte**

Import and render `ApplicationMenu`. Remove the version, local-saving, cloud-backup, and standalone Help controls from the header while preserving the separate existing moon-image button. Route the menu update action to `checkForStableUpdate(true)` and Help to the existing modal.

Add `updateCurrentMessage` state. A manual `{ kind: "current" }` result shows “RADsuite {version} is up to date.” for five seconds. Clear and replace its timeout when a new check begins, an update becomes available, or an error occurs. Automatic checks do not set this message. Clean up the timeout on component destruction.

- [ ] **Step 6: Add a failing source contract for removed status clutter**

Add a focused source-contract assertion in `applicationHeader.test.ts` that reads `App.svelte` and verifies the old `version-chip`, `status-chip`, “Cloud backup”, and standalone `help-button` header markup are absent while `ApplicationMenu` and `theme-toggle` remain. It must also verify that `App.svelte` renders the existing update-available notice/actions and visible `updateError` notice, and consumes `presentStableUpdateCheck` plus `UPDATE_CURRENT_NOTICE_MS`; the presentation tests prove what those consumed values mean.

Run it before the markup change to observe the expected failure, then run it after integration to observe PASS.

- [ ] **Step 7: Run integrated frontend tests**

Run: `cd apps/desktop-ui && npm test -- --run`

Expected: all frontend tests PASS.

- [ ] **Step 8: Commit updater integration**

```bash
git add apps/desktop-ui/src/App.svelte apps/desktop-ui/src/lib/applicationHeader.test.ts apps/desktop-ui/src/lib/updateCheck.ts apps/desktop-ui/src/lib/updateCheck.test.ts apps/desktop-ui/src/lib/updatePresentation.ts apps/desktop-ui/src/lib/updatePresentation.test.ts apps/desktop-ui/src/lib/updateState.test.ts
git commit -m "feat: add manual update checks"
```

### Task 3: Existing-Style Menu Polish and Responsive Behaviour

**Files:**
- Modify: `apps/desktop-ui/src/styles.css`
- Modify: `apps/desktop-ui/src/components/ApplicationMenu.svelte`
- Modify: `apps/desktop-ui/src/components/ApplicationMenu.test.ts`

- [ ] **Step 1: Write failing semantic/style contract assertions**

Extend the component test to require stable classes for the application-menu wrapper, trigger, panel, actions, and version footer, and assert that the trigger contains three individually styled hamburger lines without adding an icon dependency.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `cd apps/desktop-ui && npm test -- --run src/components/ApplicationMenu.test.ts`

Expected: FAIL because the final styling hooks are not present.

- [ ] **Step 3: Implement styling in the existing design language**

Add CSS using current variables (`--bg-panel`, `--line`, `--fg-*`, spacing, radius, focus, and shadow conventions). Keep the action group right-aligned on wide layouts and compatible with the existing grid stacking at 820px. Constrain the panel to the viewport with a responsive width such as `min(240px, calc(100vw - 2 * var(--s-4)))`.

Remove obsolete `.status-strip`, `.status-chip`, `.version-chip`, and `.help-button` rules if they have no remaining consumers. Preserve `.theme-toggle` and its current moon image/dark-theme behaviour.

- [ ] **Step 4: Run component and static checks**

Run:

```bash
cd apps/desktop-ui
npm test -- --run src/components/ApplicationMenu.test.ts
npm run check
npm run build
```

Expected: component tests PASS, Svelte reports 0 errors and 0 warnings, and Vite build exits 0.

- [ ] **Step 5: Commit visual integration**

Before committing, run RADsuite locally and manually verify the actual header and menu in light and dark themes, with a long project title and at the existing narrow breakpoint. Verify pointer outside-click dismissal, Escape dismissal with focus return, keyboard tab order, Help opening, and the disabled update-check state. Correct any visual overflow, contrast, or focus problem and rerun the component test, Svelte check, and build after the correction.

```bash
git add apps/desktop-ui/src/components/ApplicationMenu.svelte apps/desktop-ui/src/components/ApplicationMenu.test.ts apps/desktop-ui/src/styles.css
git commit -m "style: simplify application header"
```

### Task 4: Version 0.2.8 and Release Verification

**Files:**
- Modify: `Cargo.lock`
- Modify: `apps/desktop-ui/package.json`
- Modify: `apps/desktop-ui/package-lock.json`
- Modify: `apps/desktop-ui/src-tauri/Cargo.toml`
- Modify: `apps/desktop-ui/src-tauri/tauri.conf.json`
- Modify: `crates/radsuite-desktop/Cargo.toml`

- [ ] **Step 1: Update every release version to 0.2.8**

Change the synchronized frontend, Tauri, desktop crate, Tauri configuration, and lockfile versions from `0.2.7` to `0.2.8`. Do not change the internal `0.1.0` library crate versions.

- [ ] **Step 2: Validate version synchronization**

Run: `python3 scripts/validate-release-version.py --tag v0.2.8`

Expected: output `0.2.8`.

- [ ] **Step 3: Run all release gates**

Run:

```bash
cd apps/desktop-ui
npm test -- --run
npm run check
npm run build
cd ../..
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
python3 scripts/test-windows-runtime-setup.py
python3 scripts/test-installer-upgrade-contract.py
```

Expected: every command exits 0; frontend tests have 0 failures; Svelte has 0 diagnostics; Rust tests have 0 failures; Python contract tests report OK.

- [ ] **Step 4: Commit the version bump**

```bash
git add Cargo.lock apps/desktop-ui/package.json apps/desktop-ui/package-lock.json apps/desktop-ui/src-tauri/Cargo.toml apps/desktop-ui/src-tauri/tauri.conf.json crates/radsuite-desktop/Cargo.toml
git commit -m "chore: bump release version to 0.2.8"
```

- [ ] **Step 5: Review and integrate the feature branch**

Run a final spec-compliance and code-quality review. Merge `codex/application-menu-update` into `main` without rewriting the existing design commits, then rerun the critical frontend and release-version checks on `main`.

- [ ] **Step 6: Push and publish**

Push `main`, create annotated tag `v0.2.8` with message `RADsuite 0.2.8`, and push the tag. Monitor the `Stable release` GitHub Actions workflow until it completes or exposes an actionable failure.

- [ ] **Step 7: Verify the public release**

Verify that GitHub release `v0.2.8` is public (not draft/prerelease), contains signed Apple Silicon, Intel Mac, and Windows updater/install assets, and that `https://github.com/radicalmove/radsuite/releases/latest/download/latest.json` reports version `0.2.8` with all three supported platform keys.
