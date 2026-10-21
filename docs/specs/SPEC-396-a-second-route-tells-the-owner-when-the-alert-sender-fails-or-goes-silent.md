# SPEC-396: a second route tells the owner when the alert sender fails or goes silent

- **Wave:** W3. **Issue:** #285. **Context(s):** `deploy` (the second route's two units and its
  script, the census of the unit templates, the host budget and the rail contract), `formal` (the
  second route's model).
- **Decided by:** ADR-410 (this SPEC's own: the population, the second route, the check that holds
  it, FORMAL by surface, drift and the host), ADR-038 (each credential is read from the credential
  socket at every start), ADR-067 (an empty credential is refused by its id), ADR-032 (the host
  budget; its table gains the second route's row), ADR-010 and SPEC-031 (the one alert path),
  SPEC-066 R3 (the alert template names no `OnFailure=`).
- **Status:** delivered by #768, which moved it from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-396.md` (ADR-016).

## 1. The problem, measured

Each fact below was read at DeckStreak `dev` `164ac206` with `git show <sha>:<path>` and `git grep -n`.

- **One alert path.** Every service but the alert template names
  `OnFailure=deck-streak-alert@%n.service`, and every timer starts one of them:
  `scripts/tests/test_alert_unit.py` `test_every_unit_names_the_alert_template_on_failure`
  (lines 532-568). The template runs `deploy/scripts/alert-telegram.sh %i` (template line 17) as
  `deck-streak` (lines 18-19), `Type=oneshot` (line 16), and names no `OnFailure=` (lines 9-10) and
  no `Restart=` line (`grep -n -E '^(OnFailure|Restart)='` prints nothing).
- **Its credentials.** Two `LoadCredential=` lines from the credential socket, the owner's id and
  the bot token (template lines 25-26; ADR-038 lines 54-61). The script reads them by `cat` from
  `$CREDENTIALS_DIRECTORY` (script lines 20-24).
- **How it fails, and what is left.** An empty credential: one line at priority 3 naming it, exit 1,
  before any request (script lines 30-39; SPEC-066 R3, lines 121-148). A request refused or never
  answered after three retries: a non-zero exit (script line 18 `set -eu`; lines 67-71). A run past
  `TimeoutStartSec=3min` (template line 28). An OOM kill at `MemoryMax=` (template line 32). A
  credential the socket does not deliver: the start fails (ADR-038 lines 65-66). Each leaves the
  instance failed and listed by the service manager.
- **How it goes silent.** The template absent or masked: `OnFailure=` then starts nothing and no
  instance fails. The host, its service manager or its network down: nothing runs at all.
- **What records it today.** SPEC-066's A5 test fails the alert unit on an empty credential and
  sees no request (`test_an_empty_credential_fails_the_alert_unit_before_any_request`, lines
  571-728). The failure is recorded where the service manager and the journal show it; the route
  that tells the owner is #285's, which SPEC-066 leaves to it ("What this does NOT do", lines
  276-278, and its risks, lines 299-300; `docs/schematics/alert-and-slo-path.md`, lines 40-41).
- **The census a new unit meets.** `scripts/tests/test_deploy_templates.py` holds a unit in ten
  tables: `SCRIPTS` (lines 123-137), `OBSERVABILITY_SERVICE` (140-182), `OBSERVABILITY_TIMERS`
  (207-213), `CREDENTIAL_SOURCES` (216-227, a shell source read as `readonly <CONSTANT>=<id>`),
  `ROLE_CREDENTIALS` (238-258), `PER_SERVICE` (339-377), `WAIVED` (410-423), `IDENTITY_ARMS`
  (455-470), the oneshots of `test_the_daemons_and_the_largest_job_fit_the_stack_share` (line
  1278), and the host budget, which `deploy/host-budget.json` and ADR-032's table both carry
  (`test_every_unit_ceiling_matches_the_host_budget_and_high_is_below_max`, line 1263). The
  credential census compares the units that load a credential with the units the table gives
  constants (`test_every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal`, line 2330).
- **Who reads the new unit without a change.** The memory watch reads every `deck-streak-*.service`
  cgroup (`deploy/scripts/memory-watch.sh` line 45). The deploy installs every `*.service` and
  `*.timer` under `deploy/systemd/` (`deploy/deploy.sh` lines 131-133) and refuses a release whose
  effective units fail the rail's check, rolling back (lines 202-214).
- **What moves.** The threat-model schematic cites `test_deploy_templates.py` seven times (its lines
  90-95: lines 1418, 1516, 1544, 2049 twice, 2189 and 2232). Rows added to the census tables move
  every one.

## 2. Requirements

R1. **The unit.** `deploy/systemd/deck-streak-second-route.service` is `Type=oneshot`, runs
`<release root>/deploy/scripts/second-route.sh` as `User=` and `Group=deck-streak`, and names
`OnFailure=deck-streak-alert@%n.service`, `StateDirectory=deck-streak-second-route`, `Nice=10`,
`IOSchedulingClass=idle`, `TimeoutStartSec=3min`, `SyslogIdentifier=deck-streak-second-route`, the
alert template's own `MemoryHigh=` and `MemoryMax=`, every hardening name the census requires,
`RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6`, `IPAddressDeny=link-local`, and `Wants=` and
`After=network-online.target`. It names no `SupplementaryGroups=`, no `EnvironmentFile=` and no
`Restart=`, and only keys `scripts/tests/_units.py` lists for a paging unit.

R2. **The timer.** `deploy/systemd/deck-streak-second-route.timer` starts it every ten minutes:
`OnCalendar=*-*-* *:00/10:00 UTC`, `RandomizedDelaySec=30s`, `AccuracySec=1us`,
`WantedBy=timers.target`, and `X-DurableServices-Waive=calendar-not-persistent` with its why (a run
reads the service manager's current state, which a missed run cannot lose).

R3. **Its own credentials.** The unit loads exactly two credentials from the credential socket,
`second-route-check-in` and `second-route-report`, each an https address the private rail maps. The
script declares each id as a `readonly` shell constant. No other unit loads either id, and the unit
loads none of the alert sender's.

R4. **No shared process.** The script never executes the alert script. It runs `systemctl` with the
verbs `list-units`, `list-unit-files` and `show` only. The unit names no ordering or requirement on
the alert template beyond its own `OnFailure=`, and the alert template names nothing of the second
route.

R5. **The read.** Under the C locale, a run reads:
- the failed alert instances: `systemctl list-units --state=failed --plain --no-legend --no-pager
  --full 'deck-streak-alert@*.service'`. Each line's first field must be an alert instance name
  built only from the characters a unit name holds; any other line, or a non-zero exit, reads as
  unreadable;
- each failed instance's invocation id: `systemctl show --property=InvocationID --value <name>`,
  32 lowercase hex digits, else unreadable;
- the template: `systemctl list-unit-files --no-legend --no-pager --full deck-streak-alert@.service`.
  No line reads as the template absent; one line whose state is `static` reads as whole; one line in
  any other state reads as the template in that state; more lines, or a non-zero exit with a line,
  read as unreadable.

The read yields keys: `failed <instance> <invocation id>`, `template <state>` and `unreadable`.

R6. **The report.** The keys not in `$STATE_DIRECTORY/reported` are new. When any is new, the run
sends one request to the report address whose plain-text body opens with "DeckStreak: the alert
sender cannot page the owner." and names each new key on its own line, at most 3500 bytes in whole
lines; the keys that do not fit are named by one count line. Only after that request is delivered
does the run write the keys of this read to `$STATE_DIRECTORY/reported`, so a key whose instance is
gone is dropped, and a later failure under the same name is a new key.

R7. **The check-in.** The run sends one request with no body to the check-in address when its read
was not unreadable and every new key was delivered in this run, or none was new. An unreadable read
withholds the check-in on every run while it lasts, and is reported once.

R8. **The requests.** Each request is made by curl with its address and body on stdin
(`--config -`), never on argv, with `--fail --silent --show-error --max-time 10 --retry 3
--retry-delay 2`, as the alert script makes its own (script lines 67-71).

R9. **Its own failure is an episode.** An empty credential, an address that does not start with
`https://`, or a request not delivered is the second route's own failure. The first run of the
episode prints one line at priority 3 naming the credential id or the request's credential id, and
exits 1, so `OnFailure=` pages through the alert sender. A later run of the same episode prints its
line at priority 4 and exits 0. A run that delivers every request it makes ends the episode, which
`$STATE_DIRECTORY` remembers as the memory watch remembers its own. The credentials are judged
before any `systemctl` call or request. No line names a credential's value.

R10. **The census.** `scripts/tests/test_deploy_templates.py` names the unit in `SCRIPTS`,
`OBSERVABILITY_SERVICE`, `ROLE_CREDENTIALS`, `PER_SERVICE`, `IDENTITY_ARMS` (`link-local`) and the
stack-share oneshots; the timer in `OBSERVABILITY_TIMERS` and `WAIVED`; and the two constants in
`CREDENTIAL_SOURCES`. `deploy/host-budget.json` and ADR-032's table give it the alert template's
ceilings. `deploy/rail-contract.json` names its `ExecStart=` and `OnCalendar=` neutral values.
`deploy/README.md` names its files, the rail's fills for it and its two credentials.

R11. **The receiver.** The receiver, off the host, tells the owner on each report, and when no
check-in arrives within a grace of at least two periods. It is private configuration (§5), held by
the host's hand-run verify (§8), not by a repository test.

R12. **The model.** `formal/tla/SecondRoute` holds the second route's read, report and check-in
against a service manager that fails and replaces alert instances between them (§7).

R13. **The schematic.** `docs/schematics/alert-and-slo-path.md` gains a section drawing both routes,
the failures each carries to the owner and the credential role each holds.

## 3. Acceptance criteria of SPEC-396

| id | criterion | decided by |
|---|---|---|
| A1 | the alert sender, failed through the existing harness (an empty credential, exit 1, no request), is then reported by the second route: against a service manager listing that failed instance, the run sends exactly one report, to the report address, naming the instance, and then one check-in | `test_alert_unit.py` `test_the_owner_is_told_through_the_second_route_when_the_alert_sender_fails` |
| A2 | with no failed instance and the template `static`, the run sends exactly one request, a check-in to the check-in address with no body, and records no key | `test_alert_unit.py` `test_a_whole_alert_path_sends_one_check_in_and_no_report` |
| A3 | no shared credential: the second route loads exactly its two ids, the alert template exactly its two, the sets are disjoint and non-empty, no other unit loads a second-route id, a run reads exactly its own two ids though the alert's are planted beside them, both units carry `ProtectSystem=strict` and `PrivateTmp=yes`, and a planted second-route unit loading an alert id is refused by name | `test_alert_unit.py` `test_the_second_route_shares_no_credential_with_the_alert_sender` |
| A4 | no shared process: the unit runs `second-route.sh` and names no dependency on the alert template beyond its own `OnFailure=`, the alert template names nothing of it, a run executes no alert script and only the three `systemctl` verbs, and a planted unit running the alert script, or wanting the template, is refused by name | `test_alert_unit.py` `test_the_second_route_shares_no_process_with_the_alert_sender` |
| A5 | an absent, masked or non-`static` template is reported once by its state with the check-in sent after it; a service manager that exits non-zero, prints a line out of shape or an invocation id out of shape is reported once as unreadable, and the check-in is withheld on that run and the next | `test_alert_unit.py` `test_a_broken_template_or_an_unreadable_manager_is_reported` |
| A6 | across runs sharing one state directory, a failed invocation is reported once; a new invocation under the same name is reported again; a path whole again leaves the reported keys empty; a read too long for one report fits 3500 bytes in whole lines and counts the rest | `test_alert_unit.py` `test_each_failed_invocation_is_reported_once` |
| A7 | a report not delivered: the first run exits 1 with one priority-3 line naming `second-route-report`, sends no check-in and records no key; the next exits 0 at priority 4 and tries again; a delivered run sends the report, records the keys, checks in and ends the episode, so the next undelivered run exits 1 again; no output line holds either address | `test_alert_unit.py` `test_an_undelivered_request_pages_once_per_episode_and_names_no_value` |
| A8 | an empty credential, or an address that does not start with `https://`, fails the run by its id at priority 3 before any `systemctl` call or request, and names no value | `test_alert_unit.py` `test_an_empty_or_plain_address_fails_the_second_route_by_its_id` |
| A9 | the census holds the second route: its unit is shipped as a timer-started oneshot, and every table R10 names carries it | `test_deploy_templates.py` `test_the_census_holds_the_second_route_as_a_timer_started_oneshot` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k the_owner_is_told_through_the_second_route_when_the_alert_sender_fails
A2: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k a_whole_alert_path_sends_one_check_in_and_no_report
A3: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k the_second_route_shares_no_credential_with_the_alert_sender
A4: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k the_second_route_shares_no_process_with_the_alert_sender
A5: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k a_broken_template_or_an_unreadable_manager_is_reported
A6: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k each_failed_invocation_is_reported_once
A7: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k an_undelivered_request_pages_once_per_episode_and_names_no_value
A8: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k an_empty_or_plain_address_fails_the_second_route_by_its_id
A9: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_census_holds_the_second_route_as_a_timer_started_oneshot
```

A1 to A8 run the scripts with stubs first on their `PATH`, as SPEC-066's A5 test does: a
`systemctl` stub answering each verb from a fixture, and the `curl` stub recording each request's
configuration and answering, or failing, as the fixture says. The credentials are synthetic https
addresses under the reserved `.invalid` name, built at run time. Each of A1 to A8 is red at the
first commit against a stub `second-route.sh` that declares its two ids and makes no request; A9 is
red there because the unit is not shipped. These modules execute deploy scripts, so their red and
green are read in CI.

## 4. File manifest

| file | context | change |
|---|---|---|
| `deploy/systemd/deck-streak-second-route.service` | `deploy` | new: R1, R3, R4 |
| `deploy/systemd/deck-streak-second-route.timer` | `deploy` | new: R2 |
| `deploy/scripts/second-route.sh` | `deploy` | new: R4 to R9 |
| `deploy/host-budget.json` | `deploy` | changed: R10, the unit's ceilings |
| `deploy/rail-contract.json` | `deploy` | changed: R10, its `ExecStart=` and `OnCalendar=` neutral values |
| `deploy/README.md` | `deploy` | changed: R10, its files, fills and credentials |
| `scripts/tests/test_alert_unit.py` | `deploy` | changed: A1 to A8, and the stubs' arms for `systemctl` and a failing `curl` |
| `scripts/tests/test_deploy_templates.py` | `deploy` | changed: R10's table rows, and A9 |
| `scripts/tests/_units.py` | `deploy` | unchanged: the unit uses only keys it lists; a key off its list is a manifest amendment |
| `scripts/mutation-rows.d/S39600-S39699.json` | `deploy` | new: the rows |
| `formal/tla/SecondRoute/SecondRoute.tla` | `formal` | new: R12 |
| `formal/tla/SecondRoute/MCSecondRoute.cfg` | `formal` | new: R12, the two invariants |
| `formal/tla/SecondRoute/MCLiveness.cfg` | `formal` | new: R12, the temporal property |
| `formal/tla/SecondRoute/witness/a-key-recorded-before-its-report-is-delivered.cfg` | `formal` | new: R12 |
| `formal/tla/SecondRoute/witness/a-failure-keyed-by-its-unit-name-alone.cfg` | `formal` | new: R12 |
| `formal/tla/SecondRoute/witness/a-check-in-after-an-undelivered-report.cfg` | `formal` | new: R12 |
| `formal/tla/SecondRoute/witness/an-unreadable-read-taken-as-nothing-failed.cfg` | `formal` | new: R12 |
| `config/formal.json` | `formal` | changed: R12, the per-run cap `budgets.tla_seconds`, raised for every entry, and the entry budget `tla/SecondRoute` (ADR-410 D4) |
| `scripts/tests/test_formal_config.py` | `formal` | changed: R12, its `EXPECTED` table pins both budgets (ADR-410 D4) |
| `docs/decisions/ADR-032-deploy-templates-and-the-host-budget.md` | `deploy` | changed: an amendment whose budget row the census parses |
| `docs/schematics/alert-and-slo-path.md` | `deploy` | changed: R13 |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | `deploy` | changed: the seven cited lines re-derived, no row added or removed |
| `docs/specs/SPEC-396-a-second-route-tells-the-owner-when-the-alert-sender-fails-or-goes-silent.md` | `deploy` | moved from `docs/specs/planned/` at delivery |
| `docs/decisions/ADR-410-a-second-route-started-by-its-own-timer-reports-a-failed-or-absent-alert-sender-and-checks-in.md` | `deploy` | `status: accepted` at delivery |
| `docs/red-first/SPEC-396.md` | `deploy` | new: the red and green of A1 to A9 |
| `changelog.d/second-alert-route-396.md` | `deploy` | new: the fragment |

## 5. What this does NOT cover

- The receiver: which service it is, its addresses, how it reaches the owner and its own health are
  private configuration the private rail holds (#41). The repository names only the two credential
  roles.
- Placing the two credentials, adding their pairs and the unit's drop-in to the private rail's map,
  and enabling the timer are the host's acts through the private rail and its grant (#41, #161).
  This SPEC ships templates and tests only.
- The alert sender itself is unchanged: its message, its retries and its refusal of an empty
  credential stay SPEC-066's (#284), and no `OnFailure=` is added to its template.
- A monitored unit's failure is not repeated through the second route: it reports the alert
  sender's failure and silence only (#285).
- Address allow lists for the units that reach the network belong to the loopback-only deploy work
  (#679); the second route takes the alert template's deny list until that lands.

## 6. Risks

- The `deck-streak` user cannot read the service manager's unit list under the hardening set. Every
  read is then unreadable: reported once, check-ins withheld, so the receiver's grace tells the
  owner. The hand-run verify (§8) detects it.
- The service manager keeps no invocation id for a failed instance. The same fail-closed reading,
  detected the same way.
- The census rows move the threat model's seven citations. The threat-model reader refuses a moved
  line in CI; the build re-derives each one in its own round.
- A receiver outage. A report then goes undelivered: one page through the alert sender, and
  check-ins withheld. A receiver down is a silence only the receiver's own health check sees (§5).
- In-flight work on the same paths (the threat-model schematic, the deploy units). The build
  re-measures each shared path at its cut.

## 7. FORMAL, decided by surface

REQUIRED, TLA+ (ADR-410 D4). The surface: the second route reads the service manager's failed set,
then reports and checks in, while the service manager fails and replaces alert instances in between;
and the reported keys persist across those interleaved runs. The entry `formal/tla/SecondRoute`
covers `deploy/scripts/second-route.sh`'s `read_alert_path`, `tell_owner`, `check_in` and `main`,
and cites #285. Its properties, each entering in report mode:

- `ACheckInCoversOnlyToldFailures`: no check-in is sent while a failure key of its run's read is
  neither delivered nor told by a later page under the same name.
- `NoCheckInAfterAnUnreadableRead`: no check-in follows an unreadable read.
- `EveryFailureIsToldOrItsSilenceIsHeard` (temporal, under fairness on the timer's runs and the
  receiver): every failure key is eventually told, or the receiver hears the check-ins stop.

Each witness switches one defect on and must be caught: keys recorded before the report is
delivered, keys by name alone, a check-in after an undelivered report, and an unreadable read taken
as an empty one. The fixed design is clean at its state floor.

Every configuration, the temporal property's and each witness's, has two alert instance names and
two invocations under each name. The model keeps what its properties read and drops what none
reads, each drop named in ADR-410 D4:

- The episode marker is not a variable of the model, and that is admitted: no property, invariant or
  witness reads it, and A7 holds its effect, the run's exit status and its one page per episode.
- A delivered report adds only its failure keys to the receiver's knowledge, and a report request
  empties the keys it carried, delivered or not, because nothing reads them until the next read.

The temporal property's run, with a margin of one half, needs more than the formal checker's per-run
cap allowed, so `config/formal.json` raises that cap for every entry and gives `tla/SecondRoute` an
entry budget of its own, each sized from the measured runs (ADR-410 D4);
`scripts/tests/test_formal_config.py` pins both.

## 8. The host's acts and the hand-run verify (W3)

No repository test proves the receiver. Who: the host's operator, after the release that carries the
units is deployed. When: at that first deploy, before the second route is relied on. In order: the
two credentials are placed and their pairs and drop-in added to the private rail before that release
is deployed; the timer is enabled; the receiver is set to tell the owner on a report and on a
check-in missing past its grace. The verify: one run with the path whole, and the check-in seen at
the receiver; one alert instance failed on purpose, and the report seen by the owner; that instance
reset, and the check-ins seen again. The record: the operator's deploy record names each observation.
