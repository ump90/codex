## 背景

Codex 在 Windows 下默认使用 PowerShell 作为终端执行命令，但模型生成的 PowerShell 命令质量不稳定，经常执行失败。二开目标是在 Windows 环境下增加 Git Bash 支持，优先让模型可以用 Bash 风格命令执行日常开发操作。

## 当前工作分支

- 基线分支：`codex/fork-release`
- 开发分支：`codex/git-bash-windows-shell-support`
- 当前状态：核心 Rust 实现已完成，已完成配置 schema 更新、格式化、编译检查和一轮 scoped 测试；当前进入测试补齐与最终验证阶段。

## 已确认决策

1. 只提供配置开关，不在代码中静默强制所有 Windows 用户切到 Git Bash。
2. 只支持 Git for Windows 自带的 Bash，不把 MSYS2/Cygwin Bash 纳入支持范围。
3. fallback 按以下规则实现：
   - 未配置 Git Bash 时，保持现有 Windows 默认选择：优先 PowerShell，找不到再退到 `cmd.exe`。
   - 显式配置 `default_shell = "git-bash"` 或 `git_bash_path` 后，找不到有效 Git for Windows Bash 就报清晰错误，不自动退回 PowerShell。
4. 需要把 Codex Windows sandbox 对 Git Bash 的支持情况纳入设计和测试范围。

## 当前实现进展

1. 已完成配置开关。
   - 新增 `[windows] default_shell = "powershell" | "git-bash" | "cmd"`。
   - 新增 `[windows] git_bash_path = "..."`。
   - 配置缺省时保持原 Windows 默认行为：PowerShell 优先，找不到时退到 `cmd.exe`。
   - 显式选择 Git Bash 或配置 `git_bash_path` 时，找不到有效 Git for Windows Bash 会返回清晰错误，不静默 fallback 到 PowerShell。

2. 已完成 Git for Windows Bash discovery。
   - 支持从 `git.exe` 反推 Git 安装根。
   - 覆盖 Program Files 和 Scoop/用户目录安装场景。
   - 只接受可验证的 Git for Windows 安装，不把 MSYS2/Cygwin Bash 当作受支持 shell。
   - Windows 下 Bash fallback 不再使用 Unix 的 `/bin/bash` / `/usr/bin/bash` 路径。

3. 已接入 session 默认 shell。
   - `session.rs` 已从裸 `default_user_shell()` 调整为读取 Windows shell 配置。
   - Git Bash 仍复用已有 Bash argv 组装逻辑：`bash.exe -c` / `bash.exe -lc`。

4. 已更新模型可见工具提示词。
   - Windows `shell_command` 不再固定描述为 PowerShell。
   - 提示模型按 `<environment_context><shell>` 选择 Git Bash、PowerShell 或 `cmd` 语法。
   - Windows safety guidance 已改成跨 shell 规则，不再只推荐 PowerShell cmdlet。
   - Git Bash 场景新增路径提示：Bash 命令中使用 `/c/...` 或 `C:/...`，避免直接使用 `C:\...` 反斜杠路径。
   - `exec_command.shell` 描述已补充 Windows 显式选择 Git Bash 时应传绝对 `bash.exe` 路径。

5. 已补 Git Bash 路径风格兼容。
   - Windows + Git Bash 的 `<environment_context><cwd>` 会渲染为 Git Bash 风格路径，例如 `/c/Users/...`。
   - `<filesystem>` 中的 workspace roots、显式 path/glob entries 会按当前主环境 shell 渲染，Git Bash 下避免暴露原始反斜杠路径给模型。
   - `shell_command`、`exec_command`、`request_permissions` 的 `workdir` / filesystem permission 路径参数在 Git Bash 会话中接受 `/c/...`，内部会转回 Windows native path。
   - `view_image.path` 在 Git Bash 会话中也接受 `/c/...`，避免被 Windows resolver 误当成当前盘根路径。

