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
      names `OnFailure=deck-streak-alert@%n.service`, starts its `ExecStart=` without the `-`
      prefix, holds no `SuccessExitStatus=` that names 1, and sets no `RestartMode=direct`. The
      prefix and the exit status would each count a refused start as a success, and the restart
      mode skips `OnFailure=` (systemd.service(5)).
R3. The alert template unit cannot page through itself: it names no `OnFailure=` (SPEC-031). Its own
    refusal is therefore surfaced as its failed state. `deploy/scripts/alert-telegram.sh` refuses
    each of the two credentials it loads whose value is empty, nothing being left once the command
    substitution that reads it has removed its trailing newlines, before it reads the journal or
    makes a request. It writes one line at error priority to standard error,
    `<3>the credential <id> is empty in the credentials directory: no page is sent`, which the
    journal keeps under the unit's identifier, and exits 1. The alert template counts no refusal a
    success: no `-` prefix and no `SuccessExitStatus=` naming 1 or FAILURE (systemd.service(5)), so
    the instance is `failed` and listed by `systemctl --failed`. No page reports it: a second route
    that does not depend on the alert sender is #285.
R4. ADR-067 records the decision and what it was chosen against. ADR-038 takes one dated note at its
    end, a pure append, naming the loader, not the service manager, as what refuses an empty
    credential.
R5. Where cargo-mutants makes no mutant, hand-proved rows in
    `scripts/mutation-rows.d/S06600-S06699.json` guard the refusal (SPEC-039 R8): the loader's
    check, the variant it returns, the id in its `Display`, the bound that admits a value of one
    character, and the script's check, its exit and its check of each of its two credentials.
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
| A4 | every unit template that loads a credential, the alert template excepted, fails and pages on a refused start (R2's four conditions), and the alert template counts no refusal a success (R3); the census prints its examined count, refuses zero, and refuses a planted template for each condition, and an alert-shaped one, which names no `OnFailure=`, for its exit status | `test_deploy_templates.py` `every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal` |
| A5 | the alert unit's route (R3): each credential the template loads, empty in each form, makes the script exit 1 with the one line naming it, before any journal read or request; the template names no `OnFailure=`, and counts no refusal a success: no `-` prefix and no `SuccessExitStatus=` naming 1 or FAILURE | `test_alert_unit.py` `an_empty_credential_fails_the_alert_unit_before_any_request` |
| A6 | the sync's login reads through the loader (R2): each of its two credentials, empty in each form, records the run as `missing_credentials` with no attempt, and the engine is never asked to sync | `retry.rs` `an_empty_sync_credential_is_recorded_missing_and_never_reaches_the_engine` |

```acceptance
A1: cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_credential_refuses_start_by_its_id
A2: cargo test -p deck-streak-kernel --test credentials -- --exact a_missing_credential_keeps_its_refusal_and_a_value_loads_unchanged
A3: cargo test -p deck-streak-kernel --test credentials -- --exact an_empty_refusal_names_the_id_and_never_a_value
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal
A5: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k an_empty_credential_fails_the_alert_unit_before_any_request
A6: cargo test -p deck-streak-ingest --test retry -- --exact an_empty_sync_credential_is_recorded_missing_and_never_reaches_the_engine
```

A1 to A3 write synthetic credentials to a temporary directory and read them through the loader.
A4 reads the templates with `_units.py`, as the rest of the census does, and plants one template for
each of R2's four conditions. It holds the alert template to R3's two exit conditions, the
`ExecStart=` prefix and `SuccessExitStatus=`, and plants a template shaped as the alert template
is, loading a credential and naming no `OnFailure=`, that its `SuccessExitStatus=1` alone refuses.
A5 runs the script as its unit runs it, with the recording stubs of SPEC-031's tests first on its
`PATH`, once for each credential the template loads and each empty form, zero bytes and a lone
newline, the other credential holding its synthetic value, and reads the same two exit conditions
in the template. A6 runs the syncer over SPEC-022's scripted engine and in-memory record, which
count every sync the engine is asked for, with one of the fixture's two credentials rewritten
empty. R6 takes no criterion of its own: the engine probe is an example a person runs by hand,
with no test, and the refusal it takes is the loader's, which A1 to A3 hold.

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
| `scripts/tests/test_deploy_templates.py` | repo | changed: A4 |
| `scripts/tests/test_alert_unit.py` | repo | changed: A5, and `run_alert` plants a credential's content |
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

- **R5 names eight rows, not five.** Beside the loader's check, its variant, the id in its message,
  and the script's check and exit, three rows guard what those five do not name: S06604, a
  refusal that reaches past an empty value to one of a single character (A2's killer), and S06607
  and S06608, the script's check of each of its two credentials removed in turn (A5's killer).
  Each was proved KILLED with its target restored (`docs/red-first/SPEC-066.md`).
- **A2 and A4 are disclosed not red.** Each pins what the base already did, and what the change
  must leave as it was: A2 the missing and unreadable refusals and the loaded values, A4 the
  templates' `OnFailure=` and exit handling. Row S06604 and the census's planted templates give
  each its killing case.
- **Two mutants the loader's file already let survive are killed.** `cargo mutants --file
  crates/kernel/src/credentials.rs` at 5e67962 reported 12 mutants: 9 caught, 1 unviable and 2
  missed, both on lines this delivery does not change. `Secret`'s `Debug` replaced by an empty
  write survived an assertion of absence alone, so SPEC-020's test now also asserts what it shows,
  `Ok(Secret(..))`; and the `NotFound` guard replaced by `true`, which reads every failed read as
  missing, survived because no test planted an unreadable credential, which A2 now does. A file
  this delivery touches carries no survivor (`docs/red-first/SPEC-066.md`).
- **A fourth reader, and R6.** The engine probe reads the sync's two credentials, so §1 counts it,
  R6 moves its read onto the loader, and §4 lists its file. It takes no criterion: the loader's A1
  to A3 hold the refusal it now takes.
- **R3 names the alert template's exit.** The alert template cannot page, but a refused start must
  still leave it failed, so R3 states that it counts no refusal a success. A4 holds R3's two exit
  conditions on it, with a planted alert-shaped template as the killing case, and A5 reads the same
  two in the template. Each addition pins what the template already declares, so each is disclosed
  not red (`docs/red-first/SPEC-066.md`).
