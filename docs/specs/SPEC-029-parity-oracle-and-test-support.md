# SPEC-029: the parity oracle writes provable goldens, and every crate reads them through one reader

- **Wave:** W0. **Issue:** #22 (epic #1). **Context(s):** `repo` (`tools/parity-oracle/`), `deck-streak-coordination` (the reader's own tests).
- **Decided by:** ADR-012 (testing and the parity oracle), ADR-002 (no crate outside the map), ADR-018 (no predecessor source copied), and this SPEC's ADR-029 (one reader included by path, a registry per SPEC, two digests per golden).
- **Status:** judged: delivered with its tests and the first golden, `goldens/study_day.json`,
  generated at the predecessor's `27ee2bc`. The delivery amended R2 to R4 where the code made them
  exact (§7).

## 1. The problem, measured

- **What exists.** SPEC-002 delivered `tools/parity-oracle/generate.py` with an empty `FUNCTIONS`
  registry, one golden schema (`phx.parity-golden.v1`, strict JSON, `sort_keys`,
  `allow_nan=False`, the predecessor's commit, the generator's own sha256, `inputs: synthetic`,
  seed 20) and two unit tests over a stand-in module. `tools/parity-oracle/goldens/` is empty, no
  Rust code can read a golden, and the generator's docstring promises a
  `tools/parity-oracle/test_goldens.py` that does not exist
  (`ls tools/parity-oracle/`).
- **What the port needs from it.** Every W0 to W8 SPEC that ports game math or a guard names a
  golden of a predecessor function: the study day (`analytics.py:study_day`), the change probe
  (`anki_reader.py:probe_change_signal`), the tick layout (`timebase.py:tick_minutes`), the
  catch-up window (`scheduler.py:run_startup_catchup`), the redaction filter
  (`logging_redact.py:SecretRedactingFilter`), and the whole economy from W3 on. Three shapes the
  current generator cannot express:
  - a function whose arguments are not JSON (`study_day` takes a `CollectionConfig`; a method
    needs its object, a store or a patched sleep), which ADR-012 says needs an adapter "recorded
    in the golden's note";
  - a constant the port must use verbatim (`scheduler.py:CATCHUP_MAX_LATE_MIN`,
    `constants.py:SYNC_RETRY_ATTEMPTS`), which CHARTER 8 forbids re-deriving by hand;
  - a return value that is a `date`, which `json.dumps` refuses.
- **Two defects the current design would cause.**
  - Every golden records the sha256 of `generate.py`, and every wave edits `FUNCTIONS` inside
    `generate.py`, so each registration would turn every earlier golden stale at once, and six
    parallel builders would conflict on one dict literal.
  - `test_generate.py` creates its temporary directory with `tempfile.mkdtemp()` and never removes
    it (read at `main` e05dfa5), which SPEC-030's temporary-file lint refuses.
- **The oracle's first real golden.** The study day is the function every other rule stands on,
  so this delivery registers it and commits `goldens/study_day.json`; SPEC-020 proves the kernel
  against it.

**Order.** This SPEC lands FIRST in W0, before SPEC-020: the kernel's study-day criterion reads the
golden this SPEC commits, through the reader this SPEC adds. It touches no kernel file, so it
conflicts with nothing. Every later SPEC that names a golden registers its own module under
`tools/parity-oracle/registry/` and needs this SPEC landed. SPEC-030 lands after it (its lint reads
the tests this SPEC repairs).

## 2. Requirements

R1. The registry is one module per SPEC, `tools/parity-oracle/registry/spec_NNN.py`, each exporting
    a `FUNCTIONS` mapping; `generate.py` loads every module in path order and refuses (exit 2,
    naming both modules) a golden name registered twice. `generate.py` itself registers nothing.
R2. A registration is one of three kinds, and the golden records which (`kind`):
    - `function`: a predecessor function by dotted path, relative to the predecessor's package
      (`generate.py --package`), called with the case's keyword arguments (the existing
      behaviour);
    - `adapter`: a function defined in the registry module that receives the predecessor's function
      resolved, resolves any other predecessor object it needs by name, builds the non-JSON
      arguments from the case's JSON input, calls the function, and returns what it returned (R3
      writes it as JSON). The golden records `function` (the predecessor function the
      adapter drives), `adapter` (the adapter's name) and `note` (one sentence on what the adapter
      builds or patches);
    - `constants`: a list of dotted attribute paths; each case is `{"input": {"name": <path>},
      "output": <the attribute's value>}`, read from the predecessor's module, never typed.
R3. No golden carries a calendar date or time string. An instant is epoch milliseconds, a day is
    its epoch day number (whole days since the Unix epoch), and a duration is a number of seconds
    or milliseconds named in its key. The generator converts a returned `date` to its epoch day
    number and an aware `datetime` to epoch milliseconds, for every kind of registration; an
    adapter converts what the generator cannot name, such as a duration.
