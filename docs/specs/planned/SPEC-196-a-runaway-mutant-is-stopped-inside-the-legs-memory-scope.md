# SPEC-196: a runaway mutant is stopped inside its leg's memory scope, and the leg fails naming it

- **Wave:** W4. **Issue:** #439 (the lever's issue; its epic #1). **Context(s):** `repo` (`scripts/`, `.github/workflows/`, their tests and `docs/`).
- **Decided by:** ADR-199 (this SPEC's own: a memory scope around the mutants step, a cap kill as a named failure, and what
  each was chosen against), SPEC-039 and ADR-057 (cargo-mutants in place, in shards, with the gate's bounds), SPEC-038 (the CI
  jobs; its schematic takes an insert-only amendment).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to `docs/specs/` with its tests
  and `docs/red-first/SPEC-196.md` (ADR-016).

## 1. The problem, measured

- **A `mutation-rust` leg can be lost while a mutant's tests run, and it is lost at the same mutant in every run.** In the
  measured window, a small share of `mutation-rust` legs (all on two pull requests' runs, #386 and #403) ended with GitHub's
  "The runner has received a shutdown signal" during cargo-mutants' test phase. Mapping each lost shard's order in the plan's
  `listed.json` (round robin, `--no-shuffle`) to the last mutant its log printed puts the same shard at the same mutant in
  every run of one pull request, and a rerun lost the same legs again.
- **The verdict reads such a shard as never tested (VOID),** so the run fails with no culprit named, and every other mutant
  that leg had already tested is thrown away with it.
- **The cause is memory, inferred.** Every loss is in the test phase (`err=interrupted phase=Test`), never a build, and no
  log prints a disk error. The in-flight mutants turn a loop's progress to zero, for example `walk` in
  `crates/ingest/src/wire.rs`, which pushes a field on every turn of `while at < data.len()`, so a `varint` mutant that
  returns zero progress makes a loop that only allocates. Each loss comes before the mutant's test timeout, so
  cargo-mutants' time bound, its only defence against a runaway (its timeouts chapter), never fires. GitHub documents the
  signal as most often a runner whose resources were exceeded.
- **A memory bound is a new bound, and it must not make the gate more permissive.** A mutant whose tests pass while using
  memory between a cap and the machine's own limit is missed today. So a test the cap stops is not a failed test: it is
  neither caught nor a timeout, and the leg that ran it fails, naming it.
- **Source:** the job logs and the plan artifacts of the affected runs; `grep -n 'phase=Test' <log>`; `python3 -c` over
  `listed.json` in the plan's round-robin order.

## 2. Requirements

R1. **The cap is read from the machine and never given.** `scripts/memory_scope.py` computes the cap as the machine's
`MemTotal` (from `/proc/meminfo`) times 15/16, rounded down to whole pages of the system's page size. Its argument parser
accepts `--report <dir>` and the command after `--`, and nothing else; no option, environment variable or workflow input
sets or raises the cap. A `/proc/meminfo` with no `MemTotal` refuses as R3 does.

R2. **The scope holds the script's own process, so the command keeps the caller's identity.** The script asks the system
manager, by one `sudo --non-interactive busctl call` of `org.freedesktop.systemd1.Manager.StartTransientUnit` (the call
`systemd-run --scope` makes for its own process), for a transient scope unit that holds the script's own process ID, with
the properties `MemoryMax` (the cap of R1), `MemorySwapMax=0`, `OOMPolicy=continue` and `CollectMode=inactive-or-failed`.
Nothing else runs as root. The command then starts as the script's child, so it inherits the caller's user, groups,
environment, working directory and limits unchanged, and only its control group differs. `OOMPolicy=continue` keeps the
scope running after a kill and leaves `memory.oom.group` at `0`, so the kernel stops one process, never the whole scope.

R3. **Inside the scope, before the command runs, the script refuses unless the cap is in force.** It reads its own control
group from `/proc/self/cgroup` (waiting a bounded number of reads for the unit to take it) and, under `/sys/fs/cgroup`,
requires: the group is the scope's unit; `memory.max` equals the cap; `memory.swap.max` is `0`; `memory.oom.group` is `0`;
`memory.events` holds `oom` and `oom_kill`, both `0`; and `systemctl show --property=OOMPolicy --value <unit>` reads
`continue`. Otherwise it runs nothing, prints `memory-scope: REFUSED: <why>`, writes the record of R4 with `in_force` false
and the reason, and exits 78.

R4. **The leg keeps a record, begun before the command and finished after it.** Before the command the script writes
`<report>/memory-scope.json` with `in_force` true and `state` `running`. After the command it replaces the file whole with
`state` `done`, the `oom`, `oom_kill` and `max` counts of the scope's `memory.events`, and the peak (`memory.peak`) as a
whole percentage of the cap, then prints one line, `memory-scope: peak <p>% of the cap; the kernel stopped <k> process(es)
at the cap`. Neither the record nor the line holds an absolute size.

R5. **The script's exit status is the command's,** unchanged; a command ended by a signal gives 128 plus the signal's
number. The step's `|| rc=$?` and its `cargo-mutants.exit` record therefore hold cargo-mutants' own exit, as before.

R6. **Every `cargo mutants` command that runs tests runs inside the scope:** `ci.yml`'s `mutation-rust` step and
`mutation-weekly.yml`'s rust step and `rehearsal` step each run `python3 scripts/memory_scope.py --report <the step's
--output directory> -- cargo mutants <arguments>`. A `cargo mutants` command with `--list` runs no test and is not wrapped.

R7. **Nothing that decides the examined set changes.** Each wrapped command's `cargo mutants` arguments, and each listing
command, are the ones the workflow ran before this SPEC, byte for byte, so the listing, the shard partition, both of
cargo-mutants' timeouts (`--timeout` and `--build-timeout`, their values unchanged), `--in-diff`, `-f`, `--package` and
`--output` are unchanged, and the unmutated baseline runs as before, inside the same scope. `.cargo/mutants.toml` and
`scripts/mutation-equivalent.d/` are unchanged.

R8. **The verdict reads every promised shard's record.** In `scripts/mutation-verdict.py`, for each shard that holds a
cargo-mutants report or a `cargo-mutants.exit`, `judge`, `battery` and `table` read its `memory-scope.json`: a record that
is absent, unreadable or not `done` is VOID by name (the tests may have run under a cap whose kills cannot be counted), and a
record whose `in_force` is false is VOID by name with its reason. A report named by `--outcomes`, which no workflow
gives, has no shard directory and is judged as before.

R9. **A shard the cap never touched is judged exactly as before.** When the record counts `oom` and `oom_kill` both `0`, the
verdict reads nothing more for that shard, and every line and exit it gives is the one it gave before this SPEC.

R10. **Any memory-cap event fails the leg, and the verdict names the mutant only when two witnesses agree.** When `oom` or
`oom_kill` is above `0`, the leg fails, always. The verdict then places each kill in the scenario logs the shard's report
names (every scenario log under `mutants.out/log/` when the report is not whole): a placed kill is one distinct test, by its
binary and name, whose nextest status reads `SIGKILL`, counted once however often nextest repeats the line. When the placed
kills equal `oom_kill`, each mutant holding one is a named memory-cap failure (R11), and a kill placed in the unmutated
baseline fails naming the baseline. In a shard whose report is not whole, a placed kill is named by the outcome whose log
holds it, or else by the scenario its log names on its first line. Otherwise the leg fails as `MEMORY-CAP AMBIGUOUS`,
naming both counts and every placed scenario, and no mutant is scored.

R11. **A named memory-cap failure is decided in one place.** The verdict scores a mutant the cap stopped only through one
function, `score_memory_cap`, which fails `<where>MEMORY-CAP <mutant>: the memory cap stopped its tests; neither caught nor
a timeout` and returns that the mutant is not examined. `judge`, `battery` and `table` call it and decide nothing about a
named kill on their own.

R12. **Every other mutant's result in the leg stands.** A named mutant is left out of the examined count (`judge`: the
examined count is cargo-mutants' caught, missed and timeout less the named mutants among them; `table`: the named mutant is
tallied nowhere), and every other outcome in the shard is judged as before. The partition (a listed mutant tested in no
shard is VOID, one tested more often than listed fails), the whole-report rule (a missing report, an exit
other than 0, 2 or 3, or counts short of the report's total is VOID) and the zero-examined VOID are unchanged.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | from a `/proc/meminfo` text and a page size, the cap is `MemTotal` times 15/16 rounded down to whole pages; a text with no `MemTotal` refuses (exit 78, the planted command never runs, the record's `in_force` false); the parser accepts `--report` and the command and no option that names a cap | `test_memory_scope.py` `the_cap_is_fifteen_sixteenths_of_the_machine_in_whole_pages_and_never_given` |
| A2 | the only privileged call is `sudo --non-interactive busctl call` of `StartTransientUnit` naming the script's own process ID, `MemoryMax` at the cap, `MemorySwapMax` `0`, `OOMPolicy` `continue` and `CollectMode` `inactive-or-failed`, and the command is not in its arguments | `test_memory_scope.py` `the_scope_holds_the_scripts_own_process_with_swap_forbidden_and_oom_policy_continue` |
| A3 | with a planted control group whose unit, `memory.max`, `memory.swap.max`, `memory.oom.group`, `memory.events` or `OOMPolicy` differs from R3, each alone: exit 78, a `REFUSED` line naming the arm, the record's `in_force` false with that reason, and the planted command, which writes a marker, never runs | `test_memory_scope.py` `a_scope_whose_cap_is_not_in_force_runs_nothing` |
| A4 | the planted command records its environment, working directory, user and groups: each equals the caller's | `test_memory_scope.py` `the_command_runs_with_the_callers_identity_and_environment` |
| A5 | the script exits 0, 2, 3 and 4 when the planted command does, and 137 when the command is ended by `SIGKILL` | `test_memory_scope.py` `the_commands_exit_status_is_the_scripts` |
| A6 | the planted command reads the record as `running`; after it, the record is `done` with the planted `oom`, `oom_kill` and `max` counts and the peak as a percentage of the cap, the line reads the same, and neither holds an absolute size | `test_memory_scope.py` `the_record_is_begun_before_the_command_and_finished_after` |
| A7 | across every workflow file, every `cargo mutants` command without `--list` runs through `python3 scripts/memory_scope.py --report <dir> --` with `<dir>` its own `--output`, and no command with `--list` does; the test prints how many commands it examined and refuses zero | `test_memory_scope.py` `every_mutants_run_that_runs_tests_is_inside_the_scope` |
| A8 | the `cargo mutants` argument strings of the three wrapped commands and the four listing commands equal the ones pinned from the tree before this SPEC | `test_memory_scope.py` `the_mutants_arguments_are_the_ones_before_the_scope` |
| A9 | a shard with a report and no record, an unreadable record, or one still `running` is VOID by name; a record with `in_force` false is VOID naming its reason | `test_memory_cap_verdict.py` `the_verdict_reads_every_promised_shards_record` |
| A10 | with `oom` and `oom_kill` both `0`, `judge`'s lines and exit equal those it gives for the same shard judged by the tree before this SPEC, even when a scenario log holds a `SIGKILL` status line | `test_memory_cap_verdict.py` `a_shard_the_cap_never_touched_is_judged_as_before` |
| A11 | with `oom_kill` `1` and one kill placed in one mutant's log: exactly one `MEMORY-CAP` failure naming that mutant; it is not examined; a missed mutant in the same shard still fails as `MISSED`, a caught one is examined, and the examined count is the tool's less one | `test_memory_cap_verdict.py` `a_mutant_the_cap_stopped_fails_its_leg_by_name_and_the_rest_stand` |
| A12 | `oom_kill` `1` with no placed kill, `oom_kill` `1` with two, and `oom` `1` with `oom_kill` `0` each fail as `MEMORY-CAP AMBIGUOUS` naming both counts, and no mutant is scored; a status line nextest repeats in its summary is one placed kill | `test_memory_cap_verdict.py` `an_attribution_that_does_not_match_fails_the_leg` |
| A13 | a kill placed in the unmutated baseline's log fails naming the baseline; a shard whose report is not whole and whose record counts a kill is VOID as before and also fails by name | `test_memory_cap_verdict.py` `a_cap_kill_in_the_baseline_or_a_partial_shard_fails_by_name` |
| A14 | `battery` and `table` fail each named kill as `MEMORY-CAP <mutant>`, `table` tallies it nowhere, and both read a missing or unfinished record as `judge` does | `test_memory_cap_verdict.py` `the_weekly_battery_and_table_refuse_a_cap_kill_by_name` |
| A15 | `score_memory_cap` is the only function that writes `MEMORY-CAP` for a named mutant: the test replaces it once and every one of `judge`, `battery` and `table` follows the replacement with no other change | `test_memory_cap_verdict.py` `a_named_cap_kill_is_scored_in_one_place` |
| A16 | the plant: a shard modelled on the proof (the plant's mutants caught but one, whose log holds nextest's `SIGKILL` status line and its summary repeat, and a record counting one kill) fails with exactly one finding, `MEMORY-CAP` naming that mutant, and an examined count one less than the plant's viable mutants; and, on this pull request's own CI at the plant commit, the `mutation-verdict` log names that mutant and only it | `test_memory_cap_verdict.py` `the_plant_fails_its_leg_naming_that_mutant_and_only_it`, and the CI run of the plant commit |
| A17 | the `rehearsal` step's wrapped command, its file and the sum it prints as examined are the ones before this SPEC, and on this pull request's own CI that job, whose cargo-mutants runs inside the scope, prints the same examined count as the same job printed for dev's tree and command in this pull request's first run | `test_memory_scope.py` `the_rehearsal_counts_what_it_counted_before`, and the CI run of the head |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k the_cap_is_fifteen_sixteenths_of_the_machine_in_whole_pages_and_never_given
A2: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k the_scope_holds_the_scripts_own_process_with_swap_forbidden_and_oom_policy_continue
A3: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k a_scope_whose_cap_is_not_in_force_runs_nothing
A4: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k the_command_runs_with_the_callers_identity_and_environment
A5: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k the_commands_exit_status_is_the_scripts
A6: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k the_record_is_begun_before_the_command_and_finished_after
A7: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k every_mutants_run_that_runs_tests_is_inside_the_scope
A8: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k the_mutants_arguments_are_the_ones_before_the_scope
A9: python3 -m unittest discover -s scripts/tests -p test_memory_cap_verdict.py -k the_verdict_reads_every_promised_shards_record
A10: python3 -m unittest discover -s scripts/tests -p test_memory_cap_verdict.py -k a_shard_the_cap_never_touched_is_judged_as_before
A11: python3 -m unittest discover -s scripts/tests -p test_memory_cap_verdict.py -k a_mutant_the_cap_stopped_fails_its_leg_by_name_and_the_rest_stand
A12: python3 -m unittest discover -s scripts/tests -p test_memory_cap_verdict.py -k an_attribution_that_does_not_match_fails_the_leg
A13: python3 -m unittest discover -s scripts/tests -p test_memory_cap_verdict.py -k a_cap_kill_in_the_baseline_or_a_partial_shard_fails_by_name
A14: python3 -m unittest discover -s scripts/tests -p test_memory_cap_verdict.py -k the_weekly_battery_and_table_refuse_a_cap_kill_by_name
A15: python3 -m unittest discover -s scripts/tests -p test_memory_cap_verdict.py -k a_named_cap_kill_is_scored_in_one_place
A16: python3 -m unittest discover -s scripts/tests -p test_memory_cap_verdict.py -k the_plant_fails_its_leg_naming_that_mutant_and_only_it
A17: python3 -m unittest discover -s scripts/tests -p test_memory_scope.py -k the_rehearsal_counts_what_it_counted_before
```

The script tests plant a `/proc/meminfo` text, a control-group directory and a `sudo` and a `systemctl` that record their
arguments, through the script's own functions (never through an option, so no test seam can set the cap), in the way
`test_memory_watch.py` plants a control-group tree; no test starts a real scope. The verdict tests build shard directories
as the artifacts land (`mutants.out/outcomes.json`, `mutants.out/log/*.log`, `cargo-mutants.exit`, `memory-scope.json`)
and run `mutation-verdict.py` as the workflows do.

**The plant, on this pull request's own CI.** One commit adds `crates/kernel/src/memory_cap_plant.rs`, a function that
pushes a fully written chunk for each halving of its argument until it reaches zero, and a test that pins the result. Every
mutant cargo-mutants makes of it is caught by that test except one: `>` replaced with `>=` in the loop's condition never
ends, so that mutant only allocates. The commit's `mutation-rust` legs run the plant's mutants inside the scope, and its
`mutation-verdict` job must fail with one finding, `MEMORY-CAP` naming that mutant and no other, with every other plant
mutant examined. The next commit removes the plant, so no Rust file changes at the head. `docs/red-first/SPEC-196.md`
records both commits, the run, the verdict's lines and the leg's `memory-scope:` line.

**The examined count, on this pull request's own CI.** The pull request changes `mutation-weekly.yml`, so its `rehearsal`
job runs cargo-mutants over one file inside the scope and prints `rehearsal: cargo-mutants examined <n>`; `<n>` equals the
count the same job printed for dev's tree and dev's command, in this pull request's first run, before the wrapper.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/memory_scope.py` | repo | added: R1 to R5 |
| `scripts/tests/test_memory_scope.py` | repo | added: A1 to A8, A17 |
| `scripts/mutation-verdict.py` | repo | changed: R8 to R12, in `judge`'s rust class, `battery` and `table`; the whole-report rule, the partition and the exit sets unchanged |
| `scripts/tests/test_memory_cap_verdict.py` | repo | added: A9 to A16 |
| `scripts/tests/test_mutation_verdict.py` | repo | changed: its shard fixtures write the record every leg now writes, with no kill counted (R8, R9); no assertion changes |
| `scripts/tests/test_mutation_workflows.py` | repo | changed: its shard fixture writes the same record (R8, R9); no assertion changes |
| `.github/workflows/ci.yml` | repo | changed: R6, the `mutation-rust` step's command line only |
| `.github/workflows/mutation-weekly.yml` | repo | changed: R6, the rust step's and the `rehearsal` step's command lines only |
| `scripts/mutation-rows.d/S19600-S19699.json` | repo | added: hand-proved rows, one per guard arm of the script, the three wrapped lines and the verdict's reading |
| `docs/schematics/ci-jobs-and-caches.md` | repo | changed: an insert-only section, the mutants step inside its scope and the verdict's reading |
| `docs/specs/SPEC-196-a-runaway-mutant-is-stopped-inside-the-legs-memory-scope.md` | repo | added, from `docs/specs/planned/` |
| `docs/decisions/ADR-199-a-memory-scope-stops-a-runaway-mutant-and-the-timeouts-stay.md` | repo | added |
| `docs/red-first/SPEC-196.md` | repo | added |
| `changelog.d/mutation-memory-scope-196.md` | repo | added |

## 5. What this does NOT do

- It does not score a mutant the cap stopped as caught, because that would move a verdict in the permissive direction, a
  change to the gate's rules that is the owner's decision (#217, #439).
- It does not score a mutant the cap stopped like a timeout; that is the owner's decision, recorded on this SPEC's issue, and
  it would change the body of `score_memory_cap` alone (#439).
- It does not change cargo-mutants' timeouts, the listing, the shard partition or the baseline, because those decide the
  examined set (#217).
- It does not skip, exclude or record as equivalent a mutant whose tests grow memory, because that would examine fewer
  mutants (#217).
- It does not rerun a leg or a mutant the cap stopped, because the same mutant runs away in every run (#217).
- It does not change the shard count or the per-mutant cost table, whose bound is its own decision (#368).
- It does not wrap `mutation_rows.py prove` or the Python mutation runner, whose killers are hand-written rows or Python
  tests (#431, #218).
- It does not change how the workflow tests find the `cargo mutants` command: the wrapper keeps that spelling on the same
  line (#418, #395).
- It does not add a Rust or toolchain cache (#370).

## 6. Risks

- **A runner image without passwordless `sudo`, `busctl`, a unified control-group tree or swap accounting.** R3 refuses
  before cargo-mutants runs, so the shard has no report and a record with `in_force` false, and the verdict reads it VOID by
  name: a visible red, never an unguarded run reported as guarded.
- **A kill the two witnesses disagree on.** A test killed by `SIGKILL` for another reason in a leg the cap also touched, a
  build the cap stopped, or a kill in a process nextest does not report on its status line: each fails the leg as
  `MEMORY-CAP AMBIGUOUS`, never passes it. A leg the cap never touched reads no log (R9), so a `SIGKILL` line alone changes
  nothing there.
- **An allocation the kernel refuses without its out-of-memory path.** The kernel raises no `oom` event when it refuses a
  high-order or no-retry allocation at the cap (the control-group documentation); a test that fails that way with no kill
  recorded reads as its tool reads it. Such a refusal needs a process already at the cap, which is the runaway class the
  kill path covers; the record's `max` count shows every leg that reached the cap, so the class is measured from the first
  run.
- **Reclaim at the cap.** A leg's page cache counts against the cap, and the kernel reclaims it there before it would on the
  whole machine. Clean pages are dropped cheaply; the timeouts are unchanged, and the `max` count shows how often a leg
  reached the cap.
- **A scheduled weekly run before a release.** A scheduled run takes the workflow from `main` and the scripts from `dev`, so
  between this delivery's merge and the release that carries it, the scheduled battery finds shards with no record and
  reads them VOID by name. A dispatch at `dev` is unaffected. The changelog fragment names it.
- **A pull request whose diff holds a runaway mutant still fails,** now by name instead of by a lost runner, until its code
  changes or the owner decides otherwise (#439).

## 7. References

Issue #439; SPEC-039 and ADR-057 (cargo-mutants in place, in shards); SPEC-038 (the CI jobs); ADR-016; ADR-199; SPEC-031
and ADR-031 (the house's reading of control-group v2 files); cargo-mutants' timeouts chapter (mutants.rs/timeouts.html);
the Linux control-group v2 documentation (`memory.max`, `memory.swap.max`, `memory.oom.group`, `memory.events`,
`memory.peak`); systemd's `systemd.scope(5)`, `systemd.service(5)` (`OOMPolicy=`) and `systemd-run(1)`; cargo-nextest's
status lines; GitHub's hosted-runner documentation (passwordless `sudo`) and the runner project's guidance on the shutdown
signal.
