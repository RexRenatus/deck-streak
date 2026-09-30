# SPEC-039: every change proves its tests kill its mutants

- **Wave:** W0. **Issue:** #217 (epic #1). **Context(s):** `repo` (`scripts/`, `.github/`,
  `.cargo/`, `web/app`'s configuration, `docs/`), and `deck-streak-kernel` for the one survivor the
  first measurement found (§1).
- **Decided by:** ADR-057 (this SPEC's own: the tools, the diff-scoped jobs, the rows and the
  weekly battery), ADR-012 (the testing strategy and the parity oracle), ADR-017 (hosted CI on
  pull requests into `dev`), ADR-055 (the gate's parallel CI jobs, beside which the mutation jobs
  run), ADR-056 (the packs stay box-only, so this delivery vendors none), and ADR-029 (the one
  golden reader).
- **Status:** written with its delivery (no planned copy existed), with its tests and
  `docs/red-first/SPEC-039.md` (ADR-016).

## 1. The problem, measured

Every number below was measured on `dev` at b1ce32d, in a scratch clone, with the pinned tools
(§2 R1).

- **Nothing proves that a test can fail.** CI runs the gate (`scripts/check.sh`) and no mutation
  tool. A test that passes whatever the code does passes CI.
- **The Rust population.** `cargo mutants --list` (cargo-mutants 27.1.0) generates 1,557 mutants
  over six packages: `deck-streak-vault` 939 (`rails.rs` alone 378), `deck-streak-kernel` 366,
  `deck-streak-ingest` 168, `deck-streak-daemon` 53, `deck-streak-api` 30 and
  `deck-streak-coordination` 1.
