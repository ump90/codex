# Workflow Strategy

This fork release branch keeps only the workflows needed for the downstream
release flow:

- `fork-windows-build.yml` builds Windows release artifacts manually from the
  selected ref.
- `fork-release.yml` builds Windows release artifacts, creates or updates a
  GitHub Release, and uploads raw binary archives plus portable archives that
  can be extracted and added to `PATH`. The portable archives include Portable
  Git for Windows/Git Bash so the fork can use Git Bash without requiring a
  separate Git installation.
- `sync-upstream.yml` merges upstream `openai/codex` changes into `main`.

The upstream OpenAI CI, triage, signing, package publishing, and release
automation workflows are intentionally not carried on this branch.
