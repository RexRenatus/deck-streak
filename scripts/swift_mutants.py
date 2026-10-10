#!/usr/bin/env python3
"""swift_mutants: DeckStreak's hand-written Swift mutant rows, censused, swept and retired
(SPEC-397, ADR-411).

    python3 scripts/swift_mutants.py census [--root DIR]
    python3 scripts/swift_mutants.py retired --base REF [--root DIR]
    python3 scripts/swift_mutants.py sweep --package ios/<package> --report DIR --run-seconds N
                                           [--root DIR]

WHY. A Swift package's mutant rows live in its own `ios/<package>/swift-mutants.json`, and only a
macOS runner can run their killers. This one module reads them for every caller: `census` holds
each row's form on Linux, on every pull request; `retired` refuses a row that leaves while its
source file stays; and `sweep` proves a package's rows in that package's own macOS job, each
mutant against its one killer, bounded, and the file restored byte for byte after each.

It imports only the standard library, so the macOS jobs need only its own path in their trigger.
"""

import argparse
import dataclasses
import hashlib
import json
import os
import pathlib
import re
import signal
import subprocess
import sys
import time

ROW_FILE = "swift-mutants.json"
RETIRED = "scripts/mutation-rows.retired.json"
TOP_KEYS = ("population", "mutants")
KEYS = ("id", "file", "find", "replace", "killer", "why")
ID = re.compile(r"SW\d{5}")
#: `<Target>.<Class>/<method>`, the selector `swift test --filter` takes.
KILLER = re.compile(r"([A-Za-z_]\w*)\.([A-Za-z_]\w*)/([A-Za-z_]\w*)")
#: The test runner's case lines, as the base's inline programs read them.
STARTED = re.compile(r"^Test Case '.*' started\.$", re.MULTILINE)
FAILED = re.compile(r"^Test Case '.*' failed \(", re.MULTILINE)
EXECUTED = re.compile(r"Executed (\d+) tests?, with")

EXIT_OK, EXIT_SURVIVED, EXIT_REFUSED, EXIT_VOID, EXIT_RESTORE = 0, 1, 2, 3, 4

KILLED = "KILLED"
SURVIVED = "SURVIVED"
NOT_GREEN = "VOID: its killer is not green on one test unmutated"
NOT_RESTORED = "VOID: the file was not restored byte for byte"
PAST_BOUND = "VOID: the killer ran past its bound, and its process group was ended"


@dataclasses.dataclass(frozen=True)
class Run:
    """What one killer run observed: the cases started and failed, the last `Executed` count (or
    `none`), the exit, whether it ran past its bound, and its seconds."""

    started: int
    failed: int
    executed: str
    code: int
    timed_out: bool = False
    seconds: float = 0.0


#: The mutated run of a row whose mutant was never installed.
UNRUN = Run(started=0, failed=0, executed="none", code=0)


def baseline_green(run: Run) -> bool:
    """Whether an unmutated run is green on exactly one test, within its bound (SPEC-397 R6)."""
    return (
        run.started == 1
        and run.failed == 0
        and run.executed == "1"
        and run.code == 0
        and not run.timed_out
    )


def verdict(baseline: Run, occurs: int, restored: bool, mutated: Run) -> str:
    """One row's verdict from what its runs observed, first match wins (SPEC-397 R6)."""
    if not baseline_green(baseline):
        return NOT_GREEN
    if occurs != 1:
        return f"VOID: its find occurs {occurs} times"
    if not restored:
        return NOT_RESTORED
    if mutated.timed_out:
        return PAST_BOUND
    if mutated.started == 1 and mutated.failed == 1 and mutated.code != 0:
        return KILLED
    if mutated.started == 1 and mutated.failed == 0 and mutated.code == 0:
        return SURVIVED
    return (
        f"VOID: {mutated.started} started, {mutated.failed} failed, exit {mutated.code}, "
        "so the killer did not judge it"
    )


# --------------------------------------------------------------------------- the census


def row_files(root: pathlib.Path) -> list[pathlib.Path]:
    """Every package's row file under `root`, sorted."""
    return sorted((root / "ios").glob(f"*/{ROW_FILE}"))


