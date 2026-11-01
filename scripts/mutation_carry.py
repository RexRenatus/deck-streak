#!/usr/bin/env python3
"""The release run's carry (SPEC-493, ADR-511): an admitted release run takes a Python mutant's
outcome from one recorded release run of this repository, for a population file whose whole read
set is byte-identical there and here, and runs every other listed mutant.

Verbs of this script:
  base      the base run, chosen by run id alone under the workflow token (R1 to R3)
  admitted  the release pull requests the carry admits (R1)
  read-set  a population file's read set at a tree (R6, R7)
  holds     whether a file's read set is unchanged between two diffs (R4, R8)
  key       each Python class's key at a tree (R5)

`scripts/mutation-verdict.py` runs the verbs `carry` and `manifest` through `verbs`, handing in
its own Python reader, so the legs a base kept are read exactly as its verdict reads them (R8 to
R12). Every refusal is printed by name, and a refusal carries nothing. Standard library only.
"""

from __future__ import annotations

import argparse
import ast
import fnmatch
import hashlib
import json
import os
import pathlib
import posixpath
import re
import subprocess
import sys
import urllib.error
import urllib.request
from collections import Counter
from dataclasses import dataclass, field

sys.dont_write_bytecode = True

#: The release pull requests whose release run may carry (SPEC-493 R1); no other is admitted.
CARRY_PULL_REQUESTS = frozenset({822})

CARRY_SCHEMA = "deckstreak.mutation-carry.v1"
OUTCOMES_SCHEMA = "deckstreak.mutation-outcomes.v1"
INPUTS_SCHEMA = "deckstreak.mutation-inputs.v1"
PYTHON_SCHEMA = "deckstreak.mutation-python.v1"
#: The outcomes a base's judgement can carry (R8); a timeout or a void one never does (R13).
CARRIED = ("killed", "survived", "uncovered", "unviable")
PYTHON_CLASSES = ("scripts", "oracle")
INPUTS = "scripts/mutation-inputs.json"
MAP = "scripts/mutation-python.json"
WORKFLOW = ".github/workflows/ci.yml"
API = "https://api.github.com"
API_VERSION = "2022-11-28"
PLAN_ARTIFACT = "mutation-plan"
OUTCOMES_ARTIFACT = "mutation-outcomes"
ROOT = pathlib.Path(__file__).resolve().parents[1]
DIFF_HEADER = re.compile(r"^diff --git a/(\S+) b/(\S+)$")
INDEX = re.compile(r"^index ([0-9a-f]+)\.\.([0-9a-f]+)")
HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
LINES = re.compile(r"[^\n]*\n|[^\n]+$")


class NoBase(Exception):
    """A reason, by name, that the run has no base: nothing carries."""


# --------------------------------------------------------------------------- admission and base


def admission(plan: object, pull_request: object) -> str | None:
    """Why this run is not admitted (R1), or None: the release run's plan, of a pull request
    `CARRY_PULL_REQUESTS` names."""
    if not isinstance(plan, dict) or plan.get("release") is not True:
        return "not admitted: this run is not the release run, so nothing carries"
    try:
        number = int(str(pull_request))
    except ValueError:
        return f"not admitted: {pull_request!r} names no pull request"
    if number not in CARRY_PULL_REQUESTS:
        return f"not admitted: pull request #{number} is not one the carry admits"
    return None


def select_base(runs: object, run_id: int, repository: str) -> tuple[dict | None, str]:
    """(the one candidate, why) from a listing of runs, by run id alone (R2): the newest run but
    this one; it is the base only when it passes every test, else there is no base, never an
    older run. No mutant outcome is read."""
    listed = runs.get("workflow_runs") if isinstance(runs, dict) else None
    if not isinstance(listed, list):
        return None, "no base: the runs listing holds no workflow runs"
    others = [run for run in listed if isinstance(run, dict) and run.get("id") != run_id]
    if not others:
        return None, "no base: no other run of ci is listed"
    candidate = max(others, key=lambda run: int(run.get("id") or 0))
    number = candidate.get("id")
    heads = candidate.get("head_repository") or {}
    pulls = candidate.get("pull_requests") or []
    tests = (
        (int(number or 0) < run_id, "it is not older than this run"),
        (str(candidate.get("path", "")).split("@")[0] == WORKFLOW, f"it is not {WORKFLOW}'s"),
        (candidate.get("event") == "pull_request", "its event is not pull_request"),
        (candidate.get("head_branch") == "dev", "its head branch is not dev"),
        (heads.get("full_name") == repository, "its head is not this repository's"),
        (
            any(((pull or {}).get("base") or {}).get("ref") == "main" for pull in pulls),
            "it names no pull request into main",
        ),
        (candidate.get("status") == "completed", "it is not completed"),
        (
            candidate.get("conclusion") not in ("cancelled", "skipped"),
            f"its conclusion is {candidate.get('conclusion')}",
        ),
    )
    for passed, why in tests:
        if not passed:
            return None, f"no base: the newest run, {number}, is no base: {why}"
    return candidate, f"the newest run, {number}, qualifies"


def fetch(url: str, token: str) -> object:
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "X-GitHub-Api-Version": API_VERSION,
        },
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.loads(response.read().decode("utf-8"))


def choose_base(args: argparse.Namespace, token: str) -> dict[str, str]:
    """The base step's outputs; the admission is checked first and no request is made for a
    run that is not admitted."""
    outputs = {"base_run": "", "base_manifest": "false"}
    refusal = admission(read_json(args.plan), args.pull_request)
    if refusal:
        print(f"carry: base: {refusal}")
        return outputs
    if not token:
        print("carry: base: no base: the step holds no token")
        return outputs
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    repo, api = args.repository, args.api.rstrip("/")
    query = "event=pull_request&branch=dev&status=completed&per_page=100"
    runs = fetch(f"{api}/repos/{repo}/actions/workflows/ci.yml/runs?{query}", token)
    write_json(out / "runs.json", runs)
    candidate, why = select_base(runs, int(args.run_id), repo)
    print(f"carry: base: {why}")
    if candidate is None:
        return outputs
    number, head = candidate["id"], candidate.get("head_sha")
    compare = fetch(f"{api}/repos/{repo}/compare/{head}...{args.head_sha}", token)
    write_json(out / "compare.json", compare)
    status = compare.get("status") if isinstance(compare, dict) else None
    if status not in ("ahead", "identical"):
        print(f"carry: base: no base: run {number}'s head is {status} of this head, no ancestor")
        return outputs
    listing = fetch(f"{api}/repos/{repo}/actions/runs/{number}/artifacts?per_page=100"
                    "&direction=asc", token)  # fmt: skip
    write_json(out / "artifacts.json", listing)
    artifacts = listing.get("artifacts") if isinstance(listing, dict) else None
    live = {
        str(entry.get("name"))
        for entry in artifacts or []
        if isinstance(entry, dict) and not entry.get("expired")
    }
    if PLAN_ARTIFACT not in live:
        print(f"carry: base: no base: run {number} keeps no live {PLAN_ARTIFACT} artifact")
        return outputs
    manifest = OUTCOMES_ARTIFACT in live
    print(f"carry: base: run {number}, its {'manifest' if manifest else 'legs'} to be read")
    return {"base_run": str(number), "base_manifest": "true" if manifest else "false"}


