#!/usr/bin/env python3
"""Create or update RigTether's initial GitHub labels, milestones, and issues."""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from collections import defaultdict
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]


def run(*args: str, capture: bool = True, check: bool = True) -> str:
    command = ["gh", *args]
    result = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=capture,
        check=False,
    )
    if check and result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise SystemExit(f"command failed: {' '.join(command)}\n{detail}")
    return result.stdout.strip()


def load_json(relative: str):
    return json.loads((ROOT / relative).read_text(encoding="utf-8"))


def milestone_numbers(repo: str) -> dict[str, int]:
    output = run("api", f"repos/{repo}/milestones?state=all&per_page=100", "--paginate")
    data = json.loads(output or "[]")
    return {item["title"]: item["number"] for item in data}


def ensure_milestones(repo: str, milestones: list[dict]) -> None:
    existing = milestone_numbers(repo)
    for milestone in milestones:
        title = milestone["title"]
        if title in existing:
            run(
                "api",
                "-X",
                "PATCH",
                f"repos/{repo}/milestones/{existing[title]}",
                "-f",
                f"title={title}",
                "-f",
                f"description={milestone['description']}",
            )
        else:
            run(
                "api",
                "-X",
                "POST",
                f"repos/{repo}/milestones",
                "-f",
                f"title={title}",
                "-f",
                f"description={milestone['description']}",
            )


def ensure_labels(repo: str, labels: list[dict]) -> None:
    for label in labels:
        run(
            "label",
            "create",
            label["name"],
            "--repo",
            repo,
            "--color",
            label["color"],
            "--description",
            label["description"],
            "--force",
        )


def existing_issues(repo: str) -> dict[str, int]:
    output = run(
        "issue",
        "list",
        "--repo",
        repo,
        "--state",
        "all",
        "--limit",
        "500",
        "--json",
        "number,title",
    )
    return {item["title"]: item["number"] for item in json.loads(output or "[]")}


def issue_number_from_url(url: str) -> int:
    return int(urlparse(url).path.rstrip("/").split("/")[-1])


def format_refs(ids: list[str], numbers: dict[str, int]) -> str:
    return ", ".join(f"#{numbers[item]}" for item in ids) if ids else "None"


def render_body(issue: dict, numbers: dict[str, int], reverse_deps: dict[str, list[str]]) -> str:
    body = (ROOT / issue["body_file"]).read_text(encoding="utf-8")
    parent = f"#{numbers[issue['parent']]}" if issue.get("parent") else "None"
    relationships = "\n".join(
        [
            "## Relationships",
            "",
            f"- Parent: {parent}",
            f"- Depends on: {format_refs(issue.get('depends_on', []), numbers)}",
            f"- Blocks: {format_refs(reverse_deps.get(issue['id'], []), numbers)}",
        ]
    )
    body = body.replace("{{RELATIONSHIPS}}", relationships)

    children = issue.get("children", [])
    if children:
        checklist = "\n".join(
            f"- [ ] #{numbers[child]} — {next(i['title'] for i in MANIFEST['issues'] if i['id'] == child)}"
            for child in children
        )
    else:
        checklist = "No focused child issues are created yet. Decompose this milestone only after its blocking evidence lands."
    return body.replace("{{CHILDREN}}", checklist)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", help="Override repository owner/name")
    args = parser.parse_args()

    global MANIFEST
    MANIFEST = load_json("planning/issues.json")
    repo = args.repo or MANIFEST["repository"]

    run("auth", "status", capture=True)
    run("repo", "view", repo, "--json", "nameWithOwner")

    repo_edit = [
        "repo",
        "edit",
        repo,
        "--description",
        MANIFEST["description"],
        "--enable-issues",
    ]
    for topic in MANIFEST.get("topics", []):
        repo_edit.extend(["--add-topic", topic])
    run(*repo_edit)

    ensure_labels(repo, load_json("planning/labels.json"))
    ensure_milestones(repo, load_json("planning/milestones.json"))

    by_title = existing_issues(repo)
    numbers: dict[str, int] = {}

    # Create every issue before rendering relationships so all numbers are known.
    for issue in MANIFEST["issues"]:
        title = issue["title"]
        number = by_title.get(title)
        if number is None:
            labels = ",".join(issue.get("labels", []))
            command = [
                "issue",
                "create",
                "--repo",
                repo,
                "--title",
                title,
                "--body",
                "Initial issue body is being rendered from the repository bootstrap manifest.",
                "--milestone",
                issue["milestone"],
            ]
            if labels:
                command.extend(["--label", labels])
            url = run(*command)
            number = issue_number_from_url(url)
            print(f"created #{number}: {title}")
        else:
            print(f"updating #{number}: {title}")
        numbers[issue["id"]] = number

    reverse_deps: dict[str, list[str]] = defaultdict(list)
    for issue in MANIFEST["issues"]:
        for dependency in issue.get("depends_on", []):
            reverse_deps[dependency].append(issue["id"])

    # Render exact relationships and update metadata idempotently.
    for issue in MANIFEST["issues"]:
        body = render_body(issue, numbers, reverse_deps)
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as temp:
            temp.write(body)
            temp_path = temp.name
        try:
            command = [
                "issue",
                "edit",
                str(numbers[issue["id"]]),
                "--repo",
                repo,
                "--body-file",
                temp_path,
                "--milestone",
                issue["milestone"],
            ]
            labels = issue.get("labels", [])
            if labels:
                command.extend(["--add-label", ",".join(labels)])
            run(*command)
        finally:
            Path(temp_path).unlink(missing_ok=True)

    print(f"Bootstrapped {len(MANIFEST['issues'])} issues in {repo}.")
    print(f"Executable queue: https://github.com/{repo}/issues?q=is%3Aissue+is%3Aopen+label%3Aagent-ready+-label%3Ain-progress")


if __name__ == "__main__":
    main()
