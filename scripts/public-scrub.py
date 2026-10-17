#!/usr/bin/env python3
"""public-scrub: refuse any file, blob or issue body that would disclose what a public repository
must not.

    python3 scripts/public-scrub.py --root .                      every tracked file
    python3 scripts/public-scrub.py --root . --subject PATH       also PATH: a file, or every file
                                                                  under a directory
    python3 scripts/public-scrub.py --root . --deny-list FILE     plus the maintainer's private list
    python3 scripts/public-scrub.py --root . --history            plus every blob HEAD reaches

The rules are composed in one place, `rules(private)`, from the scrub's own files in
`scripts/scrub-rules/`, which keep the persona-core and privacy-gdpr packs' public shapes; the box
run compares each with its pack's own, field by field, so they cannot drift (ADR-069):

* persona-core's public deny shapes: IPv4 addresses and hostnames that encode one, provider token
  shapes, Telegram bot tokens and supergroup ids, private-key headers, and email addresses outside
  the documentation domains;
* privacy-gdpr's public-repository shapes: IPv6, a cloud project id on a command line, in a key or
  in a resource path, a secret name, a bucket, a cloud or internal host, a home directory that
  names a user, and a Telegram chat or user id or a phone number written in context;
* a PRIVATE list of the same schema (`--deny-list FILE` or `$PERSONA_CORE_DENY_LIST`), kept outside
  this repository on the maintainer's machine, naming the literals that are the owner's alone:
  deck names, the project id, secret names, host names, study words. CI has no private list and
  judges by the public shapes.

A systemd unit instance name, such as `getty@tty1.service`, matches the email shape and is not an
address: a match whose last label is exactly a unit type passes (SPEC-054 R2).

A `--subject` that is a file is examined alone, and a directory means every file under it. A
subject that does not exist stops the run, and one that examined no file makes it VOID, each by
name (SPEC-054 R1).

`--history` reads every blob reachable from `--rev` (HEAD by default) exactly once, so a value
that only a deleted file or an old version still holds is found before it is pushed (SPEC-033). A
binary file (a NUL byte in its first 8000 bytes, or bytes that are not UTF-8) is refused by the rule
`binary` wherever it is, and its bytes are still searched for the private literals; a file over the
size limit is refused by the rule `oversize`. A shallow repository makes `--history` VOID, because
the history it would read is incomplete.

A finding names the rule, the file (or `history:<path>@<blob>`) and the line, never the value (a
literal is reported by its index in the private list). The deny lists themselves are skipped: they
are the rules, not a disclosure. Exit 0 when clean, 1 on a finding, 2 on a usage error (a subject
that is not there, a private list that cannot be read), 3 when nothing was examined, a subject
examined nothing, or the history is shallow.
"""

from __future__ import annotations

import argparse
import ipaddress
import json
import os
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
# The scrub's own public shapes, in the order they are composed around the private list.
RULES = REPO / "scripts" / "scrub-rules"
# The schema every deny list carries, the public ones and the maintainer's private one.
DENY_SCHEMA = "phx.persona.deny.v1"
MAX_BYTES = 2_000_000
# git's own binary heuristic reads the first 8000 bytes for a NUL.
SNIFF_BYTES = 8000
# The rule files themselves, and license texts, are not disclosures.
SKIP_NAMES = {"deny-list.json", "LICENSE"}
SKIP_PREFIXES = ("LICENSES/", "scripts/scrub-rules/")
ADDRESS_RULES = {"ipv4", "ipv6"}
# The ranges privacy-gdpr's public scrub passes: documentation (RFC 5737, RFC 3849, RFC 9637).
DOCUMENTATION = [
    ipaddress.ip_network(net)
    for net in ("192.0.2.0/24", "198.51.100.0/24", "203.0.113.0/24", "2001:db8::/32", "3fff::/20")
]
CONTINUES = re.compile(r"^\.\d")
#: persona-core's rule for an email address, whose shape a unit instance name also matches.
EMAIL_RULE = "email"
#: The systemd unit types an instance's unit name may end with, lowercase as systemd writes them
#: (systemd.unit(5)): `getty@tty1.service` is a unit, not an address (SPEC-054 R2).
UNIT_TYPES = frozenset({"service", "timer", "socket", "path", "mount", "target", "slice", "scope"})


