#!/usr/bin/env python3
"""Merge the most recently published upstream Codex release, including prereleases."""

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import time


RELEASE_TAG = re.compile(
    r"rust-v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?"
)
WORKSPACE_VERSION = re.compile(r'(?m)^(version\s*=\s*")[^"]+("\s*)$')
MANIFEST = "codex-rs/Cargo.toml"


def release_pages(repository: str) -> list[list[dict]]:
    command = [
        "gh", "api", "--paginate", "--slurp",
        f"repos/{repository}/releases?per_page=100",
    ]
    for attempt in range(4):
        try:
            # Never parse partial stdout from a failed paginated request.
            output = subprocess.check_output(
                command, text=True, encoding="utf-8", stderr=subprocess.PIPE,
                timeout=300,
            )
            return json.loads(output)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
            if isinstance(error, subprocess.CalledProcessError):
                detail = error.stderr or ""
                print(detail, file=sys.stderr, flush=True)
                if not re.search(
                    r"HTTP (?:429|5[0-9]{2})\b|TLS handshake timeout|"
                    r"i/o timeout|connection reset|unexpected EOF|"
                    r"temporary failure in name resolution",
                    detail, re.IGNORECASE,
                ):
                    raise
            if attempt == 3:
                raise
            delay = 5 * 2**attempt
            print(
                f"GitHub release query failed (attempt {attempt + 1}/4); "
                f"retrying in {delay}s",
                file=sys.stderr, flush=True,
            )
            time.sleep(delay)


def latest_release(pages: list[list[dict]]) -> dict:
    if any(not isinstance(page, list) for page in pages):
        raise ValueError("Incomplete upstream release response; retry the sync")
    releases = [
        release
        for page in pages
        for release in page
        if not release["draft"]
        and release.get("published_at")
        and RELEASE_TAG.fullmatch(release["tag_name"])
    ]
    if not releases:
        raise ValueError("No published upstream Codex releases found")
    return max(releases, key=lambda release: (release["published_at"], release["id"]))


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(repo), *args], text=True, encoding="utf-8"
    ).strip()


def without_workspace_version(text: str) -> str:
    before, section = text.split("[workspace.package]", 1)
    package, *following = section.split("\n[", 1)
    package, count = WORKSPACE_VERSION.subn(r"\1VERSION\2", package)
    if count != 1:
        raise ValueError("Expected exactly one workspace package version")
    return (
        before
        + "[workspace.package]"
        + package
        + ("\n[" + following[0] if following else "")
    )


def merge_release(repo: Path, revision: str) -> None:
    result = subprocess.run(["git", "-C", str(repo), "merge", "--no-edit", revision])
    if result.returncode == 0:
        return
    conflicts = git(repo, "diff", "--name-only", "--diff-filter=U").splitlines()
    if conflicts != [MANIFEST]:
        raise ValueError(
            "Upstream release has source conflicts; manual resolution required"
        )
    base = git(repo, "show", f":1:{MANIFEST}")
    ours = git(repo, "show", f":2:{MANIFEST}")
    theirs = git(repo, "show", f":3:{MANIFEST}")
    # Only discard our old release stamp when it is the sole local change to
    # this manifest. All new upstream manifest changes are then preserved.
    if without_workspace_version(base) != without_workspace_version(ours):
        raise ValueError("Local manifest changes require manual conflict resolution")
    without_workspace_version(theirs)
    (repo / MANIFEST).write_text(theirs + "\n", encoding="utf-8")
    git(repo, "add", MANIFEST)
    git(repo, "commit", "--no-edit")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", default="openai/codex")
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repository):
        parser.error("Expected an owner/repository name")
    pages = release_pages(args.repository)
    release = latest_release(pages)
    tag = release["tag_name"]
    print(
        f"Syncing {tag}: {release['html_url']} (published {release['published_at']})",
        flush=True,
    )
    git(
        args.repo,
        "fetch",
        "--no-tags",
        f"https://github.com/{args.repository}.git",
        f"refs/tags/{tag}",
    )
    revision = git(args.repo, "rev-parse", "FETCH_HEAD^{commit}")
    merge_release(args.repo, revision)


if __name__ == "__main__":
    main()
