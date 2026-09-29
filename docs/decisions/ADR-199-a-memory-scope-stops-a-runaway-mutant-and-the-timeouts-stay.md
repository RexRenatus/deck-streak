---
status: proposed
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A memory scope around the mutants step stops a runaway mutant, the leg fails naming it, and the timeouts stay

## Context and Problem Statement

A `mutation-rust` leg is lost when a mutant turns a loop that allocates into one that never ends: the test grows until
GitHub's runner is shut down, cargo-mutants' test timeout never fires, the shard's report is never written, and the verdict
reads the shard as never tested (VOID), naming no culprit and discarding every other mutant the leg had tested. The same
shard stops at the same mutant in every run (SPEC-196 §1). How do we stop the runaway test inside our own process tree,
name the mutant, and keep every other result, without examining fewer mutants, changing a timeout, skipping a baseline, or
making any verdict more permissive than it is today?

## Decision Drivers

- The examined set is the owner's: fewer tests or mutants, a raised or lowered timeout, a skipped baseline, or a narrowed
  diff is a weakening, and a lever that needs one is not taken.
- A new bound must not move any verdict in the permissive direction. A mutant whose tests pass while using memory between
  a cap and the machine's own limit is missed today; a cap may make it fail, never pass.
- The runaway must be stopped by the kernel inside our own process tree, before the machine is under memory pressure, not
  by whatever the host does once it is.
- Attribution must be proved, not guessed: a kill the evidence does not place on one mutant fails the leg.
- A guard that did not apply must fail visibly, never run unguarded while reported as guarded.
- The cap must not drift: nobody raises it from a workflow input, a variable or an option.

## Considered Options (the alternatives it was chosen against)

