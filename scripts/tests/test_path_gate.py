"""SPEC-634: the path gate's decisions are its Lean entry's vectors (A1), each of the entry's five
covers is its function's span (A3), and six planted controls show that the vectors catch a path the
gate would miss (A2).

The formal checker writes `formal/vectors/path-gate.jsonl` from the Lean port; this module reads
every vector against `scripts/path_gate.py`. A planted control is the script's own text with one
change, compiled in memory, so the module writes no file and starts no program.
"""

import ast
import hashlib
import json
import re
import types
import unittest

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "path_gate.py"
ENTRY = REPO / "formal" / "lean" / "Formal" / "PathGate.lean"
VECTORS = REPO / "formal" / "vectors" / "path-gate.jsonl"
ANCHORS = ("prefix_of", "reads_entry", "reads", "decide", "admit")
COVER = re.compile(
    r"-- @phx covers scripts/path_gate\.py anchor=(\S+) digest=sha256:([0-9a-f]{64})"
)
ZEROS = "0" * 64

# The planted changes of SPEC-634 section 2a, each the script's text and what replaces it.
ALWAYS = 'for e in m["always"] + jobs[job]["reads"]'
JOB_READS_ALONE = 'for e in jobs[job]["reads"]'
PREFIX = 'return d.endswith("/") and q.startswith(d)'
PREFIX_ANY_END = "return q.startswith(d)"
PREFIX_NO_SLASH = 'return d.endswith("/") and q.startswith(d[:-1])'
NONE_RUNS = 'if changed is None:\n        return "run"'
NONE_EMPTY = "if changed is None:\n        changed = []"
PULL_REQUEST = 'kind != "pull_request"'
PULL_REQUEST_OR_PUSH = 'kind not in ("pull_request", "push")'
INTO_DEV = 'kind != "pull_request" or base_ref != "dev"'
GATE_OK = 'result == "skipped" and gate_ok and gate_out.get(job) == "skip"'
GATE_OUT_ALONE = 'result == "skipped" and gate_out.get(job) == "skip"'
NO_ENTRY_READS = "if job not in jobs:\n        return True"
NO_ENTRY_READS_NOTHING = "if job not in jobs:\n        return False"


def span_digest(path, name):
    """The sha256 of a top-level function's span: every line from its `def` to its last line,
    each followed by a newline, as the formal checker stamps a cover."""
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    for node in ast.parse(text).body:
        if isinstance(node, ast.FunctionDef) and node.name == name:
            span = "".join(line + "\n" for line in lines[node.lineno - 1 : node.end_lineno])
            return hashlib.sha256(span.encode("utf-8")).hexdigest()
    raise AssertionError(f"{path.name} defines no top-level {name}")


def load(name, source):
    """`source`, the script's text or a planted copy of it, compiled as a module named `name`."""
    module = types.ModuleType(name)
    module.__file__ = str(SCRIPT)
    exec(compile(source, str(SCRIPT), "exec"), module.__dict__)
    return module


def vectors():
    """The vectors file's header and its vectors."""
    lines = VECTORS.read_text(encoding="utf-8").splitlines()
    return json.loads(lines[0]), [json.loads(line) for line in lines[1:]]


def pairs(rest):
    """The returned jobs and results as the vectors write them, each pair a list."""
    return [list(pair) for pair in rest]


def answers(module, vector):
    """What `module` answers for one vector, under the keys the vector holds them.

    A decide vector's map is read by the module's reader over a tree that tracks nothing: its
    map text with a key named twice must be refused, and every other map admitted as written. Its
    admission is that of the job skipped while the gate succeeded with its decision.
    """
    if vector["kind"] == "admit":
        needs = [tuple(need) for need in vector["needs"]]
        refused, rest = module.admit(needs, vector["gate_ok"], vector["gate_out"])
        return {"refused": refused, "rest": pairs(rest)}
    text = vector["map_text"]
    if text is None and vector["map"] is not None:
        text = json.dumps(vector["map"])
    m = None if text is None else module.read_map(text, [])[0]
    job, changed = vector["job"], vector["changed"]
    decision = module.decide(tuple(vector["event"]), job, changed, m)
    refused, rest = module.admit([(job, "skipped")], True, {job: decision})
    return {
        "map": m,
        "reads": None if changed is None else [module.reads(m, job, q) for q in changed],
        "decision": decision,
        "admit": {"refused": refused, "rest": pairs(rest)},
    }


def mismatches(module, vector):
    """The keys of one vector whose answer `module` gives otherwise."""
    return sorted(key for key, value in answers(module, vector).items() if vector[key] != value)


