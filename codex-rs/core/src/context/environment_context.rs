use codex_protocol::models::ManagedFileSystemPermissions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::permissions::FileSystemAccessMode;
use codex_protocol::permissions::FileSystemPath;
use codex_protocol::permissions::FileSystemSandboxEntry;
use codex_protocol::permissions::FileSystemSpecialPath;
use codex_utils_path_uri::PathUri;
use std::collections::HashSet;

use crate::git_bash_paths::PathDisplayStyle;
use crate::git_bash_paths::format_native_path_for_shell;
use crate::git_bash_paths::format_path_text_for_shell;
use crate::git_bash_paths::format_path_uri_for_shell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSystemContext {
    workspace_roots: Vec<String>,
    permission_profile: FileSystemPermissionProfileContext,
    path_display_style: PathDisplayStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FileSystemPermissionProfileContext {
    Managed(ManagedFileSystemContext),
    Disabled,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ManagedFileSystemContext {
    Restricted {
        entries: Vec<FileSystemSandboxEntry>,
        glob_scan_max_depth: Option<usize>,
    },
    Unrestricted,
}

impl FileSystemContext {
    pub(super) fn from_permission_profile(
        permission_profile: &PermissionProfile,
        workspace_roots: &[PathUri],
        path_display_style: PathDisplayStyle,
    ) -> Self {
        let materialized_workspace_roots = workspace_roots
            .iter()
            .filter_map(|workspace_root| workspace_root.to_abs_path().ok())
            .collect::<Vec<_>>();
        let permission_profile = permission_profile
            .clone()
            .materialize_project_roots_with_workspace_roots(&materialized_workspace_roots);
        let workspace_roots = workspace_roots
            .iter()
            .map(|root| format_path_uri_for_shell(root, path_display_style))
            .collect();
        let permission_profile = match permission_profile {
            PermissionProfile::Managed { file_system, .. } => {
                FileSystemPermissionProfileContext::Managed(ManagedFileSystemContext::from(
                    file_system,
                ))
            }
            PermissionProfile::Disabled => FileSystemPermissionProfileContext::Disabled,
            PermissionProfile::External { .. } => FileSystemPermissionProfileContext::External,
        };
        Self {
            workspace_roots,
            permission_profile,
            path_display_style,
        }
    }

    pub(super) fn render(&self) -> String {
        let mut rendered = "<filesystem>".to_string();
        if !self.workspace_roots.is_empty() {
            rendered.push_str("<workspace_roots>");
            for root in &self.workspace_roots {
                push_text_element(&mut rendered, "root", root);
            }
            rendered.push_str("</workspace_roots>");
        }
        self.permission_profile
            .render(&mut rendered, self.path_display_style);
        rendered.push_str("</filesystem>");
        rendered
    }
}

impl From<ManagedFileSystemPermissions> for ManagedFileSystemContext {
    fn from(file_system: ManagedFileSystemPermissions) -> Self {
        match file_system {
            ManagedFileSystemPermissions::Restricted {
                mut entries,
                glob_scan_max_depth,
            } => {
                dedupe_file_system_entries(&mut entries);
                Self::Restricted {
                    entries,
                    glob_scan_max_depth: glob_scan_max_depth.map(usize::from),
                }
            }
            ManagedFileSystemPermissions::Unrestricted => Self::Unrestricted,
        }
    }
}

impl FileSystemPermissionProfileContext {
    fn render(&self, rendered: &mut String, path_display_style: PathDisplayStyle) {
        match self {
            Self::Managed(file_system) => {
                rendered.push_str("<permission_profile type=\"managed\">");
                file_system.render(rendered, path_display_style);
                rendered.push_str("</permission_profile>");
            }
            Self::Disabled => {
                rendered.push_str(
                    "<permission_profile type=\"disabled\"><file_system type=\"unrestricted\" /></permission_profile>",
                );
            }
            Self::External => {
                rendered.push_str(
                    "<permission_profile type=\"external\"><file_system type=\"external\" /></permission_profile>",
                );
            }
        }
    }
}

impl ManagedFileSystemContext {
    fn render(&self, rendered: &mut String, path_display_style: PathDisplayStyle) {
        match self {
            Self::Restricted {
                entries,
                glob_scan_max_depth,
            } => {
                if entries.is_empty() && glob_scan_max_depth.is_none() {
                    rendered.push_str("<file_system type=\"restricted\" />");
                    return;
                }

                rendered.push_str("<file_system type=\"restricted\"");
                if let Some(glob_scan_max_depth) = glob_scan_max_depth {
                    rendered.push_str(&format!(" glob_scan_max_depth=\"{glob_scan_max_depth}\""));
                }
                rendered.push('>');
                for entry in entries {
                    render_file_system_entry(rendered, entry, path_display_style);
                }
                rendered.push_str("</file_system>");
            }
            Self::Unrestricted => {
                rendered.push_str("<file_system type=\"unrestricted\" />");
            }
        }
    }
}

fn render_file_system_entry(
    rendered: &mut String,
    entry: &FileSystemSandboxEntry,
    path_display_style: PathDisplayStyle,
) {
    rendered.push_str("<entry access=\"");
    let access = entry.access.to_string();
    rendered.push_str(&access);
    if entry.access == FileSystemAccessMode::Deny {
        rendered.push_str("\" escalatable=\"false");
    }
    rendered.push_str("\">");
    match &entry.path {
        FileSystemPath::Path { path } => {
            let path = format_native_path_for_shell(path.as_path(), path_display_style);
            push_text_element(rendered, "path", &path);
        }
        FileSystemPath::GlobPattern { pattern } => {
            let pattern = format_path_text_for_shell(pattern, path_display_style);
            push_text_element(rendered, "glob", &pattern);
        }
        FileSystemPath::Special { value } => {
            let value = render_special_path(value, path_display_style);
            push_text_element(rendered, "special", &value);
        }
    }
    rendered.push_str("</entry>");
}

fn render_special_path(
    value: &FileSystemSpecialPath,
    path_display_style: PathDisplayStyle,
) -> String {
    match value {
        FileSystemSpecialPath::Root => ":root".to_string(),
        FileSystemSpecialPath::Minimal => ":minimal".to_string(),
        FileSystemSpecialPath::ProjectRoots { subpath } => {
            render_special_path_with_subpath(":workspace_roots", subpath, path_display_style)
        }
        FileSystemSpecialPath::Tmpdir => ":tmpdir".to_string(),
        FileSystemSpecialPath::SlashTmp => ":slash_tmp".to_string(),
        FileSystemSpecialPath::Unknown { path, subpath } => {
            render_special_path_with_subpath(path, subpath, path_display_style)
        }
    }
}

fn render_special_path_with_subpath(
    base: &str,
    subpath: &Option<String>,
    path_display_style: PathDisplayStyle,
) -> String {
    match subpath {
        Some(subpath) => {
            let subpath = format_path_text_for_shell(subpath, path_display_style);
            format!("{base}/{subpath}")
        }
        None => base.to_string(),
    }
}

fn dedupe_file_system_entries(entries: &mut Vec<FileSystemSandboxEntry>) {
    let mut seen = HashSet::new();
    entries.retain(|entry| seen.insert(entry.clone()));
}

fn push_text_element(rendered: &mut String, name: &str, value: &str) {
    rendered.push_str(&format!("<{name}>"));
    push_xml_escaped_text(rendered, value);
    rendered.push_str(&format!("</{name}>"));
}

pub(crate) fn push_xml_escaped_text(rendered: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => rendered.push_str("&amp;"),
            '<' => rendered.push_str("&lt;"),
            '>' => rendered.push_str("&gt;"),
            '"' => rendered.push_str("&quot;"),
            '\'' => rendered.push_str("&apos;"),
            _ => rendered.push(ch),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NetworkContext {
    enabled: bool,
    allowed_domains: Vec<String>,
    denied_domains: Vec<String>,
}

impl NetworkContext {
    pub(crate) fn new(enabled: bool, allowed: Vec<String>, denied: Vec<String>) -> Self {
        Self {
            enabled,
            allowed_domains: allowed,
            denied_domains: denied,
        }
    }

    pub(super) fn render(&self) -> String {
        let mut rendered = format!("<network enabled=\"{}\">", self.enabled);
        if self.enabled {
            Self::push_rendered_domain_element(&mut rendered, "allowed", &self.allowed_domains);
            Self::push_rendered_domain_element(&mut rendered, "denied", &self.denied_domains);
        }
        rendered.push_str("</network>");
        rendered
    }

    fn push_rendered_domain_element(rendered_network: &mut String, name: &str, domains: &[String]) {
        if domains.is_empty() {
            return;
        }

        rendered_network.push_str(&format!("<{name}>"));
        rendered_network.push_str(&domains.join(","));
        rendered_network.push_str(&format!("</{name}>"));
    }
}
