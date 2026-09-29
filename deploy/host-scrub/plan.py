#!/usr/bin/env python3
"""plan: turn an inventory and the rules into a deletion list, one item per path (or per listed
package), each with the rule and the reason that selected it, its bytes and its digest (SPEC-060
R3, R4, R10; ADR-060).

    python3 plan.py INVENTORY RULES --out FILE

The candidates are the stale copies of each backup family and the loose files each rule matched,
as the inventory recorded them; the virtual environments and worktrees under a rule's directories
that no loaded unit's commands or working directory name; and the installed packages the owner
lists. Nothing under a protected path, or holding one, is ever listed, and neither is a path
that is not absolute and canonical, nor an item inside another listed item. The inventory already
kept back the newest copies a service's own rotation would make again.

A file's digest is SHA-256 over `path NUL size NUL mtime_ns NUL mode NUL content`, where `mode`
is `st_mode` in octal and `content` the hex SHA-256 of its bytes (of its target, for a symbolic
link, which is never followed; empty for anything else). A directory's digest is SHA-256 over the
sorted lines of every entry under it, itself included, joined by newlines. A package's digest is
SHA-256 over its name, architecture, version and state. An item's `bytes` are what it frees when
the whole list is applied: a file whose other links survive frees nothing, and a file linked from
two items counts once. The list carries the total, the inventory's instant and digest, and its own
digest, SHA-256 over the list as canonical JSON without its `digest` field. The inventory and the
rules are each read once, and each digest the plan checks or records is taken over the bytes it
parsed.

The plan reads each candidate's content to digest it, so it runs where the candidates are. It
deletes nothing and writes only its output, which is private and lies outside this repository
(R8). Exit 0 when written, 1 when refused (the rules are not the ones the inventory read), 2 on a
usage error.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import sys
from collections import Counter
from pathlib import Path

from inventory import (
    Usage,
    canonical,
    inside_repository,
    load_rules,
    now_utc,
    read_json,
    sha256_file,
    walk,
)

SCHEMA = "deck-streak-host-scrub-list/1"
#: The kernel's table of mounts, whose fifth field is a mount point.
MOUNTINFO = "/proc/self/mountinfo"
#: The order classes are listed in, which gives each item its id.
ORDER = ("backup", "loose", "venv", "worktree", "package")
#: A loaded unit's settings that name what it runs, and the directory it runs in.
COMMANDS = (
    "ExecCondition",
    "ExecStartPre",
    "ExecStart",
    "ExecStartPost",
    "ExecReload",
    "ExecStop",
    "ExecStopPost",
    "WorkingDirectory",
)


def within(child: str, parent: str) -> bool:
    return child == parent or child.startswith(parent.rstrip("/") + "/")


def conflict(path: str, protected: list[str]) -> str | None:
    """The protected path `path` lies under or holds, or None (R7, R10). A path that is not
    absolute and canonical is never compared: it raises ValueError. Each protected path is compared
    as written and as it resolves, so a protected link protects what it points at."""
    if not canonical(path):
        raise ValueError("a path that is not absolute and canonical is never compared")
    for guard in protected:
        for form in {guard, os.path.realpath(guard)}:
            if within(path, form) or within(form, path):
                return guard
    return None


def read_mountinfo() -> str:
    """The kernel's table of mounts, through one seam so a test can fake it (R7)."""
    return Path(MOUNTINFO).read_text(encoding="utf-8", errors="surrogateescape")


def unescape(field: str) -> str:
    """A mount table field with its octal escapes (`\\040` for a space) read as the bytes they
    name."""
    raw = re.sub(rb"\\([0-7]{3})", lambda m: bytes([int(m.group(1), 8) & 0xFF]), os.fsencode(field))
    return os.fsdecode(raw)


def mounted(path: str) -> str | None:
    """The mount point `path` is or holds, or None. A bind mount shares its device with the tree
    around it, so the device check cannot see it; the mount table can (R7). An item inside a mount
    whose root (field 4) is not `/` is refused too: it is a bind of some other directory, whose
    protection an operator's list would have to know. Raises OSError when the table cannot be read,
    since an item is never judged clear of mounts without it."""
    rows = read_mountinfo().splitlines()
    # A file system mounted whole (its root `/`) at two points shows each at the other: a bind of
    # its root directory, `/` or a protected directory that is itself a mount point, reads `/`.
    whole = Counter(
        fields[2]
        for fields in (row.split() for row in rows)
        if len(fields) >= 5 and unescape(fields[3]) == "/"
    )
    for row in rows:
        fields = row.split()
        if len(fields) < 5:
            continue
        point = unescape(fields[4])
        if point == path:
            return point
        if point.startswith(path.rstrip("/") + "/"):
            return point
        if unescape(fields[3]) != "/" and path.startswith(point.rstrip("/") + "/"):
            return point
        if whole[fields[2]] > 1 and path.startswith(point.rstrip("/") + "/"):
            return point
    return None


