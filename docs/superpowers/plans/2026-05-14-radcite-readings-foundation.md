# RADcite Readings Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a functional local RADcite Readings workspace backed by module/week records and module-scoped reading entries.

**Architecture:** Add a small `CourseModule` domain and SQLite repository, then attach `ReferenceEntryType::Reading` entries to modules with reading-specific metadata. Expose this through Tauri commands and a Svelte Readings workspace that follows the existing References/Exports patterns.

**Tech Stack:** Rust workspace, SQLx SQLite migrations, Tauri commands, Svelte 5, TypeScript, Vitest, existing RADcite CSS tokens.

---

## File Structure

- Modify: `crates/radsuite-core/src/ids.rs`
  - Add `ModuleId`.
- Modify: `crates/radsuite-core/src/domain.rs`
  - Add `CourseModule` and `ReadingCategory`.
  - Extend `ReferenceEntry` with module/readings metadata.
- Modify: `crates/radsuite-core/src/lib.rs`
  - Export new id/domain types.
- Modify: `crates/radsuite-core/tests/domain_contracts.rs`
  - Cover serialization and defaults.
- Create: `crates/radsuite-db/migrations/0002_course_modules_readings.sql`
  - Add module table and reading metadata columns.
- Modify: `crates/radsuite-db/src/repositories.rs`
  - Add `CourseModuleRepository`.
  - Persist/hydrate reading metadata on `ReferenceEntry`.
  - Add module-scoped reading listing.
- Modify: `crates/radsuite-db/tests/repository_roundtrip.rs`
  - Cover module and reading roundtrips.
- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add command DTOs and local RADcite module/readings commands.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Cover command behavior and validation.
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
  - Register Tauri commands.
- Modify: `apps/desktop-ui/src/types.ts`
  - Add module and reading DTOs.
- Create: `apps/desktop-ui/src/lib/readingCommands.ts`
  - Tauri wrappers.
- Create: `apps/desktop-ui/src/lib/readingCommands.test.ts`
  - Wrapper tests.
- Create: `apps/desktop-ui/src/components/RadciteReadingsWorkspace.svelte`
  - Readings UI.
- Modify: `apps/desktop-ui/src/components/ProjectSidebar.svelte`
  - Enable Readings.
- Modify: `apps/desktop-ui/src/App.svelte`
  - Load modules/readings and route the new workspace.
- Modify: `apps/desktop-ui/src/styles.css`
  - Style reading workspace controls and grouped list.
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
  - Include Readings route/component/style checks.

## Task 1: Core Domain

**Files:**
- Modify: `crates/radsuite-core/src/ids.rs`
- Modify: `crates/radsuite-core/src/domain.rs`
- Modify: `crates/radsuite-core/src/lib.rs`
- Test: `crates/radsuite-core/tests/domain_contracts.rs`

- [ ] **Step 1: Write failing domain tests**

Add tests for:

```rust
let module = CourseModule::new(project_id, "Module 1", Some(1));
assert_eq!(module.title, "Module 1");
assert_eq!(module.order_index, Some(1));

let mut reading = ReferenceEntry::new(project_id, ReferenceEntryType::Reading);
reading.module_id = Some(module.id);
reading.reading_category = Some(ReadingCategory::Optional);
reading.lesson_code = Some("2.3".to_string());
reading.reading_notes = Some("Read before workshop".to_string());
reading.estimated_reading_time = Some("20 minutes".to_string());
let json = serde_json::to_string(&reading).expect("serialize reading");
assert!(json.contains("optional"));
```

- [ ] **Step 2: Run core test and verify RED**

Run:

```bash
cargo test -p radsuite-core --test domain_contracts course_modules_and_reading_metadata_are_serializable
```

Expected: FAIL because `CourseModule`, `ModuleId`, and `ReadingCategory` do not exist yet.

- [ ] **Step 3: Implement minimal core types**

Add:

```rust
id_type!(ModuleId);
```

Add `CourseModule`, `ReadingCategory`, and default `None` fields on `ReferenceEntry`.

- [ ] **Step 4: Run core test and verify GREEN**

Run:

```bash
cargo test -p radsuite-core --test domain_contracts course_modules_and_reading_metadata_are_serializable
```

Expected: PASS.

- [ ] **Step 5: Run full core tests**

Run:

