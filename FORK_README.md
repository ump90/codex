# Codex CLI Windows Git Bash fork

This document records the implementation and merge-maintenance details that
exist only on `codex/fork-release`. Keep it current whenever downstream
functionality is added, removed, or moved. The root `README.md` is the Chinese
user-facing guide for this fork; this file is the detailed maintainer guide.

## Install the portable Windows build

Download and extract the archive for your architecture:

```text
codex-portable-windows-x86_64-pc-windows-msvc.zip
codex-portable-windows-aarch64-pc-windows-msvc.zip
```

Add the extracted root directory, which contains `codex.cmd`, to `PATH`. Do not
add only the `bin` directory. `codex.cmd` adds the bundled Portable Git to the
process environment before starting `bin\codex.exe`.

Back up `%USERPROFILE%\.codex` before switching from another Codex build if you
need to preserve existing configuration and session data.

## Downstream-only functionality

| Functionality | Behavior | Main code locations |
| --- | --- | --- |
| Git Bash shell selection | On Windows, automatically discovers Git for Windows and prefers Git Bash. Falls back to PowerShell when Git Bash is unavailable. | `codex-rs/core/src/shell.rs`, `codex-rs/shell-command/src/shell_detect.rs`, `codex-rs/core/src/config/mod.rs` |
| Windows shell configuration | Adds `[windows].default_shell` and `[windows].git_bash_path`. Supported shells are `git-bash`, `powershell`, and `cmd`. | `codex-rs/config/src/types.rs`, `codex-rs/core/config.schema.json` |
| Shell-aware path conversion | Converts Windows paths such as `C:\work` to `/c/work` for Git Bash, and converts model/tool inputs back to native paths where Windows APIs require them. Covers cwd, workspace roots, permissions, tool workdirs, image paths, and tool responses. | `codex-rs/core/src/git_bash_paths.rs`, `codex-rs/utils/path-uri/src/git_bash.rs`, `codex-rs/core/src/context/environment_context.rs` |
| Explicit shell switching | Treats the default model-visible shell as the source of structured path syntax. Selecting PowerShell or cmd for one `exec_command` changes command parsing without misreading a Git Bash-form `workdir`. | `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs`, `codex-rs/core/src/tools/handlers/unified_exec_tests.rs` |
| Codex App file links | Keeps Git Bash `/c/...` syntax for commands and tool arguments, but tells the model to emit native `C:/...` targets for clickable local Markdown links. | `codex-rs/core/src/context/git_bash_file_link_instructions.rs`, `codex-rs/core/src/session/world_state.rs`, `codex-rs/core/tests/suite/windows_git_bash.rs` |
| Windows V8 release artifacts | Downloads and verifies the exact sandbox-enabled V8 archive and binding published by Codex before x64 or ARM64 MSVC Cargo builds. | `.github/actions/setup-rusty-v8/action.yml`, `.github/workflows/fork-windows-build.yml`, `.github/workflows/fork-release.yml` |
| Git Bash `apply_patch` | Interprets add, update, delete, move, and optional `cd` paths using Git Bash semantics without misreading MSYS paths such as `/usr/bin`. | `codex-rs/apply-patch/src/invocation.rs`, `codex-rs/apply-patch/src/lib.rs`, `codex-rs/core/src/tools/handlers/apply_patch.rs` |
| Windows sandbox integration | Makes the complete Git for Windows runtime available read-only inside the Windows sandbox instead of copying only `bash.exe`. | `codex-rs/sandboxing/src/manager.rs`, `codex-rs/windows-sandbox-rs/src/helper_materialization.rs` |
| UTF-8 Git Bash commands | Sets `LANG`, `LC_CTYPE`, and `LC_ALL` to `C.UTF-8` for Git Bash commands to reduce localized Windows encoding problems. | `codex-rs/core/src/tools/handlers/shell/shell_command.rs` |
| SQLx line-ending compatibility | Accepts historical migration checksums that differ only by LF/CRLF line endings and keeps migration SQL files normalized to LF. | `.gitattributes`, `codex-rs/state/src/migrations.rs`, `codex-rs/state/src/sqlite.rs` |
| Portable release automation | Builds raw Windows binaries, portable CLI archives with Portable Git, and unpackaged portable Codex App archives with downstream sidecars. | `.github/workflows/fork-windows-build.yml`, `.github/workflows/fork-release.yml`, `.github/scripts/build-codex-app-portable.ps1` |
| Upstream synchronization | Periodically merges `openai/codex:main` into this fork's `main`; maintainers then merge `main` into `codex/fork-release`. | `.github/workflows/sync-upstream.yml` |

