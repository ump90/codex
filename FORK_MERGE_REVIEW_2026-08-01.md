# 二开分支主线合并审查记录 - 2026-08-01

## 审查范围

本次审查针对 `codex/fork-release` 合并上游 `main` 后的二开功能。审查不评价上游变更本身，只检查二开功能在合并主线后是否仍然正确，重点关注 Windows Git Bash 支持和模型可见上下文。

合并边界：

- 合并提交：`b1f99391386867cbd7179a79f70a0c914136c170`
- 合并前二开父提交：`10ae331da`
- 上游父提交：`464237054`
- 共同基线：`bdd3118c71a29f26b9df3a47f91efea38a0d58bd`

合并后，相对上游主线的二开 Rust 增量约为 51 个文件、2,715 行新增和 165 行删除。完整二开差异涉及 98 个文件，其中还包含二开发布自动化和有意保留的上游工作流删除。

## 问题清单

### P1 - Windows 发布工作流没有运行测试

`.github/workflows/fork-windows-build.yml` 和 `.github/workflows/fork-release.yml` 只执行 `cargo build`。二开分支已经不再包含上游 Rust CI 工作流，因此 Windows 专属回归仍可能正常产出并发布制品。

后续必须处理：

- 增加 Windows PR/push 测试工作流；
- 运行 `codex-core`、`codex-shell-command`、`codex-apply-patch`、`codex-sandboxing`、`codex-utils-path-uri` 和 `codex-state` 的聚焦测试；
- 运行 `just test-github-scripts`；
- 让发布任务依赖上述检查全部通过。

### P1 - macOS 测试目标无法编译

`codex-rs/core/src/shell.rs` 已将 `get_shell_by_model_provided_path` 的返回类型改为 `anyhow::Result<Shell>`，但 `codex-rs/core/tests/suite/shell_snapshot.rs` 中仅在 macOS 编译的调用点仍将该结果直接传给要求 `Shell` 的 `with_user_shell`。

处理状态（2026-08-01）：已修复。macOS 调用点现在使用 `?` 向上传播 shell 解析错误，与测试函数的 `anyhow::Result<()>` 返回类型一致。

验证限制：已安装 `x86_64-apple-darwin` Rust 标准库并执行交叉目标 `cargo check -p codex-core --tests --target x86_64-apple-darwin`。检查在进入本次调用点前被 `ring` 构建脚本阻断，因为当前 Windows 环境没有 macOS 目标 C 编译器；需要由 macOS CI 完成最终目标编译确认。

### P1 - 显式切换原生 Windows shell 时可能错误解析 Git Bash 工作目录

环境默认使用 Git Bash 时，模型看到的路径形如 `/d/Workspace/codex`。当前 `exec_command` 根据本次显式请求的执行 shell 决定是否进行路径转换。如果模型显式选择 PowerShell 或 cmd，同时复用模型上下文中的 `/d/...` 工作目录，路径转换会被跳过，最终可能将路径解析为 `D:\d\...`，而不是预期的 `D:\...`。

相关代码：

- `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs`
- `codex-rs/core/src/context/world_state/environment.rs`
- `codex-rs/utils/path-uri/src/tests.rs`

处理状态（2026-08-01）：已修复。`exec_command` 现在始终根据环境上下文的默认 shell 反解析工具路径，而不是根据本次显式请求的执行 shell。执行 shell 仍决定命令语法，但不会改变已经暴露给模型的路径约定。

新增覆盖：

- 默认 Git Bash、显式切换 PowerShell 时，将 `/c/Users/Alice/project` 转为 `C:\Users\Alice\project`；
- 默认 PowerShell、显式切换 Git Bash 时，保留模型看到的原生 Windows 路径；
- 端到端测试通过真实 `exec_command` 显式切换 `cmd.exe`，确认 Git Bash 形式且包含空格的工作目录能够正确执行。

### P1 - 二开差异过大，无法作为单一审查单元可靠确认

排除约 6,104 行机械性的工作流删除后，二开仍包含约 4,600 行实质变更。Git Bash 支持横跨 shell 发现、配置、沙箱、命令执行、上下文渲染、工具参数与输出、`apply_patch`、数据库迁移和发布打包。

后续必须处理：按照文档末尾的整改顺序分阶段维护和审查，不能继续将完整二开差异视为一个变更单元。

### P2 - Git Bash 集成测试并未真正执行 Git Bash

