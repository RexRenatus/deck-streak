"""SPEC-397: the Swift mutant rows' one reader, `scripts/swift_mutants.py` (A1-A6, A11).

A1 reads the live tree. A2-A5 drive the module's command line over trees they plant under a
temporary directory. A5's `swift` is a planted shell script first on `PATH`: it prints the test
runner's case lines, in the form the base's inline sweep programs read, as markers in the planted
source decide. A6 calls the verdict function over every class of its inputs, against a table the
test writes from R6. A11 reads the module's source.
"""

import ast
import contextlib
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

from _support import REPO, examined

MODULE = REPO / "scripts" / "swift_mutants.py"
#: The bound on each run of the module a test makes (SPEC-397 A5): a mutant that removes the
#: module's own bound on a killer run fails A5 by this bound instead of hanging it.
RUN_BOUND = 20
#: How long the planted killer that hangs, and the grandchild it starts, sleep: past RUN_BOUND.
HANG_SECONDS = 30
#: How long A5 waits for the hanging killer's grandchild to end once the sweep has returned.
GRANDCHILD_SECONDS = 10

ALPHA = "ios/Alpha/swift-mutants.json"
BETA = "ios/Beta/swift-mutants.json"
ALPHA_SOURCE = "ios/Alpha/Sources/Alpha/Code.swift"
ALPHA_TESTS = "ios/Alpha/Tests/AlphaTests/CodeTests.swift"


def runner_module():
    """The module imported, for A6's calls of the verdict function."""
    scripts = str(REPO / "scripts")
    if scripts not in sys.path:
        sys.path.insert(0, scripts)
    import swift_mutants

    return swift_mutants


def module(root, *args, env=None):
    """The module's command line over `root`, bounded by RUN_BOUND seconds."""
    return subprocess.run(
        [sys.executable, str(MODULE), *args, "--root", str(root)],
        capture_output=True,
        text=True,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1", **(env or {})),
        timeout=RUN_BOUND,
        check=False,
    )


def git(root, *args):
    """A fixture repository's git, with a fixture identity and no signing."""
    return subprocess.run(
        [
            "git",
            "-C",
            str(root),
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            *args,
        ],
        capture_output=True,
        text=True,
        timeout=RUN_BOUND,
        check=True,
    )


def swift_test_file(name, *methods):
    """A Swift test file that declares `name` with an empty test method per name in `methods`."""
    body = "".join(f"    func {method}() {{}}\n" for method in methods)
    return f"import XCTest\n\nfinal class {name}: XCTestCase {{\n{body}}}\n"


def swift_row(row_id, file, find, replace, killer, why):
    return {
        "id": row_id,
        "file": file,
        "find": find,
        "replace": replace,
        "killer": killer,
        "why": why,
    }


def alpha_beta():
    """A planted tree of two packages with one clean row each, as {path: text or JSON value}."""
    tree = {}
    for package, row_id in (("Alpha", "SW00001"), ("Beta", "SW00002")):
        tree[f"ios/{package}/Sources/{package}/Code.swift"] = "let kills = 1\nlet survives = 1\n"
        tree[f"ios/{package}/Tests/{package}Tests/CodeTests.swift"] = swift_test_file(
            "CodeTests", "test_kills", "test_survives"
        )
        tree[f"ios/{package}/swift-mutants.json"] = {
            "population": f"the {package} package's sources",
            "mutants": [
                swift_row(
                    row_id,
                    f"Sources/{package}/Code.swift",
                    "let kills = 1",
                    "let kills = 0",
                    f"{package}Tests.CodeTests/test_kills",
                    "the kill marker",
                )
            ],
        }
    return tree


def alpha(tree):
    """The planted Alpha package's one row, to edit in place."""
    return tree[ALPHA]["mutants"][0]


def write_tree(root, tree):
    for relative, content in tree.items():
        path = Path(root) / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        text = content if isinstance(content, str) else json.dumps(content, indent=2) + "\n"
        path.write_text(text, encoding="utf-8")