# --------------------------------------------------------------------------- tree identity


@dataclass(frozen=True)
class Section:
    """One path's section of a `git diff`."""

    path: str
    text: str
    added: bool
    deleted: bool
    binary: bool
    new_id: str | None


def sections(text: str) -> tuple[dict[str, Section], list[str]]:
    """Each path's section of a diff, and each header this reader cannot name a path from."""
    found: dict[str, Section] = {}
    unread: list[str] = []
    blocks: list[list[str]] = []
    for line in LINES.findall(text):
        if line.startswith("diff --git "):
            blocks.append([line])
        elif blocks:
            blocks[-1].append(line)
    for block in blocks:
        header = DIFF_HEADER.match(block[0].rstrip("\n"))
        if header is None or header.group(1) != header.group(2):
            unread.append(block[0].rstrip("\n"))
            continue
        body = "".join(block)
        index = next((INDEX.match(line) for line in block if INDEX.match(line)), None)
        found[header.group(2)] = Section(
            path=header.group(2),
            text=body,
            added=any(line.startswith("new file mode") for line in block[1:4]),
            deleted=any(line.startswith("deleted file mode") for line in block[1:4]),
            binary=any(line.startswith(("Binary files ", "GIT binary patch")) for line in block),
            new_id=index.group(2) if index else None,
        )
    return found, unread


def ancestors(path: str) -> list[str]:
    """Every directory above `path`, the root `.` last."""
    found = []
    while "/" in path:
        path = path.rsplit("/", 1)[0]
        found.append(path)
    return [*found, "."]


@dataclass
class Changes:
    """What differs between the base run's tree and this run's (R4)."""

    changed: set[str] = field(default_factory=set)
    binary: set[str] = field(default_factory=set)
    listings: set[str] = field(default_factory=set)
    unread: list[str] = field(default_factory=list)


def changes(base_text: str, head_text: str) -> Changes:
    """A path is changed when its two sections differ byte for byte, a path in neither diff
    being equal, and every binary section is changed; a directory's listing is changed when a
    changed path below it is present on exactly one side."""
    base, base_unread = sections(base_text)
    head, head_unread = sections(head_text)
    found = Changes(unread=base_unread + head_unread)
    for path in sorted(set(base) | set(head)):
        one, two = base.get(path), head.get(path)
        if (one and one.binary) or (two and two.binary):
            found.changed.add(path)
            found.binary.add(path)
        elif (one.text if one else None) != (two.text if two else None):
            found.changed.add(path)
        if path in found.changed and present(one, two) != present(two, one):
            found.listings.update(ancestors(path))
    return found


def present(side: Section | None, other: Section | None) -> bool:
    """Whether the path is present on `side`'s tree; with no section there it is as at the
    merge-base, which the other side's section names."""
    if side is not None:
        return not side.deleted
    return not (other is not None and other.added)


def blob_id(data: bytes) -> str:
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def applied(old: str, section: Section) -> str:
    """`old` with the section's hunks applied, refused by name where a line does not match."""
    old_lines = LINES.findall(old)
    out: list[str] = []
    at = 0
    lines = LINES.findall(section.text)
    index = 0
    last = None
    while index < len(lines):
        hunk = HUNK.match(lines[index])
        index += 1
        if hunk is None:
            continue
        start, count = int(hunk.group(1)), int(hunk.group(2) or "1")
        begin = start - 1 if count else start
        if begin < at or begin > len(old_lines):
            raise NoBase(f"{section.path}: a hunk of the base's section does not apply")
        out += old_lines[at:begin]
        at = begin
        while index < len(lines) and not lines[index].startswith("@@"):
            line = lines[index]
            index += 1
            tag, body = line[:1], line[1:]
            if tag == "\\":
                if last in (" ", "+") and out:
                    out[-1] = out[-1].removesuffix("\n")
                continue
            if tag in (" ", "-"):
                if at >= len(old_lines) or old_lines[at].rstrip("\n") != body.rstrip("\n"):
                    raise NoBase(f"{section.path}: the base's section does not apply")
                if tag == " ":
                    out.append(old_lines[at])
                at += 1
            elif tag == "+":
                out.append(body)
            else:
                break
            last = tag
    out += old_lines[at:]
    return "".join(out)


class Trees:
    """The base run's files, rebuilt from the merge-base and the base's sections (R5), beside
    this run's files at the root."""

    def __init__(self, root: pathlib.Path, merge_base: str, base: str, head: str) -> None:
        self.root = root
        self.merge_base = merge_base
        self.base, base_unread = sections(base)
        self.head, head_unread = sections(head)
        self.unread = base_unread + head_unread

    def here(self, path: str) -> str | None:
        file = self.root / path
        return file.read_text(encoding="utf-8") if file.is_file() else None

    def at_base(self, path: str) -> str | None:
        one, two = self.base.get(path), self.head.get(path)
        if (one.text if one else None) == (two.text if two else None) and not (one and one.binary):
            return self.here(path)
        if one is not None and one.binary:
            raise NoBase(f"{path}: the base's section is binary, so its file cannot be rebuilt")
        if one is not None and one.deleted:
            return None
        if one is None or not one.added:
            shown = subprocess.run(
                ["git", "-C", str(self.root), "show", f"{self.merge_base}:{path}"],
                capture_output=True,
                check=False,
            )
            if shown.returncode != 0:
                if one is None:
                    return None
                raise NoBase(f"{path}: the merge-base holds no file to rebuild")
            old = shown.stdout.decode("utf-8")
            if one is None:
                return old
        else:
            old = ""
        rebuilt = applied(old, one)
        if one.new_id is None or not blob_id(rebuilt.encode()).startswith(one.new_id):
            raise NoBase(
                f"{path}: the rebuilt file's blob id does not begin with its section's index "
                f"id {one.new_id}"
            )
        return rebuilt


# --------------------------------------------------------------------------- the key


def job_lines(text: str | None, name: str) -> list[str] | None:
    """A job's lines, comment and blank lines set aside, or None when the job is absent."""
    if text is None:
        return None
    found, inside = [], False
    for line in text.splitlines():
        match = JOB.match(line)
        if match:
            inside = match.group(1) == name
            if inside:
                found.append(line)
            continue
        if line[:1].strip():
            inside = False
        if inside and line.strip() and not line.lstrip().startswith("#"):
            found.append(line)
    return found or None


