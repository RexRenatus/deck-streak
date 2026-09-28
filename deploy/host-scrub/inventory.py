#!/usr/bin/env python3
"""inventory: read a host into one JSON file, running only the read commands of its own allow list
(SPEC-060 R1, R2, R9; ADR-060).

    python3 inventory.py RULES --out FILE

RULES is the private rules file; `rules.example.json` beside this file shows its shape with neutral
values. The inventory records each mount's used and free space; for every root the rules name,
each entry's size, blocks on disk, link count, modification time, owner and mode, with a
hard-linked file counted once in every total; the stale copies of each backup family and the loose
files each rule matches; the virtual environments and git worktrees under the roots; the installed
packages; the unit files, and the loaded units with the settings that say what they run, from
where, as whom, what they may write and when they fire; and the ten largest memory users. It reads
every health check of the rules before its first read, and it records every command it ran, with
its exit status.

It changes nothing. Every command is checked against the read-only allow list below before the
first one runs, so a command outside it refuses the whole run and nothing runs at all; each runs
under `nice -n 19` and `ionice -c3`. A virtual environment or worktree is one entry with its own
totals, never listed file by file, and a root marked `sizes_only` gives its totals and no entries.
The output is private (R8): it must lie outside this repository.

`plan.py` and `apply.py` import this file's allow list, runner, walk, rules and health reads, so
each rule has one implementation. Exit 0 when written, 1 when a command was refused, 2 on a usage
error.
"""

from __future__ import annotations

import argparse
import fnmatch
import grp
import hashlib
import json
import os
import pwd
import re
import stat
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path

SCHEMA = "deck-streak-host-inventory/1"
RULES_SCHEMA = "deck-streak-host-scrub-rules/1"
#: Every command runs at the lowest processor priority and in the idle IO class (R2).
NICE = ["nice", "-n", "19", "ionice", "-c3"]
COMMAND_TIMEOUT = 300
HEALTH_TIMEOUT = 10
UNIT = re.compile(
    r"^[A-Za-z0-9:_.@\\-]+\.(?:service|socket|timer|target|path|mount|automount|swap|slice|scope"
    r"|device)$"
)
PACKAGE = re.compile(r"^[a-z0-9][a-z0-9+.-]+(?::[a-z0-9-]+)?$")
#: The loaded units whose settings are read: what runs, and what fires it.
SHOWN = (".service", ".socket", ".timer")
#: The settings read for each such unit. Never `Environment=`, which can carry a secret.
PROPERTIES = (
    "Id",
    "LoadState",
    "ActiveState",
    "SubState",
    "UnitFileState",
    "FragmentPath",
    "User",
    "Group",
    "WorkingDirectory",
    "ExecCondition",
    "ExecStartPre",
    "ExecStart",
    "ExecStartPost",
    "ExecReload",
    "ExecStop",
    "ExecStopPost",
    "ReadWritePaths",
    "ReadOnlyPaths",
    "InaccessiblePaths",
    "ProtectSystem",
    "ProtectHome",
    "TimersCalendar",
    "Triggers",
    "TriggeredBy",
)
SHOW = ["systemctl", "show", "--no-pager", "--property=" + ",".join(PROPERTIES), "--"]
PACKAGE_FORMAT = (
    "--showformat=${Package}\\t${Architecture}\\t${Version}\\t${Installed-Size}"
    "\\t${db:Status-Abbrev}\\n"
)
DPKG_QUERY = ["dpkg-query", "-W", PACKAGE_FORMAT]
#: The inventory's own reads, each an exact argument vector (R1).
READS = {
    "unit_files": ["systemctl", "list-unit-files", "--no-legend", "--no-pager"],
    "units": ["systemctl", "list-units", "--all", "--no-legend", "--no-pager", "--plain"],
    "packages": DPKG_QUERY,
    "memory": ["ps", "-eo", "pid=,uid=,rss=,comm=", "--sort=-rss"],
}
MEMORY_USERS = 10
#: File systems that hold no files of their own.
PSEUDO = frozenset(
    {
        "autofs",
        "binfmt_misc",
        "bpf",
        "cgroup",
        "cgroup2",
        "configfs",
        "debugfs",
        "devpts",
        "efivarfs",
        "fusectl",
        "hugetlbfs",
        "mqueue",
        "nsfs",
        "proc",
        "pstore",
        "rpc_pipefs",
        "securityfs",
        "squashfs",
        "sysfs",
        "tracefs",
    }
)
RULE_CLASSES = ("backup", "environment", "loose", "package")


