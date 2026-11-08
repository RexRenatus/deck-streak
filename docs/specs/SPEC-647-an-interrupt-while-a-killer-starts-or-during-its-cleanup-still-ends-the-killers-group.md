# SPEC-647: an interrupt while a killer starts or during its cleanup still ends the killer's group

- **Issue:** #952 (the follow-up the design of #421 filed; refs #432). **Context(s):** `repo`
  (`scripts/mutation_rows.py`, its tests, this SPEC's band and `docs/`).
- **Decided by:** ADR-670 (this SPEC's own: SIGINT joins the run's one handler, and how the run
  still ends as an interrupt), ADR-482 (the latch, the hold and the restore this joins, its D2 and
  D5) and ADR-196 (the SIGTERM handler of #409). It amends SPEC-025 (section 13, insert-only) and
  SPEC-465 section 5's SIGINT bullet (by ADR-670; SPEC-465 itself is unchanged).
- **Mutation band:** `S64700-S64799`, claimed, holding six rows.
- **Status:** delivered in one pull request with its tests, its rows and its red-first record,
  `docs/red-first/SPEC-647.md`.

## 1. The problem, measured

Measured at the base `bd953ba7`, by `git show <base>:<path>` and `grep -n`; every `:line` below is
in `scripts/mutation_rows.py` unless it names another file.

`main()` (`:1600`) runs `dispatch` inside `ending_on_signals` (`:1505`). That context saves the
handlers of SIGTERM and SIGHUP only, `found = {signum: signal.getsignal(signum) for signum in
(signal.SIGTERM, signal.SIGHUP)}` (`:1518`), and installs the run's one handler,
`RunSignals.handle` (`:1462`, `:1475`), for SIGTERM and for SIGHUP unless SIGHUP was ignored
(`:1522-1524`). The handler records the first signal, ignores every later one, and raises
`SystemExit(128 + signum)` at once (`:1479-1480`) unless a killer is being started; a signal held
over a start ends the run at `released` (`:1493`, `:1500`), inside the `try` whose `finally` ends
the killer's group, and one still held when the command returns ends it after the `yield`
(`:1525-1527`).

SIGINT is outside all of it: it stays on the interpreter's `default_int_handler`, which raises
`KeyboardInterrupt` wherever the main thread is. `run_in_own_group` begins the hold (`:1004`),
starts the killer by `subprocess.Popen` (`:1007`) before the `try` at `:1022`, and ends the group
in that `try`'s `finally` (`:1029-1031`). SPEC-025 states the gap as a rule: "SIGINT is untouched
(A21)" (`docs/specs/SPEC-025-api-service-shell.md:303`, and again in its section 12), and the
group tests' driver sets SIGINT to `signal.default_int_handler` before it runs `main()`
(`scripts/tests/test_mutation_rows_group.py:117`) for A21's test (`:291`).

| the interrupt | at the base |
|---|---|
| during the killer's start | `KeyboardInterrupt` inside `subprocess.Popen`, before the `try`: no cleanup runs and the group is left running |
| a second interrupt during the cleanup | `KeyboardInterrupt` inside the `finally`, before `kill_group`: the group is left running |
| a SIGTERM after an interrupt, during the cleanup | `SystemExit` raised there by the run's handler: the group is left running |
| an interrupt after a SIGTERM, during the cleanup | `KeyboardInterrupt` there, past the latch: the group is left running |
| a single interrupt outside both windows | the `finally` ends the group and the run ends by SIGINT, as A21 holds |

## 2. Requirements

- R1. SIGINT joins the run's one handler: `ending_on_signals` saves SIGINT's handler beside
  SIGTERM's and SIGHUP's and installs the run's handler for SIGINT, unless SIGINT was ignored when
  the run began (as a shell leaves it for a job it starts in the background), in which case it
  stays ignored for the whole run.
- R2. The first SIGINT ends the run as the interpreter's own `KeyboardInterrupt`: at once outside
  a killer's start; at `released`, inside the `try` whose `finally` ends the group, when it arrived
  while the killer was starting (held as SPEC-465 R7 holds the others); and when the command
  returns, when it was held while a killer failed to start. An uncaught `KeyboardInterrupt` ends
  the process by SIGINT, so a calling shell reads the run as interrupted, as at the base. SIGTERM
  and SIGHUP keep `SystemExit(128 + signum)`.
- R3. Every signal after the first is ignored, whichever signals they are, so the cleanup the
  first began ends the killer's group: a second SIGINT, a SIGTERM after a SIGINT, and a SIGINT
  after a SIGTERM.
- R4. On every way out `main()` reinstalls the SIGINT handler it found with the other two (the
  default action for one set outside Python), and SPEC-025's A21 still holds: a SIGINT of the
  runner ends the killer's group.
- R5. Nothing else moves: SIGTERM and SIGHUP behave as SPEC-465 states, every committed row on
  `scripts/mutation_rows.py` still finds its line once, no committed row is re-anchored, and six
  rows join `S64700-S64799`.

## 3. Acceptance criteria of SPEC-647

| id | criterion | kind | decided by |
|---|---|---|---|
| A1 | a SIGINT sent while the killer is starting, after its fork and before its output is collected, is held until the group kill is in place: neither the killer nor its grandchild runs after it, and the runner ends by SIGINT (R1, R2) | red-first | `test_a_sigint_while_the_killer_starts_is_held_until_its_group_can_end` |
| A2 | a second signal sent once the cleanup has begun, before the group kill, is ignored in each of three orders (SIGINT then SIGINT, SIGINT then SIGTERM, SIGTERM then SIGINT): neither runs, and the runner ends as its first signal says (R3) | red-first | `test_a_second_signal_during_the_cleanup_is_ignored_whichever_came_first` |
| A3 | a runner started with SIGINT ignored, sent SIGINT then SIGTERM, exits 143 and leaves neither running (R1) | not red | `test_a_runner_started_with_sigint_ignored_keeps_it_ignored` |
| A4 | `main()` takes SIGINT through its own handler and ends a SIGINT way out as `KeyboardInterrupt`; after a return, an argument error, an exception and a SIGINT the SIGINT handler is the one it found; an ignored SIGINT stays ignored during the run and after; a handler `signal.getsignal` reads as `None` comes back as the default action; and the handler it found hears nothing (R1, R2, R4) | red-first | `test_main_takes_sigint_and_restores_its_handler_on_every_way_out` |
| A5 | a SIGINT that arrives while a killer's tool is being resolved, when the resolution then refuses, ends the run as `KeyboardInterrupt`, and the handler `main()` found hears nothing (R2) | red-first | `test_a_sigint_held_while_a_killer_fails_to_start_still_ends_the_run` |
| A6 | a single SIGINT of the runner still ends the killer's group and the run by SIGINT, and SPEC-025's A21 test, SPEC-465's four process tests and `MainLeavesTheProcessAsItFoundIt` stay green (R2, R4, R5) | not red | `test_a_sigint_of_the_runner_still_ends_it_by_sigint_through_its_cleanup`; `test_sigint_to_the_runner_ends_the_killers_group`; SPEC-465's process tests; `MainLeavesTheProcessAsItFoundIt` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigint_while_the_killer_starts_is_held_until_its_group_can_end
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_second_signal_during_the_cleanup_is_ignored_whichever_came_first
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_runner_started_with_sigint_ignored_keeps_it_ignored
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_main_takes_sigint_and_restores_its_handler_on_every_way_out
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigint_held_while_a_killer_fails_to_start_still_ends_the_run
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigint_of_the_runner_still_ends_it_by_sigint_through_its_cleanup
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_group.py -k test_sigint_to_the_runner_ends_the_killers_group
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sighup_of_the_runner_ends_the_killers_group
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigterm_while_the_killer_starts_is_held_until_its_group_can_end
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_second_sigterm_during_the_cleanup_is_ignored_and_the_group_ends
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_runner_started_with_sighup_ignored_keeps_it_ignored
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k MainLeavesTheProcessAsItFoundIt
```

A1 to A3 and A6 run the runner as a process through SPEC-465's driver and edges: the child
writes a marker at each edge, the test waits for it within the tests' own bound before it
signals, and asserts by pid that the killer and its grandchild are gone and how the runner ended.
A4 and A5 run `main()` in this process, in the new class `TheRunEndsOnASigintAsOnItsOtherSignals`.
Every loop prints the count it examined and refuses zero. A3 and A6 read green at the base and
guard it against the cure.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation_rows.py` | #952 | changed: SIGINT saved, installed unless ignored, and ended as `KeyboardInterrupt` in `RunSignals.handle`, `released` and after the `yield`; four docstrings |
| `scripts/tests/test_mutation_rows.py` | #952 | changed: the driver's comment and its SIGINT lines, A1 to A3 and A6 in `TheRunnerEndsTheKillersGroupOnEverySignal`, the new class with A4 and A5, and SIGINT in `MainLeavesTheProcessAsItFoundIt.setUp` |
| `scripts/mutation-rows.d/S64700-S64799.json` | #952 | added: the six rows of section 7 |
| `docs/specs/SPEC-647-an-interrupt-while-a-killer-starts-or-during-its-cleanup-still-ends-the-killers-group.md` | #952 | added: this SPEC |
| `docs/decisions/ADR-670-sigint-joins-the-runners-one-signal-handler-and-still-ends-the-run-as-an-interrupt.md` | #952 | added |
| `docs/schematics/mutation-row-runner-sigint.md` | #952 | added |
| `docs/red-first/SPEC-647.md` | #952 | added |
| `docs/specs/SPEC-025-api-service-shell.md` | #952 | changed: section 13 inserted, insert-only |
| `changelog.d/mutation-rows-sigint-647.md` | #952 | added |

## 5. What this does NOT cover

- A descendant that leaves the killer's group is still not reached, and a SIGKILL of the runner
  still runs no cleanup: SPEC-025 section 10's limits stand (#409).
- SIGQUIT and every other signal keep their defaults; only SIGINT joins SIGTERM and SIGHUP (#952).
- No workflow changes: the jobs that run the runner call it as before (#952).
- The Python mutation legs' price for this file grows by the new tests' cost; refreshing a price
  row whose suite grew is the stale-price reading's subject (#873).

## 6. Risks

- **An interrupt during a killer's start now waits for the start.** The run ends once
  `subprocess.Popen` returns and the hold is released, so the wait is bounded by the start itself;
  A1 drives that window and reads the run ended by SIGINT.
- **A caller's own SIGINT handler is silent during the run.** A program that calls `main()` with
  its own SIGINT handler no longer hears an interrupt while the run lasts, as it already did not
  for SIGTERM, and gets its handler back on every way out (A4).
- **A process test runs slow on a loaded runner.** Every wait is bounded on a marker the other
  side writes, so a slow machine waits longer, never races; a run that misses a bound fails by
  name, with every pid it started ended by number.
- **The census modules read the runner.** The missing-tool census admits only the runner's current
  imports and refuses a spawn outside its two helpers; the cure adds neither.

## 7. Formal, rows and mutation

**Formal:** not required (ADR-670 D5). The latch gains one signal and one ending, and every new
edge is a criterion and a row; the span of `retirement_records` the Lean entry covers is
byte-identical.

**Rows:** all in `scripts/mutation-rows.d/S64700-S64799.json`, `SCRIPT_MUTATIONS`, target
`scripts/mutation_rows.py`, each killer a `test_mutation_rows.<Class>.<test>` that selects one test.

| row | mutant | killer |
|---|---|---|
| S64700 | SIGINT's installation of the run's handler removed | A4 |
| S64701 | SIGINT installed over an ignored SIGINT | A4 |
| S64702 | the handler's interrupt ending outside a start removed, so a SIGINT exits with a status | A4 |
| S64703 | `released`'s interrupt ending of a held SIGINT removed | A1 |
| S64704 | the interrupt ending of a SIGINT held at a failed start removed | A5 |
| S64705 | SIGINT dropped from the handlers the run saves | A4 |

**Mutation:** the pull request's own Python mutation legs judge the diff's mutants of
`scripts/mutation_rows.py`.

## 8. References

#952, #432, #421 and #409; ADR-670, ADR-482 and ADR-196; SPEC-465 and SPEC-025.