def rows_digest(rows: str | None, runner: str | None, module: str) -> str | None:
    """The runner digest (R5 (c)): the syntax trees of every top-level statement that is not a
    `def` or `class`, and of each function or class the runner names as `<module>.<name>`,
    closed over the module-level names their bodies reference."""
    if rows is None or runner is None:
        return None
    try:
        rows_tree, runner_tree = ast.parse(rows), ast.parse(runner)
    except SyntaxError:
        return None
    named = {
        node.attr
        for node in ast.walk(runner_tree)
        if isinstance(node, ast.Attribute)
        and isinstance(node.value, ast.Name)
        and node.value.id == module
    }
    kinds = (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)
    defined = {node.name: node for node in rows_tree.body if isinstance(node, kinds)}
    closure, pending = set(), sorted(named & set(defined))
    while pending:
        name = pending.pop()
        if name in closure:
            continue
        closure.add(name)
        for node in ast.walk(defined[name]):
            if isinstance(node, ast.Name) and node.id in defined and node.id not in closure:
                pending.append(node.id)
    parts = [ast.dump(node) for node in rows_tree.body if not isinstance(node, kinds)]
    parts += [f"{name}: {ast.dump(defined[name])}" for name in sorted(closure)]
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()


def class_key(read, inputs: dict) -> dict:
    """One Python class's key over `read(path)`, which gives a file's text or None."""
    jobs = [job_lines(read(inputs["workflow"]), name) for name in inputs["leg_jobs"]]
    joined = None if any(job is None for job in jobs) else "\n".join(sum(jobs, []))
    runner_parts = []
    for path in inputs["runner"]:
        text = read(path)
        runner_parts.append(None if text is None else f"{path}\0{sha256(text.encode())}")
    digest = inputs["runner_digest"]
    return {
        "jobs": None if joined is None else sha256(joined.encode()),
        "runner": None if None in runner_parts else sha256("\n".join(runner_parts).encode()),
        "rows": rows_digest(read(digest["file"]), read(digest["named_by"]), digest["module"]),
    }


def read_inputs(text: str | None) -> dict:
    """The read-set map's classes, refused by name when it cannot be read."""
    try:
        document = json.loads(text or "")
    except ValueError as error:
        raise NoBase(f"{INPUTS} cannot be read: {error}") from None
    classes = document.get("classes") if isinstance(document, dict) else None
    if document.get("schema") != INPUTS_SCHEMA or not isinstance(classes, dict):
        raise NoBase(f"{INPUTS} is not of the schema {INPUTS_SCHEMA}")
    for name in PYTHON_CLASSES:
        entry = classes.get(name)
        keys = ("rule", "workflow", "leg_jobs", "runner", "runner_digest")
        if not isinstance(entry, dict) or any(key not in entry for key in keys):
            raise NoBase(f"{INPUTS} holds no complete entry for the {name} class")
    return classes


def keys(read) -> dict:
    classes = read_inputs(read(INPUTS))
    return {name: class_key(read, classes[name]) for name in PYTHON_CLASSES}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


# --------------------------------------------------------------------------- the read set


FS_METHODS = frozenset(
    {
        "read_text",
        "read_bytes",
        "write_text",
        "write_bytes",
        "open",
        "exists",
        "is_file",
        "is_dir",
        "is_symlink",
        "stat",
        "lstat",
        "iterdir",
        "glob",
        "rglob",
        "walk",
        "mkdir",
        "rmdir",
        "unlink",
        "touch",
        "chmod",
        "samefile",
        "readlink",
        "symlink_to",
        "hardlink_to",
        "owner",
        "group",
    }
)
OS_FS = frozenset(
    {
        "listdir",
        "scandir",
        "walk",
        "fwalk",
        "stat",
        "lstat",
        "remove",
        "unlink",
        "rmdir",
        "removedirs",
        "mkdir",
        "makedirs",
        "rename",
        "renames",
        "replace",
        "chdir",
        "chmod",
        "chown",
        "link",
        "symlink",
        "readlink",
        "access",
        "open",
        "truncate",
        "utime",
    }
)
OS_PATH_FS = frozenset(
    {"exists", "lexists", "isfile", "isdir", "islink", "ismount", "getsize", "samefile"}
    | {"getmtime", "getatime", "getctime"}
)
FS_MODULES = frozenset({"shutil", "glob", "fileinput", "filecmp", "zipfile", "tarfile", "io"})
PROCESSES = frozenset({"system", "popen", "fork", "forkpty", "posix_spawn", "posix_spawnp"})
FORBIDDEN_IMPORTS = frozenset({"ctypes", "mmap", "sqlite3", "posix", "_os"})
PATH_TYPES = frozenset({"Path", "PurePath", "PosixPath", "PurePosixPath"})
TEMPS = frozenset({"mkdtemp", "mkstemp", "TemporaryDirectory", "NamedTemporaryFile"})
TEMPS_TOO = frozenset({"gettempdir", "TemporaryFile", "SpooledTemporaryFile"})
JOINS = frozenset({"joinpath", "with_name", "with_suffix", "with_stem"})


def is_temp_call(name: str) -> bool:
    """Whether a call by this dotted name makes a temporary path (R7)."""
    last = name.split(".")[-1]
    return last in TEMPS | TEMPS_TOO and (name.startswith("tempfile.") or name == last)


SAME = frozenset({"resolve", "absolute", "expanduser"})
CWD = "<cwd>"


@dataclass(frozen=True)
class Value:
    """A constant expression's value: a repository path, a text, the working directory, or a
    temporary value."""

    kind: str
    text: str = ""


def dotted(node: ast.AST) -> str:
    parts = []
    while isinstance(node, ast.Attribute):
        parts.append(node.attr)
        node = node.value
    if isinstance(node, ast.Name):
        parts.append(node.id)
        return ".".join(reversed(parts))
    return ""


def joined(base: str, *parts: str) -> str:
    """A repository path, `.` the root; one that escapes the root is the root."""
    path = posixpath.normpath(posixpath.join(base, *parts))
    if path == ".." or path.startswith("../") or path.startswith("/"):
        return "."
    return path


