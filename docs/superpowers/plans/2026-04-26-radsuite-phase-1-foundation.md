# RADsuite Phase 1 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first Rust RADsuite foundation: shared domain model, local database, Rust server skeleton, Tauri desktop skeleton, sync primitives, engine adapter stubs, packaging skeleton, and CI.

**Architecture:** Use one Rust workspace with small crates for shared core, database, sync, server, engine adapters, and desktop command logic. The desktop app is a Tauri installable shell backed by Rust commands and local SQLite; the server is an Axum API for accounts, projects, sharing, assets, sync, health, and admin visibility. This phase does not port RADcite, RADcast, or RADTTS feature parity yet; it creates the tested foundation those ports will use.

**Tech Stack:** Rust stable, Cargo workspace, Tauri 2, TypeScript/Vite UI, Axum 0.8, Tokio, SQLx 0.8 with SQLite/Postgres features, UUIDs, Serde, Argon2 password hashing, tower-http tracing/CORS, GitHub Actions.

---

## Source Context

Approved design spec: `docs/superpowers/specs/2026-04-25-radsuite-rust-consolidation-design.md`

Existing app references:

- RADcite: `/Users/rcd58/citation-checker`
- RADcast: `/Users/rcd58/RADcast`
- RADTTS: `/Users/rcd58/RADTTS`

Implementation target:

- New repository/workspace root: `/Users/rcd58/Documents/New project`
- Later GitHub remote: `https://github.com/radicalmove/RADsuite.git` or final repository name chosen by Richard

## Scope Boundary

This plan builds Phase 1 only.

Included:

- Rust workspace
- Shared types and API contracts
- Local SQLite schema and repositories
- Server health/auth/project/member/asset/sync skeleton
- Desktop Tauri skeleton and command bridge
- Local app directory discovery
- Engine registry stubs
- Platform packaging skeleton
- CI and developer documentation

Excluded:

- RADcite document extraction/citation checking
- RADcast audio enhancement
- RADTTS voice cloning/synthesis
- Production model downloads
- Real file upload streaming beyond metadata stubs
- Existing data migration
- Public release signing/notarisation

## File Structure

Create or modify these files:

- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `.editorconfig`
- Create: `.cargo/config.toml`
- Create: `crates/radsuite-core/Cargo.toml`
- Create: `crates/radsuite-core/src/lib.rs`
- Create: `crates/radsuite-core/src/ids.rs`
- Create: `crates/radsuite-core/src/roles.rs`
- Create: `crates/radsuite-core/src/domain.rs`
- Create: `crates/radsuite-core/src/api.rs`
- Create: `crates/radsuite-core/tests/domain_contracts.rs`
- Create: `crates/radsuite-db/Cargo.toml`
- Create: `crates/radsuite-db/src/lib.rs`
- Create: `crates/radsuite-db/src/error.rs`
- Create: `crates/radsuite-db/src/migrate.rs`
- Create: `crates/radsuite-db/src/repositories.rs`
- Create: `crates/radsuite-db/migrations/0001_foundation.sql`
- Create: `crates/radsuite-db/tests/repository_roundtrip.rs`
- Create: `crates/radsuite-sync/Cargo.toml`
- Create: `crates/radsuite-sync/src/lib.rs`
- Create: `crates/radsuite-sync/src/change.rs`
- Create: `crates/radsuite-sync/src/conflict.rs`
- Create: `crates/radsuite-sync/src/asset_manifest.rs`
- Create: `crates/radsuite-sync/tests/sync_contracts.rs`
- Create: `crates/radsuite-engines/Cargo.toml`
- Create: `crates/radsuite-engines/src/lib.rs`
- Create: `crates/radsuite-engines/src/registry.rs`
- Create: `crates/radsuite-engines/src/capabilities.rs`
- Create: `crates/radsuite-engines/tests/registry.rs`
- Create: `crates/radsuite-server/Cargo.toml`
- Create: `crates/radsuite-server/src/main.rs`
- Create: `crates/radsuite-server/src/lib.rs`
- Create: `crates/radsuite-server/src/config.rs`
- Create: `crates/radsuite-server/src/state.rs`
- Create: `crates/radsuite-server/src/routes/mod.rs`
- Create: `crates/radsuite-server/src/routes/health.rs`
- Create: `crates/radsuite-server/src/routes/auth.rs`
- Create: `crates/radsuite-server/src/routes/projects.rs`
- Create: `crates/radsuite-server/src/routes/assets.rs`
- Create: `crates/radsuite-server/src/routes/sync.rs`
- Create: `crates/radsuite-server/tests/server_contracts.rs`
- Create: `crates/radsuite-desktop/Cargo.toml`
- Create: `crates/radsuite-desktop/src/lib.rs`
- Create: `crates/radsuite-desktop/src/app_paths.rs`
- Create: `crates/radsuite-desktop/src/commands.rs`
- Create: `crates/radsuite-desktop/src/state.rs`
- Create: `crates/radsuite-desktop/tests/desktop_contracts.rs`
- Create: `apps/desktop-ui/package.json`
- Create: `apps/desktop-ui/index.html`
- Create: `apps/desktop-ui/src/main.tsx`
- Create: `apps/desktop-ui/src/App.tsx`
- Create: `apps/desktop-ui/src/styles.css`
- Create: `apps/desktop-ui/src-tauri/Cargo.toml`
- Create: `apps/desktop-ui/src-tauri/src/main.rs`
- Create: `apps/desktop-ui/src-tauri/tauri.conf.json`
- Create: `apps/desktop-ui/tsconfig.json`
- Create: `apps/desktop-ui/vite.config.ts`
- Create: `packaging/macos/README.md`
- Create: `packaging/windows/README.md`
- Create: `.github/workflows/ci.yml`
- Create: `docs/development.md`
- Create: `docs/server-alpha-ops.md`
- Create: `.env.example`

