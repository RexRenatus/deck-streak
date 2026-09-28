"""The host scrub: an inventory that only reads, a list that binds each item by its digest, and an
apply that deletes only what the owner approved, all or nothing (SPEC-060; ADR-060, ADR-010).

Every test builds a synthetic host in a `TemporaryDirectory` at run time, with stub commands first
on the tools' `PATH`, each recording its argument vector there, so no fixture holds a real path,
size or name and no test reads the machine it runs on (SPEC-060 section 3). The expected digests are
computed here from the form SPEC-060 R4 and `docs/schematics/host-scrub.md` give, never read from
the tools. Every enumerating test prints `examined N` and refuses zero, and every absence it asserts
is paired with a positive one.
"""

import hashlib
import json
import os
import stat
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import uuid
from datetime import datetime, timedelta
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from _support import REPO, examined

TOOLS = REPO / "deploy" / "host-scrub"
EXAMPLE_RULES = TOOLS / "rules.example.json"
RUNBOOK = REPO / "docs" / "runbooks" / "host-scrub.md"
SCRUB = REPO / "scripts" / "public-scrub.py"
DAY = 86400

#: One stub, installed under every command name the tools may call. It records its argument vector
#: into the scratch directory, then answers from the scenario the test wrote beside it. `nice` and
#: `ionice` hand on to the command they wrap, so a wrapped command is recorded three times.
STUB = r"""#!{python}
import json
import os
import sys

HERE = {here!r}
name = os.path.basename(sys.argv[0])
args = sys.argv[1:]
with open(os.path.join(HERE, "calls.jsonl"), "a", encoding="utf-8") as log:
    log.write(json.dumps([name] + args) + "\n")


def after_options(args, valued):
    i = 0
    while i < len(args) and args[i].startswith("-"):
        i += 2 if args[i] in valued else 1
    return args[i:]


if name in ("nice", "ionice"):
    rest = after_options(args, {{"-n", "-c", "-p", "-P", "-u"}})
    os.execvp(rest[0], rest)
with open(os.path.join(HERE, "scenario.json"), encoding="utf-8") as handle:
    scenario = json.load(handle)
if name == "systemctl" and args and args[0] == "list-unit-files":
    print("\n".join(scenario["unit_files"]))
elif name == "systemctl" and args and args[0] == "list-units":
    print("\n".join(scenario["units"]))
elif name == "systemctl" and args and args[0] == "show":
    units = args[args.index("--") + 1:] if "--" in args else []
    blocks = []
    for unit in units:
        settings = scenario["show"].get(unit, {{"Id": unit}})
        blocks.append("\n".join(f"{{key}}={{value}}" for key, value in settings.items()))
    print("\n\n".join(blocks))
elif name == "systemctl" and args and args[0] == "is-active":
    states = []
    for unit in [a for a in args[1:] if not a.startswith("-")]:
        state = scenario["active"].get(unit, False)
        states.append(os.path.exists(state) if isinstance(state, str) else bool(state))
    if "--quiet" not in args:
        print("\n".join("active" if s else "inactive" for s in states))
    sys.exit(0 if states and all(states) else 3)
elif name == "dpkg-query":
    wanted = [a for a in args if not a.startswith("-")]
    rows = [r for r in scenario["packages"] if not wanted or f"{{r[0]}}:{{r[1]}}" in wanted or r[0] in wanted]
    if wanted and not rows:
        print(f"dpkg-query: no packages found matching {{wanted[0]}}", file=sys.stderr)
        sys.exit(1)
    for row in rows:
        print("\t".join(row))
elif name == "dpkg":
    package = args[-1] if args else ""
    sys.exit(scenario.get("dpkg_exit", {{}}).get(package, 0))
elif name == "ps":
    print("\n".join(scenario["ps"]))
"""

COMMANDS = ("nice", "ionice", "systemctl", "dpkg-query", "dpkg", "ps", "apt-get")


def write(path, size=None, *, text=None, mtime=None):
    """Write a synthetic file of `size` bytes (or `text`) and return its size."""
    path.parent.mkdir(parents=True, exist_ok=True)
    data = text.encode() if text is not None else bytes((i * 7 + 3) % 251 for i in range(size))
    path.write_bytes(data)
    if mtime is not None:
        os.utime(path, (mtime, mtime))
    return len(data)