def mount_reason(path: str, point: str) -> str:
    """Why `path` may not be judged against the mount point `point`. A mount point shorter than the
    path can only lie above it, and a mount above an item whose root is not `/` shows another
    directory of the disk at that place, so the item's own path is not where it lives (R7)."""
    if len(point) < len(path):
        return f"lies inside the bind mount {point}"
    return f"is or holds the mount point {point}"


def names(text: str, path: str) -> bool:
    """Whether a unit's setting `text` names `path`, or anything under it, as a whole path."""
    return re.search(r"(?<![\w.])" + re.escape(path) + r"(?=$|[/\s;\"'])", text) is not None


def unit_settings(inventory: dict) -> list[str]:
    return [
        unit["settings"].get(key, "")
        for unit in inventory.get("units", [])
        if unit.get("load") == "loaded"
        for key in COMMANDS
    ]


def line(path: str, st: os.stat_result) -> bytes:
    """One entry's digest line (R4)."""
    if stat.S_ISREG(st.st_mode):
        content = sha256_file(path)
    elif stat.S_ISLNK(st.st_mode):
        content = hashlib.sha256(os.fsencode(os.readlink(path))).hexdigest()
    else:
        content = ""
    fields = [path, str(st.st_size), str(st.st_mtime_ns), format(st.st_mode, "o"), content]
    return b"\0".join(os.fsencode(field) for field in fields)


def measure(path: str) -> tuple[str, list[tuple]]:
    """The item's digest (R4), and each entry's inode, link count and bytes on disk. An entry that
    cannot be read makes the item unmeasurable, never half-digested."""
    errors: list[dict] = []
    lines, inodes = [], []
    for sub, st in walk(path, errors):
        if inodes and st.st_dev != inodes[0][0][0]:
            raise OSError(f"{sub} lies on another device than the item, so it is not digested")
        lines.append(line(sub, st))
        directory = stat.S_ISDIR(st.st_mode)
        inodes.append(((st.st_dev, st.st_ino), st.st_nlink, st.st_blocks * 512, directory))
    if errors or not lines:
        raise OSError(f"cannot read {errors[0]['path'] if errors else path} to digest it")
    if len(lines) == 1 and not inodes[0][3]:
        return hashlib.sha256(lines[0]).hexdigest(), inodes
    return hashlib.sha256(b"\n".join(sorted(lines))).hexdigest(), inodes


def package_digest(package: dict) -> str:
    fields = [package["name"], package["architecture"], package["version"], package["status"]]
    return hashlib.sha256("\0".join(fields).encode()).hexdigest()


def list_digest(listing: dict) -> str:
    """SHA-256 over the list as canonical JSON, its own `digest` field left out."""
    body = {key: value for key, value in listing.items() if key != "digest"}
    text = json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode()).hexdigest()


def candidates(inventory: dict, rules: dict) -> list[dict]:
    """Every (class, rule, target) the rules select from the inventory, before exclusions."""
    by_name = {rule["name"]: rule for rule in rules["rules"]}
    found = []
    for family in inventory.get("backups", []):
        for path in family["stale"]:
            found.append({"class": "backup", "rule": by_name[family["rule"]], "path": path})
    for group in inventory.get("loose", []):
        for match in group["matches"]:
            found.append({"class": "loose", "rule": by_name[group["rule"]], "path": match["path"]})
    settings = unit_settings(inventory)
    environments = [("venv", e) for e in inventory.get("venvs", [])]
    environments += [("worktree", e) for e in inventory.get("worktrees", [])]
    for rule in (r for r in rules["rules"] if r["class"] == "environment"):
        for kind, environment in environments:
            path = environment["path"]
            if any(within(path, top) for top in rule["under"]) and not any(
                names(text, path) for text in settings
            ):
                found.append({"class": kind, "rule": rule, "path": path})
    installed = [p for p in inventory.get("packages", []) if p["status"] == "ii"]
    for rule in (r for r in rules["rules"] if r["class"] == "package"):
        for package in installed:
            if rule["package"] in (package["name"], f"{package['name']}:{package['architecture']}"):
                found.append({"class": "package", "rule": rule, "package": package})
    return found


