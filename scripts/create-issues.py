#!/usr/bin/env python3
"""create-issues: create DeckStreak's epics, feature and owner issues from a plan, idempotently.

    python3 scripts/create-issues.py --plan PLAN.json [--repo OWNER/NAME] [--dry-run]

The plan is a JSON list of issues, each `{"key", "title", "body", "labels": [...],
"milestone": "<title>" | null, "parent": "<epic key>" | null}`. `docs/issues-manifest.json`
maps each key to its issue `number` and numeric `id`, and it is written after EVERY create, so a
re-run after a failure creates nothing twice. Children are linked to their epic as sub-issues
(`POST /repos/{repo}/issues/{epic}/sub_issues` with the child's numeric id), and a link that
already exists is left alone.

Creates are paced (about two seconds apart) because GitHub's secondary rate limits refuse bursts
of content creation. Before anything is posted, every title and body passes the public scrub
(`scripts/public-scrub.py`, with the private deny list when `$PERSONA_CORE_DENY_LIST` names one):
an issue in a public repository is published text.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
MANIFEST = REPO_ROOT / "docs" / "issues-manifest.json"
PACE_SECONDS = 2.2


def gh(args: list[str], stdin: str | None = None) -> str:
    done = subprocess.run(["gh", *args], input=stdin, capture_output=True, text=True, check=False)
    if done.returncode != 0:
        raise RuntimeError(f"gh {' '.join(args[:3])} failed: {done.stderr.strip()[:300]}")
    return done.stdout


def load_manifest() -> dict:
    if MANIFEST.is_file():
        return json.loads(MANIFEST.read_text(encoding="utf-8"))
    return {"schema": "deckstreak.issues-manifest.v1", "issues": {}}


def save_manifest(manifest: dict) -> None:
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    ordered = dict(sorted(manifest["issues"].items(), key=lambda kv: kv[1]["number"]))
    manifest["issues"] = ordered
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def scrub(plan: list[dict]) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        for index, issue in enumerate(plan):
            Path(tmp, f"{index:04d}-{issue['key']}.md").write_text(
                f"{issue['title']}\n\n{issue['body']}\n", encoding="utf-8"
            )
        done = subprocess.run(
            [
                sys.executable,
                str(REPO_ROOT / "scripts" / "public-scrub.py"),
                "--root",
                str(REPO_ROOT),
                "--no-tree",
                "--subject",
                tmp,
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        print(done.stdout.strip().splitlines()[-1] if done.stdout.strip() else done.stderr)
        if done.returncode != 0:
            print(done.stdout)
            raise SystemExit("create-issues: the public scrub refused the plan; nothing was posted")


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def update_bodies(repo: str, plan: list[dict], manifest: dict) -> int:
    """Re-post each created issue whose planned body changed, once; the manifest records why not."""
    updated = 0
    for issue in plan:
        entry = manifest["issues"].get(issue["key"])
        if entry is None or entry.get("body_sha256") == digest(issue["body"]):
            continue
        gh(
            ["api", "-X", "PATCH", f"repos/{repo}/issues/{entry['number']}", "--input", "-"],
            stdin=json.dumps({"body": issue["body"]}),
        )
        entry["body_sha256"] = digest(issue["body"])
        save_manifest(manifest)
        updated += 1
        time.sleep(PACE_SECONDS)
    print(f"updated {updated} body(ies)")
    return 0


def milestones(repo: str) -> dict[str, int]:
    found = json.loads(gh(["api", f"repos/{repo}/milestones?state=all&per_page=100"]))
    return {m["title"]: m["number"] for m in found}


def create(repo: str, issue: dict, milestone_numbers: dict[str, int]) -> tuple[int, int]:
    payload: dict = {"title": issue["title"], "body": issue["body"], "labels": issue["labels"]}
    if issue.get("milestone"):
        payload["milestone"] = milestone_numbers[issue["milestone"]]
    made = json.loads(
        gh(["api", "-X", "POST", f"repos/{repo}/issues", "--input", "-"], stdin=json.dumps(payload))
    )
    return made["number"], made["id"]


def link(repo: str, epic_number: int, child_id: int) -> None:
    try:
        gh(
            [
                "api",
                "-X",
                "POST",
                f"repos/{repo}/issues/{epic_number}/sub_issues",
                "-F",
                f"sub_issue_id={child_id}",
            ]
        )
    except RuntimeError as error:
        if "already" in str(error).lower() or "duplicate" in str(error).lower():
            return
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--plan", required=True)
    parser.add_argument("--repo", default="RexRenatus/deck-streak")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--update-bodies",
        action="store_true",
        help="after every issue exists: re-post the bodies that changed (dependency numbers)",
    )
    args = parser.parse_args()
    plan = json.loads(Path(args.plan).read_text(encoding="utf-8"))
    keys = [issue["key"] for issue in plan]
    if len(keys) != len(set(keys)):
        raise SystemExit("create-issues: the plan repeats a key")
    scrub(plan)
    manifest = load_manifest()
    todo = [issue for issue in plan if issue["key"] not in manifest["issues"]]
    print(
        f"plan {len(plan)} issue(s); {len(plan) - len(todo)} already created; {len(todo)} to create"
    )
    if args.dry_run:
        return 0
    if args.update_bodies:
        return update_bodies(args.repo, plan, manifest)
    milestone_numbers = milestones(args.repo)
    # Epics first, so every child can be linked the moment it exists.
    todo.sort(key=lambda issue: (issue.get("parent") is not None, keys.index(issue["key"])))
    for issue in todo:
        number, ident = create(args.repo, issue, milestone_numbers)
        manifest["issues"][issue["key"]] = {
            "number": number,
            "id": ident,
            "title": issue["title"],
            "body_sha256": digest(issue["body"]),
        }
        save_manifest(manifest)
        print(f"#{number} {issue['key']}")
        time.sleep(PACE_SECONDS)
    linked = 0
    for issue in plan:
        parent = issue.get("parent")
        if not parent:
            continue
        epic = manifest["issues"][parent]["number"]
        child = manifest["issues"][issue["key"]]
        if child.get("parent_linked") == epic:
            continue
        link(args.repo, epic, child["id"])
        child["parent_linked"] = epic
        save_manifest(manifest)
        linked += 1
        time.sleep(PACE_SECONDS / 2)
    print(f"linked {linked} sub-issue(s)")
    return 0


if __name__ == "__main__":
    if "PERSONA_CORE_DENY_LIST" not in os.environ:
        print("create-issues: no private deny list named; scrubbing with the public shapes only")
    sys.exit(main())