class Host:
    """A synthetic host in a scratch directory: its tree, the private rail's directory beside it,
    and stub commands that record every argument vector they are called with."""

    def __init__(self, scratch):
        self.base = Path(scratch)
        self.root = self.base / "host"
        self.srv = self.root / "srv"
        self.var = self.root / "var"
        self.apps = self.srv / "apps"
        self.backups = self.srv / "backups"
        self.outside = self.base / "outside"
        self.private = self.base / "private"
        self.bin = self.base / "bin"
        self.now = time.time()
        self.written = {}
        self.private.mkdir(parents=True)
        self.build()
        self.scenario = self.default_scenario()
        self.install_stubs()

    def file(self, path, size=None, *, text=None, mtime=None):
        self.written[path] = write(path, size, text=text, mtime=mtime)
        return path

    def build(self):
        days = lambda n: self.now - n * DAY  # noqa: E731
        b = self.backups
        self.file(b / "nightly-1.tar", 1500, mtime=days(4))
        self.file(b / "nightly-2.tar", 1600, mtime=days(2))
        os.link(b / "nightly-2.tar", b / "nightly-3.tar")
        self.file(b / "nightly-4.tar", 1700, mtime=days(1))
        self.file(b / "weekly-old.tar", 1800, mtime=days(100))
        self.file(b / "weekly-new.tar", 1900, mtime=days(1))
        for n in range(1, 5):
            self.file(b / f"rotated-{n}.tar", 100, mtime=days(11 - n))
        for venv, size in (
            (self.apps / "idle" / ".venv", 3000),
            (self.apps / "busy" / ".venv", 2000),
        ):
            self.file(venv / "pyvenv.cfg", text="home = /usr/bin\n")
            self.file(venv / "lib" / "site.py", size)
        self.file(self.srv / "guarded" / ".venv" / "pyvenv.cfg", text="home = /usr/bin\n")
        self.file(self.srv / "guarded" / ".venv" / "lib" / "site.py", 900)
        self.file(self.apps / "old-checkout" / ".git" / "HEAD", text="ref: refs/heads/main\n")
        self.file(self.apps / "old-checkout" / "README.md", 500)
        linked = self.apps / "linked-tree"
        self.file(linked / ".git", text="gitdir: ../old-checkout/.git/worktrees/linked-tree\n")
        self.file(linked / "notes.txt", 700)
        self.file(self.outside / "precious.txt", text="kept outside every item\n")
        os.symlink(self.outside / "precious.txt", linked / "outside-link")
        self.file(
            self.apps / "serving" / ".git", text="gitdir: ../old-checkout/.git/worktrees/serving\n"
        )
        self.file(self.apps / "serving" / "app.py", 800)
        self.file(self.apps / "probe-1.py", 120)
        self.file(self.apps / "probe-2.py", 130)
        self.file(self.apps / "keep.py", 140)
        self.file(self.srv / "cache" / "old-1.log", 150)
        self.file(self.var / "log" / "journal.bin", 2500)

    def default_scenario(self):
        busy = self.apps / "busy" / ".venv" / "bin" / "python"
        return {
            "unit_files": [
                "busy.service enabled enabled",
                "serving.service enabled enabled",
                "example.service enabled enabled",
            ],
            "units": [
                "busy.service loaded active running Busy example",
                "serving.service loaded active running Serving example",
                "example.service loaded active running Example",
                "gone.service not-found inactive dead gone.service",
            ],
            "show": {
                "busy.service": {
                    "Id": "busy.service",
                    "ExecStart": f"{{ path={busy} ; argv[]={busy} -m busy ; ignore_errors=no }}",
                    "WorkingDirectory": "",
                },
                "serving.service": {
                    "Id": "serving.service",
                    "ExecStart": "{ path=/usr/bin/python3 ; argv[]=/usr/bin/python3 app.py }",
                    "WorkingDirectory": str(self.apps / "serving"),
                },
                "example.service": {
                    "Id": "example.service",
                    "ExecStart": "{ path=/usr/bin/true ; argv[]=/usr/bin/true }",
                },
            },
            "active": {"example.service": True},
            "packages": [
                ["example-unused-tool", "amd64", "1.0-1", "2048", "ii "],
                ["example-needed-tool", "amd64", "2.0-1", "1024", "ii "],
            ],
            "ps": ["101 0 204800 example-daemon", "102 0 102400 example-worker"],
        }

    def install_stubs(self):
        self.bin.mkdir()
        stub = self.bin / "stub.py"
        stub.write_text(STUB.format(python=sys.executable, here=str(self.bin)))
        stub.chmod(0o755)
        for command in COMMANDS:
            (self.bin / command).symlink_to(stub)
        self.write_scenario()

    def write_scenario(self):
        (self.bin / "scenario.json").write_text(json.dumps(self.scenario))

    def calls(self):
        log = self.bin / "calls.jsonl"
        if not log.exists():
            return []
        return [json.loads(line) for line in log.read_text().splitlines()]

    def rules(self, name="rules.json", **changes):
        document = {
            "schema": "deck-streak-host-scrub-rules/1",
            "roots": [{"path": str(self.srv)}, {"path": str(self.var), "sizes_only": True}],
            "rules": [
                {
                    "name": "nightly-copies",
                    "class": "backup",
                    "dir": str(self.backups),
                    "pattern": "nightly-*.tar",
                    "keep_copies": 2,
                    "reason": "a nightly copy beyond the two newest",
                },
                {
                    "name": "weekly-copies",
                    "class": "backup",
                    "dir": str(self.backups),
                    "pattern": "weekly-*.tar",
                    "keep_days": 30,
                    "reason": "a weekly copy older than thirty days",
                },
                {
                    "name": "rotated-copies",
                    "class": "backup",
                    "dir": str(self.backups),
                    "pattern": "rotated-*.tar",
                    "keep_copies": 1,
                    "rotation_keeps": 3,
                    "reason": "a rotated copy beyond the service's own rotation",
                },
                {
                    "name": "idle-environments",
                    "class": "environment",
                    "under": [str(self.apps), str(self.srv / "guarded")],
                    "reason": "no loaded unit's command names it",
                },
                {
                    "name": "probe-scripts",
                    "class": "loose",
                    "dir": str(self.apps),
                    "pattern": "probe-*.py",
                    "reason": "a one-off probe left loose",
                },
                {
                    "name": "stale-logs",
                    "class": "loose",
                    "dir": str(self.srv / "cache"),
                    "pattern": "old-*.log",
                    "reason": "a one-off log left loose",
                },
                {
                    "name": "unused-tool",
                    "class": "package",
                    "package": "example-unused-tool",
                    "reason": "the owner lists it as unused",
                },
                {
                    "name": "absent-tool",
                    "class": "package",
                    "package": "example-absent-tool",
                    "reason": "the owner lists it as unused",
                },
            ],
            "protected": [str(self.srv / "guarded")],
            "health": [
                {
                    "id": "example-service",
                    "argv": ["systemctl", "is-active", "--quiet", "example.service"],
                }
            ],
        }
        document.update(changes)
        path = self.private / name
        path.write_text(json.dumps(document, indent=2))
        return path

    def run(self, tool, *args):
        env = dict(
            os.environ,
            PATH=f"{self.bin}{os.pathsep}{os.environ.get('PATH', '')}",
            PYTHONDONTWRITEBYTECODE="1",
        )
        return subprocess.run(
            [sys.executable, str(TOOLS / tool), *map(str, args)],
            capture_output=True,
            text=True,
            env=env,
            check=False,
            timeout=120,
        )

    def inventory(self, rules=None):
        out = self.private / "inventory.json"
        done = self.run("inventory.py", rules or self.rules(), "--out", out)
        return done, out

    def plan(self, rules=None):
        rules = rules or self.rules()
        done, inventory = self.inventory(rules)
        if done.returncode != 0:
            raise AssertionError(f"the inventory failed: {done.stdout}{done.stderr}")
        out = self.private / "list.json"
        done = self.run("plan.py", inventory, rules, "--out", out)
        if done.returncode != 0:
            raise AssertionError(f"the plan failed: {done.stdout}{done.stderr}")
        return json.loads(out.read_text()), out

    def approve(self, listing, ids, name="approval.json", **changes):
        taken = datetime.fromisoformat(listing["inventory"]["taken_at"])
        document = {
            "list_digest": listing["digest"],
            "items": list(ids),
            "approver": "the owner",
            "date": taken.date().isoformat(),
            "snapshot": {
                "name": "example-pre-scrub",
                "taken_at": (taken + timedelta(minutes=5)).isoformat(),
            },
        }
        document.update(changes)
        document = {key: value for key, value in document.items() if value is not None}
        path = self.private / name
        path.write_text(json.dumps(document, indent=2))
        return path

    def apply(self, listing_path, approval, *flags, rules=None, log="apply-log.json"):
        return self.run(
            "apply.py",
            listing_path,
            approval,
            "--rules",
            rules or self.rules(),
            "--log",
            self.private / log,
            *flags,
        )


