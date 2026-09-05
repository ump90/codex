# Codex CLI Windows Git Bash 二开版

这是基于 [OpenAI Codex](https://github.com/openai/codex) 的 Windows 二开分支，当前开发与发布分支为 `codex/fork-release`。

该版本保留上游 Codex 能力，并重点完善 Windows 下的 Git Bash、路径转换、沙箱执行和便携发布体验。便携包已包含 Git for Windows，解压后通过根目录的 `codex.cmd` 启动即可使用。

## 主要功能

- 自动发现 Git for Windows，并在 Windows 上优先使用 Git Bash；未找到时回退到 PowerShell。
- 支持通过 `[windows].default_shell` 在 Git Bash、PowerShell 和 cmd 之间切换。
- 模型上下文中的 cwd、workspace roots 和权限路径会按照默认 shell 展示，例如 Git Bash 下使用 `/d/Workspace/codex`。
- `shell_command`、`exec_command`、`apply_patch`、`view_image` 和权限工具能够正确处理 Git Bash 与 Windows 原生路径。
- 显式切换 PowerShell 或 cmd 只改变命令解释器，不改变模型已经看到的路径约定；Git Bash 形式的 `workdir` 仍会转换为正确的 Windows 路径。
- Windows 沙箱可以只读使用完整的 Git for Windows 运行时，而不是只复制一个 `bash.exe`。
- Git Bash 命令使用 `C.UTF-8` locale，降低中文 Windows 下的乱码概率。
- 提供包含 Portable Git、ripgrep 和 Codex 辅助程序的 Windows 便携包。
- 兼容因 LF/CRLF 行尾差异产生的历史 SQLx migration checksum。

## 安装便携版 CLI

从本仓库的 GitHub Release 下载对应架构的文件：

```text
codex-portable-windows-x86_64-pc-windows-msvc.zip
codex-portable-windows-aarch64-pc-windows-msvc.zip
```

安装步骤：

1. 将压缩包解压到固定目录，例如 `C:\Tools\codex`。
2. 将包含 `codex.cmd` 的解压根目录加入 `PATH`，不要只加入 `bin` 目录。
3. 打开一个新终端并运行：

   ```powershell
   codex --version
   codex
   ```

`codex.cmd` 会先把便携包内的 Git for Windows 加入当前进程环境，再启动 `bin\codex.exe`。绕过 `codex.cmd` 直接运行可执行文件时，Codex 可能无法发现内置 Git Bash。

## Codex App 便携包

Release 还可能包含：

```text
codex-app-portable-windows-x86_64-pc-windows-msvc.zip
codex-app-portable-windows-aarch64-pc-windows-msvc.zip
```

解压后通过 `codex-app.cmd` 启动。该制品替换了上游 App 包中的部分 sidecar，因此不是可直接安装的 MSIX。

## 使用前备份

Codex 的用户配置和会话数据通常位于：

```text
%USERPROFILE%\.codex
```

从上游版本或其他分支切换前，建议先备份：

```powershell
Copy-Item "$env:USERPROFILE\.codex" "$env:USERPROFILE\.codex.backup" -Recurse
```

## Windows shell 配置

配置文件通常为 `%USERPROFILE%\.codex\config.toml`。

默认使用 Git Bash：

```toml
[windows]
default_shell = "git-bash"
```

指定已有的 Git for Windows：

```toml
[windows]
default_shell = "git-bash"
git_bash_path = "C:\\Program Files\\Git\\bin\\bash.exe"
```

切换回原生 Windows shell：

```toml
[windows]
default_shell = "powershell"
# default_shell = "cmd"
```

`git_bash_path` 必须是有效 Git for Windows `bash.exe` 的绝对路径。显式选择 Git Bash 但路径无效时，Codex 会返回配置错误，不会静默使用其他 Bash 实现。

## 路径规则

当默认 shell 是 Git Bash 时，模型看到的 Windows 路径采用 Git Bash 形式：

```text
C:\Users\Alice\project  ->  /c/Users/Alice/project
```

工具接收到结构化路径字段后，会在调用 Windows API 或原生 shell 前转换回 Windows 路径。`/usr/bin` 等 MSYS 路径不会被误判为磁盘路径。

`exec_command` 的显式 `shell` 参数只决定如何解释 `cmd`，不会改变本轮模型上下文的路径约定。例如默认 Git Bash 时，即使显式选择 `cmd.exe`，`workdir: "/d/work"` 仍会解析为 `D:\work`。

Git Bash 路径仍应用于命令和工具的结构化路径参数。Codex App 中需要点击打开的本地 Markdown 链接则使用 Windows 原生绝对路径和正斜杠，例如 `[FORK_README.md](D:/Workspace/codex/FORK_README.md)`；不要将 `/d/Workspace/codex/FORK_README.md` 用作链接目标。

## 常见问题

### 找不到 Git Bash

- 确认 `PATH` 中加入的是包含 `codex.cmd` 的便携包根目录。
- 确认通过 `codex.cmd` 启动，而不是直接运行 `bin\codex.exe`。
- 使用系统安装的 Git for Windows 时，检查 `git_bash_path` 是否指向对应的 `bin\bash.exe`。

### 中文乱码

本版本会为 Git Bash 命令设置 `LANG`、`LC_CTYPE` 和 `LC_ALL=C.UTF-8`。如果乱码来自 `cmd.exe`、MSVC 或其他原生 Windows 程序，它们仍可能按照系统代码页输出，需要单独调整程序或终端编码。

## 分支与上游同步

- `main`：跟踪 `openai/codex:main`。
- `codex/fork-release`：包含 Windows Git Bash 和便携发布相关二开功能。
- 上游同步完成后，需要将 `main` 合并到 `codex/fork-release`，并复核二开功能没有因主线结构变化而退化。

维护者可继续阅读：

- [二开维护与合并手册](./FORK_README.md)
- [上游 Codex 文档](https://developers.openai.com/codex)
- [许可证](./LICENSE)