def killer_problems(package: pathlib.Path, killer: str) -> list[tuple[str, str]]:
    """Each way a killer fails to name exactly one test method of one class in its target."""
    match = KILLER.fullmatch(killer)
    if not match:
        return [("killer-form", f"{killer!r} is not <Target>.<Class>/<method>")]
    target, name, method = match.groups()
    found = []
    if not method.startswith("test"):
        found.append(("killer-not-a-test", f"{method} does not start with test"))
    tests = package / "Tests" / target
    if not tests.is_dir():
        return [*found, ("target-absent", f"no directory Tests/{target}")]
    declares = re.compile(rf"\bclass\s+{re.escape(name)}\b")
    homes = []
    for path in sorted(tests.rglob("*.swift")):
        homes += [path] * len(declares.findall(path.read_text(encoding="utf-8")))
    if not homes:
        return [
            *found,
            ("class-absent", f"no file under Tests/{target} declares class {name}"),
        ]
    if len(homes) > 1:
        return [
            *found,
            ("class-repeated", f"class {name} is declared {len(homes)} times"),
        ]
    text = homes[0].read_text(encoding="utf-8")
    methods = len(re.findall(rf"\bfunc\s+{re.escape(method)}\s*\(", text))
    if methods == 0:
        found.append(("method-absent", f"class {name} declares no func {method}("))
    elif methods > 1:
        found.append(("method-repeated", f"func {method}( is declared {methods} times"))
    return found


def row_problems(package: pathlib.Path, row: object, seen: dict, name: str) -> list:
    """Each way one row breaks the form SPEC-397 R2 states, as (reason, detail)."""
    if not isinstance(row, dict):
        return [("row-form", "the row is not an object")]
    if set(row) != set(KEYS):
        missing = sorted(set(KEYS) - set(row))
        extra = sorted(set(row) - set(KEYS))
        return [("keys", f"missing {missing}, extra {extra}")]
    loose = [key for key in KEYS if not isinstance(row[key], str)]
    if loose:
        return [("row-form", f"not a string: {', '.join(loose)}")]
    found = []
    row_id = row["id"]
    if not ID.fullmatch(row_id):
        found.append(("id-form", f"{row_id!r} is not SW and five digits"))
    elif row_id in seen:
        found.append(("id-repeated", f"{row_id} is also in {seen[row_id]}"))
    else:
        seen[row_id] = name
    relative = pathlib.PurePosixPath(row["file"])
    source = None
    if relative.is_absolute() or ".." in relative.parts or relative.parts[:1] != ("Sources",):
        found.append(("file-outside-sources", f"{row['file']} is not under Sources/"))
    elif not (package / relative).is_file():
        found.append(("file-absent", f"{row['file']} does not exist"))
    else:
        source = (package / relative).read_text(encoding="utf-8")
    if not row["find"]:
        found.append(("find-empty", "the find is empty"))
    elif row["find"] == row["replace"]:
        found.append(("find-equals-replace", "the find equals its replacement"))
    elif source is not None and source.count(row["find"]) != 1:
        found.append(
            (
                "find-not-once",
                f"the find occurs {source.count(row['find'])} times in {relative}",
            )
        )
    if not row["why"].strip():
        found.append(("why-empty", "the reason is empty"))
    return found + killer_problems(package, row["killer"])


