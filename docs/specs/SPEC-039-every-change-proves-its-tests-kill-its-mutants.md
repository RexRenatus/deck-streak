# SPEC-039: every change proves its tests kill its mutants

- **Wave:** W0. **Issue:** #217 (epic #1). **Context(s):** `repo` (`scripts/`, `.github/`,
  `.cargo/`, `.packs/`, `web/app`'s configuration, `docs/`), and `deck-streak-kernel` for the one
  survivor the first measurement found (§1).
- **Decided by:** ADR-057 (this SPEC's own: the tools, the diff-scoped jobs, the rows and the
  weekly battery), ADR-012 (the testing strategy and the parity oracle), ADR-017 (hosted CI on
  pull requests into `dev`), ADR-055 (the gate's parallel CI jobs, beside which the mutation jobs
  run), ADR-004, ADR-039 and ADR-056 (the vendored packs and the script that vendors them), and
  ADR-029 (the one golden reader).
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
- **Copying the tree against building in place.** cargo-mutants over `clock.rs`, `-j 1`: the
  default copy took 41 s, 21 s of it a cold build in a temporary directory; `--in-place` took
  34 s cold (leaving a 1.5 GB `target/`) and 17 s warm. With Anki's engine in the workspace
  (SPEC-022) a cold build of `ingest` is about 5 minutes and 15 to 18 GB, which the copy pays on
  every run.
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
R3. **A diff-scoped run on every pull request into `dev` and `main`.** Two CI jobs, each a need of
    the aggregate `ci` job, run on every event and decide from the diff what applies:
    - `mutation-rust`: `cargo mutants --in-place --in-diff` over the pull request's diff
      (`HEAD^1...HEAD` of the merge ref), then the rows the diff selects (R10), then the
      retirement check (R11), then the verdict (R4);
    - `mutation-web`: StrykerJS over every web production file the diff changes, whole, then the
      verdict.
    Each job reads its event's case by name, from `scripts/mutation-verdict.py plan --event`, and
    is never skipped, because `ci` reads a skipped need as failed:
    - a pull request into `dev` is judged on its diff;
    - a release pull request into `main` reads `not-applicable`: it carries `dev`'s changes, each
      judged by these jobs on its own pull request into `dev`, and the weekly battery sweeps `dev`;
    - a push whose subject names the pull request it merges (`Merge pull request #N`) reads
      `not-applicable`, naming `#N`, whose jobs judged that same tree: `dev` and `main` accept a
      pull request only with an up-to-date head (ADR-034);
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
    - A missing or unreadable report on a class that applies is VOID, never zero.
R5. **Survivors and exclusions.** A surviving mutant blocks the pull request until a test kills
    it, or until it is recorded as equivalent. An equivalent Rust mutant is one anchored
    `exclude_re` entry in `.cargo/mutants.toml`, with `# EQUIVALENT: <reason> (#N)` on the line
    above; a Mini App one is `// Stryker disable next-line <mutator>: EQUIVALENT: <reason> (#N)`.
    A test refuses any exclusion without a reason and an issue, and any `mutants::skip`
    attribute. Nothing is excluded silently.
R6. **The configurations** load under their tools' own rules: `.cargo/mutants.toml` holds only
    cargo-mutants 27.1.0's keys, and `web/app/stryker.config.json` sets `testRunner` `vitest`, the
    `json` reporter and `thresholds` whose `break` is 100. The vendored probe's blocking
    `tool-config-valid` row judges both.
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
    - `scripts/mutation_rows.py` is the one reader. `scripts/row_target.py` resolves a target.
      Both keep the pack's interface, so the vendored `mutation-probe.py` judges the rows with its
      `find-differs`, `band-ids` and `mutants-distinct` classes.
R9. **The runner, `scripts/mutation_rows.py prove`,** proves each row it is given:
    - it refuses a tree with a tracked change (exit 2), naming the file;
    - it checks the anchor occurs exactly once, and records the target's sha256;
    - it runs the killer on the unmutated tree, which must pass, selecting exactly one test;
    - it installs the mutant once, and treats a mutant that does not build (`cargo test
      --no-run`) or does not parse (Python) as VOID, never a kill;
    - it runs only the killer, counting the tests selected from libtest's `running N test` lines
      or unittest's `Ran N test` line, and anything but exactly one is VOID;
    - a killer that fails with the mutant installed is KILLED; one that passes is SURVIVED;
    - it restores the saved bytes and re-checks the sha256 before anything else runs;
    - it prints one line per row and `examined N`, and exits 0 when every row was KILLED, 1 on a
      survivor, 3 on a VOID with no survivor.