def said(done):
    return done.stdout + done.stderr


def tree(root):
    """Every entry under `root`: a file's or link's mode, size, modification time and content (or
    target); a directory's mode alone, since adding or removing a child changes its own time."""
    found = {}
    for directory, dirs, files in os.walk(root):
        for name in dirs + files:
            path = Path(directory) / name
            st = os.lstat(path)
            if stat.S_ISREG(st.st_mode):
                body = hashlib.sha256(path.read_bytes()).hexdigest()
            elif stat.S_ISLNK(st.st_mode):
                body = os.readlink(path)
            else:
                found[str(path)] = (st.st_mode, None, None, "")
                continue
            found[str(path)] = (st.st_mode, st.st_size, st.st_mtime_ns, body)
    return dict(examined(f"entry(ies) under {root.name}", sorted(found.items())))


def line(path):
    """One entry's digest line, in the form SPEC-060 R4 and the schematic give."""
    st = os.lstat(path)
    if stat.S_ISREG(st.st_mode):
        content = hashlib.sha256(Path(path).read_bytes()).hexdigest()
    elif stat.S_ISLNK(st.st_mode):
        content = hashlib.sha256(os.fsencode(os.readlink(path))).hexdigest()
    else:
        content = ""
    fields = [str(path), str(st.st_size), str(st.st_mtime_ns), format(st.st_mode, "o"), content]
    return b"\0".join(os.fsencode(field) for field in fields)


