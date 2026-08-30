use super::ContextualUserFragment;
use codex_protocol::models::ContentItemKind;

pub(crate) struct GitBashFileLinkInstructions;

impl ContextualUserFragment for GitBashFileLinkInstructions {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("git_bash_file_link_instructions".to_string())
    }

    fn role(&self) -> &'static str {
        "developer"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        (
            "<git_bash_file_link_instructions>",
            "</git_bash_file_link_instructions>",
        )
    }

    fn body(&self) -> String {
        "\n## Windows Git Bash file links\n\
Use Git Bash paths such as `/c/...` in commands and structured tool path arguments. For Markdown links to local files in your response, use native Windows absolute paths with forward slashes, such as `C:/...`, in the link destination. Codex App opens local file links using native Windows paths, so do not use Git Bash `/c/...` paths in Markdown link destinations. A Git Bash path `/c/work/file.rs` maps to `C:/work/file.rs`.\n"
            .to_string()
    }
}