- A transient systemd scope that holds the wrapper's own process, `MemoryMax` at 15/16 of the machine, `MemorySwapMax=0` and `OOMPolicy=continue`, with a cap kill a named per-mutant failure that the scope's `oom_kill` count and the scenario logs must agree on — chosen because the kernel stops the one largest process inside our tree before the machine swaps, cargo-mutants lives on to test the next mutant, the leg fails deterministically naming the culprit, every other result stands, and nothing that decides the examined set changes.
- Scoring a mutant the cap stopped as caught (the test failed, as cargo-mutants reads it) — rejected because it moves a verdict in the permissive direction: a mutant whose tests pass using memory between the cap and the machine's limit is missed today and would pass, a weakening that needs the owner's signed ruling.
- Scoring a mutant the cap stopped like a timeout — rejected because it is the owner's decision and is deferred to them (#439): it too passes a mutant that is missed today, so the design keeps it one function in the verdict, `score_memory_cap`, whose body alone the owner's line would change.
- Leaving the shard VOID when the cap stops a test (no attribution) — rejected because a VOID names no culprit and discards every other mutant's result in the leg, where a named failure keeps them and fails the leg as surely.
- Failing the leg's own job from the wrapper, by a non-zero exit when the cap stopped a process — rejected because the step records the wrapper's exit as cargo-mutants' own, and the verdict reads that exit to decide whether the report is whole; a changed exit would VOID the whole shard and discard every other result, so the verdict fails the leg instead.
- `sudo systemd-run --scope --uid --gid` running the command — rejected because the command then starts under `sudo`'s environment (its `secure_path` replaces `PATH`, its delete list drops variables, `SUDO_*` variables are added) and with root's supplementary groups, since `systemd-run` changes the user and group without initialising the groups, so the tests would not run as they run today; placing the wrapper's own process in the scope keeps its identity whole.
- `systemd-run --user --scope` — rejected because it depends on a per-user manager for the runner's user and on memory delegation to it, two more runner facts to hold, where the system manager is always there and one `sudo` call reaches it.
- `OOMPolicy=stop` (systemd's default) — rejected because systemd would stop the whole scope on the first kill, cargo-mutants with it, so the shard would end with no report and read VOID.
- `OOMPolicy=kill`, or `memory.oom.group` set to 1 — rejected because the kernel would then kill every process in the scope together, cargo-mutants included.
- A scope per test process, through a cargo target runner — rejected because it changes how every test binary is started and adds a variable every test can see, so the tests would not run as they run today, and it costs a privileged call for every test process of every mutant.
- Attribution from the scope's `oom_kill` count alone, blaming the mutant under test — rejected because a counter has no time and cargo-mutants moves on, so a count cannot tell which scenario was running when the kill came.
- Attribution from the logs alone, a test ended by `SIGKILL` — rejected because a test can end by `SIGKILL` for other reasons (a test that signals itself, a host's own out-of-memory daemon, a runner's cleanup), so a log line is not proof the cap stopped it; the two witnesses must agree, and any disagreement fails the leg.
- Attribution from the kernel log — rejected because it names a process ID and a command, not a mutant, reading it needs privilege, and the host's log holds other processes' kills.
- A per-leg proof scope that allocates past a small cap before the command — rejected because the self-check reads the kernel's own files for the cap in force, and the delivery's plant proves the whole path on real CI once; a proof on every leg adds a second scope and a second privileged call, and a new way to VOID a leg, for no strictness gained.
- Reading a shard with no record as a shard judged before this change — rejected because a record lost while the report survived would let a cap kill read as a caught mutant; a shard with a report and no finished record is VOID by name.
- `oom_score_adj` raised on the mutants step — rejected because the runner already lowers its job processes' out-of-memory priority and is still lost, so the kernel's choice of victim is not what fails.
- An address-space limit (`ulimit -v`) on the step or on each test — rejected because it bounds reserved address space, not memory: rustc, the linker and threaded tests reserve far more than they touch, so builds could fail and mutants turn unviable, which examines fewer, and an allocation it refuses fails a test with no witness.
- A lower `--timeout`, or a nextest `terminate-after` in a mutants profile — rejected because a lowered timeout is a weakening by the owner's rule, and a loop that only allocates exhausts memory before any timeout that ordinary tests can live with.
- Skipping the runaway mutants (`mutants::skip`, an `exclude_re`, or an equivalent-list entry) — rejected because it examines fewer mutants, a weakening.
- A larger swap file on the runner — rejected because it only moves the point of exhaustion: a loop that only allocates fills any swap, and the machine thrashes on the way.
- A larger runner — rejected because it is spend, the maintainer's decision, and a loop that only allocates exhausts any size.
- Rerunning a lost leg automatically — rejected because the same shard stops at the same mutant in every run, so a rerun repeats the loss and its cost.
- A cap read from a workflow input, an environment variable or an option — rejected because a cap anyone can raise is a bound that drifts; the script reads it from `/proc/meminfo` and takes no option for it.

## Decision Outcome

Chosen option: "a transient systemd scope that holds the wrapper's own process, `MemoryMax` at 15/16 of the machine,
`MemorySwapMax=0` and `OOMPolicy=continue`, with a cap kill a named per-mutant failure", because it stops the runaway inside
our own tree before the machine is under pressure, it fails the leg by name where today the leg is lost with no name, every
other mutant's result in the leg stands, and the listing, the partition, both timeouts, the baseline and cargo-mutants'
arguments stay byte for byte what they were.

`scripts/memory_scope.py` computes the cap from `/proc/meminfo`, asks the system manager (one `sudo busctl` call of
`StartTransientUnit`, the call `systemd-run --scope` makes for its own process) for a scope that holds its own process,
checks from inside that the unit took it and that `memory.max`, `memory.swap.max`, `memory.oom.group` and the unit's
`OOMPolicy` are the ones asked for, writes a record, runs the command as its child with its own identity, finishes the record
with the scope's `oom`, `oom_kill` and `max` counts and the peak as a share of the cap, and passes the command's exit through.
The kernel's out-of-memory killer, invoked in the scope when its usage reaches `memory.max` and cannot be reduced, picks the
process with the most memory, which is the runaway test; nextest reports that test's status as `SIGKILL` and the run as
failed, and cargo-mutants, a small separate process that the scope keeps running, records the outcome and tests the next
mutant.

`scripts/mutation-verdict.py` reads each promised shard's record. A shard the cap never touched is judged exactly as before.
When the cap touched it, the leg fails, always; the verdict names a mutant only when the kills it places in the scenario logs
(distinct tests whose nextest status is `SIGKILL`) equal the scope's `oom_kill` count, and otherwise fails the leg as
ambiguous. A named mutant is scored through one function, `score_memory_cap`, which fails it as `MEMORY-CAP` and leaves it
out of the examined count; every other outcome in the shard is judged as before.

### Consequences

- Good, because a leg that is lost today finishes: its shard is examined but for the named mutants, and its run gets a
  verdict that names the culprit.
- Good, because no verdict moves in the permissive direction: a mutant the cap stops fails its leg, where today it is
  missed or VOID, and a shard the cap never touched is judged byte for byte as before.
- Good, because the tests run with the caller's identity and environment, since only the wrapper's control group changes.
- Good, because a scope that did not apply refuses before cargo-mutants runs, and the verdict reads its record VOID by name.
- Good, because the owner's later decision to score a cap kill like a timeout is one function's body.
- Bad, because a pull request whose diff holds a runaway mutant still cannot pass until its code changes or the owner
  decides; it fails by name instead of by a lost runner.
- Bad, because a kill the two witnesses disagree on fails the leg even when the mutant was caught by other tests; the leg
  names both counts, so the case is read, never guessed.
- Bad, because the step now depends on the runner image's `sudo`, `busctl`, unified control groups and swap accounting; a
  change there shows as a VOID shard, not as an unguarded pass.
- Bad, because a scheduled weekly run between the merge and the release that carries the workflow change reads its shards
  VOID, since it takes the workflow from `main` and the scripts from `dev`.

### Confirmation

SPEC-196's A1 to A17 (`scripts/tests/test_memory_scope.py` and `scripts/tests/test_memory_cap_verdict.py`), each written red
first; the hand-proved rows of band S19600-S19699, one per guard arm; the delivery's plant commit, whose `mutation-verdict`
job fails with one finding naming the plant's runaway mutant and no other; and the delivery's own `rehearsal` job, which runs
cargo-mutants inside the scope and prints the examined count, equal to dev's for the same file.

## More Information

Issue #439; SPEC-196; SPEC-039 and ADR-057 (cargo-mutants in place, in shards, with the gate's bounds); SPEC-038 (the CI
jobs); the Linux control-group v2 documentation (`memory.max`: the out-of-memory killer is invoked in the group when its usage
reaches the limit and cannot be reduced; `memory.oom.group`, default `0`; `memory.events` `oom` and `oom_kill`); the kernel's
`/proc` documentation (the badness heuristic: the share of the allowed memory a process uses, the allowed memory being the
group's limit); systemd's `systemd.service(5)` (`OOMPolicy=`: `continue` keeps the unit running, `kill` sets
`memory.oom.group`), `systemd.scope(5)`, `systemd-system.conf(5)` (`DefaultOOMPolicy=`, `stop`) and `systemd-run(1)`;
cargo-mutants' timeouts chapter and its outcome summaries; cargo-nextest's status lines; GitHub's hosted-runner documentation
(passwordless `sudo`) and the runner project's guidance on the shutdown signal (actions/runner #2662).
