# SPEC-032: every unit, timer and the Caddy block exist as hardened templates inside a declared host budget, and none names a private value

- **Wave:** W0. **Issue:** #25 (epic #1). **Context(s):** `deploy` (`deploy/`), `deck-streak-coordination` (the timer-versus-table test), `repo` (`.packs/wiring.json`).
- **Decided by:** ADR-007 (one Caddy site block, loopback API, the security headers, `noindex`), ADR-010 (hardened units per role, `MemoryHigh` below `MemoryMax` from the host budget), ADR-038 (credentials by `LoadCredential=` from the credential socket at each start, superseding ADR-010's `LoadCredentialEncrypted=`), ADR-011 (side by side with the predecessor), ADR-025 (health closed at the edge), ADR-027 (the timers and their slots), and this SPEC's ADR-032 (neutral, lint-valid templates the private rail fills; the host budget's numbers).
- **Status:** judged: delivered with its tests and `docs/red-first/SPEC-032.md`. The delivery
  amended §1, R4, R8, A4's box reading, §6, the manifest and ADR-032, each for the reason §7 gives.

## 1. The problem, measured

- **What exists.** `docs/schematics/deployment.md` draws the units (`deck-streak-api.service` and
  `deck-streak-bot.service` as `notify` with a watchdog, `deck-streak-job@<name>.service` behind
  timers, the alert and memory-watch units) and the Caddy site; `deploy/` holds none of them (read
  at `main` e05dfa5). `.packs/wiring.json` DEFERS the whole durable-services pack to this issue,
  because it reads a tree with no unit as a finding rather than as VOID.
- **The predecessor's unit, which set the pattern** (predecessor `27ee2bc`, `deploy/` and the
  inventory's `sd-notify-watchdog` entry): `Type=notify`, `WatchdogSec=90`, `Restart=on-failure`,
  `RestartSec=15`, a start limit of 5 in 300 seconds, `TimeoutStartSec=180`, a memory ceiling,
  `OOMPolicy=kill`.
- **The budget.** DeckStreak's share of the host is ADR-032's, read from `deploy/host-budget.json`;
  the host's own capacity is private configuration.
- **The rows that judge the templates:** durable-services' 35 blocking tree rows (the gate, once
  enforced), rust-service's unit-tied rows (`rs.notify-ready`, `rs.watchdog-ping`,
  `rs.sigterm-handled`, `rs.credentials-read`), web-security's header rows over the Caddy block (on
  the box), and the public scrub over `deploy/`.

**Order.** After SPEC-025 (the binary whose roles the units run, sending `READY=1` and
`WATCHDOG=1`) and SPEC-027 (the job table the timers are written from). SPEC-026 may land before or
after it: the bot unit is a template for the role SPEC-026 adds. SPEC-031 lands after it: the SLO
names this SPEC's API unit, and until then the observability rows that need an SLO are deferred
here.

## 2. Requirements

R1. `deploy/systemd/` holds `deck-streak-api.service` and `deck-streak-bot.service` (`Type=notify`,
    `WatchdogSec=90`, `Restart=on-failure`, `RestartSec=15`, `StartLimitBurst=5` in
    `StartLimitIntervalSec=300`, `TimeoutStartSec=180`, `TimeoutStopSec=30`, `OOMPolicy=kill`,
    `WantedBy=multi-user.target`), `deck-streak-job@.service` (`Type=oneshot`, `Nice=10`, idle IO
    class) and one `deck-streak-job@<id>.timer` per entry of coordination's job table (`sync`,
    `maintenance`, `liveness`), each `WantedBy=timers.target`.