class TheLeanVectors(unittest.TestCase):
    """A1 and A3 (R2 to R5, R7 to R9): the Lean entry's vectors and covers are the script's."""

    def test_every_vector_is_the_gates_decision(self):
        self.assertTrue(VECTORS.is_file(), f"{VECTORS.relative_to(REPO)} is absent")
        header, population = vectors()
        self.assertEqual(header["schema"], "phx.formal.vectors.v1")
        self.assertEqual(header["entry"], "PathGate")
        self.assertEqual(header["covers"], "scripts/path_gate.py")
        self.assertEqual(header["anchor"], "prefix_of")
        self.assertNotEqual(
            header["digest"], "sha256:" + ZEROS, "the vectors were written unstamped"
        )
        self.assertEqual(
            header["digest"],
            "sha256:" + span_digest(SCRIPT, "prefix_of"),
            "the entry's stamp is stale",
        )
        self.assertEqual(header["vectors"], len(population))
        cases = [vector["case"] for vector in population]
        self.assertEqual(len(cases), len(set(cases)), "a case is named twice")
        refusals = [v for v in population if v["kind"] == "decide" and v["map_text"] is not None]
        examined("map texts the reader must refuse", refusals)
        path_gate = load("path_gate_vectors", SCRIPT.read_text(encoding="utf-8"))
        for vector in examined("Lean vectors", population):
            with self.subTest(case=vector["case"]):
                got = answers(path_gate, vector)
                self.assertEqual(got, {key: vector[key] for key in got})

    def test_each_cover_is_its_span(self):
        lines = ENTRY.read_text(encoding="utf-8").splitlines()
        found = [COVER.fullmatch(line) for line in lines if line.startswith("-- @phx covers ")]
        self.assertNotIn(None, found, "a covers line is not in the form the checker stamps")
        covers = examined("covers lines", found)
        self.assertGreaterEqual(len(covers), 5, "fewer than the entry's five covers were examined")
        self.assertEqual(tuple(cover.group(1) for cover in covers), ANCHORS)
        for cover in covers:
            anchor, digest = cover.groups()
            with self.subTest(anchor=anchor):
                self.assertNotEqual(digest, ZEROS, f"the {anchor} cover is unstamped")
                self.assertEqual(
                    digest, span_digest(SCRIPT, anchor), f"the {anchor} cover is stale"
                )


class ThePlantedControls(unittest.TestCase):
    """A2 (R9, SPEC-634 section 2a): each planted change is caught by its named vectors, and those
    vectors hold at the script itself."""

    def assert_caught(self, plants, cases):
        source = SCRIPT.read_text(encoding="utf-8")
        planted = source
        for old, new in plants:
            self.assertEqual(planted.count(old), 1, f"the script does not hold {old!r} once")
            planted = planted.replace(old, new)
        script = load("path_gate_script", source)
        plant = load("path_gate_planted", planted)
        named = {vector["case"]: vector for vector in vectors()[1]}
        for case in examined("named vectors", cases):
            with self.subTest(case=case):
                self.assertIn(case, named, f"no vector is named {case}")
                self.assertEqual(mismatches(script, named[case]), [], f"{case} fails the script")
                self.assertNotEqual(mismatches(plant, named[case]), [], f"{case} misses the plant")

    def test_an_always_entry_dropped_from_reads(self):
        self.assert_caught(
            [(ALWAYS, JOB_READS_ALONE)],
            ("rust-always-exact", "rust-always-prefix", "engine-always-exact"),
        )

    def test_the_prefix_test_without_its_endswith(self):
        self.assert_caught(
            [(PREFIX, PREFIX_ANY_END)],
            (
                "release-exact-beside-toolsx",
                "release-exact-beside-subpath",
                "rust-exact-beside-longer-name",
            ),
        )

    def test_the_prefix_test_without_its_slash(self):
        self.assert_caught(
            [(PREFIX, PREFIX_NO_SLASH)],
            (
                "rust-prefix-beside-toolsx",
                "rust-prefix-bare-name",
                "engine-always-prefix-beside-cratesx",
                "engine-always-prefix-bare-name",
            ),
        )

    def test_none_treated_as_an_empty_list(self):
        self.assert_caught(
            [(NONE_RUNS, NONE_EMPTY)],
            (
                "changed-none-rust",
                "changed-none-engine",
                "changed-none-release",
                "changed-none-census",
                "changed-none-web",
            ),
        )

    def test_a_push_treated_as_a_pull_request(self):
        self.assert_caught([(PULL_REQUEST, PULL_REQUEST_OR_PUSH)], ("event-push-into-dev",))

    def test_a_push_treated_as_a_pull_request_with_no_base_test(self):
        self.assert_caught([(INTO_DEV, PULL_REQUEST_OR_PUSH)], ("event-push-without-base",))

    def test_admit_without_its_gate_ok_check(self):
        self.assert_caught(
            [(GATE_OK, GATE_OUT_ALONE)],
            ("admit-gate-failed-skip", "admit-mixed-gate-failed"),
        )

    def test_reads_false_for_a_job_with_no_entry(self):
        self.assert_caught(
            [(NO_ENTRY_READS, NO_ENTRY_READS_NOTHING)],
            ("unknown-job-outside", "unknown-job-boundary", "bare-map-no-entry"),
        )


if __name__ == "__main__":
    unittest.main()