R10. **The rows a diff selects:** every row whose target the diff changes, every row the diff adds
    or changes, and every row whose killer's file the diff changes. The `mutation-rust` job proves
    each; a row that is not KILLED fails the job.
R11. **No weakening.** A row whose id leaves the population while its target file stays fails the
    `mutation-rust` job (`scripts/mutation_rows.py retired --base <ref>`), unless
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
    - every job keeps its report under `if: always()`;
    - `survivors`: files each surviving mutant's file as one issue, deduplicated against the open
      issues by title, its body scrubbed by `scripts/public-scrub.py` before it is posted, with
      `issues: write` granted to that job alone and never on a pull request.
R13. **The known lag.** The schedule and the dispatch go live only when a release carries the
    workflow to `main`. Until then, a `pull_request` trigger filtered to the workflow's own file
    runs `rehearsal`: one small file through cargo-mutants, one row, one Stryker file, and the
    survivors job's drafting and scrub, with no issue filed.
R14. **The mutation-rows pack is vendored** through `scripts/vendor-packs.py` at the commit
    `.packs/VENDORED.json` already pins, and wired `enforced` in `.packs/wiring.json`: its
    `practice` rows run, `tool-config-valid` blocking; its `rows` rows that run the pack author's
    own guards are excluded by name with that reason; `find-differs`, `band-ids` and
    `mutants-distinct` run over DeckStreak's rows.
R15. **`docs/BUILDER-BRIEF.md` gains a mutation section,** so every builder inherits R3 to R11.
R16. **The first rows** guard invariants that exist on `dev`: the kernel's 04:00 rollover and
    the redactor's constants (SPEC-020), ingest's once-a-study-day refusal and its no-upload rule
    (SPEC-022), identity's constant-time compare, the `auth_date` bound, the owner pin and the
    cookie flags (SPEC-024), the parity oracle's golden comparison (SPEC-029), and the vault's
    atomic write and topic key (SPEC-042). Each is proved by the runner.
