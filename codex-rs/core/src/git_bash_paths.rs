use codex_utils_path_uri::PathConvention;
use codex_utils_path_uri::PathUri;
use serde_json::Value;
use std::path::Path;

use crate::shell::ShellType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathDisplayStyle {
    Native,
    GitBash,
}

pub(crate) fn path_display_style_for_shell(
    shell_name: Option<&str>,
    cwd: &PathUri,
) -> PathDisplayStyle {
    if shell_name == Some(ShellType::Bash.name())
        && cwd.infer_path_convention() == Some(PathConvention::Windows)
    {
        PathDisplayStyle::GitBash
    } else {
        PathDisplayStyle::Native
    }
}

pub(crate) fn format_path_uri_for_shell(path: &PathUri, style: PathDisplayStyle) -> String {
    let path = path.inferred_native_path_string();
    format_path_text_for_shell(&path, style)
}

pub(crate) fn format_native_path_for_shell(path: &Path, style: PathDisplayStyle) -> String {
    let path = path.to_string_lossy();
    format_path_text_for_shell(path.as_ref(), style)
}

pub(crate) fn format_path_text_for_shell(path: &str, style: PathDisplayStyle) -> String {
    match style {
        PathDisplayStyle::Native => path.to_string(),
        PathDisplayStyle::GitBash => {
            windows_path_to_git_bash_path(path).unwrap_or_else(|| path.replace('\\', "/"))
        }
    }
}

pub(crate) fn git_bash_path_to_windows_path(path: &str) -> Option<String> {
    if let Some(path) = path.strip_prefix("//") {
        if path.is_empty() {
            return None;
        }
        return Some(format!(r"\\{}", path.replace('/', "\\")));
    }

    let rest = path.strip_prefix('/')?;
    let mut parts = rest.splitn(2, '/');
    let drive = parts.next()?;
    let tail = parts.next();
    let drive_bytes = drive.as_bytes();
    if !matches!(drive_bytes, [drive] if drive.is_ascii_alphabetic()) {
        return None;
    }

    let drive = (drive_bytes[0] as char).to_ascii_uppercase();
    let mut windows = format!("{drive}:\\");
    if let Some(tail) = tail
        && !tail.is_empty()
    {
        windows.push_str(&tail.replace('/', "\\"));
    }
    Some(windows)
}

fn windows_path_to_git_bash_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    if bytes.len() < 3
        || bytes[1] != b':'
        || !bytes[0].is_ascii_alphabetic()
        || !is_windows_separator(bytes[2])
    {
        return windows_unc_path_to_git_bash_path(path);
    }

    let drive = (bytes[0] as char).to_ascii_lowercase();
    let tail = path[3..].replace('\\', "/");
    if tail.is_empty() {
        Some(format!("/{drive}/"))
    } else {
        Some(format!("/{drive}/{tail}"))
    }
}

fn windows_unc_path_to_git_bash_path(path: &str) -> Option<String> {
    let path = path.strip_prefix(r"\\")?;
    let path = path.replace('\\', "/");
    Some(format!("//{path}"))
}

fn is_windows_separator(byte: u8) -> bool {
    matches!(byte, b'\\' | b'/')
}

pub(crate) fn rewrite_git_bash_path_arguments(arguments: &str) -> serde_json::Result<String> {
    let mut value: Value = serde_json::from_str(arguments)?;
    rewrite_git_bash_path_arguments_value(&mut value);
    serde_json::to_string(&value)
}

fn rewrite_git_bash_path_arguments_value(value: &mut Value) {
    let Value::Object(arguments) = value else {
        return;
    };

    rewrite_string_field(arguments.get_mut("workdir"));
    rewrite_permission_profile(arguments.get_mut("additional_permissions"));
    rewrite_permission_profile(arguments.get_mut("permissions"));
}

fn rewrite_permission_profile(value: Option<&mut Value>) {
    let Some(Value::Object(profile)) = value else {
        return;
    };
    rewrite_file_system_permissions(profile.get_mut("file_system"));
}