def file_digest(path):
    return hashlib.sha256(line(path)).hexdigest()


def directory_digest(path):
    lines = [line(path)]
    for directory, dirs, files in os.walk(path):
        lines.extend(line(Path(directory) / name) for name in dirs + files)
    return hashlib.sha256(b"\n".join(sorted(lines))).hexdigest()


def list_digest(listing):
    body = {key: value for key, value in listing.items() if key != "digest"}
    text = json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode()).hexdigest()


def by_path(listing):
    return {item.get("path") or item.get("package"): item for item in listing["items"]}


class Inventory(unittest.TestCase):
    def test_the_inventory_runs_only_its_read_only_allow_list(self):
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            done, out = host.inventory()
            self.assertEqual(done.returncode, 0, said(done))
            calls = host.calls()
            ran = examined(
                "command(s) the inventory ran",
                [c for c in calls if c[0] not in ("nice", "ionice")],
            )
            # Every command ran under nice and the idle IO class, wrapped exactly once each.
            wrapped = [c for c in calls if c[0] == "nice"]
            self.assertEqual(sorted(c[5:] for c in wrapped), sorted(ran))
            for call in wrapped:
                self.assertEqual(call[:5], ["nice", "-n", "19", "ionice", "-c3"], call)
            self.assertEqual(sorted(c[2:] for c in calls if c[0] == "ionice"), sorted(ran))
            # Only the read commands SPEC-060 R1 names, and the health check, ran.
            self.assertEqual(
                {(c[0], c[1]) for c in ran},
                {
                    ("systemctl", "list-unit-files"),
                    ("systemctl", "list-units"),
                    ("systemctl", "show"),
                    ("systemctl", "is-active"),
                    ("dpkg-query", "-W"),
                    ("ps", "-eo"),
                },
            )
            # The inventory records every command it ran, with its exit status.
            record = json.loads(out.read_text())
            self.assertEqual(
                sorted(c["argv"] for c in record["commands"]), sorted(c for c in wrapped)
            )
            self.assertEqual({c["exit"] for c in record["commands"]}, {0})
            # A planted changing command is refused before any command runs.
            for planted in (
                ["systemctl", "restart", "example.service"],
                ["apt-get", "remove", "example-unused-tool"],
            ):
                (host.bin / "calls.jsonl").unlink(missing_ok=True)
                out.unlink(missing_ok=True)
                rules = host.rules(health=[{"id": "planted", "argv": planted}])
                done, out = host.inventory(rules)
                self.assertEqual(done.returncode, 1, said(done))
                self.assertIn(" ".join(planted), said(done))
                self.assertIn("read-only allow list", said(done))
                self.assertEqual(host.calls(), [], "a command ran before the refusal")
                self.assertFalse(out.exists(), "a refused inventory wrote its output")

    def test_the_inventory_reports_space_backups_venvs_and_worktrees(self):
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            done, out = host.inventory()
            self.assertEqual(done.returncode, 0, said(done))
            record = json.loads(out.read_text())
            roots = {root["path"]: root for root in record["roots"]}
            # The free space of the file system each root lies on, measured there.
            disk = os.statvfs(host.srv)
            space = roots[str(host.srv)]["space"]
            self.assertEqual(space["total"], disk.f_blocks * disk.f_frsize)
            self.assertGreater(space["free"], 0)
            self.assertLessEqual(space["free"], space["total"])
            mounts = examined("mount(s) the inventory measured", record["mounts"])
            holding = [m for m in mounts if m.get("device") == os.stat(host.srv).st_dev]
            self.assertTrue(holding, "no mount holds the synthetic root")
            self.assertEqual(holding[0]["total"], disk.f_blocks * disk.f_frsize)
            # Each root's size counts the hard-linked copy once, and a sizes-only root lists none.
            under = lambda top: sum(  # noqa: E731
                size for path, size in host.written.items() if top in path.parents
            )
            self.assertEqual(roots[str(host.srv)]["size"], under(host.srv))
            self.assertEqual(roots[str(host.var)]["size"], under(host.var))
            entries = examined("entry(ies) the inventory recorded", record["entries"])
            paths = {entry["path"] for entry in entries}
            self.assertIn(str(host.backups / "nightly-3.tar"), paths)
            self.assertFalse([p for p in paths if p.startswith(str(host.var))], "sizes only")
            nightly = next(e for e in entries if e["path"].endswith("nightly-2.tar"))
            self.assertEqual(nightly["nlink"], 2)
            for field in ("size", "bytes", "mtime", "owner", "mode"):
                self.assertIn(field, nightly)
            # The stale copies of each backup family, by its retention.
            stale = {family["rule"]: set(family["stale"]) for family in record["backups"]}
            b = host.backups
            self.assertEqual(
                stale,
                {
                    "nightly-copies": {str(b / "nightly-1.tar"), str(b / "nightly-2.tar")},
                    "weekly-copies": {str(b / "weekly-old.tar")},
                    "rotated-copies": {str(b / "rotated-1.tar")},
                },
            )
            # The virtual environments and the worktrees under the roots.
            self.assertEqual(
                {venv["path"] for venv in record["venvs"]},
                {
                    str(host.apps / "idle" / ".venv"),
                    str(host.apps / "busy" / ".venv"),
                    str(host.srv / "guarded" / ".venv"),
                },
            )
            self.assertEqual(
                {(tree_["path"], tree_["kind"]) for tree_ in record["worktrees"]},
                {
                    (str(host.apps / "old-checkout"), "main"),
                    (str(host.apps / "linked-tree"), "linked"),
                    (str(host.apps / "serving"), "linked"),
                },
            )
            # The packages, the loaded units with their commands, and the largest memory users.
            self.assertIn("example-unused-tool", {p["name"] for p in record["packages"]})
            units = {unit["unit"]: unit for unit in record["units"]}
            busy = units["busy.service"]["settings"]["ExecStart"]
            self.assertIn(str(host.apps / "busy" / ".venv" / "bin" / "python"), busy)
            self.assertEqual(units["serving.service"]["active"], "active")
            self.assertEqual(len(record["memory"]), 2)


