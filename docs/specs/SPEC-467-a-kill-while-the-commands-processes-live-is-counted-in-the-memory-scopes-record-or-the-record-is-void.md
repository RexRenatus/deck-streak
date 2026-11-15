# SPEC-467: a kill while the command's processes live is counted in the memory scope's record, or the record is void

- **Issue:** #469 (refs #439, #440, #468). **Context(s):** `repo` (`scripts/`, `formal/`, their tests and `docs/`).
- **Decided by:** ADR-484 (this SPEC's own: the counts after the command are read once the scope holds the script's own
  process alone, a TLA+ model holds the reads, and what each was chosen against); SPEC-196 and ADR-199 (the scope, its
  record and the verdict's reading). SPEC-196 takes an insert-only amendment, its section 8.
- **Status:** built. It holds docs/red-first/SPEC-467.md.

## 1. The problem, measured

Every fact below was read at dev `36c4783a`.

- **Nothing reads which processes the scope still holds once the command returns.**
  `grep -c -E 'cgroup\.procs|cgroup\.events|populated' scripts/memory_scope.py` reads 0.
- **`run()` waits only for its own child.** `subprocess.run(command, check=False)` (`scripts/memory_scope.py:301`) returns
  once the command itself has exited; a process the command started is never waited for, counted or listed.
- **The counts are two separate reads, taken at once.** `counts_after` (`:162-178`, called at `:302`) reads
  `memory.events` through `events_of` (`:169`, the file read at `:142`) and only then `memory.peak` through `peak_of`
  (`:175`, the file read at `:156`).
- **The record is written once, from those reads** (`:313`, its fields at `:304-312`). The scope holds the script's own
  process (`:75-78`), so it is never empty while the script reads.
- **The verdict reads a record in force with no kill as a clean run.** `memory_cap` reads the leg as untouched when
  `oom` and `oom_kill` are both 0 (`scripts/mutation-verdict.py:1946-1948`); `read_scope_record` (`:1866-1885`) voids
  only a record that is missing, unreadable, not `done` or not in force. Its callers are the pull request's Rust shards
  (`:1782`), the weekly battery (`:2574`) and the release table (`:3418`).
- **So a kill can escape.** A process the command started can outlive it, and the kernel can stop it at the cap between
  the two reads, or after both. Such a kill is neither counted in the record nor makes the record void, and the leg
  reads as a clean run (#469).
- **No workflow is known to leave a process behind.** `grep -c -i -E 'sccache|RUSTC_WRAPPER|nohup|setsid'` reads 0
  over `.github/workflows/ci.yml` and over `.github/workflows/mutation-weekly.yml`; a test that spawns a process that
  outlives cargo-mutants is not ruled out.
- **This pull request's own CI runs no real cargo-mutants leg unless Rust changes.** `mutation-rust`
  (`.github/workflows/ci.yml:461`, its scope call at `:533`) runs over changed Rust only, and `rehearsal`
  (`.github/workflows/mutation-weekly.yml:399`) runs only for a pull request that changes that file (`:21`).

## 2. Requirements

- **R1.** After the command returns, the script reads its scope's `cgroup.procs` at most `reads` times (the bound
  `run()` already takes, 50 by default), pausing with `time.sleep(0.1)` between two reads and never after the last, and
  stops at the first read that lists its own process alone. No parameter, option or seam is added:
  `test_the_seams_default_to_this_machine` (`scripts/tests/test_memory_scope.py:776-796`) pins `run()`'s defaults.
- **R2.** Only then does it read `memory.events` and then `memory.peak`, and write the record exactly as SPEC-196 R4
  and R13 say. A run whose processes all ended with its command reads `cgroup.procs` once, sleeps nothing, and records,
  prints and returns byte for byte what it did before.
- **R3.** When the bound passes with another process still listed, the record is written with `in_force` false,
  `state` `done`, the reason `cgroup.procs holds <n> other process(es) after the command` and zero counts; the script
  prints `memory-scope: NOT IN FORCE after the command: <reason>`, and its exit status is still the command's.
- **R4.** `cgroup.procs` is whole only when it holds one or more lines, each a process ID of one to twenty ASCII
  decimal digits (as `counter_of` reads a counter), with no ID twice. A file that is absent, a directory, empty, not
  UTF-8, written with CRLF, ending in a blank line, or holding a non-ASCII digit, a sign, a word or an ID twice is not
  whole: it is read once, with no pause, and writes R3's record with the reason
  `cgroup.procs is not whole after the command`.
- **R5.** The verdict is unchanged: `read_scope_record` already voids a record that is not in force, by name, so
  `scripts/mutation-verdict.py` does not change.
- **R6.** A TLA+ model, `formal/tla/LateKillCounted`, holds the reads (section 8). R6 is decided by the formal check,
  each witness caught and each property clean in the main configuration, not by a unittest, so it has no line in the
  acceptance fence.
- **R7.** Rows S46700 to S46710 hold each new production line (section 7), each killed by a criterion's test and read
  in CI by id.

## 3. Acceptance criteria of SPEC-467

Each criterion is a test of the new class `TheLateKill(ScriptCase)` in `scripts/tests/test_memory_scope.py`, read in CI
by name; the module never runs on a workstation. A1 to A5 are RED-FIRST: each is red at the base for its criterion's
reason, an assertion on the record or on the reads of `cgroup.procs`. A6 is MUTATION COVERAGE: green at the base, it
pins the clean run. Every expected value is a literal. A4 and A5 print how many cases they examined, after each case's
assertions, and refuse zero.

| A | criterion | decided by |
|---|---|---|
| A1 | the planted command leaves another process listed (it rewrites `cgroup.procs` to the script's own process and `424242`), and `events_of` is wrapped to read the real file and then plant a kill (`oom` 1, `oom_kill` 1, `max` 1) right after that read: the whole record is `{"in_force": false, "state": "done", "reason": "cgroup.procs holds 1 other process(es) after the command", "oom": 0, "oom_kill": 0, "max": 0, "peak_percent": 0}` and the exit status is the command's | `test_memory_scope.py` `a_kill_between_the_two_reads_while_a_process_lives_is_counted_or_the_record_is_void` |
| A2 | the same plant, with `write_record` wrapped to plant a kill right after the real `done` write: the whole record is A1's and the exit status is the command's | `test_memory_scope.py` `a_kill_after_both_reads_while_a_process_lives_is_counted_or_the_record_is_void` |
| A3 | the planted command leaves another process listed; the patched sleep, at its first call after the command, rewrites `cgroup.procs` to the script's own process alone, `memory.events` to `oom` 1, `oom_kill` 1, `max` 2 and `memory.peak` to half the cap: the record is in force with `oom` 1, `oom_kill` 1, `max` 2 and `peak_percent` 50, the sleeps read `[0.1]`, and the printed line is `memory-scope: peak 50% of the cap; the kernel stopped 1 process(es) at the cap` | `test_memory_scope.py` `a_kill_during_the_wait_is_counted_once_the_scope_holds_the_script_alone` |
| A4 | with the reads of `cgroup.procs` counted (`read_raw` wrapped, its calls on that name counted), over the cases (the read at which the script is alone, `reads`, the reads expected, the sleeps expected, `in_force`) (1, 3, 1, 0, true), (2, 3, 2, 1, true), (3, 3, 3, 2, true), (4, 3, 3, 2, false) and (2, 1, 1, 0, false): each case's read count, its sleeps as a literal list of `0.1` pauses and its record's `in_force` equal the case's | `test_memory_scope.py` `the_scope_is_read_after_the_command_at_most_reads_times_and_no_longer` |
| A5 | over the R4 shapes that are not whole (absent, a directory, empty, bytes that are not UTF-8, CRLF, a trailing blank line, a non-ASCII digit, a sign, a word, an ID twice), each is read once, nothing sleeps, and the whole record is R3's with the reason `cgroup.procs is not whole after the command`; two whole shapes are the controls: the script's own process alone reads in force, and the script beside `424242` with `reads` 1 reads `cgroup.procs holds 1 other process(es) after the command` | `test_memory_scope.py` `a_process_list_that_is_not_whole_after_the_command_voids_the_record` |
| A6 | the planted command leaves the script alone in `cgroup.procs`, with the plant's AFTER counts (`scripts/tests/test_memory_scope.py:175`): the record and the printed line equal their literals, and the sleeps read `[]` | `test_memory_scope.py` `a_run_whose_processes_end_with_it_records_what_it_recorded_before` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k a_kill_between_the_two_reads_while_a_process_lives_is_counted_or_the_record_is_void
A2: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k a_kill_after_both_reads_while_a_process_lives_is_counted_or_the_record_is_void
A3: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k a_kill_during_the_wait_is_counted_once_the_scope_holds_the_script_alone
A4: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k the_scope_is_read_after_the_command_at_most_reads_times_and_no_longer
A5: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k a_process_list_that_is_not_whole_after_the_command_voids_the_record
A6: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k a_run_whose_processes_end_with_it_records_what_it_recorded_before
```

The plant changes in the red commit, with no assertion changed: `Plant.files` (`scripts/tests/test_memory_scope.py:106-112`)
gains `"cgroup.procs": f"{self.pid}\n"`, and the stand-in machine's wrapper plant in `scripts/tests/test_dispatch_shards.py`
(`WRAPPER_PLANT`, `:1117-1154`) gains the same file listing `os.getpid()`, so every planted run whose command leaves
nothing behind stays in force. No `getattr`, `importlib`, `inspect`, `compile`, `exec` or `eval` site is added to either
module (the `DYNAMIC_IMPORTS` register of `scripts/tests/test_ci_workflows.py` at `:7172`, its `test_memory_scope` entries at
`:7221-7232` and `:7462-7468`), so that file stays outside the manifest.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/memory_scope.py` | 1 | changed: R1 to R4, the reads of the scope's process list after the command (two new functions and one call in run) |
| `scripts/tests/test_memory_scope.py` | 1 | changed: the plant's process-list file, and the class TheLateKill with A1 to A6 |
| `scripts/tests/test_dispatch_shards.py` | 1 | changed: the stand-in machine's wrapper plant writes the process-list file with its own process; no assertion changes, and none of its assertions reads the record |
| `scripts/mutation-rows.d/S46700-S46799.json` | 1 | added: rows S46700 to S46710 |
| `formal/tla/LateKillCounted/LateKillCounted.tla` | 1 | added: the model |
| `formal/tla/LateKillCounted/MCLateKillCounted.cfg` | 1 | added: the main configuration, the cure |
| `formal/tla/LateKillCounted/witness/EveryKillIsCountedOrTheRecordIsVoid.cfg` | 1 | added: a witness, the base's order |
| `formal/tla/LateKillCounted/witness/NoKillIsReadAsACleanRun.cfg` | 1 | added: a witness, the base's order |
| `formal/tla/LateKillCounted/witness/AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid.cfg` | 1 | added: a witness, the base's order |
| `formal/tla/LateKillCounted/witness/AKillAfterBothReadsIsCountedOrTheRecordIsVoid.cfg` | 1 | added: a witness, the base's order |
| `docs/specs/SPEC-467-a-kill-while-the-commands-processes-live-is-counted-in-the-memory-scopes-record-or-the-record-is-void.md` | 1 | added: this SPEC |
| `docs/decisions/ADR-484-the-memory-scope-reads-its-counts-once-it-holds-only-its-own-process-and-a-tla-model-holds-the-reads.md` | 1 | added: its decision |
| `docs/specs/SPEC-196-a-runaway-mutant-is-stopped-inside-the-legs-memory-scope.md` | 1 | changed: section 8 appended after its last line, insert-only |
| `docs/schematics/memory-scope-reads-after-the-command.md` | 1 | added: the reads after the command, the model's actions, the tests and the witnesses |
| `docs/red-first/SPEC-467.md` | 1 | added: the reds and greens CI read |
| `changelog.d/memory-scope-late-kill-467.md` | 1 | added: the changelog fragment |

## 5. What this does NOT cover

- The reads before the command and the checks of `check_in_force`, which stand as SPEC-196 states them (#439).
- The scope's other limits and readings: the cap, swap, `memory.oom.group` and the out-of-memory policy (#439).
- The verdict's reading of the record, `read_scope_record` and `memory_cap`, which does not change (#439).
- The mutation legs, the workflows that call the scope, and their timeouts (#440).
- An `oom` event with no kill, SPEC-196's own risk (#439).
- A CI job that runs the formal check on every pull request: DeckStreak's CI has none, so the entry is read by its
  builder's formal check (#806).

## 6. Risks

- **Legs whose processes outlive the wait's bound now read VOID by name.** Only a later CI run measures how often a
  real leg does; the verdict's VOID line names the reason when one does (#953).
- **The cure rests on the kernel's order.** It assumes the kernel raises `oom_kill` no later than its victim leaves
  `cgroup.procs`, which this tree does not measure. A kernel that did otherwise could let the wait see the script
  alone with a kill not yet counted; such a reading would most likely show `oom` above `oom_kill`, which the verdict
  fails as `MEMORY-CAP AMBIGUOUS` (#439).
- **A process the kernel still lists after it ended** (a zombie) holds the wait or voids the record; it never reads as
  a clean run (#953).
- **The stand-in plant now writes `cgroup.procs`.** A stand-in without it would read its runs as not in force, which
  none of its assertions reads; the plant line keeps its runs as they were (#440).
- **Python mutants of the new lines are read in this pull request's `mutation-python` legs**
  (`scripts/mutation-python.json:30-35` maps `scripts/memory_scope.py` to `test_memory_scope`); a survivor there fails
  the leg by name, beside rows S46700 to S46710 (#469).

## 7. Rows

Rows S46700 to S46710, in `scripts/mutation-rows.d/S46700-S46799.json`, each killed by `test_memory_scope.TheLateKill.<its
criterion's test>`. Each find is written from the landed line, occurs exactly once in `scripts/memory_scope.py`, and
carries no marker word.

| row | the landed code it mutates | its mutant | killer |
|---|---|---|---|
| S46700 | `run()`'s call of `alone_after` after the command | the call's result replaced by None | A1 |
| S46701 | the return of the other-process reason once the bound passes | returns None | A2 |
| S46702 | the test that the scope lists no other process (`others == 0`) | `others <= 1` | A1 |
| S46703 | the return of the not-whole reason | returns None | A5 |
| S46704 | the `0.1` pause between two reads | a pause of `0` | A4 |
| S46705 | the bound on the reads (`reads` of them) | one read | A3 |
| S46706 | the script's own process left out of the count (`- {pid}`) | not left out | A6 |
| S46707 | the refusal of an ID listed twice | removed | A5 |
| S46708 | the refusal of a line that is not an ID | removed | A5 |
| S46709 | the refusal of an empty list (`not lines`) | `lines is None` | A5 |
| S46710 | the guard that reads the counts only when the wait returned None | the counts read always | A1 |

The 17 existing rows on `scripts/memory_scope.py` (S19600 to S19614, S19631 and S19632, in
`scripts/mutation-rows.d/S19600-S19699.json`) keep their finds: none is duplicated or moved by the new lines. S42012 and
S42125, on `scripts/mutation-verdict.py`, are untouched.

## 8. The model

`formal/tla/LateKillCounted` holds the reads, as ADR-484 D2 decides.

- **State.** `live`, the command's processes alive in the scope, from `begun`, the command and every process it or a
  process it started has started; `wrapper`, the script's own process, live or killed; `pc`, the script's place in the
  code's order (`command`, `wait`, `events`, `peak`, `write`, `written`); `tries`, the reads of `cgroup.procs` taken;
  `counter`, the kernel's `oom_kill`; `seen`, the count the script read; `voided`; `record` (`state`, `inForce`,
  `kills`); and three history variables, `owed`, `between` and `after`.
- **Actions.** `Start(p)` and `End(q)` for the command's processes, `End` of the command being its exit; `Kill(v)`, the
  kernel's out-of-memory killer, which stops any live process, the script's own included, and raises `counter` in the
  same step; `Return`, `run()` returning from `subprocess.run`; `Wait`, one read of `cgroup.procs`; `ReadEvents`,
  `ReadPeak` and `Write`, in the code's order; and `Finished`, a stutter enabled only once no process of the command
  lives and the script has written its record or was killed.
- **The switch.** `ReadBeforeAlone = TRUE` is the base's `run()`, where `Return` goes straight to `ReadEvents`; `FALSE`
  is the cure. The main configuration models the cure; each witness is that configuration with the switch on and
  nothing else changed.
- **The reader.** `Verdict` is `read_scope_record` and `memory_cap`: void when the record is not `done` or not in force,
  clean when its kill count is 0, counted otherwise.
- **Properties,** each entering at `ramp=report`: `EveryKillIsCountedOrTheRecordIsVoid`, `NoKillIsReadAsACleanRun`,
  `AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid` and `AKillAfterBothReadsIsCountedOrTheRecordIsVoid`. The last two
  split the first by the windows #469 names, so that each window has a witness of its own.
- **Bounds and budget.** Two processes the command may start and two reads of `cgroup.procs`, under the default entry
  budget of `config/formal.json`, with no row of its own.
- **Witnesses,** one per property, each violating its own property with the switch on, among them a kill between the
  two reads and a kill after both reads while a process the command started still lives.
- **Covers.** `scripts/memory_scope.py`'s `run`, `counts_after`, `alone_after` and `others_after`, and
  `scripts/mutation-verdict.py`'s `read_scope_record` and `memory_cap`, each stamped with its span's digest after the
  last edit.

## 9. References

Issue #469; #439, SPEC-196 and ADR-199 (the scope, its record and the verdict's reading); #440 (the legs and the
workflows that call the scope); #468, SPEC-295 and ADR-295 (the formal-check settings); ADR-484; #806 (a CI job for the
formal check); #953 (a real leg measured against the wait); the kernel's control-group documentation (`cgroup.procs`,
`memory.events`, `memory.peak`); systemd's `systemd.scope(5)`.
