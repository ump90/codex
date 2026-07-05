use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum ShellType {
    Zsh,
    Bash,
    PowerShell,
    Sh,
    Cmd,
}

impl ShellType {
    pub fn name(self) -> &'static str {
        match self {
            Self::Zsh => "zsh",
            Self::Bash => "bash",
            Self::PowerShell => "powershell",
            Self::Sh => "sh",
            Self::Cmd => "cmd",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedShell {
    pub shell_type: ShellType,
    pub shell_path: PathBuf,
}

impl DetectedShell {
    pub fn name(&self) -> &'static str {
        self.shell_type.name()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBashShell {
    pub shell: DetectedShell,
    pub installation_root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitBashPathHint<'a> {
    SearchPath,
    Configured(&'a Path),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBashDiscoveryError {
    message: String,
    attempted_paths: Vec<PathBuf>,
}

impl GitBashDiscoveryError {
    fn new(message: impl Into<String>, attempted_paths: Vec<PathBuf>) -> Self {
        Self {
            message: message.into(),
            attempted_paths,
        }
    }

    pub fn attempted_paths(&self) -> &[PathBuf] {
        &self.attempted_paths
    }
}

impl std::fmt::Display for GitBashDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)?;
        if !self.attempted_paths.is_empty() {
            let attempted_paths = self
                .attempted_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            write!(f, " Tried: {attempted_paths}")?;
        }
        Ok(())
    }
}

impl std::error::Error for GitBashDiscoveryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellResolutionError {
    message: String,
}

impl ShellResolutionError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ShellResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ShellResolutionError {}

pub fn find_git_bash_shell(
    path_hint: GitBashPathHint<'_>,
) -> Result<GitBashShell, GitBashDiscoveryError> {
    #[cfg(windows)]
    {
        find_git_bash_shell_windows(path_hint)
    }

    #[cfg(not(windows))]
    {
        let attempted_paths = match path_hint {
            GitBashPathHint::SearchPath => Vec::new(),
            GitBashPathHint::Configured(path) => vec![path.to_path_buf()],
        };
        Err(GitBashDiscoveryError::new(
            "Git Bash is only supported on Windows",
            attempted_paths,
        ))
    }
}

pub fn detect_shell_type(shell_path: impl AsRef<std::path::Path>) -> Option<ShellType> {
    let shell_path = shell_path.as_ref();
    match shell_path.as_os_str().to_str() {
        Some("zsh") => Some(ShellType::Zsh),
        Some("sh") => Some(ShellType::Sh),
        Some("cmd") => Some(ShellType::Cmd),
        Some("bash") => Some(ShellType::Bash),
        Some("pwsh") => Some(ShellType::PowerShell),
        Some("powershell") => Some(ShellType::PowerShell),
        _ => {
            let shell_name = shell_path.file_stem();
            if let Some(shell_name) = shell_name {
                let shell_name_path = std::path::Path::new(shell_name);
                if shell_name_path != shell_path {
                    return detect_shell_type(shell_name_path);
                }
            }
            None
        }
    }
}

#[cfg(unix)]
fn get_user_shell_path() -> Option<PathBuf> {
    let uid = unsafe { libc::getuid() };
    use std::ffi::CStr;
    use std::mem::MaybeUninit;
    use std::ptr;

    let mut passwd = MaybeUninit::<libc::passwd>::uninit();

    // We cannot use getpwuid here: it returns pointers into libc-managed
    // storage, which is not safe to read concurrently on all targets (the musl
    // static build used by the CLI can segfault when parallel callers race on
    // that buffer). getpwuid_r keeps the passwd data in caller-owned memory.
    let suggested_buffer_len = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
    let buffer_len = usize::try_from(suggested_buffer_len)
        .ok()
        .filter(|len| *len > 0)
        .unwrap_or(1024);
    let mut buffer = vec![0; buffer_len];

    loop {
        let mut result = ptr::null_mut();
        let status = unsafe {
            libc::getpwuid_r(
                uid,
                passwd.as_mut_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut result,
            )
        };

        if status == 0 {
            if result.is_null() {
                return None;
            }

            let passwd = unsafe { passwd.assume_init_ref() };
            if passwd.pw_shell.is_null() {
                return None;
            }

            let shell_path = unsafe { CStr::from_ptr(passwd.pw_shell) }
                .to_string_lossy()
                .into_owned();
            return Some(PathBuf::from(shell_path));
        }

        if status != libc::ERANGE {
            return None;
        }

        // Retry with a larger buffer until libc can materialize the passwd entry.
        let new_len = buffer.len().checked_mul(2)?;
        if new_len > 1024 * 1024 {
            return None;
        }
        buffer.resize(new_len, 0);
    }
}

#[cfg(not(unix))]
fn get_user_shell_path() -> Option<PathBuf> {
    None
}

fn file_exists(path: &std::path::Path) -> Option<PathBuf> {
    if std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        Some(PathBuf::from(path))
    } else {
        None
    }
}