def refusals(done):
    """(reason, file, row) of each refusal line the census printed."""
    found = []
    for line in done.stdout.splitlines():
        if line.startswith("refused "):
            reason, file, row = line[len("refused ") :].split(": ")[:3]
            found.append((reason, file, row))
    return found


class TheLiveRowsHoldTheirForm(unittest.TestCase):
    def test_every_live_swift_row_holds_its_form(self):
        """SPEC-397 A1 (R2): the census over the live tree reads no problem, and examines exactly
        the rows this test counts in every package's row file itself."""
        files = examined("Swift row files", sorted(REPO.glob("ios/*/swift-mutants.json")))
        rows = sum(len(json.loads(p.read_text(encoding="utf-8"))["mutants"]) for p in files)
        done = module(REPO, "census")
        self.assertEqual(
            done.stdout, f"examined {rows} row(s) in {len(files)} file(s)\n", done.stderr
        )
        self.assertEqual(done.returncode, 0, done.stdout)


#: Each row-form defect A2 plants: (label, the edit of the clean tree, the reason the census must
#: name, the row file and the row it must name).
ROW_DEFECTS = (
    ("a key missing", lambda t: alpha(t).pop("why"), "keys", ALPHA, "SW00001"),
    ("a key extra", lambda t: alpha(t).update(note="x"), "keys", ALPHA, "SW00001"),
    ("an id of four digits", lambda t: alpha(t).update(id="SW0001"), "id-form", ALPHA, "SW0001"),
    (
        "an id of six digits",
        lambda t: alpha(t).update(id="SW000001"),
        "id-form",
        ALPHA,
        "SW000001",
    ),
    ("an id without SW", lambda t: alpha(t).update(id="SX00001"), "id-form", ALPHA, "SX00001"),
    (
        "an id repeated in another package's file",
        lambda t: t[BETA]["mutants"][0].update(id="SW00001"),
        "id-repeated",
        BETA,
        "SW00001",
    ),
    (
        "a file outside Sources",
        lambda t: alpha(t).update(file="Tests/AlphaTests/CodeTests.swift"),
        "file-outside-sources",
        ALPHA,
        "SW00001",
    ),
    (
        "a file that climbs out of Sources",
        lambda t: alpha(t).update(file="Sources/../../Beta/Sources/Beta/Code.swift"),
        "file-outside-sources",
        ALPHA,
        "SW00001",
    ),
    (
        "a file absent",
        lambda t: alpha(t).update(file="Sources/Alpha/Gone.swift"),
        "file-absent",
        ALPHA,
        "SW00001",
    ),
    (
        "a find absent",
        lambda t: alpha(t).update(find="let gone = 1"),
        "find-not-once",
        ALPHA,
        "SW00001",
    ),
    (
        "a find repeated",
        lambda t: t.update({ALPHA_SOURCE: t[ALPHA_SOURCE] + "let kills = 1\n"}),
        "find-not-once",
        ALPHA,
        "SW00001",
    ),
    (
        "a find equal to its replacement",
        lambda t: alpha(t).update(replace="let kills = 1"),
        "find-equals-replace",
        ALPHA,
        "SW00001",
    ),
    ("an empty find", lambda t: alpha(t).update(find=""), "find-empty", ALPHA, "SW00001"),
    ("an empty reason", lambda t: alpha(t).update(why=" \n"), "why-empty", ALPHA, "SW00001"),
    (
        "an empty population",
        lambda t: t[ALPHA].update(population=" "),
        "population",
        ALPHA,
        "the file",
    ),
    (
        "an extra top-level key",
        lambda t: t[ALPHA].update(extra=[]),
        "top-level-keys",
        ALPHA,
        "the file",
    ),
)

