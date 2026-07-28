use pretty_assertions::assert_eq;

use super::git_bash_path_to_windows_path;

#[test]
fn converts_drive_and_unc_paths() {
    assert_eq!(
        git_bash_path_to_windows_path("/c/Users/Alice Smith/Desktop").as_deref(),
        Some(r"C:\Users\Alice Smith\Desktop")
    );
    assert_eq!(git_bash_path_to_windows_path("/d").as_deref(), Some(r"D:\"));
    assert_eq!(
        git_bash_path_to_windows_path("//server/share/project").as_deref(),
        Some(r"\\server\share\project")
    );
}

#[test]
fn leaves_non_windows_git_bash_paths_unresolved() {
    assert_eq!(git_bash_path_to_windows_path("/usr/bin"), None);
    assert_eq!(git_bash_path_to_windows_path("relative/path"), None);
    assert_eq!(git_bash_path_to_windows_path("//"), None);
}
