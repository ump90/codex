# 二开分支维护指南

这份文档只记录 `codex/fork-release` 相对上游的功能，以及每次合并
`main` 后需要确认的事项。用户安装和配置说明在根目录的 `README.md`；
一次性的合并结论和提交记录不在这里保留。

## 分支职责

- `main` 跟踪 `openai/codex:main`。
- `codex/fork-release` 承载 Windows Git Bash 和 Windows 便携发布相关功能。
- 上游同步只更新 `main`。合并到 `codex/fork-release` 时，以主线当前实现
  为基础，恢复并验证下表中的二开行为。

## 合并时需要复核的功能

| 功能 | 合并后必须确认的行为 | 主要位置 |
| --- | --- | --- |
| Git Bash 发现与配置 | Windows 优先使用 Git Bash；未安装时回退到 PowerShell；`[windows]` 中的 `default_shell` 和 `git_bash_path` 仍然有效。 | `codex-rs/core/src/shell.rs`、`codex-rs/shell-command/src/shell_detect.rs`、`codex-rs/config/src/types.rs` |
| 模型可见路径和执行路径 | Git Bash 环境中的 cwd、工作区和工具路径使用 `/c/...`；传给 Windows API 前正确还原；`/usr/...` 不能被当作盘符路径。 | `codex-rs/core/src/git_bash_paths.rs`、`codex-rs/utils/path-uri/src/git_bash.rs`、`codex-rs/core/src/context/` |
| 显式切换 shell | `exec_command` 的命令语法由本次选择的 shell 决定，但 `workdir` 必须按模型默认可见 shell 的路径约定解析。 | `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs` |
| Codex App 本地文件链接 | Git Bash 命令和工具参数仍使用 `/c/...`；Markdown 本地链接使用 `C:/...`，使 Windows App 可以点击打开。 | `codex-rs/core/src/context/git_bash_file_link_instructions.rs`、`codex-rs/core/src/session/world_state.rs` |
| `apply_patch`、沙箱和编码 | Git Bash 路径在 `apply_patch` 中可用；Windows 沙箱可使用完整 Portable Git；Git Bash 命令保持 UTF-8。 | `codex-rs/apply-patch/src/`、`codex-rs/sandboxing/src/`、`codex-rs/core/src/tools/handlers/shell/` |
| SQLx 迁移 | 仅 LF/CRLF 不同的历史 migration checksum 仍可打开数据库。 | `codex-rs/state/src/migrations.rs`、`codex-rs/state/src/sqlite.rs` |
| Windows 发布 | x64/ARM64 使用匹配的 sandbox-enabled `rusty_v8` 制品；便携包包含 `codex.cmd`、Portable Git、`rg.exe` 和所需 helper；sccache 连接缓存服务失败时会回退到原生 Cargo 编译。 | `.github/actions/setup-rusty-v8/`、`.github/workflows/fork-*.yml`、`.github/scripts/` |

## 合并原则

1. 采用上游最新的数据结构和执行流程，在边界处接入二开行为，不保留旧的上游实现。
2. 路径转换集中在 `git_bash_paths` 和 `utils/path-uri`，不要在工具处理器中复制规则。
3. 保留上游的环境选择和增量上下文逻辑，只按环境的 shell 和 cwd 格式化模型可见路径。
4. SQLx 兼容性保留在集中迁移入口；依赖变更按仓库要求更新生成的锁文件。
5. 根 `README.md` 是用户指南；本文件只维护二开功能和合并检查项。

## 合并后验证

先检查二开触及的位置和冲突处理结果，再运行与变更相符的测试。Windows
Git Bash 改动至少覆盖以下项目：

```powershell
just test -p codex-core
just test -p codex-apply-patch
just test -p codex-shell-command
just test -p codex-sandboxing
just test -p codex-state
just test -p codex-utils-path-uri
```

发布工作流或打包逻辑变更时，还要运行：

```powershell
python .github/scripts/test_fork_windows_workflows.py
just bazel-lock-check
```

合并并推送后，检查工作流列表。分支只应运行 fork 工作流；保留用于减少
合并冲突的上游 `rust-release.yml` 必须维持手动禁用状态：

```powershell
gh workflow list --all --repo ump90/codex
gh api repos/ump90/codex/actions/workflows/rust-release.yml --jq ".state"
```