R4. Each golden records `generator_sha256` (of `generate.py`), and `registry` with
    `registry_sha256` (the path and digest of the registry module that built it), beside the
    existing `source_commit`, `generator`, `seed`, `inputs` and `schema`. A golden is stale when
    either digest differs from the committed file; editing one registry module never stales
    another module's golden.
R5. `tools/parity-oracle/test_goldens.py` reads every committed golden, prints
    `examined N golden(s)` and refuses zero, and fails a golden whose schema is not
    `phx.parity-golden.v1`, whose cases are empty, whose digests differ from the committed
    generator and registry module, whose `inputs` is not `synthetic`, or whose `seed` differs from
    the generator's `SEED`. It never imports the predecessor, so it runs in public CI.
R6. A registry module never reads a file, a database or the network: a case builder receives only a
    `random.Random` seeded with `SEED`, and `test_goldens.py` refuses a registry module whose
    source calls `open`, imports `sqlite3`, `socket`, `urllib` or `http`, or reads a `Path`
    (a static read of each module's syntax tree, with a planted module that it must refuse).
    An adapter may build a synthetic SQLite file in a temporary directory from the case's own
    input; that is a write of synthetic data, never a read of a real one.
R7. `tools/parity-oracle/golden.rs` is the one Rust reader. It parses a golden into typed cases
    (`input` and `output` as JSON values, `class` optional), refuses a golden with the wrong
    schema, no case, or a missing field, and exposes a function that iterates every case of a named
    golden, prints `examined N case(s) of <function>`, and panics on zero.
R8. The reader is compiled into a crate's integration tests with
    `#[path = "../../../tools/parity-oracle/golden.rs"] mod golden;` (ADR-029). No crate is added
    to the workspace, and no crate's production dependencies change; a proving crate adds `serde`
    and `serde_json` as dev-dependencies only.
R9. The study-day golden is registered in `registry/spec_029.py` as an adapter over
    `analytics.py:study_day`: input `{instant_ms, rollover_hour, utc_offset_minutes}`, output the
    epoch day number. Its cases, seeded, cover: an instant one millisecond before and exactly at
    the rollover for rollover hours 0, 4 and 23; offsets of -720, 0, +330 and +840 minutes; an
    instant before the Unix epoch; and an offset that moves the local day across the UTC day. Each
    boundary case carries `class: "rollover"`, `class: "negative"` or `class: "offset"`.
R10. The goldens are generated on the owner's private checkout of the predecessor, never in public
    CI (ADR-012); the delivery's pull request says which predecessor commit produced them, and
    carries no predecessor source (ADR-018).
R11. `test_generate.py`'s temporary directories are removed by the test (`tempfile.TemporaryDirectory`
    or `addCleanup`), so SPEC-030's lint finds no leak in the oracle.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a golden records the digests of its generator and of the registry module that built it, with sort_keys and no NaN | `test_generate.py` `a_golden_records_the_digests_of_its_generator_and_registry` |
| A2 | two registry modules registering one golden name are refused, naming both | `test_generate.py` `two_registrations_of_one_golden_name_are_refused` |
| A3 | an adapter golden names the predecessor function it drives, its adapter and its note | `test_generate.py` `an_adapter_golden_names_the_function_it_drives` |
| A4 | a constants golden holds each named attribute's value, read from the module | `test_generate.py` `a_constants_golden_holds_each_named_value` |
| A5 | a returned date is written as its epoch day number, never as a date string | `test_generate.py` `a_returned_date_is_written_as_its_epoch_day` |
| A6 | a golden whose generator digest differs from the committed generator fails | `test_goldens.py` `a_golden_whose_generator_digest_differs_is_refused` |
| A7 | a golden whose registry digest differs from its committed module fails, and no other golden does | `test_goldens.py` `a_golden_whose_registry_digest_differs_is_refused_alone` |
| A8 | every committed golden records `inputs: synthetic` and the generator's seed (no personal data) | `test_goldens.py` `every_committed_golden_records_its_seed_and_synthetic_inputs` |
| A9 | a registry module that reads a file, a database or the network is refused | `test_goldens.py` `a_registry_module_that_reads_a_file_is_refused` |
| A10 | no committed golden holds a calendar date string | `test_goldens.py` `no_committed_golden_holds_a_calendar_date` |
| A11 | the study-day golden carries rollover, negative and offset boundary cases | `test_goldens.py` `the_study_day_golden_carries_its_boundary_classes` |
| A12 | the Rust reader reports how many cases it examined for a function | `cargo test -p deck-streak-coordination --test golden_reader -- --exact the_reader_reports_how_many_cases_it_examined` |
| A13 | the Rust reader refuses a golden with no case | `cargo test -p deck-streak-coordination --test golden_reader -- --exact a_golden_with_no_case_is_refused` |
| A14 | the Rust reader refuses a golden of another schema | `cargo test -p deck-streak-coordination --test golden_reader -- --exact a_golden_of_another_schema_is_refused` |

