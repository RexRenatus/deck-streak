"""SPEC-423: the release pull request's Rust legs may pass the pull request ceiling up to one
signed ceiling, in one matrix of at most 256 (ADR-437, #822).

Every listing, price and expected line is a literal or is built here, never by the code under
test. A `deck-streak-ingest` mutant costs 126 s and a `deck-streak-daemon` mutant 80 s, and a leg
starts with a 2141 s baseline, so a leg of 10800 s holds 68 ingest mutants, the slowest at
10709 s, and 113 daemon mutants deal two legs, the slowest at 6701 s. `a.py` is priced 600 s, so
each of its mutants deals one Python leg of 1800 s. Every run patches both release ceilings,
so no test here but the two that read the tree's own source depends on the figure the tree sets.
"""

import ast
import contextlib
import io
import json
import os
import re
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import test_release_python_ceiling
from _support import REPO, examined
from test_mutation_verdict import (
    LIB,
    LIB_TEXT,
    Fixture,
    listing,
    shard_outcomes,
    shard_reports,
    verdict_module,
)
from test_release_python_ceiling import (
    GOLDEN_LINES,
    GOLDEN_OUTPUTS,
    RULING,
    SCHEMA,
    deal,
    mutants,
    release_line,
    scratch,
)

VERDICT = REPO / "scripts" / "mutation-verdict.py"
RELEASING = REPO / "RELEASING.md"
MAP = REPO / "scripts" / "mutation-python.json"
#: The price table every run here reads: each `a.py` mutant deals one Python leg of 1800 s.
PRICES = {"a.py": 600}
#: A plan with no `release` key at all.
ABSENT = object()
DAEMON = "deck-streak-daemon"
INGEST = "deck-streak-ingest"
#: A pull request plan's run over 113 daemon mutants and three `a.py` mutants, the Python ceiling
#: at its cap and the Rust ceiling at the pull request ceiling, as the plan's job prints it.
RUST_GOLDEN_LINES = (
    "mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\n"
    "mutation: shards: python legs dealt by cost: 3 priced leg(s), the slowest at 1800 s of its "
    "1800 s bound; 0 unpriced leg(s) of at most 20\n"
    "mutation: shards: 2 shard(s) for 113 listed mutant(s), projected at 9040 s serially; the "
    "slowest at 6701 s of its 10800 s bound; legs 2 of ceiling 189\n"
    "mutation: shards: run legs 5 of 221: python 3 of 32, rust 2 of 189; projected 313 "
    "runner-minute(s) of 34980\n"
)
RUST_GOLDEN_OUTPUTS = {
    "shards": "2",
    "matrix": "[0, 1]",
    "listed": "113",
    "python_shards": "3",
    "python_matrix": "[0, 1, 2]",
    "python_release_matrix": "[]",
    "python_release_legs": "0",
}
#: The same listing on the release plan under a Rust ceiling of 200.
RUST_RELEASE_LINES = (
    "mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\n"
    "mutation: shards: python legs dealt by cost: 3 priced leg(s), the slowest at 1800 s of its "
    "1800 s bound; 0 unpriced leg(s) of at most 20\n"
    "mutation: shards: release run: 2 rust leg(s) of ceiling 200 (RUST_RELEASE_CEILING): 2 in "
    "mutation-rust, at most 256 a matrix\n"
    "mutation: shards: 2 shard(s) for 113 listed mutant(s), projected at 9040 s serially; the "
    "slowest at 6701 s of its 10800 s bound; legs 2 of ceiling 200\n"
    "mutation: shards: run legs 5 of 232: python 3 of 32, rust 2 of 200; projected 313 "
    "runner-minute(s) of 36960\n"
)
#: The Rust shards line of the 113 daemon mutants under a Rust ceiling of 200.
DAEMON_LINE_200 = (
    "mutation: shards: 2 shard(s) for 113 listed mutant(s), projected at 9040 s serially; the "
    "slowest at 6701 s of its 10800 s bound; legs 2 of ceiling 200\n"
)


def rust_listing(package, count):
    """`count` Rust mutants of `package`, as cargo-mutants lists them."""
    return [{"name": f"{package}: mutant {index}", "package": package} for index in range(count)]


