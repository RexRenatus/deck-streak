---
status: accepted
decision-makers: "the DeckStreak architect"
---

# SIGINT joins the row runner's one signal handler, hold and latch, and still ends the run as an interrupt

## Context and Problem Statement

The row runner, `scripts/mutation_rows.py`, starts each killer as the leader of a process group of
its own and ends that group in the `finally` of the `try` that collects the killer's output.
ADR-482 gave SIGTERM and SIGHUP one handler for the run: it records the first signal, ignores
every later one so a cleanup runs to its end, holds a signal that arrives while a killer is
starting until the cleanup is in place, and `main()` restores the handlers it found. ADR-482 left
SIGINT with the interpreter's own handling, and SPEC-025 states it as "SIGINT is untouched (A21)".

#952, filed after the ruling on the design of #421, names what that leaves open: an interrupt that
lands while `subprocess.Popen` starts the killer raises `KeyboardInterrupt` before the `try`, and
an interrupt during the cleanup raises it inside the `finally` before the group kill. Either way
the killer's group is left running. A SIGTERM after an interrupt, or an interrupt after a SIGTERM,
during the cleanup does the same, since the latch knows only the first two signals. #952 asks
whether SIGINT joins the same hold and latch; SPEC-647 measures the gap at the base `bd953ba7`.

## Decision Drivers

- No signal that ends the runner leaves a process of the killer's group running, whenever it
  arrives and whichever signal came first.
- An interrupted run still ends by SIGINT, so a calling shell reads it as interrupted.
- A run started with SIGINT ignored keeps it ignored.
- Every anchor of an existing row stays byte-identical, and no other band is edited.
- One test per edge, red at the base for the edge's reason, and one row per behaviour.

## Decision Outcome, and what each decision was chosen against

### D1. SIGINT joins the one handler, its hold and its latch

`ending_on_signals` saves SIGINT's handler with the other two and installs the run's handler for
it, unless SIGINT was ignored when the run began: a job a shell starts in the background has it
ignored, and the interpreter itself keeps an inherited ignored SIGINT, the reason ADR-482 keeps an
ignored SIGHUP. This amends ADR-482's statement that SIGINT keeps the interpreter's own handling,
and SPEC-025's "SIGINT is untouched"; ADR-482 and SPEC-465 are left as they are, and SPEC-025
gains an insert-only section 13.

- Chosen against: leaving SIGINT out, because both edges of #952 stay open.
- Chosen against: a second handler and latch for SIGINT alone, because two latches could each begin a cleanup, and a SIGTERM during an interrupt's cleanup would still end it early.
- Chosen against: blocking SIGINT across the start with `signal.pthread_sigmask`, because the mask is inherited across fork and kept across exec, as ADR-482 found for the other two.

### D2. The first SIGINT still ends the run as `KeyboardInterrupt`

The handler raises `KeyboardInterrupt` for a first SIGINT outside a start, `released` raises it
for one held over a start, and the context raises it after the `yield` for one held at a failed
start. An uncaught one ends the process by SIGINT, so a calling shell reads the run as
interrupted, as it did at the base. SIGTERM and SIGHUP keep `SystemExit(128 + signum)`.

- Chosen against: `SystemExit(130)` like the other two, because the process would end with a status rather than by the signal, so a shell running it in a loop would take the interrupt as handled and go on.
- Chosen against: catching `KeyboardInterrupt` in `main()`, because it would hide the interpreter's own exit by SIGINT.

### D3. Every committed row keeps its line

Each SIGINT branch is inserted immediately before the unchanged `raise SystemExit(...)` line it
precedes, so S46512, S46519 and every other row on the runner still find their lines once, and
no other band is edited.

- Chosen against: one helper that raises the ending of a recorded signal, because although it is fewer lines it re-anchors two rows of SPEC-465's band, the move ADR-482 admitted once for S02506.

### D4. Tests, rows and one push

Four criteria read red at the base by assertion (A1, A2, A4 and A5) and two guards read green there
(A3 and A6). Six rows, `S64700` to `S64705`, pin the installation, the ignored case, each of the
three interrupt endings and the saved handler. The delivery is one pull request.

- Chosen against: testing the interrupt only in `scripts/tests/test_mutation_rows_group.py`, because the start-window and cleanup-window edges need SPEC-465's driver and edge markers, which live in `scripts/tests/test_mutation_rows.py`.

### D5. Formal not required

The latch gains one signal and one ending. ADR-482 D5 declined a TLA+ model of this latch because
every cell of its table already has a killer, and that holds again here: each new edge is a
criterion and a row. The span the Lean entry covers, `retirement_records`, is byte-identical.

- Chosen against: a TLA+ model of the three-signal latch, because it would restate a table whose every cell has a killer, at the cost of a model nothing in CI checks.

## Consequences

- Good, because an interrupt can no longer strand a killer's group in either window.
- Good, because the three signals share one rule: the first ends the run, every later one is
  ignored until the cleanup ends.
- Bad, because an interrupt during a killer's start now ends the run only once the start returns.
- Bad, because a caller's own SIGINT handler hears nothing while a run lasts.

## Confirmation

`scripts/tests/test_mutation_rows.py` drives a SIGINT during the start, a second signal in each of
three orders during the cleanup, a run with SIGINT ignored and a single SIGINT into a runner
process, and asserts by pid that the killer and its grandchild are gone and how the runner ended
(A1 to A3 and A6); it asserts in-process that `main()` ends a SIGINT way out as
`KeyboardInterrupt` and restores the SIGINT handler on every way out, and that a SIGINT held at a
failed start still ends the run (A4 and A5). The six rows of `S64700-S64799` read KILLED by id,
and the pull request's Python mutation legs judge the diff's mutants at the head.

## What would make this wrong

- A caller that needs its own SIGINT handler called during a run.
- A platform where an uncaught `KeyboardInterrupt` does not end the process by SIGINT.

## More Information

Issue #952, following #432, #421 and #409; SPEC-647; SPEC-465; SPEC-025 sections 10, 12 and 13;
ADR-482; ADR-196.
