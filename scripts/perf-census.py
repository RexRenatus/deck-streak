#!/usr/bin/env python3
"""The bench census (SPEC-630, ADR-652): which functions earn a bench, and whether the benches
still reach them.

    perf-census.py derive --listing FILE --share S --source-root DIR --workload PATH
                          --workload-sha256 HEX --derived-at SHA --release TAG --toolchain T
                          --valgrind V --hot-set OUT --bench-reach OUT
                          [--reach BENCH=FILE]... [--untoggled FILE]...
    perf-census.py check --root DIR [--enforce]

`derive` is the parse and write half. It reads listings that `callgrind_annotate --inclusive=no
--threshold=<S> --auto=no` printed: the study day's at the share S, and each bench's toggle and
the untoggled bench run at 100, every function. It strips Rust legacy hash suffixes, keeps the
path and the file (relative under the source root), merges equal entries, and writes the hot set
and the bench reach with every schema key. It refuses a listing it cannot read whole, or a study
day listing that does not stop at the share, and then writes nothing. The facts a listing cannot
give (the workload's sha256, derived_at, the release, the toolchain, Valgrind's version) come from
the arguments.

`check` judges `<root>/perf/hot-set.json`, `<root>/perf/bench-reach.json` and the tree at `<root>`
in five arms: `shape`, `unreached`, `stale`, `void` and `edge`, which only informs. It reads git at
`<root>`. It is report-only and exits 0 unless `--enforce` is given; then any refusal of the first
four arms exits 1. Standard library only.
"""

import argparse
import json
import re
import subprocess
import sys
from fractions import Fraction
from pathlib import Path, PurePosixPath

HOT_SCHEMA = "deckstreak.perf-hot-set.v1"
REACH_SCHEMA = "deckstreak.perf-bench-reach.v1"
REQUIRED = (
    "--listing",
    "--share",
    "--source-root",
    "--workload",
    "--workload-sha256",
    "--derived-at",
    "--release",
    "--toolchain",
    "--valgrind",
    "--hot-set",
    "--bench-reach",
)
#: The threshold a bench's toggle listing and the untoggled listing are printed at.
EVERY = 100
#: How near the cut a member's cumulative share may end before `edge` names it.
HALF = Fraction("0.5")
CREATOR = re.compile(r"^Profile data file '[^']*' \(creator: (.*)\)$", re.M)
EVENTS = re.compile(r"^Events shown: +(.*)$", re.M)
THRESHOLD = re.compile(r"^Thresholds: +([\d.]+)$", re.M)
TOTALS = re.compile(r"^ *([\d,]+)(?: \( *[\d.]+%\))? +PROGRAM TOTALS(?: \(calculated\))?$", re.M)
HEADER = re.compile(r"^Ir +file:function\n-+\n", re.M)
ENTRY = re.compile(r" *([\d,]+)(?: \( *[\d.]+%\))? +(\S.*?)(?: \[[^\[\]]*\])?")
HASH = re.compile(r"::h[0-9a-f]{16}(?![0-9A-Za-z_])")
RUST_BENCH = re.compile(r"#\[divan::bench(?:\([^)]*\))?\]\s*(?:pub\s+)?fn\s+(\w+)")
WEB_BENCH = re.compile(r"\bbench\(\s*(['\"])(.+?)\1")
SEMVER = re.compile(r"v(\d+)\.(\d+)\.(\d+)")


class Refused(Exception):
    """A listing `derive` cannot read whole, or one that does not stop at the share."""


def ir(text):
    return int(text.replace(",", ""))


def percent(part, total):
    return Fraction(part * 100, total)


def normal(fn):
    return HASH.sub("", fn)


def local(file, root):
    """`file` relative to the source root, or None for a file outside it."""
    try:
        return PurePosixPath(file).relative_to(root).as_posix()
    except ValueError:
        return None


