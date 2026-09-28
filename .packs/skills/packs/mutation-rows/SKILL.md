---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/mutation-rows

Mutation testing, in two halves. The first, "Mutation testing in any repository", is the practice
any repository runs: choosing and configuring a tool, running it on the change and over the whole
tree, reading its outcomes, and triaging what survives. Each of its rules comes from a tool's own
documentation or a published study (SPEC-V2-2208). The rest is phoenix-v2's craft for its
hand-written rows: how to write a row that can be killed, and how to read a kill. SPEC-V2-2162
moved those rules out of the orchestrator's measured memory, so a builder reads them before the
gate teaches them again. Every rule carries the measurement or the source that earned it.
Standards Librarian owns the bar text. Which seats consume this pack is its catalog row's
`consumes`, the one record of that edge (ADR-V2-1990), so this body names none.

The builder seat already teaches the mechanics every PR-path delivery uses: insert rows beside
their file's rows as text (its PR-path item 4), select each killer the way `mutants-diff` runs it
(item 5), prove the rows `mutants-diff` will not re-select (item 6), and audit the anchors staged
(item 7). Its "What builders measured" list holds the row bullets too. This pack does not repeat
them. It holds the rest: why a row that reads right proves nothing.

## The one rule

A row proves something only when its killer observed its mutant. Every trap below is a verdict
decided by something other than the mutant: a killer that ran nothing, a mutant that never
compiled, an assertion the mutant cannot move, or a tree the runner was not measuring. For each
row ask which assertion observed the mutant, and whether it could have failed for another reason
(S18808, below).

## Mutation testing in any repository

The sections after this one are phoenix-v2's craft, in which each mutant is a row written by hand.
Most repositories have a tool generate their mutants instead, and this section is the practice for
them. The pack's `practice` stage checks it on any tree, and
`python3 scripts/mutation-probe.py --root <repo> walk` runs that stage alone. The owner's rule for
its severity is "Split by evidence": a row blocks only what the tool itself refuses, and every
judgement (a score, a cadence, an equivalent mutant, a missing CI job) is advisory.

### What a score measures

- A mutant is a small seeded fault. A test suite that fails on it has detected it. Mutation analysis
  is "one of the strongest test-adequacy criteria" (Petrović, Ivanković, Fraser and Just, *Practical
  Mutation Testing at Scale*, IEEE TSE 48(10), 2022).
- Detecting mutants correlates with detecting real faults, independently of coverage: 357 real
  faults in 5 applications (Just et al., FSE 2014). Controlled for test-suite size the correlation
  is weak, and yet the suites that score higher still find more faults (Papadakis et al., ICSE 2018).
  So read a score as a pointer to the next test worth writing, never as a verdict that a suite is
  adequate. That is why `score-threshold` advises and never refuses.
- Redundant mutants inflate a score. Mixing subsuming and subsumed mutants made about 62% of
  mutation-based comparisons liable to a false conclusion (Papadakis et al., ISSTA 2016). Compiling
  each mutant and comparing object code found more than 7% of C mutants equivalent and 21%
  duplicated (Trivial Compiler Equivalence, ICSE 2015). Compare scores only within one tool, one
  operator set and one code base.
- Equivalent mutants "cannot generally be detected automatically" (Petrović et al. 2022), though a
  few classes can: a `size()` compared below zero, a memoisation cache lookup. A survivor may be one,
  which is why a triaged survivor records its reason.
- Developers who were shown mutants wrote more tests, and fewer mutants survived their later changes
  (Petrović et al., ICSE 2021).

### Choose the tool by language

| language | tool, as read on 2026-09-27 | configuration | change-only mode | report |
|---|---|---|---|---|
| Rust | cargo-mutants 27.1.0 | `.cargo/mutants.toml` | `--in-diff FILE` | `mutants.out/outcomes.json` |
| JavaScript, TypeScript, Svelte | StrykerJS 10.0.0 | `stryker.config.json` or `stryker.config.mjs` | `--incremental` | `reports/mutation/mutation.json` |
| Python | mutmut 3.8.0, or cosmic-ray | `[tool.mutmut]`, or a `[cosmic-ray]` TOML | none | `mutmut results`, `cr-report` |
| JVM | PIT | the `pitest-maven` plugin | `pitest:scmMutationCoverage` | XML, HTML or CSV |

StrykerJS writes the mutation-testing-elements report, whose JSON schema (3.9.0) is the one
published report format across languages. `report-schema` reads it, and cargo-mutants'
`outcomes.json` too, whose format its docs call "subject to change".

### Configure it so the tool accepts it

