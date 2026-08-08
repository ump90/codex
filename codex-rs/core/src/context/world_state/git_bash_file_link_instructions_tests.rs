use super::*;
use crate::context::ContextualUserFragment;
use crate::context::world_state::test_support::render_section_cases;
use codex_protocol::models::ResponseItem;

#[test]
fn snapshots() {
    use PreviousSectionState::Absent;
    use PreviousSectionState::Known;
    use PreviousSectionState::Unknown;

    let disabled = GitBashFileLinkInstructionsState::new(/*enabled*/ false);
    let enabled = GitBashFileLinkInstructionsState::new(/*enabled*/ true);

    insta::assert_snapshot!(render_section_cases(&[
        (Absent, Absent),
        (Absent, Known(&disabled)),
        (Absent, Known(&enabled)),
        (Known(&disabled), Known(&enabled)),
        (Known(&enabled), Known(&enabled)),
        (Known(&enabled), Known(&disabled)),
        (Unknown, Known(&disabled)),
        (Unknown, Known(&enabled)),
    ]));
}

#[test]
fn retained_guidance_is_not_injected_again() {
    let mut world_state = super::super::WorldState::default();
    world_state.add_section(GitBashFileLinkInstructionsState::new(/*enabled*/ true));
    let retained: ResponseItem = ContextualUserFragment::into(GitBashFileLinkInstructions);

    assert!(
        world_state
            .render_history_diff(/*previous*/ None, &[retained])
            .is_empty()
    );
}