class Member:
    """One closure member's census: what it imports and loads, the paths it reads, and each
    construct outside R7's allowlist."""

    def __init__(self, path: str, text: str, cwd: str) -> None:
        self.path, self.cwd = path, cwd
        self.imports: set[str] = set()
        self.relative: set[str] = set()
        self.loads: set[str] = set()
        self.search: set[str] = set()
        self.reads: set[str] = set()
        self.whole: list[str] = []
        try:
            tree = ast.parse(text)
        except (SyntaxError, ValueError) as error:
            self.whole.append(f"{path}: does not parse: {error}")
            return
        self.module_env: dict[str, Value] = {}
        self.scope(tree.body, self.module_env, set(), module=True)

    def refuse(self, node: ast.AST, why: str) -> None:
        self.whole.append(f"{self.path}:{getattr(node, 'lineno', 0)}: {why}")

    def scope(self, body: list, env: dict, temps: set, module: bool = False) -> None:
        env = env if module else dict(env)
        for statement in body:
            self.bind(statement, env, temps)
        for statement in body:
            self.walk(statement, env, temps)

    def bind(self, statement: ast.AST, env: dict, temps: set) -> None:
        if isinstance(statement, ast.Assign) and len(statement.targets) == 1:
            target = statement.targets[0]
            if isinstance(target, ast.Name):
                value = self.value(statement.value, env, temps)
                if value is not None:
                    env[target.id] = value
                elif self.tempish(statement.value, temps):
                    temps.add(target.id)

    def tempish(self, node: ast.AST, temps: set) -> bool:
        """Whether `node` holds a temporary value: a `tempfile` call, or a name bound to one."""
        return any(
            (isinstance(each, ast.Name) and each.id in temps)
            or (isinstance(each, ast.Call) and is_temp_call(dotted(each.func)))
            for each in ast.walk(node)
        )

    def value(self, node: ast.AST, env: dict, temps: set) -> Value | None:
        """The node's constant value, or None."""
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            return Value("text", node.value)
        if isinstance(node, ast.Name):
            if node.id == "__file__":
                return Value("path", self.path)
            return env.get(node.id) if node.id not in temps else Value("temp")
        if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Div):
            left, right = self.value(node.left, env, temps), self.value(node.right, env, temps)
            if left is None or right is None:
                return None
            if "temp" in (left.kind, right.kind):
                return Value("temp")
            if left.kind in ("path", "cwd") and right.kind == "text":
                return Value("path", joined(left.text, right.text))
            return None
        if isinstance(node, ast.Attribute):
            inner = self.value(node.value, env, temps)
            if inner is None or inner.kind not in ("path", "cwd", "temp"):
                return None
            if inner.kind == "temp":
                return Value("temp")
            if node.attr == "parent":
                return Value("path", joined(inner.text, ".."))
            return None
        if (
            isinstance(node, ast.Subscript)
            and isinstance(node.value, ast.Attribute)
            and node.value.attr == "parents"
        ):
            inner = self.value(node.value.value, env, temps)
            if (
                inner is not None
                and inner.kind == "path"
                and isinstance(node.slice, ast.Constant)
                and isinstance(node.slice.value, int)
            ):
                return Value("path", joined(inner.text, *[".."] * (node.slice.value + 1)))
            return None
        if isinstance(node, ast.Call):
            return self.call_value(node, env, temps)
        return None

    def call_value(self, node: ast.Call, env: dict, temps: set) -> Value | None:
        name = dotted(node.func)
        last = node.func.attr if isinstance(node.func, ast.Attribute) else name.split(".")[-1]
        args = [self.value(arg, env, temps) for arg in node.args]
        if is_temp_call(name):
            return Value("temp")
        if last == "cwd" or name in ("os.getcwd", "os.getcwdb"):
            return Value("cwd", self.cwd)
        if last in PATH_TYPES or name in ("str", "os.fspath", "os.path.abspath", "os.path.realpath",
                                          "os.path.normpath"):  # fmt: skip
            if not node.args:
                return Value("cwd", self.cwd) if last in PATH_TYPES else None
            first = args[0]
            if first is None or len(args) != 1:
                return None
            if first.kind == "text" and last in PATH_TYPES:
                if first.text in ("", "."):
                    return Value("cwd", self.cwd)
                return Value("path", joined(self.cwd, first.text))
            return first
        if name == "os.path.join" and args and None not in args:
            if "temp" in (arg.kind for arg in args):
                return Value("temp")
            if args[0].kind in ("path", "cwd") and all(arg.kind == "text" for arg in args[1:]):
                return Value("path", joined(args[0].text, *(arg.text for arg in args[1:])))
            return None
        if name == "os.path.dirname" and len(args) == 1 and args[0] is not None:
            return Value("path", joined(args[0].text, "..")) if args[0].kind == "path" else None
        if isinstance(node.func, ast.Attribute):
            receiver = self.value(node.func.value, env, temps)
            if receiver is None or receiver.kind not in ("path", "cwd", "temp"):
                return None
            if receiver.kind == "temp":
                return Value("temp")
            if last in SAME and not node.args:
                return receiver
            if (
                last == "joinpath"
                and args
                and all(arg is not None and arg.kind == "text" for arg in args)
            ):
                return Value("path", joined(receiver.text, *(arg.text for arg in args)))
            if last in JOINS:
                # A sibling by name or suffix: the parent directory holds it, whole.
                return Value("path", joined(receiver.text, ".."))
        return None

    def walk(self, node: ast.AST, env: dict, temps: set) -> None:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
            for default in node.args.defaults + node.args.kw_defaults:
                if default is not None:
                    self.walk(default, env, temps)
            body = node.body if isinstance(node.body, list) else [ast.Return(node.body)]
            self.scope(body, env, set(temps))
            return
        if isinstance(node, ast.ClassDef):
            for each in node.bases + node.decorator_list:
                self.walk(each, env, temps)
            self.scope(node.body, env, temps)
            return
        if isinstance(node, (ast.With, ast.AsyncWith)):
            for item in node.items:
                self.walk(item.context_expr, env, temps)
                if isinstance(item.optional_vars, ast.Name) and self.tempish(
                    item.context_expr, temps
                ):
                    temps.add(item.optional_vars.id)
            for statement in node.body:
                self.bind(statement, env, temps)
            for statement in node.body:
                self.walk(statement, env, temps)
            return
        if isinstance(node, ast.Import):
            for alias in node.names:
                self.imported(node, alias.name)
            return
        if isinstance(node, ast.ImportFrom):
            if node.level:
                self.relative.add(node.module or "")
                return
            self.imported(node, node.module or "")
            for alias in node.names:
                self.imports.add(f"{node.module}.{alias.name}")
            return
        if isinstance(node, ast.Attribute) and dotted(node) in ("os.environ", "os.environb"):
            self.refuse(node, "it reads the environment")
        if isinstance(node, ast.Call):
            self.called(node, env, temps)
        if isinstance(node, (ast.Assign, ast.AugAssign)):
            targets = node.targets if isinstance(node, ast.Assign) else [node.target]
            if any(dotted(target).startswith("sys.path") for target in targets):
                self.refuse(node, "it sets sys.path to a value held in data")
        for child in ast.iter_child_nodes(node):
            self.walk(child, env, temps)

    def imported(self, node: ast.AST, name: str) -> None:
        top = name.split(".")[0]
        if top in FORBIDDEN_IMPORTS:
            self.refuse(node, f"it imports {top}")
        self.imports.add(name)

    def constant(self, node: ast.AST, env: dict, temps: set) -> Value | None:
        value = self.value(node, env, temps)
        return value if value is not None and value.kind in ("path", "temp") else None

    def read(self, node: ast.AST, value: Value | None, what: str) -> None:
        """A read, stat or listing of `value`: a constant path is held; the root, the working
        directory and a path held in data make the read set the whole tree."""
        if value is None:
            self.refuse(node, f"{what} a path held in data")
        elif value.kind == "cwd":
            self.refuse(node, f"{what} the working directory")
        elif value.kind == "path" and value.text == ".":
            self.refuse(node, f"{what} the repository root")
        elif value.kind == "path":
            self.reads.add(value.text)

    def called(self, node: ast.Call, env: dict, temps: set) -> None:
        name = dotted(node.func)
        last = node.func.attr if isinstance(node.func, ast.Attribute) else name.split(".")[-1]
        head = name.split(".")[0]
        if name in ("eval", "exec", "compile", "__import__", "importlib.import_module"):
            first = node.args[0] if node.args else None
            if isinstance(first, ast.Constant) and isinstance(first.value, str):
                if name in ("__import__", "importlib.import_module"):
                    self.imports.add(first.value)
                return
            return self.refuse(node, f"it calls {name} on a value held in data")
        if name in ("os.getenv", "os.putenv", "os.unsetenv", "os.getenvb"):
            return self.refuse(node, "it reads the environment")
        if head == "os" and (last in PROCESSES or last.startswith(("exec", "spawn"))):
            return self.refuse(node, f"it starts a process by {name}")
        if head in ("subprocess", "pty", "multiprocessing") or last.startswith("create_subprocess"):
            return self.process(node, env, temps)
        if name in ("sys.path.insert", "sys.path.append", "sys.path.extend"):
            target = node.args[-1] if node.args else None
            value = self.value(target, env, temps) if target is not None else None
            if value is None or value.kind != "path":
                return self.refuse(node, "it puts a path held in data on sys.path")
            self.search.add(value.text)
            return None
        if last in ("spec_from_file_location", "run_path", "SourceFileLoader"):
            target = node.args[-1] if node.args else None
            value = self.value(target, env, temps) if target is not None else None
            if value is None or value.kind != "path" or value.text == ".":
                return self.refuse(node, f"it loads code by {name} from a path held in data")
            self.loads.add(value.text)
            return None
        if last == "run_module":
            first = node.args[0] if node.args else None
            if isinstance(first, ast.Constant) and isinstance(first.value, str):
                self.imports.add(first.value)
                return None
            return self.refuse(node, "it runs a module named in data")
        if name in ("open", "io.open") or head in FS_MODULES:
            for arg in node.args:
                self.read(node, self.fs_arg(arg, env, temps), f"{name} on")
            return None
        if head == "os" and (last in OS_FS or (name.startswith("os.path.") and last in OS_PATH_FS)):
            for arg in node.args[:2]:
                self.read(node, self.fs_arg(arg, env, temps), f"{name} on")
            return None
        if isinstance(node.func, ast.Attribute) and last in FS_METHODS:
            receiver = self.value(node.func.value, env, temps)
            if receiver is not None and receiver.kind == "temp":
                return None
            self.read(node, receiver if receiver is None or receiver.kind != "text" else None,
                      f"it calls {last} on")  # fmt: skip
            return None
        for arg in [*node.args, *(keyword.value for keyword in node.keywords)]:
            value = self.value(arg, env, temps)
            if value is None or last in PATH_TYPES or name in ("str", "os.fspath"):
                continue
            if value.kind == "cwd":
                self.refuse(node, f"it passes the working directory to {name or 'a call'}")
            elif value.kind == "path" and value.text == ".":
                self.refuse(node, f"it passes the repository root to {name or 'a call'}")
            elif value.kind == "path":
                self.reads.add(value.text)
        return None

    def fs_arg(self, node: ast.AST, env: dict, temps: set) -> Value | None:
        value = self.value(node, env, temps)
        if value is not None and value.kind == "text":
            return None if node is None else Value("path", joined(self.cwd, value.text))
        if value is not None and value.kind == "temp":
            return Value("temp")
        return value

    def process(self, node: ast.Call, env: dict, temps: set) -> None:
        """Only an interpreter child whose script is a constant in-repository path is allowed."""
        argv = node.args[0] if node.args else None
        if not isinstance(argv, (ast.List, ast.Tuple)) or not argv.elts:
            return self.refuse(node, f"it starts a process by {dotted(node.func)}")
        program = argv.elts[0]
        interpreter = dotted(program) == "sys.executable" or (
            isinstance(program, ast.Constant) and program.value in ("python3", "python")
        )
        if not interpreter or any(k.arg == "shell" for k in node.keywords):
            return self.refuse(node, "it starts a process other than the interpreter")
        script = None
        for element in argv.elts[1:]:
            value = self.value(element, env, temps)
            if value is None:
                return self.refuse(node, "it hands the interpreter an argument held in data")
            if value.kind == "path" and value.text.endswith(".py") and script is None:
                script = value.text
        if script is None:
            return self.refuse(node, "it runs the interpreter on no constant script")
        self.loads.add(script)
        return None