`tool-config-valid` is the practice stage's one blocking row, because each state it refuses is one
the tool itself refuses:

- cargo-mutants reads `.cargo/mutants.toml` with `deny_unknown_fields`, so an unknown key
  (`exclude_glob` for `exclude_globs`) or a value of the wrong type refuses the whole file (its test
  `invalid_field_rejected`). The keys are those its `src/config.rs` declares in 27.1.0, plus
  `test_tool` (`cargo` or `nextest`) and `sharding` (`slice` or `round-robin`). In a glob, `*` stops
  at `/` since 24.3.0, so write `**` to cross directories.
- StrykerJS reads a `.json` config with `JSON.parse`, which allows no comment: put a note in a
  `_comment` key. `thresholds` holds only `high`, `low` and `break`, each from 0 to 100, and `break`
  may be null. `high` below `low` is a ConfigError once the defaults, high 80 and low 60, are
  applied, so `{"high": 50}` alone is refused. `ignoreStatic` is refused too unless
  `coverageAnalysis` is `perTest`, its default.
- A StrykerJS JavaScript config exports an object as its default. Exporting a function
  (`module.exports = function (config) { config.set(...) }`) has been refused since 6.0.0
  (2022-05-03), and so has an ES module with no default export.

`tool-config-current` advises on what a tool only warns about:

- StrykerJS's deprecated options: `maxConcurrentTestRunners` (use `concurrency`), `files` (use
  `ignorePatterns`), `htmlReporter.baseDir` (use `htmlReporter.fileName`), `jest.enableBail` (use
  `disableBail`), `testFramework`, `transpilers` and a string `mutator`;
- mutmut 3.6.0 (2026-06-06) renamed `paths_to_mutate` to `source_paths` and deprecated `tests_dir`;
  mutmut also warns that a `do_not_mutate` pattern ending in neither `*` nor `.py` is likely invalid;
- PIT's `maxMutationsPerClass` gave way to the feature `+CLASSLIMIT(limit[n])`.

StrykerJS 10.0.0 (2026-08-14) needs Node.js 22 or later. It has mutated `.svelte` files since 8.0.0
and supports Svelte 5 and Vitest 2 and later since 9.0.0.

### Run it on the change, and sweep the whole tree

- **Mutate the change under review.** Google runs mutation testing on each diff at code review and
  surfaces at most one surviving mutant per line, after suppressing unproductive ones. That gave a
  median of 7 mutants per changelist against 820 for traditional mutagenesis, and 82% of the
  surfaced mutants that drew feedback were labelled productive (Petrović et al. 2022; 75% useful in
  Petrović and Ivanković, *State of Mutation Testing at Google*, ICSE-SEIP 2018). `ci-diff-run` asks each tool in
  use for its change-only mode on a `pull_request` trigger: `cargo mutants --in-diff`,
  `stryker run --incremental`, `pitest:scmMutationCoverage`, or phoenix-v2's `mutants-diff`. A tool
  with no such mode (mutmut, cosmic-ray) only has to run there. The cargo-mutants recipe checks out
  with `fetch-depth: 0`, writes `git diff origin/${{ github.base_ref }}.. | tee git.diff`, then runs
  `cargo mutants --no-shuffle -vV --in-diff git.diff`.
- **A diff run is not a sweep.** `--in-diff` matches the diff against the code under test only. A
  change to tests alone runs no mutant, and an edit can leave another region less tested
  (mutants.rs). `ci-full-run` asks for one whole-tree run of each tool somewhere in CI, on a
  schedule or a dispatch.
- **A shard split must cover the population.** `--shard k/n` counts from zero, so
  `--shard ${{ matrix.shard }}/8` needs the matrix `[0, 1, 2, 3, 4, 5, 6, 7]`. A matrix of
  `[1, ..., 8]` never runs shard 0, and nothing fails. `shards-complete` reads every split whose
  denominator is a number. When `--in-diff` is sharded, every shard must see the same diff.
- **Keep the report when the run fails.** A missed mutant fails the job (cargo-mutants exits 2),
  which is when the report is needed. `report-kept` asks every job that runs mutants for an
  `actions/upload-artifact` step under `if: always()`, carrying `mutants.out` or Stryker's `reports/`.

### Timeouts, flaky tests and a green baseline

- cargo-mutants times each mutant from its unmutated baseline run, with a floor of 20 s. Under
  `--baseline=skip` there is no baseline, and under `--in-place` the multiplier options do not apply,
  so without `--timeout` the test timeout falls back to 300 s. `timeouts-bounded` asks a run that
  skips the baseline, or sets a multiplier in place, for `--timeout`. `--timeout` and `--timeout-multiplier` conflict (24.5.0). Skip the baseline only in a
  job that `needs:` a green test job.