def unit_instance(token: str) -> bool:
    """Whether a match of the email shape is a systemd unit instance name: its last label is
    exactly a unit type. A unit word anywhere else (`service.<corp>.com`), in a longer label
    (`<corp>.services`) or in capitals leaves an address."""
    return token.rpartition(".")[2] in UNIT_TYPES


def harmless_address(text: str, before: str, after: str) -> bool:
    """Loopback, unspecified, link-local, multicast, broadcast and documentation addresses are
    not disclosures; a dotted number that continues or follows a letter (a version, an OID) is not
    an address, and neither is a two-group IPv6 shape such as a slice `[::2]`."""
    if CONTINUES.match(after) or (before and (before.isalnum() or before == ".")):
        return True
    if ":" in text and len([group for group in text.strip("[]").split(":") if group]) < 3:
        return True
    try:
        address = ipaddress.ip_address(text.strip("[]"))
    except ValueError:
        return True
    if address.is_loopback or address.is_unspecified or address.is_link_local:
        return True
    if address.is_multicast or str(address) == "255.255.255.255":
        return True
    return any(address in net for net in DOCUMENTATION)


class DenyListError(ValueError):
    """A deny list cannot be read: str() names the file and why."""


def compiled(row: object, where: str) -> tuple:
    """A pattern row as (its id, its compiled regex); the flag `i` ignores case."""
    if not isinstance(row, dict) or not isinstance(row.get("regex"), str):
        raise DenyListError(f"{where}: a row has no regex")
    flags = re.IGNORECASE if "i" in str(row.get("flags", "")) else 0
    try:
        return (str(row.get("id", "?")), re.compile(row["regex"], flags))
    except re.error as error:
        raise DenyListError(f"{where}: {row.get('id')}: {error}") from error


def load_deny(path: Path, origin: str) -> dict:
    """The deny list at `path` in its schema, each entry tagged with `origin`: its `patterns` as
    (origin, id, compiled regex) and its `literals` as (origin, literal), casefolded. Its key
    markers and journal paths are checked and not kept: the scrub reads neither. An unreadable or
    malformed list raises DenyListError."""
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise DenyListError(f"{path}: unreadable ({error.strerror or error})") from error
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise DenyListError(f"{path}: not JSON ({error})") from error
    if not isinstance(data, dict) or data.get("schema") != DENY_SCHEMA:
        raise DenyListError(f"{path}: schema is not {DENY_SCHEMA}")
    loaded: dict = {"patterns": [], "literals": []}
    for key in ("key_markers", "patterns", "literals", "journal_paths"):
        values = data.get(key, [])
        if not isinstance(values, list):
            raise DenyListError(f"{origin} deny-list: {key} is not a list")
        for value in values:
            if key == "patterns":
                loaded["patterns"].append((origin, *compiled(value, f"{origin} deny-list")))
            elif not isinstance(value, str) or not value:
                raise DenyListError(f"{origin} deny-list: {key} holds a non-string")
            elif key == "literals":
                loaded["literals"].append((origin, value.casefold()))
    return loaded


def tracked(root: Path) -> list[Path]:
    done = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        capture_output=True,
        check=False,
    )
    if done.returncode != 0:
        raise SystemExit(f"public-scrub: git ls-files failed in {root}")
    names = [name for name in done.stdout.decode("utf-8").split("\0") if name]
    return [root / name for name in names]


def under(directory: Path) -> list[Path]:
    return sorted(path for path in directory.rglob("*") if path.is_file())


def skipped(path: Path, root: Path) -> bool:
    try:
        rel = path.relative_to(root).as_posix()
    except ValueError:
        rel = path.name
    return skipped_name(rel)


def skipped_name(rel: str) -> bool:
    return Path(rel).name in SKIP_NAMES or rel.startswith(SKIP_PREFIXES)


def is_binary(data: bytes) -> bool:
    if b"\0" in data[:SNIFF_BYTES]:
        return True
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        return True
    return False