def census_files(root: pathlib.Path, paths: list[pathlib.Path]) -> tuple[list[str], list, int]:
    """(refusal lines, the rows read, the rows examined) over `paths`; ids are unique across all."""
    problems, rows, examined, seen = [], [], 0, {}
    for path in paths:
        name = path.relative_to(root).as_posix()

        def refuse(reason: str, label: str, detail: str, name: str = name) -> None:
            problems.append(f"refused {reason}: {name}: {label}: {detail}")

        if not path.is_file():
            refuse("no-row-file", "the file", "it does not exist")
            continue
        try:
            document = json.loads(path.read_text(encoding="utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            refuse("not-json", "the file", str(error))
            continue
        if not isinstance(document, dict) or sorted(document) != sorted(TOP_KEYS):
            refuse(
                "top-level-keys",
                "the file",
                "its keys are not exactly population, mutants",
            )
            document = document if isinstance(document, dict) else {}
        population = document.get("population")
        if not isinstance(population, str) or not population.strip():
            refuse("population", "the file", "the population is not a non-empty string")
        mutants = document.get("mutants")
        if not isinstance(mutants, list):
            refuse("mutants", "the file", "the mutants are not a list")
            continue
        for index, row in enumerate(mutants):
            examined += 1
            label = row.get("id") if isinstance(row, dict) else None
            label = label if isinstance(label, str) else f"row {index}"
            found = row_problems(path.parent, row, seen, name)
            for reason, detail in found:
                refuse(reason, label, detail)
            rows.append(row)
    return problems, rows, examined


def census(root: pathlib.Path) -> int:
    files = row_files(root)
    problems, _rows, examined = census_files(root, files)
    for line in problems:
        print(line)
    print(f"examined {examined} row(s) in {len(files)} file(s)")
    if problems:
        return EXIT_REFUSED
    return EXIT_OK if examined else EXIT_VOID


# --------------------------------------------------------------------------- departures


def git(root: pathlib.Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=True
    ).stdout


def rows_at(root: pathlib.Path, base: str) -> list[tuple[str, str, str]]:
    """(package, row id, file) of every row the revision `base` holds."""
    listed = git(root, "ls-tree", "-r", "--name-only", "-z", base, "--", "ios")
    found = []
    for name in sorted(entry for entry in listed.split("\0") if entry):
        parts = name.split("/")
        if len(parts) != 3 or parts[2] != ROW_FILE:
            continue
        document = json.loads(git(root, "show", f"{base}:{name}"))
        for row in document.get("mutants", []):
            found.append((parts[1], str(row.get("id")), str(row.get("file"))))
    return found


def approvals(root: pathlib.Path) -> dict[str, str]:
    """{id: approval} of every entry the approvals record holds with a reason and an approval."""
    path = root / RETIRED
    if not path.is_file():
        return {}
    document = json.loads(path.read_text(encoding="utf-8"))
    return {
        str(entry.get("id")): str(entry.get("approval"))
        for entry in document.get("retired", [])
        if str(entry.get("reason", "")).strip() and str(entry.get("approval", "")).strip()
    }


def retired(root: pathlib.Path, base: str) -> int:
    before = rows_at(root, base)
    now = set()
    for path in row_files(root):
        document = json.loads(path.read_text(encoding="utf-8"))
        now |= {str(row.get("id")) for row in document.get("mutants", [])}
    approved = approvals(root)
    refused = False
    for package, row_id, file in before:
        if row_id in now:
            continue
        source = f"ios/{package}/{file}"
        if not (root / source).exists():
            print(f"retired: {row_id}: its file {source} left with it")
        elif row_id in approved:
            print(f"retired: {row_id}: its file stays; retired with approval: {approved[row_id]}")
        else:
            refused = True
            print(
                f"retired: {row_id}: REFUSED: it left while its file {source} stays, and "
                f"{RETIRED} records no reason and approval for it"
            )
    print(f"examined {len(before)}")
    return EXIT_SURVIVED if refused else EXIT_OK


# --------------------------------------------------------------------------- the sweep


def run_killer(root: pathlib.Path, package: str, killer: str, run_seconds: int) -> Run:
    """The killer alone, in a session of its own, bounded by `run_seconds`: past the bound its
    whole process group is ended, so no test process it started outlives the run."""
    selector = "^" + re.escape(killer) + "$"
    start = time.monotonic()
    proc = subprocess.Popen(
        ["swift", "test", "--package-path", package, "--filter", selector],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        # An `errors` handler alone opens the pipe as text, so a byte that is not UTF-8 is replaced.
        errors="replace",
        start_new_session=True,
    )
    timed_out = False
    try:
        out, _ = proc.communicate(timeout=run_seconds)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        out, _ = proc.communicate()
    executed = EXECUTED.findall(out)
    return Run(
        started=len(STARTED.findall(out)),
        failed=len(FAILED.findall(out)),
        executed=executed[-1] if executed else "none",
        code=proc.returncode,
        timed_out=timed_out,
        seconds=time.monotonic() - start,
    )


def restore(path: pathlib.Path, original: bytes, digest: str, row_id: str) -> None:
    """Write the file back and check its sha256; on a mismatch, exit 4 at once."""
    path.write_bytes(original)
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        print(f"{row_id}: {NOT_RESTORED}")
        print(f"sweep: REFUSED: {path} was not restored; stopping before another row")
        sys.stdout.flush()
        sys.exit(EXIT_RESTORE)


def sweep(root: pathlib.Path, package: str, report: pathlib.Path, run_seconds: int) -> int:
    problems, rows, _examined = census_files(root, [root / package / ROW_FILE])
    for line in problems:
        print(line)
    if problems:
        print(f"sweep: REFUSED: the census refused {package}'s rows; nothing was applied")
        sys.stdout.flush()
        return EXIT_REFUSED
    print(f"examined {len(rows)} mutant row(s)", flush=True)
    baseline = {}
    for killer in sorted({row["killer"] for row in rows}):
        run = baseline[killer] = run_killer(root, package, killer, run_seconds)
        bound = ", past its bound" if run.timed_out else ""
        print(
            f"unmutated {killer}: {run.started} started, {run.failed} failed, "
            f"Executed {run.executed}, exit {run.code}, {run.seconds:.1f} s{bound}",
            flush=True,
        )
    lines = ["| id | verdict | why |", "|---|---|---|"]
    tally = {KILLED: 0, SURVIVED: 0, "VOID": 0}
    for row in rows:
        path = root / package / row["file"]
        original = path.read_bytes()
        digest = hashlib.sha256(original).hexdigest()
        text = original.decode("utf-8")
        occurs = text.count(row["find"])
        mutated = UNRUN
        if baseline_green(baseline[row["killer"]]) and occurs == 1:
            try:
                path.write_bytes(text.replace(row["find"], row["replace"]).encode("utf-8"))
                mutated = run_killer(root, package, row["killer"], run_seconds)
            finally:
                restore(path, original, digest, row["id"])
        result = verdict(baseline[row["killer"]], occurs, True, mutated)
        tally[result if result in tally else "VOID"] += 1
        seconds = "" if mutated is UNRUN else f", {mutated.seconds:.1f} s"
        print(f"{row['id']}: {result}{seconds}", flush=True)
        lines.append(f"| {row['id']} | {result} | {row['why']} |")
    last = (
        f"swift-mutants {package}: examined {len(rows)} row(s): {tally[KILLED]} killed, "
        f"{tally[SURVIVED]} survived, {tally['VOID']} void"
    )
    lines += ["", last]
    report.mkdir(parents=True, exist_ok=True)
    (report / "sweep.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with pathlib.Path(summary).open("a", encoding="utf-8") as handle:
            handle.write("\n".join(lines) + "\n")
    print(last)
    sys.stdout.flush()
    if not rows:
        return EXIT_VOID
    return EXIT_OK if tally[KILLED] == len(rows) else EXIT_SURVIVED


# --------------------------------------------------------------------------- the command line


def end_on_sigterm() -> None:
    """A SIGTERM ends the sweep through its `finally` blocks, so the file in flight is restored:
    the killer leads a session of its own, so a job's cancel no longer reaches it directly."""
    signal.signal(signal.SIGTERM, lambda signum, _frame: sys.exit(128 + signum))


def main(argv: list[str] | None = None) -> int:
    end_on_sigterm()
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=["census", "retired", "sweep"])
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parents[1]))
    parser.add_argument("--base", help="retired: the revision to compare the rows against")
    parser.add_argument("--package", help="sweep: the package directory, ios/<package>")
    parser.add_argument("--report", help="sweep: the directory sweep.md is written to")
    parser.add_argument("--run-seconds", type=int, help="sweep: the bound on each killer run")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.root).resolve()
    if args.verb == "census":
        return census(root)
    if args.verb == "retired":
        if not args.base:
            parser.error("retired needs --base")
        try:
            return retired(root, args.base)
        except (
            OSError,
            json.JSONDecodeError,
            subprocess.CalledProcessError,
        ) as refusal:
            print(f"retired: REFUSED: {refusal}")
            return EXIT_REFUSED
    if not (args.package and args.report and args.run_seconds):
        parser.error("sweep needs --package, --report and --run-seconds")
    if args.run_seconds < 1:
        parser.error("--run-seconds must be at least 1")
    package = pathlib.PurePosixPath(args.package).as_posix()
    return sweep(root, package, pathlib.Path(args.report), args.run_seconds)


if __name__ == "__main__":
    sys.exit(main())