- A timeout is detected, not survived: mutation-testing-elements counts it as detected, while
  cargo-mutants exits 3 and advises investigating it. StrykerJS allows `timeoutMS` 5000 plus a
  `timeoutFactor` of 1.5, and PIT a factor of 1.25 plus 4000 ms.
- A flaky test makes the score flaky. Repeated runs moved scores by about 4 points and left 9% of
  mutant-test pairs unknown (Shi, Bell and Marinov, ISSTA 2019). cargo-mutants refuses a red
  baseline (exit 4), and its docs say to fix an intermittent failure first. Fix the flake; never widen
  a timeout to hide it.

### Read the outcome words right

| what happened | cargo-mutants | mutation-testing-elements | phoenix-v2 rows |
|---|---|---|---|
| a test failed | caught | `Killed` | KILLED |
| every test passed | missed | `Survived`, or `NoCoverage` when no test reached it | SURVIVED |
| the mutant did not compile | unviable | `CompileError` | VOID |
| the tests ran past the timeout | timeout | `Timeout` | no such outcome |
| it was never run | not listed | `Pending`, `Ignored` | not selected |

An unviable mutant is not a kill: it is "inconclusive about test coverage" (mutants.rs).
mutation-testing-elements scores detected over valid mutants, and `CompileError` and `RuntimeError`
are invalid. phoenix-v2's runner reads the same case as VOID ("Mutants that cannot be judged").

### Triage every survivor

- **First look for the test.** A survivor names a behaviour no test observes. cargo-mutants' advice
  is to ask what public behaviour would break, and to "assert the correct behavior at the right level
  of abstraction", never to write a test aimed at the one mutant.
- **An exclusion carries its reason beside it.** When a mutant is equivalent or unproductive, exclude
  it where the tool reads exclusions, with the reason on the same line or the line above.
  `exclusions-justified` reads each form:
  - `#[cfg_attr(test, mutants::skip)] // returning false would hang the poll loop`, from the
    `mutants` crate. The attribute is honoured at any scope, and its condition is never evaluated.
  - an `exclude_globs`, `exclude_re` or `skip_calls` entry in `.cargo/mutants.toml`, with a `#`
    comment;
  - `// Stryker disable next-line EqualityOperator: <= and < agree when a equals b`, where the text
    after the colon reaches the report as the mutant's `statusReason`;
  - `# pragma: no mutate` (mutmut and cosmic-ray), with the reason on the line above.
- **An unproductive class belongs in configuration, once.** Google calls such code arid: logging,
  capacity hints, memoisation caches, time and deadline settings, and its expert rules for it grew
  past a hundred from developers' "Not useful" feedback. cargo-mutants skips `with_capacity` calls by
  default and takes more in `skip_calls`; PIT's `avoidCallsTo` skips logging by default.
- **Leave nothing unexamined in a report.** No mutant stays `Survived`, `NoCoverage` or `Pending`,
  no `Ignored` mutant goes without a reason, and nothing stays missed in `outcomes.json`
  (`survivors-triaged`).

### Score gates

- StrykerJS exits 1 below `thresholds.break`, and `break` is null by default, so until it is set a
  run passes at any score (`score-threshold`). cosmic-ray's gate is `cr-rate --fail-over`, and PIT's
  is `mutationThreshold`. cargo-mutants has no score: any missed mutant exits 2, so an `--in-diff` run
  is already a full gate on the changed code.
- A report scoring below its own `low` threshold is advisory too. Where a threshold belongs is
  judgement, so this pack never picks the number.

### Adopting it in a Rust and SvelteKit repository

For a repository like DeckStreak, with a Rust workspace, a SvelteKit front end in `web/` and
GitHub-hosted CI:

1. Commit `.cargo/mutants.toml` with a reason beside every exclusion. Add `@stryker-mutator/core`
   and `@stryker-mutator/vitest-runner` to the front end, with `web/stryker.config.json` setting
   `"testRunner": "vitest"`, the `json` reporter, and `thresholds` with a numeric `break`.
2. In a `pull_request` workflow, add one job that runs
   `cargo mutants --no-shuffle -vV --in-diff git.diff` after the diff step, and one that runs
   `pnpm exec stryker run --incremental` in `web/`. End each with `actions/upload-artifact` under
   `if: always()`.
3. In a scheduled workflow, after a green test job, run
   `cargo mutants --no-shuffle -vV --shard ${{ matrix.shard }}/4 --baseline=skip --timeout 300 --in-place`
   over the matrix `[0, 1, 2, 3]`, and a full `stryker run`, each keeping its report.
