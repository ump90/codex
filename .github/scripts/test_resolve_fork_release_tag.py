import tempfile
import unittest
from pathlib import Path

import resolve_fork_release_tag as resolver


class ResolveForkReleaseTagTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.repo = Path(self.temporary.name)
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Release Test")
        self.git("config", "user.email", "release@example.com")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "tag.gpgsign", "false")
        self.git("config", "core.autocrlf", "false")
        self.write("codex-rs/Cargo.toml", '[workspace.package]\nversion = "0.0.0"\n')
        self.base = self.commit("Initial main")

    def git(self, *args: str) -> str:
        return resolver.git(self.repo, *args)

    def write(self, name: str, text: str) -> None:
        path = self.repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def commit(self, message: str) -> str:
        self.git("add", ".")
        self.git("commit", "-qm", message)
        return self.git("rev-parse", "HEAD")

    def release(self, baseline: str, version: str, *, source_change=False) -> str:
        self.git("checkout", "--detach", baseline)
        self.write(
            "codex-rs/Cargo.toml", f'[workspace.package]\nversion = "{version}"\n'
        )
        if source_change:
            self.write("unmerged.rs", "unmerged release source")
        commit = self.commit(f"Release {version}")
        self.git("update-ref", f"{resolver.UPSTREAM_TAG_REFS}{version}", commit)
        self.git("checkout", "main")
        return commit

    def test_matches_version_only_release_commit_off_main(self) -> None:
        self.release(self.base, "0.162.0-alpha.3")
        self.write("fork.rs", "fork changes")
        self.commit("Fork fix")
        self.assertEqual(resolver.resolve_release_tag(self.repo), "0.162.0-alpha.3")

    def test_ignores_newer_release_whose_source_is_not_merged(self) -> None:
        self.release(self.base, "0.162.0-alpha.3")
        self.write("upstream.rs", "new main source")
        newer_base = self.commit("New upstream main")
        self.release(newer_base, "0.162.0-alpha.4")
        self.assertEqual(
            resolver.resolve_release_tag(self.repo, self.base), "0.162.0-alpha.3"
        )

    def test_prefers_closest_source_over_highest_version_number(self) -> None:
        self.release(self.base, "0.162.0-alpha.3")
        self.write("upstream.rs", "new main source")
        newer_base = self.commit("New upstream main")
        self.release(newer_base, "0.161.0")
        self.assertEqual(resolver.resolve_release_tag(self.repo), "0.161.0")

    def test_handles_merge_and_annotated_upstream_tag(self) -> None:
        self.git("checkout", "-b", "fork")
        self.write("fork.rs", "fork changes")
        self.commit("Fork changes")
        self.git("checkout", "main")
        self.write("upstream.rs", "upstream source")
        baseline = self.commit("Upstream source")
        release = self.release(baseline, "0.162.0-alpha.3")
        self.git("tag", "-a", "rust-v0.162.0-alpha.3", release, "-m", "Release")
        tag_object = self.git("rev-parse", "refs/tags/rust-v0.162.0-alpha.3")
        self.git(
            "update-ref", f"{resolver.UPSTREAM_TAG_REFS}0.162.0-alpha.3", tag_object
        )
        self.git("checkout", "fork")
        self.git("merge", "--no-ff", "main", "-m", "Merge main")
        self.assertEqual(resolver.resolve_release_tag(self.repo), "0.162.0-alpha.3")

    def test_accepts_release_commit_itself_as_ref(self) -> None:
        release = self.release(self.base, "0.162.0-alpha.3")
        self.assertEqual(
            resolver.resolve_release_tag(self.repo, release), "0.162.0-alpha.3"
        )

    def test_rejects_release_with_unmerged_source_changes(self) -> None:
        self.release(self.base, "0.162.0-alpha.3", source_change=True)
        with self.assertRaisesRegex(ValueError, "No upstream release matches"):
            resolver.resolve_release_tag(self.repo)

    def test_rejects_other_manifest_changes_in_release_commit(self) -> None:
        self.git("checkout", "--detach", self.base)
        self.write(
            "codex-rs/Cargo.toml",
            '[workspace.package]\nversion = "0.162.0-alpha.3"\nedition = "2024"\n',
        )
        release = self.commit("Release with manifest change")
        self.git("update-ref", f"{resolver.UPSTREAM_TAG_REFS}0.162.0-alpha.3", release)
        self.git("checkout", "main")
        with self.assertRaisesRegex(ValueError, "No upstream release matches"):
            resolver.resolve_release_tag(self.repo)

    def test_does_not_use_fork_tags(self) -> None:
        self.git("tag", "0.999.0", self.base)
        with self.assertRaisesRegex(ValueError, "No upstream release tags found"):
            resolver.resolve_release_tag(self.repo)


if __name__ == "__main__":
    unittest.main()