## Task 1: Workspace Baseline

**Files:**

- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `.editorconfig`
- Create: `.cargo/config.toml`
- Create: `docs/development.md`

- [ ] **Step 1: Create workspace files**

Add this root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/radsuite-core",
  "crates/radsuite-db",
  "crates/radsuite-sync",
  "crates/radsuite-engines",
  "crates/radsuite-server",
  "crates/radsuite-desktop",
  "apps/desktop-ui/src-tauri",
]

[workspace.package]
edition = "2024"
license = "MIT"
repository = "https://github.com/radicalmove/RADsuite"
rust-version = "1.88"

[workspace.dependencies]
anyhow = "1"
argon2 = "0.5"
async-trait = "0.1"
axum = "0.8"
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
password-hash = "0.5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls-ring-native-roots", "sqlite", "postgres", "uuid", "chrono", "json", "migrate"] }
thiserror = "2"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
uuid = { version = "1", features = ["serde", "v4", "v7"] }
```

Add `rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

Add `.gitignore`:

```gitignore
/target/
/.env
/.env.*
!.env.example
/apps/desktop-ui/node_modules/
/apps/desktop-ui/dist/
/apps/desktop-ui/src-tauri/target/
/.DS_Store
Thumbs.db
*.log
*.sqlite
*.sqlite3
```

Add `.cargo/config.toml`:

```toml
[alias]
xtest = "test --workspace --all-features"
xclippy = "clippy --workspace --all-targets --all-features -- -D warnings"
xfmt = "fmt --all --check"
```

- [ ] **Step 2: Add developer doc stub**

Create `docs/development.md` with:

````markdown
# RADsuite Development

## Local Checks

Run from the repository root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Scope

This repository is the new Rust/Tauri implementation of RADsuite. The existing Python apps remain reference implementations only.
````

- [ ] **Step 3: Verify baseline formatting command**

Run:

```bash
cargo fmt --all --check
```

Expected: It may fail until crates exist. Confirm the error is only that no Rust packages exist yet.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore .editorconfig .cargo/config.toml docs/development.md
git commit -m "chore: initialise RADsuite Rust workspace"
```

## Task 2: Shared Domain Crate

**Files:**

- Create: `crates/radsuite-core/Cargo.toml`
- Create: `crates/radsuite-core/src/lib.rs`
- Create: `crates/radsuite-core/src/ids.rs`
- Create: `crates/radsuite-core/src/roles.rs`
- Create: `crates/radsuite-core/src/domain.rs`
- Create: `crates/radsuite-core/src/api.rs`
- Create: `crates/radsuite-core/tests/domain_contracts.rs`

- [ ] **Step 1: Write failing domain contract tests**

Create `crates/radsuite-core/tests/domain_contracts.rs`:

```rust
use radsuite_core::{
    ApiProjectSummary, Project, ProjectRole, UserId,
};