@dataclass
class ReadSet:
    file: str
    entry: object = None
    members: set[str] = field(default_factory=set)
    paths: set[str] = field(default_factory=set)
    trees: set[str] = field(default_factory=set)
    globs: set[str] = field(default_factory=set)
    listings: set[str] = field(default_factory=set)
    whole: list[str] = field(default_factory=list)

    def document(self) -> dict:
        return {
            "file": self.file,
            "whole": bool(self.whole),
            "why": "; ".join(self.whole),
            "entry": self.entry,
            "members": sorted(self.members),
            "paths": sorted(self.paths),
            "trees": sorted(self.trees),
            "globs": sorted(self.globs),
            "listings": sorted(self.listings),
        }


def read_set(root: pathlib.Path, file: str, mapping: dict) -> ReadSet:
    """S(f) at `root` (R6): `f`'s closure, every probe each import implies in every directory
    the child's sys.path holds, every constant path with its listings, and the whole tree when a
    member holds a construct outside R7's allowlist."""
    found = ReadSet(file=file, entry=mapping.get(file))
    entry = found.entry
    if not isinstance(entry, dict) or not isinstance(entry.get("dir"), str):
        found.whole.append(f"{file}: its map entry names no dir and modules")
        return found
    directory = entry["dir"]
    pending = [file, *(f"{directory}/{module}.py" for module in entry.get("modules") or [])]
    search = {directory}
    census: dict[str, Member] = {}
    while pending:
        path = pending.pop(0)
        if path in census:
            continue
        source = root / path
        if not source.is_file():
            found.whole.append(f"{path}: a member of the closure is not in the tree")
            continue
        try:
            text = source.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as error:
            found.whole.append(f"{path}: cannot be read: {error}")
            continue
        member = Member(path, text, directory)
        census[path] = member
        search |= member.search | {posixpath.dirname(load) for load in member.loads}
        pending += sorted(member.loads)
        for every in census.values():
            for name in every.imports:
                pending += resolved(root, name, search)
            for name in every.relative:
                base = posixpath.dirname(every.path)
                pending += resolved(root, name, {base}) if name else []
    found.members = set(census)
    for member in census.values():
        found.whole += member.whole
        found.trees |= member.reads
        for name in member.imports:
            top = name.split(".")[0]
            for place in search:
                probe = joined(place, top)
                found.paths.add(f"{probe}.py")
                found.trees.add(probe)
                found.globs.add(f"{probe}.*")
    for path in found.members | found.paths | found.trees:
        found.listings.update(ancestors(path))
    found.listings |= {tree for tree in found.trees if (root / tree).is_dir()}
    return found