def listing(path, valgrind):
    """A listing's threshold, its program total and its (count, function, file) entries."""
    name = Path(path).name
    text = Path(path).read_text(encoding="utf-8")
    creator = CREATOR.search(text)
    if creator is None or creator.group(1) != f"callgrind-{valgrind}":
        raise Refused(f"{name} was not made by callgrind-{valgrind}")
    events = EVENTS.search(text)
    if events is None or events.group(1) != "Ir":
        raise Refused(f"{name} shows events other than Ir")
    threshold, totals = THRESHOLD.search(text), TOTALS.search(text)
    if threshold is None or totals is None:
        raise Refused(f"{name} has no thresholds line or no program totals line")
    header = HEADER.search(text)
    if header is None:
        raise Refused(f"{name} has no file:function header")
    entries = []
    for line in text[header.end() :].split("\n\n")[0].splitlines():
        found = ENTRY.fullmatch(line)
        if found is None:
            raise Refused(f"{name}: {line!r} is no function line")
        file, _colon, fn = found.group(2).partition(":")
        entries.append((ir(found.group(1)), fn, file))
    return threshold.group(1), ir(totals.group(1)), entries


def whole(path, threshold):
    """Refuse a bench or untoggled listing that does not print every function."""
    if Fraction(threshold) != EVERY:
        raise Refused(
            f"{Path(path).name} was printed at the threshold {threshold}%, not at {EVERY}% "
            "(every function)"
        )


def hot_set(args, share):
    """The study day's program total and its members, the listing stopping at the share."""
    threshold, total, entries = listing(args.listing, args.valgrind)
    name = Path(args.listing).name
    if Fraction(threshold) != share:
        raise Refused(
            f"{name} was printed at the threshold {threshold}%, not at the share {float(share):g}%"
        )
    if not total:
        raise Refused(f"{name} counted 0 instructions")
    counts = [count for count, _fn, _file in entries]
    for index in range(len(counts)):
        if percent(sum(counts[:index]), total) >= share:
            raise Refused(f"{name} goes on past the cut at {float(share):g}%")
    if percent(sum(counts), total) < share:
        raise Refused(f"{name} sums short of the share {float(share):g}%")
    merged = {}
    for count, fn, file in entries:
        key = (normal(fn), local(file, args.source_root) or file)
        merged[key] = merged.get(key, 0) + count
    rows = sorted((-count, fn, file) for (fn, file), count in merged.items())
    members = [
        {
            "fn": fn,
            "file": file,
            "self_ir": -negative,
            "share": float(round(percent(-negative, total), 4)),
        }
        for negative, fn, file in rows
    ]
    return total, members


def reach(args):
    """Each bench's toggle total and the functions it reaches, and the untoggled files."""
    benches = {}
    for spec in args.reach:
        bench, _equals, path = spec.partition("=")
        threshold, total, entries = listing(path, args.valgrind)
        whole(path, threshold)
        functions = sorted({normal(fn) for count, fn, _file in entries if count})
        benches[bench] = {"total_ir": total, "functions": functions}
    files = set()
    for path in args.untoggled:
        threshold, _total, entries = listing(path, args.valgrind)
        whole(path, threshold)
        files.update(local(file, args.source_root) for _count, _fn, file in entries)
    return {bench: benches[bench] for bench in sorted(benches)}, sorted(files - {None})


def write(path, doc):
    Path(path).write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")


def derive(args):
    share = Fraction(args.share)
    try:
        total, members = hot_set(args, share)
        benches, files = reach(args)
    except Refused as problem:
        print(f"perf-census: derive: {problem}")
        return 1
    write(
        args.hot_set,
        {
            "schema": HOT_SCHEMA,
            "share": float(share),
            "workload": {"path": args.workload, "sha256": args.workload_sha256},
            "derived_at": args.derived_at,
            "release": args.release,
            "toolchain": args.toolchain,
            "valgrind": args.valgrind,
            "total_ir": total,
            "members": members,
        },
    )
    write(
        args.bench_reach,
        {
            "schema": REACH_SCHEMA,
            "derived_at": args.derived_at,
            "benches": benches,
            "files": files,
        },
    )
    print(
        f"perf-census: derive wrote {len(members)} members at {float(share):g}%, "
        f"{len(benches)} benches, {len(files)} files"
    )
    return 0


# ---------------------------------------------------------------------------- check: shape


class Each:
    """The rule of an object whose every value, whatever its key, follows `rule`."""

    def __init__(self, rule):
        self.rule = rule


def text(value):
    return isinstance(value, str) and value != ""


def count(value):
    return type(value) is int and value >= 0


def percentage(value):
    return type(value) in (int, float) and 0 < value <= 100


def names(value):
    return isinstance(value, list) and all(text(item) for item in value)


def exactly(expected):
    return lambda value: value == expected