class Usage(Exception):
    """A file the tool cannot use: missing, unreadable, malformed, or in the wrong place."""


class Refused(Exception):
    """A command outside the allow list, refused before it runs (R2)."""

    def __init__(self, argv: list[str]):
        super().__init__(" ".join(argv))
        self.argv = argv


def allowed(argv: list[str]) -> bool:
    """Whether `argv` is one of the read commands: the inventory's own reads, `systemctl show` and
    `systemctl is-active` of named units, and `dpkg-query` of one named package (R2)."""
    if argv in READS.values():
        return True
    if argv[: len(SHOW)] == SHOW:
        units = argv[len(SHOW) :]
        return bool(units) and all(UNIT.match(unit) for unit in units)
    if argv[:2] == ["systemctl", "is-active"]:
        units = argv[3:] if argv[2:3] == ["--quiet"] else argv[2:]
        return bool(units) and all(UNIT.match(unit) for unit in units)
    if argv[: len(DPKG_QUERY)] == DPKG_QUERY and len(argv) == len(DPKG_QUERY) + 1:
        return bool(PACKAGE.match(argv[-1]))
    return False


class Runner:
    """Runs a command only when `allow` admits it, under `nice` and `ionice -c3`, and records every
    command it ran with its exit status (R1, R2)."""

    def __init__(self, allow=allowed):
        self.allow = allow
        self.ran: list[dict] = []

    def check(self, commands: list[list[str]]) -> None:
        """Refuse the first command `allow` does not admit, before any command runs."""
        for argv in commands:
            if not self.allow(argv):
                raise Refused(argv)

    def run(self, argv: list[str]) -> tuple[int, str]:
        self.check([argv])
        wrapped = NICE + argv
        try:
            done = subprocess.run(
                wrapped,
                capture_output=True,
                text=True,
                errors="replace",
                check=False,
                timeout=COMMAND_TIMEOUT,
                env=dict(os.environ, LC_ALL="C"),
            )
            code, output = done.returncode, done.stdout
        except FileNotFoundError:
            code, output = 127, ""
        except subprocess.TimeoutExpired:
            code, output = 124, ""
        self.ran.append({"argv": wrapped, "exit": code})
        return code, output


def now_utc() -> datetime:
    return datetime.now(timezone.utc)


def iso(timestamp: float) -> str:
    return datetime.fromtimestamp(timestamp, timezone.utc).isoformat()


def sha256_file(path: str) -> str:
    """The SHA-256 of a regular file's bytes, opened without following a symbolic link."""
    digest = hashlib.sha256()
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def inside_repository(path: str) -> bool:
    """Whether `path` lies inside the repository these tools were run from, whose tree is public:
    the inventory, the lists, the approvals and the logs are private (R8)."""
    repository = Path(__file__).resolve().parents[2]
    if not (repository / ".git").exists():
        return False
    target = Path(path).resolve()
    return target == repository or repository in target.parents


def load_json(path: str, what: str):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle)
    except OSError as error:
        raise Usage(f"cannot read the {what} at {path}: {error.strerror}") from error
    except ValueError as error:
        raise Usage(f"the {what} at {path} is not JSON: {error}") from error


def file_digest(path: str) -> str:
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()


def absolute(value, where: str) -> str:
    if not isinstance(value, str) or not os.path.isabs(value):
        raise Usage(f"{where} must be an absolute path")
    return os.path.normpath(value)