def resolved(root: pathlib.Path, name: str, search: set[str]) -> list[str]:
    """The in-repository modules an import of `name` may load from `search`."""
    parts = name.split(".")
    found = []
    for place in sorted(search):
        for end in range(1, len(parts) + 1):
            stem = joined(place, *parts[:end])
            if (root / f"{stem}.py").is_file():
                found.append(f"{stem}.py")
            elif (root / stem / "__init__.py").is_file():
                found.append(f"{stem}/__init__.py")
    return found


def holds(found: ReadSet, diff: Changes) -> tuple[bool, str]:
    """Whether no path, probe or listing of the read set changed (R8)."""
    if diff.unread:
        return False, f"a section of a diff names no path this reader can read: {diff.unread[0]}"
    if found.whole:
        if diff.changed:
            first = sorted(diff.changed)[0]
            return False, f"the read set is the whole tree, and {first} changed"
        return True, "the read set is the whole tree, and no path changed"

    def hit(path: str) -> bool:
        return (
            path in found.members
            or path in found.paths
            or any(
                path == tree or tree == "." or path.startswith(f"{tree}/") for tree in found.trees
            )
            or any(fnmatch.fnmatchcase(path, pattern) for pattern in found.globs)
        )

    for path in sorted(diff.binary):
        if hit(path):
            return False, f"{path} has a binary section, which is always changed"
    for path in sorted(diff.changed):
        if hit(path):
            return False, f"{path}, a path of the read set, changed"
    for listing in sorted(diff.listings):
        if listing in found.listings or hit(listing):
            return False, f"the listing of {listing}, which the read set holds, changed"
    return True, "no path, probe or listing of the read set changed"


# --------------------------------------------------------------------------- the union rule


class Scratch:
    """A verdict for a base's legs or a manifest's read: its VOIDs are printed by name."""

    def __init__(self, klass: str, where: str) -> None:
        self.klass, self.where = klass, where
        self.voids: list[str] = []
        self.failures: list[str] = []
        self.examined = 0

    def say(self, text: str) -> None:
        print(f"carry: {self.where}: {text}")

    def void(self, text: str) -> None:
        self.voids.append(text)
        self.say(f"VOID {text}")

    def fail(self, text: str) -> None:
        self.failures.append(text)
        self.say(text)


MISSING = object()