6. 已完成 Windows sandbox 兼容处理。
   - Git Bash 默认 shell 会解析为绝对 `bash.exe`，不依赖 sandbox 内部 PATH。
   - Git Bash 安装根会加入 sandbox/helper read roots，避免只放行 `bash.exe` 而缺 DLL、`usr/bin` 等运行时文件。
   - direct-spawn wrapper 遇到 Git Bash 时避免把 `bash.exe` 当作单文件 helper 复制，改为保留原始 Git Bash 路径并使用 Codex wrapper 启动。
   - Git 安装根只作为 read root，不扩大 workspace 写权限。

7. 已完成的验证。
   - 已运行 `just write-config-schema`。
   - 已运行 `just test -p codex-core config::schema::tests::config_schema_matches_fixture`，通过。
   - 已运行 `cargo check -p codex-config -p codex-shell-command -p codex-sandboxing -p codex-core`，通过。
   - 已单独构建 `test_stdio_server`、`codex.exe`、`codex-windows-sandbox-setup.exe`、`codex-command-runner.exe`，解决本地测试依赖二进制缺失问题。
   - 已运行 `just test -p codex-config -p codex-shell-command -p codex-sandboxing -p codex-core`。结果为 `3001` 个测试中 `3000` 个通过，`subagent_stop_replaces_stop_and_skips_internal_subagents` 一次失败；后续已定位为测试等待条件过宽导致的 flaky，并修复该测试同步点。
   - 已运行 `just fmt`。
   - 已运行 `just fix -p codex-core -p codex-shell-command -p codex-sandboxing -p codex-config`，并补跑 scoped `cargo check` 通过。
   - 已运行 `git diff --check`，通过。
   - 已补 Git Bash discovery edge cases、core Windows shell 配置测试、Windows Git Bash command safety 测试。
   - 已补 `subagent_stop_replaces_stop_and_skips_internal_subagents` flaky 同步点，并用 20 次 stress 验证通过。
   - 已补 Windows Git Bash 环境上下文集成测试：配置 `[windows] default_shell = "git-bash"` 后，模型可见 `<environment_context><shell>` 渲染为 `bash`。
   - 已补 Windows sandbox direct-spawn 单元测试：Git Bash 不被复制成单文件 helper，保留原始 `bash.exe`，并把 Git 安装根加入 read roots。
   - 已补 `sandbox_smoketests.py` 中的 Git Bash smoke case：`pwd && ls`、workspace-write 写入、read-only 拒绝写入、Git 安装根可读。
   - 已修复 `sandbox_smoketests.py` 的 CLI 调用方式：当前 CLI 是 `codex sandbox -- ...`，不再是旧的 `codex sandbox windows -- ...`。
   - 已运行 `just test -p codex-sandboxing transform_for_direct_spawn_windows_preserves_git_bash_runtime_root`，通过。
   - 已运行 `just test -p codex-sandboxing`，`38/38` 通过。
   - 已运行 `just test -p codex-core suite::windows_git_bash::configured_git_bash_renders_environment_context_shell`，通过（nextest 标记为 leaky，但退出码为 0）。
   - 已运行 `python -m py_compile codex-rs/windows-sandbox-rs/sandbox_smoketests.py`，通过。
   - 已运行 `cargo build -p codex-cli`，通过。
   - 已运行完整 `python codex-rs/windows-sandbox-rs/sandbox_smoketests.py`：
     - 新增 Git Bash smoke case 全部通过：`pwd && ls`、workspace-write 写入、read-only 拒绝写入、Git 安装根可读。
     - 整体结果为 `44/50` 通过；剩余失败为既有 smoke 覆盖或本机环境/策略问题，非本次 Git Bash 新增路径：sandbox 内按名称找不到 `python` / `git`、loopback proxy/direct 期望不匹配、ADS 写入未拒绝、`Start-Process https` 仍是已标注 known fail。
   - 已运行 `just test -p codex-core git_bash_paths`，通过。
   - 已运行 `just test -p codex-core context::world_state::environment`，通过。
   - 已重新运行 `just test -p codex-core suite::windows_git_bash::configured_git_bash_renders_environment_context_shell`，通过，并新增断言确认 `<cwd>` 为 `/c/...` Git Bash 风格路径。
   - 已运行 `just test -p codex-core tools::handlers::shell`，通过。
   - 已运行 `just test -p codex-core tools::handlers::unified_exec`，通过。
   - 已运行 `just test -p codex-core tools::handlers::view_image`，通过。
   - 已运行 `just test -p codex-core request_permissions`，通过。
   - 已运行 `just fix -p codex-core`，通过；随后已运行 `just fmt`。

