#!/usr/bin/env python3
"""sdd-probe: spec-driven development, judged on any repository root (SPEC-V2-2186 R1).

    python3 scripts/sdd-probe.py --root <repo> check <class>|all [--across-refs]
    python3 scripts/sdd-probe.py --root <repo> next spec|adr [--across-refs]
    python3 scripts/sdd-probe.py --root <repo> list

Seven classes, each over the documents `methodology.json`'s `sdd` section locates (defaults
`docs/specs/SPEC-<n>-<slug>.md` and `docs/decisions/ADR-<n>-<slug>.md`):

  spec-sections      every SPEC carries each required section, and none is empty
  exclusions-cited   every unit of a SPEC's exclusions cites a tracked item (`#12`, `i7`)
  acceptance-fenced  every stated criterion has a line in the ```acceptance fence, and back
  manifest-present   the manifest names at least one repository-relative path
  adr-alternatives   every ADR names an alternative it was chosen against, with the reason
  adr-linked         every SPEC is decided by an ADR that exists
  numbering-unique   no number is held by two documents, unless the second is an amendment

A SPEC numbered below `adopted_from` predates the repository's adoption: it is counted and not
judged. `numbering-unique` and `next` read every document whatever its number, and with
`--across-refs` every git ref and every worktree too, because concurrent builders each read only
their own tree. The contract (exit codes, verdict lines, the config) is methodology_probe's.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import methodology_probe as mp  # noqa: E402

SECTION_PATTERNS = {
    "problem": r"\bproblem\b",
    "requirements": r"\brequirements?\b",
    "acceptance": r"\bacceptance\b",
    "manifest": r"\bmanifest\b",
    "exclusions": (
        r"what this does not|does not do|\bnot[ -]covered\b|non-goals|out of scope"
    ),
    "risks": r"\brisks?\b",
}
ALTERNATIVES = r"\bagainst\b|\balternatives?\b|options considered|\brejected\b"
LIST_ITEM = re.compile(r"^( {0,3})(?:[-*+]|\d+[.)])\s+")
BACKTICKED = re.compile(r"`([^`\s]+)`")
FILE_NAME = re.compile(r"^[\w.-]+\.[A-Za-z0-9]+$")
REASON = re.compile(r" [—–-] |: |\bbecause\b")


def validate(config: mp.Config) -> None:
    sdd = config.sections["sdd"]
    unknown = sorted(set(sdd["spec_sections"]) - set(SECTION_PATTERNS))
    if unknown:
        raise mp.ConfigError(
            f"{mp.CONFIG_FILE}: sdd: spec_sections names unknown section(s): "
            f"{', '.join(unknown)}; known: {', '.join(SECTION_PATTERNS)}"
        )
    try:
        re.compile(sdd["citation"])
    except re.error as error:
        raise mp.ConfigError(f"{mp.CONFIG_FILE}: sdd: citation: {error}") from error


def titled(sections: list[mp.Section], pattern: str) -> list[mp.Section]:
    return [section for section in sections if re.search(pattern, section.title, re.I)]


def read(doc: mp.Doc) -> str:
    return doc.path.read_text(encoding="utf-8", errors="replace")


def no_specs(context: mp.Context, notes: list[str]) -> str:
    return f"no SPEC to judge under {context.section('sdd')['specs']}"


def spec_sections(context: mp.Context) -> mp.Result:
    judged, notes, _, _ = mp.judged_specs(context)
    result = mp.Result(len(judged), "SPEC(s)", notes=notes)
    result.void_reason = no_specs(context, notes)
    for doc in judged:
        found = mp.sections(read(doc))
        for key in context.section("sdd")["spec_sections"]:
            matching = titled(found, SECTION_PATTERNS[key])
            if not matching:
                result.findings.append(f"{doc.rel}: no section named for {key}")
            elif all(section.is_empty() for section in matching):
                result.findings.append(f"{doc.rel}: the {key} section is empty")
    return result


def exclusion_units(section: mp.Section) -> list[tuple[int, str]]:
    """Each bullet or paragraph of an exclusions section, and each data row of its tables."""
    units: list[list] = []
    current: list | None = None
    for number, line in section.prose():
        stripped = line.strip()
        if not stripped or stripped.startswith("|"):
            current = None
            continue
        if LIST_ITEM.match(line) or current is None:
            current = [number, stripped]
            units.append(current)
        else:
            current[1] += " " + stripped
    for number, cells in mp.table_rows(section.prose()):
        units.append([number, " | ".join(cells)])
    return sorted((number, text) for number, text in units)


def exclusions_cited(context: mp.Context) -> mp.Result:
    judged, notes, _, _ = mp.judged_specs(context)
    citation = re.compile(context.section("sdd")["citation"])
    result = mp.Result(0, "exclusion unit(s)", notes=notes)
    silent = 0
    for doc in judged:
        found = titled(mp.sections(read(doc)), SECTION_PATTERNS["exclusions"])
        if not found:
            silent += 1
            continue
        for section in found:
            units = exclusion_units(section)
            if not units:
                result.findings.append(f"{doc.rel}: the exclusions section states none")
            for number, unit in units:
                result.examined += 1
                if not citation.search(unit):
                    result.findings.append(
                        f"{doc.rel}:{number}: cites no tracked item: {unit[:100]}"
                    )
    if silent:
        result.notes.append(
            f"{silent} SPEC(s) state no exclusions (spec-sections judges it)"
        )
    result.void_reason = f"no exclusion unit in {len(judged)} judged SPEC(s)"
    return result


def stated_criteria(sections: list[mp.Section]) -> set[str]:
    """Every criterion id the acceptance section states, in a table's first cell or a bullet."""
    stated = set()
    criterion = re.compile(rf"^{mp.CRITERION}$")
    bullet = re.compile(rf"^\s*[-*+]\s+\**`?({mp.CRITERION})`?\**[.:)]")
    for section in titled(sections, SECTION_PATTERNS["acceptance"]):
        for _, cells in mp.table_rows(section.prose()):
            first = cells[0].strip("*` ") if cells else ""
            if criterion.match(first):
                stated.add(first)
        for _, line in section.prose():
            matched = bullet.match(line)
            if matched:
                stated.add(matched.group(1))
    return stated