```bash
cargo test -p radsuite-core
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/radsuite-core/src/ids.rs crates/radsuite-core/src/domain.rs crates/radsuite-core/src/lib.rs crates/radsuite-core/tests/domain_contracts.rs
git commit -m "feat: add RADcite module reading domain"
```

## Task 2: SQLite Persistence

**Files:**
- Create: `crates/radsuite-db/migrations/0002_course_modules_readings.sql`
- Modify: `crates/radsuite-db/src/repositories.rs`
- Test: `crates/radsuite-db/tests/repository_roundtrip.rs`

- [ ] **Step 1: Write failing repository tests**

Add tests for:

- inserting and listing modules for a project
- inserting a reading attached to a module
- listing readings for that module only
- preserving reading category, lesson code, URL, notes, and estimated reading time

- [ ] **Step 2: Run repository tests and verify RED**

Run:

```bash
cargo test -p radsuite-db --test repository_roundtrip module_readings
```

Expected: FAIL because repository methods and DB columns do not exist.

- [ ] **Step 3: Add migration**

Create `0002_course_modules_readings.sql`:

```sql
CREATE TABLE course_modules (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  code TEXT,
  title TEXT NOT NULL,
  order_index INTEGER,
  description TEXT,
  archived_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_course_modules_project_order
ON course_modules(project_id, order_index, title);

ALTER TABLE reference_entries ADD COLUMN module_id TEXT REFERENCES course_modules(id);
ALTER TABLE reference_entries ADD COLUMN lesson_code TEXT;
ALTER TABLE reference_entries ADD COLUMN reading_category TEXT;
ALTER TABLE reference_entries ADD COLUMN reading_notes TEXT;
ALTER TABLE reference_entries ADD COLUMN estimated_reading_time TEXT;

CREATE INDEX idx_reference_entries_module_type
ON reference_entries(module_id, reference_type);
```

- [ ] **Step 4: Implement repository support**

Add `CourseModuleRepository` and `SqliteCourseModuleRepository`.

Extend `ReferenceEntryRepository` with:

```rust
async fn list_reference_entries_for_module(
    &self,
    module_id: ModuleId,
    reference_type: ReferenceEntryType,
) -> Result<Vec<ReferenceEntry>, DbError>;
```

Update insert/select SQL and row mapping for the new fields.

- [ ] **Step 5: Run repository tests and verify GREEN**

Run:

```bash
cargo test -p radsuite-db --test repository_roundtrip module_readings
```

Expected: PASS.

- [ ] **Step 6: Run full DB tests**

Run:

```bash
cargo test -p radsuite-db
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/radsuite-db/migrations/0002_course_modules_readings.sql crates/radsuite-db/src/repositories.rs crates/radsuite-db/tests/repository_roundtrip.rs
git commit -m "feat: persist RADcite module readings"
```

## Task 3: Desktop Commands

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Test: `crates/radsuite-desktop/tests/desktop_contracts.rs`

- [ ] **Step 1: Write failing desktop command tests**

Add tests for:

- `add_radcite_module` creates a module under the local RADcite project
- `list_radcite_modules` returns modules in order
- `add_module_reading` stores a compulsory or optional reading
- `list_module_readings` returns only readings for the selected module
- empty module title is rejected
- empty reading text is rejected
- invalid reading category is rejected

- [ ] **Step 2: Run desktop tests and verify RED**

Run:

```bash
cargo test -p radsuite-desktop --test desktop_contracts module_readings
```

Expected: FAIL because the commands do not exist.

- [ ] **Step 3: Implement command DTOs and errors**

Add:

- `CourseModuleSummary`
- `AddRadciteModuleRequest`
- `ModuleReadingSummary`
- `ListModuleReadingsRequest`
- `AddModuleReadingRequest`
- `RadciteModuleError`
- `ModuleReadingError`

- [ ] **Step 4: Implement command functions**

Add:

- `list_radcite_modules`
- `add_radcite_module`
- `list_module_readings`
- `add_module_reading`

Use `load_or_create_local_radcite_project`.

- [ ] **Step 5: Run desktop tests and verify GREEN**

Run:

```bash
cargo test -p radsuite-desktop --test desktop_contracts module_readings
```

Expected: PASS.

- [ ] **Step 6: Run full desktop tests**

Run:

```bash
cargo test -p radsuite-desktop
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/radsuite-desktop/src/commands.rs crates/radsuite-desktop/tests/desktop_contracts.rs
git commit -m "feat: add RADcite readings desktop commands"
```

