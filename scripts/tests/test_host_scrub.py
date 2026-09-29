"""The host scrub: an inventory that only reads, a list that binds each item by its digest, and an
apply that deletes only what the owner approved, with every check passing before the first
deletion (SPEC-060; ADR-060, ADR-010).

Every test builds a synthetic host in a `TemporaryDirectory` at run time, with stub commands first
on the tools' `PATH`, each recording its argument vector there, so no fixture holds a real path,
size or name and no test runs a real command (SPEC-060 section 3). The expected digests are
computed here from the form SPEC-060 R4 and `docs/schematics/host-scrub.md` give, never read from
the tools. Every enumerating test prints `examined N` and refuses zero, and every absence it asserts
is paired with a positive one. A10 runs a tool through a reader (`READER`) that serves a file's
bytes differently on a second open, so a tool that parses one read and digests another shows it.
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
from datetime import datetime, timedelta, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from types import SimpleNamespace

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
elif name == "timedatectl":
    print(scenario.get("clock", "yes"))
"""

COMMANDS = ("nice", "ionice", "systemctl", "dpkg-query", "dpkg", "ps", "apt-get", "timedatectl")

#: A reader that serves a file's bytes differently on a second read (A10). It runs a tool in its own
#: process, as `python3 TOOL` runs it, with an audit hook that sees every open of the served path:
#: each open is recorded, and just before the second one the path takes the other file's bytes, so
#: a tool that opens the file twice parses one content and digests another.
READER = r"""
import os
import runpy
import sys

served, other, record, tool, *args = sys.argv[1:]
opens = []


def serve(event, details):
    if event != "open" or not details or not isinstance(details[0], (str, bytes, os.PathLike)):
        return
    if os.path.abspath(os.fsdecode(details[0])) != served:
        return
    opens.append(served)
    with open(record, "a", encoding="utf-8") as log:
        log.write("open\n")
    if len(opens) == 2:
        os.replace(other, served)


sys.addaudithook(serve)
sys.argv, sys.path[0] = [tool, *args], os.path.dirname(tool)
runpy.run_path(tool, run_name="__main__")
"""


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
            (self.apps / "named" / ".venv", 2000),
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
        os.link(self.apps / "probe-2.py", self.apps / "probe-3.py")
        self.file(self.apps / "keep.py", 140)
        self.file(self.srv / "cache" / "old-1.log", 150)
        self.file(self.var / "log" / "journal.bin", 2500)

    def default_scenario(self):
        named = self.apps / "named" / ".venv" / "bin" / "python"
        return {
            "unit_files": [
                "named.service enabled enabled",
                "serving.service enabled enabled",
                "example.service enabled enabled",
            ],
            "units": [
                "named.service loaded active running Named example",
                "serving.service loaded active running Serving example",
                "example.service loaded active running Example",
                "gone.service not-found inactive dead gone.service",
            ],
            "show": {
                "named.service": {
                    "Id": "named.service",
                    "ExecStart": f"{{ path={named} ; argv[]={named} -m named ; ignore_errors=no }}",
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
                    "name": "tree-notes",
                    "class": "loose",
                    "dir": str(self.apps / "linked-tree"),
                    "pattern": "notes*.txt",
                    "reason": "a note left in a worktree",
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
                    "argv": ["systemctl", "is-active", "--quiet", "--", "example.service"],
                }
            ],
        }
        document.update(changes)
        path = self.private / name
        path.write_text(json.dumps(document, indent=2))
        return path

    def run(self, tool, *args, cwd=None, through=()):
        env = dict(
            os.environ,
            PATH=f"{self.bin}{os.pathsep}{os.environ.get('PATH', '')}",
            PYTHONDONTWRITEBYTECODE="1",
            # A proxy the environment names is never used for a health check's GET.
            http_proxy="http://127.0.0.1:9",
            HTTP_PROXY="http://127.0.0.1:9",
            no_proxy="",
            NO_PROXY="",
        )
        return subprocess.run(
            [sys.executable, *map(str, through), str(TOOLS / tool), *map(str, args)],
            capture_output=True,
            text=True,
            env=env,
            cwd=cwd,
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
            # The snapshot is taken after the inventory and before the approval is written.
            "snapshot": {
                "name": "example-pre-scrub",
                "taken_at": datetime.now(timezone.utc).isoformat(),
            },
        }
        document.update(changes)
        document = {key: value for key, value in document.items() if value is not None}
        path = self.private / name
        path.write_text(json.dumps(document, indent=2))
        return path

    def relist(self, listing, name="crafted-list.json", **changes):
        """The list with `changes`, its own digest taken again in the form R4 gives, as a hand
        other than the plan's might write it: what refuses it is the apply's own checks."""
        crafted = {key: value for key, value in listing.items() if key != "digest"}
        crafted.update(changes)
        crafted["digest"] = list_digest(crafted)
        path = self.private / name
        path.write_text(json.dumps(crafted, indent=1))
        return crafted, path

    def craft(self, listing, written, kind="loose", real=None, name="crafted-list.json"):
        """The list with one more item, `x001`, whose path is `written` and whose digest is taken
        where it lies (`real`, when `written` is relative), so only its path can refuse it."""
        item = {
            "class": kind,
            "rule": "crafted",
            "reason": "an item the plan did not list",
            "path": written,
            "bytes": 0,
            "digest": item_digest(real or written, written),
            "id": "x001",
        }
        return self.relist(listing, name, items=[*listing["items"], item])

    def apply(self, listing_path, approval, *flags, rules=None, log="apply-log.json", cwd=None):
        return self.run(
            "apply.py",
            listing_path,
            approval,
            "--rules",
            rules or self.rules(),
            "--log",
            self.private / log,
            *flags,
            cwd=cwd,
        )

    def served(self, tool, path, second, *args):
        """`tool` run with `args` through the reader: `path` serves the tool its own bytes at the
        first open and `second` from the second open on (A10). Returns the run, and how many times
        the tool opened `path`."""
        reader = self.base / "reader.py"
        other, record = path.with_name(f"{path.name}.second"), path.with_name(f"{path.name}.opens")
        reader.write_text(READER)
        other.write_bytes(second)
        record.unlink(missing_ok=True)
        done = self.run(tool, *args, through=(reader, path, other, record))
        opens = record.read_text().splitlines() if record.exists() else []
        return done, len(opens)


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