R2. Every service runs `deckstreakd <role>` from the release root's `current` link, as the system user
    `deck-streak`, with `StateDirectory=deck-streak`, `ProtectSystem=strict`, `ProtectHome=yes`,
    `PrivateTmp=yes`, `PrivateDevices=yes`, `NoNewPrivileges=yes`,
    `SystemCallFilter=@system-service`, `SystemCallArchitectures=native`,
    `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6` (`AF_UNIX` for sd_notify), an empty
    `CapabilityBoundingSet=`, the remaining hardening the pack's advisory rows score, `UMask=0077`,
    `SyslogIdentifier=` its own name, and `OnFailure=deck-streak-alert@%n.service` (the alert unit is
    SPEC-031's).
R3. Secrets reach a unit only as `LoadCredential=<id>:/run/deck-streak-credentials/socket`
    (ADR-038), `<id>` being one of DeckStreak's own credential ids and the path the credential socket
    the private rail serves; never `LoadCredentialEncrypted=`, never a credential in an
    `Environment=` line, and never a literal. Non-secret settings come from one required
    `EnvironmentFile=` (never the optional `-` form), whose committed example,
    `deploy/deck-streak.env.example`, holds neutral values only.
R4. Each timer's calendar equals its job's slot in coordination's job table, written in UTC as the
    neutral zone (the private rail renders the owner's zone, ADR-027); `Persistent=` is set only on
    `catch_up` jobs (`sync` alone: SPEC-027 and ADR-037 made it the table's one catch-up job), and a
    timer that departs from a durable-services advisory
    (`timers.catch-up`, `timers.spread`) says why in `X-DurableServices-Waive=`.
R5. `deploy/host-budget.json` declares DeckStreak's share, `"memory": "640M"` and `"cpus": 2`, and each
    unit's `memory_high` and `memory_max` (ADR-032); every unit's `MemoryHigh=` and `MemoryMax=` equal
    its entry, `MemoryHigh` below `MemoryMax`, and the long-running units' ceilings plus the largest
    oneshot's fit the share (durable-services `resources.memory-order`, `resources.budget`).
R6. `deploy/caddy/deck-streak.caddy` is one site block for `{$DECKSTREAK_HOST}`: static files from
    `{$DECKSTREAK_WEB_ROOT}` with `try_files {path} {path}.html /index.html`; `/api/*` proxied to
    `{$DECKSTREAK_API_UPSTREAM}` (a loopback address); `404` for `/api/livez` and `/api/readyz` from
    outside (ADR-025); a `robots.txt` that disallows everything; and the headers
    `Strict-Transport-Security` (at least a year, `includeSubDomains`), `X-Content-Type-Options:
    nosniff`, a `Referrer-Policy` that keeps URLs from other origins, `X-Robots-Tag: noindex`, the
    `Server` header removed, and a `Content-Security-Policy` of `frame-ancestors
    https://web.telegram.org; object-src 'none'; base-uri 'self'` (the script policy is the page's own
    meta policy with its build's hashes, SPEC-028). No `auto_https off`, no `tls internal`, no TLS
    below 1.2.
R7. No template carries an IP address other than loopback, a host name, a path of the maintainer's
    machines, a cloud project id or a secret name: concrete values are the private rail's
    (ADR-032).
R8. `.packs/wiring.json` moves durable-services to `enforced`, with `backup.copies`,
    `backup.offsite` and `backup.restore-drill` deferred to the backups issue (#44; the daily
    backup and the restore drill are W2's); and defers the observability rows that need an SLO or the alert unit
    (`obs.slo-declared`, `obs.error-budget-policy`, `obs.burn-rate-alerts`, `obs.slo-measurable`,
    `obs.alert-route`, `obs.failure-alerts`, `obs.memory-watch`) to the observability issue
    (#24), which lifts them, and so moves observability to `enforced`, since every other blocking
    row of the pack then runs and passes (SPEC-030 R6); and moves the box's web-security expectation
    from `ws.csp-present`, which the Caddy block turns green, to `ws.csp-script-strict` (#60), which
    reads the served policy alone (§7).
R9. Nothing here touches a host: no unit is installed, no Caddy is reloaded (the first deploy, with
    the owner's go, is W2's).
R10. No committed file writes a template instance name literally (a template, `@`, an instance, a
    dot and a unit type): the public scrub reads that shape as an email address. A timer relies on
    systemd's default `Unit=` (the service of its own name) instead of naming it, and tests build
    instance names at run time.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| ~~A1~~ | the durable lint finds no blocking defect in the templates (examined units, zero refused) | `test_deploy_templates.py`; durable-services tree rows, enforced |
| A2 | every unit's `MemoryHigh` is below its `MemoryMax`, and both equal its host-budget entry | `test_deploy_templates.py`; `resources.memory-order` |
| A3 | the long-running units' ceilings plus the largest oneshot's fit the stack's share | `test_deploy_templates.py`; `resources.budget` |
| A4 | the Caddy block's policy admits Telegram Web as a framer and sends the security headers | `test_deploy_templates.py`; web-security header rows on the box |
| A5 | the Caddy block serves the SPA with its fallback, proxies `/api/*` to loopback, and hides the health routes | `test_deploy_templates.py` |
| A6 | no deploy template names a private value, and a planted one is refused | `test_deploy_templates.py`; the public scrub |
| A7 | every timer's calendar equals its job's slot in the job table | coordination `job_table` test |
| A8 | no unit passes a secret through its environment | `test_deploy_templates.py`; durable-services `secrets.*` |
| A9 | every credential line of every template has the socket form, and a planted `LoadCredentialEncrypted=` line is refused (examined count reported) | `test_deploy_templates.py` |

```acceptance
```
```retired
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_durable_lint_finds_no_blocking_defect_in_the_templates
```
```acceptance
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_ceiling_matches_the_host_budget_and_high_is_below_max
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_daemons_and_the_largest_job_fit_the_stack_share
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_caddy_policy_admits_telegram_web_and_sends_the_security_headers
A5: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_caddy_block_serves_the_spa_proxies_the_api_and_hides_health
A6: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k no_deploy_template_names_a_private_value
A7: cargo test -p deck-streak-coordination --test job_table -- --exact every_timer_calendar_equals_its_job_table_entry
A8: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k no_unit_passes_a_secret_through_its_environment
A9: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_credential_line_has_the_socket_form_and_encrypted_is_refused
```

A1 runs the vendored `durable-unit-lint.py lint --root . --format json` and asserts the examined
units and zero blocking findings; A6 runs `scripts/public-scrub.py` over `deploy/`, and over a planted
template the test writes at run time into a `TemporaryDirectory`, carrying a private-range address
assembled from its octets in the test, so no address literal is ever committed. A9 reads every
`LoadCredential*=` line of every template, reports how many it examined, and refuses a planted
template written at run time into a `TemporaryDirectory`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `deploy/systemd/deck-streak-api.service` | deploy | added |
| `deploy/systemd/deck-streak-bot.service` | deploy | added |
| `deploy/systemd/deck-streak-job@.service` | deploy | added |
| `deploy/systemd/deck-streak-job@<id>.timer`, one per job of the table (`sync`, `maintenance`, `liveness`) | deploy | added |
| `deploy/deck-streak.env.example` | deploy | added: neutral values only |
| `deploy/host-budget.json` | deploy | added |
| `deploy/caddy/deck-streak.caddy` | deploy | added |
| `deploy/README.md` | deploy | added: what each template is, and that the private rail fills it |
| `scripts/tests/test_deploy_templates.py` | repo | added: A1 to A6, A8, A9 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A7 |
| `.packs/wiring.json` | repo | changed: durable-services enforced with three deferred rows; observability enforced with seven deferred rows; the box's web-security and cyber-pipeline expectations (§7) |
| `docs/schematics/deployment.md` | repo | changed: the budget per unit, and a neutral label for the edge |
| `docs/decisions/ADR-032-deploy-templates-and-the-host-budget.md` | repo | changed: accepted, with the decisions made at delivery |
| `docs/red-first/SPEC-032.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It installs nothing on any host and reloads no Caddy; the first deploy is W2's, with the owner's
  go (#42).
- It writes no deploy or rollback script (#42).
- It provisions no credential and no environment file on the host (#41).
- It ships no credential fetch helper, no socket unit for it and no map of credential ids to secret
  names: the templates name only the socket path, and those pieces are the private rail's, outside
  this repository (ADR-038, #41).
- It builds no daily backup, offsite copy or restore drill (#44).
- It ships no alert unit, SLO, evaluator or memory watch (#24).
- It opens no tunnel for the agent (#43).
- It serves no landing page (#59).

## 6. Risks

- **The budget is wrong for the live host.** The numbers are ADR-032's design; the memory watch
  (SPEC-031) and the first day of the W2 deploy measure them, and a resize is an owner decision
  (ADR-011).
- **The owner's `/sync` meets the bot's ceiling.** SPEC-026 R11 runs the sync cycle inside the bot's
  own process, whose `MemoryMax=` is 96M, while a full download may take the sync's whole budget of
  256 MiB (ADR-022). An incremental sync fits; a full download there is killed at the ceiling, fails
  the bot and pages. SPEC-026 decides whether `/sync` runs in the bot or starts the `sync` job, whose
  ceiling is sized for it.
- **A timer's zone is wrong on the host.** The templates carry UTC; the private rail renders the
  owner's zone, and the liveness job's drift check pages on the first maintenance fire that lands
  off its slot (SPEC-027).
- **The Caddy change reaches beyond DeckStreak.** The block is its own site; installing it is the
  owner's gate in W2, and web-security's header rows are read on the box before that.
- **An instance name trips the public scrub.** A literal such as a job template's instance name
  followed by `.timer` matches the scrub's email shape and fails the gate's scrub stage; R10 keeps
  such names out of every committed file, and the scrub itself names the file and line if one slips
  in.
- **A hardening option breaks a role.** `MemoryDenyWriteExecute=` and the system-call filter are
  exercised when W2 first starts the units; a denied call shows as the unit's failure with its
  result, paged through the alert unit.
- **A credential cannot be fetched when a unit starts.** The unit does not start, and its
  `OnFailure=` alert names the credential id, never a value (ADR-038); the service's restart policy
  retries.
- **A template's instances reach the socket under their instance names.** systemd names the unit in
  the address it binds for each credential (ADR-038), and the job and alert templates run under a
  new instance name each time, so the private rail's map must match an instance by its template
  (#41); W2's rehearsal proves it on the host.

## 7. Amended in delivery

The code, the packs' measurements and the public-prose rule changed these statements of the planned
SPEC. Each is corrected above; the reasons are these.

- **§1, §6 and the schematic: reworded under the public-prose rule.** §1's host bullet is now the
  budget's: it names ADR-032's share and `deploy/host-budget.json`, and no figure or tenant of the
  host. §6's Caddy risk and its budget risk name nothing of the host beyond that share. §1 keeps the
  predecessor's unit pattern and drops the pack's findings over the predecessor's tree, and
  `docs/schematics/deployment.md` labels the edge as the host's reverse proxy.
- **R4: `sync` is the one catch-up job.** SPEC-027's job table sets `catch_up` on `sync` (ADR-037),
  so its timer alone carries `Persistent=true`; the planned "none at W0" predated ADR-037's
  amendment of the plan. The `maintenance` and `liveness` timers waive `timers.catch-up`, and every
  timer waives `timers.spread`, each with its why: the table places each job on its own minute, and
  a random delay would move a fire off it.
- **R8: observability is enforced.** With its seven rows deferred, observability's six other
  blocking rows run and pass, and `scripts/pack-rows.py` refuses that pending state as stale
  (measured: `STALE observability:* pending on #24, but every blocking row ran and passed (6
  row(s))`). The pack is enforced now, with the seven rows deferred to #24, and SPEC-031's R7 lifts
  the deferrals when it ships the SLO, the alert unit and the memory watch.
- **R8 and A4: one header row reads the served policy alone.** On the maintainer's box the Caddy
  block turns web-security's `ws.csp-present` green, so its expectation (#25) is lifted in
  web-security and in cyber-pipeline, and every other header row is green: HSTS and its
  subdomains, `nosniff`, the referrer policy, `frame-ancestors`, `object-src` and `base-uri`, the
  HTTPS redirect, the TLS floor, the removed `Server` header and the closed directory listing.
  `ws.csp-script-strict` turns red, because it judges each policy alone and R6's served policy
  carries no `script-src` or `default-src` by design: the page's own meta policy carries the script
  sources with its build's hashes (SPEC-028 R14), which the probe does not read. A served script
  policy strict enough for the row would block the scripts those hashes admit, so the row is an
  expected red owned by #60, which brings every binary-built pack to green.
- **ADR-032: decided at delivery.** The neutral release root and settings file, the daemons'
  processor and task caps, the job template's start timeout, and each unit's credentials, each
  recorded in ADR-032 with what it was chosen against.
- **§6: one new risk.** Loading the sync's account into the bot's unit, for the owner's `/sync`,
  showed that SPEC-026 runs the sync cycle inside the bot, beside ADR-032's bot ceiling.
- **The manifest.** The `.packs/wiring.json` row names every change above, and ADR-032 is changed
  (accepted) rather than added, since the plan wrote it.

## 8. Amendment, 2026-09-28: criteria whose tests SPEC-056 removed

Made by SPEC-056 (ADR-069), insert-only under ruling (i) of SPEC-038 section 8: every earlier byte
is kept in order. It inserts:

- section 3: `~~` around A1 in the criteria table, so the table no longer states them;
- section 3: the fence lines that set their commands apart in a `` ```retired `` fence, between the
  acceptance fence's two halves;
- this section.

The retired criteria, why their subject is gone, and what judges it now:

- A1 (the durable lint finds no blocking defect in the templates): SPEC-056 removed the vendored
  lint the test ran. The box run's durable-services pack, enforced, judges the templates.

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).