class UsageError(Exception):
    """A usage or environment error (git could not answer, a subject that is not there): never a
    verdict on the content."""


class RulesError(Exception):
    """The deny lists cannot be composed (a private list that is not a file, or a list that
    cannot be read): a usage error, reported with exit 2 by every caller of `rules()`."""


class Scan:
    """Applies the composed shapes and literals, and collects findings that never carry a value."""

    def __init__(self, rows: list, literals: list) -> None:
        self.rows = rows
        self.literals = literals
        self.findings: list[str] = []

    def text(self, shown: str, text: str) -> None:
        normal = unicodedata.normalize("NFKC", text)
        for number, line in enumerate(normal.splitlines(), start=1):
            for rule, pattern in self.rows:
                for match in pattern.finditer(line):
                    if rule in ADDRESS_RULES and harmless_address(
                        match.group(0),
                        line[max(match.start() - 1, 0) : match.start()],
                        line[match.end() : match.end() + 2],
                    ):
                        continue
                    if rule == EMAIL_RULE and unit_instance(match.group(0)):
                        continue
                    self.findings.append(f"{shown}:{number}: {rule}")
        folded = normal.casefold()
        for index, (origin, literal) in enumerate(self.literals):
            if literal and literal in folded:
                number = folded[: folded.index(literal)].count("\n") + 1
                self.findings.append(f"{shown}:{number}: {origin} literal #{index}")

    def binary(self, shown: str, data: bytes) -> None:
        # Refused for being binary; its bytes are still searched, so the report says whether it
        # also carried a private literal (a compiled cache embeds the paths it was built from).
        self.findings.append(f"{shown}: binary")
        folded = data.decode("latin-1").casefold()
        for index, (origin, literal) in enumerate(self.literals):
            if literal and literal in folded:
                self.findings.append(f"{shown}: {origin} literal #{index}")


def git(root: Path, *args: str, stdin: bytes | None = None) -> bytes:
    done = subprocess.run(
        ["git", "-C", str(root), *args], input=stdin, capture_output=True, check=False
    )
    if done.returncode != 0:
        message = done.stderr.decode("utf-8", "replace").strip()[:200]
        raise UsageError(f"git {args[0]} failed in {root}: {message}")
    return done.stdout


def read_blobs(root: Path, ids: list[str]) -> list[bytes]:
    """Every blob's bytes through one `git cat-file --batch`, in the order asked."""
    if not ids:
        return []
    out = git(root, "cat-file", "--batch", stdin=("\n".join(ids) + "\n").encode("ascii"))
    contents, position = [], 0
    for _ in ids:
        end = out.index(b"\n", position)
        size = int(out[position:end].split()[2])
        contents.append(out[end + 1 : end + 1 + size])
        position = end + 1 + size + 1
    return contents


def history(root: Path, rev: str, scan: Scan) -> int | None:
    """Scan every blob reachable from `rev` once. None when the repository is shallow (VOID)."""
    if git(root, "rev-parse", "--is-shallow-repository").strip() != b"false":
        return None
    try:
        git(root, "rev-parse", "--verify", "--quiet", f"{rev}^{{commit}}")
    except UsageError:
        return 0
    first_path: dict[str, str] = {}
    for line in git(root, "rev-list", "--objects", rev).decode("utf-8", "replace").splitlines():
        object_id, _, path = line.partition(" ")
        if path and object_id not in first_path:
            first_path[object_id] = path
    if not first_path:
        return 0
    kinds = git(
        root,
        "cat-file",
        "--batch-check=%(objectname) %(objecttype) %(objectsize)",
        stdin=("\n".join(first_path) + "\n").encode("ascii"),
    )
    blobs, readable = 0, []
    for line in kinds.decode("ascii").splitlines():
        object_id, kind, size = line.split()
        if kind != "blob":
            continue
        blobs += 1
        shown = f"history:{first_path[object_id]}@{object_id[:9]}"
        if int(size) > MAX_BYTES:
            scan.findings.append(f"{shown}: oversize")
        else:
            readable.append((object_id, shown, first_path[object_id]))
    for (object_id, shown, path), data in zip(
        readable, read_blobs(root, [item[0] for item in readable]), strict=True
    ):
        if is_binary(data):
            scan.binary(shown, data)
        elif not skipped_name(path):
            scan.text(shown, data.decode("utf-8"))
    return blobs


