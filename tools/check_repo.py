#!/usr/bin/env python3
"""Validate RigTether repository structure and initial planning metadata."""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []


def error(message: str) -> None:
    ERRORS.append(message)


def load_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        error(f"{path.relative_to(ROOT)}: {exc}")
        return None


required_paths = [
    "README.md",
    "AGENTS.md",
    "LICENSE",
    "docs/product.md",
    "docs/architecture.md",
    "docs/hardware-safety.md",
    "planning/labels.json",
    "planning/milestones.json",
    "planning/issues.json",
]
for relative in required_paths:
    if not (ROOT / relative).is_file():
        error(f"missing required file: {relative}")

labels_data = load_json(ROOT / "planning/labels.json") or []
milestones_data = load_json(ROOT / "planning/milestones.json") or []
manifest = load_json(ROOT / "planning/issues.json") or {}
issues = manifest.get("issues", [])

label_names = {item.get("name") for item in labels_data if isinstance(item, dict)}
# GitHub's default labels are allowed even though bootstrap does not redefine them.
label_names.update({"enhancement", "bug", "documentation", "good first issue", "help wanted", "question", "duplicate", "invalid", "wontfix"})
milestone_names = {item.get("title") for item in milestones_data if isinstance(item, dict)}

ids: set[str] = set()
titles: set[str] = set()
by_id: dict[str, dict] = {}
for issue in issues:
    issue_id = issue.get("id")
    title = issue.get("title")
    if not issue_id or issue_id in ids:
        error(f"duplicate or missing issue id: {issue_id!r}")
    if not title or title in titles:
        error(f"duplicate or missing issue title: {title!r}")
    ids.add(issue_id)
    titles.add(title)
    by_id[issue_id] = issue

for issue in issues:
    issue_id = issue["id"]
    body_path = ROOT / issue.get("body_file", "")
    if not body_path.is_file():
        error(f"{issue_id}: missing body file {issue.get('body_file')}")
        continue
    body = body_path.read_text(encoding="utf-8")
    required_headings = ["## Outcome", "## Completion evidence", "{{RELATIONSHIPS}}"]
    if issue.get("kind") == "tracking":
        required_headings.extend(["## Child work", "## Exit criteria", "{{CHILDREN}}"])
    else:
        required_headings.extend(["## Context", "## Scope", "## Non-goals", "## Acceptance criteria"])
    for heading in required_headings:
        if heading not in body:
            error(f"{issue_id}: body missing {heading!r}")

    milestone = issue.get("milestone")
    if milestone not in milestone_names:
        error(f"{issue_id}: unknown milestone {milestone!r}")

    for label in issue.get("labels", []):
        if label not in label_names:
            error(f"{issue_id}: unknown label {label!r}")

    deps = issue.get("depends_on", [])
    for dep in deps:
        if dep not in by_id:
            error(f"{issue_id}: unknown dependency {dep!r}")
        if dep == issue_id:
            error(f"{issue_id}: depends on itself")

    parent = issue.get("parent")
    if parent is not None:
        if parent not in by_id:
            error(f"{issue_id}: unknown parent {parent!r}")
        elif issue_id not in by_id[parent].get("children", []):
            error(f"{issue_id}: parent {parent!r} does not list child")

    for child in issue.get("children", []):
        if child not in by_id:
            error(f"{issue_id}: unknown child {child!r}")
        elif by_id[child].get("parent") != issue_id:
            error(f"{issue_id}: child {child!r} does not point back to parent")

    if "agent-ready" in issue.get("labels", []) and deps:
        error(f"{issue_id}: agent-ready issue still has dependencies")
    if "agent-ready" in issue.get("labels", []) and "human-required" in issue.get("labels", []):
        error(f"{issue_id}: cannot be both agent-ready and human-required")

# Cycle detection.
visiting: set[str] = set()
visited: set[str] = set()


def visit(issue_id: str, chain: list[str]) -> None:
    if issue_id in visited:
        return
    if issue_id in visiting:
        error("dependency cycle: " + " -> ".join(chain + [issue_id]))
        return
    visiting.add(issue_id)
    for dep in by_id[issue_id].get("depends_on", []):
        if dep in by_id:
            visit(dep, chain + [issue_id])
    visiting.remove(issue_id)
    visited.add(issue_id)


for issue_id in by_id:
    visit(issue_id, [])

# Basic text hygiene for project-authored source. License texts are excluded.
text_suffixes = {
    ".c",
    ".conf",
    ".h",
    ".md",
    ".py",
    ".json",
    ".yml",
    ".yaml",
    ".sh",
    ".toml",
}
generated_parts = {".git", ".jj", "node_modules", "target"}
for path in ROOT.rglob("*"):
    if (
        not path.is_file()
        or "LICENSES" in path.parts
        or generated_parts.intersection(path.parts)
    ):
        continue
    if path.suffix not in text_suffixes and path.name not in {"LICENSE", "NOTICE", "CODEOWNERS", "check", "ci", "publish", "bootstrap-github"}:
        continue
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    if not text.endswith("\n"):
        error(f"{path.relative_to(ROOT)}: missing final newline")
    for number, line in enumerate(text.splitlines(), 1):
        if "\t" in line and path.suffix not in {".c", ".h"}:
            error(f"{path.relative_to(ROOT)}:{number}: tab character")
        if line.rstrip(" ") != line and path.suffix != ".md":
            error(f"{path.relative_to(ROOT)}:{number}: trailing whitespace")

if ERRORS:
    print("Repository validation failed:", file=sys.stderr)
    for item in ERRORS:
        print(f"- {item}", file=sys.stderr)
    raise SystemExit(1)

print(f"Validated {len(issues)} initial issues, {len(milestones_data)} milestones, and {len(labels_data)} custom labels.")
