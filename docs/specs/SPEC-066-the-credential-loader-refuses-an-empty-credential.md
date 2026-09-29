# SPEC-066: the credential loader refuses an empty credential by its id, and the refusal fails its unit

- **Wave:** W2. **Issue:** #284 (epic #3). **Context(s):** `deck-streak-kernel` (the loader and its
  error), `deck-streak-ingest` (a test of the sync's login, and the engine probe's login), `deploy`
  (the alert unit's script, and the census of the unit templates).
- **Decided by:** ADR-067 (this SPEC's own: the loader refuses an empty credential, and what that
  was chosen against), ADR-038 (each credential is read from the credential socket at every start;
  it takes a dated note naming ADR-067), ADR-010 (each unit pages through `OnFailure=` the alert
  template), SPEC-020 R11 (the loader) and SPEC-031 (the one alert path).
- **Status:** judged: delivered with its tests, its hand-proved rows and
  `docs/red-first/SPEC-066.md` (ADR-016). It waited in `docs/specs/planned/` from its own commit
  until its tests were green; §7 lists what the delivery amended.

## 1. The problem, measured

Measured at `dev` 53184dd.

- **The loader refuses a missing credential, and returns any other.** `CredentialLoader::load`
  (`crates/kernel/src/credentials.rs`) refuses an id that is not a plain file name, a file that is
  absent (`CredentialError::Missing`, SPEC-020 R11), a file it cannot read and one that is not
  UTF-8. Every other file loads as a `Secret`, less one trailing newline. A file of zero bytes and a
  file holding only a newline each load as an empty `Secret`, and `CredentialError` has no variant
  that names an empty credential. A1 observed both on the base, before the implementation
  (`docs/red-first/SPEC-066.md`).
- **The manual documents no start failure for an empty credential.** ADR-038 reads every
  credential as `LoadCredential=<id>:<the credential socket>`. systemd.exec(5) describes that
  credential as "the credential data read from the connection", and bounds a unit's credentials
  from above only: "an accumulated credential size limit of 1 MB per unit". It states no lower
  bound, and no start failure when the data read is empty.
- **Ten credential lines in four templates.** The census over `deploy/`
  (`scripts/tests/test_deploy_templates.py`, `credential_lines`) reads 10 `LoadCredential=` lines in
  4 of the 6 service templates: the api's 2, the bot's 4, the job template's 2 and the alert
  template's 2. The first three name `OnFailure=deck-streak-alert@%n.service`. The alert template
  names none, because a page that fails must not start a page about the page (SPEC-031).
- **Four readers of a credential: two through the loader, two outside it.**
  - The api refuses an empty owner id or token already, in the application: identity's owner gate
    (`crates/identity/src/owner.rs`) passes a refusal of the loader on as
    `IdentityError::Credential`, and refuses a blank value by its own shape check as
    `IdentityError::Malformed` (SPEC-024). The bot role, which `dev` gained with SPEC-026 while
    this delivery was open (merged at f5322b2), reads the same two credentials through the
    loader and holds the same shape checks (`crates/daemon/src/role_bot.rs`). This SPEC adds
    nothing to either check.
  - The sync's login (`crates/ingest/src/sync.rs`, `Syncer::login`) reads both of its credentials
    through the loader, records every refusal of the loader as the run's `missing_credentials`
    outcome with no attempt made (SPEC-022 R9), and hands any value the loader returns to the
    engine as the login.
  - The alert unit's script (`deploy/scripts/alert-telegram.sh`) does not use the loader: it reads
    its two credentials with `cat` (SPEC-031 R3).
  - The engine probe (`crates/ingest/examples/engine_probe.rs`), the example a person runs by hand
    to drive the ingest port and `engine-measure.yml` builds for its size (SPEC-022 R2), reads the
    sync's two credentials from the credentials directory with its own read of each file, not the
    loader.

## 2. Requirements

R1. `CredentialLoader::load(id)` refuses an EMPTY credential, a file of zero bytes or one that
    holds nothing once the one trailing newline it trims is removed, with `CredentialError::Empty`
    naming the id, before the value is registered with the redactor or returned. The refusal's
    `Display` is `the credential <id> is empty in the credentials directory`, and its `Debug` is
    `Empty { id: "<id>" }`: each names the id, and neither carries a value or the directory's path.
    Everything else keeps its behaviour: a missing file refuses as `Missing`, an id that is not a
    plain file name as `InvalidId`, an unreadable file as `Unreadable`, one that is not UTF-8 as
    `NotText`, and a value of one character or more loads unchanged, less one trailing newline, so
    a file of two newlines loads as one newline.
R2. The refusal fails the unit that loads the credential, and that unit's `OnFailure=` starts the
    one page, which quotes the failed run's error lines (SPEC-031 R3):
    - the `api` role reads its credentials before it binds, and the `bot` role before its first
      request to the Bot API (SPEC-026); a role that refuses start writes the refusal, which
      names the id, at error priority and exits 1 (SPEC-025 R1);
    - the `sync` job's login records the refusal as the run's `missing_credentials` outcome, with
      no attempt made, so an empty login never reaches the sync engine (SPEC-022 R9), and the
      runner exits 1 to page, with that reason code, when the failure opens the job's error streak
      (SPEC-027 R7). A repeat failure only logs, and the liveness job pages once the episode
      outlives its window (SPEC-027 R8);
    - every unit template under `deploy/` that loads a credential, the alert template excepted,
      names `OnFailure=deck-streak-alert@%n.service`, names no `ExecCondition=` and no `[Unit]`
      condition or assertion (a `Condition*=` or an `Assert*=`, an empty one included), starts its
      `ExecStart=` without the `-` prefix, holds no `SuccessExitStatus=` word that reads as 1, and
      sets no `RestartMode=direct`. A condition that exits 1 to 254 skips the start, which neither
      fails the unit nor starts `OnFailure=`, and a `[Unit]` condition or assertion can stop the
      start before it runs; the prefix and the exit status would each count a refused start as a
      success; and the restart mode skips `OnFailure=` (systemd.service(5), systemd.unit(5)).
      The census does not model how systemd reads a unit file: it reads the plain syntax the
      templates hold, and refuses the rest, on every template it reads:
      - a line that ends in a backslash; a control character other than a tab; whitespace outside
        ASCII; and a line that is neither blank, a comment, a section header nor an assignment in
        a section. Lines end at a newline alone;
      - an exit-status word that is neither a decimal of at most 255, with no sign and no leading
        zero, nor a status name `systemd-analyze exit-status` lists, where a value splits into
        words at spaces and tabs;
      - a `Restart=`, `RestartMode=` or `CollectMode=` that is empty or not a value its manual
        lists, each read at every assignment, so the census never decides which of two is in
        force;
      - a key that is off the list of its unit's kind. The tests hold a literal list of
        `(section, key)` pairs for the alert template and another for the units that load a
        credential and page on failure, `OnFailure=` being on the second alone. Every assignment
        of a unit, its own drop-ins included, is checked against its list, and one off it, in any
        section, is refused by its key: a `Requisite=`, a `Requires=`, a `BindsTo=`, an `X-` key,
        or a key the list holds in another section. The lists are the keys those units use, and
        the tests hold each list equal to the `(section, key)` pairs its units hold, drop-ins
        included;
      - a value off the table of a key of a unit that loads a credential and pages on failure:
        `Restart=` holds `on-failure`, and the restart budget and ordering hold the values those units
        use (`StartLimitIntervalSec=300`, `StartLimitBurst=5`, `RestartSec=15`,
        `After=` and `Wants=` `network-online.target`), at every assignment, its drop-ins
        included, and any other value is refused by its key and value;
      - a unit that loads a credential and pages on failure and assigns `Restart=` to anything but
        `no`, in the unit or a drop-in, that holds no `StartLimitIntervalSec=`,
        `StartLimitBurst=` or `RestartSec=`: each key it lacks is refused by name. Without the
        start limit the default interval is shorter than the restart delay, so the unit never
        reaches failed and pages on every restart;
      - an `OnFailure=` of such a unit that is not exactly the alert template, at every
        assignment, its drop-ins included: a target beside it, in its place, or an empty one is
        refused by its key and value;
      - a `*.d/` directory under `deploy/` that is not the `<unit name>.d/` of a unit shipped
        beside it, the one directory of a file that is no unit excepted by name. Only a unit's own
        drop-in directory is read with it;
      - a credential directive read by the raw form of a line: the check of R2's credential form
        reads each unit file and drop-in through the same reader, so a blank before `=` is a key
        like any other.
R3. The alert template unit cannot page through itself: it names no `OnFailure=` (SPEC-031). Its own
    refusal is therefore surfaced as its failed state. `deploy/scripts/alert-telegram.sh` refuses
    each of the two credentials it loads whose value is empty, nothing being left once the command
    substitution that reads it has removed its trailing newlines, before it reads the journal or
    makes a request. It writes one line at error priority to standard error,
    `<3>the credential <id> is empty in the credentials directory: no page is sent`, which the
    journal keeps under the unit's identifier, and exits 1. The alert template counts no refusal a
    success and leaves it failed, by four exit conditions: no `-` prefix, and no
    `SuccessExitStatus=` at all, so that no status can count the refusal a success; no
    `RestartMode=direct`, which skips the failed state on a restart; and no `ExecCondition=`, since
    a condition that exits 1 to 254 skips the start and leaves the instance inactive, not failed
    (systemd.service(5)). It names no `[Unit]` condition or assertion either, an empty one
    included, since one can stop the start before it runs (systemd.unit(5)). It also restarts no
    refused start: no `Restart=` other than `no`, and no `RestartForceExitStatus=` at all: on its
    `Type=oneshot` the service manager refuses the unit outright (a bad unit file setting), and on
    another type one naming 1 forces a restart whatever `Restart=` says. A restart at the default
    mode only passes through the failed state, and the instance waits for its next start
    activating, not failed (systemd.service(5)), so a loop of restarts settles failed only when its
    start limit ends it (SPEC-031). And it is never unloaded while failed: no `CollectMode=` other
    than `inactive`, since `inactive-or-failed` unloads the failed instance, which
    `systemctl --failed` then no longer lists (systemd.unit(5)). Its `Restart=`, `RestartMode=`
    and `CollectMode=` are read at every assignment, and one that is empty or not a value its
    manual lists is refused, as R2 refuses it; the template is read by R2's reader, which refuses
    the lines R2 lists, and its keys are held to the alert template's list, which carries no
    dependency directive (`Requisite=`, `Requires=`, `BindsTo=`) and no `OnFailure=`, since a start
    that a dependency stops leaves the instance not failed. A template these checks admit therefore leaves a refused instance `failed`
    and listed by `systemctl --failed`. No page reports it: a second route that does not depend on
    the alert sender is #285.
R4. ADR-067 records the decision and what it was chosen against. ADR-038 takes one dated note at its
    end, a pure append, naming the loader, not the service manager, as what refuses an empty
    credential.
R5. Where cargo-mutants makes no mutant, hand-proved rows in
    `scripts/mutation-rows.d/S06600-S06699.json` guard the refusal (SPEC-039 R8): the loader's
    check, the variant it returns, the id in its `Display`, the bound that admits a value of one
    character, and the script's check, its exit and its check of each of its two credentials, the
    table of the values the units that load a credential and page on failure admit, the check
    that their `OnFailure=` is the alert template alone, and the check that a unit which restarts
    holds its whole restart budget.
R6. The engine probe (`crates/ingest/examples/engine_probe.rs`) reads the sync's two credentials
    through the loader, by the ids `deck_streak_ingest::settings` declares, as the sync's login
    does. A refusal ends it before it builds a login: it writes the refusal's `Display`, which names
    the id and never a value, to standard error and exits 1.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a credential of zero bytes, and one holding only a newline, each refuse to load with `CredentialError::Empty` naming the id, and the refusal says the credential is empty | `credentials.rs` `an_empty_credential_refuses_start_by_its_id` |
| A2 | beside them, a missing credential keeps its `Missing` refusal, an unreadable one (a directory at its path) its `Unreadable` refusal, and a value loads unchanged less one trailing newline: one character, and a file of two newlines as one newline | `credentials.rs` `a_missing_credential_keeps_its_refusal_and_a_value_loads_unchanged` |
| A3 | the refusal's `Display` and `Debug` carry the id only: a sentinel planted as the directory's name, and as a sibling credential the same loader read first, is in neither | `credentials.rs` `an_empty_refusal_names_the_id_and_never_a_value` |
| A4 | every unit template that loads a credential, the alert template excepted, fails and pages on a refused start (R2's conditions), and the alert template counts no refusal a success, names no `[Unit]` condition or assertion, restarts none and is never unloaded while failed (R3's four exit conditions, told from `OnFailure=` by the condition and never by a refusal's text, its restart and its collection); the census reads every template with one reader, which refuses the lines R2 lists by file and line, reads an exit-status word only as a decimal of at most 255 or a status name, held to a table of words, and refuses every other word and every empty or unknown `Restart=`, `RestartMode=` or `CollectMode=`; it prints its examined count, refuses zero, refuses a planted template for each condition and each refusal, and alert-shaped ones, which name no `OnFailure=`, each for what it breaks (listed below), and admits none of a cross-check corpus of exit-status words | `test_deploy_templates.py` `every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal` |
| A5 | the alert unit's route (R3): each credential the template loads, empty in each form, makes the script exit 1 with the one line naming it, before any journal read or request; the template, read by A4's reader, names no `OnFailure=`, counts no refusal a success, restarts none and is never unloaded while failed: no `-` prefix, no `ExecCondition=`, no `[Unit]` condition or assertion, no `SuccessExitStatus=` at all and no `RestartMode=direct`; no `Restart=` other than `no` and no `RestartForceExitStatus=` at all; no `CollectMode=` other than `inactive`; and no `Restart=`, `RestartMode=` or `CollectMode=` that is empty or not a known value, each read at every assignment; each of these planted on the template, and a line the reader refuses, is refused | `test_alert_unit.py` `an_empty_credential_fails_the_alert_unit_before_any_request` |
| A6 | the sync's login reads through the loader (R2): each of its two credentials, empty in each form, records the run as `missing_credentials` with no attempt, and the engine is never asked to sync | `retry.rs` `an_empty_sync_credential_is_recorded_missing_and_never_reaches_the_engine` |
| A7 | a unit that loads a credential and pages on failure and assigns `Restart=` to anything but `no` holds `StartLimitIntervalSec=`, `StartLimitBurst=` and `RestartSec=`, in the unit or a drop-in, and a unit that lacks one is refused by the key it lacks; the tree's units hold all three | `test_deploy_templates.py` `a_restarting_paging_unit_holds_the_whole_restart_budget` |

```acceptance
A1: cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_credential_refuses_start_by_its_id
A2: cargo test -p deck-streak-kernel --test credentials -- --exact a_missing_credential_keeps_its_refusal_and_a_value_loads_unchanged
A3: cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_refusal_names_the_id_and_never_a_value
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal
A5: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k an_empty_credential_fails_the_alert_unit_before_any_request
A6: cargo test -p deck-streak-ingest --test retry -- --exact an_empty_sync_credential_is_recorded_missing_and_never_reaches_the_engine
A7: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k a_restarting_paging_unit_holds_the_whole_restart_budget
```

A1 to A3 write synthetic credentials to a temporary directory and read them through the loader.
A4 reads the templates with `_units.py`, as the rest of the census does. Its reader does not
model how systemd reads a unit file: it reads the plain syntax the templates hold, and refuses each
line R2 lists by its file and line (`logical_lines`, `assignments`). A5 reads the alert template
with the same reader, so the two cannot drift, and both hold a unit to the literal list of its
kind (`ALERT_KEYS`, `PAGING_KEYS`), refusing any key off it by name, and A4 holds each list equal to the pairs its units hold; a
table (`PAGING_VALUES`) holds the values the units that load a credential and page on failure
admit (their one `Restart=` value and their restart budget and ordering), and every `OnFailure=` of
those units is the alert template alone, each checked at every assignment, drop-ins
included, with plants for `Restart=always` and `Restart=on-success` on a `Type=oneshot` unit, for
`Restart=always` in a drop-in, for a restart value extending the admitted one, for a start limit
interval, a start limit burst, a restart delay and an ordering or pull-in other than the admitted
one, for a target beside
the alert's, in its place, after it and reset, and in a drop-in, and the admitted controls; only a unit's own `<name>.d/`
is read with it, and A4 refuses any other `*.d/` directory under `deploy/`. A value splits into exit-status words at spaces and
tabs alone (`status_words`), and a word is read only as a decimal of at most 255, with no sign and
no leading zero, or as a status name `systemd-analyze exit-status` lists (`exit_status`); the
census refuses every other word. A4 holds that reading to a table of words, each with its reading
or none. `Restart=`, `RestartMode=` and `CollectMode=` are read at every assignment, and a value
that is empty or not one their manuals list is refused. A4 plants one template for each of R2's
conditions and for each of its refusals: exit statuses the census does not read, an empty or
unknown restart value, and a `[Unit]` condition and an assertion. It holds the alert template to
R3's four exit conditions, the `ExecStart=` prefix, `SuccessExitStatus=`, `RestartMode=` and
`ExecCondition=`, which are R2's conditions but `OnFailure=`, told apart by the condition each
refusal names and never by its text, with a `SuccessExitStatus=` that names no 1 refused too; to
its `[Unit]` conditions; to R3's restart, `Restart=` and any `RestartForceExitStatus=`; and to its
collection, `CollectMode=`. It plants templates shaped as the alert template is, each loading a
credential and naming no `OnFailure=`, and each refused for what it breaks: `SuccessExitStatus=1`,
and exit statuses the census does not read, by the exit conditions; `Restart=on-failure` beside
`RestartMode=direct`, by the exit conditions for its `RestartMode=` and by the restart for its
`Restart=`; `RestartForceExitStatus=1` alone, by the restart; an `ExecCondition=`, by the exit
conditions; `CollectMode=inactive-or-failed`, by the collection; `SuccessExitStatus=2` beside
`RestartForceExitStatus=2`, which R2 admits and the alert template's checks refuse; a
`Type=oneshot` one naming `RestartForceExitStatus=2`, which the service manager refuses outright;
a `Restart=` or `CollectMode=` assigned and then reset, and an unknown `RestartMode=`; and a
`[Unit]` condition and an assertion. It plants a template for each kind of line the reader
refuses, which is refused by its file and line, and one holding a tab and a `§` in a comment,
which is read. And it plants each word of a cross-check corpus of exit-status words, as a paging
template's `SuccessExitStatus=` and as an alert-shaped one's `SuccessExitStatus=` and
`RestartForceExitStatus=`, and admits none. A5 runs the script as its unit runs it, with the
recording stubs of SPEC-031's tests first on its `PATH`, once for each credential the template
loads and each empty form, zero bytes and a lone newline, the other credential holding its
synthetic value, and reads the same four exit conditions, the `[Unit]` conditions, the restart
and the collection in the template. It plants on the template each thing R3 refuses, a reset or
unknown restart or collect value, a `[Unit]` condition or assertion, and a line the reader
refuses, and each is refused.
A7 plants a unit that loads a credential and pages, assigns `Restart=on-failure`, and lacks keys of
the restart budget (`RESTART_BUDGET`): a restart with a delay and no start limit, a restart with no
budget at all, and a restart with no burst, no interval or no delay, each refused by every key it
lacks, beside controls that assign `Restart=no`, assign no `Restart=`, or hold the whole budget,
none refused. The value table holds a key's value and cannot hold that a key is present, so a unit
whose every assigned value is admitted can still page without bound.
A6 runs the syncer over SPEC-022's scripted engine and in-memory record, which count every sync
the engine is asked for, with one of the fixture's two credentials rewritten empty. R6 takes no
criterion of its own: the engine probe is an example a person runs by hand, with no test, and the
refusal it takes is the loader's, which A1 to A3 hold.

The pull request's mutation jobs decide #284's last criterion, and no command here does: the
diff-scoped jobs (SPEC-039 R3, R4) must read the `rust` class examined, with a count above zero and
no survivor. cargo-mutants 27.1.0 mutates no attribute or string, and `cargo mutants --list` over
the loader at the base lists the operators, the match guard and the whole body of `load`, but no
mutant of the trim's `if` condition, so none of an `if` the refusal adds. The rows of R5 carry the
refusal: each counts as examined for the changed line its anchor overlaps (SPEC-039 R10), and each
is proved with `python3 scripts/mutation_rows.py prove --band S06600-S06699`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/credentials.rs` | `deck-streak-kernel` | changed: R1, the refusal of an empty credential |
| `crates/kernel/src/error.rs` | `deck-streak-kernel` | changed: R1, `CredentialError::Empty` |
| `crates/kernel/tests/credentials.rs` | `deck-streak-kernel` | changed: A1 to A3 |
| `crates/ingest/tests/retry.rs` | `deck-streak-ingest` | changed: A6 |
| `crates/ingest/examples/engine_probe.rs` | `deck-streak-ingest` | changed: R6, the probe reads its sync login through the loader |
| `deploy/scripts/alert-telegram.sh` | deploy | changed: R3 |
| `deploy/README.md` | deploy | changed: an empty credential refuses start, and the alert unit's own refusal |
| `scripts/tests/test_deploy_templates.py` | repo | changed: A4 and A7, the key lists' check, the drop-in directory check and the credential lines' reader |
| `scripts/tests/test_alert_unit.py` | repo | changed: A5, and `run_alert` plants a credential's content; `unit_file` reads through `_units.py`'s reader |
| `scripts/tests/_units.py` | repo | changed: one reader for A4 and A5 that refuses the lines R2 lists (`logical_lines`, `assignments`), an exit-status word read only as a decimal of at most 255 or a status name (`exit_status`, `status_words`), every assignment of a key (`Unit.every`), and the literal lists of keys per kind of unit (`ALERT_KEYS`, `PAGING_KEYS`, `off_list`) and the table of admitted values (`PAGING_VALUES`) and the keys of the restart budget (`RESTART_BUDGET`) |
| `scripts/mutation-rows.d/S06600-S06699.json` | repo | added: R5 |
| `docs/schematics/startup-settings-and-secrets.md` | repo | changed: the loader's refusal of an empty credential |
| `docs/schematics/alert-and-slo-path.md` | repo | changed: the alert unit's own refusal |
| `docs/decisions/ADR-067-the-loader-refuses-an-empty-credential-by-its-id.md` | repo | added |
| `docs/decisions/ADR-038-credentials-come-from-the-secret-manager-at-unit-start.md` | repo | changed: a dated note at its end (the ADR is accepted, so it is appended to) |
| `docs/specs/SPEC-066-the-credential-loader-refuses-an-empty-credential.md` | repo | added, from `docs/specs/planned/` |
| `docs/red-first/SPEC-066.md` | repo | added |
| `changelog.d/fix-empty-credential-066.md` | repo | added |

## 5. What this does NOT do

- It builds no second route that tells the owner when the alert unit itself fails. That unit's own
  refusal leaves it failed, in `systemctl --failed` and the journal (R3), and the route that pages
  about it is #285.
- It judges no credential's shape beyond empty, because a value's shape is its caller's to judge.
  A value of spaces, a carriage return, or a user id or token of the wrong form still loads, and
  each caller's own check refuses what it cannot use, as identity's owner gate (SPEC-024, #17) and
  the bot role's check of its token (SPEC-026, #19) do.
- It changes how no existing caller handles the loader's refusal. Identity's owner gate passes it on
  as `IdentityError::Credential` (SPEC-024, #17), and the sync's login records it as
  `missing_credentials`, the closed set's code, which names no credential (SPEC-022 R9, #15). The
  engine probe becomes a caller (R6).
- It changes no unit template: A4 and A5 hold what the templates already declare, SPEC-032's (#25)
  and SPEC-031's alert template (#24).
- It installs nothing on a host. The credential socket, its helper and its map are the private
  rail's (#41).

## 6. Risks

- **A credential that is meant to be empty would refuse start.** None of the four ids can be: each
  is a user id, a token, a username or a password. An optional credential would need its own
  decision (ADR-067, "What would make this wrong").
- **The sync job pages once per episode.** Its first refused login pages (R2); a repeat only logs,
  and the liveness job pages when no sync has succeeded within its window (SPEC-027 R8).
- **The alert unit's own refusal pages no one** until #285 builds its route. The failed instance and
  its error line stay in `systemctl --failed` and the journal.
- **A blank credential passes the loader.** The loader is not a validator: each caller's shape
  check (identity's), or the far end (the sync server's login), refuses what it cannot use.

## 7. Amended in delivery

- **R5 names twelve rows, not five.** Beside the loader's check, its variant, the id in its message,
  and the script's check and exit, seven rows guard what those five do not name: S06604, a
  refusal that reaches past an empty value to one of a single character (A2's killer); S06607
  and S06608, the script's check of each of its two credentials removed in turn (A5's killer);
  S06609 and S06611, the value table admitting a second `Restart=` value and dropping a key of
  the restart budget (A4's value test); S06610, the target check admitting a target beside
  the alert's (A4's target test); and S06612, the restart budget's presence check disabled (A7's
  test).
  Each was proved KILLED with its target restored (`docs/red-first/SPEC-066.md`).
- **A unit that restarts holds its whole restart budget (R2, A7).** The value table bounds the
  value of each key a unit holds and says nothing of a key it lacks. A unit that loads a credential
  and pages, and restarts, needs the start limit beside its restart delay: without it the default
  start limit interval is shorter than the restart delay, so the limit never trips and the unit
  pages on every restart. R2 therefore refuses each key of the budget a restarting unit lacks, by
  name, A7 decides it, S06612 guards it, and the plants were committed red first
  (`docs/red-first/SPEC-066.md`). The three keys and their values are the ones `deck-streak-api`
  and `deck-streak-bot` hold.
- **A2 and A4 are disclosed not red.** Each pins what the base already did, and what the change
  must leave as it was: A2 the missing and unreadable refusals and the loaded values, A4 the
  templates' `OnFailure=` and exit handling. Row S06604 and the census's planted templates give
  each its killing case.
- **The loader's file carries no survivor, and two kills are dev's.** `cargo mutants --file
  crates/kernel/src/credentials.rs` at 5e67962 reported 12 mutants: 9 caught, 1 unviable and 2
  missed, both on lines this delivery does not change: `Secret`'s `Debug` replaced by an empty
  write, and the `NotFound` guard replaced by `true`. dev now carries a killer of each
  (`a_secret_shows_no_value_in_debug` and
  `a_credential_that_exists_and_cannot_be_read_is_unreadable_and_not_missing`). This delivery's
  tests state the same two facts where they touch: SPEC-020's test also asserts `Ok(Secret(..))`,
  and A2 plants a directory at a credential's path, refused as `Unreadable`. They add no kill dev's
  do not, so this delivery claims none of its own for the two; it guards its refusal by rows
  S06601 to S06604 (`docs/red-first/SPEC-066.md`).
- **A fourth reader, and R6.** The engine probe reads the sync's two credentials, so §1 counts it,
  R6 moves its read onto the loader, and §4 lists its file. It takes no criterion: the loader's A1
  to A3 hold the refusal it now takes.
- **R3 names the alert template's exit, its restart and its collection.** The alert template cannot
  page, but a refused start must still leave it failed and listed, so R3 states that it counts no
  refusal a success, restarts none and is never unloaded while failed. A4 holds R3's four exit
  conditions on it, told from `OnFailure=` by the condition each refusal names rather than by its
  text, its restart and its collection, with planted alert-shaped templates as the killing cases,
  and A5 reads the same in the template. The restart is pinned whole, not `RestartMode=direct`
  alone, since a restart at the default mode also leaves the instance activating, not failed,
  between its attempts. `SuccessExitStatus=` and `RestartForceExitStatus=` are pinned whole as well,
  since the alert template names neither: on its `Type=oneshot` the service manager refuses a unit
  that names the second, rather than restarting it. Each addition pins what the template already
  declares, so each is disclosed not red (`docs/red-first/SPEC-066.md`).
- **R2 refuses a condition, and the census refuses what it does not read.** R2's census refuses an
  `ExecCondition=`: one that exits 1 to 254 skips the start, and the unit is not marked failed
  (systemd.service(5)), so its `OnFailure=` never starts. It also refuses every `[Unit]` condition
  and assertion, which can stop the start before it runs (systemd.unit(5)), and R3 holds the alert
  template to the same. The census does not model how systemd reads a unit file or an exit status:
  an earlier draft did, and each review found a spelling it read otherwise than systemd. It reads
  the plain syntax the templates hold, and refuses the rest, as R2 lists: a line it does not read,
  an exit-status word other than a decimal of at most 255 or a status name, and an empty or unknown
  `Restart=`, `RestartMode=` or `CollectMode=`, each read at every assignment. No template under
  `deploy/` holds any of them, so each refusal gives up nothing the templates need. `_units.py`
  holds the one reader, which A4 and A5 use, and §4 lists it. Each addition pins what the templates
  already declare, so A4 stays disclosed not red, with its plants committed red before each
  refusal, and its killing cases (`docs/red-first/SPEC-066.md`).
- **Any key off a unit's list is refused, and only its own drop-in directory is read.** R2 and R3
  hold every unit to a literal list of `(section, key)` pairs for its kind, which closes the
  directives that stop a start without failing the unit (`Requisite=`, `Requires=`, `BindsTo=`)
  along with every key the lists do not name; the lists are the keys those units use, the units
  that load a credential and page on failure (a test holds each list equal to the pairs its units
  hold), and the alert template's carries no `OnFailure=`. A table holds the one restart value
  those units use, and their `OnFailure=` is the alert template alone. A4 refuses a `*.d/` directory under `deploy/` that is not a
  shipped unit's own `<unit name>.d/`, and the credential lines are read by the unit reader. Each
  addition pins what the templates already declare, so it is disclosed not red, with its plants
  committed red before each refusal (`docs/red-first/SPEC-066.md`).

Amendment (2026-09-29): a shipped template's instance drop-in directory is that template's own, and a template loads a credential when it or its instance drop-in carries one (SPEC-062 R14).
