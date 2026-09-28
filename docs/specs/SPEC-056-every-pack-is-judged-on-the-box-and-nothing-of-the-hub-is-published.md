# SPEC-056: every pack is judged on the box, and nothing of the hub is published

- **Wave:** W0. **Issue:** none of its own: it applies the owner's box-only ruling (ADR-056) in
  full, and #60 keeps its goal, every pack wired and green. **Context(s):** `repo` (`scripts/`,
  `.github/`, `docs/`), `deck-streak-vault` (the data its adapter reads).
- **Decided by:** ADR-069 (the public tree carries no vendored hub files, and the box driver's
  shape), ADR-056 (the packs stay box-only), ADR-059 (public text describes DeckStreak only).
- **Status:** judged: written at delivery, because it had no planned copy, and delivered with its
  tests and `docs/red-first/SPEC-056.md` (ADR-016).

## 1. The problem, measured

Measured at `22fb19a` (the redaction's head, which this delivery is stacked on).

- **The vendored tree.** `.packs/` holds 183 files: the packs' skills, rows and probes, their
  manifest `VENDORED.json` and the wiring `wiring.json`. Beside it, seven scripts exist only to run
  or refresh that copy: `scripts/pack-rows.py`, `scripts/vendor-packs.py`, the four methodology
  probes (`scripts/methodology_probe.py`, `sdd-probe.py`, `ddd-probe.py`, `tdd-probe.py`) and
  `scripts/no-apikeyhelper-scan.py`.
- **Names of the maintainer's private tooling.** 211 lines in 39 files outside `.packs/`, and two
  file paths, name that tooling: its project, its runner, its proxy and its paths, counted with the
  maintainer's private word list, which is not in this repository.
- **Readers.** 69 files outside `.packs/` read or name a file this delivery removes, in 279 lines,
  as this command prints them, one file a line with its count:

  ```
  git grep -I -c -E '\.packs\b|pack-rows|vendor-packs|VENDORED\.json|methodology_probe\b|sdd-probe|ddd-probe|tdd-probe|no-apikeyhelper-scan' 22fb19a -- . ':(exclude).packs'
  ```

  They are:
  - the gate, `scripts/check.sh`, whose `packs` stage ran 480 rows of 29 packs and which CI's
    `packs` job ran;
  - 8 scripts: the six this delivery removes, which name each other, `box-packs.sh` and
    `public-scrub.py`;
  - 6 tests;
  - 3 vault crate files;
  - 11 delivered SPECs and 7 planned ones;
  - 11 ADRs, 4 schematics and 4 red-first records;
  - 3 changelog fragments and 8 living docs;
  - one web test's comment;
  - `stack.json` and `ruff.toml`.
- **Advisory departures.** `test_deploy_templates.py`'s
  `test_every_departure_from_an_advisory_is_waived_with_its_why` read the vendored durable lint's
  report. It held every advisory departure of the deploy templates to a waiver in its unit
  (`X-DurableServices-Waive=`), with a why of more than five words, or to the one issue it waits on
  (#44), and it pinned the set of waivers. The box driver reads a failing advisory row as advisory
  and never fails on it, so removing that test without a port would lose the judgment.
- **Code that reads vendored data.** The vault adapter compiles in the vault-duties pack's rails,
  layout and rows (`include_str!` in `crates/vault/src/rails.rs` and `staged.rs`); the public scrub
  composes its rules from two vendored deny lists through a vendored probe's loader; and the
  deploy templates' tests parse units with the vendored durable lint. Public CI holds no private
  checkout (ADR-017), so none of these can read the pack there.
- **The settings scan.** The gate's scrub stage ran the subscription-proxy pack's apiKeyHelper scan
  (`scripts/no-apikeyhelper-scan.py`), and reported it pending by name while the tree holds no
  settings file (SPEC-043). The box driver ran only the proxy-client scan, which has no apiKeyHelper
  row.
- **Acceptance lines of removed tests.** 21 lines in the acceptance fences of seven delivered SPECs
  run a test this delivery removes, and the tdd pack judges every line of every `` ```acceptance ``
  fence. An accepted SPEC may change only insert-only (ruling (i), SPEC-038 section 8).
- **Schema ids.** 53 files carry `phx.*` schema ids (parity goldens, vault duty records, their
  fixtures and constants). They are contracts the box-run packs judge by, not prose (ADR-069).
- **The gate's log line.** `scripts/check.sh` prints `logs: <directory>` on stdout, so quoting the
  gate's tail pastes a path of the machine that ran it. CI never reads that line: each job names
  `CHECK_LOG_DIR` and uploads it.

## 2. Requirements

R1. No vendored pack file is in the tree: `.packs/` and the seven scripts above are removed, and so
    is every test that exists only for one of them (section 4 names each, with its reason).
R2. `methodology.json` holds DeckStreak's own configuration only, which the box run's probes read
    with `--root`: it has no `vendored_from` key.
R3. `scripts/check.sh` has no `packs` stage, and its `scrub` stage runs the public scrub alone.
    `.github/workflows/ci.yml` has no `packs` job, and the `ci` job's `needs` does not name one.
    Every other stage and job stays; SPEC-038's layout tests change only in the tables that list
    jobs and stages, and no other assertion is weakened.
R4. `scripts/check.sh` never prints its log directory on stdout. When the caller names no directory
    (`CHECK_LOG_DIR` unset), it names the fresh one on stderr; when the caller names one, as each CI
    job does, it prints none, and that directory holds every stage's log and `timings.tsv`.
R5. DeckStreak's own code reads only DeckStreak's own files. The vault adapter's rails, default
    layout and gate classes are `crates/vault/data/rails.json`, `layout.json` and
    `gate-classes.json`; the public scrub's shapes are `scripts/scrub-rules/persona-core.json` and
    `privacy-gdpr.json`, read by a loader in `scripts/public-scrub.py`; the deploy templates' tests
    read units with `scripts/tests/_units.py`. Each data file keeps exactly the fields its parser
    reads and its `phx.*` schema id, and no reference to its source's documents.
R6. Behaviour is unchanged: every vault test passes with no assertion edited; the scrub examines
    the same files and history blobs, with the same findings, and still reads the maintainer's
    private list in its schema; each parser refuses a document missing a field it reads.
R7. `scripts/box-packs.sh` names none of the maintainer's private tooling. It reads three variables:
    `PACKS_WIRING`, a private file (schema `deckstreak.box-wiring.v1`) holding the pin, the packs,
    the box section, the paths of the methodology probes and of the two scans, the skills
    directory and the owned files' sources; `PACKS_CHECKOUT`, the private checkout at the pin; and
    `PACKS_RUNNER`, the runner built from it. It refuses VOID by name when one is unset or unusable.
R8. The driver judges every pack the private file names through the runner, with the verb its
    catalog admits (`pack probe --scope tree` for a probe pack), and reads each card by the suffix
    of its schema. The `packs` section keeps the judgment of the removed row runner: an enforced
    pack fails on a blocking row that is RED, VOID or in ERROR; a pending pack reads a blocking VOID
    row as pending and is STALE once every blocking row passes; a deferred pack runs no row; an
    excluded row is counted and never judged; a deferred row that passes is STALE. The `box`
    section keeps its judgment (expected reds, pending, issues read with `gh`, stale expectations).
R9. The driver runs the sdd, ddd and tdd probes, the proxy-client scan and the apiKeyHelper scan
    from the checkout, against the judged tree, and prints one verdict line for each. The
    apiKeyHelper scan keeps the removed gate step's refusals: a finding fails it, and so does a
    settings file it cannot read; while the tree holds no settings file it reads `pending` with the
    issue the private file names, is VOID without one, and is stale once it examines a file or
    its issue closes.
R10. For each owned file (R5) the private file maps to a source in the checkout, the driver compares
    the fields the owned file keeps and fails naming the first field that differs; a source missing
    from the checkout makes the run VOID.
R11. `--post-status` posts one commit status on the judged commit: context `box/packs`, state
    `success`, `failure` or `error`, and a description of a few words naming no row and no private
    tool. It is off by default, and it is not a required check.
R12. The pull request template says that a pack verdict comes from the maintainer's box run
    (`box/packs`).
R13. Public text names none of the maintainer's private tooling. Living docs, planned SPECs and
    proposed ADRs say "the box-run packs"; an accepted document takes a pure name replacement and
    one dated note citing ADR-059; a delivered SPEC whose acceptance command runs a removed file
    takes an insert-only amendment giving the criterion's box form.
R14. A criterion whose test this delivery removes is retired insert-only, every byte of its SPEC and
    its red-first record kept in order. Its id is struck (`~~A3~~`) in the SPEC's criteria table.
    Inserted fence lines move its command out of the `` ```acceptance `` fence into a
    `` ```retired `` fence, splitting the acceptance fence where the command stood (a fence whose
    first line is retired is closed empty before it), and move its lines in the red-first record
    into a `` ```retired `` fence the same way. The SPEC's dated amendment section names each
    retired criterion, why its subject is gone and its box form, and section 7 lists every
    retirement.
R15. The box run keeps the judgment of the removed advisory-waiver test. A pack whose entry in the
    private file names an advisory lint (today durable-services, whose lint is in the checkout,
    because the runner's card cuts each row's report short) runs that lint over the judged tree
    and reads every advisory finding by its unit and reason. Each finding must be waived in its unit
    (`X-DurableServices-Waive=<reason> <why>`, with a why of more than five words) or wait on an
    open issue the private file names. An unwaived departure, and a thin why, fail the pack by
    name. A waiver in a unit or a waiting entry that matches no finding is stale, and so is a
    waiting entry whose issue is closed. Every other pack's advisory rows still never fail.
R16. A public test reads every unit under `deploy/` with `scripts/tests/_units.py` and pins the set
    of `X-DurableServices-Waive=` declarations by unit and reason, each with a why of more than five
    words. Deleting or emptying a waiver therefore fails in public CI. Only the box run can see a
    new departure that carries no waiver (R15).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | no vendored pack file and no removed script is in the tree | `test_box_only_packs.py` `test_no_vendored_pack_file_or_removed_script_is_in_the_tree` |
| A2 | `methodology.json` names no vendored commit | `test_box_only_packs.py` `test_methodology_json_names_no_vendored_commit` |
| A3 | the gate has no `packs` stage, CI no `packs` job, and `ci.needs` no `packs` | `test_box_only_packs.py` `test_the_gate_and_ci_have_no_packs_stage_or_job` |
| A4 | no file of the tree reads a vendored path, and the census finds a planted one | `test_box_only_packs.py` `test_no_file_reads_a_vendored_path` |
| A5 | the gate never names its log directory on stdout, names a fresh one on stderr, and the directory a caller names holds every stage's log | `test_check_gate.py` `test_the_log_directory_is_never_on_stdout_and_holds_every_stage_log` |
| A6 | the rails parser refuses the owned rails without each field it reads | `the_rails_parser_refuses_the_owned_rails_without_each_field_it_reads` |
| A7 | the layout parser refuses the owned layout without each field it requires | `the_layout_parser_refuses_the_owned_layout_without_each_field_it_requires` |
| A8 | the gate-class parser refuses the owned classes without a field it reads | `staged::tests::the_gate_class_parser_refuses_the_owned_classes_without_a_field_it_reads` |
| A9 | the scrub finds a planted value of each public shape family from its own rules | `test_public_scrub.py` `test_each_public_shape_family_is_found_from_the_scrubs_own_rules` |
| A10 | the scrub's loader reads a synthetic private list in the private list's schema | `test_public_scrub.py` `test_the_loader_reads_a_synthetic_private_list_in_its_schema` |
| A11 | the driver refuses VOID by name without each of its three variables | `test_box_packs.py` `test_the_driver_refuses_void_by_name_without_its_private_variables` |
| A12 | the `packs` section keeps the removed row runner's judgment | `test_box_packs.py` `test_the_packs_section_keeps_the_row_runners_judgment` |
| A13 | the methodology probes and the proxy-client scan run from the checkout | `test_box_packs.py` `test_the_methodology_probes_and_the_scan_run_from_the_checkout` |
| A14 | an owned file equal to its source passes, a changed field fails by name, a missing source is VOID | `test_box_packs.py` `test_an_owned_file_is_judged_against_its_pinned_source` |
| A15 | `--post-status` posts exactly one verdict-only `box/packs` status | `test_box_packs.py` `test_post_status_posts_one_verdict_only_status` |
| A16 | the pull request template says a pack verdict comes from the box run | `test_box_only_packs.py` `test_the_pull_request_template_says_pack_verdicts_come_from_the_box_run` |
| A17 | the apiKeyHelper scan runs from the checkout: pending while no settings file exists, and failing on a finding, an unreadable file, VOID without an issue, or a stale or closed expectation | `test_box_packs.py` `test_the_api_key_helper_scan_runs_from_the_checkout_and_waits_for_a_settings_file` |
| A18 | every retired criterion is struck in its SPEC's table, fenced apart in its SPEC and its red-first record, and listed in section 7; a planted SPEC that breaks the rule is refused | `test_box_only_packs.py` `test_every_retired_criterion_is_struck_and_fenced_apart_in_its_spec_and_record` |
| A19 | an advisory departure its unit does not waive, and a thin why, fail the pack by name; a departure that waits on an open issue passes; a waiver or waiting entry that matches no departure, or waits on a closed issue, is stale | `test_box_packs.py` `test_every_advisory_departure_is_waived_in_its_unit_or_waits_on_an_open_issue` |
| A20 | the deploy templates' waivers are pinned by unit and reason, each with a why of more than five words | `test_deploy_templates.py` `test_every_advisory_waiver_is_pinned_with_its_why` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_box_only_packs.py -k test_no_vendored_pack_file_or_removed_script_is_in_the_tree
A2: python3 -m unittest discover -s scripts/tests -p test_box_only_packs.py -k test_methodology_json_names_no_vendored_commit
A3: python3 -m unittest discover -s scripts/tests -p test_box_only_packs.py -k test_the_gate_and_ci_have_no_packs_stage_or_job
A4: python3 -m unittest discover -s scripts/tests -p test_box_only_packs.py -k test_no_file_reads_a_vendored_path
A5: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_the_log_directory_is_never_on_stdout_and_holds_every_stage_log
A6: cargo test -p deck-streak-vault --test owned_data -- --exact the_rails_parser_refuses_the_owned_rails_without_each_field_it_reads
A7: cargo test -p deck-streak-vault --test owned_data -- --exact the_layout_parser_refuses_the_owned_layout_without_each_field_it_requires
A8: cargo test -p deck-streak-vault --lib -- --exact staged::tests::the_gate_class_parser_refuses_the_owned_classes_without_a_field_it_reads
A9: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_each_public_shape_family_is_found_from_the_scrubs_own_rules
A10: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_the_loader_reads_a_synthetic_private_list_in_its_schema
A11: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_the_driver_refuses_void_by_name_without_its_private_variables
A12: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_the_packs_section_keeps_the_row_runners_judgment
A13: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_the_methodology_probes_and_the_scan_run_from_the_checkout
A14: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_an_owned_file_is_judged_against_its_pinned_source
A15: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_post_status_posts_one_verdict_only_status
A16: python3 -m unittest discover -s scripts/tests -p test_box_only_packs.py -k test_the_pull_request_template_says_pack_verdicts_come_from_the_box_run
A17: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_the_api_key_helper_scan_runs_from_the_checkout_and_waits_for_a_settings_file
A18: python3 -m unittest discover -s scripts/tests -p test_box_only_packs.py -k test_every_retired_criterion_is_struck_and_fenced_apart_in_its_spec_and_record
A19: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_every_advisory_departure_is_waived_in_its_unit_or_waits_on_an_open_issue
A20: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_every_advisory_waiver_is_pinned_with_its_why
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `.packs/` (183 files) | repo | removed: the vendored skills, rows, probes, `VENDORED.json` and `wiring.json` (R1); the wiring moves to the maintainer's private file (R7) |
| `scripts/pack-rows.py` | repo | removed: the vendored rows' runner; the box driver judges every pack (R8) |
| `scripts/vendor-packs.py` | repo | removed: it refreshed the vendored copy |
| `scripts/methodology_probe.py`, `scripts/sdd-probe.py`, `scripts/ddd-probe.py`, `scripts/tdd-probe.py` | repo | removed: the box run executes them from the checkout (R9) |
| `scripts/no-apikeyhelper-scan.py` | repo | removed: the box run runs the pack's apiKeyHelper scan from the checkout instead (R9, A17) |
| `scripts/tests/test_vendor_packs.py` | repo | removed: it tests only `vendor-packs.py` |
| `scripts/tests/test_vendored_packs.py` | repo | removed: it tests only the vendored copy's digests |
| `scripts/tests/test_pack_wiring.py` | repo | removed: it tests only the vendored wiring and `pack-rows.py`; the judgment it proved is A12's |
| `scripts/tests/test_vault_rails_rows.py` | repo | removed: it runs only the vendored vault-duties probe, and bounds the build that run needs; the box run judges the rails rows |
| `scripts/tests/fixtures/box-packs/fake-runner` | repo | renamed from the old fake runner: it answers the runner's verbs with synthetic cards |
| `scripts/tests/fixtures/box-packs/checkout/skills/catalog.json` | repo | renamed from the old catalog fixture, with synthetic packs |
| `scripts/tests/fixtures/box-packs/checkout/` | repo | added: the synthetic checkout's catalog; the tests plant its probes, scans and sources at run time, and none names a private tool |
| `scripts/box-packs.sh` | repo | changed: the private-file driver, the apiKeyHelper scan and the advisory waivers (R7 to R11, R15) |
| `scripts/check.sh` | repo | changed: no `packs` stage, the scrub alone, the log directory off stdout (R3, R4) |
| `.github/workflows/ci.yml` | repo | changed: no `packs` job or need (R3); a comment names the guard test that still builds Rust |
| `.github/pull_request_template.md` | repo | changed: the box run's pack verdict (R12) |
| `methodology.json` | repo | changed: no `vendored_from` (R2) |
| `ruff.toml` | repo | changed: no vendored excludes |
| `stack.json` | repo | changed: no vendored radar path |
| `scripts/public-scrub.py` | repo | changed: its own rules and loader (R5, R6) |
| `scripts/scrub-rules/persona-core.json`, `scripts/scrub-rules/privacy-gdpr.json` | repo | added: the scrub's public shapes (R5) |
| `crates/vault/data/rails.json`, `crates/vault/data/layout.json`, `crates/vault/data/gate-classes.json` | `deck-streak-vault` | added: the adapter's rails, default layout and gate classes (R5) |
| `crates/vault/src/rails.rs`, `crates/vault/src/staged.rs`, `crates/vault/src/lib.rs` | `deck-streak-vault` | changed: read the crate's own data; A8's test; doc comments name the owned data |
| `crates/vault/tests/owned_data.rs` | `deck-streak-vault` | added: A6, A7 |
| `crates/vault/tests/staged.rs`, `crates/vault/tests/fixtures/gate/stand-in-probe.py` | `deck-streak-vault` | changed, and added: its red-class gate is a synthetic probe that judges only `note-links`; no assertion edited |
| `scripts/tests/_units.py` | repo | added: systemd unit syntax, as systemd.syntax(7) reads it, and each unit's waivers, for the deploy templates' tests |
| `scripts/tests/test_deploy_templates.py` | repo | changed: reads units with `_units.py`; A20. Its test of the durable lint's blocking rows is removed, because the box run's durable-services pack fails on each of them; its advisory-waiver test moves to the box run (R15) and to a public pin of the waivers (R16) |
| `scripts/tests/test_box_packs.py`, `scripts/tests/fixtures/box-packs/bin/gh` | repo | changed: the new interface, and A11 to A15, A17 and A19; the fake gh answers `api --method POST` |
| `scripts/tests/test_box_only_packs.py` | repo | added: A1 to A4, A16 and A18 |
| `scripts/tests/test_check_gate.py` | repo | changed: the stage table, A5, and a comment on the python stage's cargo |
| `scripts/tests/test_ci_workflows.py` | repo | changed: the job and stage tables, and a comment on the stages that compile Rust |
| `scripts/tests/test_public_scrub.py` | repo | changed: A9, A10 |
| `web/app/src/lib/styles/cjk.css`, `web/app/tests/a11y.spec.ts`, `web/app/tests/telegram-palettes.ts` | repo | changed: comments name the box-run packs |
| `CLAUDE.md`, `README.md`, `CONTRIBUTING.md`, `ARCHITECTURE.md`, `deploy/README.md`, `docs/BUILDER-BRIEF.md`, `docs/CONTEXT-MAP.md`, `docs/LEXICON.md`, `docs/TESTING.md` | repo | changed: the box-run packs (R13) |
| `docs/specs/planned/SPEC-026-bot-transport-and-owner-gate.md`, `SPEC-041-notification-router-core.md`, `SPEC-043-agent-core-runner-gate-and-degradation.md`, `SPEC-044-persona-engine-and-private-roster.md`, `SPEC-046-readings-generation-gates-and-repair.md` | repo | changed in place: the box-run packs (R13) |
| `docs/decisions/ADR-043-shell-runner-pack-gate-and-duty-caps.md` | repo | changed in place (proposed) |
| `docs/decisions/ADR-002`, `ADR-004`, `ADR-009`, `ADR-014`, `ADR-016`, `ADR-022`, `ADR-030`, `ADR-039`, `ADR-056` | repo | accepted: name replacement and one dated note (R13) |
| `docs/specs/SPEC-002`, `SPEC-024`, `SPEC-028`, `SPEC-030`, `SPEC-032`, `SPEC-037`, `SPEC-038`, `SPEC-042`, `SPEC-054` | repo | delivered: name replacement and one dated note, and the insert-only retirement of each criterion whose test this delivery removes (R13, R14) |
| `docs/schematics/box-pack-runner.md`, `pack-vendoring.md` | repo | accepted schematics: name replacement and one dated note |
| `docs/schematics/ci-jobs-and-caches.md` | repo | accepted schematic: one dated note, the `packs` job removed |
| `docs/schematics/agent-duty-run.md` | repo | changed in place (SPEC-043's, planned): the proxy's capacity endpoint named without its path |
| `docs/red-first/SPEC-002.md`, `SPEC-030.md`, `SPEC-032.md`, `SPEC-037.md`, `SPEC-038.md`, `SPEC-042.md`, `SPEC-054.md` | repo | accepted records: the retired criteria's lines set apart insert-only, and one dated note (R14); SPEC-030's, SPEC-037's and SPEC-054's also a name replacement |
| `changelog.d/chore-repin-packs-e54f39c.md`, `feat-repo-hygiene-030.md`, `feat-vendor-packs-037.md` | repo | changed: unreleased fragments name the box-run packs |
| `docs/specs/SPEC-056-every-pack-is-judged-on-the-box-and-nothing-of-the-hub-is-published.md` | repo | added: this SPEC |
| `docs/decisions/ADR-069-the-public-tree-carries-no-vendored-hub-files.md` | repo | added |
| `docs/schematics/pack-judgment-on-the-box.md` | repo | added: the gate, CI and the box run after the change |
| `scripts/mutation-rows.d/S05600-S05699.json` | repo | added: three hand-proved rows, one per owned vault data file, each killed by its parser test (A6 to A8) |
| `docs/red-first/SPEC-056.md` | repo | added |
| `changelog.d/chore-box-only-packs-056.md` | repo | added |

## 5. What this does NOT do

- It makes no pack verdict a required check: `box/packs` is posted, and a new required context
  would strand the pull requests opened before it (#60).
- It renames no `phx.*` schema id: the box-run packs judge DeckStreak's outputs by them, and a
  later rename is the maintainer's (#60).
- It changes no pack's rows. The private file carries today's wiring, with SPEC-021's and
  SPEC-031's changes and the apiKeyHelper scan's pending issue, less telegram-platform's two
  deferred rows, which only the vendored files kept red; its expected reds keep their issues (#60).
- It re-plans no planned SPEC's pack-row tests: SPEC-041, SPEC-043, SPEC-044, SPEC-046 and SPEC-051
  each plan a public test that runs a pack's rows, which the box run now judges, and each is
  re-planned when it is built (#60).
- It re-pins no pack, and builds nothing the box runs: the checkout, the runner and the private
  file stay the maintainer's (#60).
- It publishes no finding about a host: those stay with the owner gate that tracks them (#167).
- It adds no name of the maintainer's private tooling to the scrub's private list. Those names are
  checked by a separate private scan over the tree, the commit messages and the pull request body,
  because the history scrub reads every blob of history, and its list takes only terms that no
  historical blob holds (#60).

## 6. Risks

- **The owned data drifts from the pack that judges it.** Detected on the box by the drift check
  (R10, A14), which names the first field that differs.
- **A pull request merges with no pack verdict.** `box/packs` is visible on the pull request and the
  template asks for it (R11, R12); the maintainer's merge waits for the box run.
- **Public CI stops showing SPEC-shape verdicts.** Accepted: the box run judges them with the sdd,
  ddd and tdd probes before a merge (R9), which is the owner's box-only ruling applied fully.
- **The private file is lost or stale.** The driver refuses VOID by name without it (A11), and a pin
  that does not match the checkout refuses the run.
- **A stage-log upload uploads nothing.** Every CI job names `CHECK_LOG_DIR` and uploads that path,
  and A5 proves a named directory holds every stage's log and `timings.tsv`.

## 7. Retired criteria

Each criterion below ran a test this delivery removes. It is retired insert-only (R14): its id is
struck in its SPEC's table, its command and its red-first lines sit in `` ```retired `` fences,
and its SPEC's dated amendment section says why its subject is gone and what judges it now.

| SPEC | criterion | its subject | what judges it now |
|---|---|---|---|
| SPEC-002 | A3 | the vendored packs' digests | the drift check (A14); no vendored file remains (A1) |
| SPEC-002 | A4 | every waiting pack or row names an issue | the private file's checks (R8) and the issue states (SPEC-054 R4) |
| SPEC-002 | A10 | the row runner refuses a wiring that forgets a pack | nothing: the runner and the vendored packs are removed (A1) |
| SPEC-030 | A6 | a pending pack whose rows all pass is stale | A12 |
| SPEC-030 | A7 | a deferred row that passes is stale | A12 |
| SPEC-030 | A8 | a deferred row still red fails nothing | A12 |
| SPEC-032 | A1 | the durable lint over the deploy templates | the box run's durable-services pack |
| SPEC-037 | A1 | the vendoring never writes an excluded file | nothing: the vendoring is removed (A1) |
| SPEC-037 | A2 | the vendoring refuses a planted address | nothing: the vendoring is removed (A1) |
| SPEC-037 | A3 | the vendoring refuses a private literal | nothing: the vendoring is removed (A1) |
| SPEC-037 | A4 | the vendoring refuses a binary file | nothing: the vendoring is removed (A1) |
| SPEC-037 | A5 | the vendoring refuses a listed file the source lacks | nothing: the vendoring is removed (A1) |
| SPEC-037 | A6 | a clean upstream re-vendors with its digests | nothing: the vendoring is removed (A1) |
| SPEC-037 | A7 | every recorded exclusion is machine-applicable | nothing: the vendoring is removed (A1) |
| SPEC-038 | A8 | the row runner's pool gives the serial verdicts | nothing: the row runner is removed (A1) |
| SPEC-038 | A9 | the row runner's pool keeps its bound | nothing: the row runner is removed (A1) |
| SPEC-039 | A20 | every exclusion names its reason and an issue, and no `mutants::skip` exists | SPEC-057 A8, which refuses every exclusion (ADR-070) |
| SPEC-042 | A3 | the adapter's rails agree with the pack's `no-executable` class | the box run's vault-duties pack, and the drift check (A6, A14) |
| SPEC-042 | A9 | the rails rows are green over the synthetic run | the box run's vault-duties pack |
| SPEC-054 | A6 | the vendoring scans with the scrub's own rules | the scrub's own rules (A9, A10); the vendoring is removed (A1) |
| SPEC-054 | A10 | a bounded command fails by name | the box run's vault-duties pack, which runs the rows it bounded |
| SPEC-054 | A11 | the adapter's cargo run is bounded | the box run's vault-duties pack, which runs the rows it bounded |

## 8. Amendment, 2026-09-28: a retirement SPEC-057 made

Made by SPEC-057's first delivery, the vault's, insert-only under ruling (i) of SPEC-038 section 8:
every earlier byte is kept in order. That delivery retired SPEC-039's A20 in R14's form, when
ADR-070 replaced the exclusion A20 held to its reason with the equivalence record, and A18's test
holds section 7 equal to every retirement the delivered SPECs hold. It inserts:

- section 7: the row of SPEC-039's A20, in the table's order;
- this section.

Section 7's first sentence names this delivery's retirements; its table now also lists one a later
delivery made, as A18 requires.
