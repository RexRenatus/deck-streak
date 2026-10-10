---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A second route, started by its own timer, reads the alert path from the service manager, reports a failed or absent alert sender and checks in, with credentials and a process of its own

## Context and Problem Statement

DeckStreak pages its owner through one path: every service and every scheduled job names
`OnFailure=deck-streak-alert@%n.service`, and that template runs the release's alert script
(ADR-010, SPEC-031; held by `scripts/tests/test_alert_unit.py`'s
`test_every_unit_names_the_alert_template_on_failure`, lines 532-568). The template names no
`OnFailure=` of its own, so a page that fails does not start a page about the page
(`deploy/systemd/deck-streak-alert@.service` lines 9-10; SPEC-066 R3). When the alert sender itself
fails, the failure is recorded where the service manager and the journal show it: its instance stays
failed (the template has no `Restart=` line), held by SPEC-066's A5 test
(`test_an_empty_credential_fails_the_alert_unit_before_any_request`, lines 571-728). The route that
tells the owner is #285's, which SPEC-066 leaves to it (its "What this does NOT do", lines 276-278, and its
risk at lines 299-300).

#285 asks for a second route, independent of the alert sender, that tells the owner about such a
failure and about an alert sender that has gone silent. Its acceptance: a test fails the alert
sender and shows the owner told through the second route, and the second route shares no credential
and no process with the alert sender. SPEC-396 is the specification; this ADR decides its five
questions.

## Decision Drivers

- #285's acceptance is the floor, and it is read literally: no shared credential id, no shared
  process, and a test that fails the alert sender and sees the owner told.
- SPEC-066 R3 stays held: the alert template names no `OnFailure=`, and its census
  (`alert_template_refusals`, `test_alert_unit.py` lines 320-385) keeps refusing a key off its list.
- Every unit property is held by a census in `scripts/tests`, so the second route is a unit the
  census reads like every other, with no waiver the census does not already admit.
- A failure path this design could not measure is a question for the owner, never a guess.

## Decisions, and the alternatives each was chosen against

### D1. The population

The alert sender is the template `deck-streak-alert@.service` and the script it runs,
`deploy/scripts/alert-telegram.sh` (template line 17). No other code sends an alert. It fails, and
leaves its instance failed and listed by the service manager, in five ways, each measured: an empty
credential, refused by its id before any request (script lines 30-39; SPEC-066 R3); a request the
far end refuses or never answers after three retries (script line 18 `set -eu`, lines 67-71
`curl --fail ... --retry 3`); a run past `TimeoutStartSec=3min` (template line 28); an OOM kill at
its `MemoryMax=` (template line 32); and a credential the socket does not deliver, which fails the
start (template lines 25-26; ADR-038 lines 65-66). It goes silent, with no failed instance at all,
in two more: the template is absent or masked, so `OnFailure=` starts nothing; or the host, its
service manager or its network is down. The second route covers all seven: the five as failed
instances it reads, the absent template as a unit file it reads, and the host's silence as the
check-ins that stop arriving.

**Chosen against:**

- Covering only the empty-credential refusal SPEC-066 names: rejected, because a refused request, a
  timeout, an OOM kill and an undelivered credential leave the same failed instance and lose the
  same page.
- Repeating every monitored unit's failure through the second route: rejected, because that is a
  second alert sender for every unit, while #285 asks for the alert sender's own failure and
  silence.

### D2. The second route