#: Each unresolvable killer A3 plants: (label, the edit of the clean tree, the reason).
KILLER_DEFECTS = (
    (
        "a killer out of form",
        lambda t: alpha(t).update(killer="AlphaTests.CodeTests.test_kills"),
        "killer-form",
    ),
    (
        "a target that does not resolve",
        lambda t: alpha(t).update(killer="GammaTests.CodeTests/test_kills"),
        "target-absent",
    ),
    (
        "a class that does not resolve",
        lambda t: alpha(t).update(killer="AlphaTests.OtherTests/test_kills"),
        "class-absent",
    ),
    (
        "a method that does not resolve",
        lambda t: alpha(t).update(killer="AlphaTests.CodeTests/test_gone"),
        "method-absent",
    ),
    (
        "a method that does not start with test",
        lambda t: (
            alpha(t).update(killer="AlphaTests.CodeTests/kills"),
            t.update({ALPHA_TESTS: swift_test_file("CodeTests", "test_kills", "kills")}),
        ),
        "killer-not-a-test",
    ),
    (
        "a class declared twice",
        lambda t: t.update(
            {
                "ios/Alpha/Tests/AlphaTests/MoreTests.swift": swift_test_file(
                    "CodeTests", "test_more"
                )
            }
        ),
        "class-repeated",
    ),
    (
        "a method declared twice",
        lambda t: t.update({ALPHA_TESTS: swift_test_file("CodeTests", "test_kills", "test_kills")}),
        "method-repeated",
    ),
)


class EachRowDefectIsRefusedByName(unittest.TestCase):
    def census_of(self, edit):
        """The census over the clean planted tree with `edit` applied."""
        with tempfile.TemporaryDirectory() as scratch:
            tree = alpha_beta()
            edit(tree)
            write_tree(scratch, tree)
            return module(scratch, "census")

    def test_each_planted_row_defect_is_refused_by_name(self):
        """SPEC-397 A2 (R2): each row-form defect is refused by its reason, its file and its row,
        with exit 2, and the clean planted tree reads its examined count with exit 0."""
        clean = self.census_of(lambda tree: None)
        self.assertEqual(
            (clean.returncode, clean.stdout), (0, "examined 2 row(s) in 2 file(s)\n"), clean.stderr
        )
        for label, edit, reason, file, row in examined("planted row defects", ROW_DEFECTS):
            with self.subTest(plant=label):
                done = self.census_of(edit)
                self.assertEqual(refusals(done), [(reason, file, row)], done.stdout)
                self.assertEqual(done.returncode, 2, done.stdout)
                self.assertEqual(
                    done.stdout.splitlines()[-1], "examined 2 row(s) in 2 file(s)", done.stdout
                )

    def test_each_unresolvable_killer_is_refused_by_name(self):
        """SPEC-397 A3 (R2): a killer whose target, class or method does not resolve, whose method
        is no test, or whose class or method is declared twice is refused by name, with exit 2."""
        for label, edit, reason in examined("planted killer defects", KILLER_DEFECTS):
            with self.subTest(plant=label):
                done = self.census_of(edit)
                self.assertEqual(refusals(done), [(reason, ALPHA, "SW00001")], done.stdout)
                self.assertEqual(done.returncode, 2, done.stdout)


def without(tree, row_id):
    """`tree` with the row `row_id` removed from the Alpha package's row file."""
    tree[ALPHA]["mutants"] = [row for row in tree[ALPHA]["mutants"] if row["id"] != row_id]
    return tree


def approval(reason, approved):
    return {
        "_": ["A planted approvals record."],
        "retired": [{"id": "SW00001", "reason": reason, "approval": approved}],
    }


REFUSED_SW00001 = (
    "retired: SW00001: REFUSED: it left while its file ios/Alpha/Sources/Alpha/Code.swift stays, "
    "and scripts/mutation-rows.retired.json records no reason and approval for it"
)
APPROVALS = "scripts/mutation-rows.retired.json"


