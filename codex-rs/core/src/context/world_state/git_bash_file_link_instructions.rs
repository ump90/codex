use super::PreviousSectionState;
use super::WorldStateSection;
use crate::context::ContextualUserFragment;
use crate::context::GitBashFileLinkInstructions;

/// Whether Windows Git Bash file-link guidance should be visible to the model.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GitBashFileLinkInstructionsState {
    enabled: bool,
}

impl GitBashFileLinkInstructionsState {
    pub(crate) fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl WorldStateSection for GitBashFileLinkInstructionsState {
    const ID: &'static str = "git_bash_file_link_instructions";
    type Snapshot = bool;

    fn snapshot(&self) -> Self::Snapshot {
        self.enabled
    }

    fn matches_legacy_fragment(role: &str, text: &str) -> bool {
        role == "developer" && GitBashFileLinkInstructions::matches_text(text)
    }

    fn has_retained_fragment_matcher() -> bool {
        true
    }

    fn matches_retained_fragment(role: &str, text: &str) -> bool {
        Self::matches_legacy_fragment(role, text)
    }

    fn render_diff(
        &self,
        previous: PreviousSectionState<'_, Self::Snapshot>,
    ) -> Option<Box<dyn ContextualUserFragment>> {
        if !self.enabled
            || matches!(previous, PreviousSectionState::Known(previous) if *previous)
            || matches!(previous, PreviousSectionState::Unknown)
        {
            return None;
        }

        Some(Box::new(GitBashFileLinkInstructions))
    }
}

#[cfg(test)]
#[path = "git_bash_file_link_instructions_tests.rs"]
mod tests;