A oneshot unit of its own, `deck-streak-second-route.service`, started every ten minutes by
`deck-streak-second-route.timer`, runs `deploy/scripts/second-route.sh` as the `deck-streak` user.
It loads exactly two credentials from the credential socket, each an https address the private rail
maps (#41): `second-route-check-in` and `second-route-report`. A run reads the alert path from the
service manager with three read-only verbs: the failed alert instances (`list-units`), each one's
invocation id (`show`), and the template's unit-file state (`list-unit-files`). A failed instance is
keyed by its name and its invocation id, so a later failure under the same name is a new key. A key
not yet delivered goes into one report to the report address. Only after that request is delivered
does the run record the keys it read, which drops a key whose instance is gone. Then the run checks
in to the check-in address, unless its read was unreadable or a report it owed was not delivered. A
receiver off the host, private configuration like every other host value, tells the owner on a
report and on a check-in that does not arrive within a grace of at least two periods. The second
route names `OnFailure=deck-streak-alert@%n.service` like every unit, so each route watches the
other, and the receiver's grace covers both being down.

A failed alert instance that a later instance of the same name replaces with a delivered page,
before the second route reads it, is told by that page: it names the same failed unit. The second
route reports what the service manager still holds as failed when it reads.

**Chosen against:**

- An `OnFailure=` hook on the alert template into the second route: rejected, because it reverses
  SPEC-066 R3's held rule that the template names no `OnFailure=`, and it covers neither an absent
  template nor a silent host.
- Reusing the alert sender's process or its two credentials: rejected, because #285 refuses both,
  and a shared bot credential that is revoked or empty silences both routes at once.
- No route: rejected, because #285 asks for one and SPEC-066 leaves the route that tells the owner
  to it, so that work stays open.
- A check-in alone, whose absence the receiver reports: rejected, because no repository test can
  then show the owner told about a failed alert sender while the host runs, so #285's acceptance
  would rest on the receiver alone.
- A system user of its own for the second route: rejected, because it loosens the census assertion
  that every service but the sync family runs as `deck-streak` (`test_deploy_templates.py`
  `test_the_sync_family_runs_as_its_own_user`, line 2232), and adds a host act, while the service
  manager already keeps a unit's credentials invisible to every other unit
  (`ProtectSystem=strict` and `PrivateTmp=yes` on both units, held by the hardening census).
- `DynamicUser=`: rejected, because the census reads that key as off the paging units' list
  (`scripts/tests/_units.py` `PAGING_KEYS`) and its `User=` rule refuses a unit without one.
- Reading the journal for every failure the service manager logs: rejected, because it adds the
  journal group, a cursor kept between runs and a dependency on retention, to close only the window
  in which a later delivered page of the same unit has already told the owner.
- Keying a failed instance by its name alone: rejected, because a second failure under a name
  already reported would then be read as told, and the check-in would cover it.
- Naming the route a heartbeat: rejected, because DeckStreak already uses that word for the
  service watchdog, and one concept keeps one name.

### D3. The check that holds it

Nine criteria, each a test in `scripts/tests`, eight of them in `test_alert_unit.py` beside
SPEC-066's A5 test and reusing its stubs and its run harness, and one census test in
`test_deploy_templates.py`. A1 is #285's floor: it fails the alert sender through the existing
harness (an empty credential, refused before any request), then runs the second route against a
service manager stub that lists that failed instance, and asserts one report to the report address
naming the instance, followed by the check-in. A3 and A4 hold the two "shares no" clauses, each with
a planted unit the census must refuse by name. The tests execute deploy scripts, so they are the
deploy class: their red is read in CI, never on the box. Each criterion is red at the first commit
by an assertion, against a stub `second-route.sh` that declares its two credential ids and does
nothing. The script's rows go in `SCRIPT_MUTATIONS` from S39600, each killed by one of these tests.

**Chosen against:**

- A new test module for the second route: rejected, because its harness would copy
  `test_alert_unit.py`'s stubs, and #285 asks for the second route's test beside SPEC-066's.
- A red read on the box: rejected, because a module that executes a deploy script never runs
  locally, so its red is read in CI by the test's name.
- Asserting only that no request reached the alert sender's address: rejected, because an
  absence-only test passes over a second route that sends nothing at all.

### D4. FORMAL, decided by surface

REQUIRED, TLA+. The second route reads the service manager's failed set and then acts on it, while
the service manager can fail and replace alert instances between the read and the act; and its
reported set persists across runs that those failures interleave with. That is a check followed by
an act on state another actor changes. The entry `formal/tla/SecondRoute` covers the script's
`read_alert_path`, `tell_owner`, `check_in` and `main`, cites #285, and states three properties: a
check-in covers only failures already told (`ACheckInCoversOnlyToldFailures`), no check-in follows
an unreadable read (`NoCheckInAfterAnUnreadableRead`), and every failure is told or its silence is
heard by the receiver (`EveryFailureIsToldOrItsSilenceIsHeard`, temporal). A witness for each switch
must be caught: keys recorded before their report is delivered, keys by name alone, a check-in
after an undelivered report, and an unreadable read taken as an empty one.

The model's scope and its budget follow the architect seat's rulings 927 and 935. Every
configuration has two alert instance names and two invocations under each name, the temporal
property's run and the witness of keys recorded before delivery included. The model keeps what its
properties read and drops what none reads:

- The episode marker is not a variable, which ruling 935 admits. No property, invariant or witness
  reads it: it decides only the run's exit status and its one page per episode through the alert
  sender, which A7 holds.
- One variable, `known`, is the receiver's knowledge of a failure key, told by a delivered report
  (`Send`) or by the delivered page of a later instance under the same name (`Replace`).
- A delivered report adds only its failure keys to `known`. Two properties read `known`, each for
  failure keys alone: `ACheckInCoversOnlyToldFailures` for the failure keys of the run's read, and
  `EveryFailureIsToldOrItsSilenceIsHeard` for every failure key. `NoCheckInAfterAnUnreadableRead`
  does not read it, no action's guard reads it, and no witness's switch reads it: `RecordFirst`
  acts in `Plan`, `NameAlone` in the stored key, `CheckInAfterUndelivered` in `Send`'s undelivered
  arm and `UnreadableAsEmpty` in `Read`. A template or unreadable key in `known` moves no verdict.
- `Send` empties `pend`, the keys the run reports, delivered or not. `Plan` writes `pend` and only
  `Send` reads it; after `Send` the run goes to `Record`, `CheckIn` or `Finish`, none of which reads
  it, and `Finish` empties it too. No property, guard or witness reads `pend` between `Send` and the
  next `Plan`, so two states that differ only there are one.

Each of the four witnesses was re-run on the reduced model at its committed names, and each is
caught by the property it names. Measured with the pinned checker: the temporal property's run at
two names explored 1,807,796 distinct states before the reductions and 580,480 after them, the
floor of both configurations. The entry's whole check through the gate's own path found every
property clean and caught every witness. The per-run cap, `budgets.tla_seconds` in
`config/formal.json`, was 300 s, below one and a half times the temporal run. It rises to 480 s by
one rule: the smallest whole minute at or above one and a half times the worst whole-entry check
on the gate's own path, and no lower than 360 s. The raise is GLOBAL: it is every entry's per-run
cap, not this entry's alone, and it supersedes the 300 s that SPEC-295 records. The entry gains a
budget of its own, `tla/SecondRoute` at 480 s, sized by the same rule, above the 300 s an entry's
runs share by default. `scripts/tests/test_formal_config.py`'s `EXPECTED` pins both values, and
SPEC-396 §4 lists both files.

**Chosen against:**

- NOT APPLICABLE: rejected, because the read-then-act window and the persisted keys are exactly the
  interleaving surface, and keying by name alone fails only when a run reads between two failures.
- A Lean proof of the run as a pure function: rejected, because the defects live in the order of
  the service manager's and the second route's steps, which a pure function of one read does not
  see.
- One alert name in the temporal property's run: rejected, because a second name is what lets one
  instance fail while another is replaced, and ruling 927 reads fewer names as a weakening.
- A state constraint bounding the invocations: rejected, because a narrowing keyword in a committed
  configuration voids the check, and ruling 935 refuses it.
- A symmetry set over the names: rejected, because symmetry reduction can miss a liveness
  violation, and the temporal property is one.
- Keeping the dropped values and sizing the cap from the unreduced run, at least 1020 s: rejected,
  because nothing reads those values, so they triple the run and move no verdict.
- An entry budget alone, the per-run cap unchanged: rejected, because the per-run cap bounds the
  temporal run itself, and one and a half times that run is above 300 s.

### D5. Drift and the host

The build is deploy-class, W3: CI judges the tests, and a hand-run verify on the host judges the
receiver. Placing the two credentials, the private rail's map pairs and drop-in for the new unit,
enabling the timer and configuring the receiver are the host's acts, never the builder's. The rail
pairs come before the release that ships the unit, because the deploy's effective check refuses a
unit whose credential the rail does not map and rolls back (`deploy/deploy.sh` lines 202-214). The
census tables gain rows above the threat model's seven citations into `test_deploy_templates.py`
(the threat-model schematic, lines 90-95), so the build re-derives each cited line in its own
round; no threat-model row is added or removed.

**Chosen against:**

- Applying the units from the build: rejected, because nothing runs on any host from a build, and
  the credentials are the owner's to place.
- Leaving the threat model's citations to a later delivery: rejected, because its reader refuses a
  cited line that no longer holds its quote, so the build's own CI would be red.

## Consequences

- Positive: the alert sender's failure and silence reach the owner by a route with its own process,
  its own credentials and its own far end; the two routes watch each other.
- Positive: every new unit fact is held by the census the other units already pass.
- Negative: two credentials and a receiver to keep; a receiver outage is itself a silence the owner
  learns only from the receiver.
- Negative: one more timer on the host, inside the alert template's own memory ceilings.

## Confirmation

SPEC-396's criteria A1 to A9, its rows from S39600, the `formal/tla/SecondRoute` entry's witnesses
and clean properties, and the host's hand-run verify (SPEC-396 §8).

## What would make this wrong

- A service manager that clears a failed unit's invocation id: every read would be unreadable,
  reported once, with check-ins withheld. The hand-run verify would show it, and the key would need
  another identity, decided in its own ADR.
- A host on which the `deck-streak` user cannot read the service manager's unit list: the same
  fail-closed reading, and the same hand-run verify.
- An owner who wants every monitored failure repeated through the second route: that is a second
  alert sender, decided in its own ADR.

## More Information

ADR-010, ADR-016, ADR-032 (its budget table gains the second route's row), ADR-038, ADR-067,
SPEC-031, SPEC-066, #41, #161, #284, #285.
