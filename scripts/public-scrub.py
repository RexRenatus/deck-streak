#!/usr/bin/env python3
"""public-scrub: refuse any file or issue body that would disclose what a public repository must not.

    python3 scripts/public-scrub.py --root .                      every tracked file
    python3 scripts/public-scrub.py --root . --subject DIR        also every file under DIR
    python3 scripts/public-scrub.py --root . --deny-list FILE     plus the maintainer's private list

The rules are the packs' own, composed and never copied (CHARTER constraint 11):

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

A finding names the rule, the file and the line, never the value (a literal is reported by its
index in the private list). The deny lists themselves are skipped: they are the rules, not a
disclosure. Exit 0 when clean, 1 on a finding, 2 on a usage error, 3 when nothing was examined.
"""

from __future__ import annotations

import argparse
import importlib.util
import ipaddress
import os
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
PACKS = REPO / ".packs" / "skills" / "packs"
PROBE = REPO / ".packs" / "scripts" / "persona-core-probe.py"
MAX_BYTES = 2_000_000
# The rule files themselves, and license texts, are not disclosures.
SKIP_NAMES = {"deny-list.json", "LICENSE"}
SKIP_PREFIXES = ("LICENSES/",)
ADDRESS_RULES = {"ipv4", "ipv6"}
# The ranges privacy-gdpr's public scrub passes: documentation (RFC 5737, RFC 3849, RFC 9637).
DOCUMENTATION = [
    ipaddress.ip_network(net)
    for net in ("192.0.2.0/24", "198.51.100.0/24", "203.0.113.0/24", "2001:db8::/32", "3fff::/20")
]
CONTINUES = re.compile(r"^\.\d")


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


def load_persona_core():
    spec = importlib.util.spec_from_file_location("persona_core_probe", PROBE)
    if spec is None or spec.loader is None:
        raise SystemExit(f"public-scrub: cannot load {PROBE}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


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
    return path.name in SKIP_NAMES or rel.startswith(SKIP_PREFIXES)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", default=str(REPO))
    parser.add_argument("--subject", action="append", default=[])
    parser.add_argument("--deny-list", default=os.environ.get("PERSONA_CORE_DENY_LIST"))
    parser.add_argument("--no-tree", action="store_true", help="scan only the --subject paths")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    pc = load_persona_core()
    private = Path(args.deny_list) if args.deny_list else None
    if private is not None and not private.is_file():
        print(f"public-scrub: the private list {private} is not a file")
        return 2
    lists = [
        pc.load_deny(PACKS / "persona-core", private),
        pc.load_deny(PACKS / "privacy-gdpr", None),
    ]
    rows = [(rid, rx) for deny in lists for _origin, rid, rx in deny["patterns"]]
    literals = [(origin, lit) for deny in lists for origin, lit in deny["literals"]]
    files = [] if args.no_tree else tracked(root)
    for subject in args.subject:
        files += under(Path(subject).resolve())
    examined, findings = 0, []
    for path in files:
        if skipped(path, root) or not path.is_file() or path.stat().st_size > MAX_BYTES:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        examined += 1
        shown = path.relative_to(root) if path.is_relative_to(root) else path
        normal = unicodedata.normalize("NFKC", text)
        for number, line in enumerate(normal.splitlines(), start=1):
            for rule, pattern in rows:
                for match in pattern.finditer(line):
                    if rule in ADDRESS_RULES and harmless_address(
                        match.group(0),
                        line[max(match.start() - 1, 0) : match.start()],
                        line[match.end() : match.end() + 2],
                    ):
                        continue
                    findings.append(f"{shown}:{number}: {rule}")
        folded = unicodedata.normalize("NFKC", text).casefold()
        for index, (origin, literal) in enumerate(literals):
            if literal and literal in folded:
                number = folded[: folded.index(literal)].count("\n") + 1
                findings.append(f"{shown}:{number}: {origin} literal #{index}")
    for finding in findings:
        print(f"public-scrub: {finding}")
    scope = "public shapes and the private list" if private else "public shapes only"
    print(f"examined {examined} file(s) against {scope}; {len(findings)} finding(s)")
    if examined == 0:
        return 3
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