def select(found: list[dict], protected: list[str], skipped: list[dict]) -> list[dict]:
    """Drop a path that is not absolute and canonical, what a protected path covers, a path
    selected twice, and a path inside another."""
    kept, paths = [], set()
    for candidate in found:
        path = candidate.get("path")
        if path is None:
            kept.append(candidate)
            continue
        try:
            guard = conflict(path, protected)
        except ValueError:
            skipped.append({"path": path, "reason": "not an absolute, canonical path"})
            continue
        if guard is not None:
            skipped.append({"path": path, "reason": f"under or holding protected {guard}"})
        elif path not in paths:
            paths.add(path)
            kept.append(candidate)
    outermost = [
        c
        for c in kept
        if c.get("path") is None or not any(p != c["path"] and within(c["path"], p) for p in paths)
    ]
    return outermost


def items_of(chosen: list[dict], skipped: list[dict]) -> list[dict]:
    chosen.sort(
        key=lambda c: (
            ORDER.index(c["class"]),
            c.get("path") or f"{c['package']['name']}:{c['package']['architecture']}",
        )
    )
    items = []
    for candidate in chosen:
        rule = candidate["rule"]
        item = {"class": candidate["class"], "rule": rule["name"], "reason": rule["reason"]}
        if candidate["class"] == "package":
            package = candidate["package"]
            item["package"] = f"{package['name']}:{package['architecture']}"
            item["version"] = package["version"]
            item["bytes"] = (package["installed_kib"] or 0) * 1024
            item["digest"] = package_digest(package)
            item["inodes"] = []
        else:
            path = candidate["path"]
            try:
                point = mounted(path)
            except OSError as error:
                skipped.append({"path": path, "reason": f"the mount table cannot be read: {error}"})
                continue
            if point is not None:
                skipped.append({"path": path, "reason": mount_reason(path, point)})
                continue
            try:
                digest, inodes = measure(path)
            except OSError as error:
                skipped.append({"path": path, "reason": str(error)})
                continue
            item.update({"path": path, "digest": digest, "inodes": inodes})
        items.append(item)
    reclaim(items)
    for number, item in enumerate(items, start=1):
        item["id"] = f"i{number:03d}"
        del item["inodes"]
    return items


def reclaim(items: list[dict]) -> None:
    """Each item's bytes: what it frees when the whole list is applied (R4)."""
    held = Counter(key for item in items for key, *_ in item["inodes"])
    counted = set()
    for item in items:
        if item["class"] == "package":
            continue
        freed = 0
        for key, links, size, directory in item["inodes"]:
            if key not in counted and (directory or held[key] >= links):
                counted.add(key)
                freed += size
        item["bytes"] = freed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("inventory", help="the inventory file")
    parser.add_argument("rules", help="the private rules file the inventory read")
    parser.add_argument("--out", required=True, help="the list file, outside this repository")
    args = parser.parse_args(argv)
    if inside_repository(args.out):
        print(f"plan: {args.out} is inside this repository; write it to the private directory")
        return 2
    try:
        inventory, inventory_digest = read_json(args.inventory, "inventory")
        rules = load_rules(args.rules)
    except Usage as error:
        print(f"plan: {error}")
        return 2
    if inventory.get("rules_digest") != rules["digest"]:
        print("plan: refused: these rules are not the ones the inventory read; run it again")
        return 1
    os.nice(max(0, 19 - os.nice(0)))
    skipped: list[dict] = []
    chosen = select(candidates(inventory, rules), rules["protected"], skipped)
    items = items_of(chosen, skipped)
    listing = {
        "schema": SCHEMA,
        "made_at": now_utc().isoformat(),
        "inventory": {
            "digest": inventory_digest,
            "taken_at": inventory["taken_at"],
            "clock_synchronised": inventory.get("clock_synchronised"),
        },
        "rules_digest": inventory["rules_digest"],
        "items": items,
        "skipped": skipped,
        "bytes": sum(item["bytes"] for item in items),
    }
    listing["digest"] = list_digest(listing)
    out = Path(args.out)
    partial = out.with_name(out.name + ".partial")
    partial.write_text(json.dumps(listing, indent=1) + "\n", encoding="utf-8")
    os.replace(partial, out)
    print(
        f"plan: examined {len(chosen)} candidate(s); listed {len(items)} item(s) reclaiming "
        f"{listing['bytes']} bytes, {len(skipped)} skipped; list digest {listing['digest']}; "
        f"wrote {out}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
