# Codex CLI Windows Git Bash

这是一个让 Codex CLI 在 Windows 上默认使用 Git Bash 的版本。便携包已经带上 Git for Windows，正常情况下解压后把包含 `codex.cmd` 的目录加入 `PATH` 就能用。

## 安装

1. 下载：

   ```text
   codex-portable-windows-x86_64-pc-windows-msvc.zip
   ```

2. 解压到固定目录，例如：

   ```text
   C:\Tools\codex
   ```

3. 把包含 `codex.cmd` 的解压根目录加入 `PATH`。

4. 打开一个新的终端，运行：

   ```powershell
   codex --version
   codex
   ```

便携包根目录里的 `codex.cmd` 会自动使用内置的 Git Bash。不要把 `bin` 目录加入 `PATH`。

## 使用前建议备份

Codex 的用户配置和会话数据通常在：

```text
%USERPROFILE%\.codex
```

如果你之前用过上游版本或其他分支，建议先备份这个目录，再启动本版本。

PowerShell:

```powershell
Copy-Item "$env:USERPROFILE\.codex" "$env:USERPROFILE\.codex.backup" -Recurse
```

## 可选配置

配置文件：

```text
%USERPROFILE%\.codex\config.toml
```

默认会使用 Git Bash。如果你想明确写出来：

```toml
[windows]
default_shell = "git-bash"
```

如果你不想用便携包里的 Git Bash，而是想用自己安装的 Git for Windows：

```toml
[windows]
default_shell = "git-bash"
git_bash_path = "C:\\Program Files\\Git\\bin\\bash.exe"
```

也可以切回 PowerShell 或 cmd：

```toml
[windows]
default_shell = "powershell"
# default_shell = "cmd"
```

## 常见问题

### 中文乱码

这个版本会尽量让 Git Bash 相关命令使用 UTF-8，避免中文 Windows 下常见的 GBK 乱码。

如果仍然乱码，先确认：

- 运行的是解压根目录里的 `codex.cmd`。
- `PATH` 里加入的是包含 `codex.cmd` 的解压根目录。
- 乱码是否来自原生 Windows 程序，例如 `cmd.exe`、MSVC 工具链或某些 Python 程序；这类程序可能不读取 Git Bash 的 `LANG/LC_*` 设置，仍按系统代码页输出。

最后手段：在 Windows“语言和区域”设置中开启“使用 Unicode UTF-8 提供全球语言支持”。这是系统级设置，可能影响其他旧程序。

### 找不到 Git Bash

优先检查 `PATH` 里加入的是包含 `codex.cmd` 的解压根目录，例如：

```text
C:\Tools\codex
```

不要加入这个目录：

```text
C:\Tools\codex\bin
```

如果绕过 `codex.cmd` 直接运行 `bin\codex.exe`，Codex 可能找不到便携包内置的 Git Bash。

## 相关链接

- [OpenAI Codex](https://github.com/openai/codex)
- [Codex Documentation](https://developers.openai.com/codex)
- [License](./LICENSE)
