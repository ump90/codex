#!/usr/bin/env python3
"""Find the upstream release whose source baseline is closest to a fork ref."""

import argparse
from collections import deque
from pathlib import Path
import re
import subprocess


UPSTREAM_TAG_REFS = "refs/codex-upstream-release-tags/"
VERSION_PATTERN = r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?"
WORKSPACE_VERSION = re.compile(r'(\[workspace\.package\]\s*\nversion\s*=\s*")[^"]+(")')


def git(repo: Path, *args: str, input_text: str | None = None) -> str:
    return subprocess.check_output(
        ["git", "-C", str(repo), *args], input=input_text, text=True, encoding="utf-8"
    ).strip()


def resolve_release_tag(repo: Path, ref: str = "HEAD") -> str:
    history = {}
    for line in git(repo, "rev-list", "--parents", ref).splitlines():
        commit, *parents = line.split()
        history[commit] = parents
    target = git(repo, "rev-parse", f"{ref}^{{commit}}")
    distances = {target: 0}
    pending = deque([target])
    while pending:
        commit = pending.popleft()
        for parent in history[commit]:
            if parent not in distances:
                distances[parent] = distances[commit] + 1
                pending.append(parent)

    tags = {}
    for line in git(
        repo,
        "for-each-ref",
        "--format=%(refname) %(objectname) %(*objectname)",
        UPSTREAM_TAG_REFS,
    ).splitlines():
        tag_ref, object_id, *peeled = line.split()
        version = tag_ref.removeprefix(UPSTREAM_TAG_REFS)
        if re.fullmatch(VERSION_PATTERN, version):
            tags.setdefault(peeled[0] if peeled else object_id, []).append(version)
    if not tags:
        raise ValueError("No upstream release tags found; fetch upstream tags first")

    candidates = []
    for line in git(
        repo,
        "log",
        "--no-walk",
        "--format=%H %ct %P",
        "--stdin",
        input_text="\n".join(tags) + "\n",
    ).splitlines():
        commit, timestamp, *parents = line.split()
        if commit in distances:
            baseline = commit
        elif len(parents) == 1 and parents[0] in distances:
            baseline = parents[0]
        else:
            continue
        candidates.append((distances[baseline], -int(timestamp), commit, baseline))

    for _, _, commit, baseline in sorted(candidates):
        if commit != baseline:
            # Release commits off main are accepted only when they stamp the
            # workspace version without introducing unmerged source changes.
            changed = git(repo, "diff", "--name-only", baseline, commit)
            if changed != "codex-rs/Cargo.toml":
                continue
            source = git(repo, "show", f"{baseline}:codex-rs/Cargo.toml")
            released = git(repo, "show", f"{commit}:codex-rs/Cargo.toml")
            if WORKSPACE_VERSION.sub(r"\1VERSION\2", source) != WORKSPACE_VERSION.sub(
                r"\1VERSION\2", released
            ):
                continue
        versions = tags[commit]
        if len(versions) != 1:
            raise ValueError(f"Ambiguous upstream versions at {commit}: {versions}")
        return versions[0]
    raise ValueError("No upstream release matches this ref; set release_tag manually")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--ref", default="HEAD")
    parser.add_argument("--fetch", action="store_true")
    args = parser.parse_args()
    if args.fetch:
        git(
            args.repo,
            "fetch",
            "--no-tags",
            "--prune",
            "https://github.com/openai/codex.git",
            f"+refs/tags/rust-v*:{UPSTREAM_TAG_REFS}*",
        )
    try:
        print(resolve_release_tag(args.repo, args.ref))
    except ValueError as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