def matching(pattern):
    compiled = re.compile(pattern)
    return lambda value: isinstance(value, str) and compiled.fullmatch(value) is not None


HOT_RULE = {
    "schema": exactly(HOT_SCHEMA),
    "share": percentage,
    "workload": {"path": text, "sha256": matching(r"[0-9a-f]{64}")},
    "derived_at": matching(r"[0-9a-f]{40}"),
    "release": matching(SEMVER.pattern),
    "toolchain": text,
    "valgrind": text,
    "total_ir": count,
    "members": [{"fn": text, "file": text, "self_ir": count, "share": percentage}],
}
REACH_RULE = {
    "schema": exactly(REACH_SCHEMA),
    "derived_at": matching(r"[0-9a-f]{40}"),
    "benches": Each({"total_ir": count, "functions": names}),
    "files": names,
}


def shape(where, value, rule, problems):
    """Append to `problems` each way `value` at `where` leaves `rule`."""
    if isinstance(rule, dict):
        if not isinstance(value, dict):
            problems.append(f"{where} is not an object")
            return
        problems.extend(f"{where} has the unknown key {key!r}" for key in value if key not in rule)
        for key, inner in rule.items():
            if key in value:
                shape(f"{where}/{key}", value[key], inner, problems)
            else:
                problems.append(f"{where} has no key {key!r}")
    elif isinstance(rule, Each):
        if not isinstance(value, dict):
            problems.append(f"{where} is not an object")
            return
        for key, item in value.items():
            shape(f"{where}/{key}", item, rule.rule, problems)
    elif isinstance(rule, list):
        if not isinstance(value, list):
            problems.append(f"{where} is not a list")
            return
        for index, item in enumerate(value):
            shape(f"{where}/{index}", item, rule[0], problems)
    elif not rule(value):
        problems.append(f"{where} {value!r} is outside the schema")


def loaded(root, name, rule, problems):
    """The committed file `name` read and shaped, or None when it is absent or not JSON."""
    path = root / "perf" / name
    if not path.is_file():
        problems.append(f"{name} is absent")
        return None
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except ValueError:
        problems.append(f"{name} is not JSON")
        return None
    shape(name, doc, rule, problems)
    return doc


# ---------------------------------------------------------------------------- check: the arms


def defined(root):
    """The benches the tree defines: `<file stem>::<fn>` for each divan bench under
    `crates/*/benches/` and `<file name less .bench.ts>::<name>` for each web bench."""
    found = set()
    for path in root.glob("crates/*/benches/*.rs"):
        source = path.read_text(encoding="utf-8")
        found.update(f"{path.stem}::{fn}" for fn in RUST_BENCH.findall(source))
    for path in root.glob("web/app/bench/**/*.bench.ts"):
        stem = path.name.removesuffix(".bench.ts")
        source = path.read_text(encoding="utf-8")
        found.update(f"{stem}::{name}" for _quote, name in WEB_BENCH.findall(source))
    return sorted(found)


def unreached(hot, reach, tree):
    """A line for each member no bench the tree defines reaches, naming each recorded bench
    that reached it and is gone."""
    benches = reach["benches"]
    reached = {fn for bench in benches if bench in tree for fn in benches[bench]["functions"]}
    lost = []
    for member in hot["members"]:
        if member["fn"] in reached:
            continue
        line = f"member {member['fn']} ({member['file']}) is reached by no bench the tree defines"
        for bench in sorted(benches):
            if member["fn"] in benches[bench]["functions"]:
                line += f"; {bench} reached it, and the tree no longer defines that bench"
        lost.append(line)
    return lost


def git(root, *args):
    return subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=False
    )


def second_parent(root, tag):
    done = git(root, "rev-parse", "--verify", "-q", f"refs/tags/{tag}^2")
    return done.stdout.strip() if done.returncode == 0 else None


