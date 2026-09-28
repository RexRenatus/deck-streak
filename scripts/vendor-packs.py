#!/usr/bin/env python3
"""vendor-packs: re-vendor the pack probes from a phoenix-v2 checkout, refusing by construction
every file an exclusion names or the public scrub would refuse (SPEC-037, ADR-039).

    python3 scripts/vendor-packs.py --source DIR [--root ROOT] [--deny-list FILE]

The run reads `ROOT/.packs/VENDORED.json` and the commit the source checkout's HEAD names:

1. The candidates are the files the manifest lists (by `from`), and every file of that commit
   under a `skills/packs/<pack>/` directory already vendored. A pack that is not vendored is a
   wiring decision: the run names it and leaves it out.
2. A candidate whose source path matches an `excluded` entry's `globs` is dropped before any byte
   of it is read. The globs are `fnmatch` patterns, where `*` and `**` both cross `/`. A listed
   file an exclusion matches loses its manifest entry when the run passes; no file is deleted.
3. A listed file the commit lacks refuses the run with exit 2.
4. Every other candidate is read from the commit into memory and scanned with the public scrub's
   own rules, as `scripts/public-scrub.py`'s `rules()` composes them: persona-core's and
   privacy-gdpr's shapes, the private list from `--deny-list` or `$PERSONA_CORE_DENY_LIST`, the
   binary rule and the size limit, with the rule files the scrub skips skipped the same way. Any
   finding refuses the run with exit 1, one line per finding naming the source path and the rule,
   never the value.
5. Only when every candidate passed are the changed and new files written, and the manifest's
   digests and `vendored_from` updated, with `methodology.json`'s `vendored_from`. A file the tree
   already holds byte for byte is not touched. A refused run leaves the tree byte-identical, and
   nothing is staged on disk on the way.

The files come from the commit, never the working tree, so `vendored_from` names exactly what was
read and an untracked file is never a candidate. The run ends with one line naming the files
examined, changed, new and excluded, and the commit they came from. Exit 0 when the tree is
vendored, 1 on a finding, 2 on a listed file the commit lacks or a usage error, 3 when nothing was
examined.
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import importlib.util
import json
import os
import re
import stat
import sys
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

# The scrub and the probe it loads are read by path. A run must write no bytecode into the tree it
# vendors, so this is set before either is loaded.
sys.dont_write_bytecode = True

REPO = Path(__file__).resolve().parents[1]
SCRUB = REPO / "scripts" / "public-scrub.py"
MANIFEST = PurePosixPath(".packs/VENDORED.json")
METHODOLOGY = PurePosixPath("methodology.json")
PACK_DIRECTORY = "skills/packs/"
# methodology.json keeps its own layout, so only the pin's value is rewritten.
PIN = re.compile(r'("vendored_from"\s*:\s*")([^"\\]*)(")')
REGULAR = {"100644", "100755"}
EXECUTABLE = "100755"


class Refusal(Exception):
    """The run cannot trust its input: a usage error, reported with exit 2 before any write."""


@dataclass(frozen=True, slots=True)
class Blob:
    """One file of the source commit, known from the listing alone: no byte of it is read."""

    mode: str
    object_id: str
    size: int


@dataclass(frozen=True, slots=True)
class Candidate:
    """A file the run may vendor: its path in the source commit and its path under the root."""

    source: str
    destination: str
    listed: bool


def say(message: str) -> None:
    print(f"vendor-packs: {message}")


def load_scrub():
    """scripts/public-scrub.py as a module: its rules are reused, never copied (ADR-039)."""
    spec = importlib.util.spec_from_file_location("public_scrub", SCRUB)
    if spec is None or spec.loader is None:
        raise Refusal(f"cannot load {SCRUB.name}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def rules(scrub, deny_list: str | None):
    """The scrub's own composition of its rules, `rules(private)`, and so the very Scan its main()
    scans with: persona-core's list with the private list, and privacy-gdpr's. Nothing here
    composes a list itself (ADR-039, SPEC-054 R3); the scrub's refusal is a usage error here."""
    try:
        return scrub.rules(Path(deny_list) if deny_list else None)
    except scrub.RulesError as error:
        raise Refusal(str(error)) from error


