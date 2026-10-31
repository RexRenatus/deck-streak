---
status: accepted
decision-makers: "the owner and the DeckStreak orchestrator"
---

# The retired list's shape is checked in its reader, and the row runner's signals are latched, held through a killer's start and restored

## Context and Problem Statement

Two gaps in `scripts/mutation_rows.py`, each measured at the base this delivery is cut from.

**A wrongly shaped retired list crashes the retirement check (#421).** `retired` reads
`scripts/mutation-rows.retired.json` through `parse_document`, which refuses a repeated key and a
text that is not JSON (SPEC-122 R8, #385), and then iterates `listed.get("retired", [])` and calls
`entry.get` on each entry with no check of either shape. A top-level array, string, number or null
raises `AttributeError` at the `.get`; a `retired` value of a number or null raises `TypeError`;
a string or an object as that value iterates its characters or keys and raises `AttributeError`;
an entry that is not an object raises `AttributeError`; an entry with a reason and an approval but
no `id` raises `KeyError`, and one whose `id` is an array or an object raises `TypeError`. Each is
an uncaught traceback and exit 1, where every other refusal of the file is one
`mutation_rows: REFUSED: <sentence>` line on stderr and exit 2. A `retired` value of `{}` or `""`
reads as no record at all, silently.

**The runner can leave a killer's process group running on a signal (#432).** After #409, `main()`
installs a SIGTERM handler, `lambda signum, _frame: sys.exit(128 + signum)`, so a SIGTERM unwinds
through `run_in_own_group`'s `finally`, which ends the killer's group. Four edges remain:

- SIGHUP has no handler: its default action ends the runner with no `finally`, and the killer's
  group, which leads a group of its own, runs on.
- The killer is forked inside `subprocess.Popen(...)`, and the `try` whose `finally` ends its group
  is entered only after `Popen` returns. A SIGTERM handled in that window raises `SystemExit`
  outside the `try`, so the group runs on.
- A second SIGTERM handled inside the `finally`, before `kill_group` runs, raises a second
  `SystemExit` that abandons the cleanup, so the group runs on.
- `main()` never restores the handler it installed, so a caller that runs `main()` in-process keeps
  a handler that exits the process on SIGTERM.

SPEC-025 states the runner's signal handling in its amendments of sections 8 (#366) and 10 and 11
(#409), not in SPEC-039, the runner's own SPEC, and ADR-196 decided it, naming the process-wide
handler a known cost. How are both gaps closed without weakening any refusal, changing what a
well-formed list admits, or moving an examined count?

## Decision Drivers

- Every refusal of the retired list reads as the ones before it: one named line on stderr, exit 2,
  no traceback, and nothing retired.
- No signal that ends the runner leaves a process of the killer's group running, whenever it
  arrives, and the runner exits `128 + signum`.
- The runner stays standard-library Python, with only the imports the missing-tool census admits.
- Every anchor of an existing row on the edited lines stays byte-identical, except the one line
  the cure must replace.
- One test per edge, red at the base for the edge's reason, and one row per behaviour.

## Decision Outcome, and what each decision was chosen against

### D1. The mechanisms, as measured

The six crash shapes and the two silent ones above are measured at the base's `retired`, its list
loop and its record line. The signal path is measured from `main()` through `end_on_sigterm`, the
killer's `Popen`, the two `try` blocks of `run_in_own_group` and its `finally`, to each return of
`main()`. What the interpreter does between the fork inside `Popen` and the bytecode that enters
the `try` is not measured here: the cure holds the signal for that whole span, so it does not rest
on that reading. The SPEC's section 1 cites each step.

### D2. The cures, and what each was chosen against

**The retired list.** A new reader, `retirement_records(listed)`, is called on the line after the
`parse_document` call, so the repeated-key refusal and the shape refusals sit side by side in one
reading. It refuses, as a `PopulationRefused` naming the file and the shape it expected, a document
that is not an object, a `retired` value that is not an array, an entry that is not an object and
an entry whose `id` is not a string, each entry named by its index. It then keeps, as before, each
entry with a non-blank reason and approval. `main()`'s existing handler prints the refusal and
returns exit 2, as it does for the repeated key. A document with no `retired` key still reads as
no record, and every well-formed list admits exactly what it admitted.

**The signals.** A small state object, created by `main()` for each run, holds the signal that is
ending the run, once one arrived, and whether a killer is being started. Its bound method is the
one handler for SIGTERM and SIGHUP:

- a signal that arrives when the run is already ending is ignored, so a cleanup runs to its end;
- otherwise the signal is recorded, and the handler raises `SystemExit(128 + signum)` unless a
  killer is starting, when it returns and the signal waits.

`run_in_own_group` starts the hold on a new line before it resolves the killer's tool, and ends it
inside the `try` whose `finally` ends the group, in the expression of the line that collects the
killer's output: `released(process).communicate(timeout=timeout)`. `released` clears the hold and
raises the held signal's exit there, so the `finally` ends the group. `main()` becomes a wrapper:
it saves both handlers, installs SIGTERM's always and SIGHUP's unless SIGHUP was ignored when the
run began, runs the command line (the base's `main` body, renamed `dispatch`), honours a signal
still held when the command returns (a killer that failed to start), and on every way out restores
each saved handler, the default action standing in for one set outside Python, and the state of
the run before it.

- Chosen against: catching `AttributeError`, `TypeError` and `KeyError` at the command's entry, because it would turn every future bug of `retired` into a refusal and could not name the shape the file needed.
- Chosen against: checking the shape inside `parse_document`, because that reader serves the header, the band files and a revision's documents, each with its own shape and its own refusal.
- Chosen against: blocking SIGTERM and SIGHUP with `signal.pthread_sigmask` across the start, because a blocked mask is inherited across fork and kept across exec, so the killer and its tests would start with both signals blocked, and the mask is per thread where the existing tests run killers on threads.
- Chosen against: setting both signals to `SIG_IGN` inside the handler to drop a second one, because it needs a second mechanism for the start window and leaves a disposition any process the cleanup started would inherit.
- Chosen against: the hold's release as its own first line inside the `try`, because that line would sit inside the find of row S03975, and its re-anchor would edit another delivery's evidence.
- Chosen against: wrapping `main()`'s body in `try`/`finally` in place, because the re-indent moves the anchors of S12206, S03960, S03961, S03962 and S03968.
- Chosen against: installing SIGHUP's handler over an ignored SIGHUP, because a run started under `nohup` asked to outlive its terminal, and the interpreter itself leaves an inherited ignored SIGINT in place for the same reason.

### D3. Production and test

Production changes `scripts/mutation_rows.py` only: `retired`'s list read, the new reader, the
signal state, `hold_signals`, `released`, `ending_on_signals`, `main` and `dispatch`. Every new test
lives in `scripts/tests/test_mutation_rows.py`, the module the Python mutation plan maps first after
the refusal module for this file, so the changed lines' generated mutants meet the tests that pin
them. A signal edge is driven into a runner started as a child process by a driver script the test
writes: the child writes a marker at the edge (the killer's grandchild started, the killer being
started, the cleanup entered), the test waits for it within a bound, sends the signal, writes a
marker of its own, and the child proceeds only when it reads that marker, through a loop whose
backward jumps run a pending handler. No step waits on a bare sleep. The restore and state edges
run in-process.

- Chosen against: the signal tests in `scripts/tests/test_mutation_rows_group.py`, because the Python mutation plan maps no run of that module to this file, so the new lines' mutants would survive its legs.
- Chosen against: raising the signal from inside the runner with `signal.raise_signal` for the process edges, because a hang-up and a cancel come from outside, and only an outside sender shows the runner's own handling.

### D4. One delivery for both issues

One delivery, SPEC-465 and ADR-482, band S46500-S46599, closes #421 and #432. It inserts sections
13 and 14 into SPEC-122 (#421's request) and section 12 into SPEC-025, each insert-only, and
re-anchors row S02506, whose line the cure replaces, keeping its id, killer and description.

- Chosen against: two deliveries, one per issue, because both edit the same production file and the same test module in regions that do not overlap, and the second could be cut only after the first lands, two gate rounds where one suffices.
- Chosen against: retiring S02506 and adding its behaviour as a new row, because a row whose code changes is re-anchored, and a retirement while its target stays needs the maintainer's approval.

### D5. Tests, rows and the formal surfaces

Nine of the fourteen criteria are red at the base for their reasons (A1 to A4, A6 to A8, A10 and
A14; the SPEC's table labels each), and the other five pin behaviour the base already had.
Rows S46500 to S46504 pin the reader's four checks and its record filter; S46510 to S46519 pin
each signal behaviour; S02506 moves to the new SIGTERM installation. Fifteen rows join the
population, so `count` reads fifteen more than at the base and no other examined count moves.

The retired list's reader is a total function whose answer decides the retirement check's verdict,
so it takes a Lean entry, `lean/RetiredListShape`, at `ramp=report`: its port, a claim that every
document is refused by name or read as the base reads a well-formed one, and a witness against the
base's reading at a top-level array. The signal handling is an interleaving surface (an outside
sender against the runner's start, run and cleanup), and a TLA+ model is declined here: its whole
state space is two signals over five phases, which the schematic enumerates, and every reachable
cell is pinned by a process test and a row. The repository's CI checks no formal entry.

- Chosen against: no Lean entry for the reader, because a pure function that decides a gate verdict takes one, and the tests are written against the same reading as the code.
- Chosen against: a TLA+ model of the signal latch, because the model would restate a five-phase table whose every cell already has a killer, at the cost of a model nothing in CI checks.

### D6. Not in this delivery

SIGINT, which keeps the interpreter's own handling (SPEC-025's A21); the Python mutation runner
and its price for this file; and the CI workflows. The SPEC's section 5 cites the issue of each.
Two other readers whose input shapes go unchecked, the `--rows-from` plan's and the population
reader's tables and rows, are left for issues of their own: neither is the retired list.

## Consequences

- Good, because a wrongly shaped retired list is refused by name, exit 2, and retires nothing.
- Good, because no signal the runner handles leaves a process of the killer's group running.
- Good, because `main()` leaves the process's handlers as it found them, so an in-process caller
  keeps its own.
- Bad, because a `retired` value of `{}` or `""`, which read as no record, is now refused, so a
  list that relied on it must be rewritten as an array.
- Bad, because after the first signal a hung cleanup can be ended only by SIGKILL, which runs no
  cleanup.
- Bad, because the new process tests add to the mapped suite's cost for this file in the Python
  mutation legs.
- This supersedes ADR-196's consequence that `main()` sets a process-wide SIGTERM disposition.

## Confirmation

`scripts/tests/test_mutation_rows.py` plants each wrong shape and reads the refusal, the exit
status and no traceback; drives SIGHUP, a SIGTERM during the start and a second SIGTERM during the
cleanup into a runner process and asserts by pid that the killer and its grandchild are gone and
the exit status; and asserts in-process that `main()` restores both handlers on every way out. The
rows of S46500-S46599 and the re-anchored S02506 read KILLED by id. The Lean entry builds with its
witness, and a test drives the reader through every vector of its proof.

## What would make this wrong

- A caller that relies on a `retired` value of `{}` reading as empty.
- A CI cancel that escalates to SIGKILL before the cleanup ends the group: no handler runs then.
- An interpreter that runs a pending handler inside `Popen` before the hold begins, which the hold
  placed before the tool's resolution rules out.

## More Information

Issues #421 and #432, following #385, #409 and #366; SPEC-465; SPEC-122 sections 11 to 14;
SPEC-025 sections 8, 10, 11 and 12; ADR-196; ADR-122.