def load_rules(path: str) -> dict:
    """The rules, checked for their shape: roots, rules by class, protected paths, health checks
    (R3, R7, R9). A health check is a read command of the allow list, or an http(s) address."""
    rules = load_json(path, "rules")
    if not isinstance(rules, dict) or rules.get("schema") != RULES_SCHEMA:
        raise Usage(f"the rules at {path} do not declare schema {RULES_SCHEMA}")
    roots = []
    for n, root in enumerate(rules.get("roots") or []):
        roots.append(
            {
                "path": absolute(root.get("path") if isinstance(root, dict) else None, f"root {n}"),
                "sizes_only": bool(root.get("sizes_only", False)),
            }
        )
    if not roots:
        raise Usage("the rules name no root")
    names, checked = set(), []
    for rule in rules.get("rules") or []:
        name = rule.get("name") if isinstance(rule, dict) else None
        if not isinstance(name, str) or not name or name in names:
            raise Usage(f"a rule has no name, or a name used twice: {name!r}")
        names.add(name)
        kind, reason = rule.get("class"), rule.get("reason")
        if kind not in RULE_CLASSES or not isinstance(reason, str) or not reason.strip():
            raise Usage(f"rule {name} needs a class of {RULE_CLASSES} and a reason")
        checked.append(check_rule(dict(rule), name, kind))
    protected = [absolute(p, "a protected path") for p in rules.get("protected") or []]
    health = [check_health(check) for check in rules.get("health") or []]
    ids = [check["id"] for check in health]
    if len(set(ids)) != len(ids):
        raise Usage("two health checks share an id")
    return {"roots": roots, "rules": checked, "protected": protected, "health": health}


def check_rule(rule: dict, name: str, kind: str) -> dict:
    if kind in ("backup", "loose"):
        rule["dir"] = absolute(rule.get("dir"), f"rule {name}'s dir")
        if not isinstance(rule.get("pattern"), str) or not rule["pattern"]:
            raise Usage(f"rule {name} needs a file-name pattern")
    if kind == "backup":
        kept = [key for key in ("keep_copies", "keep_days") if key in rule]
        counts = [rule.get(key) for key in (*kept, "rotation_keeps") if key in rule]
        if len(kept) != 1 or not all(isinstance(c, int) and c >= 0 for c in counts):
            raise Usage(f"rule {name} keeps copies or days, one of them, as a whole number")
    if kind == "environment":
        under = rule.get("under")
        if not isinstance(under, list) or not under:
            raise Usage(f"rule {name} names no directory to look under")
        rule["under"] = [absolute(p, f"rule {name}'s under") for p in under]
    if kind == "package" and not PACKAGE.match(str(rule.get("package", ""))):
        raise Usage(f"rule {name} names no package")
    return rule


def check_health(check) -> dict:
    if not isinstance(check, dict) or not isinstance(check.get("id"), str) or not check["id"]:
        raise Usage("a health check has no id")
    if "argv" in check:
        argv = check["argv"]
        if not isinstance(argv, list) or not argv or not all(isinstance(a, str) for a in argv):
            raise Usage(f"health check {check['id']}'s argv is not a list of words")
        return {"id": check["id"], "argv": list(argv)}
    url = check.get("url")
    if not isinstance(url, str) or urllib.parse.urlsplit(url).scheme not in ("http", "https"):
        raise Usage(f"health check {check['id']} needs an argv or an http(s) url")
    return {"id": check["id"], "url": url}


def health_commands(checks: list[dict]) -> list[list[str]]:
    return [check["argv"] for check in checks if "argv" in check]


def read_health(checks: list[dict], runner: Runner) -> list[dict]:
    """Each health check's verdict, in the order the rules give them (R9)."""
    results = []
    for check in checks:
        if "argv" in check:
            code, _ = runner.run(check["argv"])
            green, detail = code == 0, f"exit {code}"
        else:
            green, detail = probe(check["url"])
        results.append({"id": check["id"], "green": green, "detail": detail})
    return results


def probe(url: str) -> tuple[bool, str]:
    """One GET, never through a proxy the environment names: green on a 2xx answer."""
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(url, timeout=HEALTH_TIMEOUT) as response:
            return 200 <= response.status < 300, f"HTTP {response.status}"
    except urllib.error.HTTPError as error:
        return False, f"HTTP {error.code}"
    except (urllib.error.URLError, OSError, ValueError) as error:
        return False, f"unreachable: {type(error).__name__}"


