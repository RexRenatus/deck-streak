# SPEC-030: tests leave no temporary files, and the pack wiring cannot lag the tree

- **Wave:** W0. **Issue:** #23 (epic #1). **Context(s):** `repo` (`scripts/`, `.github/workflows/`, `.packs/wiring.json`).
- **Decided by:** ADR-004 (the vendored packs and their wiring), ADR-012 (testing), ADR-017 (hosted CI on pull requests into `dev` and `main`), ADR-030 (this SPEC's own: the box-pack runner uses each pack's own verb, judges the committed tree without the vendored rules, and names every expected red).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-030.md` (ADR-016).

## 1. The problem, measured

- **What SPEC-002 already holds** (its `scripts/tests/test_ci_workflows.py` and
  `scripts/tests/test_pack_wiring.py`, read in the architect's working tree after `main` e05dfa5):
  every workflow defaults to `permissions: contents: read`; every `uses:` is pinned by a 40-hex
  commit; no workflow runs on a self-hosted runner or `pull_request_target`; `ci.yml` runs every
  stage of `scripts/check.sh`; the aggregate `ci` job needs every job; every vendored pack has a
  wiring state; every pending or deferred pack or row names an issue in the manifest; the runner
  refuses a wiring that forgets a pack.
- **What nothing holds yet.**
  - **The triggers.** No test says `ci.yml` runs on pull requests into `dev` AND `main`: deleting
    `main` from its `branches` list would pass every test above while release pull requests went
    unchecked.
  - **Temporary files.** The predecessor pinned pytest's `tmp_path_retention_policy = "failed"` so
    green runs left nothing on the small shared host (`pyproject.toml`, `tests/test_tmp_retention.py`
    at predecessor `27ee2bc`). DeckStreak has no such guard, and two of its tests already leak:
    `scripts/tests/test_pack_wiring.py` and `tools/parity-oracle/test_generate.py` both call
    `tempfile.mkdtemp()` and never remove the directory (read at `main` e05dfa5; SPEC-029 repairs
    the second).
    Amended by the delivery, measured at `dev` cb66427: `scripts/tests/test_public_scrub.py`,
    added after e05dfa5, calls `tempfile.mkdtemp()` twice (its `scrub` helper and
    `test_an_empty_subject_is_void`) and removes neither directory, so R5 repairs it too.
  - **Honest states.** `scripts/pack-rows.py` accepts a pack left `pending` after its subject
    exists and every row passes, and never runs a deferred row, so a deferral outlives the issue
    that was to lift it and nobody learns the row is already green (`.packs/wiring.json` holds
    six deferred rows, sixteen pending packs and one deferred pack in the architect's working tree
    after `main` e05dfa5, counted by state).
- **The box-pack runner judges nothing.** `scripts/box-packs.sh`, run at `dev` c1f53c1 with a phxd
  built from the vendored phoenix-v2 commit, reported all ten cards RED for reasons unrelated to the
  tree:
  - every `pack probe` exited 4 with `skills/catalog.json not found`, because the runner never
    passes `--skills-root`;
  - four packs (web-security, cyber-pipeline, ux-laws, ui-styles) declare `phxd.pack.run.v1`, and
    `pack probe` refuses them (`wrong_verb`); `pack run` needs a project row in a ledger;
  - with the verbs corrected, web-security still read the vendored rule code as DeckStreak's own:
    two false reds came from `.packs/scripts/privacy-gdpr-probe.py` and
    `.packs/scripts/telegram-platform-probe.py`;
  - the subscription-proxy client scan exits 2 (all VOID) while no settings document exists, and
    the runner reports that as red.
  With the verbs corrected and the vendored rules excluded, the same tree has 37 red rows in seven
  packs. Some are real gaps with owners, such as a missing Content-Security-Policy (#21) and
  unvalidated init data (#17); others need a built site (#59). Nothing records which reds are
  expected.
- **What the predecessor's CI did that DeckStreak's gate already does.** Lint, types, tests and a
  supply-chain audit (`ci.yml` at predecessor `27ee2bc`): SPEC-002's `check.sh` stages and its
  `audit` stage.

**Order.** This SPEC lands after SPEC-029 (which repairs the oracle's leak) and needs no Rust
context, so it may run beside SPEC-020. Landing it early means every later W0 test is written under
its lint rather than repaired after it.

## 2. Requirements

R1. `.github/workflows/ci.yml` runs on `pull_request` into `dev` and into `main`, and on `push` to
    both, and a test fails if either branch leaves either trigger.
R2. `scripts/tests/test_temp_hygiene.py` reads every test file of the repository: Rust integration
    tests (`crates/*/tests/**/*.rs`), the repository's Python tests (`scripts/tests/*.py`,
    `tools/parity-oracle/test_*.py`) and the Mini App's tests (`web/app/src/**/*.test.ts`,
    `web/app/tests/**/*.ts`), prints `examined N test file(s)` and refuses zero.
R3. The lint refuses, naming the file and line:
    - in Rust: `TempDir::into_path`, `TempDir::keep`, `NamedTempFile::keep`,
      `NamedTempFile::into_temp_path` followed by `keep`, `std::env::temp_dir()`, and a string
      literal that begins with `/tmp/`;
    - in Python: `tempfile.mkdtemp(`, `tempfile.mkstemp(`, and `NamedTemporaryFile(` with
      `delete=False`, unless the same function registers the path's removal (`addCleanup` or a
      `finally` that removes it); `tempfile.TemporaryDirectory` is the accepted form;
    - in TypeScript: `mkdtempSync(` or `mkdtemp(` in a file with no `rmSync(` or `rm(` of the result,
      and a string literal that begins with `/tmp/`.
    The accepted forms (`tempfile::TempDir` and `NamedTempFile` dropped at scope end,
    `TemporaryDirectory`, a removed `mkdtemp`) remove what they create on drop or exit.
R4. The lint's own planted fixtures, one leaking file per language under
    `scripts/tests/fixtures/temp-hygiene/`, carry a `.fixture` suffix after their language's
    extension, so no test runner and no probe glob collects them; the lint reads them by the inner
    extension, refuses each, and excludes the directory from the tree census.
R5. `scripts/tests/test_pack_wiring.py` and `scripts/tests/test_public_scrub.py` create their
    temporary directories with `tempfile.TemporaryDirectory`, so the tree is clean under R3 (the
    second amended in: it leaks the same way, and section 1's census predates it).
R6. `scripts/pack-rows.py` refuses (exit 1, naming the pack) a `pending` pack whose every blocking
    row ran and passed with a non-zero examined count: the pack's subject exists and is green, so
    its state must say `enforced`.
R7. `scripts/pack-rows.py` runs every deferred row in a separate pass and refuses (exit 1, naming
    the pack and the row) one that passes: a passing row's deferral is stale and must be removed. A
    deferred row that is red, void or in error stays deferred and fails nothing.
R8. Wherever R6 or R7 would refuse the tree this SPEC lands on, `.packs/wiring.json` is updated in
    the same delivery (the pack enforced, the row's deferral removed), so the gate stays green.
R9. SPEC-002's wiring tests keep passing: every vendored pack has a state, and every waiting pack or
    row names an issue in `docs/issues-manifest.json`.

R10. `scripts/box-packs.sh` runs each phxd pack with the verb its catalog entry admits, read from
    `phxd pack list` and never hard-coded:
    - `phxd pack probe --skills-root <phoenix>/skills` for a pack declaring `phxd.pack.probe.v1`;
    - `phxd --ledger <scratch> pack run --project <id> --skills-root <phoenix>/skills` for
      `phxd.pack.run.v1`, against a scratch ledger it creates with `phxd init` and
      `phxd project register` in a temporary directory outside both repositories;
    - `phxd verify seo-pipeline` for the site once it is built (#59).
R11. It judges the committed tree at `--rev` (default `HEAD`), exported with `git archive`, without
    the vendored rule code: `.packs/` and the vendored methodology probes. A rule's own source is
    never read as DeckStreak's code.
R12. `.packs/wiring.json` names, for each phxd pack, every row expected red on the tree and the open
    issue that builds that row's subject.
    - A red row it does not name fails the run, naming the pack and the row.
    - A named row that is no longer red is refused as stale, as in R6 and R7.
    - An advisory row never fails the run.
R13. The subscription-proxy client scan reads `pending` with its issue while it examines no
    settings document (#29), and fails the run on any red. VOID is never reported as green.
R14. The run prints one line per pack: its examined count, its unexpected, expected and stale rows.
    It exits 0 only when every pack examined at least one row, or reads `pending` with the open
    issue `.packs/wiring.json` names for it, and no row is an unexpected red or a stale expectation.
    A `pending` pack that examines a row is a stale expectation. (Amended by the delivery: ui-styles
    declares only catalog rows, so every `pack run` of it is VOID with `examined: 0` and exit 3, by
    its own SKILL.md, whatever the tree holds; and the seo-pipeline pack has no built site to verify
    until #59. As first written, R14 could exit 0 on no tree. Both read `pending`, as R13's proxy
    scan does, and neither is ever reported green.)

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the ci workflow runs on pull requests into dev and main, and on pushes to both | `test_ci_workflows.py` `the_ci_workflow_runs_on_pull_requests_into_dev_and_main` |
| A2 | every test file of the tree creates temporary files only in forms removed on drop (examined count, zero refused) | `test_temp_hygiene.py` `every_test_file_removes_what_it_creates` |
| A3 | a planted Rust test that keeps its temporary directory is refused | `test_temp_hygiene.py` `a_planted_rust_leak_is_refused` |
| A4 | a planted Python test that never removes its `mkdtemp` directory is refused | `test_temp_hygiene.py` `a_planted_python_leak_is_refused` |
| A5 | a planted TypeScript test that never removes its `mkdtempSync` directory is refused | `test_temp_hygiene.py` `a_planted_typescript_leak_is_refused` |
| A6 | the runner refuses a pending pack whose every blocking row passed | `test_pack_wiring.py` `a_pending_pack_whose_rows_all_pass_is_refused_as_stale` |
| A7 | the runner refuses a deferred row that now passes | `test_pack_wiring.py` `a_deferred_row_that_passes_is_refused_as_stale` |
| A8 | a deferred row that is still red stays deferred and fails nothing | `test_pack_wiring.py` `a_deferred_row_that_is_still_red_fails_nothing` |
| A9 | each phxd pack runs with the verb its catalog entry admits, the run packs against a scratch ledger outside the repository | `test_box_packs.py` `each_pack_runs_with_the_verb_its_catalog_admits` |
| A10 | the judged tree holds no vendored rule code | `test_box_packs.py` `the_judged_tree_holds_no_vendored_rule_code` |
| A11 | a red row the wiring does not expect fails the run by name | `test_box_packs.py` `an_unexpected_red_row_fails_the_run_by_name` |
| A12 | an expected red row that turns green is refused as stale | `test_box_packs.py` `an_expected_red_row_that_turns_green_is_refused_as_stale` |
| A13 | the proxy scan with no settings document reads pending, never green | `test_box_packs.py` `the_proxy_scan_without_a_settings_document_reads_pending` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_ci_workflow_runs_on_pull_requests_into_dev_and_main
A2: python3 -m unittest discover -s scripts/tests -p test_temp_hygiene.py -k every_test_file_removes_what_it_creates
A3: python3 -m unittest discover -s scripts/tests -p test_temp_hygiene.py -k a_planted_rust_leak_is_refused
A4: python3 -m unittest discover -s scripts/tests -p test_temp_hygiene.py -k a_planted_python_leak_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_temp_hygiene.py -k a_planted_typescript_leak_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k a_pending_pack_whose_rows_all_pass_is_refused_as_stale
A7: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k a_deferred_row_that_passes_is_refused_as_stale
A8: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k a_deferred_row_that_is_still_red_fails_nothing
A9: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k each_pack_runs_with_the_verb_its_catalog_admits
A10: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k the_judged_tree_holds_no_vendored_rule_code
A11: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k an_unexpected_red_row_fails_the_run_by_name
A12: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k an_expected_red_row_that_turns_green_is_refused_as_stale
A13: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k the_proxy_scan_without_a_settings_document_reads_pending
```

A6 to A8 run the runner over a copy of `.packs/` and `scripts/` in a `TemporaryDirectory`, with a
synthetic pack whose rows are one-line probes that exit 0, 1 or 3 as the case needs; they never run
the real packs, so they stay fast.

A9 to A13 drive `scripts/box-packs.sh` with a fake phxd and a fake phoenix checkout, under
`scripts/tests/fixtures/box-packs/`. The fake phxd records its argv and prints planted cards, so the
tests need neither phxd nor the private phoenix-v2 checkout and run in CI. The real run stays on the
maintainer's box.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_ci_workflows.py` | repo | changed: A1 |
| `scripts/tests/test_temp_hygiene.py` | repo | added: A2 to A5 |
| `scripts/tests/fixtures/temp-hygiene/leaks_tempdir.rs.fixture` | repo | added: planted Rust leak |
| `scripts/tests/fixtures/temp-hygiene/leaks_mkdtemp.py.fixture` | repo | added: planted Python leak |
| `scripts/tests/fixtures/temp-hygiene/leaks_mkdtemp.ts.fixture` | repo | added: planted TypeScript leak |
| `scripts/tests/test_pack_wiring.py` | repo | changed: A6 to A8; `TemporaryDirectory`; the `box` expectations name manifest issues (R9) |
| `scripts/tests/test_public_scrub.py` | repo | changed: `TemporaryDirectory` (R5, amended in: its census postdates section 1) |
| `scripts/pack-rows.py` | repo | changed: the stale-pending refusal and the deferred-row pass |
| `.packs/wiring.json` | repo | changed where R8 applies, and the `box` section R12 and R13 read |
| `scripts/box-packs.sh` | repo | changed: R10 to R14 |
| `scripts/tests/test_box_packs.py` | repo | added: A9 to A13 |
| `scripts/tests/fixtures/box-packs/fake-phxd` | repo | added: the fake phxd and its planted cards |
| `scripts/tests/fixtures/box-packs/phoenix/skills/catalog.json` | repo | added: a fake catalog naming one probe pack, one run pack and one seo-pipeline pack (amended: A9 pins R10's third verb too) |
| `docs/TESTING.md` | repo | changed: the temporary-file rule and the honest-state rule |
| `docs/red-first/SPEC-030.md` | repo | added |
| `docs/decisions/ADR-030-the-box-pack-runner-uses-each-packs-own-verb.md` | repo | added: R10 to R14's design and what it was chosen against |
| `docs/schematics/box-pack-runner.md` | repo | added: the runner's data flow and each pack's verdict (amended in: CLAUDE.md asks for a schematic before a changed data flow) |
| `changelog.d/` fragment | repo | added |


## 5. What this does NOT do

- It changes no CI stage and no workflow's jobs: the gate's stages and the hardened workflow are
  SPEC-002's, delivered (#23).
- It enforces no pack whose subject is not built: each waits for the issue its wiring names, and
  the last of them are lifted by W7's work (#60).
- It builds no phxd and runs no phxd pack in CI: the maintainer builds phxd from the vendored
  phoenix-v2 commit, and the phxd packs stay box-run until the open-source pack runner exists
  (#60).
- It keeps no pytest retention setting: DeckStreak's Python tests are `unittest`, and the lint
  replaces the setting's purpose (#23).
- It runs no deferred PACK in R7's pass: durable-services reads a tree with no unit as a finding,
  not as VOID, so its rows stay unrun until the deploy templates enforce it (#25). R7's pass runs
  the deferred ROWS of packs whose rows run.
- It builds no site for the seo-pipeline pack: the runner verifies `web/site/dist` only when the
  judged commit holds it, and building the landing page for the run is the landing page's work
  (#59).

## 6. Risks

- **The lint reads a string, not a path.** A temporary directory built from a variable escapes a
  text read. Mitigated by refusing the leaking APIs themselves (`into_path`, `keep`, `mkdtemp`
  without removal) rather than paths; a builder who needs a kept directory must argue it in the
  SPEC that needs it.
- **Running deferred rows slows the gate.** A deferred row is one probe run; the six deferred at
  SPEC-002 each finish in seconds. `pack-rows.py`'s summary line reports the extra pass's count and
  time, so growth is visible.
- **A pack flips between green and void across deliveries.** R6 refuses only when every blocking
  row examined something and passed; a void row keeps a pack pending, so a subject that disappears
  is reported as void, never hidden.