fn get_shell_path(
    shell_type: ShellType,
    provided_path: Option<&PathBuf>,
    binary_name: &str,
    fallback_paths: &[&str],
) -> Option<PathBuf> {
    if let Some(path) = provided_path.and_then(|path| file_exists(path)) {
        return Some(path);
    }

    let default_shell_path = get_user_shell_path();
    if let Some(default_shell_path) = default_shell_path
        && detect_shell_type(&default_shell_path) == Some(shell_type)
        && file_exists(&default_shell_path).is_some()
    {
        return Some(default_shell_path);
    }

    if let Ok(path) = which::which(binary_name) {
        return Some(path);
    }

    for path in fallback_paths {
        if let Some(path) = file_exists(std::path::Path::new(path)) {
            return Some(path);
        }
    }

    None
}

const ZSH_FALLBACK_PATHS: &[&str] = &["/bin/zsh"];

fn get_zsh_shell(path: Option<&PathBuf>) -> Option<DetectedShell> {
    let shell_path = get_shell_path(ShellType::Zsh, path, "zsh", ZSH_FALLBACK_PATHS);

    shell_path.map(|shell_path| DetectedShell {
        shell_type: ShellType::Zsh,
        shell_path,
    })
}

#[cfg(not(windows))]
const BASH_FALLBACK_PATHS: &[&str] = &["/bin/bash", "/usr/bin/bash"];

#[cfg(windows)]
fn get_bash_shell(path: Option<&PathBuf>) -> Option<DetectedShell> {
    find_git_bash_shell(git_bash_path_hint_for_bash(path))
        .ok()
        .map(|git_bash| git_bash.shell)
}

#[cfg(not(windows))]
fn get_bash_shell(path: Option<&PathBuf>) -> Option<DetectedShell> {
    let shell_path = get_shell_path(ShellType::Bash, path, "bash", BASH_FALLBACK_PATHS);

    shell_path.map(|shell_path| DetectedShell {
        shell_type: ShellType::Bash,
        shell_path,
    })
}

const SH_FALLBACK_PATHS: &[&str] = &["/bin/sh"];

fn get_sh_shell(path: Option<&PathBuf>) -> Option<DetectedShell> {
    let shell_path = get_shell_path(ShellType::Sh, path, "sh", SH_FALLBACK_PATHS);

    shell_path.map(|shell_path| DetectedShell {
        shell_type: ShellType::Sh,
        shell_path,
    })
}

// Note the `pwsh` and `powershell` fallback paths are where the respective
// shells are commonly installed on GitHub Actions Windows runners, but may not
// be present on all Windows machines:
// https://docs.github.com/en/actions/tutorials/build-and-test-code/powershell