def rules(private: Path | None) -> Scan:
    """The scrub's rules, composed in this one place: persona-core's public shapes, then the
    private list `private`, then privacy-gdpr's public shapes, as a Scan. RulesError names a
    private list that is not a file, or a deny list that cannot be read."""
    if private is not None and not private.is_file():
        raise RulesError(f"the private list {private} is not a file")
    try:
        lists = [load_deny(RULES / "persona-core.json", "public")]
        if private is not None:
            lists.append(load_deny(private, "private"))
        lists.append(load_deny(RULES / "privacy-gdpr.json", "public"))
    except DenyListError as error:
        raise RulesError(f"a deny list cannot be read: {error}") from error
    rows = [(rule, pattern) for deny in lists for _origin, rule, pattern in deny["patterns"]]
    literals = [(origin, literal) for deny in lists for origin, literal in deny["literals"]]
    return Scan(rows, literals)


def subject_files(subject: str) -> list[Path]:
    """The files a `--subject` names: a file is its own subject, and a directory means every file
    under it. Anything else raises UsageError, naming the subject as it was given."""
    path = Path(subject).resolve()
    if path.is_file():
        return [path]
    if path.is_dir():
        return under(path)
    if not path.exists():
        raise UsageError(f"the subject {subject} does not exist")
    raise UsageError(f"the subject {subject} is not a file or a directory")


def examine(scan: Scan, files: list[Path], root: Path) -> int:
    """Scan every file, and return how many were examined: a rule file the scrub skips is not."""
    examined = 0
    for path in files:
        if not path.is_file():
            continue
        shown = str(path.relative_to(root) if path.is_relative_to(root) else path)
        if path.stat().st_size > MAX_BYTES:
            examined += 1
            scan.findings.append(f"{shown}: oversize")
            continue
        try:
            data = path.read_bytes()
        except OSError:
            examined += 1
            scan.findings.append(f"{shown}: unreadable")
            continue
        if is_binary(data):
            examined += 1
            scan.binary(shown, data)
            continue
        if skipped(path, root):
            continue
        examined += 1
        scan.text(shown, data.decode("utf-8"))
    return examined


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", default=str(REPO))
    parser.add_argument(
        "--subject", action="append", default=[], help="a file, or every file under a directory"
    )
    parser.add_argument("--deny-list", default=os.environ.get("PERSONA_CORE_DENY_LIST"))
    parser.add_argument("--no-tree", action="store_true", help="scan only the --subject paths")
    parser.add_argument(
        "--history", action="store_true", help="also read every blob reachable from --rev"
    )
    parser.add_argument("--rev", default="HEAD", help="the revision whose history --history reads")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    private = Path(args.deny_list) if args.deny_list else None
    try:
        scan = rules(private)
        subjects = [(subject, subject_files(subject)) for subject in args.subject]
    except (RulesError, UsageError) as error:
        print(f"public-scrub: {error}")
        return 2
    examined = examine(scan, [] if args.no_tree else tracked(root), root)
    voids = []
    for subject, files in subjects:
        counted = examine(scan, files, root)
        examined += counted
        if counted == 0:
            voids.append(f"the subject {subject} examined no file")
    blobs = 0
    if args.history:
        try:
            counted = history(root, args.rev, scan)
        except UsageError as error:
            print(f"public-scrub: {error}")
            return 2
        if counted is None:
            voids.append("the repository is shallow, so the history it would read is incomplete")
        else:
            blobs = counted
    for finding in scan.findings:
        print(f"public-scrub: {finding}")
    scope = "public shapes and the private list" if private else "public shapes only"
    read = f"{examined} file(s)" + (f" and {blobs} history blob(s)" if args.history else "")
    print(f"examined {read} against {scope}; {len(scan.findings)} finding(s)")
    for void in voids:
        print(f"public-scrub: VOID: {void}")
    if scan.findings:
        return 1
    return 3 if voids or examined + blobs == 0 else 0


if __name__ == "__main__":
    sys.exit(main())