4. Check the result with `python3 <phoenix-v2>/scripts/mutation-probe.py --root . walk`. It exits 1
   only when `tool-config-valid` finds something, and it prints every advisory finding. The same
   classes are the pack's `practice` rows; its `rows` rows read red on such a tree, because they
   judge phoenix-v2's own manifest.

## The rows file

`scripts/mutation-rows.json` is `{_, arities, tables}`, and the rows live under `tables`. The
TABLE decides the layout, never the arity, because all three tables hold rows of arity 7:

| table | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| `MUTATIONS` | crate | file, relative to the crate | find | replace | killer | description |
| `CARGO_KILLED_SCRIPT_MUTATIONS` | path from the root | find | replace | killer crate | killer | description |
| `SCRIPT_MUTATIONS` | path from the root | find | replace | description | killer | axis |

Cell 0 is the stem. So a reader that joins `MUTATIONS` cell 1 as a path from the root finds no
file, and a census that builds every target as `crates/<crate>/<rel>` mis-files the repo-rooted
tables: one read 92 anchored rows where the per-table rule gave 105 (2026-09-22). Resolve the
target per table.

Since d2170, a new delivery's rows go in its own band fragment,
`scripts/mutation-rows.d/S<lo>-S<hi>.json`, never in the monolith directly. A fragment is
`{"tables": {...}}` only — no `_` and no `arities` key, both of which live solely in the monolith
— and only a re-anchor of an EXISTING row, never a new one, touches `scripts/mutation-rows.json`
itself.

A stem is usually `S<n>-<NAME>`, but not always: at `0534cb0b8` the file held 2,217 compound
stems, 13 bare `S<n>` stems and 304 legacy stems such as `M1-DEFERRED-BEGIN`. `grep -oE '"S[0-9]+"'`
sees only the 13, and `'"S[0-9]{4,6}-'` misses them. Take the band frontier with the prefix
pattern `'"S[0-9]{3,6}'`, and verify a merged file by its delta against main, never by a count.

Write the file with `ensure_ascii=False`. It stores em dashes as UTF-8, and a default
`json.dumps` re-escapes about 60 lines of them: every test still passes and the diff doubles
(d1220). Inside `scripts/tests` every reader goes through `_rows` (`scripts/tests/_rows.py`): a
test that opens the file itself passes alone and fails under `run.py`, whose audit hook names the
line (`unguarded read`). A row on `config/seal.json` anchors on its entry's `"path": ` line, never
on a digest a vote rewrites (`test_seal_digest_anchors.py`).

## Merging the rows file

