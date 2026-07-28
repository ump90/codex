/// Converts a Git Bash drive or UNC path to Windows-native spelling.
///
/// Returns `None` for relative paths and Git Bash virtual paths such as
/// `/usr/bin`, whose Windows location cannot be inferred from the path alone.
pub fn git_bash_path_to_windows_path(path: &str) -> Option<String> {
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
    let mut windows = format!(r"{drive}:\");
    if let Some(tail) = tail
        && !tail.is_empty()
    {
        windows.push_str(&tail.replace('/', "\\"));
    }
    Some(windows)
}

#[cfg(test)]
#[path = "git_bash_tests.rs"]
mod tests;