#[cfg(windows)]
const PWSH_FALLBACK_PATHS: &[&str] = &[r#"C:\Program Files\PowerShell\7\pwsh.exe"#];
#[cfg(not(windows))]
const PWSH_FALLBACK_PATHS: &[&str] = &["/usr/local/bin/pwsh"];

#[cfg(windows)]
const POWERSHELL_FALLBACK_PATHS: &[&str] =
    &[r#"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"#];
#[cfg(not(windows))]
const POWERSHELL_FALLBACK_PATHS: &[&str] = &[];

fn get_powershell_shell(path: Option<&PathBuf>) -> Option<DetectedShell> {
    let shell_path = get_shell_path(ShellType::PowerShell, path, "pwsh", PWSH_FALLBACK_PATHS)
        .or_else(|| {
            get_shell_path(
                ShellType::PowerShell,
                path,
                "powershell",
                POWERSHELL_FALLBACK_PATHS,
            )
        });

    shell_path.map(|shell_path| DetectedShell {
        shell_type: ShellType::PowerShell,
        shell_path,
    })
}

fn get_cmd_shell(path: Option<&PathBuf>) -> Option<DetectedShell> {
    let shell_path = get_shell_path(ShellType::Cmd, path, "cmd", &[]);

    shell_path.map(|shell_path| DetectedShell {
        shell_type: ShellType::Cmd,
        shell_path,
    })
}

pub fn ultimate_fallback_shell() -> DetectedShell {
    if cfg!(windows) {
        DetectedShell {
            shell_type: ShellType::Cmd,
            shell_path: PathBuf::from("cmd.exe"),
        }
    } else {
        DetectedShell {
            shell_type: ShellType::Sh,
            shell_path: PathBuf::from("/bin/sh"),
        }
    }
}

pub fn get_shell_by_model_provided_path(
    shell_path: &PathBuf,
) -> Result<DetectedShell, ShellResolutionError> {
    let shell_type = detect_shell_type(shell_path).ok_or_else(|| {
        ShellResolutionError::new(format!(
            "unsupported shell `{}`; expected zsh, bash, powershell, pwsh, sh, or cmd",
            shell_path.display()
        ))
    })?;

    resolve_model_provided_shell(shell_type, shell_path)
}

pub fn get_shell(shell_type: ShellType, path: Option<&PathBuf>) -> Option<DetectedShell> {
    match shell_type {
        ShellType::Zsh => get_zsh_shell(path),
        ShellType::Bash => get_bash_shell(path),
        ShellType::PowerShell => get_powershell_shell(path),
        ShellType::Sh => get_sh_shell(path),
        ShellType::Cmd => get_cmd_shell(path),
    }
}

pub fn default_user_shell() -> DetectedShell {
    default_user_shell_from_path(get_user_shell_path())
}

pub fn default_user_shell_from_path(user_shell_path: Option<PathBuf>) -> DetectedShell {
    if cfg!(windows) {
        get_shell(ShellType::PowerShell, /*path*/ None).unwrap_or_else(ultimate_fallback_shell)
    } else {
        let user_default_shell = user_shell_path
            .and_then(|shell| detect_shell_type(&shell))
            .and_then(|shell_type| get_shell(shell_type, /*path*/ None));

        let shell_with_fallback = if cfg!(target_os = "macos") {
            user_default_shell
                .or_else(|| get_shell(ShellType::Zsh, /*path*/ None))
                .or_else(|| get_shell(ShellType::Bash, /*path*/ None))
        } else {
            user_default_shell
                .or_else(|| get_shell(ShellType::Bash, /*path*/ None))
                .or_else(|| get_shell(ShellType::Zsh, /*path*/ None))
        };

        shell_with_fallback.unwrap_or_else(ultimate_fallback_shell)
    }
}

#[cfg(windows)]
fn git_bash_path_hint_for_bash(path: Option<&PathBuf>) -> GitBashPathHint<'_> {
    match path {
        Some(path) if path.components().count() > 1 || path.is_absolute() => {
            GitBashPathHint::Configured(path.as_path())
        }
        Some(_) | None => GitBashPathHint::SearchPath,
    }
}

#[cfg(windows)]
fn resolve_model_provided_shell(
    shell_type: ShellType,
    shell_path: &PathBuf,
) -> Result<DetectedShell, ShellResolutionError> {
    if shell_type == ShellType::Bash {
        return find_git_bash_shell(git_bash_path_hint_for_bash(Some(shell_path)))
            .map(|git_bash| git_bash.shell)
            .map_err(|err| {
                ShellResolutionError::new(format!(
                    "could not resolve requested bash shell `{}`: {err}",
                    shell_path.display()
                ))
            });
    }

    get_shell(shell_type, Some(shell_path)).ok_or_else(|| {
        ShellResolutionError::new(format!(
            "could not resolve requested shell `{}`",
            shell_path.display()
        ))
    })
}

#[cfg(not(windows))]
fn resolve_model_provided_shell(
    shell_type: ShellType,
    shell_path: &PathBuf,
) -> Result<DetectedShell, ShellResolutionError> {
    get_shell(shell_type, Some(shell_path)).ok_or_else(|| {
        ShellResolutionError::new(format!(
            "could not resolve requested shell `{}`",
            shell_path.display()
        ))
    })
}

#[cfg(windows)]
fn find_git_bash_shell_windows(
    path_hint: GitBashPathHint<'_>,
) -> Result<GitBashShell, GitBashDiscoveryError> {
    match path_hint {
        GitBashPathHint::Configured(path) => {
            let attempted_path = path.to_path_buf();
            if !path.is_absolute() {
                return Err(GitBashDiscoveryError::new(
                    format!(
                        "`windows.git_bash_path` must be an absolute path to Git for Windows bash.exe: {}",
                        path.display()
                    ),
                    vec![attempted_path],
                ));
            }
            let Some(shell_path) = file_exists(path) else {
                return Err(GitBashDiscoveryError::new(
                    format!(
                        "`windows.git_bash_path` points to `{}` but that file does not exist",
                        path.display()
                    ),
                    vec![attempted_path],
                ));
            };
            git_bash_from_candidate(&shell_path).ok_or_else(|| {
                GitBashDiscoveryError::new(
                    format!(
                        "`windows.git_bash_path` points to `{}` but only Git for Windows bash.exe is supported",
                        path.display()
                    ),
                    vec![shell_path],
                )
            })
        }
        GitBashPathHint::SearchPath => {
            let candidates = git_bash_search_candidates();
            for candidate in &candidates {
                if let Some(git_bash) = file_exists(candidate)
                    .as_deref()
                    .and_then(git_bash_from_candidate)
                {
                    return Ok(git_bash);
                }
            }
            Err(GitBashDiscoveryError::new(
                "`windows.default_shell = \"git-bash\"` was configured, but no Git for Windows Bash was found. Set `windows.git_bash_path` to the absolute path of Git for Windows `bash.exe`.",
                candidates,
            ))
        }
    }
}

#[cfg(windows)]
fn git_bash_from_candidate(shell_path: &Path) -> Option<GitBashShell> {
    let installation_root = git_for_windows_install_root_from_bash(shell_path)?;
    Some(GitBashShell {
        shell: DetectedShell {
            shell_type: ShellType::Bash,
            shell_path: shell_path.to_path_buf(),
        },
        installation_root,
    })
}

#[cfg(windows)]
fn git_bash_search_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(git_path) = which::which("git") {
        candidates.extend(git_bash_candidates_from_git_exe(&git_path));
    }
    candidates.extend([
        PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"),
        PathBuf::from(r"C:\Program Files\Git\usr\bin\bash.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Git\bin\bash.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Git\usr\bin\bash.exe"),
    ]);
    if let Ok(bash_path) = which::which("bash") {
        candidates.push(bash_path);
    }
    dedupe_paths(&mut candidates);
    candidates
}

