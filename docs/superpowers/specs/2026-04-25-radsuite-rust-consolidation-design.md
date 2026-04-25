# RADsuite Rust Consolidation Design

## Status

Draft approved in conversation on 2026-04-25 for initial planning. This document captures the agreed direction before implementation planning.

## Existing Applications

RADsuite will consolidate three existing applications:

- RADcite, currently in `/Users/rcd58/citation-checker`, GitHub remote `https://github.com/radicalmove/citation-checker`
- RADcast, currently in `/Users/rcd58/RADcast`, GitHub remote `https://github.com/radicalmove/RADcast.git`
- RADTTS, currently in `/Users/rcd58/RADTTS`, GitHub remote `https://github.com/radicalmove/RADTTS.git`

The existing applications are Python web applications. RADcast and RADTTS are already local-first FastAPI packages with project folders, manifests, long-running jobs, and worker queues. RADcite is a Flask and SQLAlchemy application with document, citation, course, module, reference, user, and sharing models.

## Core Decision

RADsuite will be a new Rust implementation rather than a Python wrapper around the current applications.

The shipped desktop app must be Python-free. Rust owns the application architecture, local database, job orchestration, sync, permissions, platform integration, packaging, and server communication. Native sidecars are allowed where technically justified, such as `ffmpeg`, `whisper.cpp`, ONNX Runtime, or platform-specific model/runtime binaries.

The goal is not "pure Rust at all costs". The goal is a maintainable, installable, cross-platform product where Rust is the product layer and native tools handle specialised processing.

## Product Shape

RADsuite will be one installable desktop app with one shared Course/Project model.

A course/project is the top-level object and contains:

- Citation-checking work: documents, paragraphs, citations, references, readings, validation results, reports, exports
- Audio cleanup work: source audio, enhancement jobs, captions, manifests, selected shared outputs
- Voice/TTS work: scripts, reference audio, voice profiles, generated narration, captions, quality reports, manifests
- Access control: owner, members, roles, admin visibility
- Sync state: local changes, server version, conflicts, asset upload/download state

Users should not need separate RADcite, RADcast, and RADTTS projects. They should create or open one project and use the relevant work areas inside it.

## Desktop App

The desktop app will use Tauri.

This gives users a normal installable app:

- `RADsuite.app` on macOS
- `RADsuite.exe` or installed Start Menu app on Windows

The UI runs in the operating system webview, but the app is not browser-based from the user's perspective. There is no Chrome/Safari/Edge workflow and no local web server UI.

The desktop app is responsible for:

- Local SQLite database
- Local asset store
- Offline-capable project access after login
- Local job orchestration and cancellation
- Calling native processing engines
- Syncing records and selected assets with the server
- Displaying conflicts and sync status
- Managing settings, logs, model downloads, and runtime diagnostics
- Platform-specific packaging and updates

## Supported Platforms

Day-one desktop platforms:

- macOS 14+ on Apple Silicon
- Windows 11 x64

macOS Intel is not required for the first release.

Both platforms should be treated as first-class targets rather than one generic build. The shared Rust core should be common, but packaging, hardware/runtime detection, filesystem integration, native sidecars, signing, and update flows should be optimised per platform.

Expected platform concerns:

- macOS: Apple Silicon detection, app bundle, code signing, notarisation, app data directories, possible Metal/Core ML/Accelerate-backed runtimes where useful
- Windows: installer, Start Menu integration, app data directories, Windows Defender friction, DirectML/CUDA/CPU runtime choices where useful

## Server

The server will also be rewritten in Rust.

For internal alpha, it will run on the existing Ubuntu Mac Mini server. GitHub will remain the source of truth for code backup and collaboration. A new repository should be created when implementation starts, likely `radicalmove/RADsuite` or `radicalmove/radsuite`.

The server is not the heavy processing engine. Its responsibilities are:

- RADsuite-managed accounts
- Password authentication
- Admin role and active/inactive user state
- Project ownership and sharing
- Project membership and permissions
- Admin visibility into all projects
- Sync coordination
- Asset manifests and selected file storage
- Resumable upload/download APIs
- Audit logging
- Update and model/runtime manifest delivery
- Operational health checks and backups