def departure_base():
    """The base revision's tree: the clean tree, with a second Alpha row over a second file."""
    tree = alpha_beta()
    tree["ios/Alpha/Sources/Alpha/Other.swift"] = "let other = 1\n"
    tree[ALPHA]["mutants"].append(
        swift_row(
            "SW00003",
            "Sources/Alpha/Other.swift",
            "let other = 1",
            "let other = 0",
            "AlphaTests.CodeTests/test_kills",
            "the other marker",
        )
    )
    return tree


#: Each departure A4 plants: (label, the work tree, its exit, its `retired: ` lines).
DEPARTURES = (
    ("no row left", lambda t: t, 0, []),
    ("a row that left while its file stays", lambda t: without(t, "SW00001"), 1, [REFUSED_SW00001]),
    (
        "a row that left with its file",
        lambda t: {k: v for k, v in without(t, "SW00003").items() if not k.endswith("Other.swift")},
        0,
        ["retired: SW00003: its file ios/Alpha/Sources/Alpha/Other.swift left with it"],
    ),
    (
        "a departure the approvals record holds",
        lambda t: {**without(t, "SW00001"), APPROVALS: approval("moved", "the maintainer")},
        0,
        ["retired: SW00001: its file stays; retired with approval: the maintainer"],
    ),
    (
        "an approval with an empty reason",
        lambda t: {**without(t, "SW00001"), APPROVALS: approval(" ", "the maintainer")},
        1,
        [REFUSED_SW00001],
    ),
    (
        "an approval with an empty approval",
        lambda t: {**without(t, "SW00001"), APPROVALS: approval("moved", "")},
        1,
        [REFUSED_SW00001],
    ),
)


class ARowLeavesOnlyWithItsFileOrAnApproval(unittest.TestCase):
    def test_a_row_leaves_only_with_its_file_or_an_approval(self):
        """SPEC-397 A4 (R4): an id that left while its file stays is refused by id with exit 1; it
        passes when its file left with it, or when the approvals record holds a reason and an
        approval for it, and not when either is empty."""
        with tempfile.TemporaryDirectory() as scratch:
            base = Path(scratch) / "base"
            write_tree(base, departure_base())
            git(base, "init", "-q")
            git(base, "add", "-A")
            git(base, "commit", "-q", "-m", "the base")
            for at, (label, edit, code, lines) in enumerate(examined("departures", DEPARTURES)):
                with self.subTest(case=label):
                    work = Path(scratch) / f"work-{at}"
                    write_tree(work, edit(departure_base()))
                    (work / ".git").write_text(f"gitdir: {base / '.git'}\n", encoding="utf-8")
                    done = module(work, "retired", "--base", "HEAD")
                    said = done.stdout.splitlines()
                    self.assertEqual([s for s in said if s.startswith("retired: ")], lines, said)
                    self.assertEqual(said[-1:], ["examined 3"], done.stdout + done.stderr)
                    self.assertEqual(done.returncode, code, done.stdout)


#: A planted `swift test --package-path P --filter S`: it logs the selector, then prints one test
#: case's lines as the markers in the planted source decide. `test_doubles` repeats a find in
#: `Twice.swift` on its unmutated run; `test_hangs`, mutated, starts a grandchild and sleeps.
FAKE_SWIFT = """#!/bin/sh
printf '%s\\n' "$5" >> "$FAKE_LOG"
sources="$3/Sources/Gamma"
method="${5##*/}"
method="${method%?}"
case "$5" in
  *test_kills*)
    if grep -q 'let kills = 1' "$sources/Code.swift"; then verdict=passed; else verdict=failed; fi ;;
  *test_survives*) verdict=passed ;;
  *test_red*) verdict=failed ;;
  *test_doubles*) printf 'let twice = 1\\n' >> "$sources/Twice.swift"; verdict=passed ;;
  *test_hangs*)
    if grep -q 'let hangs = 1' "$sources/Code.swift"; then verdict=passed; else
      (sleep HANG > "$FAKE_DIR/grandchild.out" 2>&1 & echo $! > "$FAKE_DIR/grandchild.pid")
      exec sleep HANG
    fi ;;
  *test_rewrites*)
    if grep -q 'let rewrites = 1' "$sources/Code.swift"; then verdict=passed; else
      printf 'rewritten\\n' > "$sources/Code.swift"; verdict=failed
    fi ;;
  *) verdict=failed ;;
esac
echo "Test Suite 'Selected tests' started."
echo "Test Case '-[GammaTests.CodeTests $method]' started."
echo "Test Case '-[GammaTests.CodeTests $method]' $verdict (0.001 seconds)."
if [ "$verdict" = passed ]; then
  echo "Executed 1 test, with 0 failures (0 unexpected) in 0.001 (0.001) seconds"
  exit 0
fi
echo "Executed 1 test, with 1 failure (0 unexpected) in 0.001 (0.001) seconds"
exit 1
""".replace("HANG", str(HANG_SECONDS))