def line(path, written=None):
    """One entry's digest line, in the form SPEC-060 R4 and the schematic give: the path as the
    list writes it (`written`, or `path`), and the entry that path reaches."""
    st = os.lstat(path)
    if stat.S_ISREG(st.st_mode):
        content = hashlib.sha256(Path(path).read_bytes()).hexdigest()
    elif stat.S_ISLNK(st.st_mode):
        content = hashlib.sha256(os.fsencode(os.readlink(path))).hexdigest()
    else:
        content = ""
    written = str(path) if written is None else written
    fields = [written, str(st.st_size), str(st.st_mtime_ns), format(st.st_mode, "o"), content]
    return b"\0".join(os.fsencode(field) for field in fields)


def file_digest(path, written=None):
    return hashlib.sha256(line(path, written)).hexdigest()


def directory_digest(path):
    lines = [line(path)]
    for directory, dirs, files in os.walk(path):
        lines.extend(line(Path(directory) / name) for name in dirs + files)
    return hashlib.sha256(b"\n".join(sorted(lines))).hexdigest()


def item_digest(path, written=None):
    """An item's digest, a directory's or a file's, by what `path` reaches."""
    if stat.S_ISDIR(os.lstat(path).st_mode):
        return directory_digest(path)
    return file_digest(path, written)


def list_digest(listing):
    body = {key: value for key, value in listing.items() if key != "digest"}
    text = json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode()).hexdigest()


def by_path(listing):
    return {item.get("path") or item.get("package"): item for item in listing["items"]}