8. 尚未完成/待补齐。
   - 还需要补 `shell_command` / `exec_command` 默认 shell 行为测试，确认未传 shell 时使用配置选中的 Git Bash，显式传 PowerShell 时仍可覆盖。
   - 还需要决定如何处理完整 Windows sandbox smoke script 的既有失败项；Git Bash 新增项已在本机通过。
   - 还没有跑完整 workspace `just test`；按仓库规则，完整测试需要后续单独确认后再跑。

## 源码调查结论

1. Shell 类型和默认 shell 选择在 `codex-rs/shell-command/src/shell_detect.rs`。
   - 已有 `ShellType::Bash`。
   - `detect_shell_type()` 已能识别 `bash` / `bash.exe`。
   - `get_bash_shell()` 当前只查 `bash` 和 Unix fallback 路径：`/bin/bash`、`/usr/bin/bash`。
   - `default_user_shell_from_path()` 在 Windows 上直接选择 `PowerShell`，找不到时退到 `cmd.exe`，完全不会尝试 Bash。

2. Shell 命令参数组装在 `codex-rs/core/src/shell.rs`。
   - `ShellType::Bash` 已按 POSIX shell 方式生成：
     - login shell: `[bash, "-lc", command]`
     - non-login shell: `[bash, "-c", command]`
   - 这部分原则上可直接复用 Git Bash，不需要新增命令组装逻辑。

3. 模型工具描述在 `codex-rs/core/src/tools/handlers/shell_spec.rs`。
   - Windows 上 `shell_command` 描述明确写的是 PowerShell，并给了 PowerShell 示例。
   - `exec_command` 的 `shell` 参数只说可传 shell binary，没有告诉模型 Windows 可选 Git Bash。
   - 支持 Git Bash 后必须更新工具描述，否则模型仍会继续生成 PowerShell 命令。
   - `windows_shell_guidance()` 也默认推荐 PowerShell cmdlets，例如 `Remove-Item` / `Move-Item`，不适合 Git Bash 默认 shell。
   - `codex-rs/core/src/tools/handlers/shell_spec_tests.rs` 固化了这些 Windows PowerShell 文案，提示词改动需要同步更新测试。

4. 环境上下文的 shell 字段是动态渲染的，不是 PowerShell 固定文案。
   - `codex-rs/core/src/context/world_state/environment.rs` 会渲染 `<shell>...</shell>`。
   - 测试里出现 `<shell>powershell</shell>` 只是样例数据。
   - 只要 session 默认 shell 解析为 Git Bash，环境上下文应自然变为对应 shell 名称/路径；实现时需要补一个 Git Bash 环境上下文测试，避免回归。

5. 会话启动默认 shell 在 `codex-rs/core/src/session/session.rs`。
   - 优先使用测试/内部传入的 `user_shell_override`。
   - zsh-fork 开启时强制使用 zsh。
   - 其他情况调用 `shell::default_user_shell()`。
   - Windows Git Bash 默认化最终要接到这里依赖的 shell detection/config 逻辑上。

6. `shell_command` 和 `exec_command` 的执行路径已经支持 Bash 类型。
   - `shell_command`：`codex-rs/core/src/tools/handlers/shell/shell_command.rs`
   - `exec_command`：`codex-rs/core/src/tools/handlers/unified_exec.rs` 与 `.../unified_exec/exec_command.rs`
   - `exec_command.shell` 会通过 `get_shell_by_model_provided_path()` 解析模型传入的 shell 路径，因此显式 `shell = "bash"` 在安装可发现时已经有基础能力。

7. Windows 安全策略需要一起评估。
   - `codex-rs/shell-command/src/command_safety/windows_safe_commands.rs` 当前只把 PowerShell wrapper 当作 Windows 已知安全命令。
   - 通用安全判断 `is_known_safe_command()` 已支持 `bash -lc "..."` 的解析，但 Windows 分支先走 PowerShell 专用逻辑。
   - 若 Git Bash 成为 Windows 默认 shell，读操作、搜索、`git status` 等命令的自动审批/提示行为需要补测试，确认不会变得过严或过松。