`codex-rs/core/tests/suite/windows_git_bash.rs` 使用的是空的假 `bash.exe`。现有测试只验证首轮上下文渲染和自定义 `apply_patch` 运行时，没有覆盖进程启动、命令参数构造、工作目录处理、UTF-8 环境变量设置以及 Windows 沙箱执行链路。

后续必须处理：在 `windows-latest` 上发现真实的 Git for Windows 安装，通过公开测试框架调用 `shell_command` 和 `exec_command`，并断言输出和工作目录。

### P2 - 数据库迁移测试绕过了生产启动路径

`codex-rs/state/src/migrations_tests.rs` 中的 CRLF 兼容测试直接调用 `line_ending_compatible_migrator`，没有验证本次合并后迁移到 `SqliteConfig::open_runtime_db` 的生产接线。

后续必须处理：先写入一个记录了 CRLF migration checksum 的数据库，关闭数据库，再通过正常 state runtime API 重新打开。

### P2 - Git Bash 发现逻辑使 shell_detect.rs 过大

`codex-rs/shell-command/src/shell_detect.rs` 当前已达到 864 行。Git for Windows 候选路径发现、安装根目录验证及相关错误类型都直接放入了这个中心模块。

后续必须处理：将 Git Bash 实现提取到同级独立模块，`shell_detect.rs` 只保留通用 shell 分派逻辑。

### P2 - 多轮上下文行为缺少回归测试

Git Bash 上下文集成测试只检查第一次推理请求，没有验证合并后的增量 world-state snapshot/diff 逻辑在多轮对话中的表现。

后续必须处理：增加多轮测试，确认未变化的 Git Bash cwd、workspace roots 和 shell 状态不会被重复注入；真实环境变化只产生一次有界更新。

## 上下文审查结论

在合并后的实现中，没有发现已经成立的上下文功能回归。冲突处理保留了上游多环境 `is_primary` 状态，同时保留二开的 Git Bash cwd 和文件系统路径渲染。上下文仍以增量且有界的方式构建，新环境片段也继续使用既有的上下文片段类型。

当前剩余风险主要是测试覆盖不足：没有二开多轮测试能够检测路径格式更新被重复注入，或由此造成的提示词缓存命中率下降和上下文劣化。

## 已执行验证

本次审查运行于 Windows 11 的 Git Bash 环境，Rust 主机目标为 `x86_64-pc-windows-msvc`。

- `just test -p codex-shell-command`：207/207 通过。
- `just test -p codex-apply-patch -p codex-sandboxing -p codex-utils-path-uri`：183/183 通过。
- `just test -p codex-state`：164/164 通过。
- 二开工作流 Python 测试：2/2 通过。
- `bazel mod deps --lockfile_mode=error`：通过，仅有依赖版本警告。
- `git diff --check 464237054..HEAD`：通过。
- 使用当前代码重新构建的 `codex.exe`、`codex-windows-sandbox-setup.exe` 和 `codex-command-runner.exe`，已成功通过 `codex sandbox` 执行真实的 Git for Windows Bash；输出包含 `git-bash-ok` 和预期工作目录。

本次修复后的验证结果：

- 两个 Windows 路径约定单元测试：2/2 通过；
- 新增 `configured_git_bash_resolves_workdir_when_switching_to_cmd` 端到端测试：1/1 通过；
- `just test -p codex-core` 共运行 2,825 项测试，其中 2,817 项通过；7 项失败和 1 项超时仍集中在本地 Windows sandbox helper 以及与本次修改无关的 MCP、code-mode 测试，两个路径约定单元测试和原有 Git Bash 集成测试均通过；
- `just fix -p codex-core`：通过；
- `just fmt`：通过。

## 建议整改顺序

1. 恢复强制 Windows CI 和发布测试门禁。
2. 已完成：修复显式 shell 下的工作目录路径约定，并增加回归测试。
3. 已完成代码修复：修复 macOS 平台条件编译错误；等待 macOS CI 完成目标编译确认。
4. 增加真实 Git Bash 进程测试和多轮上下文集成测试。
5. 通过生产数据库启动路径测试 CRLF 兼容性。
6. 从 `shell_detect.rs` 中提取 Git Bash 发现逻辑。
7. 将迁移、shell 发现与配置、沙箱、路径与 `apply_patch`、上下文与工具、发布工作流分别作为可独立审查的阶段维护。

## 仓库状态

初始审查对应合并提交 `b1f993913`。2026-08-01 已在工作区修复上述两个 P1，目前相关代码和本文档均尚未提交。