class Plan(unittest.TestCase):
    def test_the_plan_lists_rule_selected_items_and_deletes_nothing(self):
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            before, outside = tree(host.root), tree(host.outside)
            listing, out = host.plan()
            b, apps = host.backups, host.apps
            items = examined("item(s) the plan listed", listing["items"])
            self.assertEqual(
                {(item["class"], item.get("path") or item.get("package")) for item in items},
                {
                    ("backup", str(b / "nightly-1.tar")),
                    ("backup", str(b / "nightly-2.tar")),
                    ("backup", str(b / "weekly-old.tar")),
                    ("backup", str(b / "rotated-1.tar")),
                    ("venv", str(apps / "idle" / ".venv")),
                    ("worktree", str(apps / "old-checkout")),
                    ("worktree", str(apps / "linked-tree")),
                    ("loose", str(apps / "probe-1.py")),
                    ("loose", str(apps / "probe-2.py")),
                    ("loose", str(host.srv / "cache" / "old-1.log")),
                    ("package", "example-unused-tool:amd64"),
                },
            )
            # Each item names the rule and the reason that selected it, and its digest.
            rules = json.loads((host.private / "rules.json").read_text())["rules"]
            rules = {rule["name"]: rule for rule in rules}
            for item in items:
                self.assertEqual(item["reason"], rules[item["rule"]]["reason"], item)
                self.assertRegex(item["digest"], r"^[0-9a-f]{64}$")
            self.assertEqual(len({item["id"] for item in items}), len(items))
            paths = by_path(listing)
            self.assertEqual(
                paths[str(apps / "probe-1.py")]["digest"], file_digest(apps / "probe-1.py")
            )
            linked = apps / "linked-tree"
            self.assertEqual(paths[str(linked)]["digest"], directory_digest(linked))
            # A copy whose other link survives reclaims nothing; the total is the items' sum.
            nightly = b / "nightly-1.tar"
            self.assertEqual(paths[str(nightly)]["bytes"], os.lstat(nightly).st_blocks * 512)
            self.assertEqual(paths[str(b / "nightly-2.tar")]["bytes"], 0)
            self.assertEqual(listing["bytes"], sum(item["bytes"] for item in items))
            self.assertEqual(listing["digest"], list_digest(listing))
            # The plan wrote only its output and left the synthetic tree byte for byte.
            self.assertEqual(
                sorted(p.name for p in host.private.iterdir()),
                ["inventory.json", out.name, "rules.json"],
            )
            self.assertEqual(tree(host.root), before)
            self.assertEqual(tree(host.outside), outside)


