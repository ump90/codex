import tempfile
import unittest
from pathlib import Path

import sync_upstream_release as sync


class LatestReleaseTest(unittest.TestCase):
    def release(self, tag, published, **fields):
        return dict(id=1, tag_name=tag, published_at=published, draft=False, **fields)

    def test_selects_latest_publication_including_prerelease_across_pages(self):
        stable = self.release("rust-v0.160.0", "2026-10-01T20:19:00Z")
        alpha = self.release(
            "rust-v0.162.0-alpha.4", "2026-10-02T05:44:00Z", prerelease=True
        )
        unrelated = self.release("rusty-v8-v152.2.0", "2026-10-03T00:00:00Z")
        draft = {**alpha, "draft": True, "published_at": "2026-10-04T00:00:00Z"}
        self.assertEqual(
            sync.latest_release([[unrelated, stable, draft], [alpha]]), alpha
        )

    def test_uses_publication_time_instead_of_version_number(self):
        newer = self.release("rust-v0.160.1", "2026-10-03T00:00:00Z")
        older = self.release("rust-v0.162.0-alpha.4", "2026-10-02T00:00:00Z")
        self.assertEqual(sync.latest_release([[older, newer]]), newer)

    def test_rejects_empty_or_unpublished_releases(self):
        with self.assertRaisesRegex(ValueError, "No published"):
            sync.latest_release([[self.release("rust-v0.162.0", None)]])

    def test_rejects_partial_api_error_response(self):
        with self.assertRaisesRegex(ValueError, "Incomplete"):
            sync.latest_release(
                [[self.release("rust-v0.162.0", "2026-10-02")], {"message": "timeout"}]
            )


class MergeReleaseTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.repo = Path(temporary.name)
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Sync Test")
        self.git("config", "user.email", "sync@example.com")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.autocrlf", "false")
        self.manifest = self.repo / sync.MANIFEST
        self.manifest.parent.mkdir(parents=True)
        self.write_version("0.0.0")
        self.commit("Initial source")
        self.git("branch", "release")

    def git(self, *args):
        return sync.git(self.repo, *args)

    def write_version(self, version, extra=""):
        self.manifest.write_text(
            f'[workspace.package]\nversion = "{version}"\n{extra}', encoding="utf-8"
        )

    def commit(self, message):
        self.git("add", ".")
        self.git("commit", "-qm", message)
        return self.git("rev-parse", "HEAD")

    def test_merges_divergent_version_stamps_and_preserves_upstream_changes(self):
        self.write_version("0.162.0-alpha.3")
        old = self.commit("Old release")
        self.git("checkout", "release")
        self.write_version("0.162.0-alpha.4", 'edition = "2024"\n')
        new = self.commit("New release")
        self.git("checkout", "main")
        sync.merge_release(self.repo, new)
        self.assertEqual(self.git("rev-parse", "HEAD^1"), old)
        self.assertEqual(self.git("rev-parse", "HEAD^2"), new)
        self.assertEqual(
            self.manifest.read_text(encoding="utf-8"),
            '[workspace.package]\nversion = "0.162.0-alpha.4"\nedition = "2024"\n',
        )
        head = self.git("rev-parse", "HEAD")
        sync.merge_release(self.repo, new)
        self.assertEqual(self.git("rev-parse", "HEAD"), head)

    def test_refuses_to_overwrite_local_manifest_changes(self):
        self.write_version("0.162.0-alpha.3", 'edition = "2021"\n')
        self.commit("Local manifest change")
        self.git("checkout", "release")
        self.write_version("0.162.0-alpha.4", 'edition = "2024"\n')
        new = self.commit("New release")
        self.git("checkout", "main")
        with self.assertRaisesRegex(ValueError, "Local manifest changes"):
            sync.merge_release(self.repo, new)

    def test_refuses_source_conflicts(self):
        source = self.repo / "source.rs"
        source.write_text("base\n", encoding="utf-8")
        base = self.commit("Shared source")
        self.git("branch", "source-release", base)
        source.write_text("local\n", encoding="utf-8")
        self.commit("Local source")
        self.git("checkout", "source-release")
        source.write_text("upstream\n", encoding="utf-8")
        new = self.commit("Upstream source")
        self.git("checkout", "main")
        with self.assertRaisesRegex(ValueError, "source conflicts"):
            sync.merge_release(self.repo, new)


if __name__ == "__main__":
    unittest.main()
