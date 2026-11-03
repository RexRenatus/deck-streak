# SPEC-582: The install carries no Telegram surface

- **Status:** proposed: the design is final and one delivery builds it, under ADR-601.
- **Decided by:** ADR-601 (amending ADR-589's removal order); ADR-341 for the transport that later
  replaces the bot; ADR-064 and ADR-347 for the host's share.
- **Schematic:** `docs/schematics/the-install-carries-no-telegram-surface.md`.

## 1. The problem, measured

DeckStreak is the web app and the iPhone and iPad app (ADR-589). The install under `deploy/` still
ships the Telegram surfaces, and three units outside the bot load the bot's two credentials, so a
host cannot install DeckStreak without them:

- `deploy/systemd/deck-streak-bot.service` runs the bot (its credentials at lines 28-29), with two
  rows in `deploy/rail-contract.json` (lines 65-76) and an entry in `deploy/host-budget.json`
  (line 7); `deploy/deploy.sh` restarts it at line 220 and in its switch back at line 181.
- The sync job's path unit and `deploy/tmpfiles.d/deck-streak-sync-request.conf` are the owner's
  `/sync` doorbell, which only the bot rings.
- `deploy/systemd/deck-streak-api.service` loads the bot's pair at lines 22-23, and the `api` role
  refuses start without both (`crates/daemon/src/role_api.rs:175-177`).
- `deploy/systemd/deck-streak-alert@.service` runs `deploy/scripts/alert-telegram.sh`, which pages
  on Telegram with the same pair (unit lines 17 and 25-26); `deploy/slo.json` routes its four alerts
  and its channel to `telegram` (lines 25, 33, 57, 65 and 79).
- The held flush's instance loads the pair through its own drop-in, and its router sends through
  the bot (`crates/daemon/src/role_job.rs:140-170`).
- The edge's policy admits Telegram's web client as a framing origin
  (`deploy/caddy/deck-streak.caddy:19`), and the settings example names the Mini App's URL and the
  doorbell's path (`deploy/deck-streak.env.example:55-61`).
- The second route watches the alert sender and reports to two addresses off the host, which do
  not exist.

At that base the rail's credential pairs are 16, the host budget names 15 units, 14 unit files name
the alert template in `OnFailure=`, and the rail contract holds 33 rows over 21 units.

## 2. Requirements

R1. **The units.** `deploy/systemd/` ships no `deck-streak-bot.service`, no path unit of the sync
    job, no `deck-streak-second-route.service` or `.timer`, and no drop-in directory for the
    `held_flush` instance; `deploy/tmpfiles.d/` ships no request-directory line. Every remaining
    service and timer-started job still names `OnFailure=deck-streak-alert@%n.service`: 12 unit
    files, where there were 14.
R2. **The settings.** `deploy/deck-streak.env.example` names neither `DECKSTREAK_MINI_APP_URL` nor
    `DECKSTREAK_SYNC_REQUEST_PATH`, and still names `DECKSTREAK_INIT_DATA_MAX_AGE_SECONDS`.
R3. **The API's start.** No unit or drop-in loads `owner-user-id` or `telegram-bot-token`. The
    `api` role starts with Telegram off when its credentials directory holds neither, or when no
    credentials directory is set; it refuses start by the missing one's id when one is held
    without the other; with both held it starts with the gate it had.
R4. **The routes with Telegram off.** `POST /api/session` and `POST /api/launch` answer 404
    `{"reason":"telegram_off"}` for a well-formed body, and 401 `init_data_invalid` for a body that
    is not the handshake's JSON, under their state-change guard, handshake bound and body limit.
    `POST /api/link/code` and `POST /api/link/redeem` answer 404 `telegram_off` after the
    state-change guard and the ceremony bound and before `linking_off`. The passkey routes and
    `/api/identities` answer 404 `linking_off`. `GET /api/me` answers 401 `no_session`.
    `telegram_off` is identity's refusal, logged by its reason alone.
R5. **The deploy.** `deploy/deploy.sh` restarts the API, waits for its readiness and restarts no
    bot; its switch back restarts the API alone. A release that ships no bot unit deploys.
R6. **The edge.** The site block's policy is
    `frame-ancestors 'none'; object-src 'none'; base-uri 'self'`.
R7. **The alert sender.** `deck-streak-alert@.service` keeps its name and its sandbox lines and
    runs `deploy/scripts/alert-journal.sh %i`, and loads no credential. The script writes one line
    to standard error with the `<3>` prefix: `DeckStreak: <unit> failed (<result>[, status <n>])`;
    it reads no credential, runs no `curl`, and exits 0 once the line is written. The line quotes
    no line of the failed run, because error text in an alert can carry user data and the unit's
    own lines already reach the shipped journal. No shipped source outside the bot names the Bot
    API's host.
R8. **The SLO declaration.** Every alert of `deploy/slo.json` routes to `journal`;
    `alerting.channel` is `journal`, and `alerting.unit` is still `deck-streak-alert@.service`.
R9. **The held flush.** With neither Telegram credential held, `deckstreakd job held_flush` builds
    the holding router with no transport, delivers nothing, keeps every held item, and exits 0.
R10. **The host budget.** `deploy/host-budget.json` names neither the bot nor the second route:
    13 units. The share and its processor count are unchanged; the long-running units' ceilings
    plus the largest job's fit inside the share, and the four daemons' quotas fit inside its
    processors.
R11. **The rail contract.** `deploy/rail-contract.json` holds no row of the bot or the second
    route, and the alert template's `ExecStart` row names the journal sender: 29 rows over 18
    units. `effective-check.py --census` holds it equal to the templates.
R12. **The second route.** Its unit and timer leave; `deploy/scripts/second-route.sh`, its model
    `formal/tla/SecondRoute` and the tests that run the script stay byte for byte where they are
    not re-pointed by A21.
R13. **The records.** ADR-601; the appended amendments to ADR-589, ADR-347, SPEC-024, SPEC-031,
    SPEC-032, SPEC-059, SPEC-062 and SPEC-396; this SPEC's schematic; the deploy README and the
    owner setup state the install as R1 to R12 leave it; one changelog fragment.

## 3. Acceptance criteria of SPEC-582

| id | criterion | test |
|---|---|---|
| A1 | the `api` role starts with neither credential held, and answers 404 `telegram_off` on the handshake, the launch and both link-code routes | `the_api_role_starts_with_telegram_off_when_neither_identity_credential_is_held` |
| A2 | the `api` role starts with no credentials directory set, with Telegram off | `the_api_role_starts_with_telegram_off_when_no_credentials_directory_is_set` |
| A3 | one credential held without the other still refuses start by the missing one's id | `the_api_role_refuses_start_without_an_identity_credential` |
| A4 | the gate is held with both credentials, off with neither, and refused by the missing id with one | `the_gate_is_held_only_with_both_credentials_and_off_with_neither` |
| A5 | `telegram_off` answers 404 with its reason alone | `telegram_off_answers_404_with_its_reason_alone` |
| A6 | with Telegram off a well-formed handshake or launch answers `telegram_off` and a malformed one keeps `init_data_invalid`; the link-code routes answer `telegram_off` before `linking_off`, the passkey routes `linking_off`, and `/api/me` `no_session` | `a_well_formed_handshake_or_launch_answers_telegram_off_and_a_malformed_one_keeps_init_data_invalid`, `the_link_code_routes_answer_telegram_off_and_the_passkey_routes_linking_off` |
| A8 | the held flush with no Telegram credential keeps what it holds and exits 0 | `the_held_flush_with_no_telegram_credential_keeps_what_it_holds` |
| A9 | no shipped source outside the bot names the Bot API | `no_delivery_goes_around_the_port` |
| A10 | the install ships no bot unit, no doorbell, no request-directory line and no held-flush drop-in, its settings example names neither removed setting, and 12 unit files page through the alert template | `test_the_install_ships_no_telegram_unit_and_no_doorbell` |
| A11 | no unit or drop-in loads the bot's pair, and the instance allowlist names `sync` alone | `test_no_unit_loads_the_telegram_pair` |
| A12 | the edge admits no framing origin and sends the security headers | `test_the_caddy_policy_admits_no_framing_and_sends_the_security_headers` |
| A13 | the alert template runs the journal sender, which names the failed unit and its result at error priority | `test_the_journal_sender_names_the_unit_and_result_at_error_priority` |
| A14 | the journal sender reads no credential and makes no request | `test_the_journal_sender_reads_no_credential_and_makes_no_request` |
| A15 | the journal sender's line names the unit and its result and carries no line of the failed run | `test_the_alert_line_carries_no_line_of_the_failed_run` |
| A16 | every SLO alert and the alert channel route to `journal` through the one template | `test_every_alert_routes_to_the_one_alert_path` |
| A17 | the MCP server's SLO pages and tickets to `journal` with its exact burn rates | `test_the_mcp_slo_pages_and_tickets_on_the_one_alert_path_with_exact_burn_rates` |
| A18 | a release with no bot unit deploys, and the deploy and its switch back restart the API alone | `test_a_release_with_no_bot_unit_deploys_and_restarts_the_api_alone` |
| A19 | the daemons and the largest job fit inside the share, and the daemons' quotas inside its processors | `test_the_daemons_and_the_largest_job_fit_the_stack_share` |
| A20 | the rail contract carries no row of the bot or the second route, and names the journal sender | `test_the_contract_carries_no_telegram_or_second_route_row` |
| A21 | the second route's unit and timer are not shipped, and its script and model are | `test_the_install_ships_no_second_route_unit_and_keeps_its_script` |
| A22 | every shipped service takes its census's way to the host's identity endpoint, the planted ways now on the alert template | `test_no_unit_but_the_exempt_reaches_the_host_identity_endpoint` |

```acceptance
A1: cargo test -p deck-streak-daemon --test lifecycle -- --exact the_api_role_starts_with_telegram_off_when_neither_identity_credential_is_held
A2: cargo test -p deck-streak-daemon --test lifecycle -- --exact the_api_role_starts_with_telegram_off_when_no_credentials_directory_is_set
A3: cargo test -p deck-streak-daemon --test lifecycle -- --exact the_api_role_refuses_start_without_an_identity_credential
A4: cargo test -p deck-streak-identity --test owner -- --exact the_gate_is_held_only_with_both_credentials_and_off_with_neither
A5: cargo test -p deck-streak-identity --test owner -- --exact telegram_off_answers_404_with_its_reason_alone
A6: cargo test -p deck-streak-api --test telegram_off -- --exact a_well_formed_handshake_or_launch_answers_telegram_off_and_a_malformed_one_keeps_init_data_invalid
A6: cargo test -p deck-streak-api --test telegram_off -- --exact the_link_code_routes_answer_telegram_off_and_the_passkey_routes_linking_off
A8: cargo test -p deck-streak-daemon --test roles -- --exact the_held_flush_with_no_telegram_credential_keeps_what_it_holds
A9: cargo test -p deck-streak-notifications --test one_router -- --exact no_delivery_goes_around_the_port
A10: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_install_ships_no_telegram_unit_and_no_doorbell
A11: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_no_unit_loads_the_telegram_pair
A12: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_caddy_policy_admits_no_framing_and_sends_the_security_headers
A13: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k test_the_journal_sender_names_the_unit_and_result_at_error_priority
A14: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k test_the_journal_sender_reads_no_credential_and_makes_no_request
A15: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k test_the_alert_line_carries_no_line_of_the_failed_run
A16: python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k test_every_alert_routes_to_the_one_alert_path
A17: python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k test_the_mcp_slo_pages_and_tickets_on_the_one_alert_path_with_exact_burn_rates
A18: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k test_a_release_with_no_bot_unit_deploys_and_restarts_the_api_alone
A19: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_daemons_and_the_largest_job_fit_the_stack_share
A20: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k test_the_contract_carries_no_telegram_or_second_route_row
A21: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_install_ships_no_second_route_unit_and_keeps_its_script
A22: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_no_unit_but_the_exempt_reaches_the_host_identity_endpoint
```

The red-first record is `docs/red-first/SPEC-582.md`. A1, A2, A8 and A9 are seen red on the tree
before the change, locally and in CI; A10 to A21 are seen red in CI on the first push, which
carries the tests alone; A3 and A22 pin behaviour the base already has, and A4 to A6 pin new
functions the first push cannot compile, so each of those is marked not red with its rows of
`scripts/mutation-rows.d/S58200-S58299.json` as its reds.

## 4. File manifest

| path | part | change |
|---|---|---|
| `deploy/systemd/deck-streak-bot.service` | deploy | deleted |
| `deploy/systemd/deck-streak-job@sync.path` | deploy | deleted |
| `deploy/tmpfiles.d/deck-streak-sync-request.conf` | deploy | deleted |
| `deploy/systemd/deck-streak-job@held_flush.service.d/20-bot-credentials.conf` | deploy | deleted |
| `deploy/systemd/deck-streak-second-route.service` | deploy | deleted |
| `deploy/systemd/deck-streak-second-route.timer` | deploy | deleted |
| `deploy/scripts/alert-telegram.sh` | deploy | deleted |
| `deploy/scripts/alert-journal.sh` | deploy | added |
| `deploy/systemd/deck-streak-alert@.service` | deploy | changed: the sender and its comments, no credential |
| `deploy/systemd/deck-streak-api.service` | deploy | changed: no credential, its comments |
| `deploy/deploy.sh` | deploy | changed: no bot restart |
| `deploy/caddy/deck-streak.caddy` | deploy | changed: no framing origin |
| `deploy/deck-streak.env.example` | deploy | changed: two settings removed |
| `deploy/slo.json` | deploy | changed: routes and channel |
| `deploy/rail-contract.json` | deploy | changed: four rows removed, one changed |
| `deploy/host-budget.json` | deploy | changed: two entries removed |
| `deploy/README.md` | docs | changed |
| `docs/OWNER-SETUP.md` | docs | changed |
| `crates/identity/src/owner.rs` | identity | changed: the held gate |
| `crates/identity/src/lib.rs` | identity | changed: the `telegram_off` refusal |
| `crates/identity/tests/owner.rs` | identity | changed: A4, A5 |
| `crates/api/src/session_routes.rs` | api | changed: the off gate, appended |
| `crates/api/src/linking_routes.rs` | api | changed: the link-code routes off with Telegram |
| `crates/api/tests/telegram_off.rs` | api | added: A6 |
| `crates/daemon/src/role_api.rs` | daemon | changed: the start's three cases |
| `crates/daemon/src/role_job.rs` | daemon | changed: the held flush with no transport |
| `crates/daemon/src/snapshot_lister.rs` | daemon | changed: the lister read with no credentials directory |
| `crates/daemon/tests/lifecycle.rs` | daemon | changed: A1, A2 |
| `crates/daemon/tests/roles.rs` | daemon | changed: A8 |
| `crates/daemon/tests/snapshot_lister.rs` | daemon | changed: the lister with no credentials directory refuses a malformed command |
| `crates/agent/tests/deck_gate_census.rs` | agent | changed: the census declares the third job runner call |
| `crates/notifications/tests/one_router.rs` | notifications | changed: the alert path's exemption removed |
| `scripts/tests/test_deploy_templates.py` | tests | changed |
| `scripts/tests/test_deploy_scripts.py` | tests | changed |
| `scripts/tests/test_alert_unit.py` | tests | changed |
| `scripts/tests/_units.py` | tests | changed: the alert template's key list drops its credential pair, and the paging units' key list drops the write path key only the bot unit held |
| `scripts/tests/test_slo_declaration.py` | tests | changed |
| `scripts/tests/test_rail_contract.py` | tests | changed |
| `scripts/tests/test_caddy_render.py` | tests | changed |
| `scripts/tests/test_sync_path.py` | tests | deleted |
| `scripts/tests/test_memory_watch.py` | tests | changed: the memory watch's second unit is the MCP server, the bot's cgroup leaving with the bot |
| `scripts/tests/fixtures/cgroup/system.slice/deck-streak-bot.service/memory.current` | tests | deleted: the bot's cgroup leaves with the bot |
| `scripts/tests/fixtures/cgroup/system.slice/deck-streak-bot.service/memory.events` | tests | deleted: the bot's cgroup leaves with the bot |
| `scripts/tests/fixtures/cgroup/system.slice/deck-streak-bot.service/memory.max` | tests | deleted: the bot's cgroup leaves with the bot |
| `scripts/tests/fixtures/cgroup/system.slice/deck-streak-mcp.service/memory.current` | tests | added: the MCP server's cgroup |
| `scripts/tests/fixtures/cgroup/system.slice/deck-streak-mcp.service/memory.events` | tests | added: the MCP server's cgroup |
| `scripts/tests/fixtures/cgroup/system.slice/deck-streak-mcp.service/memory.max` | tests | added: the MCP server's cgroup, its ceiling the host budget's |
| `scripts/mutation-rows.d/S04100-S04199.json` | rows | changed: one row re-anchored, its exemption gone |
| `scripts/mutation-rows.d/S05900-S05999.json` | rows | changed: three rows whose targets leave |
| `scripts/mutation-rows.d/S06200-S06299.json` | rows | changed: one row leaves, two killers renamed |
| `scripts/mutation-rows.d/S06600-S06699.json` | rows | changed: four rows whose target leaves |
| `scripts/mutation-rows.d/S33300-S33399.json` | rows | changed: one killer renamed |
| `scripts/mutation-rows.d/S35400-S35499.json` | rows | changed: two rows whose target leaves |
| `scripts/mutation-rows.d/S39600-S39699.json` | rows | changed: two rows whose target leaves |
| `scripts/mutation-rows.d/S58200-S58299.json` | rows | added |
| `docs/specs/SPEC-582-the-install-carries-no-telegram-surface.md` | docs | added |
| `docs/decisions/ADR-601-the-install-carries-no-telegram-surface-before-the-telegram-code-leaves-the-tree.md` | docs | added |
| `docs/schematics/the-install-carries-no-telegram-surface.md` | docs | added |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | docs | changed: seven cites follow their moved tests |
| `docs/red-first/SPEC-582.md` | docs | added |
| `docs/decisions/ADR-589-web-and-ios-only-mini-app-and-bot-deprecated.md` | docs | appended |
| `docs/decisions/ADR-347-the-sync-server-is-built-from-the-engines-pin-reads-hashed-users-from-credentials-and-serves-under-its-own-path.md` | docs | appended |
| `docs/specs/SPEC-024-identity-initdata-and-owner-pin.md` | docs | appended |
| `docs/specs/SPEC-031-observability-foundation.md` | docs | appended |
| `docs/specs/SPEC-032-deploy-templates.md` | docs | appended |
| `docs/specs/SPEC-059-the-owners-sync-runs-as-the-sync-job-through-a-path-unit.md` | docs | appended |
| `docs/specs/SPEC-062-first-deploy-units-caddy-block-and-https.md` | docs | appended |
| `docs/specs/SPEC-396-a-second-route-tells-the-owner-when-the-alert-sender-fails-or-goes-silent.md` | docs | appended |
| `changelog.d/tg-free-install-582.md` | docs | added |

## 5. What this does NOT cover

- The Telegram code's removal (`crates/bot/`, identity's launch-data port, the web app's Telegram
  mapping, the root `.env.example`'s bot settings), ADR-589's removal delivery after native push
  carries the router (#640).
- A transport for the router and the held flush outside a session, native push (#640).
- The reader of the journal sender's line, its alert and its channel off the host: the monitoring
  delivery (#1069).
- The apps' own owner sign-in, which supplies an owner pin that does not come from Telegram
  (#1070).

Outside this repository and outside this SPEC: the host's install, its rail map and its stored
secrets. No change here touches the engine, the scheduler, a web screen or the bot's code.

## 6. Risks

- **No page leaves the host.** Until the monitoring delivery lands, a failure is one line at error
  priority and an entry in `systemctl --failed`. The template's name and its callers stay, so that
  delivery changes one sender.
- **No web sign-in on the install.** Until the apps' own owner sign-in lands, no session opens; the
  sync server, the jobs and the backups run without one.
- **The held queue grows.** With no transport, held items stay; the first flush through a joined
  transport abandons what outlived the policy's age and names it in its recap.
- **A pack row reads the old path.** The observability pack's alert-route row asks for a Telegram
  sender and channel; it reads this install as a broken alert path until the pack learns the
  journal channel.