GAMMA = "ios/Gamma"
GAMMA_SOURCE = "ios/Gamma/Sources/Gamma/Code.swift"
GAMMA_TEXT = "let kills = 1\nlet survives = 1\nlet red = 1\nlet hangs = 1\nlet rewrites = 1\n"
#: Each row of the planted Gamma package: (id, its find's marker, its killer, its file, the verdict
#: R6 gives it, whether its mutant runs).
GAMMA_ROWS = (
    ("SW00010", "kills", "test_kills", "Code.swift", "KILLED", True),
    ("SW00011", "survives", "test_survives", "Code.swift", "SURVIVED", True),
    (
        "SW00012",
        "red",
        "test_red",
        "Code.swift",
        "VOID: its killer is not green on one test unmutated",
        False,
    ),
    ("SW00013", "twice", "test_doubles", "Twice.swift", "VOID: its find occurs 2 times", False),
    (
        "SW00014",
        "hangs",
        "test_hangs",
        "Code.swift",
        "VOID: the killer ran past its bound, and its process group was ended",
        True,
    ),
    ("SW00015", "rewrites", "test_rewrites", "Code.swift", "KILLED", True),
)
METHODS = sorted({row[2] for row in GAMMA_ROWS})


def gamma(rows):
    """A planted Gamma package holding `rows` of GAMMA_ROWS."""
    return {
        GAMMA_SOURCE: GAMMA_TEXT,
        "ios/Gamma/Sources/Gamma/Twice.swift": "let twice = 1\n",
        "ios/Gamma/Tests/GammaTests/CodeTests.swift": swift_test_file("CodeTests", *METHODS),
        f"{GAMMA}/swift-mutants.json": {
            "population": "the planted Gamma package's sources",
            "mutants": [
                swift_row(
                    row_id,
                    f"Sources/Gamma/{file}",
                    f"let {marker} = 1",
                    f"let {marker} = 0",
                    f"GammaTests.CodeTests/{method}",
                    f"the {marker} marker",
                )
                for row_id, marker, method, file, _verdict, _runs in rows
            ],
        },
    }


def selector(method):
    return "^" + re.escape(f"GammaTests.CodeTests/{method}") + "$"


def alive(pid):
    """Whether `pid` names a process that has not ended: its `/proc` entry is there, and its state
    is neither zombie nor dead."""
    with contextlib.suppress(FileNotFoundError, ProcessLookupError):
        stat = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8")
        return stat.rsplit(")", 1)[1].split()[0] not in ("Z", "X")
    return False