class Apply(unittest.TestCase):
    def test_apply_without_a_matching_approval_deletes_nothing(self):
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            before = tree(host.root)
            absent = host.private / "no-approval.json"
            done = host.apply(list_path, absent, "--apply")
            self.assertEqual(tree(host.root), before)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("no approval", said(done))
            ids = [item["id"] for item in listing["items"]]
            for digest in ("0" * 64, None):
                approval = host.approve(listing, ids, list_digest=digest)
                done = host.apply(list_path, approval, "--apply")
                self.assertEqual(tree(host.root), before)
                self.assertEqual(done.returncode, 1, said(done))
                self.assertIn("does not carry the list's digest", said(done))
            log = json.loads((host.private / "apply-log.json").read_text())
            self.assertIn("does not carry the list's digest", log["refused"]["reason"])

    def test_apply_deletes_exactly_the_approved_items_or_nothing(self):
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            paths = by_path(listing)
            chosen = [host.backups / "nightly-1.tar", host.apps / "probe-1.py"]
            ids = [paths[str(p)]["id"] for p in chosen]
            # One approved item changed after the list was made: the whole run deletes nothing.
            before = tree(host.root)
            probe = host.apps / "probe-1.py"
            probe.write_bytes(probe.read_bytes() + b"#")
            changed = tree(host.root)
            done = host.apply(list_path, host.approve(listing, ids), "--apply")
            self.assertEqual(tree(host.root), changed)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn(f"{ids[1]} ", said(done))
            self.assertIn("digest changed", said(done))
            self.assertNotEqual(changed, before)
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            paths = by_path(listing)
            chosen = [
                host.backups / "nightly-1.tar",
                host.apps / "probe-1.py",
                host.apps / "idle" / ".venv",
            ]
            ids = [paths[str(p)]["id"] for p in chosen]
            approval = host.approve(listing, ids)
            before = tree(host.root)
            # Dry by default: the run names what would go and deletes nothing.
            done = host.apply(list_path, approval)
            self.assertEqual(done.returncode, 0, said(done))
            self.assertIn("dry run", said(done))
            self.assertEqual(tree(host.root), before)
            done = host.apply(list_path, approval, "--apply")
            self.assertEqual(done.returncode, 0, said(done))
            after = tree(host.root)
            kept = [
                p for p in before if not any(Path(p) == c or c in Path(p).parents for c in chosen)
            ]
            self.assertEqual(after, {p: before[p] for p in kept})
            for path in chosen:
                self.assertFalse(path.exists(), path)
            for item in listing["items"]:
                if item["id"] not in ids and item.get("path"):
                    self.assertTrue(os.path.lexists(item["path"]), item["path"])
            log = json.loads((host.private / "apply-log.json").read_text())
            self.assertEqual(sorted(entry["id"] for entry in log["deleted"]), sorted(ids))

    def test_apply_refuses_an_approval_without_a_later_snapshot(self):
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            ids = [by_path(listing)[str(host.apps / "probe-1.py")]["id"]]
            before = tree(host.root)
            taken = datetime.fromisoformat(listing["inventory"]["taken_at"])
            cases = (
                (host.approve(listing, ids, "none.json", snapshot=None), "names no snapshot"),
                (
                    host.approve(
                        listing,
                        ids,
                        "before.json",
                        snapshot={
                            "name": "example-pre-scrub",
                            "taken_at": (taken - timedelta(hours=1)).isoformat(),
                        },
                    ),
                    "taken before the inventory",
                ),
                (
                    host.approve(
                        listing,
                        ids,
                        "unnamed.json",
                        snapshot={"name": "", "taken_at": taken.isoformat()},
                    ),
                    "names no snapshot",
                ),
            )
            for approval, reason in cases:
                done = host.apply(list_path, approval, "--apply")
                self.assertEqual(tree(host.root), before)
                self.assertEqual(done.returncode, 1, said(done))
                self.assertIn(reason, said(done))
            # A snapshot taken after the inventory lets the same approval through.
            done = host.apply(list_path, host.approve(listing, ids), "--apply")
            self.assertEqual(done.returncode, 0, said(done))
            self.assertFalse((host.apps / "probe-1.py").exists())
            self.assertTrue((host.apps / "probe-2.py").exists())

    def test_apply_refuses_protected_paths_and_symbolic_links_out(self):
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            paths = by_path(listing)
            before = tree(host.root)
            probe = paths[str(host.apps / "probe-1.py")]["id"]
            # An item under a protected path of the synthetic list, whatever the approval says.
            guarded = host.rules("guarded.json", protected=[str(host.apps)])
            done = host.apply(list_path, host.approve(listing, [probe]), "--apply", rules=guarded)
            self.assertEqual(tree(host.root), before)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn(f"{probe} ", said(done))
            self.assertIn("protected path", said(done))
            # An item that holds a protected path is refused the same way.
            checkout = paths[str(host.apps / "old-checkout")]["id"]
            inner = host.rules(
                "inner.json", protected=[str(host.apps / "old-checkout" / "README.md")]
            )
            done = host.apply(list_path, host.approve(listing, [checkout]), "--apply", rules=inner)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("protected path", said(done))
            self.assertEqual(tree(host.root), before)
            # A directory item's link out is removed, and what it points at is kept.
            precious = tree(host.outside)
            linked = paths[str(host.apps / "linked-tree")]["id"]
            done = host.apply(list_path, host.approve(listing, [linked]), "--apply")
            self.assertEqual(done.returncode, 0, said(done))
            self.assertFalse(os.path.lexists(host.apps / "linked-tree"))
            self.assertEqual(tree(host.outside), precious)
            # An item reached through a symbolic link is refused, and the far file is kept.
            cache, far = host.srv / "cache", host.outside / "cache"
            os.rename(cache, far)
            os.symlink(far, cache)
            kept = tree(host.outside)
            old_log = paths[str(cache / "old-1.log")]["id"]
            done = host.apply(list_path, host.approve(listing, [old_log]), "--apply")
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("symbolic link", said(done))
            self.assertTrue((far / "old-1.log").exists())
            self.assertEqual(tree(host.outside), kept)


