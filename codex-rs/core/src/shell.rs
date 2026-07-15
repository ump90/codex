use codex_config::types::WindowsDefaultShellToml;
use codex_exec_server::ShellInfo;
use codex_shell_command::shell_detect::DetectedShell;
use codex_shell_command::shell_detect::GitBashPathHint;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

pub use codex_shell_command::shell_detect::ShellType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Shell {
    pub(crate) shell_type: ShellType,
    pub(crate) shell_path: PathBuf,
}

impl Shell {
    pub fn name(&self) -> &'static str {
        self.shell_type.name()
    }

    /// Takes a string of shell and returns the full list of command args to
    /// use with `exec()` to run the shell command.
    pub fn derive_exec_args(&self, command: &str, use_login_shell: bool) -> Vec<String> {
        match self.shell_type {
            ShellType::Zsh | ShellType::Bash | ShellType::Sh => {
                let arg = if use_login_shell { "-lc" } else { "-c" };
                vec![
                    self.shell_path.to_string_lossy().to_string(),
                    arg.to_string(),
                    command.to_string(),
                ]
            }
            ShellType::PowerShell => {
                let mut args = vec![self.shell_path.to_string_lossy().to_string()];
                if !use_login_shell {
                    args.push("-NoProfile".to_string());
                }

                args.push("-Command".to_string());
                args.push(command.to_string());
                args
            }
            ShellType::Cmd => {
                let mut args = vec![self.shell_path.to_string_lossy().to_string()];
                args.push("/c".to_string());
                args.push(command.to_string());
                args
            }
        }
    }
}

impl From<DetectedShell> for Shell {
    fn from(detected: DetectedShell) -> Self {
        Self {
            shell_type: detected.shell_type,
            shell_path: detected.shell_path,
        }
    }
}

impl Shell {
    pub(crate) fn from_environment_shell_info(shell_info: ShellInfo) -> anyhow::Result<Self> {
        let shell_type = match shell_info.name.as_str() {
            "zsh" => ShellType::Zsh,
            "bash" => ShellType::Bash,
            "powershell" => ShellType::PowerShell,
            "sh" => ShellType::Sh,
            "cmd" => ShellType::Cmd,
            name => anyhow::bail!("unknown environment shell `{name}`"),
        };

        Ok(Self {
            shell_type,
            shell_path: PathBuf::from(shell_info.path),
        })
    }
}

fn ultimate_fallback_shell() -> Shell {
    codex_shell_command::shell_detect::ultimate_fallback_shell().into()
}

pub fn get_shell_by_model_provided_path(shell_path: &PathBuf) -> anyhow::Result<Shell> {
    codex_shell_command::shell_detect::get_shell_by_model_provided_path(shell_path)
        .map(Into::into)
        .map_err(|err| anyhow::anyhow!("{err}"))
}

pub fn get_shell(shell_type: ShellType, path: Option<&PathBuf>) -> Option<Shell> {
    codex_shell_command::shell_detect::get_shell(shell_type, path).map(Into::into)
}

pub fn default_user_shell() -> Shell {
    codex_shell_command::shell_detect::default_user_shell().into()
}

pub fn default_user_shell_for_windows_config(
    default_shell: Option<WindowsDefaultShellToml>,
    git_bash_path: Option<&PathBuf>,
) -> anyhow::Result<Shell> {
    if !cfg!(windows) {
        return Ok(default_user_shell());
    }

    match (default_shell, git_bash_path) {
        (Some(WindowsDefaultShellToml::GitBash), path) => {
            let path_hint = match path {
                Some(path) => GitBashPathHint::Configured(path.as_path()),
                None => GitBashPathHint::SearchPath,
            };
            codex_shell_command::shell_detect::find_git_bash_shell(path_hint)
                .map(|git_bash| git_bash.shell.into())
                .map_err(|err| anyhow::anyhow!("{err}"))
        }
        (None, Some(path)) => codex_shell_command::shell_detect::find_git_bash_shell(
            GitBashPathHint::Configured(path.as_path()),
        )
        .map(|git_bash| git_bash.shell.into())
        .map_err(|err| anyhow::anyhow!("{err}")),
        (None, None) | (Some(WindowsDefaultShellToml::PowerShell), _) => {
            Ok(get_shell(ShellType::PowerShell, /*path*/ None)
                .unwrap_or_else(ultimate_fallback_shell))
        }
        (Some(WindowsDefaultShellToml::Cmd), _) => {
            Ok(get_shell(ShellType::Cmd, /*path*/ None).unwrap_or_else(ultimate_fallback_shell))
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
fn default_user_shell_from_path(user_shell_path: Option<PathBuf>) -> Shell {
    codex_shell_command::shell_detect::default_user_shell_from_path(user_shell_path).into()
}

#[cfg(test)]
#[cfg(unix)]
#[path = "shell_tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(windows)]
#[path = "shell_windows_tests.rs"]
mod windows_tests;
