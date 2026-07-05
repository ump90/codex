# Workflow Strategy

This fork release branch keeps only the workflows needed for the downstream
release flow:

- `fork-windows-build.yml` builds Windows release artifacts manually from the
  selected ref.
- `fork-release.yml` builds Windows release artifacts, creates or updates a
  GitHub Release, and uploads the release archives.
- `sync-upstream.yml` merges upstream `openai/codex` changes into the fork
  release branch.

The upstream OpenAI CI, triage, signing, package publishing, and release
automation workflows are intentionally not carried on this branch.