def rust_release_line(legs, ceiling):
    """R6 (a)'s line, spelled here."""
    return (
        f"mutation: shards: release run: {legs} rust leg(s) of ceiling {ceiling} "
        f"(RUST_RELEASE_CEILING): {legs} in mutation-rust, at most 256 a matrix\n"
    )


def release_refusal(listed, serial, ceiling):
    """R4's refusal of a release listing past its Rust ceiling, spelled here."""
    return (
        f"mutation: shards: REFUSED: {listed} mutant(s), projected at {serial} s serially, need "
        f"more than {ceiling} legs within 10800 s each, the release run's ceiling "
        "(RUST_RELEASE_CEILING): a larger figure needs a new measurement and a new owner-signed "
        "ruling, since a run is never capped\n"
    )


def pull_request_refusal(listed, serial):
    """The pull request run's refusal past its ceiling, byte for byte as before (SPEC-362 R3)."""
    return (
        f"mutation: shards: REFUSED: {listed} mutant(s), projected at {serial} s serially, need "
        "more than 189 legs within 10800 s each, the pull request run's ceiling: split the "
        "change, since a run is never capped\n"
    )


def range_refusal(shown):
    """R5's refusal of a Rust ceiling outside its range, spelled here."""
    return (
        f"mutation: shards: REFUSED: the release ceiling RUST_RELEASE_CEILING = {shown} is "
        "outside 189..256; a ceiling outside it is never dealt\n"
    )


def run_main(module, argv, rust, python, prices=None):
    """One verb of `module`'s own `main`, in process, as a job runs it under GitHub Actions: the
    price table patched to `prices` (PRICES when None), the scoped table empty, and both release
    ceilings patched, `RUST_RELEASE_CEILING` to `rust` and `PYTHON_RELEASE_CEILING` to `python`:
    (the exit, the printed text, the step outputs)."""
    with tempfile.TemporaryDirectory() as directory:
        sink = Path(directory) / "github-output"
        sink.write_text("", encoding="utf-8")
        out = io.StringIO()
        table = PRICES if prices is None else prices
        with contextlib.ExitStack() as stack:
            stack.enter_context(mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(sink)}))
            stack.enter_context(mock.patch.dict(module.PYTHON_CONTROL_SECONDS, table, clear=True))
            stack.enter_context(mock.patch.dict(module.PYTHON_SCOPED_SECONDS, {}, clear=True))
            stack.enter_context(mock.patch.object(module, "PYTHON_RELEASE_CEILING", python))
            stack.enter_context(
                mock.patch.object(module, "RUST_RELEASE_CEILING", rust, create=True)
            )
            stack.enter_context(contextlib.redirect_stdout(out))
            code = module.main([str(each) for each in argv])
        written = sink.read_text(encoding="utf-8").splitlines()
    return code, out.getvalue(), dict(line.split("=", 1) for line in written)


def rust_verb(argv, rust, python, prices=None):
    """One verb of a fresh `mutation-verdict.py`, run by `run_main` with both ceilings patched, so
    no patch of one test's module reaches another's."""
    return run_main(verdict_module(), argv, rust, python, prices)