#[test]
fn ids_are_serializable_uuid_wrappers() {
    let id = UserId::new();
    let encoded = serde_json::to_string(&id).expect("serialize id");
    let decoded: UserId = serde_json::from_str(&encoded).expect("deserialize id");
    assert_eq!(id, decoded);
}

#[test]
fn project_owner_can_be_returned_as_api_summary() {
    let owner_id = UserId::new();
    let project = Project::new("COMS435", "Good data and how to use it", owner_id);
    let summary = ApiProjectSummary::from_project(&project, ProjectRole::Owner);

    assert_eq!(summary.id, project.id);
    assert_eq!(summary.code.as_deref(), Some("COMS435"));
    assert_eq!(summary.title, "Good data and how to use it");
    assert_eq!(summary.role, ProjectRole::Owner);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p radsuite-core --test domain_contracts
```

Expected: FAIL because `radsuite-core` does not exist.

- [ ] **Step 3: Implement crate and domain types**

Create `crates/radsuite-core/Cargo.toml`:

```toml
[package]
name = "radsuite-core"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
chrono.workspace = true
serde.workspace = true
uuid.workspace = true
```

Create `src/lib.rs`:

```rust
pub mod api;
pub mod domain;
pub mod ids;
pub mod roles;

pub use api::*;
pub use domain::*;
pub use ids::*;
pub use roles::*;
```

Create `src/ids.rs`:

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

id_type!(UserId);
id_type!(ProjectId);
id_type!(AssetId);
id_type!(DocumentId);
id_type!(ParagraphId);
id_type!(CitationId);
id_type!(ReferenceEntryId);
id_type!(JobId);
id_type!(SyncRecordId);
```

Create `src/roles.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    Owner,
    Editor,
    Viewer,
}

impl ProjectRole {
    pub fn can_edit(self) -> bool {
        matches!(self, Self::Owner | Self::Editor)
    }
}
```

Create `src/domain.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ProjectId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub owner_id: UserId,
    pub code: Option<String>,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    pub fn new(code: impl Into<String>, title: impl Into<String>, owner_id: UserId) -> Self {
        let now = Utc::now();
        let code = code.into();
        Self {
            id: ProjectId::new(),
            owner_id,
            code: (!code.trim().is_empty()).then_some(code),
            title: title.into(),
            created_at: now,
            updated_at: now,
        }
    }
}
```

Create `src/api.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::{Project, ProjectId, ProjectRole};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiProjectSummary {
    pub id: ProjectId,
    pub code: Option<String>,
    pub title: String,
    pub role: ProjectRole,
}

impl ApiProjectSummary {
    pub fn from_project(project: &Project, role: ProjectRole) -> Self {
        Self {
            id: project.id,
            code: project.code.clone(),
            title: project.title.clone(),
            role,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p radsuite-core --test domain_contracts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/radsuite-core Cargo.toml
git commit -m "feat: add shared RADsuite domain model"
```

## Task 3: Local Database Crate

**Files:**

- Create: `crates/radsuite-db/Cargo.toml`
- Create: `crates/radsuite-db/src/lib.rs`
- Create: `crates/radsuite-db/src/error.rs`
- Create: `crates/radsuite-db/src/migrate.rs`
- Create: `crates/radsuite-db/src/repositories.rs`
- Create: `crates/radsuite-db/migrations/0001_foundation.sql`
- Create: `crates/radsuite-db/tests/repository_roundtrip.rs`

- [ ] **Step 1: Write failing repository roundtrip test**

Create `crates/radsuite-db/tests/repository_roundtrip.rs`:

```rust
use radsuite_core::{Project, ProjectRole, UserId};
use radsuite_db::{migrate, ProjectRepository, SqliteProjectRepository};
use sqlx::SqlitePool;

#[tokio::test]
async fn project_can_be_inserted_and_listed_for_owner() {
    let pool = SqlitePool::connect("sqlite::memory:").await.expect("connect");
    migrate(&pool).await.expect("migrate");

    let repo = SqliteProjectRepository::new(pool);
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);

    repo.insert_project(&project).await.expect("insert project");
    let rows = repo
        .list_projects_for_user(owner_id)
        .await
        .expect("list projects");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "Legal Method");
    assert_eq!(rows[0].role, ProjectRole::Owner);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p radsuite-db --test repository_roundtrip
```

Expected: FAIL because `radsuite-db` does not exist.

- [ ] **Step 3: Implement migration and repository skeleton**

Create `crates/radsuite-db/Cargo.toml`:

```toml
[package]
name = "radsuite-db"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
async-trait.workspace = true
chrono.workspace = true
radsuite-core = { path = "../radsuite-core" }
sqlx.workspace = true
thiserror.workspace = true
uuid.workspace = true
```

Create `migrations/0001_foundation.sql`:

```sql
CREATE TABLE users (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1,
  is_admin INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL REFERENCES users(id),
  code TEXT,
  title TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE project_members (
  project_id TEXT NOT NULL REFERENCES projects(id),
  user_id TEXT NOT NULL REFERENCES users(id),
  role TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, user_id)
);

CREATE TABLE assets (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  sha256 TEXT NOT NULL,
  byte_size INTEGER NOT NULL,
  mime_type TEXT NOT NULL,
  original_name TEXT NOT NULL,
  sync_policy TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE sync_records (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  operation TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  local_created_at TEXT NOT NULL,
  server_applied_at TEXT
);
```

Create `src/lib.rs`:

```rust
pub mod error;
pub mod migrate;
pub mod repositories;

pub use error::*;
pub use migrate::migrate;
pub use repositories::*;
```

Create `src/migrate.rs`:

```rust
use sqlx::SqlitePool;

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await
}
```

Create `src/repositories.rs` with a simple insert/list implementation. Store UUIDs as strings and timestamps as RFC3339 strings. When inserting a project, also insert the owner into `project_members` with role `owner`.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p radsuite-db --test repository_roundtrip
```

Expected: PASS.

- [ ] **Step 5: Run full core/db tests**

Run:

```bash
cargo test -p radsuite-core -p radsuite-db
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/radsuite-db Cargo.toml
git commit -m "feat: add local database foundation"
```

## Task 4: Sync Primitives

**Files:**

- Create: `crates/radsuite-sync/Cargo.toml`
- Create: `crates/radsuite-sync/src/lib.rs`
- Create: `crates/radsuite-sync/src/change.rs`
- Create: `crates/radsuite-sync/src/conflict.rs`
- Create: `crates/radsuite-sync/src/asset_manifest.rs`
- Create: `crates/radsuite-sync/tests/sync_contracts.rs`

- [ ] **Step 1: Write failing sync contract tests**

Create tests that assert:

- A local change serializes as JSON with `entity_type`, `entity_id`, and `operation`.
- An asset manifest includes `sha256`, `byte_size`, `mime_type`, `sync_policy`.
- A conflict preserves both local and remote payloads.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p radsuite-sync --test sync_contracts
```

Expected: FAIL because `radsuite-sync` does not exist.

- [ ] **Step 3: Implement sync DTOs**

Use Serde DTOs only in this phase:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperation {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalChange {
    pub project_id: ProjectId,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: SyncOperation,
    pub payload: serde_json::Value,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p radsuite-sync --test sync_contracts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/radsuite-sync Cargo.toml
git commit -m "feat: add sync contract primitives"
```

## Task 5: Server Health And Configuration

**Files:**

- Create: `crates/radsuite-server/Cargo.toml`
- Create: `crates/radsuite-server/src/main.rs`
- Create: `crates/radsuite-server/src/lib.rs`
- Create: `crates/radsuite-server/src/config.rs`
- Create: `crates/radsuite-server/src/state.rs`
- Create: `crates/radsuite-server/src/routes/mod.rs`
- Create: `crates/radsuite-server/src/routes/health.rs`
- Create: `crates/radsuite-server/tests/server_contracts.rs`
- Create: `.env.example`

- [ ] **Step 1: Write failing server health test**

Create `crates/radsuite-server/tests/server_contracts.rs`:

```rust
use axum::{body::Body, http::Request};
use radsuite_server::{build_router, AppConfig, AppState};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let state = AppState::for_tests().await;
    let app = build_router(state, AppConfig::test());

    let response = app
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p radsuite-server --test server_contracts health_endpoint_returns_ok
```

Expected: FAIL because server crate does not exist.

- [ ] **Step 3: Implement Axum router and health route**

Implement:

- `AppConfig::from_env()`
- `AppConfig::test()`
- `AppState::for_tests()`
- `build_router(state, config)`
- `GET /healthz` returns JSON `{ "status": "ok" }`

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p radsuite-server --test server_contracts health_endpoint_returns_ok
```

Expected: PASS.

- [ ] **Step 5: Add `.env.example`**

```dotenv
RADSUITE_SERVER_BIND=127.0.0.1:8088
RADSUITE_DATABASE_URL=sqlite://radsuite-server.sqlite
RADSUITE_ASSET_ROOT=./data/assets
RADSUITE_BOOTSTRAP_ADMIN_EMAIL=admin@example.com
RADSUITE_BOOTSTRAP_ADMIN_PASSWORD=change-me
```

- [ ] **Step 6: Commit**

```bash
git add crates/radsuite-server .env.example Cargo.toml
git commit -m "feat: add Rust server health endpoint"
```

## Task 6: Server Auth Skeleton

**Files:**

- Modify: `crates/radsuite-core/src/api.rs`
- Modify: `crates/radsuite-server/src/routes/mod.rs`
- Create: `crates/radsuite-server/src/routes/auth.rs`
- Modify: `crates/radsuite-server/tests/server_contracts.rs`

- [ ] **Step 1: Write failing auth tests**

Add tests for:

- `POST /auth/register` creates a user for internal alpha.
- `POST /auth/login` returns a session token for correct credentials.
- `POST /auth/login` rejects bad credentials with 401.

Use request bodies:

```json
{ "email": "owner@example.com", "display_name": "Owner", "password": "correct horse battery staple" }
```

and:

```json
{ "email": "owner@example.com", "password": "correct horse battery staple" }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p radsuite-server --test server_contracts auth
```

Expected: FAIL because auth routes do not exist.

- [ ] **Step 3: Implement minimal auth**

Add API DTOs:

```rust
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
}

pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub struct LoginResponse {
    pub token: String,
}
```

Use Argon2 for password hashing. For Phase 1, an opaque random bearer token stored in memory is acceptable for tests. Add a plan comment in code that persistent sessions or refresh tokens must replace this before external release.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p radsuite-server --test server_contracts auth
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/radsuite-core crates/radsuite-server
git commit -m "feat: add alpha auth endpoints"
```

## Task 7: Project And Membership API Skeleton

**Files:**

- Modify: `crates/radsuite-core/src/api.rs`
- Modify: `crates/radsuite-server/src/routes/mod.rs`
- Create: `crates/radsuite-server/src/routes/projects.rs`
- Modify: `crates/radsuite-server/tests/server_contracts.rs`

- [ ] **Step 1: Write failing project API tests**

Add tests for:

- Authenticated user can create a project.
- Owner can list their project.
- Non-member cannot see a project.
- Owner can share a project with another registered user as `editor`.
- Shared user can list the project with role `editor`.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p radsuite-server --test server_contracts project
```

Expected: FAIL because project routes do not exist.

- [ ] **Step 3: Implement routes**

Create endpoints:

- `POST /projects`
- `GET /projects`
- `GET /projects/{project_id}`
- `POST /projects/{project_id}/members`
- `GET /admin/projects`

`GET /admin/projects` should only return data for admin users. Create an admin bootstrap helper, but keep it test-only until deployment configuration is implemented.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p radsuite-server --test server_contracts project
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/radsuite-core crates/radsuite-server
git commit -m "feat: add project sharing API skeleton"
```

## Task 8: Asset And Sync API Skeleton

**Files:**

- Modify: `crates/radsuite-core/src/api.rs`
- Modify: `crates/radsuite-server/src/routes/mod.rs`
- Create: `crates/radsuite-server/src/routes/assets.rs`
- Create: `crates/radsuite-server/src/routes/sync.rs`
- Modify: `crates/radsuite-server/tests/server_contracts.rs`

- [ ] **Step 1: Write failing asset/sync tests**

Add tests for:

- Project member can register an asset manifest.
- Non-member cannot register an asset manifest.
- Project member can push sync records.
- Project member can pull sync records after a cursor.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p radsuite-server --test server_contracts asset sync
```

Expected: FAIL because routes do not exist.

- [ ] **Step 3: Implement metadata-only endpoints**

Create endpoints:

- `POST /projects/{project_id}/assets`
- `GET /projects/{project_id}/assets`
- `POST /projects/{project_id}/sync/push`
- `GET /projects/{project_id}/sync/pull?after=...`

Do not implement file streaming yet. Return a clear `upload_required: true` field when an asset manifest has no server-side blob.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p radsuite-server --test server_contracts asset sync
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/radsuite-core crates/radsuite-server
git commit -m "feat: add asset and sync metadata APIs"
```

## Task 9: Engine Registry Stub

**Files:**

- Create: `crates/radsuite-engines/Cargo.toml`
- Create: `crates/radsuite-engines/src/lib.rs`
- Create: `crates/radsuite-engines/src/registry.rs`
- Create: `crates/radsuite-engines/src/capabilities.rs`
- Create: `crates/radsuite-engines/tests/registry.rs`

- [ ] **Step 1: Write failing engine registry test**

Test that the registry reports named engine slots for:

- `ffmpeg`
- `asr`
- `audio_cleanup`
- `tts`

Each engine should report `available: false` with a helpful reason until real detection is implemented.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p radsuite-engines --test registry
```

Expected: FAIL because engine crate does not exist.

- [ ] **Step 3: Implement registry structs**

Use:

```rust
pub struct EngineStatus {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub detail: String,
}
```

Add `EngineRegistry::default().list()` returning deterministic stubs.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p radsuite-engines --test registry
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/radsuite-engines Cargo.toml
git commit -m "feat: add native engine registry skeleton"
```

## Task 10: Desktop Rust Command Crate

**Files:**

- Create: `crates/radsuite-desktop/Cargo.toml`
- Create: `crates/radsuite-desktop/src/lib.rs`
- Create: `crates/radsuite-desktop/src/app_paths.rs`
- Create: `crates/radsuite-desktop/src/commands.rs`
- Create: `crates/radsuite-desktop/src/state.rs`
- Create: `crates/radsuite-desktop/tests/desktop_contracts.rs`

- [ ] **Step 1: Write failing desktop command tests**

Test:

- App paths resolve a platform-specific data directory for `RADsuite`.
- `get_app_status` returns app name, database status, sync status, and engine statuses.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p radsuite-desktop --test desktop_contracts
```

Expected: FAIL because desktop crate does not exist.

- [ ] **Step 3: Implement desktop state and commands**

Create command functions as plain Rust functions first. Do not require Tauri macros in tests.

```rust
pub struct AppStatus {
    pub app_name: String,
    pub database_ready: bool,
    pub sync_configured: bool,
    pub engines: Vec<EngineStatus>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p radsuite-desktop --test desktop_contracts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/radsuite-desktop Cargo.toml
git commit -m "feat: add desktop command foundation"
```

## Task 11: Tauri Desktop Shell

**Files:**

- Create: `apps/desktop-ui/package.json`
- Create: `apps/desktop-ui/index.html`
- Create: `apps/desktop-ui/src/main.tsx`
- Create: `apps/desktop-ui/src/App.tsx`
- Create: `apps/desktop-ui/src/styles.css`
- Create: `apps/desktop-ui/src-tauri/Cargo.toml`
- Create: `apps/desktop-ui/src-tauri/src/main.rs`
- Create: `apps/desktop-ui/src-tauri/tauri.conf.json`
- Create: `apps/desktop-ui/tsconfig.json`
- Create: `apps/desktop-ui/vite.config.ts`

- [ ] **Step 1: Create minimal UI**

Build a restrained internal-alpha UI with:

- App title `RADsuite`
- Server status placeholder
- Local database status placeholder
- Engine status list
- Project list placeholder

Avoid marketing copy. The first screen should be the working shell, not a landing page.

- [ ] **Step 2: Wire Tauri command**

Expose `get_app_status` from `radsuite-desktop` through the Tauri wrapper in `apps/desktop-ui/src-tauri/src/main.rs`.

- [ ] **Step 3: Run Rust build**

Run:

```bash
cargo check -p radsuite-tauri
```

Expected: PASS.

- [ ] **Step 4: Run frontend install and build**

Run:

```bash
cd apps/desktop-ui
npm install
npm run build
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop-ui Cargo.toml
git commit -m "feat: add Tauri desktop shell"
```

## Task 12: Packaging Skeletons

**Files:**

- Create: `packaging/macos/README.md`
- Create: `packaging/windows/README.md`
- Modify: `apps/desktop-ui/src-tauri/tauri.conf.json`

- [ ] **Step 1: Add macOS packaging notes**

Document:

- Apple Silicon-only target for first release
- Code signing requirement
- Notarisation requirement
- Bundled sidecar location
- App data directory expectation

- [ ] **Step 2: Add Windows packaging notes**

Document:

- Windows 11 x64 target
- Installer format choice to confirm
- Defender/code signing concerns
- Sidecar binary location
- App data directory expectation

- [ ] **Step 3: Verify Tauri config has product metadata**

Confirm `tauri.conf.json` includes product name `RADsuite`, bundle identifier placeholder, macOS bundle config, and Windows bundle config.

- [ ] **Step 4: Commit**

```bash
git add packaging apps/desktop-ui/src-tauri/tauri.conf.json
git commit -m "docs: add platform packaging skeleton"
```

## Task 13: CI

**Files:**

- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Add GitHub Actions workflow**

Create a workflow that runs on pull requests and pushes to `main`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - run: cargo test --workspace --all-features

  desktop-ui:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: apps/desktop-ui
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: apps/desktop-ui/package-lock.json
      - run: npm ci
      - run: npm run build
```

- [ ] **Step 2: Run local equivalent**

Run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd apps/desktop-ui && npm run build
```

Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add Rust and desktop UI checks"
```

## Task 14: Alpha Server Operations Doc

**Files:**

- Create: `docs/server-alpha-ops.md`

- [ ] **Step 1: Document Ubuntu Mac Mini deployment assumptions**

Include:

- Server runs on Ubuntu Mac Mini for internal alpha.
- Reverse proxy will be nginx.
- Process manager will be systemd.
- Database can start as SQLite for earliest alpha but should move to Postgres before real multi-user testing if server-side concurrency grows.
- Assets can start on local disk behind `RADSUITE_ASSET_ROOT`.
- Backups must cover database and asset root.

- [ ] **Step 2: Add initial verification commands**

Use:

```bash
systemctl status radsuite --no-pager
curl -fsS http://127.0.0.1:8088/healthz
```

- [ ] **Step 3: Commit**

```bash
git add docs/server-alpha-ops.md
git commit -m "docs: add alpha server operations notes"
```

## Task 15: Final Phase 1 Verification

**Files:**

- Modify only if verification exposes issues.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --all --check
```

Expected: PASS.

- [ ] **Step 2: Run clippy**

Run:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 3: Run Rust tests**

Run:

```bash
cargo test --workspace --all-features
```

Expected: PASS.

- [ ] **Step 4: Run desktop UI build**

Run:

```bash
cd apps/desktop-ui
npm run build
```

Expected: PASS.

- [ ] **Step 5: Confirm git state**

Run:

```bash
git status --short
```

Expected: clean working tree.

## Review Notes

The plan review subagent step from the writing-plans workflow was not run while creating this document because the active Codex tool policy only permits subagents when the user explicitly asks for delegation. If a reviewer is desired, ask for a subagent plan review before implementation starts.

## Execution Handoff

Recommended execution mode: inline execution for Task 1 through Task 3, then reassess whether to use parallel workers for independent crates. The first three tasks establish the workspace and contracts that later tasks depend on.