class Health(unittest.TestCase):
    def test_a_health_check_red_after_an_apply_stops_the_scrub(self):
        class Ready(BaseHTTPRequestHandler):
            def do_GET(self):  # noqa: N802
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b"ok")

            def log_message(self, *args):
                pass

        server = ThreadingHTTPServer(("127.0.0.1", 0), Ready)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            with tempfile.TemporaryDirectory() as scratch:
                host = Host(scratch)
                probe = host.apps / "probe-1.py"
                # The service stays active while the probe exists: deleting it turns the check.
                host.scenario["active"]["example.service"] = str(probe)
                host.write_scenario()
                url = f"http://127.0.0.1:{server.server_address[1]}/ready"
                checks = [
                    {
                        "id": "example-service",
                        "argv": ["systemctl", "is-active", "--quiet", "example.service"],
                    },
                    {"id": "example-ready", "url": url},
                ]
                rules = host.rules(health=checks)
                listing, list_path = host.plan(rules)
                inventory = json.loads((host.private / "inventory.json").read_text())
                self.assertEqual(
                    {c["id"]: c["green"] for c in inventory["health_before"]},
                    {"example-service": True, "example-ready": True},
                )
                ids = [by_path(listing)[str(probe)]["id"]]
                done = host.apply(list_path, host.approve(listing, ids), "--apply", rules=rules)
                self.assertEqual(done.returncode, 4, said(done))
                self.assertIn("example-service turned red", said(done))
                log = json.loads((host.private / "apply-log.json").read_text())
                self.assertEqual(log["turned"], ["example-service"])
                self.assertEqual(
                    {c["id"]: c["green"] for c in log["health_after"]},
                    {"example-service": False, "example-ready": True},
                )
                self.assertFalse(probe.exists())
        finally:
            server.shutdown()
            server.server_close()