The server should be designed so it can move from the Ubuntu Mac Mini to a cloud host later without changing the desktop app architecture.

## Accounts And Permissions

RADsuite will use its own account system for the first server.

Initial account model:

- Users have email or username login, password hash, display name, active state, and admin flag.
- Admin users can see all courses/projects.
- Regular users see only projects they own or projects shared with them.
- Sharing is project-level at first.
- Project roles should include at least owner, editor, and viewer.
- Audit events should record security and collaboration actions.

The design should keep authentication boundaries clear enough to add institutional SSO later, but SSO is not a first-release requirement.

## Offline And Sync Model

RADsuite should be offline-first after login.

Once a user has logged in and synced their accessible project list, they can open local projects and perform local work without server connectivity. Sharing, admin views, inviting users, and cross-device sync require the server.

Sync model:

- The desktop app keeps local SQLite records with stable UUIDs.
- The server stores canonical account, project, membership, audit, sync, and selected asset state.
- Records created offline are assigned UUIDs locally and uploaded later.
- Small structured records sync automatically.
- Source assets needed for collaboration sync by default.
- Large generated outputs stay local unless explicitly marked for upload/share.
- Conflicts are tracked per record.
- For alpha, conflict handling should preserve both edits and ask the user to resolve, rather than silently merging complex records.

Default sync behaviour:

- Sync automatically: project metadata, permissions, document records, paragraph records, citations, references, scripts, captions, reports, settings, job summaries, manifests
- Sync source assets needed for collaboration: DOCX/PDF uploads, source audio, reference audio
- Do not auto-sync bulky derived outputs: enhanced WAV/MP3 files, generated narration, temporary chunks, intermediate model outputs
- Never sync: model caches, temporary processing files, local machine logs with private paths, credentials

Asset storage should be content-addressed using hashes so duplicate uploads are avoided and interrupted uploads can resume.

## Proposed Rust Workspace

The new GitHub repository should be a Rust workspace with clear crate boundaries:

```text
radsuite/
  crates/
    radsuite-core/
    radsuite-db/
    radsuite-sync/
    radsuite-cite/
    radsuite-cast/
    radsuite-tts/
    radsuite-engines/
    radsuite-server/
    radsuite-desktop/
  apps/
    desktop-ui/
  packaging/
    macos/
    windows/
  docs/
```

Crate responsibilities:

- `radsuite-core`: shared domain model, IDs, permissions, job model, errors, shared API contracts
- `radsuite-db`: local SQLite schema, migrations, repository layer
- `radsuite-sync`: sync protocol, local change queue, conflict records, asset manifest logic
- `radsuite-cite`: document parsing, citation detection, reference lookup orchestration, APA validation/reporting
- `radsuite-cast`: audio cleanup workflow orchestration, manifests, caption review integration
- `radsuite-tts`: transcription, script, voice profile, synthesis, caption, and quality workflow orchestration
- `radsuite-engines`: native sidecar discovery, runtime adapters, model manifest handling, hardware capability detection
- `radsuite-server`: Rust server for auth, projects, sync, assets, admin, audit, update/model manifests
- `radsuite-desktop`: Tauri commands, desktop app state, platform integration, IPC boundary to UI
- `apps/desktop-ui`: TypeScript UI for the Tauri app
- `packaging/macos` and `packaging/windows`: platform-specific signing, installer, bundled sidecars, and release configuration

## Initial Data Model

The shared model should be stable and UUID-based:

```text
User
Project
ProjectMember
Asset
Document
Paragraph
Citation
ReferenceEntry
AudioSource
AudioOutput
Script
VoiceProfile
TtsOutput
Job
AuditEvent
SyncRecord
```

Local integer IDs should not be used as durable cross-device identifiers. UUIDs allow offline creation and later sync.

The detailed schema should be designed in the implementation plan, but the initial model should preserve concepts from the current applications:

- RADcite course/module/document/reference structure
- RADcast project/source audio/output/job/manifest structure
- RADTTS project/script/reference audio/voice/output/job/manifest structure