R17. **The gate is proved red first.** The survivor in `SystemClock::now` (§1) is moved into a
    pure conversion, `UtcMillis::from_system_time`, beside a test that leaves it alive: the
    `mutation-rust` job goes RED on the pull request. A test that a time before the epoch reads as
    negative milliseconds then kills it, and the job goes GREEN. Both run ids are recorded.

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
| A20 | every exclusion names its reason and an issue, and no `mutants::skip` exists | `test_mutation_workflows.py` |
| A21 | the tools' configurations load under their own rules, judged by DeckStreak's own check | `test_mutation_workflows.py` |
| A22 | the weekly battery's shards cover their denominator and keep every report | `test_mutation_workflows.py` |
| A23 | only the survivors job may write issues, and never on a pull request | `test_mutation_workflows.py` |
| A24 | the mutation jobs are needs of `ci`, install pinned tools and save no cache | `test_mutation_workflows.py` |
| A25 | the builder brief teaches the mutation rules | `test_mutation_workflows.py` |
| A26 | a time before the epoch reads as negative milliseconds | `cargo test -p deck-streak-kernel --test clock` |
| A27 | each event reads its case by name, and a diff with no production path names its paths | `test_mutation_verdict.py` |
| A28 | every job that runs cargo-mutants installs the nextest its configuration names | `test_mutation_workflows.py` |
| A29 | the weekly battery fails, naming each shard, the rows report and the Stryker report it lacks or holds only in part | `test_mutation_verdict.py` |
| A30 | a report the tool left partial is VOID: an exit other than 0, 2 or 3, or counts short of its total | `test_mutation_verdict.py` |
| A31 | a comment opener inside a string, a raw string, a character, a template or a regular expression opens no comment | `test_mutation_verdict.py` |
| A32 | every cargo-mutants command bounds its builds and its tests | `test_mutation_workflows.py` |
| A33 | the survivors job counts every report its jobs promise, whatever they returned | `test_mutation_workflows.py` |

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
A20: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k every_exclusion_names_its_reason_and_issue
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
```

A1 to A8 run the runner against a fixture repository built at run time in a temporary directory:
a git repository with one committed target, one unittest module, and (for A8) one crate with no
dependency and its own `Cargo.lock`, built into the fixture's own `target/`. A13 to A19 run the
verdict over synthetic diffs and reports written by the test, never over a real tool run. A9 and
A11 read the committed rows. A21 runs the vendored probe's `tool-config-valid` class over the tree
and reads its examined count.

The red stubs, committed with the tests, keep every entry point and do nothing: the runner reads
every row KILLED without running anything, the census and the retirement check examine nothing,
and the verdict reads every class green. So each criterion fails by assertion for its own reason.
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
| `scripts/mutation_rows.py` | `repo` | added: the reader, the census, the runner and the retirement check (R8 to R11) |
| `scripts/row_target.py` | `repo` | added: the target resolver the pack's interface names (R8) |
| `scripts/mutation-verdict.py` | `repo` | added: the plan, the verdict and the survivors' drafts (R3, R4, R12) |
| `scripts/mutation-rows.json` | `repo` | added: the header (R8) |
| `scripts/mutation-rows.retired.json` | `repo` | added: the retirement record, empty (R11) |
| `scripts/mutation-rows.d/S02000-S02099.json` | `repo` | added: SPEC-020's rows (R16) |
| `scripts/mutation-rows.d/S02200-S02299.json` | `repo` | added: SPEC-022's rows (R16) |
| `scripts/mutation-rows.d/S02400-S02499.json` | `repo` | added: SPEC-024's rows (R16) |
| `scripts/mutation-rows.d/S02900-S02999.json` | `repo` | added: SPEC-029's rows (R16) |
| `scripts/mutation-rows.d/S03900-S03999.json` | `repo` | added: this SPEC's rows on its own runner and verdict (R16) |
| `scripts/mutation-rows.d/S04200-S04299.json` | `repo` | added: SPEC-042's rows (R16) |
| `scripts/tests/test_mutation_rows.py` | `repo` | added: A1 to A11 |
| `scripts/tests/test_mutation_verdict.py` | `repo` | added: A12 to A19, A27, A29 to A31 |
| `scripts/tests/test_mutation_workflows.py` | `repo` | added: A20 to A25, A28, A32, A33 |
| `.cargo/mutants.toml` | `repo` | added (R5, R6) |
| `web/app/stryker.config.json` | `repo` | added (R6) |
| `web/app/package.json`, `pnpm-lock.yaml` | `miniapp` | changed: StrykerJS 10.0.0 (R1) |
| `.gitignore` | `repo` | changed: the tools' output directories |
| `.github/workflows/ci.yml` | `repo` | changed: `mutation-rust` and `mutation-web`, needs of `ci` (R3) |
| `scripts/tests/test_ci_workflows.py` | `repo` | changed: `ci` needs the two mutation jobs beside the gate's four (R3) |
| `.github/workflows/mutation-weekly.yml` | `repo` | added (R12, R13) |
| `.packs/VENDORED.json`, `.packs/wiring.json` | `repo` | changed: the mutation-rows pack (R14) |
| `.packs/skills/packs/mutation-rows/SKILL.md`, `.packs/skills/packs/mutation-rows/checks.json`, `.packs/scripts/mutation-probe.py` | `repo` | added: vendored (R14) |
| `crates/kernel/src/clock.rs` | `deck-streak-kernel` | changed: `UtcMillis::from_system_time` (R17) |
| `crates/kernel/tests/clock.rs` | `deck-streak-kernel` | changed: A26 (R17) |
| `changelog.d/feat-mutation-039.md` | `repo` | added |

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
- **The Rust run's time grows with the diff.** One serial `--in-place` run pays about 10 to 20 s a
  mutant, and `--in-place` cannot run jobs in parallel. `timeout-minutes` bounds it, and the job's
  log names the count it reached. A diff large enough to reach the bound shards the job the way the
  weekly battery is sharded, every shard reading the same diff.
- **A flaky test makes a survivor or a kill flaky.** cargo-mutants refuses a red baseline (exit
  4, VOID here), and the runner's control run refuses a killer red without its mutant (R9).
- **The weekly battery files noise.** Each issue is one file's survivors, titled by the file, and
  a title already open is not filed twice.

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
