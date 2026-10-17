# SPEC-465: a wrongly shaped retired list is refused by name, and the row runner ends its killer's group on every signal

- **Issues:** #421 and #432. **Context(s):** `repo` (`scripts/mutation_rows.py`, its tests, this
  SPEC's band, the Lean entry of its reader and `docs/`).
- **Decided by:** ADR-482 (this SPEC's own: where the shape is checked, how the signals are
  latched, held and restored, and what each was chosen against), ADR-196 (the SIGTERM handler of
  #409, one of whose consequences ADR-482 supersedes), ADR-122 (the one parser) and SPEC-039 R8 to
  R11 (the rows, the runner and the retirement check). It amends SPEC-122 (sections 13 and 14) and
  SPEC-025 (section 12), each insert-only.
- **Status:** delivered in one pull request with its tests, its rows, its Lean entry and its
  red-first record, `docs/red-first/SPEC-465.md` (ADR-016).

## 1. The problem, measured

Measured at the base `36c4783a`, by `git show <base>:<path>` and `grep -n`.

**The retired list (#421).** `retired` (`scripts/mutation_rows.py:1383`) reads its base's rows
(`:1384`) and returns early when the base holds none (`:1385-1388`). Otherwise it reads
`scripts/mutation-rows.retired.json` (`RETIRED`, `:75`) through `parse_document` (`:1393`), which
refuses a repeated key as `<file> repeats the key '<key>' in one object` and a text that is not
JSON as `<file> is not JSON: ...` (`:234-244`), each a `PopulationRefused`. `main()` prints such a
refusal as `mutation_rows: REFUSED: <sentence>` on stderr and returns 2 (`:1489-1491`); a missing
tool prints `retired: REFUSED: ...` and returns 2 (`:1492-1494`); a row that left while its target
stays is refused per row and the verb returns 1 (`:1407-1415`). The list is then iterated with no
shape check, `for entry in listed.get("retired", [])` (`:1394`), `entry.get("reason", ...)` and
`entry.get("approval", ...)` (`:1395`) and `record[entry["id"]] = entry` (`:1396`):

| the list | at the base |
|---|---|
| a top-level array, string, number or null | `AttributeError` at `:1394`, traceback, exit 1 |
| `{"retired": 1}` or `{"retired": null}` | `TypeError` at `:1394`, traceback, exit 1 |
| `{"retired": "x"}`, or an object holding keys | `AttributeError` at `:1395`, traceback, exit 1 |
| `{"retired": {}}` or `{"retired": ""}` | read as no record, silently |
| `{"retired": [1]}` | `AttributeError` at `:1395`, traceback, exit 1 |
| an entry with a reason and an approval and no `id` | `KeyError` at `:1396`, traceback, exit 1 |
| an entry whose `id` is an array or an object, with a reason and an approval | `TypeError` at `:1396`, traceback, exit 1 |

The committed list is an object holding `_` and `retired`, an array of 2 objects, each with a
string `id`, a `reason` and an `approval`, so it reads under every rule below. The population
reader refuses its own shapes in the same words (`assemble`, `:160-199`: `must hold a top-level
object`, `` `tables` is not an object ``, `is not an array of rows`).

**The signals (#432).** `main()` (`:1430`) first calls `end_on_sigterm()` (`:1431`), which installs
`lambda signum, _frame: sys.exit(128 + signum)` for SIGTERM (`:1427`), and nothing else: the
module names no other signal but SIGKILL, in `kill_group` (`:1030`). `run_killer` (`:1033`) calls
`run_in_own_group` (`:1047`), which resolves the tool (`:992`), starts the killer as a group leader
inside `subprocess.Popen(...)` (`:994-1003`, `process_group=0` at `:1002`), and only then enters the
`try` that collects its output (`:1010-1011`); its `finally` (`:1016-1020`) ends the group when the
leader still runs, waits for it and closes both pipes. Nothing between `:1430` and every `return` of
`main()` restores the handler. So:

- SIGHUP takes its default action: the runner ends with no `finally`, and the killer's group runs
  on.
- A SIGTERM handled after the fork inside `Popen` and before `:1010` raises `SystemExit` outside
  the `try`, and the group runs on. Which bytecode boundary in that span runs a pending handler is
  not measured here; the cure holds the signal across the whole span.
- A second SIGTERM handled inside the `finally` before `kill_group` (`:1016-1017`) raises a second
  `SystemExit` that abandons the cleanup, and the group runs on.
- A caller that runs `main()` in-process keeps the SIGTERM handler after it returns; the tests that
  do so restore it themselves (`scripts/tests/test_mutation_rows.py:1251-1252`).

The issue cites SPEC-025 sections 10 and 11. SPEC-025 is the API service shell's SPEC, and the row
runner's signal handling lives in its amendments, section 8 (#366) and sections 10 and 11 (#409,
criteria A18 to A21, rows S02506 to S02508), decided by ADR-196; SPEC-039, the runner's own SPEC,
states none of it. This SPEC amends SPEC-025 with section 12 and states the criteria here.

**Anchors.** 90 rows target `scripts/mutation_rows.py`. Inside the edited spans: S03966 (the tool's
resolution, `try:` and `process = subprocess.Popen(`), S03975 (the `except OSError` block through
`try:` and `stdout, stderr`), S03978, S02503, S02504, S02505, S02507, S02508, S12210 (the
`parse_document` call of the list), S03905, S12206, S03960, S03961, S03962, S03968 and S02506 (the
SIGTERM installation, `:1427`). The cure keeps each find byte-identical but S02506's.

## 2. Requirements

R1. **A list that is not an object is refused.** When the document `parse_document` returns is not
    a JSON object, `retired` raises `PopulationRefused("scripts/mutation-rows.retired.json must hold
    a top-level object")`.
R2. **A `retired` value that is not an array is refused.** When the object holds `retired` and its
    value is not an array, the sentence is ``scripts/mutation-rows.retired.json: `retired` is not an
    array of records``. An object with no `retired` key holds no record, as at the base.
R3. **An entry that is not an object is refused at its place.** The entries are read in order, and
    the first that is not an object is refused as `scripts/mutation-rows.retired.json: retired[<i>]
    is not an object`, `<i>` its index from 0.
R4. **An entry whose `id` is not a string is refused at its place.** The first object entry whose
    `id` is missing or is not a string is refused as `scripts/mutation-rows.retired.json:
    retired[<i>] has no string id`, whatever its reason and approval. R3 and R4 are judged entry by
    entry, so the first wrong entry is the one named.
R5. **Each refusal reads as a repeated key's does, and a well-formed list reads as before.** The
    checks sit in one reader, `retirement_records(listed)`, called on the line after the list's
    `parse_document` call. Each refusal reaches stderr as `mutation_rows: REFUSED: <sentence>`,
    exit 2, with no traceback, nothing on stdout and no row retired. The reader keeps, as the base
    does, each entry whose reason and approval are both non-blank, keyed by its `id`. The list is
    read only when the base holds rows, as at the base.
R6. **SIGTERM and SIGHUP end the run through its cleanups.** `main()` installs one handler for
    SIGTERM and, unless SIGHUP is ignored when `main()` begins, for SIGHUP. The first signal it
    handles raises `SystemExit(128 + signum)`, so every `finally` runs, the killer's group ends and
    the runner exits 143 for SIGTERM and 129 for SIGHUP. A SIGHUP ignored when `main()` begins stays
    ignored.
R7. **A signal while a killer starts is held until its group can end.** `run_in_own_group` holds
    signals from before it resolves the killer's tool until inside the `try` whose `finally` ends
    the group, where `released(process)` ends the hold and raises a held signal's exit. A signal
    held when the start fails (a refusal or an exception before the `try`) ends the run with
    `128 + signum` when the command returns.
R8. **A later signal during the cleanup is ignored.** Once a signal is recorded, every later
    SIGTERM or SIGHUP is ignored until `main()` returns, so a cleanup runs to its end, and the exit
    status is the first signal's.
R9. **`main()` leaves the process as it found it.** On every way out (a return, a `SystemExit`, an
    exception, a signal's exit), `main()` reinstalls each handler it found for SIGTERM and SIGHUP,
    the default action where `signal.getsignal` read `None`, and the signal state of the run before
    it, none outside `main()`; there, holding and releasing do nothing, as at the base. The base's
    `main` body becomes `dispatch(argv)` unchanged but for its first line, and `main(argv)` is
    `with ending_on_signals(): return dispatch(argv)`.
R10. **Nothing else moves.** Fifteen rows join `S46500-S46599`, so `count` and `ids` differ from
    the base by them alone; S02506 keeps its id, killer and description, with its find on the new
    SIGTERM installation; every other find on the file stays byte-identical; no import is added; no
    workflow changes; SPEC-025's A18 to A21 and SPEC-122's A7 hold unchanged.
R11. **The reader carries a Lean proof.** `lean/RetiredListShape` ports `retirement_records` branch
    for branch, at `ramp=report`, with the claim that every document is refused by R1 to R4's
    sentence or read as the base reads a well-formed one, a witness against the base's reading at
    a top-level array, and vectors its writer derives, which a test drives through the reader.

## 3. Acceptance criteria of SPEC-465

| id | criterion | kind | decided by |
|---|---|---|---|
| A1 | a top-level array is refused with R1's sentence, exit 2, no `Traceback`, no row retired, and the reader raises the same sentence in-process | red-first | `test_a_top_level_list_is_refused_naming_the_shape` |
| A2 | a `retired` value of `1` and one of `{}` are each refused with R2's sentence, as in A1 | red-first | `test_a_retired_value_that_is_not_an_array_is_refused_naming_the_shape` |
| A3 | `retired: [<a well-formed entry>, 1]` is refused with R3's sentence for `retired[1]`, as in A1 | red-first | `test_an_entry_that_is_not_an_object_is_refused_naming_its_place` |
| A4 | `retired: [<a well-formed entry>, <an entry with a reason and an approval and no id>]` is refused with R4's sentence for `retired[1]`, as in A1 | red-first | `test_an_entry_without_a_string_id_is_refused_naming_its_place` |
| A5 | a list holding `_`, an entry with a blank approval and the leaving row's approved entry admits that row, exit 0, `examined 1`; the retirement and repeated-key tests stay green | green at base | `test_a_well_formed_list_still_admits_its_row_beside_an_unapproved_entry`; `test_a_row_that_leaves_while_its_target_stays_is_refused`; SPEC-122's A7 tests |
| A6 | a SIGHUP of a runner process whose killer and grandchild run leaves neither running, and the runner exits 129 | red-first | `test_a_sighup_of_the_runner_ends_the_killers_group` |
| A7 | a SIGTERM sent while the killer is starting, after its fork and before its output is collected, leaves neither running, and the runner exits 143 | red-first | `test_a_sigterm_while_the_killer_starts_is_held_until_its_group_can_end` |
| A8 | a second SIGTERM sent once the cleanup has begun, before the group kill, is ignored: neither runs, and the runner exits 143 | red-first | `test_a_second_sigterm_during_the_cleanup_is_ignored_and_the_group_ends` |
| A9 | a runner started with SIGHUP ignored, sent SIGHUP then SIGTERM, exits 143 and leaves neither running | green at base | `test_a_runner_started_with_sighup_ignored_keeps_it_ignored` |
| A10 | after `main()` returns, exits by `SystemExit` (an argument error, a signal) or raises, both handlers are the ones it found, and a handler `signal.getsignal` reads as `None` comes back as the default action | red-first | `test_main_restores_the_handlers_it_found_on_every_way_out` |
| A11 | two in-process runs each ended by a signal exit 143, and a killer run after them reads `Run(1, True)` | green at base | `test_main_leaves_no_signal_state_behind` |
| A12 | a SIGTERM that arrives while a killer's tool is being resolved, when the resolution then refuses, ends the run with 143 | green at base | `test_a_signal_held_while_a_killer_fails_to_start_still_ends_the_run` |
| A13 | every committed row, S02506 and S46500-S46599 among them, is anchored once and names one test, and SPEC-025's A18 to A21 stay green | green at base | `test_every_committed_row_is_anchored_once_and_names_one_test`; `test_mutation_rows_group` tests |
| A14 | for every vector of `formal/vectors/retired-list-shape.jsonl`, `retired` refuses with the vector's sentence or admits the leaving row exactly when the vector says so | red-first | `test_the_reader_agrees_with_every_vector_of_its_proof` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_top_level_list_is_refused_naming_the_shape
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_retired_value_that_is_not_an_array_is_refused_naming_the_shape
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_an_entry_that_is_not_an_object_is_refused_naming_its_place
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_an_entry_without_a_string_id_is_refused_naming_its_place
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_well_formed_list_still_admits_its_row_beside_an_unapproved_entry -k test_a_row_that_leaves_while_its_target_stays_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_band_repeated_key.py -k test_a_retired_list_that_repeats_a_key_is_refused_naming_it -k test_a_well_formed_retired_list_still_admits_its_row
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sighup_of_the_runner_ends_the_killers_group
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigterm_while_the_killer_starts_is_held_until_its_group_can_end
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_second_sigterm_during_the_cleanup_is_ignored_and_the_group_ends
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_runner_started_with_sighup_ignored_keeps_it_ignored
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_main_restores_the_handlers_it_found_on_every_way_out
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_main_leaves_no_signal_state_behind
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_signal_held_while_a_killer_fails_to_start_still_ends_the_run
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_every_committed_row_is_anchored_once_and_names_one_test
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_group.py
A14: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_reader_agrees_with_every_vector_of_its_proof
```

Every new test lives in `scripts/tests/test_mutation_rows.py`: A1 to A5 in
`TheRetiredListRefusesAWrongShape`, A6 to A9 in `TheRunnerEndsTheKillersGroupOnEverySignal`, A10
to A12 in `MainLeavesTheProcessAsItFoundIt` and A14 in `TheRetiredListReaderAgreesWithItsProof`.
Each asserts a positive artifact by equality (a whole stderr line, an exit status, a handler's
identity, a `Run`), never builds an expected value with the code under test, and reads no byte of
`scripts/mutation_rows.py`. The process tests drive a runner the test starts in a group of its
own, through a driver script it writes; the child writes a marker at each edge, the test waits for
it within a bound before it signals, and the child proceeds only on the test's own marker. A2,
A10 and A14 print the count they examined and refuse zero.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation_rows.py` | #421, #432 | changed: `retirement_records` and `retired`'s list read (R1 to R5); the signal state, `hold_signals`, `released`, `ending_on_signals`, `main` and `dispatch` (R6 to R9) |
| `scripts/tests/test_mutation_rows.py` | #421, #432 | changed: the four classes of section 3, their driver and killer texts |
| `scripts/mutation-rows.d/S46500-S46599.json` | #421, #432 | added: the fifteen rows of section 7 |
| `scripts/mutation-rows.d/S02500-S02599.json` | #432 | changed: S02506's find and replacement only |
| `docs/specs/SPEC-465-a-wrongly-shaped-retired-list-is-refused-by-name-and-the-row-runner-ends-its-killers-group-on-every-signal.md` | #421, #432 | added: this SPEC |
| `docs/decisions/ADR-482-the-retired-list-is-shape-checked-in-its-reader-and-the-runners-signals-are-latched-held-and-restored.md` | #421, #432 | added |
| `docs/schematics/mutation-row-runner-signals-and-the-retired-list.md` | #421, #432 | added |
| `docs/specs/SPEC-122-the-mutation-row-band-files-refuse-a-repeated-key.md` | #421 | changed: sections 13 and 14 inserted |
| `docs/specs/SPEC-025-api-service-shell.md` | #432 | changed: section 12 inserted |
| `docs/red-first/SPEC-465.md` | #421, #432 | added |
| `docs/red-first/SPEC-122.md` | #421 | changed: A8's addendum inserted |
| `changelog.d/mutation-rows-retired-shape-465.md` | #421, #432 | added |
| `formal/lean/Formal/RetiredListShape.lean` | #421 | added: the port, the claim, the theorem and the witness |
| `formal/lean/Formal/RetiredListShapeVectors.lean` | #421 | added: the vectors writer |
| `formal/lean/Formal/Vectors.lean` | #421 | changed: one import and one arm |
| `formal/vectors/retired-list-shape.jsonl` | #421 | added: written by the writer |

## 5. What this does NOT cover

- No workflow changes: the `mutation-rows` job runs `retired` as before, and a wrongly shaped list
  now fails it with exit 2 where it failed with exit 1 (#421).
- A descendant that leaves the killer's group is still not reached, and a SIGKILL of the runner
  still runs no cleanup: SPEC-025 section 10's limits stand (#409).
- SIGINT keeps the interpreter's own handling, as SPEC-025's A21 states (#409); this SPEC handles
  SIGTERM and SIGHUP (#432).
- The Python mutation legs' price for this file grows by the new tests' cost; refreshing a price
  row whose suite grew is the stale-price reading's subject (#873).
- This delivery lands on `dev` after the release pull request merges (#822).

## 6. Risks

- **A process test runs slow on a loaded runner.** Every wait is bounded (20 s) on a marker the
  other side writes, so a slow machine waits longer, never races; a run that misses a bound fails
  by name, with every pid it started ended by number.
- **A refusal is newly strict.** A `retired` value of `{}` or `""` read as no record and is now
  refused. The committed list is an array (section 1), so no committed state reads differently.
- **The hold outlives a failed start.** A refusal before the `try` leaves the hold set until
  `main()` returns; no killer starts in between (the refusal ends the run), and R7 ends the run
  with a signal that arrived there.
- **A cleanup that hangs after the first signal** can be ended only by SIGKILL (R8). The cleanup
  is a group SIGKILL, a reap of the leader and a byte restore of one target.
- **A re-anchored row.** S02506's find moves with the line it pins; its killer is unchanged and
  must read KILLED at the new find, which the rows job reads by id.
- **The census modules read the runner.** `test_mutation_rows_missing_tool.py` admits only the
  runner's current imports and refuses a spawn outside its two helpers; the cure adds neither.

## 7. The rows

All in `scripts/mutation-rows.d/S46500-S46599.json`, `SCRIPT_MUTATIONS`, target
`scripts/mutation_rows.py`, each killer a `test_mutation_rows.<Class>.<test>` that selects one test.

| row | mutant | killer |
|---|---|---|
| S46500 | R1's check removed | A1 |
| S46501 | R2's check removed | A2 |
| S46502 | R3's check removed | A3 |
| S46503 | R4's check removed | A4 |
| S46504 | the record filter's `and` to `or` | A14 |
| S46510 | SIGHUP's installation removed | A6 |
| S46511 | the hold before the tool's resolution removed | A7 |
| S46512 | `released`'s raise of a held signal removed | A7 |
| S46513 | the handler raising while a killer starts | A7 |
| S46514 | the latch that ignores a later signal removed | A8 |
| S46515 | SIGHUP installed over an ignored SIGHUP | A9 |
| S46516 | the handlers' restore removed | A10 |
| S46517 | the default action for a `None` handler removed | A10 |
| S46518 | the outer signal state left unrestored | A11 |
| S46519 | a signal held at a failed start dropped | A12 |

S02506, in `scripts/mutation-rows.d/S02500-S02599.json`, keeps its id, killer
(`test_mutation_rows_group.ASignalledRunnerEndsTheKillersGroup.test_sigterm_to_the_runner_ends_the_killers_group`)
and description; its find becomes the SIGTERM installation in `ending_on_signals`, and its
replacement removes it.

## 8. References

Issues #421 and #432; #385, #409 and #366. ADR-482, ADR-196, ADR-122. SPEC-122 sections 11 to 14,
SPEC-025 sections 8, 10, 11 and 12, SPEC-039 R8 to R11.