def read_json(path: object) -> object:
    """The document at `path`; MISSING when there is no file, None when it cannot be read."""
    if path is None:
        return MISSING
    file = pathlib.Path(str(path))
    if not file.is_file():
        return MISSING
    try:
        return json.loads(file.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def write_json(path: pathlib.Path, document: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


def listed_mutants(listing: object) -> list[dict]:
    mutants = listing.get("mutants") if isinstance(listing, dict) else None
    if not isinstance(mutants, list) or any(not isinstance(each, dict) for each in mutants):
        raise ValueError("the listing holds no list of mutants")
    return mutants


def union_rule(listed: Counter, rerun: Counter, carried: Counter) -> str | None:
    """Why the listing is not the re-run names plus the carried ones with none in both, as
    multisets (R10), or None when it is."""
    both = sorted(set(rerun) & set(carried))
    held = rerun + carried
    missing, extra = listed - held, held - listed
    if not missing and not extra and not both:
        return None
    first = next(iter(sorted(missing) or sorted(extra) or both), "")
    return (
        f"the union rule: {sum(listed.values())} listed, {sum(rerun.values())} re-run and "
        f"{sum(carried.values())} carried: {sum(missing.values())} missing, "
        f"{sum(extra.values())} extra and {len(both)} in both, so the listing is not the re-run "
        f"names plus the carried ones (first {first})"
    )


def held_to_listing(verdict, plan: dict, directory: str | None, whole: list, classify) -> list:
    """`python_reports`' whole legs held to the plan's listing (R10, R11): the class's listed
    names are the whole re-run legs' names plus the carried ones, none in both, or the class is
    VOID by name with its three counts; each carried outcome joins the reports, to be judged at
    the head as a re-run one is. A run with no carry, no listing and no carried line is read as
    it always was."""
    where = pathlib.Path(directory) / PLAN_ARTIFACT if directory else None
    carry = read_json(where / "carry.json") if where else MISSING
    listing = read_json(where / "python-listed.json") if where else MISSING
    files = plan.get("files") if isinstance(plan, dict) else None
    moved = any(isinstance(entry, dict) and "carried_code" in entry for entry in files or [])
    if carry is MISSING and listing is MISSING and not moved:
        return whole
    klass = verdict.klass
    try:
        listed = listed_mutants(listing)
    except ValueError:
        verdict.void(
            "the union rule: no readable python-listed.json beside the plan, so the listing "
            "cannot hold the legs and the carry"
        )
        return whole
    if carry is MISSING:
        records, base_run = [], "none"
    elif not isinstance(carry, dict) or carry.get("schema") != CARRY_SCHEMA:
        verdict.void(f"the union rule: carry.json is not of the schema {CARRY_SCHEMA}")
        return whole
    else:
        records = [each for each in carry.get("carried") or [] if isinstance(each, dict)]
        base_run = str(carry.get("base_run") or "none")
    mine = [each for each in records if classify(str(each.get("file"))) == klass]
    for each in mine:
        if each.get("outcome") not in CARRIED:
            verdict.void(
                f"carried outcome {each.get('outcome')}: {each.get('name')}, never carried"
            )
    names = Counter(
        str(each.get("name")) for each in listed if classify(str(each.get("file"))) == klass
    )
    rerun = Counter(
        str(mutant.get("name"))
        for _where, report in whole
        for entry in report.get("files", [])
        if isinstance(entry, dict) and classify(str(entry.get("path"))) == klass
        for mutant in entry.get("mutants", [])
        if isinstance(mutant, dict)
    )
    carried = Counter(str(each.get("name")) for each in mine)
    refusal = union_rule(names, rerun, carried)
    if refusal:
        verdict.void(refusal)
    verdict.say(f"carried {sum(carried.values())} from run {base_run}")
    kept = [each for each in mine if each.get("outcome") in CARRIED]
    if not kept:
        return whole
    grouped: dict[str, list[dict]] = {}
    for each in kept:
        grouped.setdefault(str(each.get("file")), []).append(each)
    entries = [
        {"path": path, "modules": [], "byte_readers": [], "void": None, "mutants": mutants}
        for path, mutants in grouped.items()
    ]
    return [*whole, (f"carried from run {base_run}", {"files": entries})]


# --------------------------------------------------------------------------- the carry


def base_judged(base: pathlib.Path, base_plan: dict, base_run: str, reader) -> tuple[dict, str]:
    """{name: [(outcome, killers, run)]} as the base judged them: by its manifest when it kept
    one, else by its legs, read by the verdict's own reader against its plan (R8)."""
    judged: dict[str, list[tuple[str, list, str]]] = {}
    manifest = read_json(base / OUTCOMES_ARTIFACT / "mutation-outcomes.json")
    if manifest is not MISSING:
        if not isinstance(manifest, dict) or manifest.get("schema") != OUTCOMES_SCHEMA:
            raise NoBase("no base: the base's manifest is not of its schema")
        for entry in manifest.get("mutants") or []:
            if isinstance(entry, dict):
                judged.setdefault(str(entry.get("name")), []).append(
                    (
                        str(entry.get("outcome")),
                        list(entry.get("killers") or []),
                        str(entry.get("run")),
                    )
                )
        return judged, "its manifest"
    scratch = Scratch("scripts", "the base's legs")
    for _where, report in reader(scratch, base_plan, str(base / "legs")):
        for entry in report.get("files", []):
            if not isinstance(entry, dict) or entry.get("void"):
                continue
            for mutant in entry.get("mutants", []):
                if isinstance(mutant, dict):
                    judged.setdefault(str(mutant.get("name")), []).append(
                        (str(mutant.get("outcome")), list(mutant.get("killers") or []), base_run)
                    )
    return judged, "its legs"


def carry(args: argparse.Namespace, reader, classify, census) -> dict:
    """The carry document; `plan.json` is rewritten only once it is written (R9)."""
    root = pathlib.Path(args.root).resolve()
    plan_path = pathlib.Path(args.plan)
    plan = read_json(plan_path)
    document = {"schema": CARRY_SCHEMA, "base_run": "", "why": "", "files": {}, "carried": []}
    refusal = admission(plan, args.pull_request)
    if refusal:
        document["why"] = refusal
        return document
    if not args.base_run:
        document["why"] = "no base: the base step chose no run"
        return document
    base = pathlib.Path(args.base_dir)
    base_plan = read_json(base / PLAN_ARTIFACT / "plan.json")
    try:
        if not isinstance(base_plan, dict) or "classes" not in base_plan:
            raise NoBase("no base: the base's plan cannot be read")
        if base_plan.get("release") is not True:
            raise NoBase("no base: the base's plan is not a release plan")
        if base_plan.get("base") != plan.get("base"):
            raise NoBase(
                f"no base: main moved: the base's plan is on {base_plan.get('base')}, this plan "
                f"on {plan.get('base')}"
            )
        texts = []
        for path in (plan_path.parent / "git.diff", base / PLAN_ARTIFACT / "git.diff"):
            if not path.is_file():
                raise NoBase(f"no base: {path.name} cannot be read beside a plan")
            texts.append(path.read_text(encoding="utf-8"))
        trees = Trees(root, str(plan["base"]), texts[1], texts[0])
        if trees.unread:
            raise NoBase(
                f"no base: a section names no path this reader can read: {trees.unread[0]}"
            )
        here_keys, base_keys = keys(trees.here), keys(trees.at_base)
        if trees.at_base(INPUTS) != trees.here(INPUTS):
            raise NoBase(f"no base: {INPUTS} differs at the base")
        here_map, base_map = (
            json.loads(trees.here(MAP) or "{}"),
            json.loads(trees.at_base(MAP) or "{}"),
        )
        _mapping, problems = census(root)
        judged, source = base_judged(base, base_plan, str(args.base_run), reader)
    except NoBase as reason:
        document["why"] = str(reason) if str(reason).startswith("no base") else f"no base: {reason}"
        return document
    classes = read_inputs(trees.here(INPUTS))
    listed = listed_mutants(read_json(args.python_listed))
    diff = changes(texts[1], texts[0])
    document["base_run"] = str(args.base_run)
    by_file: dict[str, list[dict]] = {}
    for entry in listed:
        by_file.setdefault(str(entry["file"]), []).append(entry)
    carried_files = []
    for file, entries in sorted(by_file.items()):
        klass = classify(file)
        why = file_refusal(file, entries, klass, here_keys, base_keys, here_map, base_map,
                           problems, classes, judged, root, diff)  # fmt: skip
        if why is None:
            for entry in entries:
                outcome, killers, run = judged[str(entry["name"])][0]
                document["carried"].append(
                    {**entry, "outcome": outcome, "killers": killers, "run": run}
                )
            carried_files.append(file)
            why = f"carried {len(entries)} from run {args.base_run}, by {source}"
        document["files"][file] = {"carried": why.startswith("carried "), "why": why}
    document["why"] = (
        f"carried {len(document['carried'])} from run {args.base_run}"
        if carried_files
        else f"run {args.base_run} carries nothing"
    )
    document["moved"] = carried_files
    return document


def file_refusal(file, entries, klass, here_keys, base_keys, here_map, base_map, problems,
                 classes, judged, root, diff) -> str | None:  # fmt: skip
    """Why `file`'s listed mutants run whole (R8), or None when every one of them carries."""
    if klass not in PYTHON_CLASSES:
        return f"its class {klass} is never carried"
    if here_keys[klass] != base_keys[klass]:
        differ = [
            part for part in here_keys[klass] if here_keys[klass][part] != base_keys[klass][part]
        ]
        return f"the key of the {klass} class differs at the base: {', '.join(differ)}"
    if problems:
        return f"the map's census at the head finds a problem: {problems[0]}"
    if here_map.get(file) != base_map.get(file):
        return "its map entry differs at the base"
    found = read_set(root, file, here_map)
    if classes[klass].get("rule") != "closure":
        found.whole.append(f"the {klass} class's rule is {classes[klass].get('rule')}")
    unchanged, why = holds(found, diff)
    if not unchanged:
        return why
    names = Counter(str(entry["name"]) for entry in entries)
    for name, count in sorted(names.items()):
        if count != 1:
            return f"{name} is listed {count} times here"
        held = judged.get(name, [])
        if len(held) != 1:
            return f"{name} was judged {len(held)} times at the base"
        if held[0][0] not in CARRIED:
            return f"{name} was {held[0][0]} at the base, which never carries"
    return None


def carry_verb(args: argparse.Namespace, reader, classify, census) -> int:
    """Write `carry.json`, then the legs' view in `plan.json` (R9); an error inside the carry
    writes a carry that holds nothing, and the plan is left as it was (R14)."""
    out = pathlib.Path(args.plan).parent / "carry.json"
    try:
        document = carry(args, reader, classify, census)
    except Exception as error:  # noqa: BLE001 - R14: any error carries nothing, by name
        document = {
            "schema": CARRY_SCHEMA,
            "base_run": "",
            "why": f"an error inside carry: {type(error).__name__}: {error}",
            "files": {},
            "carried": [],
        }
    moved = set(document.pop("moved", []))
    try:
        write_json(out, document)
    except OSError as error:
        print(f"carry: carry.json cannot be written, so the plan is left whole: {error}")
        return 0
    print(f"carry: {document['why']}")
    for file, verdict in document["files"].items():
        print(
            f"carry: {file}: {verdict['why'] if verdict['carried'] else 'runs whole: ' + verdict['why']}"
        )
    if moved:
        plan = json.loads(pathlib.Path(args.plan).read_text(encoding="utf-8"))
        for entry in plan["files"]:
            if entry.get("path") in moved:
                entry["carried_code"], entry["code"] = entry.get("code", []), []
        pathlib.Path(args.plan).write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    return 0


# --------------------------------------------------------------------------- the manifest


def manifest_verb(args: argparse.Namespace, reader, classify) -> int:
    """`mutation-outcomes` (R12): the tree identity, each class's key, and one entry per Python
    mutant judged in a whole leg or carried; never a partial or VOID leg's, a timeout or a void."""
    try:
        plan = read_json(args.plan)
        if not isinstance(plan, dict) or "classes" not in plan:
            raise ValueError(f"{args.plan} is not a mutation plan")
        root = pathlib.Path(args.root).resolve()
        diff = pathlib.Path(args.plan).parent / "git.diff"
        entries, seen = [], set()
        for klass in PYTHON_CLASSES:
            if not (plan["classes"].get(klass) or {}).get("applies"):
                continue
            scratch = Scratch(klass, f"the manifest's {klass} class")
            for where, report in reader(scratch, plan, args.python):
                carried = where.startswith("carried from run")
                for entry in report.get("files", []):
                    path = str(entry.get("path")) if isinstance(entry, dict) else ""
                    if not path or entry.get("void") or classify(path) != klass:
                        continue
                    for mutant in entry.get("mutants", []):
                        name = str(mutant.get("name"))
                        if mutant.get("outcome") not in CARRIED or name in seen:
                            continue
                        seen.add(name)
                        entries.append(
                            {
                                "name": name,
                                "file": path,
                                "class": klass,
                                "outcome": mutant["outcome"],
                                "killers": list(mutant.get("killers") or []),
                                "run": str(mutant.get("run")) if carried else str(args.run_id),
                            }
                        )

        def read(path: str) -> str | None:
            file = root / path
            return file.read_text(encoding="utf-8") if file.is_file() else None

        try:
            kept_keys = keys(read)
        except NoBase as reason:
            kept_keys = {"refused": str(reason)}
        document = {
            "schema": OUTCOMES_SCHEMA,
            "tree": {
                "base": plan.get("base"),
                "head": plan.get("head"),
                "diff_sha256": sha256(diff.read_bytes()) if diff.is_file() else None,
            },
            "keys": kept_keys,
            "mutants": entries,
        }
        write_json(pathlib.Path(args.out), document)
        print(f"carry: manifest: {len(entries)} whole or carried Python outcome(s) kept")
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        print(f"carry: manifest: nothing kept: {type(error).__name__}: {error}")
    return 0


def verbs(argv: list[str] | None, reader, classify, census) -> None:
    """The verdict's verbs `carry` and `manifest`; any other verb returns to the verdict's own
    `main`. `reader` is the verdict's Python reader, `python_reports`."""
    words = sys.argv[1:] if argv is None else [str(word) for word in argv]
    if not words or words[0] not in ("carry", "manifest"):
        return None
    parser = argparse.ArgumentParser(prog="mutation-verdict.py " + words[0])
    parser.add_argument("--root", default=str(ROOT))
    parser.add_argument("--plan", required=True)
    if words[0] == "carry":
        parser.add_argument("--python-listed", required=True)
        parser.add_argument("--base-dir", required=True)
        parser.add_argument("--base-run", default="")
        parser.add_argument("--pull-request", default="")
        raise SystemExit(carry_verb(parser.parse_args(words[1:]), reader, classify, census))
    parser.add_argument("--python", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--run-id", default="")
    raise SystemExit(manifest_verb(parser.parse_args(words[1:]), reader, classify))


# --------------------------------------------------------------------------- this script's verbs


def tree_reader(root: pathlib.Path):
    def read(path: str) -> str | None:
        file = root / path
        return file.read_text(encoding="utf-8") if file.is_file() else None

    return read


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=["base", "admitted", "read-set", "holds", "key"])
    parser.add_argument("--root", default=str(ROOT))
    parser.add_argument("--plan")
    parser.add_argument("--pull-request", default="")
    parser.add_argument("--run-id", type=int, default=0)
    parser.add_argument("--repository", default="")
    parser.add_argument("--head-sha", default="")
    parser.add_argument("--out")
    parser.add_argument("--api", default=API)
    parser.add_argument("--github-output")
    parser.add_argument("--file")
    parser.add_argument("--base-diff")
    parser.add_argument("--head-diff")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.root).resolve()
    if args.verb == "admitted":
        print(json.dumps(sorted(CARRY_PULL_REQUESTS)))
        return 0
    if args.verb == "base":
        try:
            outputs = choose_base(args, os.environ.get("GH_TOKEN", ""))
        except (OSError, ValueError, KeyError, TypeError, urllib.error.URLError) as error:
            print(f"carry: base: no base: a request or its answer failed: {error}")
            outputs = {"base_run": "", "base_manifest": "false"}
        lines = "".join(f"{key}={value}\n" for key, value in outputs.items())
        print(lines, end="")
        if args.github_output:
            with open(args.github_output, "a", encoding="utf-8") as sink:
                sink.write(lines)
        return 0
    if args.verb == "key":
        print(json.dumps(keys(tree_reader(root)), indent=2, sort_keys=True))
        return 0
    mapping = json.loads((root / MAP).read_text(encoding="utf-8"))
    found = read_set(root, args.file, mapping)
    if args.verb == "read-set":
        print(json.dumps(found.document(), indent=2))
        return 0
    base_text = pathlib.Path(args.base_diff).read_text(encoding="utf-8")
    head_text = pathlib.Path(args.head_diff).read_text(encoding="utf-8")
    unchanged, why = holds(found, changes(base_text, head_text))
    print(json.dumps({"file": args.file, "whole": bool(found.whole), "unchanged": unchanged,
                      "why": why}, indent=2))  # fmt: skip
    return 0


if __name__ == "__main__":
    sys.exit(main())