fn rewrite_file_system_permissions(value: Option<&mut Value>) {
    let Some(Value::Object(file_system)) = value else {
        return;
    };

    rewrite_string_array_field(file_system.get_mut("read"));
    rewrite_string_array_field(file_system.get_mut("write"));

    let Some(Value::Array(entries)) = file_system.get_mut("entries") else {
        return;
    };
    for entry in entries {
        rewrite_file_system_entry(entry);
    }
}

fn rewrite_file_system_entry(value: &mut Value) {
    let Value::Object(entry) = value else {
        return;
    };
    let Some(Value::Object(path)) = entry.get_mut("path") else {
        return;
    };

    match path.get("type").and_then(Value::as_str) {
        Some("path") => rewrite_string_field(path.get_mut("path")),
        Some("glob_pattern") => rewrite_string_field(path.get_mut("pattern")),
        Some("special") | None | Some(_) => {}
    }
}

fn rewrite_string_array_field(value: Option<&mut Value>) {
    let Some(Value::Array(paths)) = value else {
        return;
    };
    for path in paths {
        rewrite_string_field(Some(path));
    }
}

fn rewrite_string_field(value: Option<&mut Value>) {
    let Some(Value::String(path)) = value else {
        return;
    };
    if let Some(rewritten) = git_bash_path_to_windows_path(path) {
        *path = rewritten;
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn formats_windows_paths_for_git_bash() {
        assert_eq!(
            format_path_text_for_shell(r"C:\Users\Alice Smith\Desktop", PathDisplayStyle::GitBash),
            "/c/Users/Alice Smith/Desktop"
        );
        assert_eq!(
            format_path_text_for_shell("D:/Workspace/codex", PathDisplayStyle::GitBash),
            "/d/Workspace/codex"
        );
        assert_eq!(
            format_path_text_for_shell(r"relative\path", PathDisplayStyle::GitBash),
            "relative/path"
        );
    }

    #[test]
    fn parses_git_bash_drive_paths_for_windows() {
        assert_eq!(
            git_bash_path_to_windows_path("/c/Users/Alice Smith/Desktop").as_deref(),
            Some(r"C:\Users\Alice Smith\Desktop")
        );
        assert_eq!(git_bash_path_to_windows_path("/d").as_deref(), Some(r"D:\"));
        assert_eq!(
            git_bash_path_to_windows_path("//server/share/project").as_deref(),
            Some(r"\\server\share\project")
        );
        assert_eq!(git_bash_path_to_windows_path("/usr/bin"), None);
        assert_eq!(git_bash_path_to_windows_path("relative/path"), None);
    }

    #[test]
    fn rewrites_known_tool_path_arguments_only() -> anyhow::Result<()> {
        let arguments = json!({
            "cmd": "printf /c/Users/Alice",
            "workdir": "/c/Users/Alice/project",
            "additional_permissions": {
                "file_system": {
                    "read": ["/c/Users/Alice/read"],
                    "write": ["/d/work"],
                    "entries": [
                        {
                            "path": {
                                "type": "path",
                                "path": "/c/Users/Alice/entry"
                            },
                            "access": "read"
                        },
                        {
                            "path": {
                                "type": "glob_pattern",
                                "pattern": "/c/Users/Alice/**/*.env"
                            },
                            "access": "deny"
                        }
                    ]
                }
            }
        })
        .to_string();

        let rewritten: Value = serde_json::from_str(&rewrite_git_bash_path_arguments(&arguments)?)?;

        assert_eq!(
            rewritten,
            json!({
                "cmd": "printf /c/Users/Alice",
                "workdir": r"C:\Users\Alice\project",
                "additional_permissions": {
                    "file_system": {
                        "read": [r"C:\Users\Alice\read"],
                        "write": [r"D:\work"],
                        "entries": [
                            {
                                "path": {
                                    "type": "path",
                                    "path": r"C:\Users\Alice\entry"
                                },
                                "access": "read"
                            },
                            {
                                "path": {
                                    "type": "glob_pattern",
                                    "pattern": r"C:\Users\Alice\**\*.env"
                                },
                                "access": "deny"
                            }
                        ]
                    }
                }
            })
        );

        Ok(())
    }
}
