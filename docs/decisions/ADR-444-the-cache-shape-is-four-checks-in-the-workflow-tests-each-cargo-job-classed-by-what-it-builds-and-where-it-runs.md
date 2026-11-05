---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The cache shape is four checks in the workflow tests, and each job that runs cargo is classed by what it builds and where it runs

## Context and Problem Statement

The CI caches already have one shape. At `dev` `ac7ebd75` the 15 workflow files hold 15
`actions/cache/restore` steps and 4 `actions/cache/save` steps, and no combined `actions/cache`
step. 12 of the restores carry one Rust key, character for character:
`rust-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-${{ hashFiles('Cargo.lock') }}`.
The two Rust saves (`ci.yml:74-83` and `rust-cache.yml:90-99`) key on a restore's
`cache-primary-key`, the pnpm store and the Playwright browser are keyed on what `pnpm-lock.yaml`
locks, and no key carries anything that changes from run to run. A dev push's `rust` job
(job 116828746605, about 53 s from the exact hit to the restored tree) and a pull request's
(job 116815921139, about 54 s) both restore the entry that one dev push saved
(job 116499004794, whose save step took about 92 s).

Nothing holds that shape. `OnlyAPushSavesACache` (SPEC-038 R1, R2; SPEC-191 R9) reads only the
`ci.yml` jobs that call `scripts/check.sh` with a stage that compiles Rust, so a restore removed from
`mutation-rust`, `mutation-rows`, `web-engine`, any `mutation-weekly.yml` job or `rust-cache.yml`
fails nothing, and a key that drifts by one character there fails nothing either. Its key test
asserts that the two `hashFiles` parts are present, not that the key is the shared one. And no
test reads a key for a run-unique part, which would make every restore miss and fill the
repository's cache budget with entries no later run reads (ADR-055, its rejected "key with the
commit in it").

The rule this delivery holds has four parts: every job that runs cargo restores the one shared Rust
key; only the warm workflow or a push to `dev` or `main` saves it, and a pull request never writes
a cache; every key hashes its lock and toolchain inputs; and no key carries a run-unique part. Read
literally, the first part names 19 jobs at `ac7ebd75`, and 8 of them restore no cache: three that
only list mutants, the two measurement workflows, and three jobs that build releases. Each of the
eight restores nothing for a reason the tree states, so each decision below says how the check
reads it.

## Decision Drivers

- The tip passes: a lint that is red on the day it lands is a backlog, not a lint, and no job's
  cache behaviour changes in this delivery.
- No member of a population is dropped before it is asserted: every job that runs cargo is
  examined, and each class carries an assertion of its own.
- No allow-list and no suppression: a class is read from the job itself (its commands, its
  triggers, its runner), never from a list of job names.
- Every check prints what it examined and is VOID on zero, and each compares its population with a
  control the code under test did not build.
- Its checks share the module with other deliveries' checks without sharing an edited line.

## D1. One shared Rust key: what runs cargo, what restores the key, and the classes

A job runs cargo when one of its steps does. A step does when its `run:` script, with comments
removed and backslash continuations joined, holds `cargo <subcommand>` at a command position (the
start of a line, after `;`, `&&`, `||`, `|`, `(`, `$(`, a backtick, `if`, `then`, `do`, `else`,
`elif`, `while`, `until`, `!`, `exec` or `time`, after an environment assignment `NAME=value`, or
after a ` -- ` separator, which is how `python3 scripts/memory_scope.py ... -- cargo mutants` runs
it), or when it calls a script that does. `<subcommand>` is the first word after `cargo` that is
not an option (a word starting `-` or `+`), and it matches `[a-z][a-z0-9-]*` at its start: so
`cargo = json.load(...)`, a Python line inside a run script (`mutation-weekly.yml:480`), is no
call, and `cargo build` and `cargo --locked test` are calls. A shell script it calls (`bash` or `sh`
with a path under `scripts/`) is read with the same reader; a Python script (`python3` with a path
under `scripts/`) is read for an argument list that starts `["cargo", "<subcommand>"`, along the
subcommand the call names (D10); and `bash scripts/check.sh` is read stage by stage, each stage's
function body from `scripts/check.sh` (every stage of `STAGES_ALL` when the call names none). A
stage body that calls `need_cargo` and shows no cargo command, as `stage_python`'s guard tests do,
counts as one build. The reader goes one script deep.