```acceptance
A1: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k a_golden_records_the_digests_of_its_generator_and_registry
A2: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k two_registrations_of_one_golden_name_are_refused
A3: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k an_adapter_golden_names_the_function_it_drives
A4: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k a_constants_golden_holds_each_named_value
A5: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k a_returned_date_is_written_as_its_epoch_day
A6: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k a_golden_whose_generator_digest_differs_is_refused
A7: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k a_golden_whose_registry_digest_differs_is_refused_alone
A8: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k every_committed_golden_records_its_seed_and_synthetic_inputs
A9: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k a_registry_module_that_reads_a_file_is_refused
A10: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k no_committed_golden_holds_a_calendar_date
A11: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k the_study_day_golden_carries_its_boundary_classes
A12: cargo test -p deck-streak-coordination --test golden_reader -- --exact the_reader_reports_how_many_cases_it_examined
A13: cargo test -p deck-streak-coordination --test golden_reader -- --exact a_golden_with_no_case_is_refused
A14: cargo test -p deck-streak-coordination --test golden_reader -- --exact a_golden_of_another_schema_is_refused
```

The Python tests use a synthetic stand-in package written into a temporary directory (as
`test_generate.py` already does); none imports the predecessor. The Rust tests read a planted
golden from `crates/coordination/tests/fixtures/goldens/` and the committed `study_day.json`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `tools/parity-oracle/generate.py` | repo | changed: loads `registry/*.py`, three registration kinds, date conversion, both digests |
| `tools/parity-oracle/registry/spec_029.py` | repo | added: the study-day adapter and its seeded case builder |
| `tools/parity-oracle/golden.rs` | repo | added: the one Rust reader (ADR-029) |
| `tools/parity-oracle/goldens/study_day.json` | repo | added: generated on the owner's checkout |
| `tools/parity-oracle/test_generate.py` | repo | changed: A1 to A5, temporary directories cleaned up |
| `tools/parity-oracle/test_goldens.py` | repo | added: A6 to A11 |
| `tools/parity-oracle/README.md` | repo | added: how to register a function, an adapter or a constant, and how to regenerate |
| `crates/coordination/tests/golden_reader.rs` | `deck-streak-coordination` | added: A12 to A14 |
| `crates/coordination/tests/fixtures/goldens/empty_cases.json` | `deck-streak-coordination` | added: planted golden with no case |
| `crates/coordination/tests/fixtures/goldens/wrong_schema.json` | `deck-streak-coordination` | added: planted golden of another schema |
| `crates/coordination/Cargo.toml` | `deck-streak-coordination` | changed: `serde`, `serde_json` as dev-dependencies |
| `Cargo.toml` | workspace | changed: `serde` (derive) and `serde_json` in `[workspace.dependencies]`, admitted by ADR-029 |
| `Cargo.lock` | workspace | changed |
| `docs/schematics/parity-oracle-goldens.md` | repo | added |
| `docs/decisions/ADR-029-one-golden-reader-included-by-path.md` | repo | added |
| `docs/red-first/SPEC-029.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It registers no predecessor function but the study day: each SPEC that ports a rule registers its
  own module and commits its own goldens (#11, #16, #20).
- It proves nothing about the kernel's study day; the kernel's test reads this golden
  (#11).
- It builds no oracle row of the data-migration pack: those judge `data-migration.json`, which the
  v9 import writes (#61).
- It runs no generator in CI and adds no workflow: the predecessor is private (ADR-012), and the
  temporary-file lint over these tests is SPEC-030's (#23).

## 6. Risks

- **A golden goes stale silently.** Detected by A6 and A7 on every run of the gate's python stage;
  the per-module digest keeps the blast radius to the module that changed.
- **An adapter re-implements the rule it should call.** An adapter that computes instead of calling
  would encode the porter's reading. Detected in review: the golden's `function` must name a
  predecessor function, and the adapter's `note` must say what it builds or patches, never what it
  computes; ADR-029 records this as the adapter's contract.
- **A seeded case builder drifts between Python versions.** `random.Random(20)` is stable within
  CPython's documented guarantees for `random()`, `randrange()` and `choice()`; a changed sequence
  changes the golden's cases and its registry digest together, which reads as a regeneration, not
  a pass. The generator records the Python version in the golden's `note` of the study-day
  adapter.
- **The reader diverges between crates.** It cannot: every crate compiles the same file, so a
  change to `golden.rs` is tested by every crate that includes it.

## 7. Amendments at delivery

- **R2: paths are relative to the predecessor's package, and an adapter receives its function.** A
  full dotted path would write the predecessor's package name into every registry module and every
  golden's `function`, while this repository cites the predecessor by `module.py:function` only.
  `generate.py` takes the package as `--package` on the owner's machine and resolves each path
  against it. It hands an adapter the function the registration names, already resolved, so the
  golden's `function` is by construction the one the adapter called.
- **R3: the generator converts dates, for every kind.** A `function` registration has no adapter to
  convert the `date` it returns, and A5 and the manifest already put the conversion in
  `generate.py`.
- **R4: a golden records its module's path.** `test_goldens.py` needs the path, `registry`, to find
  the committed module whose digest the golden records.