#[cfg(windows)]
fn git_bash_candidates_from_git_exe(git_path: &Path) -> Vec<PathBuf> {
    git_for_windows_install_root_from_git_exe(git_path)
        .map(|root| {
            vec![
                root.join("bin").join("bash.exe"),
                root.join("usr").join("bin").join("bash.exe"),
            ]
        })
        .unwrap_or_default()
}

#[cfg(windows)]
fn git_for_windows_install_root_from_git_exe(git_path: &Path) -> Option<PathBuf> {
    if !path_file_stem_eq(git_path, "git") {
        return None;
    }

    let parent = git_path.parent()?;
    if path_file_name_eq(parent, "shims") {
        return parent
            .parent()
            .map(|scoop_root| scoop_root.join("apps").join("git").join("current"));
    }

    if path_file_name_eq(parent, "cmd") {
        return parent.parent().map(Path::to_path_buf);
    }

    if path_file_name_eq(parent, "bin") {
        let grandparent = parent.parent()?;
        if path_file_name_eq(grandparent, "mingw64")
            || path_file_name_eq(grandparent, "mingw32")
            || path_file_name_eq(grandparent, "usr")
        {
            return grandparent.parent().map(Path::to_path_buf);
        }
    }

    None
}

#[cfg(windows)]
pub fn git_for_windows_install_root_from_bash(shell_path: &Path) -> Option<PathBuf> {
    if !path_file_stem_eq(shell_path, "bash") {
        return None;
    }

    let parent = shell_path.parent()?;
    let root = if path_file_name_eq(parent, "bin") {
        let grandparent = parent.parent()?;
        if path_file_name_eq(grandparent, "usr") {
            grandparent.parent()?.to_path_buf()
        } else {
            grandparent.to_path_buf()
        }
    } else {
        return None;
    };

    let git = root.join("cmd").join("git.exe");
    let msys = root.join("usr").join("bin").join("msys-2.0.dll");
    if file_exists(&git).is_some() && file_exists(&msys).is_some() {
        Some(root)
    } else {
        None
    }
}