- **A diff that changes code the tool cannot mutate gives zero mutants, and exit 0.**
  `cargo mutants --list --in-diff` over six one-file diffs of `crates/kernel/src/study_day.rs`
  printed `No mutants to filter` and exited 0 for every one of them: a doc comment, a comment
  inside a function body, an added attribute, a changed `derive`, a changed constant, and a
  changed comparison inside `Hour::new`.
  - The last is not an edge. cargo-mutants never looks inside a method named `new` in an `impl`
    block (`src/visit.rs::visit_impl_item_fn`: "Don't look inside constructors (called "new")
    because there's often no good alternative"). Renaming `Hour::new` to `make` gave three mutants
    on the same line.
  - `crates/*/src` holds 24 methods named `new`. Several of them validate: `Hour::new` (0 to 23),
    `TopicKey::new` (the vault's lowercase slug, SPEC-042 A8), and the settings and data-rights
    constructors.
- **A real survivor exists today.** cargo-mutants over `crates/kernel/src/clock.rs` (9 mutants):
  5 caught, 3 unviable, and 1 missed: `delete -` in `SystemClock::now`, the branch that reads a
  system time before the Unix epoch. No test reaches it.
- **The web front end has survivors too.** StrykerJS 10.0.0 with the Vitest runner over
  `web/app/src/lib/startapp.ts`: 20 mutants, 13 killed, 7 survived (among them the regular
  expression's two anchors), in 16 s. It ran only after `paraglide-js compile` and
  `svelte-kit sync`; without them the sandbox's build failed.
- **mutmut does not fit the parity oracle's Python.** mutmut 3.8.0 over
  `tools/parity-oracle/generate.py` stopped: the tests load the generator by path as
  `parity_generate`, and mutmut keys its trampolines by the file's module path
  (`tools.parity-oracle.generate`, a directory name with a hyphen). With the test changed to load
  it under that name, mutmut reported 415 of 415 mutants as "no tests" and exited 0: a run that
  examined nothing, reported green.
- **Copying the tree against building in place.** cargo-mutants over `clock.rs`, with `-j 1` for
  the copy and serial in place: the default copy took 41 s, 21 s of it a cold build in a temporary
  directory; `--in-place` took 34 s cold and 17 s warm. With Anki's engine in the workspace
  (SPEC-022) a cold build of `ingest` takes minutes, which the copy pays on every run.
- **GitHub runs `schedule` only for a workflow on the default branch.** "Scheduled workflows will
  only run on the default branch", and `workflow_dispatch` "will only trigger a workflow run if
  the workflow file exists on the default branch" (GitHub's "Events that trigger workflows").
  DeckStreak's default branch is `main`, which changes only by a release (CHARTER 21).

## 2. Requirements

R1. **The tools, pinned.** Rust: cargo-mutants 27.1.0, installed in CI by
    `taiki-e/install-action` (pinned by SHA) as `cargo-mutants@27.1.0` with `fallback: none`,
    running each mutant's tests under cargo-nextest 0.9.146 (`test_tool = "nextest"`), the gate's
    own runner, installed the same way. The
    Mini App: StrykerJS 10.0.0, `@stryker-mutator/core` and `@stryker-mutator/vitest-runner` at
    exactly `10.0.0`, as `web/app` devDependencies. The parity oracle's Python has no generated
    mutants (mutmut measured unfit, §1); its invariants are hand-proved rows (R8).
R2. **Production code** is exactly:
    - `rust`: `crates/*/src/**/*.rs`;
    - `web`: `web/app/src/**/*.{ts,js,svelte}`, less `*.test.*`, `*.spec.*`, `*.d.ts` and the
      generated `web/app/src/lib/paraglide/**`;
    - `oracle`: `tools/parity-oracle/generate.py`.
    Everything else (tests, `scripts/`, the golden reader `tools/parity-oracle/golden.rs`, the
    registry modules, documents and workflows) is not production code. A row (R8) may still guard
    it.
R3. **A diff-scoped run on every pull request into `dev` and `main`, the release's included.** Five
    CI jobs, each a need of the aggregate `ci` job, run on every event and decide from the diff what
    applies. The diff is the merge ref's, `HEAD^1...HEAD`, written once to `git.diff`:
    - `mutation-plan`: `scripts/mutation-verdict.py plan --event` reads the event's case and the
      diff into `plan.json`; when the Rust class applies, `cargo mutants --list --json --in-diff`
      lists the diff's mutants, and `mutation-verdict.py shards` sizes the shards (R18);
    - `mutation-rust`: one job per shard `k` of the plan's `n`, its matrix the plan's, each running
      `cargo mutants --in-place --in-diff --sharding round-robin --shard k/n` over the same
      `git.diff`;
    - `mutation-rows`: the rows the diff selects (R10), then the retirement check (R11);
    - `mutation-verdict`: under `if: always()`, the verdict (R4) over every shard's report and the
      rows' report, for the Rust class and the oracle's;
    - `mutation-web`: StrykerJS over every web production file the diff changes, whole, then the
      verdict. A release's Mini App changes run in this one job (R18).
    Each job prints its event's case by name and is never skipped, because `ci` reads a skipped
    need as failed:
    - a pull request into `dev` is judged on its diff. `dev` accepts a pull request only with an
      up-to-date head (ADR-034), so that diff is the merge's own;
    - a release pull request into `main` is judged on its merge diff, every change `dev` carries
      since the last release, in as many shards as fit (R18). The oracle's Python is judged, as on
      `dev`, by the rows the diff selects (R8; #219);
    - a push whose subject names the pull request it merges (`Merge pull request #N`) reads
      `not-applicable`, naming `#N`, whose jobs judged that same tree: `dev` accepts a pull request
      only with an up-to-date head, and `main` cannot move under a release pull request, since only
      `dev` reaches it and one pull request per head and base can be open (ADR-034);
    - a push that names no pull request is judged on its first-parent diff, `HEAD^1...HEAD`.
    A pull request whose diff holds no production path for a job's class reads `not-applicable`
    and names the paths it changes. Each job uploads its report under `if: always()`, restores
    caches and saves none.
R4. **The verdict, `scripts/mutation-verdict.py judge`, per class the diff touches.**
    - It reads the tool's own report, never its exit alone: `mutants.out/outcomes.json` and
      Stryker's `mutation.json`.
    - Examined is caught plus missed plus timed out (Stryker: killed, survived, no coverage and
      timed out). An unviable mutant, or a compile or runtime error, is not a kill and is not
      examined.
    - A missed or uncovered mutant fails the job, naming it.
    - A class whose production files changed a code line (neither blank nor a comment) and whose
      examined count is zero is **VOID**, and VOID fails the job. A row the diff selects whose
      anchor overlaps a changed line of that file counts as examined for it (R10).
    - A class whose changed lines are all blank or comments reads
      `not-applicable: N changed line(s), all blank or comments` and passes, naming each file. A
      file whose change only deletes lines reads the same way, with its count of deleted lines.
    - *Inserted by section 11:* in the Rust class, a changed line inside an item that
      cargo-mutants never mutates for a test attribute is test-only, not a code line, and a class
      whose changed code lines are all test-only reads `not-applicable` by name (SPEC-057 R22).
    - A missing or unreadable report on a class that applies is VOID, never zero. So is a partial
      one: cargo-mutants writes its report as it goes, so a report is read only when the tool's
      exit is 0, 2 or 3 and its caught, missed, timed-out and unviable counts sum to its
      `total_mutants`.
    - A line is blank or a comment by a lexer that reads each language's literals: Rust's strings,
      raw strings and character literals, TypeScript's and JavaScript's quoted strings, template
      literals and regular expressions, and Python through its own tokenizer. A comment opener
      inside a literal opens nothing, and a line inside a multi-line literal is code.
R5. **Survivors and exclusions.** A surviving mutant blocks the pull request until a test kills
    it, or until it is recorded as equivalent. An equivalent Rust mutant is one anchored
    `exclude_re` entry in `.cargo/mutants.toml`, with `# EQUIVALENT: <reason> (#N)` on the line
    above; a Mini App one is `// Stryker disable next-line <mutator>: EQUIVALENT: <reason> (#N)`.
    A test refuses any exclusion without a reason and an issue, and any `mutants::skip`
    attribute. Nothing is excluded silently.
R6. **The configurations** load under their tools' own rules: `.cargo/mutants.toml` holds only
    cargo-mutants 27.1.0's keys, and `web/app/stryker.config.json` sets `testRunner` `vitest`, the
    `json` reporter and `thresholds` whose `break` is 100. `scripts/mutation-verdict.py configs`
    judges both by what each tool itself refuses: a key cargo-mutants' `Config` does not declare or
    a value of the wrong type, and a Stryker threshold outside 0 to 100, `high` below `low`, JSON
    that does not parse, or `ignoreStatic` without `coverageAnalysis` `perTest`. It also refuses a
    second Stryker configuration in `web/app` under any of the tool's sixteen default names
    (`stryker.conf` or `stryker.config`, with or without a leading dot, in `.json`, `.js`, `.mjs` or
    `.cjs`), since StrykerJS reads the first it finds, and a `mutate` list other than R2's web
    production code.
R7. **Local runs** stay targeted (one file, `-f`, or the diff, `--in-diff`), `--in-place` on a
    committed tree, one mutant at a time, then `cargo clean`. `--in-place` is the rule's `-j 1`:
    cargo-mutants 27.1.0 refuses any `-j` beside it (exit 1, measured). Heavy runs belong in CI.
    `docs/BUILDER-BRIEF.md` says so.
R8. **Hand-proved rows** cover invariants generic mutants test weakly: a constant, a method named
    `new`, a guard, a security check, a parity comparison.
    - They live in band fragments, `scripts/mutation-rows.d/S<lo>-S<hi>.json`, in the
      mutation-rows pack's shape: `{"tables": {...}}`, with the header (`_`, `arities`, empty
      `tables`) in `scripts/mutation-rows.json`.
    - One band per SPEC: SPEC-NNN owns `S<NNN>00` to `S<NNN>99`.
    - A row is a stem `S<n>-<NAME>`, its target, an anchor (`find`) that occurs exactly once in
      the target, one mutant (`replace`), a killer that names exactly one test, and a description
      of the behaviour the mutant removes. `MUTATIONS` rows target `crates/<crate>/<file>`;
      `CARGO_KILLED_SCRIPT_MUTATIONS` and `SCRIPT_MUTATIONS` rows name a path from the root.
    - A cargo killer is `<target>::<test path>`: an integration-test target of the row's crate,
      or `lib`, then the test's path in it. A script killer is `<module>.<Class>.<method>`, a
      unittest id in `scripts/tests` or `tools/parity-oracle`.
    - `scripts/mutation_rows.py` is the one reader, and resolves each row's target by its table's
      declared spelling. Its census refuses a row that can prove nothing: a find that is empty or
      equals its replacement, and a second row that installs another row's mutant for its killer;
      the reader refuses an id outside its band or held twice.
R9. **The runner, `scripts/mutation_rows.py prove`,** proves each row it is given, every row its
    selectors name together (`--all`, or a `--band`, each `--row` and a plan's `--rows-from`), each
    once:
    - it refuses a tree with a tracked change (exit 2), naming the file;
    - it checks the anchor occurs exactly once, and records the target's sha256;
    - it runs the killer on the unmutated tree, which must pass, selecting exactly one test;
    - it installs the mutant once, and treats a mutant that does not build (`cargo test
      --no-run`) or does not parse (Python) as VOID, never a kill;
    - *Inserted by section 12:* a target that is a shell script, by its `.sh` or `.bash`
      extension or by a shebang naming `sh`, `bash` or `dash`, has its mutant parse-checked on
      the mutated bytes, with `bash -n` for a bash script and `sh -n` otherwise (the shebang
      decides when it names a shell; else `.bash` is bash and `.sh` is sh), and a mutant that
      fails is VOID, never a kill (A41);
    - it runs only the killer, counting the tests selected from libtest's `running N test` lines
      or unittest's `Ran N test` line, and anything but exactly one is VOID;
    - a killer that fails with the mutant installed is KILLED; one that passes is SURVIVED;
    - it restores the saved bytes and re-checks the sha256 before anything else runs;
    - it prints one line per row and `examined N`, and exits 0 when every row was KILLED, 1 on a
      survivor, 3 on a VOID with no survivor.
R10. **The rows a diff selects:** every row whose target the diff changes, every row the diff adds
    or changes, and every row whose killer's file the diff changes. The `mutation-rows` job proves
    each, and a row that is not KILLED fails the verdict.
R11. **No weakening.** A row whose id leaves the population while its target file stays fails the
    `mutation-rows` job (`scripts/mutation_rows.py retired --base <ref>`), unless
    `scripts/mutation-rows.retired.json` records its id with a reason and the maintainer's
    approval. A census test (`scripts/tests/test_mutation_rows.py`) holds every committed row:
    its target exists, its anchor occurs exactly once, its killer resolves to exactly one test,
    its id lies in its fragment's band and is held once. So a killing test that leaves while its
    row stays is refused too.
R12. **A weekly full-repository battery,** `.github/workflows/mutation-weekly.yml`, on
    GitHub-hosted runners, over `dev`:
    - `schedule` weekly, and `workflow_dispatch`;
    - `rust`: cargo-mutants over the whole workspace in round-robin shards `--shard k/32`, the
      matrix naming every `k` from 0 to 31, each with `--timeout` on every cargo command and a
      job `timeout-minutes`;
    - `web`: a whole StrykerJS run; `rows`: every row proved;
    - every job keeps its report under `if: always()`, and `--build-timeout` bounds each
      mutant's build, which `--timeout` does not in place;
    - `survivors`: files each surviving mutant's file as one issue, deduplicated against the open
      issues by title, its body scrubbed by `scripts/public-scrub.py` before it is posted, with
      `issues: write` granted to that job alone and never on a pull request; then, whatever the
      jobs before it returned, it counts every report they promise (`mutation-verdict.py battery
      --shards 32`): each shard's whole `outcomes.json`, the rows' report and the Stryker sweep,
      and fails naming each that is missing or partial.
R13. **The known lag.** The schedule and the dispatch go live only when a release carries the
    workflow to `main`. Until then, a `pull_request` trigger filtered to the workflow's own file
    runs `rehearsal`: one small file through cargo-mutants, one row, one Stryker file, and the
    survivors job's drafting and scrub, with no issue filed.
R14. **No vendored pack.** The packs stay box-only (ADR-056, and the owner's decision for this
    delivery), so no file of the mutation-rows pack is vendored. What CI needs of it is
    DeckStreak's own code: the census (R8, R11) and the configuration check (R6).
R15. **`docs/BUILDER-BRIEF.md` gains a mutation section,** so every builder inherits R3 to R11 and
    R18.
R16. **The first rows** guard invariants that exist on `dev`: the kernel's 04:00 rollover and
    the redactor's constants (SPEC-020), ingest's once-a-study-day refusal and its no-upload rule
    (SPEC-022), identity's constant-time compare, the `auth_date` bound, the owner pin and the
    cookie flags (SPEC-024), the parity oracle's golden comparison (SPEC-029), and the vault's
    atomic write and topic key (SPEC-042). Each is proved by the runner.
R17. **The gate is proved red first.** The survivor in `SystemClock::now` (§1) is moved into a
    pure conversion, `UtcMillis::from_system_time`, beside a test that leaves it alive: the
    `mutation-rust` job goes RED on the pull request. A test that a time before the epoch reads as
    negative milliseconds then kills it, and the job goes GREEN. Both run ids are recorded.
R18. **The shards, and their bound.** `scripts/mutation-verdict.py shards` sizes a diff's Rust run
    from cargo-mutants' own listing of its mutants, so that no shard reaches its job's timeout:
    - a shard's projected time is the unmutated baseline's plus, for each mutant round-robin gives
      it (mutant `i` in shard `i mod n`, as cargo-mutants assigns them), its package's cost. Both
      are means over the weekly battery's shards on GitHub's `ubuntu-24.04` runners, rounded up and
      held in `mutation-verdict.py`: over run 36384080819's 31 reported shards, the baseline 371 s
      and a mutant of `ingest` 126 s, `daemon` 80, `coordination` 64, `api` 54, `kernel` 13,
      `identity` 8 and `vault` 8. A package the table does not name costs the table's highest;
    - it takes the fewest shards whose slowest is projected within 60 minutes, half the shard
      job's `timeout-minutes` of 120: the shards of runs 36373915578 and 36384080819 took from 0.66
      to 1.33 times the table's projection;
    - a diff that needs more than 256 shards, the most a job matrix holds, is refused with its
      projection, never capped;
    - the verdict counts every shard from `0` to `n-1`: one with no report, or a partial one, is
      VOID by name, and the shards' reports hold every listed mutant once: one in two shards fails,
      and one in none is VOID, each by name. A shard the plan gave no mutant owes no report, since
      cargo-mutants exits 0 and writes none when it has nothing to test.
    - *Inserted by section 11:* the listing step's empty output, which cargo-mutants leaves when
      no mutant overlaps the diff (it exits 0 before it prints), is an empty listing, one shard of
      no mutant; a missing listing is VOID (SPEC-057 R22).
    The mutation jobs' own bounds (SPEC-038 section 8): `mutation-plan` 15 minutes, each
    `mutation-rust` shard 120, `mutation-rows` 90, `mutation-verdict` 10 and `mutation-web` 60.
    Section 8 records the release's measured plan.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the runner refuses a tree with a tracked change before it installs any mutant | `test_mutation_rows.py` |
| A2 | an anchor that does not occur exactly once is VOID | `test_mutation_rows.py` |
| A3 | a killer that selects no test, or two, is VOID | `test_mutation_rows.py` |
| A4 | a killed mutant reads KILLED and its target is restored byte for byte | `test_mutation_rows.py` |
| A5 | a surviving mutant reads SURVIVED and fails the run | `test_mutation_rows.py` |
| A6 | a mutant that does not build or parse is VOID, never a kill | `test_mutation_rows.py` |
| A7 | a killer that fails without its mutant is VOID | `test_mutation_rows.py` |
| A8 | a cargo killer is built first and selected exactly once; its kill restores the file | `test_mutation_rows.py` |
| A9 | every committed row is anchored once and names exactly one test, inside its band | `test_mutation_rows.py` |
| A10 | a row that leaves while its target stays is refused without a recorded approval | `test_mutation_rows.py` |
| A11 | the census refuses a row that can prove nothing: a find equal to its replacement, a second row installing one row's mutant for its killer, an id outside its band or held twice | `test_mutation_rows.py` |
| A12 | a diff's paths fall into exactly the production classes of R2 | `test_mutation_verdict.py` |
| A13 | a production diff that examined nothing is VOID | `test_mutation_verdict.py` |
| A14 | a diff of blank lines and comments reads not-applicable, by name | `test_mutation_verdict.py` |
| A15 | a missed mutant fails the verdict, and an unviable one is not examined | `test_mutation_verdict.py` |
| A16 | a proved row on a changed line carries a file the tool could not mutate | `test_mutation_verdict.py` |
| A17 | a diff selects the rows on its paths, its added rows and its rows' killers | `test_mutation_verdict.py` |
| A18 | a missing report on a class that applies is VOID | `test_mutation_verdict.py` |
| A19 | the weekly battery's survivors become deduplicated, scrubbed issue drafts | `test_mutation_verdict.py` |
| ~~A20~~ | every exclusion names its reason and an issue, and no `mutants::skip` exists | `test_mutation_workflows.py` |
| A21 | the tools' configurations load under their own rules, judged by DeckStreak's own check | `test_mutation_workflows.py` |
| A22 | the weekly battery's shards cover their denominator and keep every report | `test_mutation_workflows.py` |
| A23 | only the survivors job may write issues, and never on a pull request | `test_mutation_workflows.py` |
| A24 | the mutation jobs are needs of `ci`, install pinned tools and save no cache, and only the verdict's job has a job-level `if`, `always()` | `test_mutation_workflows.py` |
| A25 | the builder brief teaches the mutation rules | `test_mutation_workflows.py` |
| A26 | a time before the epoch reads as negative milliseconds | `cargo test -p deck-streak-kernel --test clock` |
| A27 | each event reads its case by name, a release pull request into `main` is judged on its merge diff, and a diff with no production path names its paths | `test_mutation_verdict.py` |
| A28 | every job that runs cargo-mutants installs the nextest its configuration names | `test_mutation_workflows.py` |
| A29 | the weekly battery fails, naming each shard, the rows report and the Stryker report it lacks or holds only in part | `test_mutation_verdict.py` |
| A30 | a report the tool left partial is VOID: an exit other than 0, 2 or 3, or counts short of its total | `test_mutation_verdict.py` |
| A31 | a comment opener inside a string, a raw string, a character, a template or a regular expression opens no comment | `test_mutation_verdict.py` |
| A32 | every cargo-mutants command bounds its builds and its tests | `test_mutation_workflows.py` |
| A33 | the survivors job counts every report its jobs promise, whatever they returned | `test_mutation_workflows.py` |
| A34 | the plan shards the listed mutants round-robin, each in exactly one shard, in the fewest shards whose projected time fits the bound, and refuses a diff beyond 256 shards | `test_mutation_verdict.py` |
| A35 | the shards' reports hold every listed mutant once: one in two shards fails, and one in none is VOID, each by name | `test_mutation_verdict.py` |
| A36 | a shard with no report, or a partial one, is VOID by name, and the other shards' survivors still fail by name | `test_mutation_verdict.py` |
| A37 | the sharded job runs the plan's matrix at the plan's count, and the verdict's job counts every shard whatever the shards returned | `test_mutation_workflows.py` |
| A38 | a shard the plan gave no mutant owes no report, and a proved row on the diff's changed line carries it; a shard given mutants still owes its report | `test_mutation_verdict.py` |
| A39 | the runner proves every row its selectors name together, a band's, a row's by id and a plan's, each once | `test_mutation_rows.py` |
| A40 | the configuration check refuses a second Stryker configuration, `ignoreStatic` without per-test coverage, and a `mutate` list other than R2's | `test_mutation_workflows.py` |
| A41 | a shell target's mutant is parse-checked, `bash -n` for a bash script and `sh -n` otherwise (the shebang decides when it names a shell; else `.bash` is bash and `.sh` is sh): one that does not parse is VOID, one that parses and is caught is KILLED, the check reads the mutant and never runs it, a parser that is missing or hangs leaves the mutant VOID, the refusal names the shell's first stderr line, and a cargo killer's row is parsed before it is built | `test_mutation_rows.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_tracked_change_is_refused_before_any_mutant_is_installed
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k an_anchor_that_does_not_occur_exactly_once_is_void
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_killer_that_selects_other_than_one_test_is_void
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_killed_mutant_is_restored_byte_for_byte
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_surviving_mutant_fails_the_run
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_mutant_that_does_not_parse_is_void_not_a_kill
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_killer_red_without_its_mutant_is_void
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_cargo_killer_is_built_first_and_selected_once
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k every_committed_row_is_anchored_once_and_names_one_test
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_row_that_leaves_while_its_target_stays_is_refused
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k the_census_refuses_a_row_that_can_prove_nothing
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k the_production_classes_are_exact
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_production_diff_that_examined_nothing_is_void
A14: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_diff_of_blanks_and_comments_reads_not_applicable
A15: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_missed_mutant_fails_and_an_unviable_one_is_not_examined
A16: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_proved_row_on_a_changed_line_carries_its_file
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_diff_selects_its_rows
A18: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_missing_report_is_void_never_zero
A19: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k survivors_become_deduplicated_scrubbed_drafts
```
```retired
A20: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k every_exclusion_names_its_reason_and_issue
```
```acceptance
A21: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_tool_configurations_load_under_their_own_rules
A22: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_weekly_shards_cover_their_denominator_and_keep_reports
A23: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k only_the_survivors_job_writes_issues_and_never_on_a_pull_request
A24: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_mutation_jobs_are_needs_of_ci_with_pinned_tools_and_no_saved_cache
A25: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_builder_brief_teaches_the_mutation_rules
A26: cargo test -p deck-streak-kernel --test clock -- --exact a_time_before_the_epoch_reads_as_negative_milliseconds
A27: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k each_event_reads_its_case_by_name
A28: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k every_job_that_runs_cargo_mutants_installs_the_test_tool_it_names
A29: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_missing_or_partial_battery_report_fails_by_name
A30: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_partial_report_is_void_never_complete
A31: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_comment_opener_inside_a_string_is_not_a_comment
A32: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k every_cargo_mutants_command_bounds_its_builds_and_its_tests
A33: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_battery_counts_every_report_its_jobs_promise
A34: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k the_plan_shards_the_listed_mutants_within_their_bound
A35: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k the_shards_reports_hold_every_listed_mutant_once
A36: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_missing_or_partial_shard_report_is_void_by_name
A37: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_sharded_job_runs_the_plans_matrix_and_the_verdict_counts_every_shard
A38: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_diff_the_tool_lists_no_mutant_of_needs_no_shard_report
A39: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k every_row_its_selectors_name_is_proved_once
A40: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_configuration_check_refuses_what_stryker_would_read_otherwise
A41: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k TheRunnerParseChecksAShellMutant
```

A1 to A8 run the runner against a fixture repository built at run time in a temporary directory:
a git repository with one committed target, one unittest module, and (for A8) one crate with no
dependency and its own `Cargo.lock`, built into the fixture's own `target/`. A13 to A19 run the
verdict over synthetic diffs and reports written by the test, never over a real tool run, and so
do A27, A29 to A31, A34 to A36 and A38; those four also write a listing in cargo-mutants' own
`--list --json` shape, and shard reports. A9 reads the committed rows, and A11 plants rows the
census must refuse. A21 runs the configuration check over the tree and over planted
configurations, and reads its examined count. A22 to A25, A28, A32, A33 and A37 read the
workflows.

The red stubs, committed with the tests, keep every entry point and do nothing: the runner reads
every row KILLED without running anything, the census and the retirement check examine nothing,
and the verdict reads every class green; the `shards` verb writes no shard, and the judge reads no
shard's report. So each criterion fails by assertion for its own reason.
A26 pins behaviour the code already had (R17): it is recorded `not red`, and the gate's own red
and green are the `mutation-rust` job's two runs.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | `repo` | added |
| `docs/schematics/mutation-testing.md` | `repo` | added: the pull request's jobs, the runner, the weekly battery |
| `docs/decisions/ADR-057-mutation-testing-runs-on-the-diff-in-ci-and-weekly-on-dev.md` | `repo` | added |
| `docs/red-first/SPEC-039.md` | `repo` | added |
| `docs/BUILDER-BRIEF.md` | `repo` | changed: the mutation section (R15) |
| `scripts/mutation_rows.py` | `repo` | added: the reader and target resolver, the census, the runner and the retirement check (R8 to R11) |
| `scripts/mutation-verdict.py` | `repo` | added: the plan, the shards, the verdict, the survivors' drafts, the battery's count and the configuration check (R3, R4, R6, R12, R18) |
| `scripts/mutation-rows.json` | `repo` | added: the header (R8) |
| `scripts/mutation-rows.retired.json` | `repo` | added: the retirement record, empty (R11) |
| `scripts/mutation-rows.d/S02000-S02099.json` | `repo` | added: SPEC-020's rows (R16) |
| `scripts/mutation-rows.d/S02200-S02299.json` | `repo` | added: SPEC-022's rows (R16) |
| `scripts/mutation-rows.d/S02400-S02499.json` | `repo` | added: SPEC-024's rows (R16) |
| `scripts/mutation-rows.d/S02900-S02999.json` | `repo` | added: SPEC-029's rows (R16) |
| `scripts/mutation-rows.d/S03900-S03999.json` | `repo` | added: this SPEC's rows on its own runner and verdict (R16) |
| `scripts/mutation-rows.d/S04200-S04299.json` | `repo` | added: SPEC-042's rows (R16) |
| `scripts/tests/test_mutation_rows.py` | `repo` | added: A1 to A11, A39 |
| `scripts/tests/test_mutation_verdict.py` | `repo` | added: A12 to A19, A27, A29 to A31, A34 to A36, A38 |
| `scripts/tests/test_mutation_workflows.py` | `repo` | added: A20 to A25, A28, A32, A33, A37, A40 |
| `.cargo/mutants.toml` | `repo` | added (R5, R6) |
| `web/app/stryker.config.json` | `repo` | added (R6) |
| `web/app/package.json`, `pnpm-lock.yaml` | `miniapp` | changed: StrykerJS 10.0.0 (R1) |
| `.gitignore` | `repo` | changed: the tools' output directories |
| `.github/workflows/ci.yml` | `repo` | changed: `mutation-plan`, `mutation-rust` (one job per shard), `mutation-rows`, `mutation-verdict` and `mutation-web`, needs of `ci` (R3, R18) |
| `scripts/tests/test_ci_workflows.py` | `repo` | changed: `ci` needs the five mutation jobs beside the gate's five (R3) |
| `.github/workflows/mutation-weekly.yml` | `repo` | added (R12, R13) |
| `crates/kernel/src/clock.rs` | `deck-streak-kernel` | changed: `UtcMillis::from_system_time` (R17) |
| `crates/kernel/tests/clock.rs` | `deck-streak-kernel` | changed: A26 (R17) |
| `changelog.d/feat-mutation-039.md` | `repo` | added |
| `scripts/mutation_rows.py` | `repo` | changed by section 12: `builds()` parse-checks a shell target's mutant |
| `scripts/tests/test_mutation_rows.py` | `repo` | changed by section 12: A41 |
| `scripts/mutation-rows.d/S03900-S03999.json` | `repo` | changed by section 12: the rows that pin A41's decisions |
| `docs/decisions/ADR-057-mutation-testing-runs-on-the-diff-in-ci-and-weekly-on-dev.md` | `repo` | changed by section 12: a dated note |
| `docs/red-first/SPEC-039.md` | `repo` | changed by section 12: A41's record |
| `changelog.d/fix-shell-mutant-parse-288.md` | `repo` | added by section 12 |

## 5. What this does NOT do

- It generates no mutants for the gate's own Python (`scripts/`). Those scripts are guards with
  their own tests, and a guard's invariant can take a row here today; a tool over them is its own
  delivery (#218).
- It generates no mutants for the parity oracle's Python. mutmut does not fit (§1); measuring
  cosmic-ray, which mutates the file in place and runs any test command, is its own delivery
  (#219). The oracle's invariants are rows until then.
- It does not mutate the registry modules (`tools/parity-oracle/registry/`). They run only beside
  the predecessor's private checkout, no public test executes them, and the goldens they write
  are held current by digest (SPEC-029, #22).
- It does not name, at pull request time, which ordinary test kills which mutant, so a pull
  request that deletes a test no row names is not refused. cargo-mutants' `outcomes.json` records
  each phase's command and exit status and no test name; a killer map built from the weekly
  battery's logs is its own delivery (#220). Rows, whose killers the census holds, are refused.
- It writes no rows for the streaks and the economy. Their rows come with their waves (#81,
  #106).

## 6. Risks

- **A mutant the tool cannot generate passes the diff run.** A changed constant, attribute or
  method named `new` gives cargo-mutants nothing to mutate. When the class examined nothing the
  job is VOID (R4), so the change needs a row; when the class examined something elsewhere, the
  verdict names the file with zero examined as `unexamined`, and the weekly battery cannot see it
  either. The rows are the remedy, and the verdict's line names the file.
- **A deletion has nothing left to mutate.** A pull request that deletes a guard reads
  `not-applicable` for that file. A row anchored on the guard stops the deletion: its anchor
  vanishes, the census fails, and the retirement check refuses the row's removal (R11).
- **StrykerJS 10.0.0's open defects read a kill as a survivor,** never a survivor as a kill:
  a test file that fails to load is reported as Survived with zero tests (stryker-js #6150), and a
  global test filter misreads static mutants (#6144; this repository sets no `testFiles`). A false
  survivor fails the job loudly, which a person then reads.
- **A shard can outrun its projection.** The costs are means measured on GitHub's runners; a
  crate whose tests grow, a new crate the table does not name, or a slow runner makes a shard
  slower than projected. The bound is half the job's timeout, above the measured error (R18). A
  shard that still times out uploads a partial report or none, the verdict names it VOID, and the
  table is measured again from the weekly battery's reports.
- **A release costs runner time.** Its merge diff holds every mutant in the repository, so its
  run is about the weekly battery's size. A newer push to the release pull request cancels the
  run it supersedes, so only the head that merges pays in full.
- **A hosted runner can be shut down mid-shard.** Three of the 64 shard jobs across the two
  dispatches on this delivery's branch stopped on "The runner has received a shutdown signal" and
  uploaded no report (runs 36373915578 and 36384080819). The verdict names such a shard VOID
  (A36), never green, and "Re-run failed jobs" runs that shard and the verdict again. At that rate
  a release of 25 shards often loses one, so its run can need a re-run to go green.
- **A flaky test makes a survivor or a kill flaky.** cargo-mutants refuses a red baseline (exit
  4, VOID here), and the runner's control run refuses a killer red without its mutant (R9).
- **The weekly battery files noise.** Each issue is one file's survivors, titled by the file, and
  a title already open is not filed twice.
- **A shard that never reports reads as a shard with no survivor.** A runner shut down mid-run
  uploads nothing, and a run stopped early leaves a partial report. The survivors job counts every
  report the battery's jobs promise and fails naming each one missing or partial (A29, A33), and
  a pull request's verdict counts every shard its plan promised (A36).

## 7. The known lag

The weekly battery's `schedule` and `workflow_dispatch` take effect only once the workflow file is
on `main` (§1), that is, after the next release. Until then its `rehearsal` job proves the
machinery on every pull request that changes the workflow (R13), and GitHub's documented
exception, a dispatch of a workflow that has already run once, is measured on this delivery's
branch and recorded in its pull request.

## 8. Amendments at delivery

- **R3 gained each event's case by name (A27).** `ci` reads a skipped need as failed, so neither
  mutation job may be skipped. The orchestrator asked for each case explicitly, and
  `mutation-verdict.py plan --event` decides it: a pull request into `dev` is judged on its diff;
  a release pull request into `main` and a push that merges `#N` read `not-applicable` by name; a
  push that names no pull request is judged on its first-parent diff.
- **Correction (2026-09-28): a squash merge's push.** A push that merges a pull request by squash
  has a subject that ends "(#N)", which the plan does not read as a merge, so it is judged again on
  its first-parent diff: the work is repeated, and nothing is skipped. The plan's subject match
  gains that form with the next mutation delivery. Releases merge with a merge commit, never a
  squash (RELEASING.md).
- **R7: `--in-place` is the rule's `-j 1`.** cargo-mutants 27.1.0 refuses any `-j` flag beside
  `--in-place` (exit 1, a usage error), and in place it runs one mutant at a time. The verdict read
  that exit as VOID the first time it met it.
- **R12: thirty-two round-robin shards, not sixteen slices.** SPEC-038 measured Anki's engine
  re-running its build script on every cargo command (about 26 to 28 s, warm) and `ingest`'s 102
  tests at about 140 s, so an `ingest` mutant costs about three minutes. A slice of sixteen would
  hold most of `ingest`'s 168 mutants in one shard for about five hours; round-robin spreads them.
- **The population grew before the delivery.** At `dev` dbbd896, where #215 added identity,
  cargo-mutants lists 1,721 mutants (`deck-streak-identity` 143).
- **R16's rows are twenty-two:** S02001 to S02004 (SPEC-020), S02201 and S02202 (SPEC-022), S02401
  to S02405 (SPEC-024), S02901 to S02903 (SPEC-029), S03901 to S03905 (this SPEC's runner and
  verdict) and S04201 to S04203 (SPEC-042). The cookie row's killer lives in `api`, so it is a
  `CARGO_KILLED_SCRIPT_MUTATIONS` row naming its crate. Two bounds could not take a row, because
  no test pins them on its own: `Hour::new`'s upper bound and identity's future skew (#222).
- **The manifest gained `scripts/tests/test_ci_workflows.py`.** SPEC-038 A3 pins `ci`'s needs as
  an exact set, and the set grows by the two required mutation jobs, still an exact equality.
- **R1: cargo-mutants runs the gate's nextest (A28).** Its default runner, `cargo test`, runs a
  binary's tests in one process; the gate's nextest gives each test a process of its own. The first
  dispatch of the whole battery on this delivery's branch (run 36372763914) stopped two of its 32
  shards at the unmutated baseline (exit 4): identity's `init_data_never_reaches_the_log` failed
  under `cargo test` though it passes under nextest and passed a local `cargo test` run, so it
  depends on what shares its process. Every shard holds identity's mutants under round-robin, so
  every shard was exposed. With `test_tool = "nextest"` the identity baseline passes: `owner.rs`
  gave 14 mutants, 6 caught, 3 missed and 5 unviable, the missed three being the weekly battery's
  to file.
- **R4 reads a partial report as VOID (A30).** The verifier measured the judge reading a report
  that held 1 of its 3 mutants, under a SIGKILL's exit 137, as green. cargo-mutants writes
  `outcomes.json` as it goes, so a report is now read only when the exit is 0, 2 or 3 and its counts
  sum to `total_mutants`.
- **R4's lexer reads literals (A31).** The verifier found that a comment opener inside a string,
  such as `"crates/*"`, opened a block comment, so every later line, code included, read as a
  comment. Rust's strings, raw strings and character literals, TypeScript's and JavaScript's
  strings, templates and regular expressions, and Python's tokens are now read before any comment
  opener. Over the 91 tracked production files (R2) at the delivery's head, every line the lexer
  calls quiet is blank or a comment.
- **R12 counts every report (A29, A33) and bounds every build (A32).** The first dispatch on this
  delivery's branch (run 36373915578) lost shards 4 and 5 to a runner shutdown: they uploaded no
  report, and the survivors job read the other 30 and passed. The battery's count now fails, naming
  both (`battery --shards 32` over that run's artifacts: `counted 32 of 34 reports whole`, exit 1).
  In place, cargo-mutants bounds no mutant's build unless told, and never bounds the unmutated
  baseline's (`src/timeouts.rs`: `for_baseline` sets no build limit), so every run passes
  `--build-timeout 600` beside `--timeout 300`.
- **R6, R8 and R14: no vendored pack.** By the owner's decision the packs stay box-only, so this
  delivery vendors no file of the mutation-rows pack. The configuration check (`configs`) and the
  census's refusal of a second row installing one mutant are DeckStreak's own code. A11 and A21
  were rewritten against them, and red and green again.
- **R16's rows are twenty-seven:** S03906 to S03910 hold the fixes' own guards (the partial report,
  the missing shard, the literal lexer, an unknown cargo-mutants key and the census's duplicate
  mutant). Each was proved KILLED by the runner.
- **The Mini App's first whole sweep is backlog.** Run 36373915578's `web` job measured 53
  survived and 28 uncovered mutants of 274; #240 tracks them, and the battery files each file once
  it is live.
- **The red-first record's home.** `dev` takes this delivery as one squash commit, so the record's
  shas are the commits of pull request #221, reachable from its head, not from `dev`'s history.
- **R3 and R18: the release is judged on its merge diff, sharded.** By the owner's directive
  ("deckstreak will need per merge diff mutation testing", "dev to main"), a release pull request
  into `main` no longer reads `not-applicable`: it is judged on its merge diff by the path every
  diff takes (`mutation-plan`, `mutation-rust` one job per shard, `mutation-rows`,
  `mutation-verdict`, `mutation-web`). A pull request into `dev` is judged as before, on its merge
  ref's diff, and a push that names the pull request it merges still reads `not-applicable`.
- **The release's plan, rehearsed.** On a local synthetic merge of `dev` 09b60d2 into `main`
  3d77726, never pushed, `plan --event pull_request --base-ref main` read 438 changed paths: 61
  Rust files, 8 Mini App files and the oracle's generator. cargo-mutants listed 2,153 mutants of
  the merge diff, every mutant the merged tree holds (`vault` 939, `kernel` 366, `ingest` 297,
  `coordination` 292, `identity` 143, `daemon` 65, `api` 51), in under a second, building nothing.
  Projected at 77,478 s serially, `shards` took 25 shards of 86 or 87 mutants, projected at 3,356
  to 3,538 s each; 24 would put one at 3,680 s, past the bound. cargo-mutants' own
  `--shard k/25 --sharding round-robin` listings matched the plan's shards exactly: 2,153 mutants,
  none in two shards and none in none. The 8 Mini App files hold 266 of the app's 274 Stryker
  mutants, which the weekly battery's `web` job ran in about three minutes, inside
  `mutation-web`'s 60.
- **The costs, measured on GitHub's runners.** The battery's dispatch at this branch's 4082551
  (run 36384080819) ran 32 shards of 67 or 68 of the same mutants, and its shard jobs took 30.6
  to 58.0 minutes. The plan's table is its 31 reported shards' means, rounded up (R18). The
  earlier table, from run 36373915578, where `coordination` had one mutant, projected this run's
  shards at 0.74 to 1.32 times their measured time; the refit one projects both runs' shards at
  0.66 to 1.33. At the worst of these the release's slowest shard takes about 78 minutes of
  baseline and mutants, inside its job's 120.
- **Correction (2026-09-28): the mutants at 4082551.** cargo-mutants listed 2,154 mutants at this
  branch's 4082551, ten shards of 68 and twenty-two of 67, not the 2,153 the rehearsal listed at
  `dev` 09b60d2.
- **Correction (2026-09-28): the projection's direction.** Over runs 36373915578 and 36384080819,
  the refit table's projection over the measured time is 0.75 to 1.52, and the measured time over
  the projection is 0.66 to 1.33, as R18 states it.
- **Correction (2026-09-28): an `ingest` mutant's cost.** ADR-057's D11 prices an `ingest` mutant at
  124 s; the refit table in R18, which `scripts/mutation-verdict.py` holds, prices it at 126 s, the
  figure the plan uses.
- **The next release is red until its survivors are triaged.** Run 36384080819's 31 reported
  shards found 310 missed mutants in 33 files (218 in `vault`, 145 of them in `rails.rs`), and its
  `web` job 53 survivors and 21 uncovered mutants among the release's 266 (#240). A release's
  merge diff holds every one, so the next release pull request fails `mutation-verdict` and
  `mutation-web` until each is killed or recorded as equivalent. The weekly battery files each
  file's survivors once it is live.
- **The layout's first run went VOID, as it should.** In `ci` run 36389287255, `mutation-rows`
  proved all 45 rows KILLED, then could not write its report into a directory the job never made,
  and the verdict read `VOID 45 row(s) selected and no rows report`. The job now makes it, and A24
  holds it.
- **A38: a diff of no mutant owes no shard report.** cargo-mutants exits 0 and writes no
  `mutants.out` when a diff lists no mutant (measured on a changed constant), so the verdict read
  such a diff VOID even when a proved row carried its changed line; A16's synthetic report of zero
  mutants is one the tool never writes. The plan's listing now tells the verdict which shard owes
  no report.
- **R9: every selector's rows are proved (A39).** `prove --band` beside `--row` proved the band's
  rows and dropped the named one; the selectors now join.
- **R6: the configuration check reads what StrykerJS would read (A40).** StrykerJS reads the first
  of its sixteen default configuration names it finds, and refuses `ignoreStatic` without
  per-test coverage. `configs` refuses a second configuration, `ignoreStatic` without
  `coverageAnalysis` `perTest`, and a `mutate` list other than R2's, and
  `web/app/stryker.config.json` now leaves out every `*.test.*` and `*.spec.*` file, as R2 says,
  where it named only the `.ts` ones.
- **R16's rows are forty-five:** S03911 to S03928 pin the round's branches: the release's scope,
  the shards' bound, the refusal past 256 shards, the round-robin partition, the listing, the plan
  file, the matrix, every shard counted, a plan with no shards, a mutant in two shards or in none,
  a missing shard named once, a shard of no mutant, the selectors' union and the three Stryker
  refusals. The runner proved all 28 of the band's rows KILLED at the delivery's head, S03906 to
  S03910 among them again, each file restored byte for byte.

## 9. Amendment, 2026-09-28: section 8's figures corrected

Made after the delivery, insert-only under ruling (i) of SPEC-038 section 8: every earlier byte is
kept in order. It inserts:

- section 8: the bullet "Correction (2026-09-28): a squash merge's push", after the bullet on R3's
  cases;
- section 8: the bullets "Correction (2026-09-28): the mutants at 4082551", "Correction
  (2026-09-28): the projection's direction" and "Correction (2026-09-28): an `ingest` mutant's
  cost", after the bullet on the costs measured on GitHub's runners;
- this section.

ADR-057 carries a note of its own: its Confirmation's range is A1 to A40, and D11's `ingest` cost
is R18's 126 s.

## 10. Amendment, 2026-09-28: A20 retired by ADR-070, and the battery's dispatch

Made by SPEC-057's first delivery, the vault's, insert-only under ruling (i) of SPEC-038 section 8:
every earlier byte is kept in order. It inserts:

- section 3: `~~` around A20 in the criteria table, so the table no longer states it;
- section 3: the fence lines that set A20 apart in a `` ```retired `` fence between A19 and A21,
  splitting the acceptance fence where its line stood;
- this section.

The retired criterion, why its subject is gone, and what judges it now:

- A20 (every exclusion names its reason and an issue, and no `mutants::skip` exists): ADR-070,
  accepted with this delivery, amends R5 and ADR-057 D6. An equivalent mutant is no longer an
  anchored `exclude_re` entry or a `Stryker disable` comment, each of which hides its mutant from
  the listing or the run, so no later run tests the claim. It is a record in
  `scripts/mutation-equivalent.d/`, bound by an anchor to exactly one listed mutant that keeps
  running. A20 planted a justified exclusion and a justified comment as passing, and SPEC-057 R11
  now refuses every exclusion, justified or not. SPEC-057 A8 judges it, and its test replaced
  A20's in `scripts/tests/test_mutation_workflows.py`.

Two facts measured since section 8, recorded here rather than in it:

- **A dispatch runs at a feature branch.** Section 1, R13 and the weekly battery's header said
  that `workflow_dispatch` runs only a workflow on the default branch. Run 36384080819 was a
  dispatch of `mutation-weekly.yml` at this SPEC's delivery branch, while `main` held no copy of
  the file: once GitHub has registered a workflow, a dispatch runs it at any ref that holds it.
  `schedule` still runs only on the default branch. The header now says so, and SPEC-057 R14's
  scoped dispatch relies on it.
- **The squash form is read.** The correction in section 8 on a squash merge's push is delivered:
  a push whose subject's first line ends with ` (#N)` reads `not-applicable`, naming `#N`
  (SPEC-057 R18, A13).

## 11. Amendment, 2026-09-28: a test-only `src` diff reads not-applicable

Made by SPEC-057's first delivery, the vault's, insert-only under ruling (i) of SPEC-038 section 8:
every earlier byte is kept in order. It inserts:

- R4: the bullet "*Inserted by section 11:* in the Rust class, ...", after the bullet on a class
  whose changed lines are all blank or comments;
- R18: the bullet "*Inserted by section 11:* the listing step's empty output, ...", after the
  bullet on the verdict's count of the shards;
- this section.

What it amends, and why:

- **R2 and R4 counted a unit test as production code.** R2 names `crates/*/src/**/*.rs` production
  code by path, and R4 counts every changed line of such a file that is neither blank nor a comment
  as a code line. A unit test in a `#[cfg(test)]` module lives on that path, and cargo-mutants never
  mutates it: 27.1.0's visitor skips an item marked `#[cfg(test)]`, or one with an attribute whose
  path ends in `test` (`attrs_excluded` in its `src/visit.rs`; https://mutants.rs/mutants.html).
  SPEC-057's vault delivery measured the cost at 8b18276 and again at dd734e5, whose `crates/` are
  the same: its diff changes only three such modules, the plan read 71 of their lines as code,
  cargo-mutants listed nothing, and `mutation-plan` and `mutation-verdict` read VOID (run
  36461579108 at 8b18276, and run 36463302615 at dd734e5). SPEC-057 R22 now reads such a
  line as test-only. A class whose changed code lines are all test-only reads `not-applicable` by
  name; one that also changes a production code line applies as before. R2's paths are unchanged.
- **R18 read the tool's empty answer as no listing.** `cargo mutants --list --json --in-diff`
  prints nothing, not `[]`, when no mutant overlaps the diff: 27.1.0 exits 0 before it lists
  (`src/main.rs` and `src/in_diff.rs`). `shards` read that empty file as a missing listing, VOID.
  It now reads it as an empty listing, one shard of no mutant, whose verdict still reads VOID when
  a production code line changed and no row covers it (R4, R8). A listing step that fails stops
  its job before `shards` runs, so the empty file is the tool's own answer; a missing file stays
  VOID.

ADR-070 carries a note of this date that records the decision and what it was chosen against;
SPEC-057 A28 decides it.

## 12. Amendment, 2026-09-29: a shell mutant that does not parse is VOID

Made by issue #288's delivery, insert-only under ruling (i) of SPEC-038 section 8: every earlier
byte is kept in order. It inserts:

- R9: the bullet "*Inserted by section 12:* a target that is a shell script, ...", after the
  bullet on a mutant that does not build or parse;
- section 3: the row and the command of A41, after A40's;
- section 4: six manifest rows, after the changelog fragment's;
- this section.

What it amends, and why:

- **R9 parse-checked a Python mutant and nothing else that is not built.** `builds()` ran
  `ast.parse` only for a target ending `.py` and `cargo test --no-run` only for a cargo killer,
  and returned no refusal for any other target. A shell mutant that breaks the script's syntax
  then made its killer fail, and the row read KILLED although the killer observed nothing about
  the mutated behaviour, which is the read R9 exists to refuse. The tree holds five rows on a
  shell target, all on `scripts/check.sh`, and each was proved without a parse check.
- **A shell target is parse-checked, by the language it is written in.** A target is a shell
  script when its extension is `.sh` or `.bash`, or its first line is a shebang naming `sh`,
  `bash` or `dash`, directly or after `env`. The shebang decides when it names a shell; else
  `.bash` is bash and `.sh` is sh. The mutated bytes are checked with `bash -n` when the script
  is bash and `sh -n` otherwise, because the two disagree: an array assignment such as
  `a=(1 2)` passes `bash -n` and fails `sh -n`, so a checker that always picked `sh` would void
  every bash script that uses an array, and one that always picked `bash` would pass a POSIX
  script that only bash reads. The check runs before the cargo branch, so a script mutant with a
  cargo killer is parse-checked and then built. A parser that is not installed is VOID with its
  reason, and a check that outlives its bound is VOID.
- **The five existing rows are re-proved at the delivery's head.** Each reads KILLED, so none of
  them was a parse failure passing for a kill (the red-first record names the run).

ADR-057 carries a note of this date that records the decision and what it was chosen against;
A41 decides it.

## 13. Amendment, 2026-09-29: a band file that repeats a key is refused

Made by issue #334's delivery, insert-only under ruling (i) of SPEC-038 section 8: every earlier
byte is kept in order. It inserts this section only.

- **R8's one reader read the last of a repeated key.** Two branches that each add a table under the
  same key merge in git without a conflict, and `json.loads` kept the later value, so the rows under
  the earlier table vanished with no failure. The reader now refuses a key repeated in one object,
  at any depth, in the tree and in a revision, naming the file and the key. SPEC-122 decides it and
  ADR-122 records it; the rows are in `S12200-S12299`.

## 14. Amendment, 2026-09-29: the verdict reads each report by name

Made by issue #351's delivery, insert-only under ruling (i) of SPEC-038 section 8: every earlier
byte is kept in order. It inserts this section only.

- **The verdict's report layout depended on how many artifacts matched.** Its one download was a
  pattern over every `mutation-*` artifact, and the action extracts a single match flat, so a run
  in which only the plan had uploaded read `VOID no plan`. The verdict now downloads the plan and
  the rows' report each by name and the shards by a merged pattern, and reads no `mutation-web`
  artifact. SPEC-126 decides it and ADR-126 records it.

## 15. Amendment, 2026-09-29: a row's killer can run a binary's own unit tests

Made by issue #352's delivery, insert-only under ruling (i) of SPEC-038 section 8: every earlier
byte is kept in order. It inserts this section and section 16 only.

- **A row's killer could name a library test and an integration-test target, and nothing else.**
  A cargo killer was `<target>::<test path>`, where the target is a file of the crate's `tests/`
  or `lib`. A unit test that lives in a binary's own source (`src/main.rs` and the modules it
  declares) had no name a killer could take, so a constant in a binary carried no row. SPEC-057's
  R20 reserves `S05754`, the daemon's `EXIT_GRACE`, for the first such row: the test that pins it
  sits in `crates/daemon/src/main.rs`, and cargo-mutants never mutates a constant.
- **The kind is `bin`.** A killer `bin::<test path>` names one unit test of the crate's binary.
  `scripts/mutation_rows.py` reuses the cargo killer's machinery, with one more target kind and
  no second copy of the resolve or the VOID rule:
  - **The binary is read, never guessed.** Its name and root source come from the crate's
    `Cargo.toml`: the one `[[bin]]` table's `name` and `path` (default `src/main.rs`), or the
    package's own name when there is none and `src/main.rs` exists. A crate that holds more than
    one binary, or any file under `src/bin/`, is refused by name, since a `bin::` killer names no
    binary.
  - **The census resolves the killer statically against the binary's sources.** The sources are
    the module tree from the binary's root file: the file, then each `mod name;` it declares, as
    `name.rs` or `name/mod.rs` beside the root file (whatever its name, as rustc reads a crate
    root) or beside a `mod.rs`, and under a directory named for any other file, recursively
    (`#[path]` is not followed). The last segment of the
    test path must be declared under a test attribute exactly once in them; a test of the
    library, or one in a file the binary does not declare, is refused with the census's
    existing sentence, `its killer <killer> names no test: <root file> declares <name> 0 times`.
  - **The runner builds one argv.** `cargo test --locked -p <package> --bin <binary> -- --exact
    <test path>`, and for the mutant's build the same flags with `--no-run`. The flags come from
    one function, `cargo_flags`, which the `lib` and `--test` kinds use too.
  - **A killer that selects nothing is VOID, by the rule every cargo killer already has.** The
    control run must select exactly one test, counted from libtest's `running N test` line, so a
    `bin::` killer naming a test that does not exist reads VOID, its mutant is never installed,
    and it is never KILLED.
  - **`bin` joins `lib` as a reserved target name.** A crate that also holds `tests/bin.rs` or
    `tests/bin/main.rs` is refused for a `bin::` killer by name (`crates/<crate> has a test target
    bin, which the bin kind shadows`), so a killer written for that file is never run against the
    binary's own test of the same path.
  - **What was rejected, and why.**
    - A killer that names its binary (`bin:<name>::<path>`): rejected because the workspace holds
      one binary, and a second spelling would need its own parse, resolve and rows; a crate with
      more than one binary is refused instead.
    - A kind name that shadows no test target (such as `main`): rejected because `bin` is cargo's
      own word for the target (`--bin`) and no crate holds a `tests/bin.rs`; the shadow is refused
      by name instead.
    - Resolving against every file under `src/`, as `lib` does: rejected because a test of the
      library would then pass the census and read VOID only when proved (S03948 pins it).
    - The test path before `--` (`--bin <binary> <path> -- --exact`): rejected because the `lib`
      and `--test` kinds already pass it after `--`, and one function, `cargo_flags`, serves all
      three kinds.
    - A bin-only VOID rule: rejected because the generic rule (the control selects exactly one
      test) already covers a `bin::` killer; S03946 is a synthetic mutant that exempts `bin` from
      it.
- **Row `S05754` is the first user.** In `scripts/mutation-rows.d/S05700-S05799.json`: the anchor
  `const EXIT_GRACE: Duration = Duration::from_secs(1);` in `crates/daemon/src/main.rs`, the
  mutant `from_secs(2)`, and the killer `bin::tests::the_exit_grace_is_one_second`, the binary
  `deckstreakd`'s own unit test. It is the only id this delivery writes in SPEC-057's band (R20:
  each delivery writes only its allotted ids); SPEC-057 stays planned and is not amended here.
- **Seven rows pin the kind's decisions**, in `scripts/mutation-rows.d/S03900-S03999.json`, each
  proved KILLED:
  - `S03945`: `--bin <binary>` replaced by `--lib`, killed by A42's argv test;
  - `S03946`: the control's exactly-one-test rule skipped for a `bin` killer, killed by A43;
  - `S03947`: the binary named by the package instead of read from `[[bin]]`, killed by A42's
    manifest test;
  - `S03948`: the binary's sources widened to the whole `src/` directory, killed by A44;
  - `S03949`: the module walk cut off at the root file, killed by A44;
  - `S03950`: `file == root_file or ` removed from the module walk, so a root not named `main.rs`
    looks for its modules under a directory, killed by A45's module test;
  - `S03951`: the shadow check's condition replaced by `if False:`, killed by A45's shadow test.
- **What it does not change.** It adds no killer kind for a binary's integration tests, which
  stay `<target>::...` (#352). It changes no row, no band and no verdict logic other than S05754
  and the seven rows above (#352). It runs no cargo command outside the fixture crates of its own
  tests and the row S05754's proof (#352).

Issue #352 is closed by this delivery.

## 16. Acceptance criteria of the 2026-09-29 amendment

| id | criterion | decided by |
|---|---|---|
| A42 | a `bin::<path>` killer on a crate with a binary target resolves against the binary's sources and runs `cargo test --locked -p <package> --bin <binary> -- --exact <path>` (and `--no-run` with the same flags for the mutant's build), the binary read from the manifest and never guessed: the `[[bin]]` name, else the package's own, else a refusal when the crate holds more than one | `test_mutation_rows.py` |
| A43 | a `bin::` killer that names no test is VOID, never KILLED, and its mutant is never installed, while a `bin::` killer beside it that names a real test reads KILLED | `test_mutation_rows.py` |
| A44 | a `bin::` killer whose test is not in the binary's module tree is refused by the census with the existing sentence, while the killers in the binary's root file and in a module it declares resolve | `test_mutation_rows.py` |
| A45 | the module tree of a binary whose root file is not named `main.rs` is walked beside that root, as rustc reads a crate root, so a `bin::` killer in a module the root declares resolves; and a `bin::` killer on a crate that also holds `tests/bin.rs` is refused by name, never run against the binary's own test of the same path | `test_mutation_rows.py` |

```acceptance
A42: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_bin_killer_runs_cargo_test_on_the_binary_by_its_exact_path
A42: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_bin_killers_binary_is_read_from_its_manifest_and_never_guessed
A43: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_bin_killer_that_selects_no_test_is_void_and_never_killed
A44: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_bin_killer_outside_the_binarys_sources_is_refused_by_the_census
A45: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_module_beside_a_root_not_named_main_resolves
A45: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_bin_killer_beside_a_tests_bin_rs_is_refused
```

A42 to A45 run the runner in a fixture repository built at run time: a crate with a library, a
binary named other than its package, a unit test in the binary's root file and one in a module it
declares, and a library test the binary does not hold. A42's first test calls the runner's
functions with `subprocess.run` replaced by a recorder, and reads the argv it built. A43 proves two
rows with cargo in the fixture's own `target/`, and A44 runs the census over three planted rows. A45's two tests plant a binary rooted at `src/other.rs` with a module beside it, and a `tests/bin.rs` beside the binary, and read the census.

## 17. Amendment, 2026-09-29: a leg with nothing to examine is not started (SPEC-290)

R3 says each of the five jobs "is never skipped, because `ci` reads a skipped need as failed".
SPEC-290 (ADR-290, #435) makes that false for two of them, and only when the plan's listing gives
the leg nothing to examine:

- `mutation-rust` runs under `if: ${{ needs.mutation-plan.outputs.listed != '0' }}`. `listed` is a
  new plan output, written by `mutation-verdict.py shards`: the number of mutants the shards hold,
  `0` when the Rust class does not apply. The matrix, the shards and the `cargo mutants` line are
  unchanged.
- `mutation-rows` runs under
  `if: ${{ needs.mutation-plan.outputs.rows == 'true' || needs.mutation-plan.outputs.scope == 'diff' }}`,
  so R11's retirement check still runs on every diff, and the leg is not started only on a
  `not-applicable` push that selects no row.
- R4's verdict gains two readings: a shard the listing gives no mutant and that left no artifact is
  `not started`, and a sum of the reports' mutants that differs from the listing's count is VOID.
  Its new `legs` verb refuses by name a skipped leg the listing owed work.
- `ci` admits `skipped` from those two legs alone, once each, beside a `mutation-verdict` that must
  succeed. `mutation-plan`, `mutation-verdict` and `mutation-web` are still never skipped, and every
  leg that starts prints its case as R3 says.

This section adds no criterion: SPEC-290's A1 to A7 decide it, and its rows are S29000-S29099.

## 18. Amendment, 2026-09-29: the Python is mutated by a runner of its own

Made by SPEC-087's delivery (issues #218 and #219), insert-only under ruling (i) of SPEC-038
section 8: every earlier byte is kept in order. It adds:

- a Python class beside the Rust, the Mini App and the parity oracle: `scripts/*.py` (a guard
  script, never its tests) is the class `scripts`, and the oracle's Python is judged by the same
  runner;
- the job `mutation-python`, a need of `mutation-verdict` and of `ci`, which runs
  `scripts/mutation_python.py` over the diff's mutants, one job per shard, and the weekly
  battery's `python` job, which sweeps every listed file in 16 shards. `mutation-python` has no
  job-level condition and is never skipped by design, and `ci` admits no skip from it: only
  `mutation-rust` and `mutation-rows`, the two legs SPEC-290's listing can leave empty, may read
  `skipped`;
- `judge --class scripts` and `judge --class oracle`, each reading the shards' reports, where a
  report that is missing, partial or of exit 4 (a failed restore) is VOID by name;
- the equivalence record `scripts/mutation-equivalent.d/python.json`, held to the same census as
  the Rust and Mini App records (SPEC-057, ADR-070), and the rows `S08700-S08799`.

What it amends, and why: the Python that guards the repository was proved only by hand-proved
rows, so a weak test of a guard script had no measure. The decision and what it was chosen
against are ADR-073; the requirements and criteria are SPEC-087's. The criteria of this SPEC
stand; SPEC-087's A1 to A22 are added beside them.

## 19. Amendment, 2026-09-29: a missing tool is a refusal

Issue #431: `scripts/mutation_rows.py prove` ended with an uncaught `FileNotFoundError`, and exit
1, which is `EXIT_SURVIVED`, when a row's killer was a cargo test and `cargo` was not on `PATH`. A
run that could not start a check said "a mutant survived". ADR-291 decides the class, and this
section states it.

- **The rule.** Every process the runner spawns, under every verb that reaches it, ends the verb
  with ONE line naming the tool and exit 2 (`EXIT_REFUSED`) when its executable cannot be run,
  for any reason the operating system gives. Six modes are read: absent from `PATH`, present but
  not executable, a directory at the name, a script whose interpreter line names a missing
  program, an empty file with the execute bit (the kernel will not execute it), and a wrapper
  whose interpreter line resolves but whose program is missing (it starts and ends with exit
  127, or 126 behind a `PATH` entry the runner cannot search). The tool may sit alone in `PATH` or
  behind an entry the runner cannot look at (a name too long to stat), which is passed over as the
  spawn's own search passes over it. It is never a traceback, never exit 1, never a verdict line
  and never `KILLED`. A mutant that was installed
  is restored byte for byte, by digest, before the verb ends. The line reads
  `<verb>: REFUSED: missing tool: <name as spawned>: <why>`, with the verb `prove` or `retired`.
- **One place.** The executable is resolved in one place, before the spawn: `run_tool` for a
  command that is run to its end, and `run_in_own_group` for a killer. A spawn that still fails
  for its executable after resolution passed is mapped to the same refusal: both helpers catch
  every `OSError` of the spawn (an error that names the working directory is still re-raised), and
  both read exit 126 and exit 127 of the tool they spawned as the refusal, `cannot be run` and `is
  not found`. `main` alone turns the refusal into the line and the exit code. No other function
  spawns a process, and the census of the module's own source (A47, A51, A52) refuses a function
  that does, by any name the standard library gives a spawner (`subprocess`, `os.system`,
  `os.popen`, `os.exec*`, `os.spawn*`, `os.posix_spawn*`, `os.fork*`, `os.startfile`, `pty`,
  `asyncio`'s subprocess calls and the loop's `subprocess_exec` and `subprocess_shell`) and through
  every import that reaches one (`import subprocess as sp`, `from subprocess import run as r`,
  `from os import *`).
- **A missing parser joins the class.** Section 12 (A41) made a shell that is not installed leave
  the mutant unchecked and VOID. That clause, and only that clause, is superseded: a shell that
  cannot be run is the same fact as a missing killer tool, the runner could not run a check, so it
  is a refusal and the verb exits 2. A41's other readings stand unchanged: a parse check that
  outlives its bound is still VOID, a mutant that does not parse is still VOID, and `-n` still
  keeps the check from running the mutant. A41's text above is not edited.
- **Reading at the verdict.** Exit 2 writes no report, so `mutation-verdict.py` reads the leg as
  "selected and no rows report", which is VOID, and the weekly run's step fails on a non-zero
  exit. Nothing reads 2 as a pass or as a usage error to ignore (ADR-291 quotes each reader).
- **What it does NOT do.** It adds no tool to any leg's setup (#431), and it does not make the
  rows leg skip its toolchain, which is the later lever this refusal makes safe (#431). It changes
  no verdict logic other than a missing tool's, and no existing row other than S03935 (#431).

## 20. Acceptance criteria of the 2026-09-29 (#431) amendment

| id | criterion | decided by |
|---|---|---|
| A46 | for every spawn site of the runner, every tool it can spawn (`git`, `cargo`, the interpreter, `bash`, `sh`), every one of the six unrunnable modes and every verb that reaches the site, the verb exits 2, prints exactly one `REFUSED` line naming the tool, prints no traceback and no verdict line, and leaves the target's bytes and the tree's tracked state as they were; the population's count is printed | `test_mutation_rows_missing_tool.py` |
| A47 | the spawn sites read from the module's own source are exactly the sites the population covers, and no function outside `run_tool` and `run_in_own_group` spawns a process by any name (A51) | `test_mutation_rows_missing_tool.py` |
| A48 | `count`, `ids` and `census` spawn nothing and succeed with every tool unrunnable | `test_mutation_rows_missing_tool.py` |
| A49 | a shell parser that cannot be run refuses the proof naming it, exit 2 and no verdict, where section 12 left the mutant VOID | `test_mutation_rows.py` |
| A50 | a tool behind a `PATH` entry the runner cannot look at is found as the spawn finds it, and `prove` does not end in a traceback | `test_mutation_rows_missing_tool.py` |
| A51 | no function outside the two helpers spawns a process by any name the standard library documents for a spawner, or through an aliasing import | `test_mutation_rows_missing_tool.py` |
| A52 | the census reads every documented spawner, spelled every way of reaching it | `test_mutation_rows_missing_tool.py` |

```acceptance
A46: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_missing_tool.py -k a_tool_the_runner_cannot_run_is_a_refusal_at_every_site_mode_and_verb
A47: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_missing_tool.py -k every_spawn_site_the_module_holds_has_a_scenario_and_owns_no_raw_spawn
A48: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_missing_tool.py -k a_verb_that_spawns_nothing_runs_with_every_tool_unrunnable
A49: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k a_missing_parser_is_a_refusal_naming_it
A50: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_missing_tool.py -k a_path_entry_the_runner_cannot_look_at_is_passed_over_as_the_spawn_passes_it
A51: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_missing_tool.py -k no_code_outside_the_helpers_spawns_a_process_by_any_other_name
A52: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_missing_tool.py -k the_census_reads_every_documented_spawner_by_every_way_of_reaching_it
```

A46 to A48 build a temporary git repository per member with the suite's own fixture, and run the
runner as a child process whose `PATH` holds real `git`, `bash` and `sh` except the tool under
test, which is in one of the six modes above (the interpreter's case replaces `sys.executable` in
the child). The population is 180 members: every site, tool, mode and verb alone in `PATH`, and
the searched tools again behind the entry the runner cannot look at. The cargo build site's member serves the control run from
a shim that then makes itself unrunnable, so the refusal comes from the mutant's build and the
member asserts the shim was reached. A49's test keeps A41's fixture and its bare `PATH`.

## 21. Amendment, 2026-09-30: every refusal is read whole, and the census refuses what it cannot read

Round 1 of #431 made the runner refuse a tool it cannot run, and CI's `mutation-verdict` then
generated mutants of the refusal's own lines: 43 generated and 23 hand rows examined, 14 survived,
none explained. The tests asserted that a refusal happened, or that its line held a word of the
reason, so a mutant of the text or of the branch that chose it read the same. ADR-291 states the
class; this section states it as criteria.

- **The rule.** Every refusal is decided by a whole-value assertion. Its exact text, `missing tool:
  <name as spawned>: <why>`, and the tool it names are compared whole, and each branch that chooses
  it is selected by a test that fails if the branch changes: the empty and `.` `PATH` entries, a
  name holding `/`, a candidate that is absent, a directory or not executable, the child's `PATH`
  (and not this process's), an `OSError` of every errno the operating system names, at both
  spawns, whether it names the tool, nothing or something else (the working directory), and every
  exit from 0 to 255 at every helper, of which 126 and 127 are the refusal and no other is.
- **The population is generated.** Reasons, errnos (`errno.errorcode`) and exits are read from
  their own tables and crossed with the spawn routes, and the member counts are printed and
  asserted. The tests are in `test_mutation_rows_refusal.py`, which the map in
  `scripts/mutation-python.json` runs first against every generated mutant of the runner.
- **No mutant is declared equivalent.** `pathlib.Path(part or ".")` named the working directory
  twice, because `Path("") == Path(".")`, so replacing `"."` with `""` changed no path and no test
  could tell them apart. The runner now reads `pathlib.Path(part)`: an empty `PATH` entry is the
  working directory by pathlib's own reading, and the test that pins both spellings stays green.
  The equivalent mutant is removed rather than recorded (#431).
- **The census refuses a spawner it cannot read.** A spawner reached by a name built at run time
  is a spawn no reading of the source can see. The census refuses the way of reaching one, wherever
  it appears and whatever it is given, and does not list spellings: the names `getattr`, `vars`,
  `globals`, `locals`, `eval`, `exec`, `compile`, `__import__`, `import_module` and
  `__builtins__`; the attributes `__import__`, `import_module`, `__dict__`, `__builtins__`,
  `__globals__` and `modules` (so `sys.modules[...]` too); and any import of `importlib`,
  `builtins`, `imp`, `runpy`, `code` or `codeop`, or of `modules` from `sys`. A test crosses 17 such
  forms with all 34 documented spawners, 578 members, and asserts every one refused, and that the
  runner's own source and four benign sources are not.
- **What it does NOT do.** It does not read a spawn built from a string handed to a shell by a
  caller outside `scripts/mutation_rows.py`; the census reads that one file, as A47 does (#431). It
  changes no timeout, drops no test and narrows no mutation diff (#431).

## 22. Acceptance criteria of the 2026-09-30 (#431) round 2

| id | criterion | decided by |
|---|---|---|
| A53 | every refusal's whole text and tool, and each branch that chooses it, at every spawn route, for every reason, errno and exit | `test_mutation_rows_refusal.py` |
| A54 | the census refuses every way of reaching a spawner by a name built at run time, for every documented spawner, and refuses nothing the runner's source holds | `test_mutation_rows_missing_tool.py` |

```acceptance
A53: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_refusal.py -k each_reason_is_refused_whole_at_every_route
A54: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_missing_tool.py -k the_census_refuses_every_spelling_it_cannot_read_by_every_way_of_reaching_a_spawner
```
