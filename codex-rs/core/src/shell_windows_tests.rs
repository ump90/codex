use super::*;

use std::path::Path;

use pretty_assertions::assert_eq;

struct TempFixture {
    path: PathBuf,
}

impl TempFixture {
    fn new(name: &str) -> std::io::Result<Self> {
        let unique = format!(
            "codex-core-shell-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn touch(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, [])?;
    Ok(())
}

fn create_git_for_windows_fixture(root: &Path) -> std::io::Result<()> {
    touch(&root.join("cmd").join("git.exe"))?;
    touch(&root.join("bin").join("bash.exe"))?;
    touch(&root.join("usr").join("bin").join("msys-2.0.dll"))?;
    Ok(())
}

#[test]
fn default_user_shell_for_windows_config_uses_git_bash_by_default() -> anyhow::Result<()> {
    let fixture = TempFixture::new("git-bash-default")?;
    let git_root = fixture.path().join("Git");
    create_git_for_windows_fixture(&git_root)?;
    let bash_path = git_root.join("bin").join("bash.exe");

    let shell = default_user_shell_for_windows_config(None, Some(&bash_path))?;

    assert_eq!(
        shell,
        Shell {
            shell_type: ShellType::Bash,
            shell_path: bash_path.clone(),
        }
    );
    assert_eq!(
        shell.derive_exec_args("echo hello", /*use_login_shell*/ false),
        vec![
            bash_path.to_string_lossy().to_string(),
            "-c".to_string(),
            "echo hello".to_string(),
        ]
    );
    assert_eq!(
        shell.derive_exec_args("echo hello", /*use_login_shell*/ true),
        vec![
            bash_path.to_string_lossy().to_string(),
            "-lc".to_string(),
            "echo hello".to_string(),
        ]
    );

    Ok(())
}

#[test]
fn default_user_shell_for_windows_config_falls_back_to_powershell() -> anyhow::Result<()> {
    let shell = default_user_shell_for_windows_config(
        /*default_shell*/ None, /*git_bash_path*/ None,
    )?;

    assert_eq!(shell.shell_type, ShellType::PowerShell);

    Ok(())
}

#[test]
fn default_user_shell_for_windows_config_rejects_invalid_git_bash_path() -> anyhow::Result<()> {
    let fixture = TempFixture::new("git-bash-invalid")?;
    let bash_path = fixture
        .path()
        .join("msys64")
        .join("usr")
        .join("bin")
        .join("bash.exe");
    touch(&bash_path)?;

    let err = default_user_shell_for_windows_config(
        Some(WindowsDefaultShellToml::GitBash),
        Some(&bash_path),
    )
    .expect_err("non-Git-for-Windows bash should be rejected");

    assert!(
        err.to_string()
            .contains("only Git for Windows bash.exe is supported")
    );

    Ok(())
}
