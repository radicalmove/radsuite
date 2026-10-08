# RADsuite 0.2.11 Git backup and platform verification

The preferred preset is **Studio — Treble (recommended)** (`studio_treble`). The previous method is **Studio — Classic** (`studio_v1`). IDs and saved selections remain compatible.

The backup retains both implementations, the earlier enhancement profiles, experiment code, QA records, provenance/licences, and the approved epoch119 Treble checkpoint. Generated audio, images, arrays and alternative experimental checkpoints remain local; retained manifests record their identities. The production preset still requires its pinned local Python runtime and verified cache.

All application version files are 0.2.11. The package-verification workflow builds Apple Silicon Mac, Intel Mac and Windows from one commit, and retains test installers for 30 days. It does not publish a stable release or update the public updater. Linux is the existing CI check platform, not a packaged OS.

The backup branch is `codex/radcast-treble-0.2.11`. It starts from the checkout used for the tested local build. The remote main branch has additional changes; this backup does not replace or roll back those changes. Integrating with current main and publishing a stable release are separate operations.