def acceptance_fenced(context: mp.Context) -> mp.Result:
    judged, notes, _, _ = mp.judged_specs(context)
    result = mp.Result(len(judged), "SPEC(s)", notes=list(notes))
    result.void_reason = no_specs(context, notes)
    commands = 0
    for doc in judged:
        text = read(doc)
        fence = mp.acceptance(text)
        if not fence.fenced:
            result.findings.append(f"{doc.rel}: no ```acceptance fence")
            continue
        for number, problem in fence.problems:
            where = doc.rel if number is None else f"{doc.rel}:{number}"
            result.findings.append(f"{where}: {problem}")
        commands += len(fence.lines)
        fenced = {criterion for _, criterion, _ in fence.lines}
        stated = stated_criteria(mp.sections(text))
        for criterion in sorted(stated - fenced):
            result.findings.append(
                f"{doc.rel}: criterion {criterion} has no line in the acceptance fence"
            )
        if stated:
            for criterion in sorted(fenced - stated):
                result.findings.append(
                    f"{doc.rel}: the fence names {criterion}, which the criteria do not state"
                )
    result.notes.insert(0, f"{commands} fenced command(s)")
    return result


def looks_like_path(token: str) -> bool:
    if token.startswith("-") or token.startswith("http"):
        return False
    return "/" in token or bool(FILE_NAME.match(token))


def manifest_present(context: mp.Context) -> mp.Result:
    judged, notes, _, _ = mp.judged_specs(context)
    result = mp.Result(len(judged), "SPEC(s)", notes=list(notes))
    result.void_reason = no_specs(context, notes)
    named = 0
    for doc in judged:
        found = titled(mp.sections(read(doc)), SECTION_PATTERNS["manifest"])
        if not found:
            result.findings.append(f"{doc.rel}: no manifest section")
            continue
        paths = [
            (number, token)
            for section in found
            for number, line in section.prose()
            for token in BACKTICKED.findall(line)
            if looks_like_path(token)
        ]
        if not paths:
            result.findings.append(f"{doc.rel}: the manifest names no repository path")
        for number, path in paths:
            if path.startswith(("/", "~")) or ".." in path.split("/"):
                result.findings.append(
                    f"{doc.rel}:{number}: names {path}, which is not repository-relative"
                )
        named += len(paths)
    result.notes.insert(0, f"{named} path(s) named")
    return result


def is_placeholder(cell: str) -> bool:
    text = cell.strip().strip("*`").strip()
    return not text or bool(re.fullmatch(r"[.…_\-\s]+|<[^>]*>", text))