#[cfg(windows)]
fn path_file_stem_eq(path: &Path, expected: &str) -> bool {
    path.file_stem()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

#[cfg(windows)]
fn path_file_name_eq(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

#[cfg(windows)]
fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    let mut normalized = Vec::<String>::new();
    paths.retain(|path| {
        let key = path.to_string_lossy().to_lowercase();
        if normalized.contains(&key) {
            false
        } else {
            normalized.push(key);
            true
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_detect_shell_type() {
        assert_eq!(
            detect_shell_type(PathBuf::from("zsh")),
            Some(ShellType::Zsh)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("bash")),
            Some(ShellType::Bash)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("bash.exe")),
            Some(ShellType::Bash)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("pwsh")),
            Some(ShellType::PowerShell)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("powershell")),
            Some(ShellType::PowerShell)
        );
        assert_eq!(detect_shell_type(PathBuf::from("fish")), None);
        assert_eq!(detect_shell_type(PathBuf::from("other")), None);
        assert_eq!(
            detect_shell_type(PathBuf::from("/bin/zsh")),
            Some(ShellType::Zsh)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("/bin/bash")),
            Some(ShellType::Bash)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("/usr/bin/bash")),
            Some(ShellType::Bash)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("powershell.exe")),
            Some(ShellType::PowerShell)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from(if cfg!(windows) {
                "C:\\windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
            } else {
                "/usr/local/bin/pwsh"
            })),
            Some(ShellType::PowerShell)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("pwsh.exe")),
            Some(ShellType::PowerShell)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("/usr/local/bin/pwsh")),
            Some(ShellType::PowerShell)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("/bin/sh")),
            Some(ShellType::Sh)
        );
        assert_eq!(detect_shell_type(PathBuf::from("sh")), Some(ShellType::Sh));
        assert_eq!(
            detect_shell_type(PathBuf::from("cmd")),
            Some(ShellType::Cmd)
        );
        assert_eq!(
            detect_shell_type(PathBuf::from("cmd.exe")),
            Some(ShellType::Cmd)
        );
    }

    #[cfg(windows)]
    struct TempFixture {
        path: PathBuf,
    }

    #[cfg(windows)]
    impl TempFixture {
        fn new(name: &str) -> std::io::Result<Self> {
            let unique = format!(
                "codex-shell-detect-{name}-{}-{}",
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

    #[cfg(windows)]
    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[cfg(windows)]
    fn touch(path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, [])?;
        Ok(())
    }

    #[cfg(windows)]
    fn create_git_for_windows_fixture(root: &Path) -> std::io::Result<()> {
        touch(&root.join("cmd").join("git.exe"))?;
        touch(&root.join("bin").join("bash.exe"))?;
        touch(&root.join("usr").join("bin").join("bash.exe"))?;
        touch(&root.join("usr").join("bin").join("msys-2.0.dll"))?;
        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn configured_git_bash_accepts_git_for_windows_bin_and_usr_bin() -> anyhow::Result<()> {
        let fixture = TempFixture::new("git-bash-configured")?;
        let git_root = fixture.path().join("Git");
        create_git_for_windows_fixture(&git_root)?;

        for bash_path in [
            git_root.join("bin").join("bash.exe"),
            git_root.join("usr").join("bin").join("bash.exe"),
        ] {
            let shell = find_git_bash_shell(GitBashPathHint::Configured(&bash_path))?;
            assert_eq!(
                shell,
                GitBashShell {
                    shell: DetectedShell {
                        shell_type: ShellType::Bash,
                        shell_path: bash_path,
                    },
                    installation_root: git_root.clone(),
                }
            );
        }

        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn configured_git_bash_rejects_non_git_for_windows_bash() -> anyhow::Result<()> {
        let fixture = TempFixture::new("git-bash-invalid")?;
        let bash_path = fixture
            .path()
            .join("msys64")
            .join("usr")
            .join("bin")
            .join("bash.exe");
        touch(&bash_path)?;

        let err = find_git_bash_shell(GitBashPathHint::Configured(&bash_path))
            .expect_err("non-Git-for-Windows bash should be rejected");

        assert!(
            err.to_string()
                .contains("only Git for Windows bash.exe is supported")
        );
        assert_eq!(err.attempted_paths(), std::slice::from_ref(&bash_path));

        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn model_provided_non_git_bash_returns_error() -> anyhow::Result<()> {
        let fixture = TempFixture::new("git-bash-model-invalid")?;
        let bash_path = fixture
            .path()
            .join("msys64")
            .join("usr")
            .join("bin")
            .join("bash.exe");
        touch(&bash_path)?;

        let err = get_shell_by_model_provided_path(&bash_path)
            .expect_err("non-Git-for-Windows bash should not fall back to cmd");

        assert!(
            err.to_string()
                .contains("only Git for Windows bash.exe is supported")
        );

        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn git_for_windows_candidates_are_derived_from_git_exe_locations() {
        let program_files_git = PathBuf::from(r"C:\Program Files\Git\cmd\git.exe");
        assert_eq!(
            git_bash_candidates_from_git_exe(&program_files_git),
            vec![
                PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"),
                PathBuf::from(r"C:\Program Files\Git\usr\bin\bash.exe"),
            ]
        );

        let scoop_git = PathBuf::from(r"C:\Users\me\scoop\shims\git.exe");
        assert_eq!(
            git_bash_candidates_from_git_exe(&scoop_git),
            vec![
                PathBuf::from(r"C:\Users\me\scoop\apps\git\current\bin\bash.exe"),
                PathBuf::from(r"C:\Users\me\scoop\apps\git\current\usr\bin\bash.exe"),
            ]
        );

        let mingw_git = PathBuf::from(r"C:\Git\mingw64\bin\git.exe");
        assert_eq!(
            git_bash_candidates_from_git_exe(&mingw_git),
            vec![
                PathBuf::from(r"C:\Git\bin\bash.exe"),
                PathBuf::from(r"C:\Git\usr\bin\bash.exe"),
            ]
        );
    }
}