class AFakeKillerReadsEachVerdict(unittest.TestCase):
    def sweep(self, scratch, rows):
        """The sweep of a planted Gamma package holding `rows`, through the planted `swift`, with a
        bound of one second on each killer run; (the run, the selectors the fake logged)."""
        root = Path(scratch) / "tree"
        write_tree(root, gamma(rows))
        fake = Path(scratch) / "bin" / "swift"
        fake.parent.mkdir()
        fake.write_text(FAKE_SWIFT, encoding="utf-8")
        fake.chmod(0o755)
        log = Path(scratch) / "fake.log"
        log.write_text("", encoding="utf-8")
        env = {
            "PATH": f"{fake.parent}{os.pathsep}{os.environ['PATH']}",
            "FAKE_LOG": str(log),
            "FAKE_DIR": str(scratch),
            "GITHUB_STEP_SUMMARY": str(Path(scratch) / "summary.md"),
        }
        args = ("--package", GAMMA, "--report", str(Path(scratch) / "report"), "--run-seconds", "1")
        done = module(root, "sweep", *args, env=env)
        return done, log.read_text(encoding="utf-8").splitlines()

    def test_a_fake_killer_reads_each_verdict_and_the_file_is_restored(self):
        """SPEC-397 A5 (R5, R6): through a fake killer, the sweep reads KILLED, SURVIVED, VOID for
        a killer red unmutated, VOID for a find that is not once, and VOID for a run past its bound
        with its process group ended; a killer that rewrites its file leaves it restored byte for
        byte; sweep.md and the last line name every row; the exits are 0, 1 and, for an empty row
        file, 3."""
        rows = examined("planted Gamma rows", GAMMA_ROWS)
        with tempfile.TemporaryDirectory() as scratch:
            done, logged = self.sweep(scratch, rows)
            said = done.stdout.splitlines()
            for row_id, _marker, _method, _file, verdict, runs in rows:
                (line,) = [s for s in said if s.startswith(f"{row_id}: ")] or [f"{row_id}: none"]
                seconds = r", \d+\.\d s" if runs else ""
                self.assertRegex(line, f"^{re.escape(f'{row_id}: {verdict}')}{seconds}$", said)
            last = "swift-mutants ios/Gamma: examined 6 row(s): 2 killed, 1 survived, 3 void"
            self.assertEqual(said[-1:], [last], done.stdout + done.stderr)
            self.assertEqual(done.returncode, 1, done.stdout)
            mutated = [selector(method) for _i, _m, method, _f, _v, runs in rows if runs]
            self.assertEqual(logged, [selector(method) for method in METHODS] + mutated)
            restored = hashlib.sha256((Path(scratch) / "tree" / GAMMA_SOURCE).read_bytes())
            self.assertEqual(
                restored.hexdigest(), hashlib.sha256(GAMMA_TEXT.encode("utf-8")).hexdigest()
            )
            table = ["| id | verdict | why |", "|---|---|---|"] + [
                f"| {row_id} | {verdict} | the {marker} marker |"
                for row_id, marker, _method, _file, verdict, _runs in rows
            ]
            report = (Path(scratch) / "report" / "sweep.md").read_text(encoding="utf-8")
            self.assertEqual(report, "\n".join([*table, "", last]) + "\n")
            summary = (Path(scratch) / "summary.md").read_text(encoding="utf-8")
            self.assertEqual(summary, report)
            pid = int((Path(scratch) / "grandchild.pid").read_text(encoding="utf-8"))
            deadline = time.monotonic() + GRANDCHILD_SECONDS
            while alive(pid) and time.monotonic() < deadline:
                time.sleep(0.1)
            self.assertTrue(Path("/proc/self/stat").is_file(), "process states are read in /proc")
            self.assertFalse(alive(pid), f"the hanging killer's grandchild {pid} outlived its run")
        with tempfile.TemporaryDirectory() as scratch:
            done, _logged = self.sweep(scratch, rows[:1])
            last = "swift-mutants ios/Gamma: examined 1 row(s): 1 killed, 0 survived, 0 void"
            self.assertEqual((done.returncode, done.stdout.splitlines()[-1:]), (0, [last]))
        with tempfile.TemporaryDirectory() as scratch:
            done, logged = self.sweep(scratch, ())
            last = "swift-mutants ios/Gamma: examined 0 row(s): 0 killed, 0 survived, 0 void"
            self.assertEqual((done.returncode, done.stdout.splitlines()[-1:]), (3, [last]))
            self.assertEqual(logged, [])


