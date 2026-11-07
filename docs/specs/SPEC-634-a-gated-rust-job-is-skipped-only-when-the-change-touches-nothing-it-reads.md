# SPEC-634: a gated Rust job is skipped only when the change touches nothing it reads

Path:
`docs/specs/SPEC-634-a-gated-rust-job-is-skipped-only-when-the-change-touches-nothing-it-reads.md`.

- **Issue:** #1261 is this delivery's issue. This delivery is the proof half of the path gates:
  a later delivery calls its decisions from `ci.yml`, and nothing calls them before that.
- **Context(s):** one new script, `scripts/path_gate.py`, with its test; one new Lean entry under
  `formal/lean/` with its vectors writer and vectors; and the formal ledger
  (`formal/obligations.json` and its test). No workflow, crate, web, deploy or iOS change.
- **Decided by:** ADR-658 (a pure decision module ported to Lean, the declarative map meaning, the
  merged exact claim and the count-based admission it replaces).
- **Schematic:** `docs/schematics/path-gate-634.md` (data flow: the map's reader, `reads`,
  `decide`, `admit`, the Lean port, the vectors writer and the vectors test).
- **Red-first record:** `docs/red-first/SPEC-634.md`.
- **Status:** delivered by the pull request that adds this file, in two parts: first the documents,
  the decision module with a stand-in `decide`, the Lean entry and its vectors writer; then the
  vectors test, red against the stand-in, the decision that turns it green, and the ledger row.

## 1. The problem, measured

