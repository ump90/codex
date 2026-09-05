---
name: merge-main-fork
description: 固定将上游 main 合并到当前 Codex 二开分支，安全处理冲突，并复核 Windows Git Bash、路径转换、上下文、沙箱、SQLx 和发布工作流等二开功能。用户要求同步 main、合并主线、解决上游合并冲突、检查合并影响或发布前复核时使用。
---

# 合并主线并复核二开

按以下顺序执行完整流程。除非用户明确缩小范围，不要跳过合并后的二开复核。

## 合并前检查

1. 确认仓库根目录、当前分支和远端：

   ```bash
   git rev-parse --show-toplevel
   git branch --show-current
   git status --short --branch
   git remote -v
   ```

2. 不要在 `main` 上执行。当前分支必须是二开分支。发现未提交修改时，先列出文件并暂停合并；不得擅自 stash、丢弃或覆盖用户修改。
3. 选择主线来源。优先使用已经同步到本仓库的 `origin/main`；如果不存在，使用 `upstream/main`。来源不明确或远端不存在时，停止并说明需要的远端配置。
4. 获取最新主线并记录合并前快照：

   ```bash
   git fetch origin main
   git rev-parse HEAD
   git rev-parse origin/main
   git log --oneline --decorate -5
   ```

   使用 `upstream/main` 时，将上述命令中的远端名称替换为 `upstream`。

## 合并与冲突

1. 执行 `git merge --no-edit <main-source>`。保留完整的合并提交，不要 rebase、reset 或改写已经推送的二开历史。
2. 有冲突时先运行：

   ```bash
   git status
   git diff --name-only --diff-filter=U
   ```

3. 逐个阅读冲突文件。以主线当前数据结构和执行流程为基础，在窄边界恢复二开行为；不要对全部文件使用 `ours` 或 `theirs`。重点保持：
   - Git Bash 发现、`[windows]` shell 配置和 UTF-8 环境变量；
   - Git Bash 与 Windows 路径的双向转换，以及模型可见 cwd、工作区和工具路径；
   - 显式切换 PowerShell 或 cmd 时仍按默认模型可见 shell 解析 `workdir`；
   - Codex App 可点击的本地 Markdown 链接使用 `C:/...`；
   - `apply_patch`、Windows 沙箱、SQLx LF/CRLF 迁移兼容；
   - x64/ARM64 V8 制品校验、便携包内容和发布工作流隔离。
4. 修改后确认没有未解决冲突，再执行：

   ```bash
   git add <resolved-files>
   git merge --continue
   git status --short --branch
   git diff --check HEAD^ HEAD
   ```

   如果 `git merge --continue` 需要编辑器，使用 `GIT_EDITOR=true git merge --continue`，不要修改合并提交的父关系。

## 合并后复核

1. 检查二开差异和本次合并触及的文件：

   ```bash
   git diff --stat <main-source>...HEAD
   git log --oneline <main-source>..HEAD
   git diff --name-only <merge-parent> HEAD
   ```

   将新增、删除或移动的二开代码与 `FORK_README.md` 的功能清单逐项对应。只有新增或移动功能时才更新清单；不要创建按日期命名的合并审查文档。
2. 按变更范围运行测试。Windows Git Bash 或路径相关代码至少运行受影响的项目测试，例如：

   ```bash
   cd codex-rs
   just test -p codex-core
   just test -p codex-apply-patch
   just test -p codex-shell-command
   just test -p codex-sandboxing
   just test -p codex-state
   just test -p codex-utils-path-uri
   just fmt
   cd ..
   ```

   只运行实际受影响的项目；如果改动 common、core 或 protocol，需要在运行完整 `just test` 前征得用户同意。
3. 发布工作流、V8 或便携包变更时，额外检查：

   ```bash
   python .github/scripts/test_fork_windows_workflows.py
   yq eval '.' .github/workflows/fork-release.yml > /dev/null
   yq eval '.' .github/workflows/fork-windows-build.yml > /dev/null
   just bazel-lock-check
   ```

4. 检查上游工作流范围：

   ```bash
   gh workflow list --all --repo ump90/codex
   gh api repos/ump90/codex/actions/workflows/rust-release.yml --jq '.state'
   ```

   保留的上游 `rust-release.yml` 应保持 `disabled_manually`；未经确认不要修改仓库设置。
5. 记录已运行的测试和剩余风险。警告可以保留；编译错误、测试失败、未解决冲突和工作流配置错误必须修复或明确报告。

## 提交与推送

- 合并成功且验证通过后，先展示待提交文件和摘要。
- 用户明确要求提交时，提交合并和必要的后续修复，提交信息应说明合并目标或修复内容。
- 用户明确要求推送时，执行 `git push <remote> <current-branch>`，并确认本地与远端提交一致。
- 推送前再次检查 `git status`，不得推送未说明的用户修改。
