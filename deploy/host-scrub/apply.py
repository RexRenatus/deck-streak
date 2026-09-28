#!/usr/bin/env python3
"""apply: delete the items of a deletion list the owner approved, all or nothing, and only when
given --apply (SPEC-060 R5, R6, R7, R9; ADR-060).

    python3 apply.py LIST APPROVAL --rules RULES --log FILE [--apply]

Before its first deletion, the apply checks everything below and refuses the whole run at the
first failure, naming the item and the reason:

- every health check of RULES is a read command of the inventory's allow list;
- the list's own digest matches its content, so the list is the one the owner was shown, and RULES
  are the rules the inventory read, which the list names by their digest: RULES are read once, and
  that digest is taken over the very bytes the apply parsed and acts on;
- there is an approval, and it carries the list's digest, the approver, the date and item ids the
  list holds;
- it names a snapshot of the boot disk, with the instant it was taken, after the inventory and not
  later than the apply's own clock;
- each approved item's path is absolute and canonical, the only form the apply compares or
  deletes;
- no approved item lies under a protected path of RULES, or holds one, whatever the approval says;
- no approved item is reached through a symbolic link;
- each approved item's digest, computed again now, equals the listed one;
- each approved package would be removed alone (`dpkg --dry-run --remove`), since `dpkg` refuses
  a removal another installed package depends on, where `apt-get` would remove that one too.

Without --apply it stops there, deletes nothing, and logs what would go. With --apply it reads
every health check of RULES, deletes each approved item (a file or a link is unlinked, never its
target; a directory is removed without following a link inside it; a package is removed with
`dpkg --remove`, which keeps its configuration files), and reads the health checks again. Each
item is read again immediately before its deletion, through directories opened without following
a link, and goes only while it is what its checks read; otherwise the apply stops there. A check
red after the apply stops the scrub, and the log says which checks turned. A JSON file that holds
a key twice is refused, and a directory item needs Python 3.11 or later.

The log is private and lies outside this repository (R8). Exit 0 when done, or after a clean dry
run; 1 when refused, with nothing deleted; 2 on a usage error; 3 when the apply stopped part way,
because a deletion failed or an item changed after its checks, the log naming what went; 4 when
a health check is red after the apply.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import stat
import sys
from datetime import date, datetime
from pathlib import Path

from inventory import (
    DPKG_QUERY,
    PACKAGE,
    Refused,
    Runner,
    Usage,
    allowed,
    canonical,
    health_commands,
    inside_repository,
    load_json,
    load_rules,
    now_utc,
    parse_packages,
    read_health,
)
from plan import conflict, list_digest, measure, package_digest

SCHEMA = "deck-streak-host-scrub-apply/1"
#: Each directory on the way to an item is opened without following a symbolic link (R7).
DIRECTORY = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
#: A directory item is removed through its parent's descriptor, which `shutil.rmtree` takes from
#: Python 3.11, and only where its removal cannot follow a link swapped in beneath it.
REMOVES_BY_DESCRIPTOR = shutil.rmtree.avoids_symlink_attacks and sys.version_info >= (3, 11)


class Refusal(Exception):
    """A check that failed before the first deletion: the whole run deletes nothing (R6). A refusal
    made with `quote=False` names the item by its id alone, never by its path."""

    def __init__(self, reason: str, item: dict | None = None, *, quote: bool = True):
        super().__init__(reason)
        self.reason = reason
        self.item = item
        self.quote = quote

    def __str__(self) -> str:
        if self.item is None:
            return self.reason
        if not self.quote:
            return f"{self.item['id']}: {self.reason}"
        return f"{self.item['id']} ({target(self.item)}): {self.reason}"


def target(item: dict) -> str:
    return item.get("path") or item.get("package") or "?"


def changing(argv: list[str]) -> bool:
    """The apply's only changing commands: a package's removal, and its dry run first."""
    if argv[:2] == ["dpkg", "--remove"] and len(argv) == 3:
        return bool(PACKAGE.match(argv[2]))
    if argv[:3] == ["dpkg", "--dry-run", "--remove"] and len(argv) == 4:
        return bool(PACKAGE.match(argv[3]))
    return False


def admitted(argv: list[str]) -> bool:
    return allowed(argv) or changing(argv)


def instant(value, what: str) -> datetime:
    try:
        moment = datetime.fromisoformat(value)
    except (TypeError, ValueError) as error:
        raise Refusal(f"{what} is not a time") from error
    if moment.tzinfo is None:
        raise Refusal(f"{what} carries no offset, so it cannot be ordered")
    return moment


def linked_ancestor(path: str) -> str | None:
    """The first directory above `path` that is a symbolic link, or None."""
    current = Path(path).anchor
    for part in Path(path).parts[1:-1]:
        current = os.path.join(current, part)
        if os.path.islink(current):
            return current
    return None


def approved_items(listing: dict, approval_path: str, rules_digest: str) -> tuple[dict, list[dict]]:
    """The approval and the items it names, or the refusal that stops the run (R5, R6)."""
    if listing.get("digest") != list_digest(listing):
        raise Refusal("the list changed after it was made: its digest does not match its content")
    if listing.get("rules_digest") != rules_digest:
        raise Refusal("these rules are not the ones the inventory read, which the list names")
    if not os.path.exists(approval_path):
        raise Refusal(f"no approval at {approval_path}")
    try:
        approval = load_json(approval_path, "approval")
    except Usage as error:
        raise Refusal(str(error)) from error
    if not isinstance(approval, dict):
        raise Refusal("the approval is not a JSON object")
    if approval.get("list_digest") != listing["digest"]:
        raise Refusal("the approval does not carry the list's digest")
    if not isinstance(approval.get("approver"), str) or not approval["approver"].strip():
        raise Refusal("the approval names no approver")
    try:
        date.fromisoformat(approval.get("date"))
    except (TypeError, ValueError) as error:
        raise Refusal("the approval's date is not a date") from error
    ids = approval.get("items")
    if not isinstance(ids, list) or not ids:
        raise Refusal("the approval names no item")
    by_id = {item["id"]: item for item in listing["items"]}
    for item_id in ids:
        if item_id not in by_id:
            raise Refusal(f"the approval names {item_id}, which the list does not hold")
    snapshot = approval.get("snapshot")
    if not isinstance(snapshot, dict) or not str(snapshot.get("name") or "").strip():
        raise Refusal("the approval names no snapshot")
    taken = instant(snapshot.get("taken_at"), "the snapshot's instant")
    inventory = instant(listing["inventory"]["taken_at"], "the inventory's instant")
    if taken <= inventory:
        raise Refusal(
            f"the snapshot {snapshot['name']} was taken before the inventory "
            f"({taken.isoformat()} is not after {inventory.isoformat()})"
        )
    if taken > now_utc():
        raise Refusal(
            f"the snapshot {snapshot['name']} is dated later than the apply's clock "
            f"({taken.isoformat()}), so it cannot have been taken yet"
        )
    return approval, [item for item in listing["items"] if item["id"] in set(ids)]


def check_item(item: dict, protected: list[str], runner: Runner) -> tuple | None:
    """Refuse an item whose path is not absolute and canonical, or that is protected, reached
    through a link, or changed (R6, R7). A path item's device, inode and modification time, as
    checked, are returned for its deletion to compare."""
    if item["class"] == "package":
        check_package(item, runner)
        return None
    path = item.get("path")
    if not canonical(path):
        raise Refusal("its path is not absolute and canonical", item, quote=False)
    guard = conflict(path, protected)
    if guard is not None:
        raise Refusal(f"lies under or holds the protected path {guard}", item)
    link = linked_ancestor(path)
    if link is not None:
        raise Refusal(f"is reached through the symbolic link {link}", item)
    if not os.path.lexists(path):
        raise Refusal("is gone since the list was made", item)
    st = os.lstat(path)
    if stat.S_ISDIR(st.st_mode) and not REMOVES_BY_DESCRIPTOR:
        raise Refusal("this Python cannot remove a directory through a descriptor", item)
    try:
        digest, _ = measure(path)
    except OSError as error:
        raise Refusal(f"cannot be digested again: {error}", item) from error
    if digest != item["digest"]:
        raise Refusal("its digest changed since the list was made", item)
    return (st.st_dev, st.st_ino, st.st_mtime_ns)


def check_package(item: dict, runner: Runner) -> None:
    code, output = runner.run(DPKG_QUERY + [item["package"]])
    found = parse_packages(output) if code == 0 else []
    if len(found) != 1 or package_digest(found[0]) != item["digest"]:
        raise Refusal("its digest changed since the list was made", item)
    code, _ = runner.run(["dpkg", "--dry-run", "--remove", item["package"]])
    if code != 0:
        raise Refusal(f"would not be removed alone: dpkg's dry run exited {code}", item)


def open_parent(path: str) -> int:
    """A descriptor of the directory that holds `path`, opened from `/` one component at a time
    and never through a symbolic link, so the deletion reads the directories its item was checked
    under (R7)."""
    descriptor = os.open("/", DIRECTORY)
    try:
        for part in Path(path).parts[1:-1]:
            try:
                child = os.open(part, DIRECTORY, dir_fd=descriptor)
            except OSError as error:
                st = os.stat(part, dir_fd=descriptor, follow_symlinks=False)
                if stat.S_ISLNK(st.st_mode):
                    reason = "a directory above it became a symbolic link after its checks"
                    raise OSError(reason) from error
                raise
            os.close(descriptor)
            descriptor = child
    except BaseException:
        os.close(descriptor)
        raise
    return descriptor


def delete(item: dict, runner: Runner, checked: tuple | None) -> None:
    """Delete one item. A package is removed by `dpkg --remove`. A path is read again immediately
    before its deletion, through the directories `open_parent` opened, and deleted only while it is
    what its checks read (R6, R7)."""
    if item["class"] == "package":
        code, _ = runner.run(["dpkg", "--remove", item["package"]])
        if code != 0:
            raise OSError(f"dpkg --remove exited {code}")
        return
    name = os.path.basename(item["path"])
    parent = open_parent(item["path"])
    try:
        st = os.stat(name, dir_fd=parent, follow_symlinks=False)
        if (st.st_dev, st.st_ino, st.st_mtime_ns) != checked:
            raise OSError("it changed after its checks, so it was left as it was")
        if stat.S_ISDIR(st.st_mode):
            shutil.rmtree(name, dir_fd=parent)
        else:
            os.unlink(name, dir_fd=parent)
    finally:
        os.close(parent)


class Log:
    """The apply's record, written again after every step, so a stop part way still says what
    went (R9)."""

    def __init__(self, path: str, mode: str, listing: dict | None):
        self.path = Path(path)
        self.record = {
            "schema": SCHEMA,
            "started_at": now_utc().isoformat(),
            "mode": mode,
            "list_digest": (listing or {}).get("digest"),
            "refused": None,
            "approval": None,
            "would_delete": [],
            "health_before": [],
            "deleted": [],
            "failed": None,
            "health_after": [],
            "red": [],
            "turned": [],
        }

    def write(self, **fields) -> None:
        self.record.update(fields, finished_at=now_utc().isoformat())
        partial = self.path.with_name(self.path.name + ".partial")
        partial.write_text(json.dumps(self.record, indent=1) + "\n", encoding="utf-8")
        os.replace(partial, self.path)


def entry(item: dict) -> dict:
    return {
        "id": item["id"],
        "class": item["class"],
        "target": target(item),
        "bytes": item["bytes"],
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("list", help="the deletion list plan.py wrote")
    parser.add_argument("approval", help="the owner's approval of the list's items")
    parser.add_argument("--rules", required=True, help="the private rules: protected paths, health")
    parser.add_argument("--log", required=True, help="the apply log, outside this repository")
    parser.add_argument("--apply", action="store_true", help="delete; without it, a dry run")
    args = parser.parse_args(argv)
    if inside_repository(args.log):
        print(f"apply: {args.log} is inside this repository; write it to the private directory")
        return 2
    try:
        listing = load_json(args.list, "list")
        rules = load_rules(args.rules)
    except Usage as error:
        print(f"apply: {error}")
        return 2
    log = Log(args.log, "apply" if args.apply else "dry", listing)
    # The health checks are read commands of the inventory's allow list; the changing commands
    # the apply admits are a listed package's, and run for its items alone (R6, R9).
    reads, runner = Runner(), Runner(allow=admitted)
    try:
        try:
            reads.check(health_commands(rules["health"]))
        except Refused as refused:
            reason = f"the health check `{refused}` is not on the read-only allow list"
            raise Refusal(reason) from refused
        approval, items = approved_items(listing, args.approval, rules["digest"])
        checked = {}
        for item in items:
            checked[item["id"]] = check_item(item, rules["protected"], runner)
    except (Refusal, Refused) as refusal:
        item = getattr(refusal, "item", None)
        reason = getattr(refusal, "reason", f"`{refusal}` is not a command the apply may run")
        log.write(refused={"item": item and item["id"], "reason": reason})
        print(f"apply: refused: {refusal}; nothing was deleted")
        return 1
    total = sum(item["bytes"] for item in items)
    log.write(approval=approval, would_delete=[entry(item) for item in items])
    if not args.apply:
        print(
            f"apply: dry run: {len(items)} approved item(s) would go, reclaiming {total} bytes; "
            "nothing was deleted (give --apply to delete)"
        )
        return 0
    before = read_health(rules["health"], reads)
    log.write(health_before=before)
    deleted = []
    for item in items:
        try:
            delete(item, runner, checked[item["id"]])
        except OSError as error:
            log.write(failed={"id": item["id"], "error": str(error)})
            print(f"apply: stopped: {item['id']} ({target(item)}) could not be deleted: {error}")
            after = read_health(rules["health"], reads)
            log.write(health_after=after, red=[c["id"] for c in after if not c["green"]])
            return 3
        deleted.append(entry(item))
        log.write(deleted=deleted)
        print(f"apply: deleted {item['id']} ({target(item)}), {item['bytes']} bytes")
    after = read_health(rules["health"], reads)
    green_before = {check["id"] for check in before if check["green"]}
    red = [check["id"] for check in after if not check["green"]]
    turned = [check_id for check_id in red if check_id in green_before]
    log.write(health_after=after, red=red, turned=turned)
    print(f"apply: deleted {len(deleted)} item(s), reclaiming {total} bytes")
    if red:
        for check_id in red:
            how = "turned red" if check_id in turned else "is red, as it was before the apply"
            print(f"apply: health: {check_id} {how}; the scrub stops here")
        return 4
    return 0


if __name__ == "__main__":
    sys.exit(main())