def walk(top: str, errors: list[dict], descend=None):
    """`top` and every entry under it, as (path, lstat), never following a symbolic link. A
    directory is entered only when `descend(path)` allows it; an unreadable entry is recorded in
    `errors` and skipped."""
    try:
        st = os.lstat(top)
    except OSError as error:
        errors.append({"path": top, "error": error.strerror})
        return
    yield top, st
    stack = []
    if stat.S_ISDIR(st.st_mode) and (descend is None or descend(top)):
        stack.append(top)
    while stack:
        directory = stack.pop()
        try:
            with os.scandir(directory) as scan:
                children = sorted(scan, key=lambda child: child.name, reverse=True)
        except OSError as error:
            errors.append({"path": directory, "error": error.strerror})
            continue
        for child in children:
            try:
                child_st = child.stat(follow_symlinks=False)
            except OSError as error:
                errors.append({"path": child.path, "error": error.strerror})
                continue
            yield child.path, child_st
            if stat.S_ISDIR(child_st.st_mode) and (descend is None or descend(child.path)):
                stack.append(child.path)


class Totals:
    """Bytes under a path, a hard-linked file counted once (R1): `size` is the regular files'
    apparent size, `bytes` the blocks on disk of every entry."""

    def __init__(self):
        self.entries = 0
        self.size = 0
        self.bytes = 0
        self.linked: set[tuple[int, int]] = set()

    def add(self, st: os.stat_result) -> None:
        self.entries += 1
        if not stat.S_ISDIR(st.st_mode) and st.st_nlink > 1:
            key = (st.st_dev, st.st_ino)
            if key in self.linked:
                return
            self.linked.add(key)
        if stat.S_ISREG(st.st_mode):
            self.size += st.st_size
        self.bytes += st.st_blocks * 512

    def summary(self) -> dict:
        return {
            "entries": self.entries,
            "size": self.size,
            "bytes": self.bytes,
            "hard_linked": len(self.linked),
        }


_names: dict[tuple[str, int], str] = {}


def name_of(kind: str, number: int) -> str:
    key = (kind, number)
    if key not in _names:
        try:
            found = pwd.getpwuid(number).pw_name if kind == "user" else grp.getgrgid(number).gr_name
        except KeyError:
            found = str(number)
        _names[key] = found
    return _names[key]


def type_of(mode: int) -> str:
    if stat.S_ISREG(mode):
        return "file"
    if stat.S_ISDIR(mode):
        return "dir"
    if stat.S_ISLNK(mode):
        return "link"
    return "other"


def entry(path: str, st: os.stat_result) -> dict:
    return {
        "path": path,
        "type": type_of(st.st_mode),
        "size": st.st_size,
        "bytes": st.st_blocks * 512,
        "nlink": st.st_nlink,
        "mtime": iso(st.st_mtime),
        "owner": name_of("user", st.st_uid),
        "group": name_of("group", st.st_gid),
        "mode": f"{stat.S_IMODE(st.st_mode):04o}",
    }


def environment_kind(directory: str) -> str | None:
    """`venv` for a virtual environment (PEP 405's `pyvenv.cfg`), `main` or `linked` for a git
    worktree (a `.git` directory, or the `.git` file of a linked worktree), else None."""
    if os.path.isfile(os.path.join(directory, "pyvenv.cfg")):
        return "venv"
    marker = os.path.join(directory, ".git")
    if os.path.islink(marker) or not os.path.lexists(marker):
        return None
    return "main" if os.path.isdir(marker) else "linked"


def space(vfs: os.statvfs_result) -> dict:
    return {
        "total": vfs.f_blocks * vfs.f_frsize,
        "used": (vfs.f_blocks - vfs.f_bfree) * vfs.f_frsize,
        "free": vfs.f_bfree * vfs.f_frsize,
        "available": vfs.f_bavail * vfs.f_frsize,
    }