def alternatives(section: mp.Section) -> list[tuple[int, str, bool]]:
    """Each alternative a section names: (line, the alternative, whether it states a reason)."""
    found = []
    for number, cells in mp.table_rows(section.prose()):
        if cells and not is_placeholder(cells[0]):
            reasoned = any(not is_placeholder(cell) for cell in cells[1:])
            found.append((number, cells[0].strip(), reasoned))
    for number, line in section.prose():
        item = re.match(r"^\s*[-*+]\s+(.*)$", line)
        if item is None or is_placeholder(item.group(1)):
            continue
        text = item.group(1).strip()
        parts = REASON.split(text, maxsplit=1)
        reasoned = len(parts) == 2 and not is_placeholder(parts[1])
        found.append((number, parts[0].strip(), reasoned))
    return found


def judged_adrs(context: mp.Context) -> tuple[list[mp.Doc], list[str]]:
    sdd = context.section("sdd")
    docs = mp.numbered_docs(context.root, sdd["decisions"], sdd["adr_prefix"])
    floor = sdd["adopted_from"]
    judged = [doc for doc in docs if not doc.amendment and doc.number >= floor]
    below = [doc for doc in docs if not doc.amendment and doc.number < floor]
    notes = (
        [f"{len(below)} below adopted_from {floor} counted, not judged"]
        if below
        else []
    )
    return judged, notes


def adr_alternatives(context: mp.Context) -> mp.Result:
    judged, notes = judged_adrs(context)
    result = mp.Result(len(judged), "ADR(s)", notes=notes)
    result.void_reason = f"no ADR to judge under {context.section('sdd')['decisions']}"
    total = 0
    for doc in judged:
        found = titled(mp.sections(read(doc)), ALTERNATIVES)
        if not found:
            result.findings.append(
                f"{doc.rel}: no section names what it was chosen against"
            )
            continue
        named = [item for section in found for item in alternatives(section)]
        if not named:
            result.findings.append(f"{doc.rel}: names no alternative")
        for number, name, reasoned in named:
            if not reasoned:
                result.findings.append(
                    f"{doc.rel}:{number}: the alternative `{name}` states no reason it lost"
                )
        total += len(named)
    result.notes.insert(0, f"{total} alternative(s)")
    return result


def adr_linked(context: mp.Context) -> mp.Result:
    sdd = context.section("sdd")
    judged, notes, _, _ = mp.judged_specs(context)
    result = mp.Result(len(judged), "SPEC(s)", notes=notes)
    result.void_reason = no_specs(context, notes)
    held = {
        doc.number
        for doc in mp.numbered_docs(context.root, sdd["decisions"], sdd["adr_prefix"])
    }
    named = re.compile(rf"(?<![A-Za-z0-9-]){re.escape(sdd['adr_prefix'])}(\d+)(?![\d])")
    for doc in judged:
        names = {int(number) for number in named.findall(read(doc))}
        for number in sorted(names - held):
            result.findings.append(
                f"{doc.rel}: names {sdd['adr_prefix']}{number}, "
                f"which is not in {sdd['decisions']}"
            )
        if doc.number not in held and not names & held:
            named_none = "it names none" if not names else "none it names exists"
            result.findings.append(
                f"{doc.rel}: no ADR carries its number and {named_none}"
            )
    return result


# ------------------------------------------------------------------------------ numbering

KINDS = (("SPEC", "specs", "spec_prefix"), ("ADR", "decisions", "adr_prefix"))


def git(root: Path, *args: str) -> str:
    done = subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=False
    )
    if done.returncode != 0:
        raise mp.ProbeError(f"git {' '.join(args)} failed: {done.stderr.strip()}")
    return done.stdout