8. 配置层当前没有用户默认 shell 配置。
   - `codex-rs/config/src/types.rs` 的 `WindowsToml` 目前只有 sandbox 配置。
   - `ConfigToml` 有 `[windows]` 配置入口，但没有 `shell` / `shell_path`。
   - 如果新增配置字段，需要更新 schema：修改 `ConfigToml` 或嵌套类型后运行 `just write-config-schema`。

9. Windows sandbox 的进程启动本身基本是 shell-agnostic。
   - `codex-rs/windows-sandbox-rs/src/wrapper.rs` 在 `--` 后解析原始 command argv，然后调用 `spawn_windows_sandbox_session_for_level()`。
   - `codex-rs/windows-sandbox-rs/src/process.rs` 最终用 `CreateProcessAsUserW` 启动 argv 组装出的命令行，没有 PowerShell 专用分支。
   - 因此 Git Bash 的主要风险不是命令行组装，而是路径解析、运行时依赖和 sandbox 可读根。

10. Windows sandbox 的 legacy/elevated backend 行为不同。
   - legacy backend 在 `codex-rs/windows-sandbox-rs/src/unified_exec/backends/legacy.rs` 调用 `prepare_legacy_spawn_context(... inherit_path: false, add_git_safe_directory: false)`。
   - elevated backend 在 `codex-rs/windows-sandbox-rs/src/spawn_prep.rs` 中会继承 PATH，并注入 Git `safe.directory`。
   - 结论：Git Bash 默认 shell 必须解析成绝对 `bash.exe` 路径，不能依赖 sandbox 内部 PATH。

11. elevated sandbox 默认平台可读根覆盖常规 Git for Windows 安装，但不覆盖所有用户本地安装。
    - `codex-rs/windows-sandbox-rs/src/setup.rs` 的平台默认只包括：
      - `C:\Windows`
      - `C:\Program Files`
      - `C:\Program Files (x86)`
      - `C:\ProgramData`
    - 本机 Git 来自 Scoop：`C:\Users\ump90\scoop\apps\git\current\cmd\git.exe`。
    - 对应 Git Bash 候选为：
      - `C:\Users\ump90\scoop\apps\git\current\bin\bash.exe`
      - `C:\Users\ump90\scoop\apps\git\current\usr\bin\bash.exe`
    - 结论：只查 Program Files 不够；需要从 `git.exe` 路径反推 Git 安装根，并在严格 read-roots/elevated 场景中确保该安装根可读。

12. direct-spawn Windows sandbox wrapper 对 Git Bash 有额外风险。
    - `codex-rs/sandboxing/src/manager.rs` 的 `wrap_windows_sandbox_exec_request_for_direct_spawn()` 会对 inner program 调用 `resolve_exe_for_launch()`。
    - `resolve_exe_for_launch()` 会把 exe 复制到 `codex_home\.sandbox-bin`。
    - 复制单个 `bash.exe` 可能破坏 Git Bash 对同目录/安装根内 DLL、`usr/bin`、profile 脚本等运行时文件的查找。
    - 结论：如果该路径会接收 Git Bash，不能把 Git Bash 当成可单文件复制的 helper；应运行原始 `bash.exe` 路径，并把 Git 安装根作为 sandbox 可读根。

13. sandbox smoke tests 目前没有 Git Bash 覆盖。
    - `codex-rs/windows-sandbox-rs/sandbox_smoketests.py` 已覆盖 cmd、PowerShell、Python、curl、git 和网络限制等场景。
    - 需要补 Git Bash smoke case，验证 `bash.exe -lc "pwd && ls"`、workspace-write 写入、read-only 拒绝写入，以及用户本地 Git 安装路径。

## 建议实现方案

第一阶段做最小可行支持，避免一次性改变所有 Windows 用户默认行为：

1. 在配置中增加 Windows shell 偏好。
   - 建议字段：
     - `[windows] default_shell = "powershell" | "git-bash" | "cmd"`
     - `[windows] git_bash_path = "C:\\Program Files\\Git\\bin\\bash.exe"` 可选
   - 配置缺省时保持现有 Windows 默认逻辑，不改变所有用户行为。
   - 显式选择 `git-bash` 时要求解析到有效 Git for Windows Bash，否则返回清晰错误。

