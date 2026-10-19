# SPEC-476: the box-pack runner's cards cleanup is held by its tests and touches nothing else

Status: accepted; built by one pull request into dev that closes #542. Decided by ADR-493. Every
`path:line` below was read at dev `36c4783a`.

## 1. The problem, measured

`scripts/box-packs.sh` removes the temporary cards directory it made when a run ends (#531), and
the review of #539 found that removal correct. What holds it is the code: the tests leave four of
its readings unpinned, and two small faults sit beside it.

- **No sibling, so a wider removal is not caught by an assertion.** The cleanup tests in
  `scripts/tests/test_box_packs.py` start from an empty temporary directory (`:1002-1016`,
  `:1018-1033`). A copy that removes `out.parent` fails them only by `FileNotFoundError` from
  `iterdir`; a copy that removes every `deckstreak-box-cards.*` entry passes them.
- **Three cells of the two variables are unpinned.** `made = not os.environ.get("BOX_PACKS_OUT")` and
  `keep = bool(os.environ.get("BOX_PACKS_KEEP"))` (`scripts/box-packs.sh:246-247`), with
  `cards_directory` testing `if chosen:` (`:539`), read an empty `BOX_PACKS_OUT` as unset and keep for
  any non-empty `BOX_PACKS_KEEP`, `0` included, as the runner's comment says (`:65-70`). The harness
  sets `BOX_PACKS_KEEP` only to `1` and `BOX_PACKS_OUT` only to a path or not at all (`:342-363`), so
  no test sets an empty `BOX_PACKS_OUT`, an empty `BOX_PACKS_KEEP` or `BOX_PACKS_KEEP=0`.
- **Three lines past 100 columns.** `awk 'length($0) > 100' scripts/box-packs.sh` prints three lines
  of its 1133: 262 (102 columns), 264 (101) and 267 (101). The repository's style is 100 columns
  (`ruff.toml:6`), and the runner's Python is never linted, since it sits in a heredoc.
- **Two statements between the make and the try.** `out = cards_directory()` is line 243, the two
  readings are lines 246 and 247, and the `try` whose `finally` removes `out` is line 248. A SIGTERM
  or SIGHUP, which the runner turns into `sys.exit` (`:194-196`), or a SIGINT, arriving between 243
  and 248 leaves the made directory behind.
- **CI cannot see a change to the runner.** `scripts/mutation-verdict.py` classes only
  `scripts/[^/]+\.py` as `scripts` (`:120`), so a change to the runner reads not-applicable in every
  class (`:802`), and the 13 hand rows on it (S12301 to S12313) are its only mutation proof.

## 2. Requirements

- R1. A run that made its cards directory removes exactly that directory. A sibling named
  `deckstreak-box-cards.decoy` in the same temporary directory survives, its file byte for byte, and
  so does the temporary directory itself, whether the run passes, fails or stops on an error.
- R2. A `BOX_PACKS_OUT` that is set and empty reads as unset: the run makes its own directory in the
  temporary directory and removes it when it ends, writes nothing into its working directory and
  prints no `cards:` line; with `BOX_PACKS_KEEP=1` beside it, the made directory is kept with its
  cards and named on stderr as `cards: <path>`.
- R3. `BOX_PACKS_KEEP=0` keeps the made directory with its cards and names it on stderr as
  `cards: <path>`.
- R4. A `BOX_PACKS_KEEP` that is set and empty keeps nothing: the made directory is removed and no
  `cards:` line is printed.
- R5. Every line of `scripts/box-packs.sh` fits 100 columns, and the three wrapped lines leave the
  runner's syntax tree as it was.
- R6. The runner reads `BOX_PACKS_OUT` and `BOX_PACKS_KEEP` before it makes the cards directory, and
  the statement that makes it is the last before the `try` whose `finally` removes it.
- R7. Every test runs the runner over the fixture tree, with `TMPDIR` and the working directory inside
  the test's own temporary directory. No existing test, assertion, fixture or examined count is
  changed or removed.

## 3. Acceptance criteria of SPEC-476

| A | criterion | decided by |
|---|---|---|
| A1 | R1: a decoy sibling and the temporary directory survive a pass, a failure and an error; the made directory does not | `test_box_packs.TheRunnerCleansUpItsCards.test_a_decoy_sibling_and_the_parent_survive_every_end` |
| A2 | R2 and R7: an empty `BOX_PACKS_OUT` reads as unset, alone and beside `BOX_PACKS_KEEP=1`, and the working directory stays empty | `test_box_packs.TheRunnerCleansUpItsCards.test_an_empty_out_is_read_as_unset` |
| A3 | R3: `BOX_PACKS_KEEP=0` keeps the made directory and names it | `test_box_packs.TheRunnerCleansUpItsCards.test_a_keep_of_0_keeps_the_made_directory` |
| A4 | R4: an empty `BOX_PACKS_KEEP` keeps nothing | `test_box_packs.TheRunnerCleansUpItsCards.test_an_empty_keep_removes_the_made_directory` |
| A5 | R5: no line of the runner runs past 100 columns | `test_box_packs.TheRunnerSourceHoldsItsCures.test_every_runner_line_fits_100_columns` |
| A6 | R6: the make is the statement just before the `try` that removes it, after both readings | `test_box_packs.TheRunnerSourceHoldsItsCures.test_the_cards_directory_is_made_just_before_its_try` |

Each line runs from `scripts/tests`, the unittest root the repository's tests are discovered from.

```acceptance
A1: python3 -m unittest test_box_packs.TheRunnerCleansUpItsCards.test_a_decoy_sibling_and_the_parent_survive_every_end
A2: python3 -m unittest test_box_packs.TheRunnerCleansUpItsCards.test_an_empty_out_is_read_as_unset
A3: python3 -m unittest test_box_packs.TheRunnerCleansUpItsCards.test_a_keep_of_0_keeps_the_made_directory
A4: python3 -m unittest test_box_packs.TheRunnerCleansUpItsCards.test_an_empty_keep_removes_the_made_directory
A5: python3 -m unittest test_box_packs.TheRunnerSourceHoldsItsCures.test_every_runner_line_fits_100_columns
A6: python3 -m unittest test_box_packs.TheRunnerSourceHoldsItsCures.test_the_cards_directory_is_made_just_before_its_try
```

A5 and A6 are red first: each fails by an assertion against the runner as it stands at dev and
passes once the cures land. A1 to A4 pin readings the runner already has, so each is green at dev;
each is read red by an assertion against the copies section 5 names, planted one at a time in the
runner as it stands at dev. `docs/red-first/SPEC-476.md` records both kinds.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/box-packs.sh` | production | lines 262, 264 and 267 wrapped to 100 columns with the same syntax tree; `made` and `keep` read above `out = cards_directory()`, with one comment line saying why |
| `scripts/tests/test_box_packs.py` | test | `Box` starts every run in a fresh empty working directory and `Box.run` takes `environ`; A1 to A4 in `TheRunnerCleansUpItsCards`; A5 and A6 in a new class `TheRunnerSourceHoldsItsCures`; both docstrings name this SPEC |
| `scripts/mutation-rows.d/S47600-S47699.json` | rows | a new band fragment: S47601 to S47610 in `SCRIPT_MUTATIONS` |
| `docs/specs/SPEC-476-the-box-pack-runners-cards-cleanup-is-held-by-its-tests-and-touches-nothing-else.md` | document | this SPEC |
| `docs/decisions/ADR-493-the-box-pack-runners-cards-cleanup-is-held-by-tests-that-plant-each-wrong-copy.md` | document | its decision |
| `docs/schematics/box-pack-runner.md` | document | an insert-only section after its last line: the cards directory's lifetime, its tests and its rows |
| `docs/red-first/SPEC-476.md` | document | the red-first record: A5 and A6 red then green, A1 to A4 not red with the rows that read them red |
| `changelog.d/box-packs-cards-held-476.md` | document | the changelog fragment |

`scripts/tests/test_box_scan_expected.py` imports `Box` and runs unchanged.

## 5. Rows that hold each criterion

Each row is `[id, path, find, replace, description, killer]` in `SCRIPT_MUTATIONS`, its target
`scripts/box-packs.sh`. The line is the find's first line at dev, or, for a row on a cured site,
the site it covers.

| row | find, on | the copy its replacement plants | killer |
|---|---|---|---|
| S47601 | the removal, `:298` | `shutil.rmtree(out.parent, ...)`: removes the temporary directory | A1 |
| S47602 | the removal, `:298` | removes every `deckstreak-box-cards.*` entry beside `out` | A1 |
| S47603 | `if chosen:`, `:539` | `if chosen is not None:`: an empty `BOX_PACKS_OUT` names the working directory | A2 |
| S47604 | the `made` reading, `:246` | `made` decided by the variable's presence | A2 |
| S47605 | the `keep` reading, `:247` | keeps only on `1` | A3 |
| S47606 | the `keep` reading, `:247` | `keep` decided by the variable's presence | A4 |
| S47607 | the moved `keep` reading and the make | puts the `keep` reading back between the make and the `try` | A6 |
| S47608 | the wrapped loop, `:262` | the loop back on one 102-column line | A5 |
| S47609 | the wrapped refusal, `:264` | the refusal back on one 101-column line | A5 |
| S47610 | the wrapped `judging` text, `:267` | the text back on one 101-column line | A5 |

No existing row is re-anchored: none of S12301 to S12313 has a find on a line this delivery changes,
and each find still occurs once.

## 6. What this does NOT cover

- A mutation leg that covers the runner's script, #542's fifth item: a change to the runner stays
  proved by its hand rows alone; the leg changes the mutation scripts and the workflow, which other
  open pull requests change, and it moves to an issue of its own, linked from #542.
- What each pack finds when the runner judges a commit, and the expectations the private file
  states: unchanged, and held by the runner's existing tests (#542 scopes them out).
- The runner's other options and outputs: `--rev`, `--post-status`, the scratch directory of the
  exported tree and its ledger, and a relative `BOX_PACKS_OUT`, which resolves against the working
  directory and is never removed (#542 scopes them out).
- A signal sent while the runner works: the handler it installs reaches the `finally` (ADR-493 D1),
  but no test sends one, and a signal inside the standard library's make, before the directory's
  path is returned, is not measured (#542 names only the two statements' window).
- The proof of every selected row in CI's `mutation-rows` job, S12301 to S12313 and S47601 to
  S47610, which only the run after the push can measure; the build proves the new rows by hand
  (#542).

## 7. Risks

- **A planted copy that removes a directory.** S47601 removes the temporary directory, and under
  S47603 the runner removes the contents of its working directory. Each is safe only because every
  test run sets `TMPDIR` and the working directory inside the test's own temporary directory; the
  working-directory change lands before any copy is planted, and a copy is only ever run through the
  module's own tests. Detected by the round's temporary directory reading empty after each run.
- **A tree-wide check in CI reads the changed test module** (a census of test doubles, of examined
  counts or of subprocess calls). The module adds no test double, and each new enumerating test
  reports what it examined; CI's scripts tests decide.
- **A dotted acceptance line run from the wrong directory selects nothing.** Each line runs from
  `scripts/tests`, and the build reads `Ran 1 test` for each.
- **The wrap's syntax-tree equality.** If Python's parser kept adjacent f-strings apart, the equality
  would fail; the build reads it at the cure commit and stops on any difference.
- **A shell row whose mutant breaks the heredoc's Python** would fail its killer by a crash and read
  KILLED. Each mutant's body is parsed before its proof, and each killer's red is read as an
  assertion.