def census(context: mp.Context, across: bool) -> tuple[dict, list[str]]:
    """{(kind, number): {file name: [where...]}} over the tree, and every ref and worktree."""
    sdd = context.section("sdd")
    pattern = {
        kind: re.compile(rf"^{re.escape(sdd[prefix])}(\d+)-(.+)\.md$")
        for kind, _, prefix in KINDS
    }
    held: dict[tuple[str, int], dict[str, list[str]]] = {}

    def add(kind: str, name: str, where: str) -> None:
        matched = pattern[kind].match(name)
        if matched:
            key = (kind, int(matched.group(1)))
            held.setdefault(key, {}).setdefault(name, []).append(where)

    def listing(base: Path, where: str) -> None:
        for kind, directory, _ in KINDS:
            folder = base / sdd[directory]
            if folder.is_dir():
                for path in sorted(folder.iterdir()):
                    if path.is_file():
                        add(kind, path.name, where)

    listing(context.root, "this tree")
    notes = []
    if across:
        refs = [
            ref
            for ref in git(context.root, "for-each-ref", "--format=%(refname)",
                           "refs/heads", "refs/remotes").split()
            if not ref.endswith("/HEAD")
        ]  # fmt: skip
        trees: dict[str, list[str]] = {}
        for ref in refs:
            for kind, directory, _ in KINDS:
                done = subprocess.run(
                    ["git", "-C", str(context.root), "rev-parse", "--verify", "-q",
                     f"{ref}:{sdd[directory]}"],
                    capture_output=True, text=True, check=False,
                )  # fmt: skip
                tree = done.stdout.strip()
                if done.returncode != 0 or not tree:
                    continue
                if tree not in trees:
                    trees[tree] = git(
                        context.root, "ls-tree", "--name-only", tree
                    ).split()
                for name in trees[tree]:
                    add(kind, name, ref)
        worktrees = [
            Path(line.split(" ", 1)[1])
            for line in git(
                context.root, "worktree", "list", "--porcelain"
            ).splitlines()
            if line.startswith("worktree ")
        ]
        others = [path for path in worktrees if path.resolve() != context.root]
        for path in others:
            listing(path, f"worktree {path}")
        notes.append(f"{len(refs)} ref(s) and {len(others)} other worktree(s) read")
    return held, notes


def numbering_unique(context: mp.Context) -> mp.Result:
    across = bool(getattr(context.args, "across_refs", False))
    held, notes = census(context, across)
    documents = sum(len(names) for names in held.values())
    result = mp.Result(documents, "document(s)", notes=notes)
    result.void_reason = "no SPEC or ADR anywhere this probe looked"
    for (kind, number), names in sorted(held.items()):
        claimants = sorted(name for name in names if not is_amendment(name, number))
        if len(claimants) > 1:
            shown = ", ".join(
                name + (f" ({', '.join(sorted(set(names[name])))})" if across else "")
                for name in claimants
            )
            result.findings.append(
                f"{kind} {number} is claimed by {len(claimants)} documents: {shown}"
            )
    return result


def is_amendment(name: str, number: int) -> bool:
    """Whether `name`, a document numbered `number`, is an amendment: its slug opens so.

    The slug is what follows the number's own digits, so a prefix holding digits of its own
    (`SPEC-V2-`) cannot be mistaken for the number.
    """
    matched = re.search(rf"0*{number}-(.+)\.md$", name)
    slug = matched.group(1) if matched else ""
    return slug == "amendment" or slug.startswith("amendment-")


def next_number(context: mp.Context) -> int:
    kind = context.args.kind.upper()
    held, notes = census(context, bool(context.args.across_refs))
    numbers = {number: names for (k, number), names in held.items() if k == kind}
    if numbers:
        highest = max(numbers)
        where = sorted({w for places in numbers[highest].values() for w in places})
        shown = f"{highest}, on {', '.join(where)}"
    else:
        highest, shown = 0, "none"
    tail = "".join(f"; {note}" for note in notes)
    print(f"SDD next {kind} number: {highest + 1} (highest held: {shown}){tail}")
    return mp.OK


def extend(check, sub) -> None:
    check.add_argument(
        "--across-refs",
        action="store_true",
        help="numbering-unique: read every git ref and worktree too",
    )
    following = sub.add_parser("next", help="print the next free SPEC or ADR number")
    following.add_argument("kind", choices=("spec", "adr"))
    following.add_argument("--across-refs", action="store_true")


CLASSES = {
    "spec-sections": (
        "every SPEC carries each required section, none empty",
        spec_sections,
    ),
    "exclusions-cited": (
        "every exclusion unit cites a tracked item",
        exclusions_cited,
    ),
    "acceptance-fenced": (
        "every stated criterion has a line in the acceptance fence, and back",
        acceptance_fenced,
    ),
    "manifest-present": (
        "the manifest names at least one repository-relative path",
        manifest_present,
    ),
    "adr-alternatives": (
        "every ADR names an alternative and the reason it lost",
        adr_alternatives,
    ),
    "adr-linked": ("every SPEC is decided by an ADR that exists", adr_linked),
    "numbering-unique": (
        "no number is held by two documents unless one is an amendment",
        numbering_unique,
    ),
}


if __name__ == "__main__":
    sys.exit(
        mp.run(
            "SDD",
            "sdd",
            CLASSES,
            extend=extend,
            verbs={"next": next_number},
            validate=validate,
        )
    )