2. 扩展 Git Bash 检测。
   - 只接受 Git for Windows 安装内的 Bash。候选路径需要能反推出 Git 安装根，并验证关键结构，例如 `cmd\git.exe`、`usr\bin\bash.exe`、`usr\bin\msys-2.0.dll` 等。
   - 解析优先级建议：
     - 显式 `git_bash_path`。
     - 从 `git.exe` 路径反推安装根：`...\cmd\git.exe` -> `...\bin\bash.exe` / `...\usr\bin\bash.exe`。
     - 常规 Program Files fallback：
       - `C:\Program Files\Git\bin\bash.exe`
       - `C:\Program Files\Git\usr\bin\bash.exe`
       - `C:\Program Files (x86)\Git\bin\bash.exe`
     - PATH 上的 `bash.exe` 只有在验证属于 Git for Windows 安装根时才接受。
   - 不支持 MSYS2/Cygwin：如果用户配置的路径不是 Git for Windows Bash，报 unsupported/invalid config，而不是当作普通 Bash 使用。
   - 原有 Unix fallback 路径只用于非 Windows：
     - `/bin/bash`
     - `/usr/bin/bash`

3. 让默认 shell 选择读取配置。
   - 不要把 `Config` 依赖下沉到 `shell-command` crate。
   - 更合适的做法是在 `codex-core` 增加一个小的解析函数，根据 `Config` 的 Windows shell 设置调用现有 `shell::get_shell(ShellType::Bash, path)` / `PowerShell` / `Cmd`。
   - `session.rs` 中替换裸调用 `shell::default_user_shell()` 为配置感知函数。

4. 明确 fallback 和错误信息。
   - `default_shell = "git-bash"` 且找不到 Bash：错误应包含配置键、尝试过的路径、以及建议配置 `windows.git_bash_path`。
   - `git_bash_path` 指向不存在路径：报路径不存在。
   - `git_bash_path` 指向 MSYS2/Cygwin/其他 Bash：报“only Git for Windows Bash is supported”。
   - 未配置 `default_shell` 时继续使用现有 PowerShell -> `cmd.exe` fallback。

5. 更新工具描述和环境提示。
   - `shell_command` 不能再在 Windows 上固定描述为 “Runs a Powershell command”。
   - 推荐短期方案：把 Windows 文案改成 shell-agnostic，并要求模型按 `<environment_context><shell>` 生成命令：
     - Git Bash/default Bash：给 Bash 示例，例如 `ls -la`、`find . -name '*.py'`、`rg TODO`、`FOO=bar python - <<'PY' ... PY`。
     - PowerShell：只在 `<shell>` 明确是 PowerShell 时使用 `Get-ChildItem`、`Select-String`、`$env:FOO=...` 等示例。
     - Cmd：只在 `<shell>` 明确是 cmd 时使用 `dir`、`set FOO=bar && ...`。
   - `windows_shell_guidance()` 改成跨 shell 安全规则：
     - 一次文件操作只用一个 shell，不在 PowerShell、`cmd /c`、Git Bash 之间拼接 destructive command。
     - 删除/移动前解析并核验绝对目标路径在 workspace 或用户明确指定目录内。
     - PowerShell 专用建议移动到 PowerShell 分支；Git Bash 分支应强调 `rm`/`mv` 也必须先核验目标。
   - `exec_command.shell` 描述中补充 Windows 可显式传 Git Bash 的绝对路径，默认 shell 以 `<environment_context><shell>` 为准。
   - 如果要做运行时动态提示，不必改 `ToolExecutor::spec()` trait；`codex-rs/core/src/tools/spec_plan.rs::spec_for_model_request()` 已经拿到 `TurnContext`，可以在最终模型可见 spec 阶段按当前 `user_shell`/environment shell 重写 `exec_command`/`shell_command` 描述。
   - 同步更新 `codex-rs/core/src/tools/handlers/shell_spec_tests.rs`，并新增覆盖 Git Bash Windows 文案不会包含 PowerShell-only 示例。