def oracle(baseline, occurs, restored, mutated):
    """R6's first-match table, written from the SPEC and never from the module."""
    green = (
        baseline["started"] == 1
        and baseline["failed"] == 0
        and baseline["executed"] == "1"
        and baseline["code"] == 0
        and not baseline["timed_out"]
    )
    if not green:
        return "VOID: its killer is not green on one test unmutated"
    if occurs != 1:
        return f"VOID: its find occurs {occurs} times"
    if not restored:
        return "VOID: the file was not restored byte for byte"
    if mutated["timed_out"]:
        return "VOID: the killer ran past its bound, and its process group was ended"
    started, failed, code = mutated["started"], mutated["failed"], mutated["code"]
    if started == 1 and failed == 1 and code != 0:
        return "KILLED"
    if started == 1 and failed == 0 and code == 0:
        return "SURVIVED"
    return f"VOID: {started} started, {failed} failed, exit {code}, so the killer did not judge it"


class TheVerdictOverEveryClass(unittest.TestCase):
    def test_the_verdict_over_every_observed_class(self):
        """SPEC-397 A6 (R6): the verdict function, over every class of its inputs, equals the
        first-match table this test writes from R6."""
        swift_mutants = runner_module()
        green = {"started": 1, "failed": 0, "executed": "1", "code": 0, "timed_out": False}
        broken = (
            ("started", 0),
            ("started", 2),
            ("failed", 1),
            ("executed", "2"),
            ("executed", "none"),
            ("code", 1),
            ("timed_out", True),
        )
        baselines = [green, *({**green, key: value} for key, value in broken)]
        mutated = [
            {"started": s, "failed": f, "executed": e, "code": c, "timed_out": t}
            for s in (0, 1, 2)
            for f in (0, 1, 2)
            for e in ("1", "none")
            for c in (0, 1, 101)
            for t in (False, True)
        ]
        cases = [
            (b, occurs, restored, m)
            for b in baselines
            for occurs in (0, 1, 2)
            for restored in (True, False)
            for m in mutated
        ]
        for b, occurs, restored, m in examined("verdict inputs", cases):
            run, after = swift_mutants.Run(**b), swift_mutants.Run(**m)
            self.assertEqual(
                swift_mutants.verdict(run, occurs, restored, after),
                oracle(b, occurs, restored, m),
                (b, occurs, restored, m),
            )


def foreign_imports(source):
    """(line, name) of each import in `source`, anywhere in it, that names a module outside the
    standard library; a relative import names none of the standard library's."""
    found = []
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.Import):
            found += [
                (node.lineno, alias.name)
                for alias in node.names
                if alias.name.split(".")[0] not in sys.stdlib_module_names
            ]
        elif isinstance(node, ast.ImportFrom):
            name = "." * node.level + (node.module or "")
            if node.level or name.split(".")[0] not in sys.stdlib_module_names:
                found.append((node.lineno, name))
    return sorted(found)


#: Planted modules and the foreign imports A11 must name in each.
PLANTED_IMPORTS = (
    ("import json\n\n\ndef read():\n    import yaml\n\n    return yaml\n", [(5, "yaml")]),
    ("from requests import get\nimport os.path\n", [(1, "requests")]),
    ("from . import sibling\nimport re\n", [(1, ".")]),
)


class TheModuleImportsOnlyTheStandardLibrary(unittest.TestCase):
    def test_the_module_imports_only_the_standard_library(self):
        """SPEC-397 A11 (R12): a planted module importing anything else is refused by name, and the
        live module is read and passes."""
        for source, names in examined("planted modules", PLANTED_IMPORTS):
            self.assertEqual(foreign_imports(source), names, source)
        source = MODULE.read_text(encoding="utf-8")
        examined(
            "imports of the module",
            [n for n in ast.walk(ast.parse(source)) if isinstance(n, (ast.Import, ast.ImportFrom))],
        )
        self.assertEqual(
            foreign_imports(source), [], "scripts/swift_mutants.py imports outside the stdlib"
        )


if __name__ == "__main__":
    unittest.main()