The regression suites most directly protecting these functions are:

```text
codex-rs/core/tests/suite/windows_git_bash.rs
codex-rs/core/src/shell_windows_tests.rs
codex-rs/utils/path-uri/src/git_bash_tests.rs
codex-rs/sandboxing/src/manager_tests.rs
codex-rs/state/src/migrations_tests.rs
.github/scripts/test_fork_windows_workflows.py
```

The current downstream feature commits are grouped below for merge review:

| Feature group | Representative commits |
| --- | --- |
| Fork workflow and upstream synchronization | `003c555c0`, `fc8d2c1f0`, `a7096c03d` |
| Windows Git Bash discovery and default shell | `ddcd55bf2`, `f2249abd4`, `e105e31c4`, `c2f2b3e0c`, `b2a70da67` |
| Windows path semantics and portable packaging | `5f8e02f97`, `7027fffc9`, `c5ede5a4f`, `a6c50c5f2` |
| SQLx migration line-ending compatibility | `5db47ac7c` |
| Git Bash `apply_patch` path resolution | `10ae331da` |
| Post-merge shell compatibility fixes | `a2c0fe3a3` |

These are representative anchors, not an exhaustive replacement for
`git log main..codex/fork-release`. Add a row or update the relevant row when a
new downstream feature is introduced.

## Windows shell configuration

The configuration file is normally `%USERPROFILE%\.codex\config.toml`.

Use Git Bash explicitly:

```toml
[windows]
default_shell = "git-bash"
```

Use an existing Git for Windows installation:

```toml
[windows]
default_shell = "git-bash"
git_bash_path = "C:\\Program Files\\Git\\bin\\bash.exe"
```

Switch back to a native Windows shell:

```toml
[windows]
default_shell = "powershell"
# default_shell = "cmd"
```

`git_bash_path` must be an absolute path to a valid Git for Windows
`bash.exe`. Explicitly selecting Git Bash with an invalid path is a
configuration error.

## Release artifacts and workflows

The fork release normally publishes:

- `codex-windows-<target>.zip`: raw Windows executables and available symbols;
- `codex-portable-windows-<target>.zip`: portable CLI, `codex.cmd`, Portable
  Git, ripgrep, and required helper binaries;
- `codex-app-portable-windows-<target>.zip`: unpackaged Codex App with the
  downstream sidecars and Portable Git;
- `SHA256SUMS.txt`: checksums for the published archives.

Launch the App archive through `codex-app.cmd`. Replacing its sidecars
invalidates the original MSIX signature, so this archive is not an installable
MSIX.

Fork workflows:

- `fork-windows-build.yml`: manually builds one Windows target as an Actions
  artifact;
- `fork-release.yml`: builds x64/ARM64 CLI releases and portable App archives;
- `fork-codex-app-release.yml`: refreshes App archives without rebuilding the
  Rust CLI;
- `sync-upstream.yml`: synchronizes upstream into the fork's `main` branch.

The App workflows resolve the target architecture's current manifest entry and
the matching MSIX checksum immediately before downloading. This keeps the
rolling `latest` links usable while still verifying the downloaded package.

Fork release tags must not use upstream's `rust-v*` namespace. Use a tag such
as `<upstream-version>-fork.1` or `<upstream-version>-gitbash.1`.

## Merging upstream `main`

### Current synchronization baseline

As of 2026-08-08:

- `main` is merged through `208f05b23`;
- merge commit `177bc14b8` preserves the fork workflow isolation and rebases
  environment permissions onto upstream's per-`TurnEnvironment` ownership;
- the Git Bash environment-context, absolute `apply_patch`, and explicit
  `cmd.exe` workdir integration tests pass on Windows;
- `just fix -p codex-core`, `just fmt`, and both diff checks pass. The full
  `codex-core` run completed with 2,943 passing tests; the remaining local
  MCP/code-mode failures and Windows sandbox timeouts are tracked as baseline
  environment failures rather than Git Bash regressions.

### Branch responsibilities

- `main` tracks `openai/codex:main`.
- `codex/fork-release` contains downstream runtime and release functionality.
- Automatic synchronization updates only `main`; merge it into
  `codex/fork-release` after reviewing downstream compatibility.

### Conflict-resolution rules

1. Use upstream's latest data structures and execution flow as the base. Add
   downstream behavior at narrow helper or adapter boundaries rather than
   retaining an old upstream implementation.
2. Keep shell-aware path conversion centralized in `git_bash_paths` and
   `utils/path-uri`. Do not duplicate Windows/Git Bash path rules in tool
   handlers.
3. Preserve upstream environment primary-selection and snapshot semantics.
   Derive model-visible cwd formatting from each environment's shell and cwd.
4. Keep SQLx line-ending compatibility in the centralized SQLite migration
   entry point (`state/src/sqlite.rs`) so upstream database refactors do not
   leave a second migration path behind.
5. Resolve `Cargo.toml`, `Cargo.lock`, and `MODULE.bazel` before regenerating
   `MODULE.bazel.lock`; do not hand-merge generated lock-file digests.
6. Preserve the fork-specific root `README.md` as the Chinese user guide. Put
   implementation details and recurring merge instructions in this file.

### Upstream workflow isolation

The fork should run only these workflows:

```text
fork-windows-build
fork-release
fork-codex-app-release
sync-upstream
```

`.github/workflows/rust-release.yml` is retained byte-for-byte from `main` to
avoid recurring modify/delete conflicts, but it must remain disabled in the
fork repository settings. After merging and pushing, verify it with:

```powershell
gh api repos/ump90/codex/actions/workflows/rust-release.yml --jq ".state"
```

The expected state is `disabled_manually`. Disable it if necessary:

```powershell
gh workflow disable rust-release.yml --repo ump90/codex
```

Also audit newly introduced upstream workflows after every synchronization:

```powershell
gh workflow list --all --repo ump90/codex
```

### Verification checklist

At minimum, confirm that:

- automatic shell selection chooses Git Bash when Git for Windows is present
  and PowerShell otherwise;
- invalid explicit `git_bash_path` values fail clearly;
- Windows cwd and workspace roots render as `/c/...` for Git Bash;
- local Markdown file links in a Git Bash session use native `C:/...` targets,
  so Codex App can open them on Windows;
- Windows x64 and ARM64 release jobs download and verify Codex-published
  sandbox-enabled `rusty_v8` artifacts before invoking Cargo;
- shell workdirs, permission paths, `view_image`, and `apply_patch` convert in
  both directions without converting `/usr/...` as a drive path;
- explicitly selecting PowerShell or cmd still resolves a Git Bash-form
  `workdir` according to the default model-visible shell;
- the Windows sandbox can execute the bundled Portable Git runtime;
- LF- and CRLF-recorded SQLx migration checksums both open successfully;
- portable archives contain `codex.cmd`, Portable Git, Windows ripgrep, and all
  required helper binaries.

Useful focused commands from `codex-rs` include:

```powershell
just test -p codex-core
just test -p codex-apply-patch
just test -p codex-shell-command
just test -p codex-sandboxing
just test -p codex-state
just test -p codex-utils-path-uri
```

Workflow and Bazel checks from the repository root:

```powershell
python .github/scripts/test_fork_windows_workflows.py
just bazel-lock-check
```
