---
status: accepted
decision-makers: "the DeckStreak architect"
---

# The memory scope reads its counts once it holds only its own process, and a TLA+ model holds the reads (ADR-484)

## Context and Problem Statement

`scripts/memory_scope.py` runs one mutants command inside a transient scope with a memory cap, and
records what the kernel did there (SPEC-196, ADR-199, #439). Once the command returns, it reads the
scope's kill counters in `memory.events` and its peak in `memory.peak`, two separate reads, and
writes the record the mutation verdict reads. A process the command started can outlive the
command, and the kernel can stop it at the cap after those reads, or between them. Such a kill is
neither counted in the record nor makes the record void, so a run the cap touched can read as a
clean run (#469).

The property #469 asks for, in its own words: every out-of-memory kill the kernel records while any
of the command's processes lives is counted in the record, or the record is void; it is never read
as a clean run. #469 owes a TLA+ model of it, now that the repository's formal-check settings are on
the development branch (#468, SPEC-295). Six decisions follow. Every `path:line` below was read at
dev `36c4783a`.

## Decision Drivers

- A count the scope did not measure must never read as a clean run (SPEC-196 R13 states the same
  rule for a count that is absent or malformed).
- A run whose processes all end with its command must record, print and return exactly what it does
  today, so no verdict moves for it.
- No existing refusal or void is weakened, and no examined count changes.
- The change stays inside the scope's own file and its tests, beside a model of the reads.

## D1. The mechanism, as measured

- The scope holds the script's own process: `scope_call` asks the manager for a unit whose `PIDs`
  is the script's process ID (`scripts/memory_scope.py:75-78`, called at `:276-282`). The command
  runs as the script's child inside it (`:301`).
- `run()` waits only for that child: `subprocess.run(command, check=False)` returns once the command
  itself has exited (`:301`). Nothing waits for, counts or lists the processes it started:
  `grep -c -E 'cgroup\.procs|cgroup\.events|populated' scripts/memory_scope.py` reads 0.
- At once it calls `counts_after(here, cap)` (`:302`), which reads `memory.events` through
  `events_of` (`:169`, the file read at `:142`) and requires its `oom`, `oom_kill` and `max` keys
  (`:172-174`), and only then reads `memory.peak` through `peak_of` (`:175`, read at `:156`). The two
  reads are separate file reads, events first.
- The record is written whole, once, at `:313`, from those two reads (`:304-312`). It is not in
  force only when a read is not whole (`:170-177`, SPEC-196 R13); before the command, `refuse`
  writes a void record (`:238-252`) when the scope is not in force (`check_in_force`, `:201-235`,
  which also requires `oom` and `oom_kill` to start at 0, `:226-227`).
- The record's kill count is read by `scripts/mutation-verdict.py`: `read_scope_record`
  (`:1866-1885`) voids a record that is missing, unreadable, not `done` or not in force, by name;
  `memory_cap` (`:1930-1969`) reads `oom` and `oom_kill` (`:1946`) and reads the leg as untouched,
  a clean run, when both are 0 (`:1947-1948`); otherwise it places the kills in the scenario logs or
  fails the leg as `MEMORY-CAP AMBIGUOUS` (`:1953`). Its callers are the pull request's Rust shards
  (`:1782`), the weekly battery (`:2574`) and the release table (`:3418`).
- So the order is: the command exits, the counters are read, the peak is read, the record is
  written, and the script exits, while any process the command started runs on in the scope.
- Not measured: whether any CI leg leaves a process alive after cargo-mutants exits. No workflow
  starts a background process for the leg (`grep -c -i -E 'sccache|RUSTC_WRAPPER|nohup|setsid'`
  reads 0 over `.github/workflows/ci.yml` and over `.github/workflows/mutation-weekly.yml`), and a
  test that spawns one is not ruled out. Also not measured from this tree: that the kernel raises
  `oom_kill` no later than its victim leaves `cgroup.procs`; D2 and D3 rest on it.

## D1 chosen against

- The scope becoming empty as the end of the command's processes: rejected because the scope holds the script itself, so it is never empty while the script reads (`:75-78`).
- The command's exit as the end of every process it started: rejected because `subprocess.run` returns when its own child exits, and a grandchild is never waited for (`:301`).

## D2. The model

A new entry, `formal/tla/LateKillCounted`, holds the reads.

- **State.** `live`, the command's processes alive in the scope (the command and each process it
  or a process it started started, `begun`); `wrapper`, the script's own process, live or killed;
  `pc`, the script's place in the code's order (`command`, `wait`, `events`, `peak`, `write`,
  `written`); `tries`, the reads of `cgroup.procs` taken; `counter`, the kernel's `oom_kill`;
  `seen`, the count the script read; `voided`; `record` (`state`, `inForce`, `kills`); and three
  history variables: `owed`, the counter's value at the last kill the kernel recorded while any of
  the command's processes lived, and `between` and `after`, the kills recorded while one lived
  after the counters' read and before the peak's read, and after both.
- **Actions.** `Start(p)`: a live process of the command's tree starts `p`. `End(q)`: a process
  ends by itself; for the command this is its exit. `Kill(v)`: the kernel's out-of-memory killer
  stops any live process, the script's own included, and raises `counter` in the same step (D1's
  unmeasured kernel order). `Return`: `run()` returns from `subprocess.run` once the command is
  gone. `Wait`: one read of `cgroup.procs`, as the cure reads it. `ReadEvents`, `ReadPeak` and
  `Write`, in the code's order. `Finished` stutters only when no process of the command lives and
  the script has written its record or was killed.
- **The switch.** `ReadBeforeAlone = TRUE` is dev's `run()`: `Return` goes straight to
  `ReadEvents`. `FALSE` is the cure of D3. The `MC` configuration models the cure; each witness is
  the same configuration with the switch on and nothing else changed.
- **The reader.** `Verdict` is `read_scope_record` and `memory_cap`: void when the record is not
  `done` or not in force, clean when its kill count is 0, counted otherwise.
- **Properties,** each at `ramp=report`, each in the issue's words:
  `EveryKillIsCountedOrTheRecordIsVoid` (a record that is not void counts at least every kill
  recorded while a process of the command lived: `record.kills >= owed`);
  `NoKillIsReadAsACleanRun` (a clean verdict means no such kill: `owed = 0`);
  `AKillBetweenTheTwoReadsIsCountedOrTheRecordIsVoid` (`between = 0` unless void); and
  `AKillAfterBothReadsIsCountedOrTheRecordIsVoid` (`after = 0` unless void). The last two split the
  first by the window #469 names, so that each window has a witness of its own.
- **Bounds and budget.** Two processes the command may start, two reads of `cgroup.procs`; the
  default entry budget of `config/formal.json`, with no row of its own.
- **Witnesses,** one per property, each violating its own property with the switch on: a kill
  between the two reads while a process the command started still lives, and a kill after both
  reads while one still lives, among them.
- **Covers.** `scripts/memory_scope.py` anchors `run`, `counts_after`, `alone_after` and
  `others_after`, and `scripts/mutation-verdict.py` anchors `read_scope_record` and `memory_cap`,
  each stamped with its span's digest after the last edit.

## D2 chosen against

- One counter of the command's other processes in place of named processes: rejected because a start, an exit and a kill each touch one process, and an action over all actors needs a scope of at least two to show it.
- One atomic read of both files: rejected because the race #469 names lands between the two reads, which an atomic read would hide.
- A temporal property under fairness: rejected because the record is written once, so a state invariant over the record and a history variable decides the property with no fairness assumption.

## D3. Production: the code does not meet the property, and the cure

By D1's order, a process the command started can be killed after `:302`'s reads and the record still
reads in force with the kills it read; each witness of D2 violates its property on that order. So
the delivery changes production code.

The cure, in `scripts/memory_scope.py`:

- `others_after(group, pid)`: how many processes besides the script's own the scope's
  `cgroup.procs` lists, or None unless the file is whole (one or more lines, each a process ID of one
  to twenty ASCII digits, as `counter_of` reads a counter, and no ID twice).
- `alone_after(group, pid, reads)`: after the command, read `cgroup.procs` at most `reads` times,
  pausing `time.sleep(0.1)` between reads as `wait_for_scope` does, and stop at the first read that
  lists the script alone (None) or that is not whole (`cgroup.procs is not whole after the command`).
  When the bound passes with another process listed, the reason is
  `cgroup.procs holds <n> other process(es) after the command`.
- `run()` calls it right after the command, with its existing `reads`, and reads the counts as today
  only when it returns None; otherwise the record is written not in force with that reason and zero
  counts, through the path SPEC-196 R13 already writes.

A run whose processes all end with its command reads `cgroup.procs` once, finds the script alone,
pauses nothing, and records, prints and returns exactly what it does today. The verdict already
voids a record that is not in force, by name, so `scripts/mutation-verdict.py` does not change. Every
red and green of `scripts/tests/test_memory_scope.py` is read in CI by name.

## D3 chosen against

- The model alone, with no cure: rejected because dev's order violates every property of D2, so the model would land properties that read unclean.
- Voiding at the first read that lists another process, with no wait: rejected because a process still on its way out when the command returns would void a run that the bounded wait records exactly as today.
- Re-reading `memory.events` after the peak and comparing the two counts: rejected because a kill after both reads still escapes, so the window narrows and never closes.
- Waiting with no bound for the scope to hold the script alone: rejected because a process that never ends would hold the leg to its job's timeout, a lost runner, where a void names it.
- Stopping or killing the scope's other processes before the reads: rejected because the script is in the scope, so stopping the unit stops the script, and killing changes what the command's processes do.
- Running the command in a child group and waiting for it to empty: rejected because it changes the scope's layout and its in-force checks, a larger change than the property needs.
- A new keyword seam for the wait's bound: rejected because `run()`'s defaults are pinned by `test_the_seams_default_to_this_machine`, and the existing `reads` already bounds the one other wait.

## D4. Shape and split

One delivery holds the model and the cure, under SPEC-467 and this ADR, with the rows band
S46700-S46799; the pull request that lands it closes #469. SPEC-196 is amended insert-only, a
section appended after its last line, because its R4 and R13 say when and how the counts after the
command are read. ADR-199 is not amended: its decision, a scope around the mutants step whose
record the verdict reads, stands as written, and this ADR records the new one. The schematic is a
new file, `docs/schematics/memory-scope-reads-after-the-command.md`.

## D4 chosen against

- Two deliveries, the model first and the cure after it: rejected because the model's main configuration and its covers name code only the cure adds, so a model-first part would land properties that read unclean or covers that do not resolve.
- Amending ADR-199: rejected because an appended decision that is new belongs in a new ADR, which names the options it rejected.
- An insert-only amendment of `docs/schematics/ci-jobs-and-caches.md`, the CI jobs' schematic SPEC-196 amended: rejected because three open pull requests change that file, and the reads are the scope's own sequence, not a job's.

## D5. Tests and rows

- Each criterion of SPEC-467 is a test in `scripts/tests/test_memory_scope.py`, read in CI by name.
  A1 to A5 are red at the base for their criterion's reason, an assertion on the record or on the
  reads of `cgroup.procs`; A6 pins the clean run the base already records and is not red.
- The plant gains a `cgroup.procs` file listing its own process, and the stand-in machine of
  `scripts/tests/test_dispatch_shards.py` gains the same line, so every planted run whose command
  leaves nothing behind stays in force; no assertion changes.
- The model is red first by its witnesses: each witness configuration violates its own property
  under the checker before the main configuration holds, and each run's distinct-state count is
  recorded. The model is decided by the formal check, not by a unittest, so it has no line in the
  acceptance fence.
- The entry needs no budget beyond the default, so neither `config/formal.json` nor
  `scripts/tests/test_formal_config.py` changes.
- Rows S46700 to S46710 hold each new production line, each killed by a criterion's test and read
  in CI by id.
- Three pushes, by red-first need: the red commit alone, then the cure, the entry and the rows, then
  the red-first record alone.

## D5 chosen against

- A budget row for the entry: rejected because each of its runs is a small model, so the default entry budget holds it and a row would change a pinned file for nothing.
- One push: rejected because the red tests must be read in CI at the base's code before the cure, and the record can name only greens CI has read.
- Running the scope's tests on a workstation: rejected because the house runs that module in CI only.

## D6. Not in this delivery

The reads before the command, the scope's other limits, the verdict's reading of the record, the
mutation legs and the workflows that call the scope, and an out-of-memory event with no kill, each
listed with its issue in SPEC-467 section 5.

## D6 chosen against

- Widening the delivery to the legs or the verdict: rejected because neither changes what the property needs, and each has its own issue.

## Consequences

- Good, because a kill of a process the command started is counted or voids the record: it can no
  longer read as a clean run.
- Good, because a run whose processes end with its command records, prints and returns byte for
  byte what it did, so no verdict moves for it.
- Good, because the model states the property and catches dev's order in each window.
- Bad, because a leg whose processes outlive cargo-mutants past the wait's bound now reads VOID by
  name; how often a real leg does is measured only by a later CI run.
- Bad, because the cure rests on the kernel raising `oom_kill` no later than its victim leaves
  `cgroup.procs`, which this tree does not measure.

## Confirmation

SPEC-467's A1 to A6, each read in CI by name, A1 to A5 red at the base first; rows S46700 to S46710,
each read KILLED by id; and the formal check of `formal/tla/LateKillCounted`: each witness caught,
each property clean in its main configuration.

## What would make this wrong

- A kernel that drops a victim from `cgroup.procs` before it raises `oom_kill`: the wait could then
  see the script alone with a kill still uncounted.
- A process that joins the scope from outside the command's tree after the wait.
- Legs that routinely leave a process alive past the bound: every such leg would read VOID, and the
  bound would need its own decision.

## More Information

Issue #469; #439 and SPEC-196 (the scope, its record and the verdict's reading); ADR-199; #468,
SPEC-295 and ADR-295 (the formal-check settings); the Linux control-group v2 documentation
(`cgroup.procs`, `memory.events`, `memory.peak`); systemd's `systemd.scope(5)`.