Every figure below was read at the base, dev at the cut (the land of #1196).

- `ci.yml` runs on every pull request into dev or main with no paths filter:
  `sed -n '14,19p' .github/workflows/ci.yml | grep -c paths` reads 0. The `ci` job needs 21 jobs
  (`ci.yml:1415`), and each of them runs on every pull request, whatever the change touches.
- The aggregate admits a skipped need only by count (`ci.yml:1462-1476`): one admission for each of
  `mutation-rust`, `mutation-rows` and `mutation-python-release` whose own result is `skipped`, and a
  skipped result passes only while admissions remain. A gated job that a gate skipped would read
  as a failure.
- Three of the four gated jobs are defined (`grep -c -E '^  (rust|engine|release|census):$'
  .github/workflows/ci.yml` reads 3); `census` is the job a later delivery adds.
- `scripts/path_gate.py`, `formal/lean/Formal/PathGate.lean`, `formal/vectors/path-gate.jsonl` and
  `scripts/read-map.json` are absent (`git ls-tree HEAD -- <path>` reads nothing).
- The formal ledger holds 17 obligations and no O18
  (`grep -c -E '"id": "O[0-9]+"' formal/obligations.json` reads 17).

## 2. Requirements

R1. **The map.** `scripts/read-map.json`, which a later delivery writes, is one JSON object:
    `{"schema": "read-map.v1", "always": [...], "jobs": {"rust": {"reads": [...]}, "engine": {...},
    "release": {...}, "census": {...}}}`. An entry ending in `/` is a directory prefix, `*` is
    every path, and any other entry is an exact repository path. `always` joins every job's reads.
    A job with no entry in `jobs` reads every path, so it is never skipped.
R2. **The reader.** `read_map(text, tracked)` returns `(map, None)`, or `(None, why)` when it
    refuses the map. It parses with an `object_pairs_hook` that raises on a key named twice in one
    object, at any depth, so every admitted map names each job once. It refuses text that is not
    JSON; a top level whose keys are not exactly `schema`, `always` and `jobs`; a schema other than
    `read-map.v1`; a `jobs` that is not an object, or a job that is not an object whose one key is
    `reads`; an `always` or a `reads` that is not a list of strings; an empty entry; and a prefix
    written without its trailing `/`. `tracked` is every path of the tree the map is read in, as
    `git ls-files` lists them, and an entry that is not `*` and does not end in `/` is such a
    prefix exactly when some tracked path begins with the entry followed by `/` (`tools` beside
    `tools/x.sh`).
R3. **What a job reads.** `prefix_of(d, q)` is true when `d` ends in `/` and `q` begins with `d`.
    `reads_entry(e, q)` is true when `e` is `*`, when `e` is `q`, or when `prefix_of(e, q)`.
    `reads(m, job, q)` is false when `m` is None (a refused map has no member), true when `job`
    has no entry in `m["jobs"]`, and otherwise true when an entry of `always` or of the job's
    `reads` reads `q`.
R4. **The decision.** `decide(event, job, changed, m)` answers `"skip"` only when the event is a
    pull request into dev, `changed` is a non-empty list, `m` is a parsed map and no changed path
    is read by `job`; it answers `"run"` on every other input: a push, the release run, an empty
    release flag, an empty or unreadable change list, a refused map and a job with no entry.
    `event` is the triple of the event name, the pull request's base ref and base-is-dev's release
    flag, and it is a pull request into dev exactly when it reads `("pull_request", "dev",
    "false")`. `changed` is the merge commit's change list against its first parent with renames
    split into their two paths, or None when it cannot be read.
R5. **The admission.** `admit(needs, gate_ok, gate_out)` returns `(refused, rest)`. Of the gated
    jobs, `rust`, `engine`, `release` and `census`, a result passes when it is `success`, or when
    it is `skipped`, `gate_ok` is true (the path-gate job succeeded) and `gate_out` gives `skip`
    for that job; each gated job whose result does not pass is refused, in `needs`' order. Every
    other job is returned in `rest` with its result, in order, for the aggregate's existing loop.
R6. **The module.** `scripts/path_gate.py` uses the standard library alone, has no command line
    and no caller, reads and writes no file and starts no process. `prefix_of`, `reads_entry`,
    `reads`, `decide` and `admit` are each a top-level function, so each cover names one whole
    function.
R7. **Lean.** `formal/lean/Formal/PathGate.lean` ports the five functions branch for branch over
    text read as characters, states the map's meaning declaratively, and proves two theorems at
    `ramp=report`, each with its witness (section 8).
R8. **The vectors.** `formal/lean/Formal/PathGateVectors.lean` writes a hostile population: each
    event; change lists empty, None, inside, outside and mixed; renames as their two paths;
    prefix-boundary paths (`tools` beside `toolsx/`); `*`; a job with no entry; and one map text
    with a key named twice, whose map is None and whose decision is `run` for every job. Each
    vector carries the port's `reads` answers and its admission beside its decision. The formal
    checker writes `formal/vectors/path-gate.jsonl`; it is never edited by hand.
R9. **The test.** `scripts/tests/test_path_gate.py` reads every vector against the script and the
    reader's refusal of the vectors' map text, checks each of the entry's five cover digests
    against its function's span, and plants six controls (section 2a), each caught by named
    vectors.
R10. **The ledger.** `formal/obligations.json` gains O18, "A gated Rust job is skipped only when
    the change touches nothing it reads", with the artifacts `lean/PathGate`
    `the_gate_skips_exactly_the_changes_no_read_reaches` and
    `a_skipped_job_passes_only_on_the_gates_own_skip`, each `present`; the tie
    `{"method": "vectors", "for": "lean/PathGate", "test": "scripts/tests/test_path_gate.py::TheLeanVectors.test_every_vector_is_the_gates_decision"}`;
    these four gaps, word for word: "a Lean port proves the port; the vectors tie it to
    scripts/path_gate.py"; "the read map's completeness is held by the read-map census and the dev
    trace, not proved"; "the YAML wiring is held by test_path_gate_wiring.py, not proved"; and "a
    skip inherits the base tip's result: decide compares HEAD^1 with the merge commit, so while
    dev's push run is red for a gated job, a PR touching no read path reads ci green; the red stays
    visible on dev and in the NET"; status `tied`; and an `entries` row `lean/PathGate`,
    disposition `keep`, linked to O18. `scripts/tests/test_formal_obligations.py` follows: its ids
    through O18, a `PATH_GATE` constant and tie beside the release gates', and O18 in its rows
    test. `scripts/mutation-python.json` maps `scripts/path_gate.py` to its test module.

## 2a. The planted controls

Each loads a planted copy of `scripts/path_gate.py` with one change and asserts that the named
vectors refuse it.

| control | the planted change | caught by |
|---|---|---|
| an `always` entry dropped from `reads` | `reads` reads the job's `reads` alone | `rust-always-exact`, `rust-always-prefix`, `engine-always-exact` |
| the prefix test without its `/` boundary | `prefix_of` drops its `d.endswith("/")` test; or it compares `d` without its last `/` | `release-exact-beside-toolsx`, `release-exact-beside-subpath`, `rust-exact-beside-longer-name`; `rust-prefix-beside-toolsx`, `rust-prefix-bare-name`, `engine-always-prefix-beside-cratesx`, `engine-always-prefix-bare-name` |
| None treated as an empty list | `decide`'s None clause sets `changed` to the empty list instead of answering `run` | `changed-none-rust`, `changed-none-engine`, `changed-none-release`, `changed-none-census`, `changed-none-web` |
| a push treated as a pull request | the event test admits `push` beside `pull_request` | `event-push-into-dev`; `event-push-without-base` when the base test goes too |
| `admit`'s `gate_ok` check removed | a skipped gated job passes on the gate's output alone | `admit-gate-failed-skip`, `admit-mixed-gate-failed` |
| `reads` false for a job with no entry | `reads` answers false where the job has no entry | `unknown-job-outside`, `unknown-job-boundary`, `bare-map-no-entry` |

`decide` tests the empty change list before the None one, so the third control's planted change is
a change of behaviour: None passes the empty-list clause and reaches the loop over no paths.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every vector is the gate's decision: its `reads` answers, its decision and its admission are the script's, and the reader refuses the vectors' map text; red against the stand-in `decide`, green at the decision (R2 to R5, R8, R9) | `test_path_gate.py` `TheLeanVectors.test_every_vector_is_the_gates_decision` |
| A2 | each of the six planted controls is caught by its named vectors (R9, section 2a) | `test_path_gate.py` `ThePlantedControls` |
| A3 | each of the entry's five cover digests is its function's span digest in the script, examined at least 5 (R7, R9) | `test_path_gate.py` `TheLeanVectors.test_each_cover_is_its_span` |
| A4 | the ledger holds O18 `tied` with its tie, its gaps and its `lean/PathGate` row (R10) | `test_formal_obligations.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_path_gate.py -k test_every_vector_is_the_gates_decision
A2: python3 -m unittest discover -s scripts/tests -p test_path_gate.py -k ThePlantedControls
A3: python3 -m unittest discover -s scripts/tests -p test_path_gate.py -k test_each_cover_is_its_span
A4: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py
```

Each test prints the population it examined as `examined N`, and N is above 0.

## 4. File manifest

Part 1 holds the documents, the module with its stand-in `decide`, the Lean entry, its writer and
the vectors the formal checker writes from it; part 2 holds the test, the decision, the stamp of
its cover and the ledger.

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-634-a-gated-rust-job-is-skipped-only-when-the-change-touches-nothing-it-reads.md` | 1 | new: this file |
| `docs/decisions/ADR-658-the-path-gates-decisions-are-a-pure-module-ported-to-lean.md` | 1 | new |
| `docs/schematics/path-gate-634.md` | 1 | new |
| `changelog.d/path-gate-634.md` | 1 | new fragment |
| `docs/red-first/SPEC-634.md` | 2 | new: the red of A1 at the stand-in, and its green |
| `scripts/path_gate.py` | 1, 2 | new: the map's reader and the five functions, with a stand-in decision; part 2 gives the decision its body |
| `scripts/tests/test_path_gate.py` | 2 | new: A1 to A3 |
| `formal/lean/Formal/PathGate.lean` | 1, 2 | new: the entry (section 8); part 2 stamps the decision's cover again |
| `formal/lean/Formal/PathGateVectors.lean` | 1 | new: its vectors writer |
| `formal/lean/Formal/Vectors.lean` | 1 | one import and one arm |
| `formal/vectors/path-gate.jsonl` | 1, 2 | new: written by the formal checker, never by hand |
| `formal/obligations.json` | 2 | O18 and its `lean/PathGate` entries row |
| `scripts/tests/test_formal_obligations.py` | 2 | the ledger's expectations: O18 tied, its tie and its row |
| `scripts/mutation-python.json` | 2 | one entry: `scripts/path_gate.py` and its test module |

## 5. What this does NOT do

- It writes no map. `scripts/read-map.json` and the census that holds it complete belong to a
  later delivery; until then nothing reads a map.
- It wires nothing. The path-gate job, the `if:` of each gated job and the aggregate's call of
  `admit` belong to the delivery that adds the path gates; until it lands, every job runs as before.
- It proves no completeness of the map and no workflow wiring: a cover is a script's function, so
  `ci.yml` cannot be covered, and those are held by the later deliveries' census and tests.
- It changes no gate figure and no workflow job, step or leg.

## 6. Risks

- **A map that misses a path a job reads** would let `decide` skip that job. The theorem holds the
  decision to the map exactly; the map's completeness is the later census's, and dev's push run,
  which no gate skips, stays the net.
- **A prefix written without its `/`** would read as an exact path and miss every path under it.
  The reader refuses it from the tree it is read in, and a refused map runs every job.
- **A key named twice** would let the script (the last value wins) and the port (the first wins)
  differ; the reader refuses it, and one vector carries such a map text.
- **A skip inherits the base tip's result**, the fourth gap of O18; the red stays visible on dev.

## 7. What only CI proves

The test reads the vectors against the script in CI. The Lean entry is checked by the formal
checker on a local run, as the release gates' entry is (SPEC-502); CI fails on the vectors' tie and
on a stale cover, through A1 and A3.

## 8. Formal

The functions decide a gate verdict, so a Lean entry, `formal/lean/Formal/PathGate.lean`, ports
`path_gate.py`'s `prefix_of`, `reads_entry`, `reads`, `decide` (as `decideGate`) and `admit` (as
`admitGate`) branch for branch over text read as characters, and covers each by its anchor, in that
order. The map's meaning is stated from R1 alone and through no port: `EntryReads` (an entry is
`*`, the path itself, or ends in `/` and begins the path) and `InMap` (a refused map has no member,
a job with no entry reads every path, and otherwise a path is read when an entry of `always` or of
the job's reads reads it). Its theorems, each entering at `ramp=report`:

- `the_gate_skips_exactly_the_changes_no_read_reaches`: `ExactGate decideGate reads`, that `reads`
  answers exactly `InMap`, and that `decide` answers `skip` exactly for a pull request into dev
  whose change list is readable and non-empty, on a parsed map, when no changed path is read.
- `a_skipped_job_passes_only_on_the_gates_own_skip`: `AdmitExact admitGate`, that the admission
  refuses exactly the gated jobs whose results do not pass, in order, a result passing when it is
  `success` or `skipped` with the path-gate job's success and its `skip`, and passes every other
  job on.

Each witness ports the base. At the base `ci.yml` has no paths filter (`:14-19`) and the `ci` job
needs every job on every event (`:1415`), so every job runs on every change: the witness
`the_base_runs_a_job_on_a_change_it_never_reads` refutes `ExactGate` for that base (`alwaysRun`) at
a pull request into dev that changes `docs/a.md`, with a map whose one job reads `crates/`. The
base's aggregate admits a skip only by count (`:1462-1476`): the witness
`the_base_refuses_a_job_the_gate_skipped` refutes `AdmitExact` for its port (`baseAdmit`) at a
skipped `rust` that the gate skipped, with no mutation leg skipped. An `example` inhabits each kind
of `InMap` entry, the skip side of `ExactGate` and each side of `Passes`; the formal checker reads
one false `example`, a `tools/` entry reading `toolsx/a`, as a model error. The vectors cover each
event, every change list kind, renames, the prefix boundary, `*`, a job with no entry and the map
text with a key named twice.

## 9. Mutation rows

None in this delivery. The generated mutants of `scripts/path_gate.py` run in the Python population
once `scripts/mutation-python.json` maps it to `test_path_gate`.

## 10. Rollback

Revert the pull request. Nothing calls `scripts/path_gate.py` until the path gates land.