Each cargo call is one of three kinds:

- **compiles nothing**: `fmt`, `fetch`, `deny`, `clean` and `metadata`, and a listing
  (`cargo mutants` with `--list`, `cargo nextest list`);
- **builds another profile or target**: `--release`, `-r`, `--profile` naming any profile but `dev`
  or `test`, `--target`, or `cargo install`;
- **builds the cache's profile**: every other call, an unknown subcommand included.

The shared entry holds what the saver built: the host's dev and test profile artifacts, after
`cargo clean --workspace` (`rust-cache.yml:8-14`, `ci.yml:71-73`); "the release build lands under
target/release, which the cache does not hold" (`ci.yml:150-152`).

A job that runs cargo takes the first class that fits:

| class | what makes it | what it must do |
|---|---|---|
| release | its workflow is started by a tag push (`on.push.tags` or `tags-ignore`) or a `release` event, or is a reusable workflow such a workflow calls, at any depth | restore no cache at all |
| another runner | its `runs-on` is a plain `macos-` or `windows-` label | restore no Rust-path cache: no job on that runner saves one |
| shared | at least one call builds the cache's profile | restore the shared key once, before its first such build |
| cache-free | every other job that runs cargo | restore the shared key, if it restores a Rust-path cache at all |

"Restores the shared key" is an `actions/cache/restore` step whose `path` lines are the Rust
cache's four and whose `key` is the shared key byte for byte, with `lookup-only` not true, at a step
index below the first step that builds the cache's profile, and with an `if:` that is absent or
equal to that step's. The job holds exactly one such step.

