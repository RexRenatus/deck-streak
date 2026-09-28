# SPEC-054: the gate's own tools tell the truth

- **Wave:** W0. **Issue:** #230 (epic #1). **Context(s):** `repo` (`scripts/`, `tools/parity-oracle/`).
- **Decided by:** ADR-033 (the public scrub), ADR-039 (vendoring reuses the scrub's rules, never
  copies them), ADR-030 and ADR-056 (the box-pack runner, and the packs staying box-only), ADR-042
  (the vault's rails guard) and ADR-012 (the parity oracle). Each change here applies one of them;
  none needs a new decision.
- **Status:** judged: written at delivery, because it had no planned copy, and delivered with its
  tests and `docs/red-first/SPEC-054.md` (ADR-016).

## 1. The problem, measured

Six tools of the gate misreport. Each was measured at `dev` c0dbf2a.

1. **A file given as `--subject` examines nothing.** `scripts/public-scrub.py` walks every subject
   with `rglob`, which yields nothing for a file. `public-scrub.py --root . --no-tree --subject
   <a clean body file>` printed `examined 0 file(s) against public shapes only; 0 finding(s)` and
   exited 3 (VOID), so a builder's natural call on a pull request's body reads VOID. A subject that
   does not exist printed the same line and exit, so it is not refused by name. A subject that
   examines nothing beside the tree passes silently: `--root <a repository of one clean file>
   --subject <an empty directory>` printed `examined 1 file(s)` and exited 0.
2. **The email rule refuses a systemd instance unit name.** persona-core's `email` shape reads a
   unit type as a top-level domain, so a note that says `getty@tty1.service` in prose was refused
   `note.md:1: email`, exit 1. systemd names a template's instance this way (systemd.unit(5)), and
   no mail can reach it.
3. **The deny-list composition is written twice.** `public-scrub.py`'s `main()` (lines 250 to 260)
   and `scripts/vendor-packs.py`'s `rules()` (lines 98 to 114) each load persona-core's list with
   the private list and privacy-gdpr's list, and flatten them into a `Scan`. ADR-039 says the
   scrub's rules are reused, never copied. The copies already differ: a deny list the probe refuses
   stops the vendoring with exit 2, and crashes the scrub with a traceback and exit 1, which reads
   as a finding.
4. **The box runner cannot see issue state.** `.packs/wiring.json`'s `box` section names 29
   `expected_red` rows, 2 `pending` packs and the proxy scan's `pending`, over four issues (#25,
   #29, #59, #60). `scripts/box-packs.sh` reads no issue (it makes no `gh` call), so an expectation
   keeps passing after its issue is closed, and each merge's verification checks them by hand. All
   four are open today.
5. **The rails guard's cargo call has no bound.** `scripts/tests/test_vault_rails_rows.py` bounds
   each probe call at 120 s, and runs `cargo run --quiet --locked -p deck-streak-vault --example
   rails_verdicts` with no timeout. It is the first Python guard to call cargo, so a hung build
   holds the gate's python stage with no reason given: in CI until the hygiene job's 30-minute
   limit kills the job, and on a machine indefinitely. A cold run of the example on the
   maintainer's machine, with an empty target directory and the crates already downloaded, took
   21.1 s. CI's whole python stage takes 44 s with its dependency cache.
6. **The `{day:N}` token is undocumented.** SPEC-042's golden `roll_note_text.json` carries 299
   `{day:N}` tokens, written and read by `registry/spec_042.py`. `tools/parity-oracle/README.md`
   names the token 0 times, so the next registry module whose text carries a date has no pattern to
   follow, and a golden that holds the raw date is refused by `test_goldens.py`.

## 2. Requirements

R1. **Subjects.** Each `--subject PATH` of `public-scrub.py` is examined as follows:
    - a file is its own subject, examined alone;
    - a directory means every file under it, as before;
    - a path that does not exist, or is neither a file nor a directory, stops the run with exit 2
      before anything is read, printing `public-scrub: the subject <PATH> does not exist` (or
      `is not a file or a directory`).
    A subject that examined no file (an empty directory, or only files the scrub skips) makes the
    run VOID. The run prints `public-scrub: VOID: the subject <PATH> examined no file` and exits 3,
    even when the tree or another subject was examined. A finding still exits 1 first.
R2. **Unit instance names.** A match of the rule `email` passes when its last dot-separated label
    is exactly one of eight systemd unit types, lowercase as systemd writes them: `service`,
    `timer`, `socket`, `path`, `mount`, `target`, `slice` or `scope`. Such a match is a unit
    instance name. Every other match is still refused, by the rule's name and never its value:
    - a domain that holds such a word anywhere but as its whole last label, such as
      `service.<corp>.com`, `<corp>.services` or `<corp>.webservice`;
    - a unit type written in capitals.
    The admission lives in the scrub's `Scan`, so the vendoring's scan (R3) applies it too.
R3. **One composition.** `public-scrub.py` holds the composition once, `rules(private)`:
    persona-core's deny list with the private list `private` (a path, or `None`), then
    privacy-gdpr's, returned as a `Scan`.
    - A private list that is not a file, or a deny list the probe refuses, raises the scrub's
      `RulesError`, which names it.
    - `main()` builds its scan with `rules()`, and reports a `RulesError` with exit 2, never a
      traceback.
    - `vendor-packs.py` calls `rules()` and composes nothing itself: no `load_deny`, no pack path,
      no second flattening.
    - Vendor-packs' output stays byte-identical, for a refusal and for a run. A run against the
      vendored commit, into a scratch copy of the tree and with the maintainer's private list,
      prints the same lines and exit code before and after this change.
R4. **Issue state on the box.** After it has read the judged commit's wiring, and before any pack
    runs, `box-packs.sh` reads the state of every issue the `box` section names: each
    `expected_red` row's issue, each `pending` issue, and the proxy scan's `pending`. It asks once
    per issue, with `gh issue view <n> --json state`, run in ROOT, so `gh` resolves the repository
    from ROOT's remotes, or from `$GH_REPO`.
    - An expectation whose issue reads `CLOSED` is stale. Its pack's line reads `FAIL` and names
      the row (or `pending`) and the closed issue, as `<row> (#N is closed)`, and the run exits 1.
    - The run is VOID when `gh` is not on the path, is not logged in (its exit 4), cannot reach
      GitHub (any other failure), or answers neither `OPEN` nor `CLOSED`. It prints
      `box-packs: VOID: <the reason>`, naming the issue, runs no pack, and exits 2, the runner's
      "cannot judge" (ADR-030). It never passes.
    `.packs/wiring.json` and its schema are unchanged.
R5. **A bounded build in the rails guard.** The guard runs the adapter's `cargo run` under a bound
    of 900 s. That is over 40 times the measured cold run, so a slower runner, a cache miss or a
    wait for a build slot on a shared machine still finishes inside it. It is also inside CI's
    30-minute hygiene job, so the guard names the failure before the runner kills the job. When
    the bound expires, the child is killed and the test fails with `<what> did not finish within
    <N> s: <the command>`.
R6. **The day token, documented.** `tools/parity-oracle/README.md` documents `{day:N}`:
    - what it encodes: a calendar day inside a text a golden carries, N its epoch day number, which
      is negative before 1970-01-01;
    - why it is lossless: each day has one token and each token one day, the adapter expands every
      token before the call and contracts every date the predecessor returns, and it refuses a
      case whose text holds a raw date or a date that does not read back as itself;
    - how a registry module emits it: `registry/spec_042.py`'s adapter `with_days_as_numbers`, with
      its `expand` and `contract`;
    - a synthetic example, in the golden's form and as the predecessor reads it.
    No registry module and no golden changes, so no golden's digest moves.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a file given as `--subject` is its own subject: an address planted in it is refused by path and rule, and a clean one passes having examined 1 file | `test_public_scrub.py` |
| A2 | a subject that does not exist stops the scrub with exit 2, by name | `test_public_scrub.py` |
| A3 | a subject that examines nothing is VOID by name, although the tree beside it was examined | `test_public_scrub.py` |
| A4 | a unit instance name of each of the eight types passes the email rule, also beside an address on the same line, which is still refused | `test_public_scrub.py` |
| A5 | an address that only looks like a unit name is still refused by `email`, never by value | `test_public_scrub.py` |
| A6 | the vendoring scans with the scrub's own `rules()`, holds no second composition, and refuses an unreadable private list as before | `test_vendor_packs.py` |
| A7 | a private list the rules cannot read stops the scrub with exit 2, by name | `test_public_scrub.py` |
| A8 | an expectation whose issue is closed fails the box run, naming the pack, the row and the issue | `test_box_packs.py` |
| A9 | a box run that cannot read an issue's state (no `gh`, not logged in, offline) is VOID with the reason, and runs no pack | `test_box_packs.py` |
| A10 | a command that outlives its bound fails by name, and one inside it returns | `test_vault_rails_rows.py` |
| A11 | the adapter's cargo run is bounded at 900 s | `test_vault_rails_rows.py` |
| A12 | the README's day-token example round-trips through the oracle's own reader and writer | `tools/parity-oracle/test_goldens.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_file_given_as_subject_is_its_own_subject
A2: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_subject_that_does_not_exist_is_refused_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_subject_that_examines_nothing_is_void_by_name
A4: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_systemd_unit_instance_name_passes_the_email_rule
A5: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k an_address_that_only_looks_like_a_unit_name_is_still_refused
A6: python3 -m unittest discover -s scripts/tests -p test_vendor_packs.py -k the_vendoring_scans_with_the_scrubs_own_rules
A7: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_private_list_the_rules_cannot_read_stops_the_scrub
A8: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k an_expectation_whose_issue_is_closed_fails_the_run
A9: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k a_run_that_cannot_read_issue_state_is_void
A10: python3 -m unittest discover -s scripts/tests -p test_vault_rails_rows.py -k a_command_that_outlives_its_bound_fails_by_name
A11: python3 -m unittest discover -s scripts/tests -p test_vault_rails_rows.py -k the_adapter_build_runs_under_its_bound
A12: python3 -m unittest discover -s tools/parity-oracle -p test_goldens.py -k the_readme_day_token_example_round_trips
```

The planted addresses are assembled at run time, so no test file holds one. A8 and A9 drive the
real runner with the fake phxd and a fake `gh` on the path, which answers `CLOSED` for the issues a
test names and `OPEN` for any other. Every other box-runner test gets the same fake, answering
`OPEN`. A9's missing `gh` is a path that holds the runner's own tools and no `gh`. A10 plants a
slow command, and never runs cargo. A11 replaces the process call and records its bound, so it runs
no cargo either.

The red stub, committed with the tests, is the base's behaviour, with one refactor: the guard's
cargo call moves into a helper that still passes no timeout. A1 to A4, A7, A8 and A9 are then red on
the base's exit codes. A6 is red because the vendoring calls `load_deny` itself. A10 is red because
the planted command runs to its end, and A11 because the recorded bound is `None`. A12 is red
because the README holds no example to examine. A5 holds on the base, which refuses every address:
it pins the admission's edges, so a looser admission cannot pass.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/public-scrub.py` | `repo` | changed: R1, R2, R3 |
| `scripts/vendor-packs.py` | `repo` | changed: R3 |
| `scripts/box-packs.sh` | `repo` | changed: R4 |
| `scripts/tests/test_public_scrub.py` | `repo` | changed: A1 to A5, A7 |
| `scripts/tests/test_vendor_packs.py` | `repo` | changed: A6 |
| `scripts/tests/test_box_packs.py` | `repo` | changed: A8, A9, and the fake `gh` on every run's path |
| `scripts/tests/fixtures/box-packs/bin/gh` | `repo` | added: the fake `gh` |
| `scripts/tests/test_vault_rails_rows.py` | `repo` | changed: R5, A10, A11 |
| `tools/parity-oracle/README.md` | `repo` | changed: R6 |
| `tools/parity-oracle/test_goldens.py` | `repo` | changed: A12 |
| `docs/schematics/box-pack-runner.md` | `repo` | changed: the issue-state step and its verdicts |
| `docs/schematics/pack-vendoring.md` | `repo` | changed: the scan composes through `rules()` |
| `docs/TESTING.md` | `repo` | changed: the box runner reads each named issue's state |
| `docs/specs/SPEC-054-the-gates-own-tools-tell-the-truth.md` | `repo` | added |
| `docs/red-first/SPEC-054.md` | `repo` | added |
| `changelog.d/feat-tooling-054.md` | `repo` | added |

## 5. What this does NOT do

- It changes no pack's deny list. persona-core's `email` shape is the pack's own, and the output
  gate that judges AI text keeps it as the pack ships it (#29); the admission lives only in the
  public scrub.
- It adds no mutation row and generates no mutants. The rows' directory and their runner arrive
  with SPEC-039 (#217), and generated mutants for the gate's own Python are #218. The red-first
  record proves this delivery's mutants by hand instead.
- It changes neither `.packs/wiring.json` nor its schema, and judges no pack differently. The runner
  only reads the state of the issues the wiring already names, and bringing every pack to green
  stays #60.

## 6. Risks

- **An address under a top-level domain named like a unit type passes.** IANA's list names `target`
  as a top-level domain, so an address under it would pass R2. The scrub's private list still names
  the owner's own values by literal, and R2's edges are pinned by A5.
- **A `gh` outage makes every box run VOID.** That is by design, because VOID is never a pass.
  `$GH_REPO` covers a checkout whose remotes `gh` cannot resolve.
- **The rails guard's bound is too short for some machine.** The failure then names the bound and
  the command, and the bound is one constant in the guard, so it is visible and cheap to revise
  with a new measurement.
- **The composition's refactor changes the vendoring.** A6 pins the refusal lines, and the dry run
  before and after, recorded in the red-first record, compares a whole run's output.