The file conflicts at the tail of each table a delivery appended to, so an absorb of main gives
one junction per table you wrote into (d1650: two tables, two junctions). Two conflict shapes need
two repairs. In an append collision the block above the `=======` ends mid-row and the block below
begins mid-row: insert `],` then `[` at that junction and nowhere else. A same-row rewrite shows a
complete row on both sides with one id: take one side and re-derive the cell, as `S16743`'s seal
digest was re-derived after the export (#1568).

Verify a resolution by content, never by its exit code:
- reconcile each table as base, less both sides' deletions, plus both sides' additions;
- census the whole file by id, because a row missing from one table may have MOVED to another
  (`S16622` to `S16625` moved into `CARGO_KILLED_SCRIPT_MUTATIONS`, d1681);
- check the arity of every row that moved;
- compare BYTES. A parse-and-reserialise merge put `"tables"` first; the real key order is `_`,
  `arities`, `tables`, and `compose::a_no_op_manifest_union_is_byte_identical` went red at byte
  offset 5 with the length unchanged (#1522).

The resolution to prefer takes main's side whole and replays your own rows onto it by id.

## Anchors

A `find` is a raw substring, leading whitespace included. Wrapping existing code in a new `if` or
`match` re-indents every line inside it and orphans every row anchored there, and a three-way merge
applies that re-indent with no conflict marker (d66: `S1923` read 0 finds while the tool reported
success). Add new behaviour as an insertion between existing statements, never as a wrapper around
one.

The census covers every row on a file you touch, not only your rows. Two builders on 2026-09-21 each
broke a row that was not theirs: a comment between two lines one anchor needed contiguous (`S16521`), a
re-indent from 16 to 12 spaces (`S16721`), a byte-identical new line that made `S16719` ambiguous.
Fix your code, never their row: re-anchoring another delivery's row edits its evidence to fit your
prose. Anchors overlap, so one installed mutant can zero two rows (`S1902` and `S1903`): a census
taken while a sweep runs cannot count mutants by its zeros.

rustfmt can move an anchor that nobody edited. A method chain wider than rustfmt's `chain_width`
(60 columns by default) is split one call per line, so an edit that lengthens a chain re-wraps it and
orphans a `find` written across its old layout. Anchor on a short statement instead: bind the
closure to a name, or extract the chain into a helper, and anchor on that line. Write the `find`
after `cargo fmt`, never before it.

## Selecting a killer

Every runner applies ONE selection policy, `mutants::selection_refusal_for` (SPEC-V2-1657,
SPEC-V2-1730): a killer that selects no test or several tests VOIDs its row. The count is read from
the killer's OUTPUT, the libtest `running N test` lines and the unittest `Ran N tests in` lines,
summed. So the builder seat's `--list` count is necessary and not sufficient: a failure message that
quotes a child run prints a second summary line, and `S19959` VOIDed as selecting 2 tests on a
killer that `--list` counted once (d1966).

- **A bare name can live in two binaries.** `cargo test -p phxd -- --exact <bare name>` walks every
  phxd target, and `default_is_dry_run_and_names_aol_dev` was defined in two seed test files, so
  `S12080` selected 2 tests and VOIDed with exit 3 (#1426). Nothing enforces a bare name's
  uniqueness.
- **A rename moves every row that names the test, in the same commit.** Renaming one side of that
  pair took its killer from two tests to none. Census the rows, and the SPEC tables that cite the
  name under `--exact`, before renaming.
- **An ignored test can never kill.** `#[ignore]` still lists and still prints `running 1 test`, so
  the row is selected, the test reports `ignored`, the killer exits 0 and the mutant survives.
- **The killer cell sits beside prose.** A killer and its description swapped keep the row's arity,
  so no bar sees it, and the runner selects nothing (d1631: six rows). Screen a cargo killer cell
  with `^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)*$`, and a script killer, which is a dotted
  unittest path, with the same shape split on `.`.
- **A new row's killer lives in the row's own crate and compiles at the merge-base.** The row's
  crate cell decides both where its mutant goes and the killer's `-p` scope, so a killer defined in
  another crate selects nothing (i1578), and `killer-selection` refuses it by shape. A killer that
  names a symbol the merge-base lacks makes its delivery compile-arm, and a train composes at most
  one such member: drive the test through the binary or an existing public API instead.
- **Splitting a killer can select two tests.** A script killer is a `-k` substring pattern, so
  splitting `test_x.C.test_rows_parse` into `test_rows_parse_green` and `test_rows_parse_red` makes
  the old name select both and VOID the row, and renaming it selects none. Move the row's killer
  cell to one of the new names in the same commit.

## What a kill must observe

- **Would the killer still pass if the function panicked on its first line?** Then it can kill
  nothing. An assertion that only says what did not happen (`Outcome::Idle`, "the verdict is
  unchanged") holds for a run that crashed before doing anything (`S4995`). Assert a positive
  artifact: the event row, the file, the exact output line.
- **Assert a value, never a shape.** `S16426`'s killer checked that a digest was 64 hex characters,
  which `sha256("")` also is, so no mutant of the hashed input could fail it.
- **Never derive a killer's input from the thing under mutation.** `S18808`'s killer built its
  fixture path from `ALLOWED_OPENER_PATH`, the constant its mutant changes, so the fixture moved with
  the mutant and the row read KILLED for a reason unrelated to its property. Spell the input
  literally.
- **Ask who built each side of a comparison.** When the test built both the expected value and the
  one production returns from the same call, the mutant cannot move them apart. A fixture that
  carries no data of the kind the mutated clause filters on is the other shape, and it is fixable
  from inside the test: add that data (S9870).
- **A new output line can un-kill a shipped row.** A killer that asserts an unanchored phrase passes
  again once new output contains it: `S1287` survived after a judge clock line repeated the builder's
  needle (d45). Assert the whole line.
- **A second site of an asserted thing un-kills the row that asserted the first.** `S99041` asserted
  that `land.yml` installs the scanner, a delivery added a second install, and the mutant deleting
  one left the killer satisfied (#1590). Anchor the killer on the closure property, such as every
  job that needs the tool installing it, never on one literal site.
- **A row on a callee stays killed after its call site dies.** `S9160` to `S9165` mutate a function
  whose only caller passed `&[]` for three days, and every row stayed KILLED (gates-that-lie #14).
  Anchor a row on the call site too, and check that one non-test caller passes the argument the rows
  exercise.
- **A guard's predicate needs a counterexample in reach.** A refusal written inline over a clean tree
  never meets a violating row, so weakening `!= 1` to `> 1` still passed (d1689). Give the predicate
  controls built to violate it, then pin those with rows.

## Mutants that cannot be judged

A mutant that does not compile is VOID, not a kill: `mutation-rc2` checks the build and reports
`VOID -- the mutant did not compile`. Deleting a `format!` placeholder while its argument stays
bound is a hard error, named or positional (`S1905`, d85's `S2044`). Mutate the argument line
instead, and keep a repl's arity only when it changes the rendered bytes at the fixture's own
values. The twenty compile-VOID rows of land battery `ps_6f22tvqh98` that SPEC-V2-1679 repaired
fell into six mechanisms, and in one of them the error surfaces in a file the row does not name.
`python3 scripts/mutant-viability.py --prove <id>` compiles a named row's mutant with `cargo check`
before you ship the row.

- **A changed signature breaks the repls on its file.** d1769 turned `modified_rows()` into
  `base_delta()`, and `S18545`'s repl `Ok(BTreeSet::new())` stopped type-checking: CI said "its
  killer reported no test total" while the killer was fine. After a signature change, audit the
  repl of every row on that file, not only its find.
- **A killer message that prints large text can read VOID.** `S2103`'s killer compared the whole
  rows file with `assert_eq!`, whose failure printed `error: could not compile` from the file's own
  text, and the runner scored a real kill as a mutant that did not compile (d92). Report lengths or
  offsets in such a message, never the content.
- **A Python mutant that breaks parsing poisons discovery.** `S204`'s repl left `red-first.py` with an
  `IndentationError`, a test module imported it at module scope, and unittest added a `_FailedTest`
  that `-k` cannot filter: the killer selected 2 tests. `scripts/mutant-viability.py` refuses such a
  mutant before a battery installs it.
- **A double needle is a false KILL.** `S8988`'s killer scanned `once.rs` for a symbol that occurred
  at the call site and in a comment, so a mutant of the call site compiled and survived. A find that
  spans every occurrence is the only one a source scan can kill.
- **Unviable is the portable word for VOID.** cargo-mutants reports a mutant that does not compile
  as `unviable`, "inconclusive about test coverage", and mutation-testing-elements as
  `CompileError`, an invalid mutant outside the score. No tool counts one as a kill, and a hand proof
  must not either.

## Reading a verdict

- **A path in your diff selects every row on it.** `mutants-diff` selects rows by the path of their
  target, so a row you did not write, on a file you touched, is yours this delivery: `S12080`
  VOIDed #1426 at `examined=1`, and it was the only row examined. A pull request can REPAIR a row in
  its rows diff and another can SELECT it by path at the same time (#1430 and #1427), so a census
  that reports only repairs cannot say which pull request is blocked.
- **A base-red VOID is a red killer.** `its killer does not pass at the base` means the killer
  failed with no mutant installed. Run the killer the gate's way at `origin/main` and at your head:
  green at main and red at the head means your delivery broke it (#1568: a new check stage broke
  eight tests through a stub tree that lacked the script it called).
- **`--select-only` examined nothing.** It prints `OK: N of N selected mutants killed` having
  installed no mutant, so its count is the size of the selection, not a reading.
- **A fail-fast count short of the population is no gap inside a FAIL.** `mutants-diff` stops at the
  first unkilled row, and `examined=6` of 8 means the run halted; the six are measured and the two
  were never required (`mutants.rs`, `FailFastOutcome::Unkilled`).
- **Which rows a pull request proves.** `mutants-diff` selects by five arms: rows on a path the
  three-dot diff changed, rows in the claimed band, rows the delivery modified or added in the rows
  file, and rows whose killer reads a changed file or walks a directory that grew or shrank. Under
  `--train-pass` (SPEC-V2-2175), which the pull request's job runs, only the band, added and
  modified arms are proved, and every other selected row is counted as deferred to the weekly
  battery. A row selected by your path is still yours; the land battery is where it meets you.
- **A prose-only change can select nothing.** A tip that touches only `docs/specs/`,
  `docs/decisions/`, `docs/schematics/`, `CHANGELOG.md`, `changelog.d/`, `docs/MILESTONES.md`,
  `CHARTER.md` or `CLAUDE.md` reads `mutants-diff: not-applicable`, and any other tip with an empty
  population is VOID. A document that cites a path selects no row by that citation (i1570).

## Hand proofs

- **One mutating job per work tree.** `mutants-diff` installs each mutant in place, and a gate or a
  `cargo test` reading the same tree meanwhile measures a mutant it never asked for (2026-09-20:
  `S337`'s mutant sat under a gate). Two batteries in one tree strip each other's mutants, which
  records a false KILL.
- **A dirty file after a stopped run may be a stranded mutant.** Look the change up in the rows file
  before treating it as work: when its `-` and `+` sides are both cells of one shipped row, it is a
  mutant (`S8968`'s `exit: -1` to `exit: 0` sat in a dead builder's tree). `mutants-diff` restores
  its own stranded mutant from its journal (SPEC-V2-2109); a hand run has no journal.
- **A hand proof that disagrees with CI is the hand proof's fault first.** CPython reuses a cached
  `.pyc` whose source size and whole-second mtime still match, so a same-length mutant ran old
  bytecode and `S4731` read SURVIVED. Give every control and every mutant its own
  `PYTHONPYCACHEPREFIX`, as the gate's command does, or the proof is fiction.
- **Restoring a source does not restore its binary.** A test binary built with `S99024` installed
  ran 1,500 more times after the source was restored and reported 388 failures (d1746). Rebuild
  before the next measurement.
- **The mutation lock is per cargo target directory.** `mutation_lock.rs` and
  `mutation-battery.py` key it to the tree's `target/`, as ADR-V2-234 decided when it superseded
  ADR-V2-058 D1, so a gate in another work tree can never block yours. `check.sh`'s comment that
  names the git common directory is the stale record.
- **A hand proof is killer-scoped and restores byte for byte.** Record the target's sha256, check
  that the `find` occurs exactly once, and install the replacement once. Run only the row's killer,
  and count that it selected exactly one test (libtest's `running 1 test`, unittest's `Ran 1 test`).
  Read the failure for the reason the row claims. Then write the saved bytes back and compare the
  sha256 before the next measurement: a proof whose restore went unchecked may leave the next
  measurement a mutant.
- **A bare integration-test killer can build every test target.** Measured at union-81
  (`8f7ef9edf`, d2201): `mutants-diff --prove-row` ran a bare integration-test killer with no target
  flag, so each mutant built and linked every `phxd` test binary to run one test. A killer declared
  inside a module (`mod name { ... }`) carries a `::` path, and the tree's registry places that path
  in its one target (SPEC-V2-1974).

## Retiring a row

- **A row your change makes wrong is re-anchored, never retired.** Point its `find` at the code that
  now carries the same property, under the same id, and prove it again. A row that leaves while its
  target file stays in the tree is a weakening: the formal ratchet's row rule
  (`phx_governance::ratchet::surface_weakenings`) reports it as `ROW-REMOVED`, and `pr / formal`
  refuses it until an owner ruling names that id. Check before pushing with
  `phxd formal check --root . --base main --ratchet-only`.
- **A row leaves only with the code it guarded.** A row whose target file is deleted in the same
  diff retires on its own. When the file stays and the guarded code is gone, the retirement is an
  owner act. Ruling R-BB (`docs/decisions/OWNER-RULING-2026-09-27-depot-row-retirements.md`) is the
  worked example: it admitted exactly seven of d2184's rows, each of whose `find` occurred 0 times
  once Depot polling was deleted, with rows `S26179`-`S26198` guarding the replacement code, and it
  admits no other row that leaves while its target stays.

## Probes

Stage `rows`: 10 rows (9 blocking, 1 advisory).
Stage `practice`: 12 rows (1 blocking, 11 advisory).

The `rows` stage judges phoenix-v2's row manifest and its guards: seven rows run an existing guard,
and `find-differs`, `band-ids` and `mutants-distinct` run `scripts/mutation-probe.py`, which reads the
manifest through the tree's own `scripts/mutation_rows.py`, the one Python reader. On another
repository these ten rows read red: there is no manifest to judge. Measured on 2026-09-27 over 7,611
rows: no find empty or equal to its replacement, no id held twice, and two pairs that install one
mutant for one killer (`S1125`/`S1617`, `CK1`/`S1520`; i962 recorded the class).

The `practice` stage judges any tree through `scripts/mutation-probe.py`, which another repository
may vendor. Its classes run one at a time as the rows below, or together through `walk`, whose exit
only the blocking class decides. Each prints its findings and `examined N`: a class whose population
must not be empty is VOID when it examines nothing, and one whose population may be empty (no
report, exclusion or sharded run) passes with `examined 0`. Every row is `scope: tree`.

| id | stage | severity | runs | enforces |
|---|---|---|---|---|
| `anchor-audit` | rows | block | `cargo test -p phxd --lib -- the_anchor_audit` | every row's find occurs exactly once at its target, read from the index |
| `killer-selection` | rows | block | `python3 -m unittest discover -s scripts/tests -p test_mutation_killer_selection.py` | every killer selects exactly one test: a script killer exactly, a cargo killer by shape in its row's own crate |
| `one-selection-policy` | rows | block | `cargo test -p phxd --test one_selection_policy` | one policy decides what "selected" means, for every runner |
| `rows-reader` | rows | block | `python3 -m unittest discover -s scripts/tests -p test_rows_reader.py` | every reader of the rows file goes through `_rows` |
| `seal-digest-anchors` | rows | block | `python3 -m unittest discover -s scripts/tests -p test_seal_digest_anchors.py` | no find embeds a value `config/seal.json` transcribes |
| `mutant-viability` | rows | block | `python3 scripts/mutant-viability.py` | no row's mutant breaks its target's delimiter nesting, a declared array length or a Python parse |
| `manifest-bytes` | rows | block | `cargo test -p phx-release --test compose -- --exact a_no_op_manifest_union_is_byte_identical` | the rows file round-trips byte for byte |
| `find-differs` | rows | block | `python3 scripts/mutation-probe.py check find-differs` | every row's find is non-empty and differs from its replacement |
| `band-ids` | rows | block | `python3 scripts/mutation-probe.py check band-ids` | the population assembles, every fragment's rows inside its band and no two bands overlapping, and no id is held twice |
| `mutants-distinct` | rows | advisory | `python3 scripts/mutation-probe.py check mutants-distinct` | no two rows install one mutant for one killer |
| `tool-config-valid` | practice | block | `python3 scripts/mutation-probe.py check tool-config-valid` | every cargo-mutants, StrykerJS and mutmut configuration loads under its tool's own rules |
| `tool-configured` | practice | advisory | `python3 scripts/mutation-probe.py check tool-configured` | every Cargo workspace, tested npm package, Python project and JVM build has a mutation tool wired |
| `tool-config-current` | practice | advisory | `python3 scripts/mutation-probe.py check tool-config-current` | no option its tool marks deprecated, renamed or likely invalid |
| `timeouts-bounded` | practice | advisory | `python3 scripts/mutation-probe.py check timeouts-bounded` | a cargo-mutants run with no baseline, or a multiplier in place, sets `--timeout`, and no run sets both timeout flags |
| `exclusions-justified` | practice | advisory | `python3 scripts/mutation-probe.py check exclusions-justified` | every exclusion carries its reason on its line or the line above |
| `ci-diff-run` | practice | advisory | `python3 scripts/mutation-probe.py check ci-diff-run` | a pull request's CI runs each tool in use, in its change-only mode where it has one |
| `ci-full-run` | practice | advisory | `python3 scripts/mutation-probe.py check ci-full-run` | some CI run sweeps the whole tree with each tool in use |
| `shards-complete` | practice | advisory | `python3 scripts/mutation-probe.py check shards-complete` | a sharded run's matrix names every shard its denominator promises |
| `report-kept` | practice | advisory | `python3 scripts/mutation-probe.py check report-kept` | every CI job that runs mutants uploads its report under `if: always()` |
| `report-schema` | practice | advisory | `python3 scripts/mutation-probe.py check report-schema` | every mutation-testing-elements report and `outcomes.json` reads in its format |
| `survivors-triaged` | practice | advisory | `python3 scripts/mutation-probe.py check survivors-triaged` | no report leaves a survivor unexamined, and every ignored mutant says why |
| `score-threshold` | practice | advisory | `python3 scripts/mutation-probe.py check score-threshold` | a score gate is declared where the tool has one, and a report meets its own low threshold |

`mutants-diff --prove-row <id|stem|range>` proves named rows the gate's way (SPEC-V2-2089). It takes
the rows you name, so it is a recipe and not a row of this table. A `range` spelled `S<lo>-S<hi>`
proves every row of a delivery's own band in one call, the same span its fragment file is named
for (d2186).

`mutants-diff --help` is not real help: it answers `REFUSED(2): unknown argument: --help`. Read
`--root`, `--prove-row`, `--list` and `--base` from `mutants-diff.rs`'s own source instead, until
d2201 adds a real one.

Each command above runs from the repository root. `checks.json` spells it with `{root}` in place of
that root, and every cargo row names `--manifest-path {root}/Cargo.toml`: cargo looks for a manifest
in each parent directory, so a probe run against a fixture tree inside this repository would
otherwise build and test the whole workspace above it. The `mutation-probe.py` rows run the script
that ships beside the pack, `{skills}/../scripts/mutation-probe.py`, against `--root {root}`.