class Window:
    """A health check's address whose next GET, once armed, makes `change` first: the change lands
    after the apply's checks and before its first deletion, while it reads its health checks."""

    def __init__(self, change):
        self.armed, self.spent = threading.Event(), threading.Event()
        window = self

        class Answer(BaseHTTPRequestHandler):
            def do_GET(self):  # noqa: N802
                if window.armed.is_set() and not window.spent.is_set():
                    window.spent.set()
                    change()
                self.send_response(200)
                self.end_headers()

            def log_message(self, *args):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Answer)
        self.url = f"http://127.0.0.1:{self.server.server_address[1]}/window"

    def __enter__(self):
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        return self

    def __exit__(self, *failure):
        self.server.shutdown()
        self.server.server_close()


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
                    ("timedatectl", "show"),
                },
            )
            # The inventory records every command it ran, with its exit status.
            record = json.loads(out.read_text())
            self.assertEqual(
                sorted(c["argv"] for c in record["commands"]), sorted(c for c in wrapped)
            )
            self.assertEqual({c["exit"] for c in record["commands"]}, {0})
            # A planted changing command is refused before any command runs, and so is a read
            # whose units are not written after `--`, each beginning with a letter or a digit.
            for planted in (
                ["systemctl", "restart", "example.service"],
                ["systemctl", "restart", "--", "example.service"],
                ["apt-get", "remove", "example-unused-tool"],
                ["systemctl", "is-active", "--quiet", "example.service"],
                ["systemctl", "is-active", "--quiet", "-Hexample.service"],
                ["systemctl", "is-active", "--quiet", "--", "-Hexample.service"],
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
                    str(host.apps / "named" / ".venv"),
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
            named = units["named.service"]["settings"]["ExecStart"]
            self.assertIn(str(host.apps / "named" / ".venv" / "bin" / "python"), named)
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
                    ("loose", str(apps / "probe-3.py")),
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
            # Two listed links to one file free its blocks once; a note inside a listed worktree
            # goes with it and is not listed again.
            pair = [paths[str(apps / name)]["bytes"] for name in ("probe-2.py", "probe-3.py")]
            self.assertEqual(sorted(pair), [0, os.lstat(apps / "probe-2.py").st_blocks * 512])
            self.assertIn("tree-notes", {rule["name"] for rule in rules.values()})
            self.assertEqual(listing["bytes"], sum(item["bytes"] for item in items))
            self.assertEqual(listing["digest"], list_digest(listing))
            # The plan wrote only its output and left the synthetic tree byte for byte.
            self.assertEqual(
                sorted(p.name for p in host.private.iterdir()),
                ["inventory.json", out.name, "rules.json"],
            )
            self.assertEqual(tree(host.root), before)
            self.assertEqual(tree(host.outside), outside)
            # Rules other than the ones the inventory read are refused, and no list is written.
            other = host.rules("other.json", protected=[str(host.apps)])
            refused = host.private / "refused-list.json"
            done = host.run("plan.py", host.private / "inventory.json", other, "--out", refused)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("not the ones the inventory read", said(done))
            self.assertFalse(refused.exists())
            # A candidate an inventory names in a form the plan does not read canonically is
            # never compared with the protected paths, and never listed.
            inventory = json.loads((host.private / "inventory.json").read_text())
            written = "/" + str(host.srv / "guarded" / ".venv" / "pyvenv.cfg")
            inventory["loose"][0]["matches"].append({"path": written})
            edited = host.private / "edited-inventory.json"
            edited.write_text(json.dumps(inventory))
            relisted = host.private / "relisted.json"
            done = host.run("plan.py", edited, host.private / "rules.json", "--out", relisted)
            self.assertEqual(done.returncode, 0, said(done))
            again = json.loads(relisted.read_text())
            self.assertEqual(set(by_path(again)), set(by_path(listing)))
            self.assertIn(
                {"path": written, "reason": "not an absolute, canonical path"}, again["skipped"]
            )


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
            # An approval without its approver, its date or ids the list holds is refused.
            for changes, reason in (
                ({"approver": " "}, "names no approver"),
                ({"date": "not a date"}, "date is not a date"),
                ({"items": ["i999"]}, "i999, which the list does not hold"),
            ):
                done = host.apply(list_path, host.approve(listing, ids, **changes), "--apply")
                self.assertEqual(tree(host.root), before)
                self.assertEqual(done.returncode, 1, said(done))
                self.assertIn(reason, said(done))
            # A list edited after it was made no longer matches the digest it carries.
            first = dict(listing["items"][0], path=str(host.apps / "keep.py"))
            edited = dict(listing, items=[first, *listing["items"][1:]])
            list_path.write_text(json.dumps(edited))
            done = host.apply(list_path, host.approve(edited, ids), "--apply")
            self.assertEqual(tree(host.root), before)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("the list changed after it was made", said(done))
            # A list that holds a key twice is refused, whichever of the two a reader keeps.
            doubled = host.private / "doubled-list.json"
            doubled.write_text('{"items": [], ' + json.dumps(listing)[1:])
            done = host.apply(doubled, host.approve(listing, ids), "--apply")
            self.assertEqual(tree(host.root), before)
            self.assertEqual(done.returncode, 2, said(done))
            self.assertIn("the key 'items' is held twice", said(done))

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
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            package = by_path(listing)["example-unused-tool:amd64"]["id"]
            approval = host.approve(listing, [package])
            # A package whose version changed since the list is refused before any removal.
            host.scenario["packages"][0][2] = "1.0-2"
            host.write_scenario()
            done = host.apply(list_path, approval, "--apply")
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn(f"{package} ", said(done))
            self.assertIn("digest changed", said(done))
            # A package dpkg would not remove alone is refused before any removal.
            host.scenario["packages"][0][2] = "1.0-1"
            host.scenario["dpkg_exit"] = {"example-unused-tool:amd64": 2}
            host.write_scenario()
            done = host.apply(list_path, approval, "--apply")
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("would not be removed alone", said(done))
            removals = [c for c in host.calls() if c[0] == "dpkg" and "--dry-run" not in c]
            self.assertEqual(removals, [])
            # The unchanged package that removes alone is removed by dpkg, and nothing else is.
            host.scenario["dpkg_exit"] = {}
            host.write_scenario()
            done = host.apply(list_path, approval, "--apply")
            self.assertEqual(done.returncode, 0, said(done))
            self.assertIn(
                ["dpkg", "--dry-run", "--remove", "example-unused-tool:amd64"], host.calls()
            )
            removals = [c for c in host.calls() if c[0] == "dpkg" and "--dry-run" not in c]
            self.assertEqual(removals, [["dpkg", "--remove", "example-unused-tool:amd64"]])
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            paths = by_path(listing)
            # A file changed at the same size, its modification time put back, is a change.
            probe = host.apps / "probe-1.py"
            st = os.lstat(probe)
            data = bytearray(probe.read_bytes())
            data[0] ^= 0xFF
            probe.write_bytes(bytes(data))
            os.utime(probe, ns=(st.st_atime_ns, st.st_mtime_ns))
            now = os.lstat(probe)
            self.assertEqual((now.st_size, now.st_mtime_ns), (st.st_size, st.st_mtime_ns))
            # So is an entry added inside an approved directory after the list was made.
            venv = host.apps / "idle" / ".venv"
            write(venv / "added-after-the-list.txt", text="not approved\n")
            before = tree(host.root)
            for changed in (probe, venv):
                item = paths[str(changed)]["id"]
                done = host.apply(list_path, host.approve(listing, [item]), "--apply")
                self.assertEqual(tree(host.root), before)
                self.assertEqual(done.returncode, 1, said(done))
                self.assertIn(f"{item} ", said(done))
                self.assertIn("digest changed", said(done))
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            probe = host.apps / "probe-1.py"
            fresh = host.apps / "probe-1.py.new"

            def replace():
                write(fresh, text="written after the checks\n")
                os.replace(fresh, probe)

            # An item replaced after the apply's checks, while it reads its health checks, is not
            # deleted: the apply stops there, and the log says nothing went.
            with Window(replace) as window:
                rules = host.rules(health=[{"id": "window", "url": window.url}])
                listing, list_path = host.plan(rules)
                approval = host.approve(listing, [by_path(listing)[str(probe)]["id"]])
                window.armed.set()
                done = host.apply(list_path, approval, "--apply", rules=rules)
            self.assertEqual(done.returncode, 3, said(done))
            self.assertIn("it changed after its checks", said(done))
            self.assertEqual(probe.read_text(), "written after the checks\n")
            log = json.loads((host.private / "apply-log.json").read_text())
            self.assertEqual(log["deleted"], [])

        # An item that changes after the apply's checks, at its own entry or anywhere below it, is
        # not deleted: each case changes it while the apply reads its health checks.
        def under_venv(host):
            venv = host.apps / "idle" / ".venv"
            return venv, lambda: write(venv / "lib" / "added-late.txt", text="not approved\n")

        def inside_venv(host):
            venv = host.apps / "idle" / ".venv"
            return venv, lambda: (venv / "lib" / "site.py").write_bytes(b"x" * 3000)

        def put_back(host):
            probe = host.apps / "probe-1.py"

            def change():
                st = os.lstat(probe)
                data = bytearray(probe.read_bytes())
                data[0] ^= 0xFF
                probe.write_bytes(bytes(data))
                os.utime(probe, ns=(st.st_atime_ns, st.st_mtime_ns))

            return probe, change

        def rewritten(host):
            probe = host.apps / "probe-1.py"
            return probe, lambda: probe.write_bytes(b"rewritten after the checks\n")

        for label, case in (
            ("an entry added below a directory item's top entry", under_venv),
            ("a file inside a directory item rewritten in place", inside_venv),
            ("a file item rewritten in place, its time put back", put_back),
            ("a file item rewritten in place", rewritten),
        ):
            with self.subTest(label), tempfile.TemporaryDirectory() as scratch:
                host = Host(scratch)
                item, change = case(host)
                with Window(change) as window:
                    rules = host.rules(health=[{"id": "window", "url": window.url}])
                    listing, list_path = host.plan(rules)
                    approval = host.approve(listing, [by_path(listing)[str(item)]["id"]])
                    window.armed.set()
                    done = host.apply(list_path, approval, "--apply", rules=rules)
                self.assertTrue(window.spent.is_set())
                self.assertEqual(done.returncode, 3, said(done))
                self.assertTrue(os.path.lexists(item), said(done))
                log = json.loads((host.private / "apply-log.json").read_text())
                self.assertEqual(log["deleted"], [])

    def test_apply_refuses_an_approval_without_a_later_snapshot(self):
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            ids = [by_path(listing)[str(host.apps / "probe-1.py")]["id"]]
            before = tree(host.root)
            taken = datetime.fromisoformat(listing["inventory"]["taken_at"])
            east, west = timezone(timedelta(hours=2)), timezone(timedelta(hours=-5))

            def snapshot(at):
                return {"name": "example-pre-scrub", "taken_at": at.isoformat()}

            cases = (
                ("none.json", None, ids, "names no snapshot"),
                (
                    "before.json",
                    snapshot(taken - timedelta(hours=1)),
                    ids,
                    "taken before the inventory",
                ),
                (
                    "unnamed.json",
                    {"name": "", "taken_at": taken.isoformat()},
                    ids,
                    "names no snapshot",
                ),
                ("same.json", snapshot(taken), ids, "taken before the inventory"),
                (
                    "naive.json",
                    snapshot((taken + timedelta(hours=1)).replace(tzinfo=None)),
                    ids,
                    "carries no offset",
                ),
                # An instant is compared as an instant, whatever offset it is written in.
                (
                    "east.json",
                    snapshot((taken - timedelta(minutes=30)).astimezone(east)),
                    ids,
                    "taken before the inventory",
                ),
                # A snapshot dated later than the apply's own clock has not been taken yet.
                (
                    "future.json",
                    snapshot(datetime.now(timezone.utc) + timedelta(days=1)),
                    [by_path(listing)[str(host.backups / "nightly-1.tar")]["id"]],
                    "later than the apply's clock",
                ),
            )
            for name, shot, approved, reason in cases:
                with self.subTest(case=name):
                    approval = host.approve(listing, approved, name, snapshot=shot)
                    done = host.apply(list_path, approval, "--apply")
                    self.assertEqual(tree(host.root), before)
                    self.assertEqual(done.returncode, 1, said(done))
                    self.assertIn(reason, said(done))
            # A snapshot taken after the inventory lets the same approval through.
            done = host.apply(list_path, host.approve(listing, ids), "--apply")
            self.assertEqual(done.returncode, 0, said(done))
            self.assertFalse((host.apps / "probe-1.py").exists())
            self.assertTrue((host.apps / "probe-2.py").exists())
            # So does one written in another offset: an inventory two hours old, and a snapshot
            # half an hour after it written five hours behind, compared as instants.
            old = taken - timedelta(hours=2)
            moved = dict(listing["inventory"], taken_at=old.isoformat())
            crafted, path = host.relist(listing, inventory=moved)
            log = host.srv / "cache" / "old-1.log"
            approval = host.approve(
                crafted,
                [by_path(crafted)[str(log)]["id"]],
                "west.json",
                snapshot=snapshot((old + timedelta(minutes=30)).astimezone(west)),
            )
            done = host.apply(path, approval, "--apply")
            self.assertEqual(done.returncode, 0, said(done))
            self.assertFalse(log.exists())
            self.assertTrue((host.apps / "probe-2.py").exists())

    def test_apply_refuses_protected_paths_and_symbolic_links_out(self):
        # An item under a protected path of the synthetic list, or one that holds a protected path,
        # is refused whatever the approval says.
        for case in ("under", "holding"):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as scratch:
                host = Host(scratch)
                listing, _ = host.plan()
                item = host.srv / "guarded" / ".venv" if case == "under" else host.srv
                before = tree(host.root)
                crafted, path = host.craft(listing, str(item), "venv")
                done = host.apply(path, host.approve(crafted, ["x001"]), "--apply")
                self.assertEqual(tree(host.root), before)
                self.assertEqual(done.returncode, 1, said(done))
                self.assertIn("x001 ", said(done))
                self.assertIn("protected path", said(done))
        # A pattern in the protected list protects nothing as a path, so the rules are refused.
        with self.subTest(case="pattern"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            probe = by_path(listing)[str(host.apps / "probe-1.py")]["id"]
            before = tree(host.root)
            pattern = host.rules("pattern.json", protected=[str(host.apps / "probe-*.py")])
            done = host.apply(list_path, host.approve(listing, [probe]), "--apply", rules=pattern)
            self.assertEqual(tree(host.root), before)
            self.assertEqual(done.returncode, 2, said(done))
            self.assertIn("is a pattern", said(done))
        # However the list writes an item's path, a path the apply does not read canonically is
        # refused by the item's id before it is compared, and what it reaches is kept.
        for form in ("parent step", "doubled slash", "trailing slash", "relative"):
            with self.subTest(form=form), tempfile.TemporaryDirectory() as scratch:
                host = Host(scratch)
                listing, _ = host.plan()
                guarded, cwd, kind = host.srv / "guarded", None, "loose"
                if form == "parent step":
                    written = f"{host.apps}/../guarded/.venv/pyvenv.cfg"
                elif form == "doubled slash":
                    written = f"/{guarded}/.venv/pyvenv.cfg"
                elif form == "trailing slash":
                    os.symlink(guarded, host.apps / "lnk")
                    written, kind = f"{host.apps / 'lnk'}/", "venv"
                else:
                    cwd, written = guarded / ".venv", "pyvenv.cfg"
                kept = tree(guarded)
                real = cwd / written if cwd else None
                crafted, path = host.craft(listing, written, kind, real=real)
                done = host.apply(path, host.approve(crafted, ["x001"]), "--apply", cwd=cwd)
                self.assertEqual(done.returncode, 1, said(done))
                self.assertIn("x001: its path is not absolute and canonical", said(done))
                self.assertNotIn(written, said(done))
                self.assertEqual(tree(guarded), kept)
        # Rules that write a path the tools do not read canonically are refused by the path's key,
        # never its value, before anything is read or deleted.
        with self.subTest(case="rules not canonical"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            ids = [by_path(listing)[str(host.apps / "probe-1.py")]["id"]]
            document = json.loads((host.private / "rules.json").read_text())
            written = "/" + str(host.srv / "guarded" / ".venv")
            document["rules"].append(
                {
                    "name": "doubled",
                    "class": "loose",
                    "dir": written,
                    "pattern": "pyvenv.cfg",
                    "reason": "a loose file",
                }
            )
            doubled = host.private / "doubled.json"
            doubled.write_text(json.dumps(document, indent=2))
            before = tree(host.root)
            inventory, _ = host.inventory(doubled)
            applied = host.apply(list_path, host.approve(listing, ids), "--apply", rules=doubled)
            for done in (inventory, applied):
                self.assertEqual(done.returncode, 2, said(done))
                self.assertIn("rule doubled's dir must be an absolute, canonical path", said(done))
                self.assertNotIn(written, said(done))
            self.assertEqual(tree(host.root), before)
        # A protected path given as a link protects what it points at: the plan leaves the
        # environment under the link's target out, and the apply refuses it when a list names it.
        with self.subTest(case="protected link"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            link = host.srv / "guard-link"
            os.symlink(host.srv / "guarded", link)
            rules = host.rules("linked.json", protected=[str(link)])
            listing, _ = host.plan(rules)
            venv = host.srv / "guarded" / ".venv"
            skipped = {entry["path"]: entry["reason"] for entry in listing["skipped"]}
            self.assertIn(str(link), skipped[str(venv)])
            self.assertNotIn(str(venv), by_path(listing))
            kept = tree(venv)
            crafted, path = host.craft(listing, str(venv), "venv")
            done = host.apply(path, host.approve(crafted, ["x001"]), "--apply", rules=rules)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("protected path", said(done))
            self.assertEqual(tree(venv), kept)
        # A sibling whose name only begins with a protected path's is not under it, and goes.
        with self.subTest(case="sibling"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            rules = host.rules("sibling.json", protected=[str(host.apps / "probe-1")])
            listing, list_path = host.plan(rules)
            probe = host.apps / "probe-1.py"
            ids = [by_path(listing)[str(probe)]["id"]]
            done = host.apply(list_path, host.approve(listing, ids), "--apply", rules=rules)
            self.assertEqual(done.returncode, 0, said(done))
            self.assertFalse(probe.exists())
        # An item that is itself a link goes as a link, and what it points at is kept; so is the
        # far end of a link inside a directory item, which goes without following it.
        with self.subTest(case="item a link"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            loose = host.apps / "probe-9.py"
            os.symlink(host.outside / "precious.txt", loose)
            listing, list_path = host.plan()
            linked = host.apps / "linked-tree"
            ids = [by_path(listing)[str(p)]["id"] for p in (loose, linked)]
            precious = tree(host.outside)
            done = host.apply(list_path, host.approve(listing, ids), "--apply")
            self.assertEqual(done.returncode, 0, said(done))
            self.assertFalse(os.path.lexists(loose))
            self.assertFalse(os.path.lexists(linked))
            self.assertEqual(tree(host.outside), precious)
        # An item reached through a symbolic link is refused, and the far file is kept.
        with self.subTest(case="link above"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            cache, far = host.srv / "cache", host.outside / "cache"
            old_log = by_path(listing)[str(cache / "old-1.log")]["id"]
            os.rename(cache, far)
            os.symlink(far, cache)
            kept = tree(host.outside)
            done = host.apply(list_path, host.approve(listing, [old_log]), "--apply")
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("symbolic link", said(done))
            self.assertTrue((far / "old-1.log").exists())
            self.assertEqual(tree(host.outside), kept)
        # A directory above an approved item that becomes a link after the apply's checks, while
        # it reads its health checks, stops the apply before that deletion: nothing goes.
        with self.subTest(case="swapped above"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            cache, moved = host.srv / "cache", host.outside / "cache-moved"
            twin = host.srv / "guarded" / "old-1.log"
            write(twin, text="a protected file with the item's name\n")

            def swap():
                os.rename(cache, moved)
                os.symlink(host.srv / "guarded", cache)

            with Window(swap) as window:
                rules = host.rules(health=[{"id": "window", "url": window.url}])
                listing, list_path = host.plan(rules)
                approval = host.approve(listing, [by_path(listing)[str(cache / "old-1.log")]["id"]])
                window.armed.set()
                done = host.apply(list_path, approval, "--apply", rules=rules)
            self.assertEqual(done.returncode, 3, said(done))
            self.assertIn(
                "a directory above it became a symbolic link after its checks", said(done)
            )
            self.assertEqual(twin.read_text(), "a protected file with the item's name\n")
            self.assertTrue((moved / "old-1.log").exists())

    def test_apply_runs_only_read_health_checks_from_the_listed_rules(self):
        # Rules other than the ones the inventory read, which the list names, are refused before
        # any command runs.
        with self.subTest(case="other rules"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            ids = [by_path(listing)[str(host.apps / "probe-1.py")]["id"]]
            other = host.rules("other.json", protected=[str(host.srv / "guarded"), str(host.var)])
            (host.bin / "calls.jsonl").unlink()
            before = tree(host.root)
            done = host.apply(list_path, host.approve(listing, ids), "--apply", rules=other)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("not the ones the inventory read", said(done))
            self.assertEqual(tree(host.root), before)
            self.assertEqual(host.calls(), [])
        # A changing command given as a health check is refused before anything runs, even in
        # rules a list names as the ones the inventory read: no package tool is called.
        with self.subTest(case="changing health check"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, _ = host.plan()
            removal = ["dpkg", "--remove", "example-needed-tool:amd64"]
            planted = host.rules("planted.json", health=[{"id": "planted", "argv": removal}])
            bound = hashlib.sha256(planted.read_bytes()).hexdigest()
            crafted, path = host.relist(listing, rules_digest=bound)
            ids = [by_path(crafted)[str(host.apps / "probe-1.py")]["id"]]
            (host.bin / "calls.jsonl").unlink()
            before = tree(host.root)
            done = host.apply(path, host.approve(crafted, ids), "--apply", rules=planted)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn(" ".join(removal), said(done))
            self.assertIn("is not on the read-only allow list", said(done))
            self.assertEqual(tree(host.root), before)
            self.assertEqual(host.calls(), [])


class OneRead(unittest.TestCase):
    """What a tool binds by a digest is what it parsed: a file it both parses and binds is read
    once (SPEC-060 A10). Each case runs a tool through the reader, which serves the file's bytes
    differently on a second open, in either order."""

    def test_each_tool_binds_the_bytes_it_parsed(self):
        other_check = {
            "id": "example-other",
            "argv": ["systemctl", "is-active", "--quiet", "--", "other.service"],
        }
        # The inventory records the digest of the rules its health checks were read from.
        for order in ("the rules first", "other rules first"):
            with self.subTest(tool="inventory", order=order), tempfile.TemporaryDirectory() as s:
                host = Host(s)
                rules = host.rules().read_bytes()
                checks = [*json.loads(rules)["health"], other_check]
                other = host.rules("other.json", health=checks).read_bytes()
                first, second = (rules, other) if order == "the rules first" else (other, rules)
                served, out = host.private / "served-rules.json", host.private / "inventory.json"
                served.write_bytes(first)
                done, _ = host.served("inventory.py", served, second, served, "--out", out)
                self.assertEqual(done.returncode, 0, said(done))
                record = json.loads(out.read_text())
                self.assertEqual(record["rules_digest"], hashlib.sha256(first).hexdigest())
                self.assertEqual(
                    [check["id"] for check in record["health_before"]],
                    [check["id"] for check in json.loads(first)["health"]],
                )
        # The plan lists from the rules whose digest it checked: rules naming one more item are
        # refused, and the rules the inventory read are listed from.
        needed = {
            "name": "needed-tool",
            "class": "package",
            "package": "example-needed-tool",
            "reason": "a package the inventory's rules never name",
        }
        for order in ("the rules first", "other rules first"):
            with self.subTest(tool="plan", order=order), tempfile.TemporaryDirectory() as s:
                host = Host(s)
                path = host.rules()
                done, inventory = host.inventory(path)
                self.assertEqual(done.returncode, 0, said(done))
                rules = path.read_bytes()
                named = [*json.loads(rules)["rules"], needed]
                other = host.rules("other.json", rules=named).read_bytes()
                first, second = (rules, other) if order == "the rules first" else (other, rules)
                served, out = host.private / "served-rules.json", host.private / "list.json"
                served.write_bytes(first)
                done, _ = host.served("plan.py", served, second, inventory, served, "--out", out)
                listed = json.loads(out.read_text())["items"] if out.exists() else []
                packages = {item.get("package") for item in listed}
                self.assertNotIn("example-needed-tool:amd64", packages, said(done))
                if order == "the rules first":
                    self.assertEqual(done.returncode, 0, said(done))
                    self.assertIn("example-unused-tool:amd64", packages)
                else:
                    self.assertEqual(done.returncode, 1, said(done))
                    self.assertIn("not the ones the inventory read", said(done))
                    self.assertFalse(out.exists())
        # The plan names the inventory it listed from by the digest of the bytes it parsed.
        for order in ("the inventory first", "another inventory first"):
            with self.subTest(tool="plan", order=order), tempfile.TemporaryDirectory() as s:
                host = Host(s)
                path = host.rules()
                done, written = host.inventory(path)
                self.assertEqual(done.returncode, 0, said(done))
                inventory = written.read_bytes()
                moved = json.loads(inventory)
                earlier = datetime.fromisoformat(moved["taken_at"]) - timedelta(minutes=1)
                moved["taken_at"] = earlier.isoformat()
                another = json.dumps(moved, indent=1).encode()
                first, second = (inventory, another)
                if order == "another inventory first":
                    first, second = (another, inventory)
                served, out = host.private / "served-inventory.json", host.private / "list.json"
                served.write_bytes(first)
                done, _ = host.served("plan.py", served, second, served, path, "--out", out)
                self.assertEqual(done.returncode, 0, said(done))
                named = json.loads(out.read_text())["inventory"]
                self.assertEqual(named["digest"], hashlib.sha256(first).hexdigest())
                self.assertEqual(named["taken_at"], json.loads(first)["taken_at"])
        # The apply acts on the rules whose digest it checked against the list's: the rules the
        # inventory read delete the approved item, and rules that drop a protected path are
        # refused, so the item that path protects stays.
        with (
            self.subTest(tool="apply", order="the rules first"),
            tempfile.TemporaryDirectory() as s,
        ):
            host = Host(s)
            path = host.rules()
            listing, list_path = host.plan(path)
            probe = host.apps / "probe-1.py"
            ids = [by_path(listing)[str(probe)]["id"]]
            rules = path.read_bytes()
            other = host.rules("other.json", protected=[]).read_bytes()
            served, log = host.private / "served-rules.json", host.private / "apply-log.json"
            served.write_bytes(rules)
            approval = host.approve(listing, ids)
            flags = ("--rules", served, "--log", log, "--apply")
            done, _ = host.served("apply.py", served, other, list_path, approval, *flags)
            self.assertEqual(done.returncode, 0, said(done))
            self.assertFalse(probe.exists())
            self.assertEqual([entry["id"] for entry in json.loads(log.read_text())["deleted"]], ids)
        # The apply's protected check reads the rules it bound: rules that name the guarded path
        # first are the ones it acts on, so the guarded environment stays.
        with (
            self.subTest(tool="apply", order="the rules first, a guarded item"),
            tempfile.TemporaryDirectory() as s,
        ):
            host = Host(s)
            path = host.rules()
            listing, _ = host.plan(path)
            guarded = host.srv / "guarded"
            crafted, list_path = host.craft(listing, str(guarded / ".venv"), "venv")
            rules = path.read_bytes()
            other = host.rules("other.json", protected=[]).read_bytes()
            served, log = host.private / "served-rules.json", host.private / "apply-log.json"
            served.write_bytes(rules)
            kept = tree(guarded)
            approval = host.approve(crafted, ["x001"])
            flags = ("--rules", served, "--log", log, "--apply")
            done, _ = host.served("apply.py", served, other, list_path, approval, *flags)
            self.assertTrue((guarded / ".venv").is_dir(), said(done))
            self.assertEqual(tree(guarded), kept)
            self.assertEqual(done.returncode, 1, said(done))
        with (
            self.subTest(tool="apply", order="other rules first"),
            tempfile.TemporaryDirectory() as s,
        ):
            host = Host(s)
            path = host.rules()
            listing, _ = host.plan(path)
            guarded = host.srv / "guarded"
            crafted, list_path = host.craft(listing, str(guarded / ".venv"), "venv")
            rules = path.read_bytes()
            other = host.rules("other.json", protected=[]).read_bytes()
            served, log = host.private / "served-rules.json", host.private / "apply-log.json"
            served.write_bytes(other)
            kept = tree(guarded)
            approval = host.approve(crafted, ["x001"])
            flags = ("--rules", served, "--log", log, "--apply")
            done, _ = host.served("apply.py", served, rules, list_path, approval, *flags)
            self.assertTrue((guarded / ".venv").is_dir(), said(done))
            self.assertEqual(tree(guarded), kept)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("not the ones the inventory read", said(done))
        # The plan lists from the inventory it named: an inventory that differs in a candidate on
        # a second open is never listed from.
        with self.subTest(tool="plan", order="the inventory, a candidate differing"):
            with tempfile.TemporaryDirectory() as s:
                host = Host(s)
                path = host.rules()
                done, written = host.inventory(path)
                self.assertEqual(done.returncode, 0, said(done))
                inventory = written.read_bytes()
                moved = json.loads(inventory)
                for package in moved["packages"]:
                    if package["name"] == "example-unused-tool":
                        package["version"] = "9.9-9"
                served, out = host.private / "served-inventory.json", host.private / "list.json"
                served.write_bytes(inventory)
                second = json.dumps(moved, indent=1).encode()
                done, _ = host.served("plan.py", served, second, served, path, "--out", out)
                self.assertEqual(done.returncode, 0, said(done))
                versions = {
                    item["package"]: item["version"]
                    for item in json.loads(out.read_text())["items"]
                    if item.get("package")
                }
                self.assertEqual(versions, {"example-unused-tool:amd64": "1.0-1"})
        # The apply deletes the items of the list it bound and approved: a list that maps the
        # approved id to another file on a second open never sends that file.
        with self.subTest(tool="apply", order="the list, an approved id remapped"):
            with tempfile.TemporaryDirectory() as s:
                host = Host(s)
                path = host.rules()
                listing, list_path = host.plan(path)
                paths = by_path(listing)
                probe, other = host.apps / "probe-1.py", host.apps / "keep.py"
                ids = [paths[str(probe)]["id"]]
                remapped = [dict(item) for item in listing["items"]]
                for item in remapped:
                    if item["id"] == ids[0]:
                        item["path"], item["digest"] = str(other), item_digest(other)
                _, second = host.relist(listing, "second-list.json", items=remapped)
                served, log = host.private / "served-list.json", host.private / "apply-log.json"
                served.write_bytes(list_path.read_bytes())
                approval = host.approve(listing, ids)
                flags = ("--rules", path, "--log", log, "--apply")
                done, _ = host.served(
                    "apply.py", served, second.read_bytes(), served, approval, *flags
                )
                self.assertEqual(done.returncode, 0, said(done))
                self.assertFalse(probe.exists())
                self.assertTrue(other.exists())
                deleted = [entry["id"] for entry in json.loads(log.read_text())["deleted"]]
                self.assertEqual(deleted, ids)

    def test_each_tool_opens_each_file_it_binds_once(self):
        # Beside A10: the rules in each tool, and the inventory in the plan, are opened once a run.
        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            path = host.rules()
            rules = path.read_bytes()
            served = host.private / "served-rules.json"
            inventory, out = host.private / "inventory.json", host.private / "list.json"
            runs = {}
            served.write_bytes(rules)
            runs["the inventory's rules"] = host.served(
                "inventory.py", served, rules, served, "--out", inventory
            )
            served.write_bytes(rules)
            runs["the plan's rules"] = host.served(
                "plan.py", served, rules, inventory, served, "--out", out
            )
            copy = host.private / "served-inventory.json"
            copy.write_bytes(inventory.read_bytes())
            runs["the plan's inventory"] = host.served(
                "plan.py", copy, inventory.read_bytes(), copy, path, "--out", out
            )
            listing = json.loads(out.read_text())
            approval = host.approve(
                listing, [by_path(listing)[str(host.apps / "probe-1.py")]["id"]]
            )
            served.write_bytes(rules)
            flags = ("--rules", served, "--log", host.private / "apply-log.json")
            runs["the apply's rules"] = host.served(
                "apply.py", served, rules, out, approval, *flags
            )
            listed_copy, approval_copy = (
                host.private / "served-list.json",
                host.private / "served-approval.json",
            )
            listed_copy.write_bytes(out.read_bytes())
            approval_copy.write_bytes(approval.read_bytes())
            runs["the apply's list"] = host.served(
                "apply.py", listed_copy, out.read_bytes(), listed_copy, approval, *flags
            )
            runs["the apply's approval"] = host.served(
                "apply.py", approval_copy, approval.read_bytes(), out, approval_copy, *flags
            )
            for read, (done, opens) in runs.items():
                with self.subTest(read=read):
                    self.assertEqual(done.returncode, 0, said(done))
                    self.assertEqual(opens, 1)


class Axes(unittest.TestCase):
    """What the digests cannot see (SPEC-060 A11): the clock the approval's instants are read on,
    and a file system mounted inside an item."""

    def test_the_tools_refuse_a_clock_that_is_not_synchronised(self):
        # The inventory records that the clock reads synchronised, and the list carries it.
        with self.subTest("the inventory records it"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, _ = host.plan()
            record = json.loads((host.private / "inventory.json").read_text())
            self.assertIs(record.get("clock_synchronised"), True)
            self.assertIs(listing["inventory"].get("clock_synchronised"), True)
        # A clock that does not read synchronised at the inventory refuses it, and writes nothing.
        with self.subTest("the inventory refuses it"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            host.scenario["clock"] = "no"
            host.write_scenario()
            done, out = host.inventory()
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("clock", said(done))
            self.assertFalse(out.exists())
        # A clock that does not read synchronised at the apply refuses it before any deletion.
        with self.subTest("the apply refuses it"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, list_path = host.plan()
            probe = host.apps / "probe-1.py"
            approval = host.approve(listing, [by_path(listing)[str(probe)]["id"]])
            host.scenario["clock"] = "no"
            host.write_scenario()
            before = tree(host.root)
            done = host.apply(list_path, approval, "--apply")
            self.assertEqual(tree(host.root), before)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("clock", said(done))
            host.scenario["clock"] = "yes"
            host.write_scenario()
            done = host.apply(list_path, approval, "--apply")
            self.assertEqual(done.returncode, 0, said(done))
            self.assertFalse(probe.exists())
        # An inventory that did not record the clock is refused, whatever the clock reads now.
        with self.subTest("the list does not carry it"), tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            listing, _ = host.plan()
            named = {key: value for key, value in listing["inventory"].items()}
            named.pop("clock_synchronised", None)
            probe = host.apps / "probe-1.py"
            crafted, list_path = host.relist(listing, inventory=named)
            approval = host.approve(crafted, [by_path(crafted)[str(probe)]["id"]])
            before = tree(host.root)
            done = host.apply(list_path, approval, "--apply")
            self.assertEqual(tree(host.root), before)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("clock", said(done))

    def test_an_item_holding_another_device_is_never_digested_or_removed(self):
        # Unprivileged mounts are blocked on the box that ran this, so a file system mounted inside
        # an item is faked at the seam: the walk reports one entry under another device.
        sys.path.insert(0, str(TOOLS))
        try:
            import apply as apply_tool
            import plan as plan_tool
        finally:
            sys.path.remove(str(TOOLS))
        real_walk = plan_tool.walk

        def across(top, errors, descend=None):
            for path, st in real_walk(top, errors, descend):
                fields = {key: getattr(st, key) for key in dir(st) if key.startswith("st_")}
                if os.path.basename(path) == "lib":
                    fields["st_dev"] += 1
                yield path, SimpleNamespace(**fields)

        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            venv = str(host.apps / "idle" / ".venv")
            digest, _ = plan_tool.measure(venv)
            candidate = {
                "class": "venv",
                "rule": {"name": "idle-environments", "reason": "no unit names it"},
                "path": venv,
            }
            item = {"id": "i001", "class": "venv", "path": venv, "digest": digest}
            plan_tool.walk = across
            try:
                with self.assertRaises(OSError) as raised:
                    plan_tool.measure(venv)
                self.assertIn("device", str(raised.exception))
                skipped = []
                self.assertEqual(plan_tool.items_of([candidate], skipped), [])
                self.assertEqual([entry["path"] for entry in skipped], [venv])
                with self.assertRaises(apply_tool.Refusal):
                    apply_tool.check_item(item, [], apply_tool.Runner())
            finally:
                plan_tool.walk = real_walk
            self.assertEqual(plan_tool.measure(venv)[0], digest)
            self.assertIsNotNone(apply_tool.check_item(item, [], apply_tool.Runner()))

    def test_an_item_that_is_or_holds_a_mount_point_is_refused(self):
        # A bind mount shares its device with the tree around it, so the device check cannot see
        # it. The mount table is read through one seam (`read_mountinfo`) and faked here.
        sys.path.insert(0, str(TOOLS))
        try:
            import apply as apply_tool
            import plan as plan_tool
        finally:
            sys.path.remove(str(TOOLS))
        real_reader = plan_tool.read_mountinfo

        def table(*points):
            rows = [
                f"{30 + n} 1 8:1 / {point} rw,relatime shared:1 - ext4 /dev/x rw"
                for n, point in enumerate(points)
            ]
            return "\n".join(["20 1 8:1 / / rw - ext4 /dev/x rw"] + rows) + "\n"

        def refused(item, candidate, mounts):
            plan_tool.read_mountinfo = lambda: mounts
            try:
                skipped = []
                listed = plan_tool.items_of([candidate], skipped)
                try:
                    apply_tool.check_item(item, [], apply_tool.Runner())
                except apply_tool.Refusal as refusal:
                    return listed, skipped, str(refusal)
                return listed, skipped, None
            finally:
                plan_tool.read_mountinfo = real_reader

        with tempfile.TemporaryDirectory() as scratch:
            host = Host(scratch)
            venv = str(host.apps / "idle" / ".venv")
            spaced = host.apps / "idle" / "old env two"
            write(spaced / "lib" / "site.py", 300)
            single = host.apps / "idle" / "one.bak"
            write(single, 200)
            rule = {"name": "idle-environments", "reason": "no unit names it"}
            cases = {}
            for path in (venv, str(spaced), str(single)):
                digest, _ = plan_tool.measure(path)
                cases[path] = (
                    {"id": "i001", "class": "venv", "path": path, "digest": digest},
                    {"class": "venv", "rule": rule, "path": path},
                )
            # A control: mount points that are neither the item nor under it change nothing.
            item, candidate = cases[venv]
            elsewhere = table(str(host.apps / "idle"), str(host.apps / "named" / ".venv"))
            with self.subTest("a control with none of them under the item"):
                listed, skipped, refusal = refused(item, candidate, elsewhere)
                self.assertEqual([entry["path"] for entry in listed], [venv], skipped)
                self.assertIsNone(refusal)
            with self.subTest("an item that is a mount point"):
                listed, skipped, refusal = refused(item, candidate, table(venv))
                self.assertEqual(listed, [])
                self.assertEqual([entry["path"] for entry in skipped], [venv])
                self.assertIn("mount", skipped[0]["reason"])
                self.assertIsNotNone(refusal)
                self.assertIn("mount", refusal)
            with self.subTest("an item that holds a mount point"):
                listed, skipped, refusal = refused(item, candidate, table(venv + "/lib"))
                self.assertEqual(listed, [])
                self.assertEqual([entry["path"] for entry in skipped], [venv])
                self.assertIsNotNone(refusal)
                self.assertIn("mount", refusal)
            with self.subTest("a mount point that only shares the item's name as a prefix"):
                listed, skipped, refusal = refused(item, candidate, table(venv + "-other"))
                self.assertEqual([entry["path"] for entry in listed], [venv], skipped)
                self.assertIsNone(refusal)
            with self.subTest("an escaped mount point is read as the path it names"):
                item, candidate = cases[str(spaced)]
                escaped = str(spaced).replace(" ", "\\040") + "/lib"
                listed, skipped, refusal = refused(item, candidate, table(escaped))
                self.assertEqual(listed, [])
                self.assertEqual([entry["path"] for entry in skipped], [str(spaced)])
                self.assertIsNotNone(refusal)
            with self.subTest("a file mounted over a file"):
                item, candidate = cases[str(single)]
                listed, skipped, refusal = refused(item, candidate, table(str(single)))
                self.assertEqual(listed, [])
                self.assertEqual([entry["path"] for entry in skipped], [str(single)])
                self.assertIsNotNone(refusal)
            with self.subTest("an item inside a bind mount of another directory"):
                # The mount's root (field 4) is a directory of the boot disk mounted at an
                # ancestor of the item: the item is reached at one path and lives at another.
                item, candidate = cases[venv]
                point = str(host.apps / "idle")
                bound = "\n".join(
                    [
                        "20 1 8:1 / / rw - ext4 /dev/x rw",
                        f"31 20 8:1 /srv/kept {point} rw,relatime shared:1 - ext4 /dev/x rw",
                    ]
                )
                listed, skipped, refusal = refused(item, candidate, bound + "\n")
                self.assertEqual(listed, [])
                self.assertEqual([entry["path"] for entry in skipped], [venv])
                self.assertIn("lies inside the bind mount " + point, skipped[0]["reason"])
                self.assertIsNotNone(refusal)
                self.assertIn("lies inside the bind mount " + point, refusal)
                # A mount whose root is `/` shows the whole filesystem: nothing is bound.
                whole = bound.replace("/srv/kept", "/")
                listed, skipped, refusal = refused(item, candidate, whole + "\n")
                self.assertEqual([entry["path"] for entry in listed], [venv], skipped)
                self.assertIsNone(refusal)
            with self.subTest("a mount table that cannot be read refuses the item"):
                item, candidate = cases[venv]

                def unreadable():
                    raise PermissionError(13, "Permission denied", "/proc/self/mountinfo")

                plan_tool.read_mountinfo = unreadable
                try:
                    skipped = []
                    listed = plan_tool.items_of([candidate], skipped)
                    with self.assertRaises(apply_tool.Refusal) as raised:
                        apply_tool.check_item(item, [], apply_tool.Runner())
                finally:
                    plan_tool.read_mountinfo = real_reader
                self.assertEqual(listed, [])
                self.assertEqual([entry["path"] for entry in skipped], [venv])
                self.assertIn("mount table", skipped[0]["reason"])
                self.assertIn("mount table", str(raised.exception))
            with self.subTest("the same items are listed and pass when nothing is mounted"):
                for path, (item, candidate) in cases.items():
                    listed, skipped, refusal = refused(item, candidate, table())
                    self.assertEqual([entry["path"] for entry in listed], [path], skipped)
                    self.assertIsNone(refusal)


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
                        "argv": ["systemctl", "is-active", "--quiet", "--", "example.service"],
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
        # The example is a whole rules file, which the tools' own reader accepts.
        done = subprocess.run(
            [
                sys.executable,
                "-c",
                "import sys, inventory; r = inventory.load_rules(sys.argv[1]); "
                "print(sorted({x['class'] for x in r['rules']}), len(r['health']))",
                str(EXAMPLE_RULES),
            ],
            cwd=TOOLS,
            capture_output=True,
            text=True,
            check=False,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("['backup', 'environment', 'loose', 'package'] 2", done.stdout)
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