## Task 4: Tauri Bridge and TypeScript Wrappers

**Files:**
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Modify: `apps/desktop-ui/src/types.ts`
- Create: `apps/desktop-ui/src/lib/readingCommands.ts`
- Create: `apps/desktop-ui/src/lib/readingCommands.test.ts`

- [ ] **Step 1: Write failing wrapper tests**

Test that:

- `listRadciteModules()` invokes `list_radcite_modules`
- `addRadciteModule()` trims title/code/description
- `listModuleReadings(moduleId)` invokes `list_module_readings`
- `addModuleReading()` trims text fields and preserves category

- [ ] **Step 2: Run wrapper tests and verify RED**

Run:

```bash
npm test -- --run src/lib/readingCommands.test.ts
```

Expected: FAIL because `readingCommands.ts` does not exist.

- [ ] **Step 3: Register Tauri commands**

Add command wrappers in `src-tauri/src/main.rs` and include them in `generate_handler!`.

- [ ] **Step 4: Add TypeScript types and wrappers**

Add matching DTOs to `types.ts` and create `readingCommands.ts`.

- [ ] **Step 5: Run wrapper tests and verify GREEN**

Run:

```bash
npm test -- --run src/lib/readingCommands.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop-ui/src-tauri/src/main.rs apps/desktop-ui/src/types.ts apps/desktop-ui/src/lib/readingCommands.ts apps/desktop-ui/src/lib/readingCommands.test.ts
git commit -m "feat: bridge RADcite readings commands"
```

## Task 5: Readings Workspace UI

**Files:**
- Create: `apps/desktop-ui/src/components/RadciteReadingsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/ProjectSidebar.svelte`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/styles.css`
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`

- [ ] **Step 1: Write failing style contract checks**

Add checks for:

- `RadciteReadingsWorkspace` imported/rendered in `App.svelte`
- `activeArea === "readings"`
- sidebar `Readings` is enabled
- workspace includes `Module readings`, `Compulsory`, and `Optional`
- CSS includes `.readings-workspace`, `.module-selector`, and `.reading-list-panel`

- [ ] **Step 2: Run style contract and verify RED**

Run:

```bash
npm run test:style
```

Expected: FAIL because the workspace does not exist and Readings is disabled.

- [ ] **Step 3: Create Readings workspace component**

Implement:

- header and refresh button
- module selector
- add module form
- add reading form
- grouped compulsory/optional reading list
- loading, empty, and error states

- [ ] **Step 4: Wire App state**

Add:

- module state
- selected module state
- reading state
- refresh/add handlers
- route for `activeArea === "readings"`

Refresh modules/readings when entering Readings.

- [ ] **Step 5: Enable sidebar route**

Remove the disabled flag from the Readings nav item.

- [ ] **Step 6: Add focused CSS**

Style the module selector, reading form, and grouped list using existing tokens.

- [ ] **Step 7: Run style contract and verify GREEN**

Run:

```bash
npm run test:style
```

Expected: PASS.

- [ ] **Step 8: Run frontend tests and build**

Run:

```bash
npm test -- --run
npm run build
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop-ui/src/components/RadciteReadingsWorkspace.svelte apps/desktop-ui/src/components/ProjectSidebar.svelte apps/desktop-ui/src/App.svelte apps/desktop-ui/src/styles.css apps/desktop-ui/scripts/verify-style-contract.mjs
git commit -m "feat: add RADcite readings workspace"
```

## Task 6: Full Verification and Browser Smoke

**Files:**
- No planned source edits unless verification exposes a bug.

- [ ] **Step 1: Run Rust formatting and linting**

Run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 2: Run Rust tests**

Run:

```bash
cargo test --workspace --all-features
```

Expected: PASS.

- [ ] **Step 3: Run frontend verification**

Run:

```bash
npm run test:style
npm test -- --run
npm run build
```

Expected: PASS.

- [ ] **Step 4: Browser smoke-test Readings**

Run Vite preview or dev server, open the app, and verify:

- Readings is clickable
- module creation form renders
- reading form renders
- compulsory/optional groups render without overlap

- [ ] **Step 5: Fix any verification failures**

Use TDD for behavioral bugs. Re-run the failing check and the relevant full suite after each fix.

- [ ] **Step 6: Final commit if needed**

If verification fixes changed files:

```bash
git add <changed-files>
git commit -m "fix: polish RADcite readings foundation"
```