Read by running this reader over the tree at `b25ed04a` (SPEC-430 section 5), the 19 jobs that
run cargo are 9 shared, 7 cache-free, 3 release and 0 on another runner, and none breaks its
class's assertion. The known false-positive classes (refused, but harmless) are a build whose
profile or target is set outside its command line (`CARGO_BUILD_TARGET`, a `.cargo/config.toml`
target, `CARGO_PROFILE_*`), which reads as the cache's profile; a narrower `if:` on the build than
on the restore; and the word `date` in a producing step's message (D4). The known false-negative
classes (passed, and possibly wrong) are a cargo call more than one script deep (a test module that
builds a crate, run by `scripts/mutation_python.py`); a Python argument list not written as one list
literal that starts `["cargo", "<subcommand>"` (a tuple, a list built in parts, a split string); a
`runs-on` that is an expression or a list, which reads as the shared runner; a bare `on: push`
workflow, which a tag push also starts but which is not read as the release class; and a cache-free
job dropping a restore it has today (`ci.yml`'s `release` and `web-engine`), which no class
requires.

## D1, chosen against

- Chosen: four classes read from the job itself, because each of the eight jobs that restore nothing has a reason the tree states and no class names a job.
- Every job that runs cargo restores the shared key (the literal reading): rejected, because five jobs would be hits on the day the rule lands, and curing them changes their cache use: `engine-measure.yml`'s cold build "restores no cache, on purpose" (SPEC-038 R12, #15), and the three listings and `fsrs7-measure.yml` would each fetch an entry of host build artifacts that none of them reads, about 53 s a run as the `rust` job's restore measured, one of them on every pull request's path to the mutation legs.
- Requiring the shared restore in release builds as well: rejected, because a release build that restores a cache built elsewhere is the sink of a cache-poisoning path, which the workflow linter's cache-poisoning audit reads the same way.
- A list of exempted jobs: rejected, because an allow-list is not part of this rule, and a list keeps passing a job after the reason for its place on it is gone.
- Only the `ci.yml` jobs that call `check.sh`, as `OnlyAPushSavesACache` reads them: rejected, because 6 of the 9 shared jobs at `ac7ebd75` call no `check.sh` stage, and a restore removed from any of them fails nothing today.
- A reader of the cargo subcommands `rustc`, `run`, `build`, `test` and `check` only, as `cargo_commands` reads the Apple build: rejected, because it misses `clippy`, `nextest`, `mutants` and every script call, and a typed list of what builds fails open on the next subcommand.
- Eleven subcommands that compile nothing, adding `tree`, `generate-lockfile`, `locate-project`, `pkgid`, `verify-project` and `version` to SPEC-430 R3's five: rejected, because the six have 0 uses in any workflow, `check.sh` stage body or called script at `b25ed04a`, and the five read fewer calls as compiling nothing, so the shorter set fails closed.
- Any token after `cargo` as its subcommand: rejected, because `cargo = json.load(...)` (`mutation-weekly.yml:480`) then reads as a call of the unknown subcommand `=`, which builds.
- The word directly after `cargo` as its subcommand, no option skipped: rejected, because `cargo --locked test` would then read as no call, and a job that builds would be lost.

## D2. Who saves

Every step that can write a cache is a writer: `actions/cache/save`, the combined `actions/cache`
(its post step saves at the end of a successful job, on any event), every action in
`CACHE_BY_THEMSELVES`, `actions/setup-node`, `pnpm/action-setup`, and any other action whose path
names `cache`, which is a problem by name because no rule here says when it writes. Each workflow's
writers are judged by the module's own `cache_problems`, read and not edited: it evaluates a save's
`if:` with the module's expression reader over a push to `dev`, a push to `main`, a pull request into
`dev` and into `main`, a fork's pull request, a push to another branch and a pushed tag, and admits
the warm workflow's scheduled save only in `rust-cache.yml` (SPEC-038 R2, SPEC-191 R9). So a
dev-push guard is any `if:` that evaluates true on a push to `dev` or `main` that missed its key and
false on every other scenario; the one shape at `ac7ebd75` is
`github.event_name == 'push' && (github.ref == 'refs/heads/dev' || github.ref == 'refs/heads/main') && steps.<id>.outputs.cache-hit != 'true'`,
on 3 saves, and the warm save's guard is `steps.lookup.outputs.cache-hit != 'true'` in a workflow
started only by `schedule` and `workflow_dispatch`.

A Rust save (its `path` lines the Rust cache's four) also keys on
`${{ steps.<id>.outputs.cache-primary-key }}`, where `<id>` is an earlier restore in the same job
with the same paths and the shared key, so a save can write no other key. At `ac7ebd75` there are
19 steps that can write, 4 that do, and 2 Rust saves.

## D2, chosen against

- Chosen: the module's own scenario reader for every writer, and a Rust-key rule beside it, because the scenarios already prove what a guard admits and a second reader of `if:` text would disagree with it.
- A dev-only guard, as the rule's shorthand reads: rejected, because SPEC-038 R2 saves on a push to `dev` or `main`, the release merge's push to `main` saves the entry every ref can read, and all three save guards at `ac7ebd75` would be hits.
- Matching the `if:` text against one spelling: rejected, because an equivalent guard written another way would fail and a wrong guard spelled close to the right one could pass.
- Keeping `CACHE_BY_THEMSELVES` as the only list of self-saving actions: rejected, because a caching action it does not name passes silently; one that names `cache` and that no rule describes is refused instead.

## D3. Every key hashes its inputs

Every key of every cache step is examined, a save's `cache-primary-key` resolved through its restore
(earlier, same job, same paths; an unresolved one is a problem), and judged by its paths:

| cache | paths | key | fallback (`restore-keys`) |
|---|---|---|---|
| Rust | the four of `RUST_CACHE` | the shared key, exactly | none, or exactly `rust-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-` |
| pnpm store | one `${{ steps.<id>.outputs.path }}` whose step runs `pnpm store path` | `pnpm-${{ runner.os }}-${{ hashFiles('pnpm-lock.yaml') }}`, exactly | none, or exactly `pnpm-${{ runner.os }}-` |
| Playwright's browser | `BROWSERS` | `playwright-${{ runner.os }}-${{ steps.<id>.outputs.version }}-chromium-headless-shell` | none |
| anything else | | a problem: a cache this rule does not know | |

The Playwright key hashes no file: it names the version `pnpm-lock.yaml` locks, so its `<id>` must be
an earlier step of the same job whose `run:` reads `pnpm-lock.yaml`; what that step derives is
`test_the_browser_cache_is_keyed_on_the_locked_playwright_version`'s to prove, and stays its. A Rust
fallback keeps the toolchain's hash, because an entry built by another toolchain holds artifacts
no build can reuse. At `ac7ebd75` the check reads 19 keys (14 Rust, 3 pnpm, 2 Playwright) and 13
fallback lines (11 Rust, 2 pnpm).

## D3, chosen against

- Chosen: one exact template per known cache, chosen by its paths, because a key that differs by one character misses every entry the others save.
- Asserting that the two `hashFiles` parts appear, as `test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile` does: rejected, because a key with an extra part, or `hashFiles('**/Cargo.lock')`, passes it and restores nothing the others saved.
- Admitting a cache whose paths match no template: rejected, because adding a cache is not part of this rule, and a new cache then arrives with its own template instead of passing unread.

## D4. No run-unique part, and the sites at the tip

Every key and every fallback line, resolved as in D3, is examined: 32 texts at `ac7ebd75`. Each
`${{ }}` in one must be `runner.os`, `hashFiles('<path>', ...)` with literal paths,
`steps.<id>.outputs.<name>`, `env.<NAME>` or `matrix.<name>`; any other expression is a problem,
and `github.run_id`, `github.run_number`, `github.run_attempt` and `github.sha` are named in it. An
`env.<NAME>` is resolved through the step's, the job's and the workflow's `env:` and judged again.
A step output is judged by its producing step's `run:`, which may name none of `GITHUB_RUN_ID`,
`GITHUB_RUN_NUMBER`, `GITHUB_RUN_ATTEMPT`, `GITHUB_SHA`, the clock (`date`, `EPOCHSECONDS`,
`EPOCHREALTIME`) or randomness (`RANDOM`, `uuidgen`).

The tip breaks none of the four rules under D1's classes: 0 sites, so this delivery cures none and
changes no workflow. A site a later tip breaks is cured in the delivery that finds it.

## D4, chosen against

- Chosen: a closed vocabulary of expressions, and a named list for the scripts that produce a key's step outputs, because the expressions a key may need are few and a script's text is not.
- A list of forbidden contexts alone: rejected, because a context it does not name (`github.event.after`, a pull request's head sha) would pass.
- Leaving a breaking site to a later delivery: rejected, because a rule that lands red is skipped or loosened; had the tip broken one, this delivery would list and cure it.

## D5. One home beside the other workflow checks

The four checks are one new class, `TheCachesHoldTheirShape`, in `scripts/tests/test_ci_workflows.py`,
inserted directly above `class TheEngineBuildsInEveryRustJob`, with its own constants (`SHAPE_*`) and
helpers (`shape_*`). It edits no existing line of the module and calls the module's helpers read-only:
`workflow_files`, `read_workflow`, `workflow_file_text`, `action`, `paths`, `lines_of`, `stage_calls`,
`cache_problems` and the constants `RUST_CACHE`, `BROWSERS`, `COMPILES_RUST`, `CACHE_BY_THEMSELVES`,
`GATE_CALL`, `STAGES`, `LOCAL_CALL`, `REPO` and `WORKFLOWS`. It reads every file through the loader
`workflow_file_text`, so the module's file-read census gains no site and `NOT_WORKFLOW_READS` is not
edited. Another delivery that adds pin and timeout checks to `WorkflowsAreHardened` shares no line
with this one. Each test prints `examined N` through `_support.examined`, refusing zero.

## D5, chosen against

- Chosen: a class of its own in the module that already reads the workflows, because the reader, the expression evaluator and the save scenarios are there, and its census already holds every file read.
- A new test module: rejected, because it would import the reader and the scenarios from this module and need entries of its own in the module's file-read and import censuses, which other deliveries edit too.
- A rule for the workflow linter: rejected, because the linter has no notion of the shared key, the warm workflow or the module's save scenarios, and a custom rule would be a second reader to keep in step with this one.
- New checks added into `OnlyAPushSavesACache`: rejected, because the edit would interleave with lines other rows anchor on and with SPEC-038's own tests.

## D6. Kill criteria

The rule is at fail with 0 hits expected. It is reviewed by these criteria, over a window that
reads "the last N merged PRs", never a number of days:

- **Live precision.** Fixes ÷ (fixes + new suppressions or allow-list entries) over the last 30
  merged PRs. Below 0.8 with at least 5 hits, a demotion is proposed.
- **New zero-hit rule.** If 2 or more of the rule's first 5 hits are false, a demotion is proposed.
- **Demotion.** A demotion, a narrowed class or a removed check is a weakening: it goes to the owner
  as a signed ruling, never into a commit alone.
- **Tooling.** `examined 0`, or a reader that fails to read a workflow or a script it names, is
  VOID, which blocks. It is never a pass.

## D6, chosen against

- Chosen: the four criteria, read over merged pull requests, because a count of pull requests measures the rule's use and a span of days measures the calendar.
- A window of days: rejected, because a quiet stretch would read as a precise rule.
- No kill criteria: rejected, because a rule nobody can retire on evidence is retired by hand, without any.

## D7. Not in this delivery

It changes no key's shape, no cache's size or eviction and no workflow, adds no cache, and adds no
allow-list arm or further lint package. Each is listed in SPEC-430 section 6 with the issue that
owns it.

## D7, chosen against

- Chosen: holding the shape as it is, because the rule was approved to hold it.
- Restoring only the registry part for the cache-free jobs: rejected, because that is a second key, a key-shape change this rule does not include.

## D8. Tests, rows and FORMAL

Each acceptance criterion has a test red before its code: the red commit holds the tests and the
census helpers as stubs that keep every input and return nothing, so each tree test fails on its
control before its count, and each planted test fails on its expected refusals. The tree tests
compare their populations with controls the code under test did not build: the `ci.yml` jobs
`stage_calls` reads with a stage of `COMPILES_RUST` other than `test-release`, the steps `action`
and `paths` read, and the fallback lines `lines_of` reads. Planted workflows are written under
both `.yml` and `.yaml`, with every key spelled literally, never built from a `SHAPE_` constant. No
test reads a clock. Hand rows S43000-S43060 mutate the new helpers, each killed by a planted test.
S43054 is the scoped Python read's negative control: its planted cargo list sits in a function the
handler calls, never in the handler's own body, and its mutant, which stops the walk before the
handler's calls, must be killed by it.

FORMAL is not applicable by surface. The surface is present: the shared entry is state that runs
share, written by a push's `rust` job and the warm job and read by every restore. This delivery adds
no writer, no reader, no ordering and no guard of it; it reads workflow text and refuses shapes, and
the guard it judges is evaluated by the module's existing scenario reader, a pure predicate. No
`@phx covers` line under `formal/` names a file it edits (0 of 251 at `ac7ebd75`).

## D8, chosen against

- Chosen: planted fixtures beside each tree census, because the tree passes and only a plant can show that a check refuses anything.
- Tree tests alone, recorded as not red: rejected, because a census that passes on the base proves no judgement.
- A model of the cache's protocol in this delivery: rejected, because it adds no actor to that protocol and the model would cover no code this delivery writes.

## D9. The shape of the record

One delivery under SPEC-430 and ADR-444. No SPEC or ADR on `dev` is amended: SPEC-038 R2 and
SPEC-191 R9 still say who saves and are still judged by `cache_problems`, unchanged. The schematic
`docs/schematics/ci-jobs-and-caches.md` gains one insert-only section after its last line, with
the census of every job that runs cargo, because its table's "restored by" column names three jobs
and the tip has eleven that restore the shared key.

## D9, chosen against

- Chosen: one SPEC and one ADR, and an appended schematic section, because the rule is new and the documents it touches keep every earlier byte.
- Amending SPEC-038: rejected, because its requirements stand as written and a second home for this rule would split it.
- A new schematic: rejected, because two documents would then draw the caches.

## D10. The Python read follows the subcommand the step names

Read whole, a Python script classes every caller by the union of its subcommands. At `b25ed04a`,
whose workflows, `check.sh` and called scripts are those of `ac7ebd75`, `scripts/mutation_rows.py`
holds two cargo argument lists, `["cargo", "test", ...]` in `run_killer` (`:1037`) and in `builds`
(`:1111`), both reached only from `prove_row`, which only `prove` calls (`:1220`). Its other
subcommands reach neither: `legs`, which `ci.yml`'s `mutation-plan` runs (`ci.yml:448`), `merge`,
which `mutation-verdict` runs (`ci.yml:766`), and `retired`. Read whole, those two jobs are shared
jobs that restore nothing, and the rule would be red on the day it lands over two jobs that build
nothing.

So a `python3` call of a script is read by an `ast` walk of the script. The walk starts at the
module's top-level statements, which run on every call, and reads in turn each top-level function
or class, and each method of a top-level class, that a body it reads names, by a name or an
attribute, until no new name is reached; the argument lists are those of the bodies read. The walk
follows the subcommand when the call names one (the word after the script's path, matching
`[a-z][a-z0-9-]*`) and the dispatch of the script's `main` compares one expression: the dispatch
is the `if` statements of `main` whose test compares an expression with a string by `==`, or with a
tuple, list or set of strings by `in` (`args.verb`, `mutation_rows.py:1448-1481`), and each gives
way to its body when its strings hold the subcommand and to its `else` otherwise, at any `elif`
depth. Every other statement of `main` is kept, the fall-through after the dispatch included. A
call that names no subcommand, and a dispatch over more than one expression, keep every branch. A
script that names `getattr`, `globals`, `vars`, `locals`, `eval`, `exec`, `__import__` or
`importlib`, through which a run can reach a name no walk follows, is read whole. So the read keeps
every list a run with that subcommand can reach and more: a name matched by attribute alone, and
the fall-through, are read for every subcommand, which fails closed. A1 prints every Python call it
read, scoped, unscoped and whole, and names each call whose read drops an argument list the whole
script holds.

At `b25ed04a` the reader reads 58 Python calls, 37 scoped, 13 unscoped and 8 whole, and 3 scoped
reads drop the script's two lists: `mutation_rows.py legs`, `retired` and `merge`. The census is
then D1's 19 jobs: `mutation-plan` is cache-free by its two listings, and `mutation-verdict` runs no
cargo. A planted subcommand whose handler calls the function that holds a cargo list is a job that
runs cargo, so the walk is proved to follow calls past the handler's own body (SPEC-430 A9).

## D10, chosen against

- Chosen: an `ast` walk of the script along the subcommand the step names, because it reads every list a run with that subcommand can reach, and a call it cannot scope keeps every branch or reads the whole script.
- Adding the shared restore to `mutation-plan` and `mutation-verdict`: rejected, because it edits two workflow jobs, which this delivery does not change (SPEC-430 R11) and which the open release pull request's workflow files also hold, and it costs about 53 s of restore a run in two jobs whose subcommands build nothing.
- Dropping the Python read: rejected, because `ci.yml`'s `mutation-rows` and `mutation-weekly.yml`'s `rows` would then read as cache-free, their restore would stop being required, and a narrowed rule is a weakening.
- Reading the whole script for every call: rejected, because every caller of a script then takes the union of its subcommands, and two jobs that build nothing read as shared jobs that break the rule.
- Reading only the handler's own body: rejected, because `prove`'s body holds no cargo list; its two lists sit below it, in `run_killer` and `builds`, which `prove_row` calls.
- A list of the subcommands that build, script by script: rejected, because it is a list of names this rule does not hold, and it keeps passing a subcommand after its handler starts to build.

## Consequences

- Good, because a restore removed, a key that drifts by one character, a pull request that writes a
  cache, a key that carries a run's identity and a cache nobody keyed each fail the hygiene job by
  name, in every workflow.
- Good, because no job's cache use changes and every existing test keeps its population.
- Bad, because a cache-free job may drop the restore it has today without failing anything (D1).
- Bad, because a cargo call two scripts deep is not read (D1).
- Good, because a job is classed by the subcommand it runs, so a script's other subcommands do not
  class it, and a call the walk cannot scope keeps every branch or is read whole (D10).
- Bad, because a Python argument list not written as one literal is not read (D1, D10).

## What would make this wrong

- A cache that holds release or cross-target artifacts: then the cache-free class holds jobs that
  read it, and its assertion is too weak.
- A second Rust key on purpose, for a job whose dependencies differ: then the one shared key is a
  constraint, and the rule needs that key's own template.
- A live precision below the D6 bound: then the classes mislabel jobs, and the rule is demoted by
  a signed ruling.

## More Information

SPEC-430 states the requirements, the criteria and the census. SPEC-038 (R1, R2, R12), SPEC-191
(R9), ADR-055 and ADR-191 decided the cache this rule holds. Issues: #15, #207, #370.
