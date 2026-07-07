#!/usr/bin/env python3

from pathlib import Path
import re
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
REQUIRED_WINDOWS_BINARIES = {
    "codex",
    "codex-code-mode-host",
    "codex-command-runner",
    "codex-windows-sandbox-setup",
}


def workflow_text(name: str) -> str:
    return (REPO_ROOT / ".github" / "workflows" / name).read_text(encoding="utf-8")


def windows_binaries(text: str) -> set[str]:
    match = re.search(r'^\s*WINDOWS_BINARIES:\s*"([^"]+)"\s*$', text, re.MULTILINE)
    if match is None:
        raise AssertionError("missing WINDOWS_BINARIES workflow env")

    return set(match.group(1).split())


class ForkWindowsWorkflowTest(unittest.TestCase):
    def test_windows_build_workflows_include_required_runtime_binaries(self) -> None:
        for workflow_name in ["fork-release.yml", "fork-windows-build.yml"]:
            with self.subTest(workflow=workflow_name):
                self.assertTrue(
                    REQUIRED_WINDOWS_BINARIES.issubset(
                        windows_binaries(workflow_text(workflow_name))
                    )
                )

    def test_portable_release_uses_prebuilt_code_mode_host(self) -> None:
        text = workflow_text("fork-release.yml")
        self.assertIn("--code-mode-host-bin", text)
        self.assertIn("codex-code-mode-host.exe", text)


if __name__ == "__main__":
    unittest.main()