class PrivateByConstruction(unittest.TestCase):
    def test_no_tool_writes_its_output_inside_the_repository(self):
        inside = REPO / "target" / f"host-scrub-{uuid.uuid4().hex}.json"
        try:
            with tempfile.TemporaryDirectory() as scratch:
                host = Host(scratch)
                rules = host.rules()
                calls = (
                    ("inventory.py", rules, "--out", inside),
                    ("plan.py", host.private / "inventory.json", rules, "--out", inside),
                    (
                        "apply.py",
                        host.private / "list.json",
                        host.private / "approval.json",
                        "--rules",
                        rules,
                        "--log",
                        inside,
                    ),
                )
                for tool, *args in examined("tool(s) asked to write inside", calls):
                    done = host.run(tool, *args)
                    self.assertEqual(done.returncode, 2, said(done))
                    self.assertIn("inside this repository", said(done))
                    self.assertFalse(inside.exists(), tool)
        finally:
            inside.unlink(missing_ok=True)

    def test_no_host_scrub_file_names_a_private_value(self):
        files = examined(
            "host-scrub file(s)",
            sorted(p for p in TOOLS.iterdir() if p.is_file()) + [RUNBOOK, Path(__file__)],
        )
        self.assertEqual(
            {p.name for p in files},
            {
                "inventory.py",
                "plan.py",
                "apply.py",
                "rules.example.json",
                "host-scrub.md",
                "test_host_scrub.py",
            },
        )
        done = subprocess.run(
            [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree"]
            + [argument for path in files for argument in ("--subject", str(path))],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn(f"examined {len(files)} file(s)", done.stdout)
        # The example rules protect what R7 names, with neutral example values elsewhere.
        example = json.loads(EXAMPLE_RULES.read_text())
        for path in (
            "/etc",
            "/usr",
            "/boot",
            "/run/deck-streak-credentials",
            "/usr/local/lib/deck-streak",
        ):
            self.assertIn(path, example["protected"])
        # A planted private value in a rules file is refused by name and line, never echoed.
        home = "/".join(["", "home", "scrub" + "keeper", "backups"])
        address = ".".join(str(octet) for octet in (10, 24, 36, 48))
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "rules.json"
            text = json.dumps(
                {"roots": [{"path": home}], "health": [{"id": "x", "url": f"http://{address}/"}]},
                indent=2,
            )
            planted.write_text(text + "\n")
            done = subprocess.run(
                [
                    sys.executable,
                    str(SCRUB),
                    "--root",
                    str(REPO),
                    "--no-tree",
                    "--subject",
                    scratch,
                ],
                capture_output=True,
                text=True,
                check=False,
            )
        numbered = {n: row for n, row in enumerate(text.splitlines(), start=1)}
        home_line = next(n for n, row in numbered.items() if home in row)
        address_line = next(n for n, row in numbered.items() if address in row)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn(f"rules.json:{home_line}: home-directory", done.stdout)
        self.assertIn(f"rules.json:{address_line}: ipv4", done.stdout)
        self.assertNotIn(address, done.stdout)


if __name__ == "__main__":
    unittest.main()