def stale(root, derived):
    """The stale line, whether it refuses (a VOID refuses), and the SemVer release tag count:
    derived_at must be an ancestor of HEAD and the dev parent of one of the two newest release
    tags, each tag's dev parent being its merge commit's second parent."""
    listed = git(root, "tag", "--list").stdout.split()
    tags = sorted(
        (tag for tag in listed if SEMVER.fullmatch(tag)),
        key=lambda tag: tuple(int(part) for part in SEMVER.fullmatch(tag).groups()),
        reverse=True,
    )
    short = derived[:8]
    ancestor = git(root, "merge-base", "--is-ancestor", derived, "HEAD").returncode
    if ancestor == 1:
        return f"derived_at {short} is not an ancestor of HEAD", True, len(tags)
    if ancestor != 0:
        return f"derived_at {short} names no commit of the repository", True, len(tags)
    if not tags:
        return "VOID the repository holds no SemVer release tag", True, len(tags)
    window = tags[:2]
    parents = {tag: second_parent(root, tag) for tag in window}
    missing = [tag for tag in window if parents[tag] is None]
    if missing:
        return (
            f"VOID {missing[0]} has no second parent, and every release merges with a merge commit",
            True,
            len(tags),
        )
    dev_parents = [parents[tag] for tag in window]
    if derived in dev_parents:
        place = dev_parents.index(derived)
        which = "the newest release tag" if place == 0 else "the release tag before the newest"
        return f"OK, derived_at is the dev parent of {window[place]}, {which}", False, len(tags)
    if len(window) == 1:
        return (
            f"derived_at {short} is not the dev parent of {window[0]}, the one release tag",
            True,
            len(tags),
        )
    return (
        f"derived_at {short} is the dev parent of neither {window[0]} nor {window[1]}",
        True,
        len(tags),
    )


def void(hot, reach):
    """A line for an empty hot set, a hot set of 0 instructions and each bench of 0."""
    voids = []
    if not hot["members"]:
        voids.append("the hot set has no member")
    if not hot["total_ir"]:
        voids.append("the hot set counted 0 instructions (total_ir 0)")
    for bench in sorted(reach["benches"]):
        if not reach["benches"][bench]["total_ir"]:
            voids.append(f"bench {bench} counted 0 instructions inside its toggle")
    return voids


def edge(hot):
    """A line for each member whose cumulative share ends within half a point of the cut."""
    members, total = hot["members"], hot["total_ir"]
    near = []
    if total:
        cut = Fraction(hot["share"])
        for k, member in enumerate(members, 1):
            ends = percent(sum(each["self_ir"] for each in members[:k]), total)
            gap = abs(ends - cut)
            if gap <= HALF:
                near.append(
                    f"member {k} of {len(members)}, {member['fn']}, ends at {float(ends):.2f}% "
                    f"of the instructions, {float(gap):.2f} from the cut at {hot['share']:g}%"
                )
    return near


def check(args):
    root = Path(args.root)
    problems = []
    hot = loaded(root, "hot-set.json", HOT_RULE, problems)
    reach = loaded(root, "bench-reach.json", REACH_RULE, problems)
    if not problems and hot["derived_at"] != reach["derived_at"]:
        problems.append("hot-set.json and bench-reach.json name different derived_at commits")
    if problems:
        said = [f"shape: {problem}" for problem in problems]
        members = benches = tags = 0
        refused = len(problems)
    else:
        tree = defined(root)
        members, benches = len(hot["members"]), len(tree)
        lost = unreached(hot, reach, tree)
        stale_line, stale_refused, tags = stale(root, hot["derived_at"])
        voids = void(hot, reach)
        reached = f"OK, {members} members, each reached by a bench the tree defines"
        said = [
            "shape: OK",
            f"benches: the tree defines {benches}: {', '.join(tree)}",
            *[f"unreached: {line}" for line in lost or [reached]],
            f"stale: {stale_line}",
            *[f"void: {line}" for line in voids or ["OK"]],
            *[
                f"edge: {line}"
                for line in edge(hot) or ["OK, no member ends within half a point of the cut"]
            ],
        ]
        refused = len(lost) + stale_refused + len(voids)
    mode = "enforced" if args.enforce else "report-only"
    for line in said:
        print(f"perf-census: {line}")
    print(
        f"perf-census: check examined {members} members, {benches} defined benches, "
        f"{tags} release tags; {refused} refused ({mode})"
    )
    return 1 if args.enforce and refused else 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(required=True)
    derived = modes.add_parser("derive")
    for name in REQUIRED:
        derived.add_argument(name, required=True)
    derived.add_argument("--reach", action="append", default=[])
    derived.add_argument("--untoggled", action="append", default=[])
    derived.set_defaults(run=derive)
    checked = modes.add_parser("check")
    checked.add_argument("--root", required=True)
    checked.add_argument("--enforce", action="store_true")
    checked.set_defaults(run=check)
    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(int(main(sys.argv[1:])))
