# Workflow Strategy

This fork release branch keeps only the workflows needed for the downstream
release flow:

- `fork-windows-build.yml` builds Windows release artifacts manually from the
  selected ref.
- `fork-release.yml` builds Windows release artifacts, creates or updates a
  GitHub Release, and uploads raw binary archives plus portable archives that
  can be extracted and added to `PATH`. The portable archives include Portable
  Git for Windows/Git Bash so the fork can use Git Bash without requiring a
  separate Git installation. It also downloads the latest official Codex App
  MSIX for each available Windows architecture, verifies the package checksum,
  replaces its Codex sidecars with the binaries built from the selected ref,
  and publishes an unpackaged `codex-app-portable-windows-<target>.zip`. The App
  archive includes Portable Git and must be launched through `codex-app.cmd`.
  It is not an installable MSIX because replacing binaries invalidates the
  original package signature. Windows ARM64 App packaging is skipped when the
  upstream release manifest does not offer an ARM64 package. App packaging runs
  in separate jobs that consume the CLI portable archives, keeping the large
  Rust build directory and extracted MSIX off the same runner disk.
- `fork-codex-app-release.yml` refreshes only the portable Codex App assets in
  an existing fork release. It reuses the release's portable CLI archives,
  downloads the latest official Codex App packages, rebuilds the App archives,
  and replaces their entries in `SHA256SUMS.txt`. It does not compile the CLI.
- `sync-upstream.yml` merges upstream `openai/codex` changes into `main`.

The upstream OpenAI CI, triage, signing, package publishing, and release
automation workflows are intentionally not carried on this branch.