The new model should avoid forcing the old app boundaries into the user experience.

## Feature Migration Order

Implementation should be phased internally, with external release blocked until full feature parity is achieved.

### Phase 1: Foundation

Build:

- Rust workspace
- Tauri desktop shell
- Rust server
- Local SQLite
- Project ownership/sharing
- Admin visibility
- Local asset store
- Sync protocol
- Job model
- Settings and logs
- Packaging skeleton for macOS and Windows
- Update/model manifest plumbing

### Phase 2: RADcite Core

Port:

- Project/course document upload
- DOCX/PDF extraction
- Paragraph display
- Citation detection
- Missing-citation flags
- Reference list management
- Crossref/OpenAlex lookup
- APA validation
- Manual citation-reference linking
- Archive/restore
- Export/reporting

RADcite is the recommended first complete product workflow because it exercises documents, database, permissions, sync, and admin behaviour without the hardest local ML packaging problems.

### Phase 3: RADcast Core

Port:

- Source audio import
- Trim/range selection
- Enhancement jobs
- Caption generation and review
- Output manifests
- Progress reporting
- Cancellation
- Selected output sharing

The enhancement engine should sit behind a Rust trait so implementation can evolve without changing the product workflow.

### Phase 4: RADTTS Core

Port:

- Scripts
- Reference audio
- Voice profiles
- Transcription
- Clip extraction
- Sentence chunk synthesis
- Pauses
- Built-in/custom voices
- Captions
- Quality reports
- Progress reporting
- Cancellation
- Output manifests

This is the highest technical-risk parity area because current Qwen/PyTorch behaviour may not map cleanly to a Python-free native runtime.

### Phase 5: Parity, Migration, And Release Readiness

Add:

- Import from existing RADcite database/project files
- Import from existing RADcast project folders
- Import from existing RADTTS project folders
- Cross-platform installer testing
- Server backup/restore operations
- Model/runtime setup validation
- Update flow validation
- Full parity checklist

No external release should happen until parity, migration, macOS installer, Windows installer, and server operations are proven.

## Engine Prototyping

Even though RADcite should be the first complete UI workflow, RADcast and RADTTS engine feasibility should be prototyped early.

Early prototypes should answer:

- Can the Python RADcast enhancement path be replaced with native tools or native model runtimes without unacceptable quality loss?
- What ASR path should be used: `whisper.cpp`, ONNX Runtime, another native runtime, or server-assisted processing?
- Can RADTTS voice cloning/synthesis be implemented locally without Python, with acceptable quality and licensing?
- What acceleration paths are realistic on Apple Silicon and Windows 11?
- How large are model downloads and what is the first-run setup experience?

If RADTTS local voice cloning cannot be implemented cleanly without Python in the first pass, the fallback should be a server-assisted or API-assisted engine behind the same Rust workflow interface, not a reintroduction of Python into the desktop package.

## Internal Builds And Release Gates

Internal builds are allowed to be incomplete. Public/external release waits for full parity.

Release gates:

- Internal alpha: shared Rust foundation plus enough RADcite/RADcast/RADTTS workflows to test architecture honestly
- Internal beta: all major workflows present, migration tools started, installers working on both platforms
- External release candidate: full parity, migration/import validation, server backup/restore, signed installers, update flow, model setup, documentation, and operational runbooks

## Open Questions For Implementation Planning

- Exact new GitHub repository name and visibility
- Whether the server stores assets on local disk only for alpha or abstracts storage immediately
- Desktop UI framework choices inside Tauri
- Rust web framework for server, likely Axum unless a better fit appears
- Local DB migration strategy, likely SQLite plus `sqlx` or `rusqlite`
- Password policy, invite flow, password reset flow, and admin bootstrap
- Detailed model/runtime feasibility for RADcast and RADTTS
- Exact migration strategy from current RADcite PostgreSQL/SQLite and project files
- Installer signing certificates and update distribution method

## Recommended Next Step

Create the new GitHub repository, initialise the Rust workspace locally, and write a detailed implementation plan for Phase 1 foundation work.
