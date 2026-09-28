# SPEC-061: the private rail provisions every credential, setting and guard the public units need, and places no secret value on the host

- **Wave:** W2. **Issue:** #41 (epic #3). **Context(s):** `deploy` (the rail's public contract:
  `deploy/scripts/`, `deploy/rail-contract.json`); the private deploy rail (outside this
  repository).
- **Decided by:** ADR-038 (credentials from the secret manager at each unit start, through a
  root-only socket that serves only the mapped pairs), ADR-032 (neutral, lint-valid templates the
  rail fills), ADR-010 (units per role; its credential storage superseded by ADR-038), ADR-037
  (the Anki login's reuse conditions), ADR-015 and ADR-054 (the guards and the device key only with
  the AI route), and this SPEC's ADR-061 (host values reach the units as drop-ins and the Caddy
  block as a rendered file).
- **Waits for:** owner gate 2 (#161) for the socket, the helper, the host identity's grant and the
  first install; gate 6 (#165) for the values the owner stores. The device key and the guards wait
  for gate 3 (#162) and are installed by SPEC-063.
- **Status:** judged: delivered with its tests and `docs/red-first/SPEC-061.md` (ADR-016). The
  delivery made R5, R7 and R8 exact where the code decided them (§8). The rail's private pieces
  wait for owner gate 2 (#161).

## 1. The problem, measured

- **The mechanism is decided, and none of it exists.** ADR-038 chose a credential socket fed from
  the secret manager, served by a root-only helper that answers only systemd, only for the (unit,
  credential id) pairs of a private map, and it requires seven rehearsal proofs on the host. The
  helper, its socket unit and the map live in the private rail (ADR-038), which does not exist yet.
- **The public side names ids only.** The templates load credentials as
  `LoadCredential=<id>:/run/deck-streak-credentials/socket` (SPEC-032 R3, SPEC-031 R6). The ids the
  code reads are `telegram-bot-token` and `owner-user-id` (the identity crate), `anki-sync-username`
  and `anki-sync-password` (the ingest crate), and, with the AI route only, `agent-device-key`
  (SPEC-043 R2). A template's instances (the job and alert templates) reach the socket under their
  instance names, so the map must match an instance by its template (SPEC-032's risk).
- **Neutral values need a mechanism.** ADR-032 keeps every template valid as committed, with an
  example release root, UTC calendars and Caddy's `{$VAR}` placeholders, and says the rail replaces
  them at deploy; how it replaces them was not decided (ADR-061 decides it). A missed value shows
  only as a unit that cannot find its binary or a timer that fires in UTC (ADR-032's consequences).
- **The owner's login reuse carries four conditions** (ADR-037): no upload path, at most one
  scheduled sync per study day plus the owner's triggers, the values read from the secret manager
  at run time and never on disk, and the grant that lets the host read them decided at gate 2.
- **The grant waits on the host findings.** The grant this rail needs is decided at gate 2,
  together with the host findings, tracked privately (gate 8, #167).
- **The agent's guards and settings are private.** The owner's guards and the concrete settings
  are private sources; the public repository ships only a settings template (SPEC-043 R7).

## 2. Requirements

R1. The rail is a private repository on the maintainer's machine that is never pushed. It holds the
    fetch helper and its socket unit, the credential map, the private configuration, the drop-ins
    it renders, the guards' pinned copies with their manifest, and its own tests. This repository
    holds only the contract the rail reads and checks (R5 to R8).
R2. The rail handles no secret value. It installs code, units and names; a value travels only from
    the secret manager, through the helper, into systemd's credentials directory for the unit that
    starts. The rail's commands never read a secret, and no value is on a disk, an argv or a log line.
R3. The socket and the helper are exactly ADR-038's: the socket unit is root-only in its user,
    group, mode and directory mode, and gives each connection its own helper instance, as ADR-038's
    Decision Outcome sets them; the helper serves a connection only when the peer is uid 0
    (`SO_PEERCRED`), its address is the abstract address systemd.exec(5) describes, and its (unit,
    credential id) pair is in the map.
    It reads that secret's latest version with the host's identity, writes the value, and closes.
    It logs each refusal with the unit, the id and the reason, never a value, and it keeps no value
    after its connection closes.
R4. A map row names a unit or a unit template. An instance (`name@<instance>.service`) matches
    the row of its template (`name@.service`), so the job and alert instances need no row each; a
    row is never a pattern over unit names.
R5. `deploy/scripts/credential-pairs.py --root .` prints, as JSON, every (unit, credential id) pair
    the committed templates declare, a template unit named as a template, and how many lines it
    examined; with `--optional NAME` it adds the pairs of the drop-ins under `deploy/optional/NAME/`
    (the AI route's, SPEC-063). It refuses a `LoadCredentialEncrypted=` line and a
    `LoadCredential=` whose source is not the credential socket. The rail refuses to install when
    its map's pairs differ from this list, with the optional sets the host has enabled, in either
    direction.
R6. Non-secret settings are one environment file, rendered by the rail from private configuration
    and installed root-owned with mode `0600`, which each unit reads as its one required
    `EnvironmentFile=` (SPEC-032 R3). It holds the settings the code names (the sync endpoint, the
    vault paths, the time zone and rollover, the Mini App's origin, the path of each private
    configuration file) and never a secret. The private configuration files the code reads by path,
    the readings taxonomy first (SPEC-045 R1), are installed root-owned, readable by the service's
    group only.
R7. Host values reach the units as drop-ins (ADR-061). Every public template is installed byte for
    byte, and the rail writes one drop-in per unit, `<unit>.d/10-rail.conf`, that overrides each
    neutral value the template carries. `deploy/rail-contract.json` lists those values by unit and
    key, and `deploy/scripts/effective-check.py` reads a unit's effective configuration (the output
    of `systemctl cat`) and refuses one that still carries a neutral value, a
    `LoadCredentialEncrypted=` line, a secret-named `Environment=` assignment, or a drop-in other than
    the rail's own.
R8. The guards and the agent's concrete settings are installed at a pinned version from their
    private sources, each file root-owned and writable by root alone, with a manifest of each file's
    path, SHA-256 and mode. `deploy/scripts/guards-check.py MANIFEST` refuses a missing file, a
    changed digest, a file writable by anyone but root, and a manifest that names no file; it reports
    how many files it examined. The rail installs the guards only when the AI route is enabled
    (SPEC-063, gate 3), and the agent's launch runs the check first (SPEC-063 R6).
R9. The grant that lets the helper read the secret manager is decided at gate 2 and recorded with
    the read-back of the bindings: the host identity, or an identity of DeckStreak's own that the
    helper impersonates, is granted access to each named secret only, never to the project's
    secrets as a whole on DeckStreak's account. The host findings are tracked privately (gate 8,
    #167).
R10. ADR-038's seven proofs are run on the host and recorded before any DeckStreak unit relies on
    the socket: a unit's credential is present under `$CREDENTIALS_DIRECTORY`; `find / -xdev` finds
    no copy; a non-root `connect()` is refused; a root connection without systemd's abstract address
    gets nothing; a unit asking for an id outside its row gets nothing; `findmnt` shows the
    credentials directory's file system type; and the host's own systemd.exec(5) confirms the peer
    address format before anything relies on it. A file system type ADR-038 does not pass goes to
    the host findings, tracked privately (gate 8, #167), and is not a pass.
R11. The login reuse conditions are each shown before the first scheduled sync: (a) SPEC-022's
    recording-server census is green on the deployed tag; (b) the job table holds `sync` to one
    claimed slot per study day (ADR-037); (c) R2 and the second proof of R10; (d) R9's recorded grant.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the pair lister prints each (unit, credential id) pair of a synthetic template tree, a template unit named as a template, and the count of lines it examined | `test_rail_contract.py` |
| A2 | the pair lister refuses a `LoadCredentialEncrypted=` line and a credential whose source is not the socket | `test_rail_contract.py` |
| A3 | the rail contract names every neutral value the committed templates carry, and a planted neutral value it does not name is refused | `test_rail_contract.py` |
| A4 | the effective check refuses a unit whose effective configuration still carries a neutral value, and passes the same unit once the rail's drop-in overrides it | `test_rail_contract.py` |
| A5 | the effective check refuses an effective `LoadCredentialEncrypted=` line, a secret-named `Environment=` assignment, and a drop-in that is not the rail's | `test_rail_contract.py` |
| A6 | the guard check refuses a missing, changed or group-writable guard file and an empty manifest, and passes a matching manifest (examined count reported) | `test_rail_contract.py` |
| A7 | no rail-contract file names a private value, and a planted one is refused by the public scrub | `test_rail_contract.py`; `scripts/public-scrub.py` |
| A8 | the pair lister reads each line as systemd does, so a credential line systemd reads on its own is never hidden in the line before it, and a byte-order mark is refused | `test_rail_contract.py` |
| A9 | the effective check reads each line as systemd does: no line systemd reads is hidden in the one before it, no reset systemd does not read counts, and a file's header glued to the file before it is refused | `test_rail_contract.py` |
| A10 | the effective check reads an `Environment=` variable's name as systemd does, and refuses a name written with an escape or a specifier | `test_rail_contract.py` |
| A11 | both checks refuse a section header systemd reads as another section | `test_rail_contract.py` |
| A12 | the effective check reads a neutral value as systemd resolves it: a path spelled with `//`, `/./` or quotes is still the neutral value, one written with an escape, a `..` segment or a specifier is refused, a calendar in a zone that fires at the neutral zone's instants is neutral, and a calendar that names no zone is refused | `test_rail_contract.py` |
| A13 | the effective check refuses the other routes a value can take into a unit: a secret-named `PassEnvironment=`, `StandardInputText=`, `StandardInputData=`, `StandardInput=file:` and a second `EnvironmentFile=` in force | `test_rail_contract.py` |
| A14 | the sync login is read by the `sync` job alone, the one instance the rail's map answers it to (§8, R4) | `test_rail_contract.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_pair_lister_prints_each_unit_and_credential_id
A2: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_pair_lister_refuses_encrypted_and_off_socket_credentials
A3: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_rail_contract_names_every_neutral_value
A4: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_effective_check_refuses_a_neutral_value_left_in_force
A5: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_effective_check_refuses_encrypted_secret_env_and_foreign_drop_ins
A6: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_guard_check_refuses_missing_changed_and_writable_files
A7: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_no_rail_contract_file_names_a_private_value
A8: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_pair_lister_reads_each_line_as_systemd_does
A9: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_effective_check_reads_each_line_as_systemd_does
A10: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_effective_check_reads_a_variables_name_as_systemd_does
A11: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_checks_refuse_a_section_systemd_reads_as_another
A12: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_effective_check_reads_a_neutral_value_as_systemd_resolves_it
A13: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_effective_check_refuses_every_other_secret_route
A14: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_only_the_sync_job_reads_the_sync_login
```

A4, A5 and A8 to A13 feed the checks synthetic `systemctl cat` outputs and template trees written at
run time; A6 builds its guard files and manifest in a `TemporaryDirectory`; A14 reads the job
table and the job role's source. None of them reads a host. A12's calendars are read against the
tz database of the machine that runs them.

## 4. The owner's gates and the evidence they record

The exact commands, the secret names and the host's names are in the maintainer's private gate
packet; this SPEC names each step only.

| step | gate | what is approved | evidence recorded (privately) | rollback |
|---|---|---|---|---|
| E1 | 6 (#165) | the owner stores the new values from standard input | each secret's existence, read by name only | the owner disables the version |
| E2 | 2 (#161) | the grant of R9 | the command, and the read-back of the bindings | remove the binding |
| E3 | 2 (#161) | the socket unit, the helper and the map | R10's seven proofs, each with its output | stop and disable the socket, remove the helper and the map |
| E4 | 2 (#161) | the environment file and the drop-ins | `credential-pairs.py` against the map; `effective-check.py` over every installed unit | remove the rail's drop-ins and the file |
| E5 | 2 (#161) | the first scheduled sync with the reused login | R11's four conditions, each with its evidence | stop the sync timer |

The rail's own tests (the helper's refusals, the map's parse, the rendering) run on the
maintainer's machine before every install; the packet names them.

## 5. File manifest

| file | context | change |
|---|---|---|
| `deploy/scripts/credential-pairs.py` | deploy | added: the (unit, credential id) pairs the templates declare |
| `deploy/scripts/effective-check.py` | deploy | added: the effective-configuration check |
| `deploy/scripts/guards-check.py` | deploy | added: the pinned-guard manifest check |
| `deploy/rail-contract.json` | deploy | added: the neutral values the rail overrides, by unit and key |
| `deploy/README.md` | deploy | changed: what the rail provides and how its contract is checked |
| `scripts/tests/test_rail_contract.py` | repo | added: A1 to A7 |
| `docs/specs/SPEC-061-the-private-deploy-rail.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-061-host-values-reach-units-as-drop-ins-and-caddy-as-a-rendered-file.md` | docs | changed: status accepted, if SPEC-062 has not accepted it first |
| `docs/red-first/SPEC-061.md` | docs | added |
| `changelog.d/` fragment | repo | added |

The helper, its socket unit, the map, the rendered environment file, the drop-ins, the guards'
copies and manifest, and the rail's tests are private: they are named here and never committed.

## 6. What this does NOT do

- It stores no secret value and creates none; the values are the owner's (#165).
- It installs no device key and no guard before the owner enables the AI route (#43, #162).
- It installs no DeckStreak unit and reloads no Caddy (#42).
- It creates no bucket (#166).
- It decides none of the host findings, tracked privately (gate 8, #167).

## 7. Risks

- **The host's systemd names the peer differently.** The seventh proof reads the host's own manual
  first, and the socket is not relied on until the format matches (ADR-038).
- **The credentials directory's file system is one ADR-038 does not pass.** The sixth proof shows
  it before any unit relies on the socket, and it joins the host findings, tracked privately
  (gate 8, #167).
- **The secret manager is unreachable when a unit starts.** The unit does not start, its
  `OnFailure=` alert names the credential id, and its restart policy retries (ADR-038).
- **The map drifts from the templates.** R5's comparison refuses the install in either direction.
- **A neutral value survives an install.** `effective-check.py` refuses the unit before it is
  started (R7).
- **The helper is root code outside CI.** It is kept small, standard-library only, and its tests run
  on the maintainer's machine before every install; the seven proofs run on the host.
- **A grant broader than the named secrets.** R9 records the bindings as read back; anything broader
  joins the host findings, tracked privately (gate 8, #167).

## 8. Amendments at delivery

- **R5: the pair lister's output and its refusals.** It prints JSON: `pairs`, each
  `{"unit", "credential"}` once and sorted; `optional`, the sets it added; and `examined`, its files
  and lines. It reads each template with the drop-ins of the `<unit>.d/` directory beside it, and
  honours the empty assignment that resets a list. An optional set's drop-in `<unit>.conf` applies
  to that unit, and `<name>.conf` to `<name>.service`, the names SPEC-063 and SPEC-065 give theirs.
  Besides `LoadCredentialEncrypted=` and a `LoadCredential=` off the socket, it refuses
  `SetCredential=`, `SetCredentialEncrypted=` and `ImportCredential=`, each of which gives a unit a
  credential the socket did not serve (ADR-038). A refused run prints no list, and a tree with no
  unit, or an optional set that does not exist, exits 2.
- **R7: the contract's shape and the census.** `deploy/rail-contract.json` holds ADR-032's four
  neutral values (the release root, the settings file, the zone, and the rollover hour the calendars
  are written at), the drop-in's name, the socket's path, and one row per unit and key with its
  neutral values. A job's timer is named by its template and an `instance` field, which the checks
  join, so no committed file writes an instance's name whole (SPEC-032 R10). Every timer's calendar
  is listed, the evaluator's and the watch's included: the zone does not move their fires, and every
  calendar in UTC then counts as a neutral value, with no exception. `effective-check.py --census`
  refuses a neutral value the contract does not name and a row no template carries (A3). The
  effective check also refuses a value that still carries a neutral value where the contract names
  no row for it, and a credential not from the socket in any of the unit's files; the rail's own
  drop-in is the one beside the unit's file. A refused `Environment=` assignment is named by its
  variable, never its value, and its secret-name rule is SPEC-032's pattern widened to any `KEY`
  segment and to `PASS`.
- **R8: the manifest's shape and the directory.** The manifest is JSON: `schema`
  `deckstreak.guards-manifest.v1`, and `files`, each with its `path`, `sha256` and `mode`. Beyond
  the four refusals R8 names, the check refuses a file whose mode is not the manifest's, a symbolic
  link, a file in a directory anyone but root could write, and a manifest anyone but root could
  rewrite, because each would let someone other than root replace what the manifest vouches for. Its
  command line judges ownership against root's uid; the tests judge it in process with their own.