def mounts() -> list[dict]:
    """Each mounted file system that holds files, with its used and free space (R1)."""
    try:
        text = Path("/proc/self/mounts").read_text(encoding="utf-8", errors="replace")
    except OSError:
        return []
    found, seen = [], set()
    for line in text.splitlines():
        fields = line.split()
        if len(fields) < 3 or fields[2] in PSEUDO:
            continue
        point = re.sub(r"\\([0-7]{3})", lambda m: chr(int(m.group(1), 8)), fields[1])
        if point in seen:
            continue
        seen.add(point)
        try:
            vfs, device = os.statvfs(point), os.stat(point).st_dev
        except OSError:
            continue
        if vfs.f_blocks:
            found.append({"path": point, "fstype": fields[2], "device": device, **space(vfs)})
    return found


def read_roots(rules: dict, errors: list[dict]) -> dict:
    grand = Totals()
    found: dict = {"roots": [], "entries": [], "venvs": [], "worktrees": []}
    for root in rules["roots"]:
        totals = Totals()

        def descend(directory: str, totals: Totals = totals) -> bool:
            kind = environment_kind(directory)
            if kind is None:
                return True
            own = Totals()
            for path, st in walk(directory, errors):
                own.add(st)
                if path != directory:
                    totals.add(st)
                    grand.add(st)
            record = {"path": directory, **own.summary()}
            if kind == "venv":
                found["venvs"].append(record)
            else:
                found["worktrees"].append({**record, "kind": kind})
            return False

        for path, st in walk(root["path"], errors, descend):
            totals.add(st)
            grand.add(st)
            if not root["sizes_only"]:
                found["entries"].append(entry(path, st))
        try:
            measured = space(os.statvfs(root["path"]))
        except OSError:
            measured = None
        found["roots"].append(
            {**root, "exists": os.path.lexists(root["path"]), "space": measured, **totals.summary()}
        )
    found["total"] = grand.summary()
    return found


def listed(directory: str, pattern: str, keep, errors: list[dict]) -> list[dict]:
    """The entries of `directory` whose names match `pattern` and whose lstat `keep` admits."""
    try:
        names = sorted(os.listdir(directory))
    except OSError as error:
        errors.append({"path": directory, "error": error.strerror})
        return []
    matches = []
    for name in names:
        if fnmatch.fnmatchcase(name, pattern):
            path = os.path.join(directory, name)
            try:
                st = os.lstat(path)
            except OSError as error:
                errors.append({"path": path, "error": error.strerror})
                continue
            if keep(st.st_mode):
                matches.append({**entry(path, st), "mtime_ns": st.st_mtime_ns})
    return matches


def stale_copies(rule: dict, taken: datetime, errors: list[dict]) -> dict:
    """A backup family's copies, newest first, and those beyond its retention. The newest
    `rotation_keeps` copies are the service's own rotation, which would make them again, so none
    of them is ever stale (R10)."""
    copies = listed(rule["dir"], rule["pattern"], stat.S_ISREG, errors)
    copies.sort(key=lambda copy: (copy["mtime_ns"], copy["path"]), reverse=True)
    rotation = rule.get("rotation_keeps", 0)
    if "keep_copies" in rule:
        stale = copies[max(rule["keep_copies"], rotation) :]
    else:
        cutoff = (taken - timedelta(days=rule["keep_days"])).timestamp() * 1e9
        stale = [copy for copy in copies[rotation:] if copy["mtime_ns"] < cutoff]
    return {
        "rule": rule["name"],
        "dir": rule["dir"],
        "copies": copies,
        "stale": [copy["path"] for copy in stale],
    }


def loose_files(rule: dict, errors: list[dict]) -> dict:
    keep = lambda mode: stat.S_ISREG(mode) or stat.S_ISLNK(mode)  # noqa: E731
    return {
        "rule": rule["name"],
        "dir": rule["dir"],
        "matches": listed(rule["dir"], rule["pattern"], keep, errors),
    }


def parse_units(text: str) -> list[dict]:
    units = []
    for line in text.splitlines():
        fields = line.split(None, 4)
        if len(fields) >= 4:
            unit, load, active, sub = fields[:4]
            description = fields[4] if len(fields) > 4 else ""
            units.append(
                {
                    "unit": unit,
                    "load": load,
                    "active": active,
                    "sub": sub,
                    "description": description,
                }
            )
    return units