6. 补齐 Windows sandbox 支持。
   - shell 解析结果需要能提供 Git Bash 安装根，供 sandbox read roots 使用。
   - 当命令使用 Git Bash 且 Windows sandbox 处于 elevated/split read-roots 场景时，把 Git 安装根加入 read roots，避免 `bash.exe` 能启动但 DLL、`usr/bin` 或启动脚本不可读。
   - legacy backend 不继承 PATH，因此默认命令必须始终使用绝对 `bash.exe`。
   - 检查 direct-spawn wrapper 是否会接收 Git Bash；如果会，避免对 Git Bash 调用单文件 materialization，优先运行原始路径并放行安装根。
   - 不要扩大 workspace 写权限；Git 安装根只应作为 read root。

7. 覆盖执行与策略测试。
   - `codex-rs/shell-command/src/shell_detect.rs`：
     - `bash.exe` 能识别为 `ShellType::Bash`。
     - Windows fallback 可找到 Git Bash 路径时返回 Bash。
     - 非 Git for Windows Bash 候选会被拒绝。
     - 从 `git.exe` 路径能推导出 Scoop/用户目录安装的 Git Bash。
   - `codex-rs/core/src/shell_tests.rs`：
     - Git Bash `derive_exec_args()` 仍为 `bash.exe -c/-lc`。
     - 配置选择 Git Bash 时默认 shell 为 Bash。
   - `codex-rs/core/src/tools/handlers/shell_tests.rs`：
     - `shell_command` 使用配置选中的 Bash 组装命令。
   - `codex-rs/core/src/tools/handlers/shell_spec_tests.rs` / `codex-rs/core/src/tools/spec_plan.rs` 相关测试：
     - Windows + Git Bash 默认 shell 时，模型可见 `shell_command` 文案包含 Bash/Git Bash 示例，不包含 “Runs a Powershell command”。
     - Windows + PowerShell 默认 shell 时，PowerShell 示例仍可用。
     - `exec_command.shell` 描述提示默认看 `<environment_context><shell>`，并说明可显式传 Git Bash 绝对路径。
   - `codex-rs/core/src/context/world_state/environment_*tests.rs`：
     - Git Bash shell 字段能正确渲染到 `<shell>...</shell>`，供模型选择命令语法。
   - `codex-rs/core/src/tools/handlers/unified_exec_tests.rs`：
     - 未传 `shell` 时使用配置默认 Bash。
     - 传 `shell = "powershell"` 时仍可覆盖为 PowerShell。
   - `codex-rs/shell-command/src/command_safety/*` 与 `codex-rs/core/src/exec_policy_tests.rs`：
     - Windows 下 `bash -lc "ls"`、`bash -lc "rg foo"`、`bash -lc "git status"` 的审批行为符合预期。
     - `bash -lc "rm -rf ..."`、重定向写文件等危险命令不会被误判为安全。
   - `codex-rs/windows-sandbox-rs/sandbox_smoketests.py`：
     - Git Bash `pwd && ls` 能在 sandbox 内运行。
     - workspace-write 下 `bash -lc "echo ok > file"` 可写 workspace。
     - read-only 下同类写入被拒绝。
     - 用户本地 Git 安装根在 elevated read-roots 场景中可读。

8. 补充构建与验证步骤。
   - 若修改 `ConfigToml` 或嵌套配置类型：运行 `just write-config-schema`。
   - 修改 Rust 代码后在 `codex-rs` 目录运行 `just fmt`。
   - 变更涉及 `codex-core` 和配置解析时，先跑：
     - `just test -p codex-shell-command`
     - `just test -p codex-core`
     - `just test -p codex-sandboxing`
     - `just test -p codex-windows-sandbox`
   - 如果 `codex-core`/协议/配置公共行为变更较大，再征询后跑完整 `just test`。

## 建议实施顺序

1. 已完成：先落配置类型、schema 和配置解析，不改变默认行为。
2. 已完成：再落 Git for Windows Bash discovery 和 validation。
3. 已完成：接入 session 默认 shell 选择。
4. 已完成：更新模型可见提示词，让 Windows shell guidance 不再固定为 PowerShell。
5. 已完成：补 sandbox read-root/direct-spawn 处理。
6. 进行中：补剩余 core 行为测试，并整理 Windows sandbox smoke script 既有失败项。
