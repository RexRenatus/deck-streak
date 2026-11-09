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


class TheReaderRefusesEachMapByItsReason(unittest.TestCase):
    """MUTATION COVERAGE: the map reader's refusals, each read as its exact reason with the owner it
    names, and the maps it admits, each read as the document itself. Each test names, by
    `line:col` in `scripts/path_gate.py`, the generated mutants it kills."""

    def test_the_object_hook_raises_its_own_error_on_a_key_held_twice(self):
        """MUTATION COVERAGE 31:26: the hook raises `TwiceError` with its own text on a key held
        twice in one object, and keeps an object whose keys differ."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        with self.assertRaises(gate.TwiceError) as raised:
            gate._once([("schema", "read-map.v1"), ("schema", "read-map.v1")])
        self.assertEqual(str(raised.exception), "a key twice")
        self.assertEqual(gate._once([("always", []), ("jobs", {})]), {"always": [], "jobs": {}})

    def test_a_key_held_twice_is_refused_with_its_reason(self):
        """MUTATION COVERAGE 50:22: a map that holds `always` twice."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        text = '{"schema": "read-map.v1", "always": [], "always": [], "jobs": {}}'
        self.assertEqual(gate.read_map(text, []), (None, "the map holds a key twice in one object"))

    def test_text_that_is_not_json_is_refused_with_its_reason(self):
        """MUTATION COVERAGE 52:16 and 52:22: an open object, an empty text and a key with no
        value."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        for text in examined("texts that are not JSON", ("{", "", '{"schema": }')):
            with self.subTest(text=text):
                self.assertEqual(gate.read_map(text, []), (None, "the map is not JSON"))

    def test_a_document_without_exactly_its_three_keys_is_refused_with_its_reason(self):
        """MUTATION COVERAGE 53:39, 54:16 and 54:22: an object with a fourth key, which the key-set
        test alone refuses, an object with no `jobs`, and a list."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        texts = (
            '{"schema": "read-map.v1", "always": [], "jobs": {}, "never": []}',
            '{"schema": "read-map.v1", "always": []}',
            "[]",
        )
        for text in examined("documents without the three keys", texts):
            with self.subTest(text=text):
                self.assertEqual(
                    gate.read_map(text, []),
                    (None, "the map is not an object of schema, always and jobs"),
                )

    def test_a_schema_other_than_read_map_v1_is_refused_with_its_reason(self):
        """MUTATION COVERAGE 56:16: a map of the schema `read-map.v2`."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        text = '{"schema": "read-map.v2", "always": [], "jobs": {}}'
        self.assertEqual(gate.read_map(text, []), (None, "the map's schema is not read-map.v1"))

    def test_jobs_that_is_not_an_object_is_refused_with_its_reason(self):
        """MUTATION COVERAGE 59:16 and 59:22: `jobs` held as a list of job names."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        text = '{"schema": "read-map.v1", "always": [], "jobs": ["rust"]}'
        self.assertEqual(gate.read_map(text, []), (None, "the map's jobs is not an object"))

    def test_a_job_that_is_not_an_object_of_reads_is_refused_by_its_name(self):
        """MUTATION COVERAGE 62:40 and 63:20: a job object with a second key, which the key-set
        test alone refuses, and a job held as a list."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        cases = (
            (
                '{"schema": "read-map.v1", "always": [], '
                '"jobs": {"rust": {"reads": [], "also": []}}}',
                "the job rust is not an object of reads",
            ),
            (
                '{"schema": "read-map.v1", "always": [], "jobs": {"engine": ["crates/"]}}',
                "the job engine is not an object of reads",
            ),
        )
        for text, reason in examined("jobs that are not an object of reads", cases):
            with self.subTest(reason=reason):
                self.assertEqual(gate.read_map(text, []), (None, reason))

    def test_entries_that_are_not_a_list_of_strings_are_refused_by_their_owner(self):
        """MUTATION COVERAGE 37:36 and 68:20: `always` held as one string, which is no list
        though each of its characters is a string, and a job's reads that hold a number."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        self.assertEqual(gate._strings("crates/"), False)
        self.assertEqual(gate._strings(["crates/", "Cargo.toml"]), True)
        cases = (
            (
                '{"schema": "read-map.v1", "always": "crates/", "jobs": {}}',
                "always is not a list of strings",
            ),
            (
                '{"schema": "read-map.v1", "always": [], '
                '"jobs": {"rust": {"reads": ["crates/", 7]}}}',
                "rust is not a list of strings",
            ),
        )
        for text, reason in examined("owners that are not a list of strings", cases):
            with self.subTest(reason=reason):
                self.assertEqual(gate.read_map(text, []), (None, reason))

    def test_an_empty_entry_is_refused_by_its_owner(self):
        """MUTATION COVERAGE 60:15, 70:25 and 71:24: an empty entry in `always`, named `always`,
        and one in a job's reads, named by the job."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        cases = (
            (
                '{"schema": "read-map.v1", "always": [""], "jobs": {}}',
                "always holds an empty entry",
            ),
            (
                '{"schema": "read-map.v1", "always": ["crates/"], '
                '"jobs": {"rust": {"reads": ["Cargo.toml", ""]}}}',
                "rust holds an empty entry",
            ),
        )
        for text, reason in examined("owners with an empty entry", cases):
            with self.subTest(reason=reason):
                self.assertEqual(gate.read_map(text, []), (None, reason))

    def test_a_directory_named_without_its_trailing_slash_is_refused_by_its_owner(self):
        """MUTATION COVERAGE 65:90, 65:93, 72:22, 72:33, 72:52 and 73:24: an entry a tracked path
        holds as a directory, written without its `/`, one level deep in a job's reads and two
        levels deep in `always`; the same directories written with their `/`, an exact path and a
        name that only begins a directory's name are admitted as written."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        tracked = ["crates/engine/src/lib.rs", "tools/x.py", "Cargo.toml"]
        cases = (
            (
                '{"schema": "read-map.v1", "always": [], "jobs": {"rust": {"reads": ["tools"]}}}',
                "rust names the directory tools without its trailing /",
            ),
            (
                '{"schema": "read-map.v1", "always": ["crates/engine"], "jobs": {}}',
                "always names the directory crates/engine without its trailing /",
            ),
        )
        for text, reason in examined("directories named without their /", cases):
            with self.subTest(reason=reason):
                self.assertEqual(gate.read_map(text, tracked), (None, reason))
        text = (
            '{"schema": "read-map.v1", "always": ["crates/engine/"], '
            '"jobs": {"rust": {"reads": ["tools/", "Cargo.toml", "tool"]}}}'
        )
        admitted = {
            "schema": "read-map.v1",
            "always": ["crates/engine/"],
            "jobs": {"rust": {"reads": ["tools/", "Cargo.toml", "tool"]}},
        }
        self.assertEqual(gate.read_map(text, tracked), (admitted, None))

    def test_a_star_entry_is_never_read_as_a_directory(self):
        """MUTATION COVERAGE 72:25: `*` reads every path, so a tracked directory named `*` never
        makes it a directory written without its `/`; the map is admitted as written."""
        gate = load("path_gate_reader", SCRIPT.read_text(encoding="utf-8"))
        text = '{"schema": "read-map.v1", "always": ["*"], "jobs": {"rust": {"reads": ["*"]}}}'
        admitted = {"schema": "read-map.v1", "always": ["*"], "jobs": {"rust": {"reads": ["*"]}}}
        self.assertEqual(gate.read_map(text, ["*/notes.md"]), (admitted, None))


if __name__ == "__main__":
    unittest.main()
