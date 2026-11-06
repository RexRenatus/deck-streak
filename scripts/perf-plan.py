#!/usr/bin/env python3
"""The bench plan of one ci lane (SPEC-618, ADR-640).

    perf-plan.py --lane rust|web --event EVENT --repo OWNER/NAME --head-repo OWNER/NAME
                 (--changed FILE | --base REV --head REV) [--metadata FILE]
                 [--github-output FILE] [--step-summary FILE]

It decides whether the lane's benches apply to this run and whether their results upload, and
prints three job outputs and one line that says why:

    applicable=true|false
    upload=true|false
    reason=<why>
    bench-<lane>: applicable: <why>        when the results upload
    bench-<lane>: not applicable: <why>    when they do not

A push to dev or main always applies. A pull request applies when its diff reaches the lane. The
Rust lane is reached by a Cargo manifest or the lockfile, the toolchain pin, the ci workflow, a
file under a bench target's folder, or a Rust file of a package in the bench packages' workspace
dependency closure, which is read from a `cargo metadata --format-version 1` document. The web
lane is reached by a bench file, the bench config, a web manifest or the lockfile, or the web app
source (its tests excepted). A pull request whose head repository is not this one is a fork's: it
never uploads, because GitHub mints a fork no OIDC token, and `applicable` still follows its diff.

The changed paths come from the file `--changed` names, one per line, or from
`git diff --name-only <base> <head>` in this repository. The three outputs are appended to the
file `--github-output` names and the line to the file `--step-summary` names. An input the plan
cannot read prints `bench-<lane>: VOID <what>` and exits 1. Standard library only.
"""

import argparse
import json
import posixpath
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
LANES = ("rust", "web")
PUSH = "a push to dev or main measures the baseline"
FORK = "fork PR (GitHub mints no OIDC token for a fork; the push to dev measures the merged change)"
NONE = "the diff reaches no %s bench (examined %d changed paths)"
CHANGES = "the diff changes %s, %s"
LINE = "bench-%s: %s: %s"
VOID_LINE = "bench-%s: VOID %s"
CARGO_FILES = ("Cargo.toml", "Cargo.lock")
WEB_MANIFESTS = (
    "web/app/package.json",
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
)


class Void(Exception):
    """An input the plan cannot read: the lane decides nothing."""


def lines_of(text):
    return [line.strip() for line in text.splitlines() if line.strip()]


def changed_paths(args):
    if args.changed:
        try:
            return lines_of(Path(args.changed).read_text(encoding="utf-8"))
        except (OSError, ValueError) as err:
            raise Void("cannot read the changed paths at " + args.changed) from err
    if not (args.base and args.head):
        raise Void("a pull request needs --changed, or --base and --head")
    done = subprocess.run(
        ["git", "-C", str(REPO), "diff", "--name-only", args.base, args.head],
        capture_output=True,
        text=True,
        check=False,
    )
    if done.returncode != 0:
        raise Void("git diff --name-only %s %s exited %d" % (args.base, args.head, done.returncode))
    return lines_of(done.stdout)


def owner_of(path, by_dir):
    here = posixpath.dirname(path)
    while True:
        if here in by_dir:
            return by_dir[here]
        if not here:
            return None
        here = posixpath.dirname(here)


def rust_reach(metadata):
    """The Rust lane's reach test, from the closure a cargo metadata document gives."""
    if not metadata:
        raise Void("the rust lane needs --metadata on a pull request")
    try:
        data = json.loads(Path(metadata).read_text(encoding="utf-8"))
        root = data["workspace_root"]
        by_dir = {}
        by_abs_dir = {}
        deps = {}
        benches = []
        bench_dirs = []
        for pkg in data["packages"]:
            name = pkg["name"]
            home = posixpath.dirname(pkg["manifest_path"])
            rel = posixpath.relpath(home, root)
            by_dir["" if rel == "." else rel] = name
            by_abs_dir[home] = name
            deps[name] = [dep["path"] for dep in pkg["dependencies"] if "path" in dep]
            for target in pkg["targets"]:
                if "bench" in target["kind"]:
                    benches.append(name)
                    folder = posixpath.dirname(target["src_path"])
                    bench_dirs.append(posixpath.relpath(folder, root))
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as err:
        raise Void("cannot read the cargo metadata at " + metadata) from err
    closure = set()
    todo = list(benches)
    while todo:
        name = todo.pop()
        if name in closure:
            continue
        closure.add(name)
        for dep in deps[name]:
            if dep in by_abs_dir:
                todo.append(by_abs_dir[dep])

    def reach(path):
        if posixpath.basename(path) in CARGO_FILES:
            return "a Cargo manifest or the lockfile"
        if path == "rust-toolchain.toml":
            return "the toolchain pin"
        if path == ".github/workflows/ci.yml":
            return "the ci workflow"
        if any(path.startswith(folder + "/") for folder in bench_dirs):
            return "a bench file"
        owner = owner_of(path, by_dir)
        if path.endswith(".rs") and owner in closure:
            return "a Rust file in the bench closure (" + owner + ")"
        return None

    return reach


def web_reach(path):
    name = posixpath.basename(path)
    if name.endswith(".bench.ts"):
        return "a bench file"
    if path == "vitest.bench.config.ts":
        return "the bench config"
    if path in WEB_MANIFESTS:
        return "a web manifest or the lockfile"
    if path.startswith("web/app/src/") and ".test." not in name and ".spec." not in name:
        return "web app source"
    return None


def decide(args):
    if not args.repo:
        raise Void("--repo names no repository")
    if args.event == "push":
        return True, True, PUSH
    if args.event != "pull_request":
        raise Void("an event the plan does not read: " + args.event)
    paths = changed_paths(args)
    reach = rust_reach(args.metadata) if args.lane == "rust" else web_reach
    applicable = False
    reason = NONE % (args.lane, len(paths))
    for path in paths:
        why = reach(path)
        if why is not None:
            applicable = True
            reason = CHANGES % (path, why)
            break
    if args.head_repo != args.repo:
        return applicable, False, FORK
    return applicable, applicable, reason


def word(flag):
    return "true" if flag else "false"


def append(path, text):
    if path:
        with Path(path).open("a", encoding="utf-8") as handle:
            handle.write(text)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lane", required=True, choices=LANES)
    parser.add_argument("--event", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--head-repo", required=True)
    parser.add_argument("--changed")
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--metadata")
    parser.add_argument("--github-output")
    parser.add_argument("--step-summary")
    args = parser.parse_args(argv)
    try:
        applicable, upload, reason = decide(args)
    except Void as err:
        print(VOID_LINE % (args.lane, err))
        return 1
    outputs = "applicable=%s\nupload=%s\nreason=%s\n" % (
        word(applicable),
        word(upload),
        reason,
    )
    line = LINE % (args.lane, "applicable" if upload else "not applicable", reason)
    print(outputs + line)
    append(args.github_output, outputs)
    append(args.step_summary, line + "\n")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