def plan_files(test, listed, python_listed, release=ABSENT):
    """A plan whose Rust class applies, marked `release` unless ABSENT, with the Rust listing
    `listed` and, unless None, the Python listing `python_listed`, written to a scratch directory:
    (the `shards` arguments, the plan's path)."""
    root = scratch(test)
    plan = {"classes": {"rust": {"applies": True}}}
    if release is not ABSENT:
        plan["release"] = release
    (root / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    (root / "listed.json").write_text(json.dumps(listed), encoding="utf-8")
    argv = ["shards", "--plan", root / "plan.json", "--listed", root / "listed.json"]
    if python_listed is not None:
        document = {"schema": SCHEMA, "listed": len(python_listed), "mutants": python_listed}
        (root / "python-listed.json").write_text(json.dumps(document), encoding="utf-8")
        argv += ["--python-listed", root / "python-listed.json"]
    return argv, root / "plan.json"


def rust_deal(test, listed, python_listed, release, rust, python):
    """`shards` through `rust_verb` over `plan_files`' plan, under the Rust ceiling `rust` and the
    Python ceiling `python`: (the exit, the printed text, the step outputs, the plan as left)."""
    argv, path = plan_files(test, listed, python_listed, release)
    code, out, outputs = rust_verb(argv, rust, python)
    return code, out, outputs, json.loads(path.read_text(encoding="utf-8"))


def rust_ceiling_problems(source, root):
    """What R9 refuses in a source of `mutation-verdict.py`, judged against the tree at `root`: one
    `RUST_RELEASE_CEILING` int with a `#:` line directly above it; at `LEG_CEILING["ci"]` the line
    cites no ruling path; above it the line cites exactly one `docs/rulings/<file>.md`, the tree
    holds it, and its first line holds the figure as a whole word and the words `release` and
    `Rust`."""
    assigned = {}
    for node in ast.parse(source).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1:
            target = node.targets[0]
            if isinstance(target, ast.Name):
                assigned.setdefault(target.id, []).append(node)
    ceilings = assigned.get("RUST_RELEASE_CEILING", [])
    tables = assigned.get("LEG_CEILING", [])
    if len(ceilings) != 1 or len(tables) != 1:
        return [f"{len(ceilings)} ceiling and {len(tables)} LEG_CEILING assignment(s), not one"]
    node, floor = ceilings[0], ast.literal_eval(tables[0].value).get("ci")
    if not (isinstance(node.value, ast.Constant) and type(node.value.value) is int):
        return ["the ceiling is not an int literal"]
    above = source.splitlines()[node.lineno - 2] if node.lineno > 1 else ""
    if not above.startswith("#: "):
        return ["the ceiling has no #: line directly above it"]
    value, cited = node.value.value, RULING.findall(above)
    if value < floor:
        return [f"the ceiling {value} is below the pull request ceiling {floor}"]
    if value == floor:
        return [f"at the pull request ceiling the #: line cites {cited}"] if cited else []
    if len(cited) != 1:
        return [f"the ceiling {value} is above {floor} and cites {len(cited)} ruling path(s)"]
    path = root / cited[0]
    if not path.is_file():
        return [f"the ceiling cites {cited[0]}, which the tree does not hold"]
    first = (path.read_text(encoding="utf-8").splitlines() or [""])[0]
    words = (rf"(?<![\w.]){value}(?![\w.])", r"\brelease\b", r"\bRust\b")
    if not all(re.search(word, first) for word in words):
        return [f"{cited[0]}'s first line does not hold {value}, release and Rust as words"]
    return []


class TheReleaseRunSizesItsRustLegsToItsCeiling(unittest.TestCase):
    """R1 to R6 (A1 to A7)."""

    def test_only_a_true_release_mark_holds_the_rust_ceiling(self):
        module = verdict_module()
        cases = [
            ("a true mark under a set ceiling", {"release": True}, 200, 200),
            ("a true mark one past the placeholder", {"release": True}, 190, 190),
            ("no mark", {}, 200, None),
            ("a false mark", {"release": False}, 200, None),
            ("a string mark", {"release": "true"}, 200, None),
            ("a mark of one", {"release": 1}, 200, None),
            ("a null mark", {"release": None}, 200, None),
            ("a true mark at the placeholder", {"release": True}, 189, None),
        ]
        for where, plan, ceiling, expected in examined("release marks and ceilings", cases):
            with mock.patch.object(module, "RUST_RELEASE_CEILING", ceiling, create=True):
                self.assertEqual(module.rust_release_ceiling(plan), expected, where)

    def test_the_placeholder_ceiling_sizes_the_release_run_as_a_pull_request_run(self):
        listed, scripts = rust_listing(DAEMON, 113), mutants(3)
        code, out, outputs, _plan = rust_deal(self, listed, scripts, True, 200, 128)
        self.assertEqual(code, 0, out)
        self.assertIn(rust_release_line(2, 200), out)
        self.assertNotEqual(out, RUST_GOLDEN_LINES)
        cases = [("the pull request plan", ABSENT), ("the release plan", True)]
        for where, release in examined("plans at the placeholder ceiling", cases):
            code, out, outputs, _plan = rust_deal(self, listed, scripts, release, 189, 128)
            self.assertEqual((code, out), (0, RUST_GOLDEN_LINES), where)
            self.assertEqual(outputs, RUST_GOLDEN_OUTPUTS, where)

    def test_the_release_run_sizes_up_to_its_ceiling_and_refuses_one_mutant_past_it(self):
        code, out, outputs, plan = rust_deal(
            self, rust_listing(INGEST, 13600), None, True, 200, 128
        )
        self.assertEqual(code, 0, out)
        self.assertEqual(
            out,
            rust_release_line(200, 200)
            + "mutation: shards: 200 shard(s) for 13600 listed mutant(s), projected at 1713600 s "
            "serially; the slowest at 10709 s of its 10800 s bound; legs 200 of ceiling 200\n",
        )
        self.assertEqual((outputs["shards"], outputs["listed"]), ("200", "13600"))
        self.assertEqual(json.loads(outputs["matrix"]), list(range(200)))
        legs = examined("legs of the release plan", plan["shards"]["shards"])
        self.assertEqual(len(legs), 200)
        self.assertEqual({len(leg["mutants"]) for leg in legs}, {68})
        self.assertEqual(max(leg["projected_seconds"] for leg in legs), 10709)
        code, out, outputs, plan = rust_deal(
            self, rust_listing(INGEST, 13601), None, True, 200, 128
        )
        self.assertEqual((code, out), (1, release_refusal(13601, 1713726, 200)))
        self.assertEqual(outputs, {})
        self.assertNotIn("shards", plan)

    def test_a_run_that_is_not_the_release_keeps_the_pull_request_ceiling_at_any_rust_ceiling(self):
        crowded = rust_listing(INGEST, 12853)
        code, out, outputs, _plan = rust_deal(self, crowded, None, True, 200, 128)
        self.assertEqual((code, outputs.get("shards")), (0, "190"), out)
        listed, scripts = rust_listing(DAEMON, 113), mutants(3)
        for ceiling in examined("rust ceilings on a plan that is not the release", [190, 200, 256]):
            code, out, outputs, plan = rust_deal(self, crowded, None, ABSENT, ceiling, 128)
            refused = (1, pull_request_refusal(12853, 1619478), {})
            self.assertEqual((code, out, outputs), refused, ceiling)
            self.assertNotIn("shards", plan, ceiling)
            code, out, outputs, _plan = rust_deal(self, listed, scripts, ABSENT, ceiling, 128)
            self.assertEqual((code, out), (0, RUST_GOLDEN_LINES), ceiling)
            self.assertEqual(outputs, RUST_GOLDEN_OUTPUTS, ceiling)

    def test_a_rust_ceiling_outside_its_range_is_refused_by_name(self):
        listed, scripts = rust_listing(DAEMON, 113), mutants(3)
        cases = [
            ("a string", "200", "200"),
            ("a float", 200.0, "200.0"),
            ("a bool", True, "True"),
            ("one below the pull request ceiling", 188, "188"),
            ("one past the matrix", 257, "257"),
        ]
        for where, ceiling, shown in examined("rust ceilings outside the range", cases):
            code, out, outputs, plan = rust_deal(self, listed, scripts, True, ceiling, 128)
            self.assertEqual((code, out), (1, range_refusal(shown)), where)
            self.assertEqual(outputs, {}, where)
            self.assertEqual(plan, {"classes": {"rust": {"applies": True}}, "release": True}, where)
        for ceiling in examined("rust ceilings at the range's ends", [189, 256]):
            code, out, outputs, _plan = rust_deal(self, listed, scripts, True, ceiling, 128)
            self.assertEqual((code, outputs.get("shards")), (0, "2"), out)

    def test_the_rust_lines_read_the_release_ceiling_on_the_release_run_alone(self):
        listed, scripts = rust_listing(DAEMON, 113), mutants(3)
        code, out, outputs, _plan = rust_deal(self, listed, scripts, True, 200, 128)
        self.assertEqual(code, 0, out)
        self.assertIn(DAEMON_LINE_200, out)
        self.assertIn(
            "mutation: shards: run legs 5 of 232: python 3 of 32, rust 2 of 200; projected 313 "
            "runner-minute(s) of 36960\n",
            out,
        )
        self.assertEqual(out, RUST_RELEASE_LINES)
        self.assertEqual(outputs, RUST_GOLDEN_OUTPUTS)
        code, out, outputs, _plan = rust_deal(self, listed, scripts, ABSENT, 200, 128)
        self.assertEqual((code, out), (0, RUST_GOLDEN_LINES))
        self.assertEqual(outputs, RUST_GOLDEN_OUTPUTS)

    def test_the_rust_release_line_stands_beside_the_python_release_line(self):
        listed, scripts = rust_listing(DAEMON, 113), mutants(3)
        code, out, _outputs, _plan = rust_deal(self, listed, scripts, True, 200, 300)
        self.assertEqual(code, 0, out)
        lines = out.splitlines(keepends=True)
        python_line = release_line(3, 300, 3, 0)
        self.assertIn(python_line, lines)
        self.assertEqual(lines[lines.index(python_line) + 1], rust_release_line(2, 200))
        code, out, _outputs, _plan = rust_deal(self, listed, None, True, 200, 128)
        self.assertEqual((code, out), (0, rust_release_line(2, 200) + DAEMON_LINE_200))
        cases = [
            ("a plan that is not the release", ABSENT, 200),
            ("the release plan at the placeholder", True, 189),
        ]
        for where, release, ceiling in examined("runs that print no rust release line", cases):
            code, out, _outputs, _plan = rust_deal(self, listed, scripts, release, ceiling, 300)
            self.assertEqual(code, 0, out)
            self.assertIn("; legs 2 of ceiling 189\n", out, where)
            self.assertNotIn("rust leg(s) of ceiling", out, where)


class EveryRustLegIsCounted(unittest.TestCase):
    """R8 (A9)."""

    def test_the_verdict_walks_every_leg_past_the_pull_request_ceiling(self):
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        plan = fixture.plan()
        legs = examined("legs past the pull request ceiling", list(range(200)))
        names = [entry["name"] for entry in listing(["deck-streak-fix"] * len(legs))]
        plan["shards"] = {
            "count": len(legs),
            "bound_seconds": 10800,
            "baseline_seconds": 2141,
            "serial_seconds": 126 * len(legs),
            "shards": [
                {"shard": leg, "mutants": [names[leg]], "projected_seconds": 2267} for leg in legs
            ],
        }
        (fixture.out / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
        (fixture.out / "listed.json").write_text(
            json.dumps(listing(["deck-streak-fix"] * len(legs))), encoding="utf-8"
        )
        every = {leg: ("0", shard_outcomes([names[leg]])) for leg in legs}
        whole = shard_reports(fixture.out / "whole", every)
        done = fixture.judge("rust", "--shard-reports", whole)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertRegex(done.stdout, r"(?m)^examined 200$")
        self.assertNotIn("VOID", done.stdout)
        gap = shard_reports(
            fixture.out / "gap", {leg: report for leg, report in every.items() if leg != 137}
        )
        done = fixture.judge("rust", "--shard-reports", gap)
        self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
        self.assertIn("VOID mutation-rust-shard-137: no report", done.stdout)
        self.assertNotIn("verdict: ok", done.stdout)


class TheRustCeilingIsSignedAndTheRunbookSaysTheMethod(unittest.TestCase):
    """R9 and R12 (A10, A12)."""

    def test_the_rust_ceiling_cites_the_signed_ruling_only_above_the_pull_request_ceiling(self):
        self.assertEqual(rust_ceiling_problems(VERDICT.read_text(encoding="utf-8"), REPO), [])
        root = scratch(self)
        (root / "docs" / "rulings").mkdir(parents=True)
        (root / "docs" / "rulings" / "signed.md").write_text(
            "Signed: 200 is the release Rust ceiling\n", encoding="utf-8"
        )
        (root / "docs" / "rulings" / "other.md").write_text(
            "Signed: 300 is the release Rust ceiling\n", encoding="utf-8"
        )

        def source(value, cites):
            return (
                'LEG_CEILING = {"ci": 189, "battery": 234}\n'
                f"#: The release run's Rust ceiling{cites}.\n"
                f"RUST_RELEASE_CEILING = {value}\n"
            )

        self.assertEqual(rust_ceiling_problems(source(200, ", docs/rulings/signed.md"), root), [])
        self.assertEqual(rust_ceiling_problems(source(189, ""), root), [])
        both = ", docs/rulings/signed.md and docs/rulings/other.md"
        planted = [
            ("above it, no ruling cited", source(200, "")),
            ("above it, the ruling absent", source(200, ", docs/rulings/absent.md")),
            ("above it, another figure signed", source(200, ", docs/rulings/other.md")),
            ("above it, two rulings cited", source(200, both)),
        ]
        for where, text in examined("planted sources", planted):
            self.assertEqual(len(rust_ceiling_problems(text, root)), 1, where)

    def test_the_runbook_names_the_rust_release_ceiling_its_matrix_and_its_refusal(self):
        lines = RELEASING.read_text(encoding="utf-8").splitlines()
        headroom = lines.index(
            "plan that needs more than C is refused whole, by name, never capped."
        )
        sentence = lines[headroom + 1]
        self.assertIn("`RUST_RELEASE_CEILING`", sentence)
        self.assertEqual(lines[headroom + 2], "")
        cut = lines.index("cut a release.")
        tag = lines.index("## 3. Tag it on main")
        self.assertEqual((lines[cut + 1], lines[tag - 1]), ("", ""))
        paragraph = lines[cut + 2 : tag - 1]
        self.assertTrue(all(examined("lines of the runbook's paragraph", paragraph)))
        self.assertTrue(
            paragraph[0].startswith(
                "**The release run's Rust legs pass the pull request ceiling only up to one signed"
            )
        )
        said = " ".join(paragraph)
        for phrase in examined(
            "phrases the paragraph says",
            [
                "`RUST_RELEASE_CEILING`",
                "`mutation-rust`",
                "`RUST_MATRIX_LEGS`",
                "refused whole, by name, never capped",
                "a new measurement and a new owner-signed ruling",
            ],
        ):
            self.assertIn(phrase, said)
        for text in (sentence, said):
            self.assertNotRegex(re.sub(r"\b(?:SPEC|ADR)-\d+\b", "", text), r"\d")


class WhatTheReleaseRunKeeps(unittest.TestCase):
    """R10, R11 and R13 (A11, A13, A14)."""

    def test_the_python_ceiling_tests_read_the_rust_ceiling_at_its_placeholder(self):
        planted = verdict_module()
        argv, _path = plan_files(self, rust_listing(DAEMON, 113), mutants(3), True)
        with mock.patch.object(planted, "RUST_RELEASE_CEILING", 200, create=True):
            code, out, _outputs = run_main(planted, argv, 200, 128)
            self.assertEqual(code, 0, out)
            self.assertIn(rust_release_line(2, 200), out)
            with mock.patch.object(test_release_python_ceiling, "verdict_module", lambda: planted):
                code, out, outputs, _plan = deal(self, mutants(3), True, planted.PYTHON_SHARE_CAP)
        self.assertEqual((code, out), (0, GOLDEN_LINES))
        self.assertEqual(outputs, GOLDEN_OUTPUTS)

    def test_the_release_run_restates_no_rust_bound_and_assigns_each_once(self):
        tree = ast.parse(VERDICT.read_text(encoding="utf-8"))
        kept = (
            "LEG_CEILING",
            "SHARD_BOUND_SECONDS",
            "SECONDS_PER_MUTANT",
            "BASELINE_SECONDS",
            "CENSUS_SECONDS",
            "MUTANT_TIMEOUT_SECONDS",
            "TIMEOUT_MARGIN",
        )
        stores = [
            node.id
            for node in ast.walk(tree)
            if isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load)
        ]
        for name in examined("values the release run keeps", kept):
            self.assertEqual(stores.count(name), 1, name)
        reached = ("rust_ceiling_refused", "rust_release_ceiling", "shards", "main")
        functions = [
            node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name in reached
        ]
        self.assertEqual(sorted(node.name for node in functions), sorted(reached))
        restated = {189, 234, 256, 10800}
        for function in examined("functions the release run's Rust legs reach", functions):
            literals = {
                node.value
                for node in ast.walk(function)
                if isinstance(node, ast.Constant) and type(node.value) is int
            }
            self.assertEqual(literals & restated, set(), function.name)

    def test_the_map_judges_the_verdict_scripts_mutants_with_the_rust_ceiling_tests(self):
        modules = json.loads(MAP.read_text(encoding="utf-8"))["scripts/mutation-verdict.py"][
            "modules"
        ]
        self.assertIn("test_release_rust_ceiling", modules)
        named = [
            "test_mutation_verdict_python_kills",
            "test_mutation_python_verdict",
            "test_mutation_python_shard_binding",
            "test_mutation_verdict",
            "test_mutation_equivalent",
            "test_memory_cap_verdict",
            "test_release_python_ceiling",
            "test_release_rust_ceiling",
        ]
        for name in examined("modules the map names for the verdict script", named):
            self.assertEqual(modules.count(name), 1, name)


if __name__ == "__main__":
    unittest.main()