def parse_unit_files(text: str) -> list[dict]:
    rows = []
    for line in text.splitlines():
        fields = line.split()
        if len(fields) >= 2:
            rows.append(
                {"unit": fields[0], "state": fields[1], "preset": fields[2] if fields[2:] else ""}
            )
    return rows


def parse_show(text: str) -> dict[str, dict]:
    blocks, current = {}, {}
    for line in [*text.splitlines(), ""]:
        if not line.strip():
            if current:
                blocks[current.get("Id", "")] = current
            current = {}
            continue
        key, separator, value = line.partition("=")
        if separator:
            current[key] = value
    return blocks


def parse_packages(text: str) -> list[dict]:
    packages = []
    for line in text.splitlines():
        fields = line.split("\t")
        if len(fields) >= 5:
            name, architecture, version, size, status = fields[:5]
            packages.append(
                {
                    "name": name,
                    "architecture": architecture,
                    "version": version,
                    "installed_kib": int(size) if size.strip().isdigit() else None,
                    "status": status.strip(),
                }
            )
    return packages


def parse_memory(text: str) -> list[dict]:
    users = []
    for line in text.splitlines():
        fields = line.split(None, 3)
        if len(fields) == 4 and fields[0].isdigit() and fields[1].isdigit():
            users.append(
                {
                    "pid": int(fields[0]),
                    "user": name_of("user", int(fields[1])),
                    "rss_kib": int(fields[2]) if fields[2].isdigit() else None,
                    "command": fields[3],
                }
            )
    return users[:MEMORY_USERS]


def read_units(runner: Runner) -> dict:
    _, files = runner.run(READS["unit_files"])
    _, loaded = runner.run(READS["units"])
    units = parse_units(loaded)
    shown = sorted(
        u["unit"]
        for u in units
        if u["load"] == "loaded" and u["unit"].endswith(SHOWN) and UNIT.match(u["unit"])
    )
    settings = parse_show(runner.run(SHOW + shown)[1]) if shown else {}
    for unit in units:
        unit["settings"] = settings.get(unit["unit"], {})
    return {"unit_files": parse_unit_files(files), "units": units}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("rules", help="the private rules file")
    parser.add_argument("--out", required=True, help="the inventory file, outside this repository")
    args = parser.parse_args(argv)
    if inside_repository(args.out):
        print(f"inventory: {args.out} is inside this repository; write it to the private directory")
        return 2
    try:
        rules = load_rules(args.rules)
    except Usage as error:
        print(f"inventory: {error}")
        return 2
    runner = Runner()
    try:
        runner.check(health_commands(rules["health"]))
    except Refused as refused:
        print(f"inventory: refused: `{refused}` is not on the read-only allow list; nothing ran")
        return 1
    os.nice(max(0, 19 - os.nice(0)))
    taken = now_utc()
    health = read_health(rules["health"], runner)
    errors: list[dict] = []
    found = read_roots(rules, errors)
    by_class = {kind: [r for r in rules["rules"] if r["class"] == kind] for kind in RULE_CLASSES}
    record = {
        "schema": SCHEMA,
        "taken_at": taken.isoformat(),
        "rules_digest": file_digest(args.rules),
        "health_before": health,
        "mounts": mounts(),
        **found,
        "backups": [stale_copies(rule, taken, errors) for rule in by_class["backup"]],
        "loose": [loose_files(rule, errors) for rule in by_class["loose"]],
        **read_units(runner),
        "packages": parse_packages(runner.run(READS["packages"])[1]),
        "memory": parse_memory(runner.run(READS["memory"])[1]),
        "commands": runner.ran,
        "errors": errors,
    }
    out = Path(args.out)
    partial = out.with_name(out.name + ".partial")
    partial.write_text(json.dumps(record, indent=1) + "\n", encoding="utf-8")
    os.replace(partial, out)
    print(
        f"inventory: examined {found['total']['entries']} entries under {len(rules['roots'])} "
        f"root(s) and ran {len(runner.ran)} command(s); {len(errors)} unreadable; wrote {out}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