def relative(path: object, what: str) -> str:
    """`path` when it is a relative POSIX path that stays inside its tree; else a refusal."""
    if not isinstance(path, str) or not path:
        raise Refusal(f"{MANIFEST}: {what} is not a path")
    pure = PurePosixPath(path)
    if pure.is_absolute() or ".." in pure.parts or "\\" in path or pure.as_posix() != path:
        raise Refusal(f"{MANIFEST}: {what} {path!r} is not a relative path inside the tree")
    return path


def read_manifest(root: Path) -> dict:
    """VENDORED.json, refused unless every entry is one the run can apply exactly."""
    try:
        manifest = json.loads((root / MANIFEST).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Refusal(f"cannot read {MANIFEST}: {error}") from error
    if not isinstance(manifest, dict):
        raise Refusal(f"{MANIFEST} is not an object")
    files, excluded = manifest.get("files"), manifest.get("excluded")
    if not isinstance(files, list) or not isinstance(excluded, list):
        raise Refusal(f"{MANIFEST} needs a `files` list and an `excluded` list")
    seen: set[tuple[str, str]] = set()
    for number, entry in enumerate(files):
        if not isinstance(entry, dict) or not isinstance(entry.get("sha256"), str):
            raise Refusal(f"{MANIFEST}: files[{number}] needs a path, a from and a sha256")
        for key in ("path", "from"):
            name = relative(entry.get(key), f"files[{number}].{key}")
            if (key, name) in seen:
                raise Refusal(f"{MANIFEST}: files[{number}].{key} {name} is listed twice")
            seen.add((key, name))
    for number, entry in enumerate(excluded):
        globs = entry.get("globs") if isinstance(entry, dict) else None
        if not isinstance(globs, list) or not globs:
            raise Refusal(
                f"{MANIFEST}: excluded[{number}] carries no globs, so no tool can apply it"
            )
        if not all(isinstance(glob, str) and glob for glob in globs):
            raise Refusal(f"{MANIFEST}: excluded[{number}] holds a glob that is not a pattern")
        if not isinstance(entry.get("why"), str) or not entry["why"].strip():
            raise Refusal(f"{MANIFEST}: excluded[{number}] gives no why")
    return manifest


def read_pin(root: Path) -> str:
    """methodology.json's text, refused unless it holds exactly one `vendored_from` to rewrite."""
    try:
        text = (root / METHODOLOGY).read_text(encoding="utf-8")
        data = json.loads(text)
    except (OSError, ValueError) as error:
        raise Refusal(f"cannot read {METHODOLOGY}: {error}") from error
    if not isinstance(data, dict) or not isinstance(data.get("vendored_from"), str):
        raise Refusal(f"{METHODOLOGY} holds no vendored_from")
    if len(PIN.findall(text)) != 1:
        raise Refusal(f"{METHODOLOGY} holds more than one vendored_from, or none to rewrite")
    return text


def source_commit(scrub, source: Path) -> tuple[str, dict[str, Blob]]:
    """The commit the checkout's HEAD names, and every file it holds, from its listing alone."""
    if not source.is_dir():
        raise Refusal(f"the source {source} is not a directory")
    try:
        commit = scrub.git(source, "rev-parse", "--verify", "HEAD^{commit}").decode().strip()
        listing = scrub.git(source, "ls-tree", "-r", "-l", "-z", "--full-tree", commit)
    except scrub.UsageError as error:
        raise Refusal(f"the source is not a git checkout with a commit: {error}") from error
    files = {}
    for record in listing.split(b"\0"):
        if not record:
            continue
        meta, _, name = record.partition(b"\t")
        mode, kind, object_id, size = meta.decode("ascii").split()
        if kind == "blob":
            files[name.decode("utf-8", "surrogateescape")] = Blob(mode, object_id, int(size))
    return commit, files


def pack_of(source: str) -> str | None:
    """The pack whose `skills/packs/<pack>/` directory holds `source`, or None."""
    parts = source.split("/")
    return parts[2] if source.startswith(PACK_DIRECTORY) and len(parts) > 3 else None


def excluded_by(source: str, globs: list[str]) -> bool:
    """Whether an exclusion names `source`. fnmatch treats `/` as an ordinary character, so `*`
    and `**` both cross directories: a glob can exclude more than it seems to, never less."""
    return any(fnmatch.fnmatchcase(source, glob) for glob in globs)


def candidates(manifest: dict, files: dict[str, Blob]):
    """(every candidate, the upstream packs that are not vendored with their files)."""
    listed = {entry["from"]: entry["path"] for entry in manifest["files"]}
    vendored = {pack for source in listed if (pack := pack_of(source)) is not None}
    found = [Candidate(source, path, True) for source, path in listed.items()]
    others: dict[str, list[str]] = {}
    for source in sorted(files):
        pack = pack_of(source)
        if pack in vendored and source not in listed:
            found.append(Candidate(source, f".packs/{source}", False))
        elif pack is not None and pack not in vendored:
            others.setdefault(pack, []).append(source)
    return found, others


def check_destinations(root: Path, chosen: list[Candidate]) -> None:
    """Refuse, before any write, a destination claimed twice or one the tree cannot take as a
    regular file (a directory, or a symlink on the way that could lead out of the tree)."""
    claimed: dict[str, str] = {}
    for candidate in chosen:
        if candidate.destination in claimed:
            first = claimed[candidate.destination]
            raise Refusal(
                f"{candidate.destination} is claimed by both {first} and {candidate.source}"
            )
        claimed[candidate.destination] = candidate.source
        path = root
        for part in PurePosixPath(candidate.destination).parts:
            path = path / part
            if path.is_symlink():
                raise Refusal(f"{candidate.destination}: the tree holds a symlink on its way")
        if path.is_dir():
            raise Refusal(f"{candidate.destination}: the tree holds a directory there")


def write_file(target: Path, body: bytes, executable: bool) -> bool:
    """Write `body` with the executable bit upstream gives it. False, touching nothing, when the
    tree already holds exactly that."""
    if target.is_file():
        mode = stat.S_IMODE(target.stat().st_mode)
        if target.read_bytes() == body and bool(mode & 0o111) == executable:
            return False
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(body)
    mode = stat.S_IMODE(target.stat().st_mode)
    wanted = mode | ((mode & 0o444) >> 2) if executable else mode & ~0o111
    if wanted != mode:
        target.chmod(wanted)
    return True


def read_and_scan(scrub, scan, source: Path, files: dict[str, Blob], kept: list[Candidate]):
    """Each regular candidate within the size limit, read from the commit into memory and scanned
    with the scrub's rules; a symlink or an oversize file is a finding without being read."""
    readable = [
        c.source
        for c in kept
        if files[c.source].mode in REGULAR and files[c.source].size <= scrub.MAX_BYTES
    ]
    ids = [files[name].object_id for name in readable]
    bodies = dict(zip(readable, scrub.read_blobs(source, ids), strict=True))
    for candidate in kept:
        name = candidate.source
        if files[name].mode not in REGULAR:
            scan.findings.append(f"{name}: symlink")
        elif name not in bodies:
            scan.findings.append(f"{name}: oversize")
        elif scrub.is_binary(bodies[name]):
            scan.binary(name, bodies[name])
        elif not scrub.skipped_name(name):
            scan.text(name, bodies[name].decode("utf-8"))
    return bodies


def write_tree(root: Path, kept: list[Candidate], files: dict[str, Blob], bodies: dict):
    """(changed, new, the manifest's entries): every candidate written that the tree does not
    already hold exactly, in destination order."""
    changed = new = 0
    entries = []
    for candidate in sorted(kept, key=lambda c: c.destination):
        body = bodies[candidate.source]
        executable = files[candidate.source].mode == EXECUTABLE
        if write_file(root / candidate.destination, body, executable):
            if candidate.listed:
                changed += 1
            else:
                new += 1
        digest = hashlib.sha256(body).hexdigest()
        entries.append({"path": candidate.destination, "from": candidate.source, "sha256": digest})
    return changed, new, entries


def summary(examined: int, changed: int, new: int, excluded: int, commit: str) -> str:
    return (
        f"examined {examined} file(s), changed {changed}, new {new}, excluded {excluded}, "
        f"from {commit}"
    )


def run(source: Path, root: Path, deny_list: str | None) -> int:
    scrub = load_scrub()
    scan = rules(scrub, deny_list)
    manifest = read_manifest(root)
    pin = read_pin(root)
    commit, files = source_commit(scrub, source)
    globs = [glob for entry in manifest["excluded"] for glob in entry["globs"]]
    found, others = candidates(manifest, files)
    kept = sorted((c for c in found if not excluded_by(c.source, globs)), key=lambda c: c.source)
    dropped = [c for c in found if excluded_by(c.source, globs)]
    check_destinations(root, kept)
    for candidate in sorted(dropped, key=lambda c: c.source):
        if candidate.listed:
            say(
                f"{candidate.source}: listed, and an exclusion matches it: never read, and "
                "dropped from the manifest when the run passes"
            )
    whole = sorted(
        pack for pack, names in others.items() if all(excluded_by(n, globs) for n in names)
    )
    left_out = sorted(set(others) - set(whole))
    if left_out:
        say(f"left out, not vendored (a new pack is a wiring decision): {', '.join(left_out)}")
    if whole:
        say(f"excluded whole by the manifest: {', '.join(whole)}")

    missing = [c.source for c in kept if c.source not in files]
    if missing:
        for name in missing:
            say(f"{name}: listed, but the commit lacks it")
        say(f"REFUSED: {len(missing)} listed file(s) the commit lacks; nothing was written")
        say(summary(0, 0, 0, len(dropped), commit))
        return 2
    if not kept:
        say("VOID: nothing was examined; nothing was written")
        say(summary(0, 0, 0, len(dropped), commit))
        return 3
    bodies = read_and_scan(scrub, scan, source, files, kept)
    if scan.findings:
        for finding in scan.findings:
            say(finding)
        say(f"REFUSED: {len(scan.findings)} finding(s); nothing was written")
        say(summary(len(kept), 0, 0, len(dropped), commit))
        return 1

    changed, new, entries = write_tree(root, kept, files, bodies)
    updated = {**manifest, "vendored_from": commit, "files": entries}
    if updated != manifest:
        (root / MANIFEST).write_bytes((json.dumps(updated, indent=2) + "\n").encode("utf-8"))
    if json.loads(pin)["vendored_from"] != commit:
        repinned = PIN.sub(lambda match: match.group(1) + commit + match.group(3), pin, count=1)
        (root / METHODOLOGY).write_bytes(repinned.encode("utf-8"))
    say(summary(len(kept), changed, new, len(dropped), commit))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--source", required=True, help="a phoenix-v2 checkout; its HEAD is read")
    parser.add_argument("--root", default=str(REPO), help="the DeckStreak tree to vendor into")
    parser.add_argument(
        "--deny-list",
        default=os.environ.get("PERSONA_CORE_DENY_LIST"),
        help="the maintainer's private list (default: $PERSONA_CORE_DENY_LIST)",
    )
    args = parser.parse_args()
    try:
        return run(Path(args.source).resolve(), Path(args.root).resolve(), args.deny_list)
    except Refusal as error:
        say(f"{error}; nothing was written")
        return 2


if __name__ == "__main__":
    sys.exit(main())
