# SPEC-032: every unit, timer and the Caddy block exist as hardened templates inside a declared host budget, and none names a private value

- **Wave:** W0. **Issue:** #25 (epic #1). **Context(s):** `deploy` (`deploy/`), `deck-streak-coordination` (the timer-versus-table test), `repo` (`.packs/wiring.json`).
- **Decided by:** ADR-007 (one Caddy site block, loopback API, the security headers, `noindex`), ADR-010 (hardened units per role, credentials by `LoadCredentialEncrypted=`, `MemoryHigh` below `MemoryMax` from the host budget), ADR-011 (side by side with the predecessor), ADR-025 (health closed at the edge), ADR-027 (the timers and their slots), and this SPEC's ADR-032 (neutral, lint-valid templates the private rail fills; the host budget's numbers).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-032.md` (ADR-016).

## 1. The problem, measured

- **What exists.** `docs/schematics/deployment.md` draws the units (`deck-streak-api.service` and
  `deck-streak-bot.service` as `notify` with a watchdog, `deck-streak-job@<name>.service` behind
  timers, the alert and memory-watch units) and the Caddy site; `deploy/` holds none of them (read
  at `main` e05dfa5). `.packs/wiring.json` DEFERS the whole durable-services pack to this issue,
  because it reads a tree with no unit as a finding rather than as VOID.
- **The predecessor's unit, which set the pattern** (predecessor `27ee2bc`, `deploy/` and the
  inventory's `sd-notify-watchdog` entry): `Type=notify`, `WatchdogSec=90`, `Restart=on-failure`,
  `RestartSec=15`, a start limit of 5 in 300 seconds, `TimeoutStartSec=180`, a memory ceiling,
  `OOMPolicy=kill`. The durable-services pack measured that tree: one blocking finding (no
  `SystemCallFilter=` allow list on its daemons) and 39 advisory ones (the pack's SKILL, "Measured
  on real subjects").
- **The host.** Two vCPU and 1.9 GiB of RAM, shared with the predecessor (until cutover) and a
  co-hosted stack, behind a Caddy the co-hosted stack also uses (CHARTER 3, ADR-007); DeckStreak's
  share is ADR-032's.
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
R3. Secrets reach a unit only through `LoadCredentialEncrypted=<id>` with no path (systemd resolves
    the credential store); no secret-named `Environment=` exists; non-secret settings come from one
    required `EnvironmentFile=` (never the optional `-` form), whose committed example,
    `deploy/deck-streak.env.example`, holds neutral values only.
R4. Each timer's calendar equals its job's slot in coordination's job table, written in UTC as the
    neutral zone (the private rail renders the owner's zone, ADR-027); `Persistent=` is set only on
    `catch_up` jobs (none at W0), and a timer that departs from a durable-services advisory
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
    (#24), which lifts them.
R9. Nothing here touches a host: no unit is installed, no Caddy is reloaded (the first deploy, with
    the owner's go, is W2's).
R10. No committed file writes a template instance name literally (a template, `@`, an instance, a
    dot and a unit type): the public scrub reads that shape as an email address. A timer relies on
    systemd's default `Unit=` (the service of its own name) instead of naming it, and tests build
    instance names at run time.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the durable lint finds no blocking defect in the templates (examined units, zero refused) | `test_deploy_templates.py`; durable-services tree rows, enforced |
| A2 | every unit's `MemoryHigh` is below its `MemoryMax`, and both equal its host-budget entry | `test_deploy_templates.py`; `resources.memory-order` |
| A3 | the long-running units' ceilings plus the largest oneshot's fit the stack's share | `test_deploy_templates.py`; `resources.budget` |
| A4 | the Caddy block's policy admits Telegram Web as a framer and sends the security headers | `test_deploy_templates.py`; web-security header rows on the box |
| A5 | the Caddy block serves the SPA with its fallback, proxies `/api/*` to loopback, and hides the health routes | `test_deploy_templates.py` |
| A6 | no deploy template names a private value, and a planted one is refused | `test_deploy_templates.py`; the public scrub |
| A7 | every timer's calendar equals its job's slot in the job table | coordination `job_table` test |
| A8 | no unit passes a secret through its environment | `test_deploy_templates.py`; durable-services `secrets.*` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_durable_lint_finds_no_blocking_defect_in_the_templates
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_ceiling_matches_the_host_budget_and_high_is_below_max
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_daemons_and_the_largest_job_fit_the_stack_share
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_caddy_policy_admits_telegram_web_and_sends_the_security_headers
A5: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_caddy_block_serves_the_spa_proxies_the_api_and_hides_health
A6: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k no_deploy_template_names_a_private_value
A7: cargo test -p deck-streak-coordination --test job_table -- --exact every_timer_calendar_equals_its_job_table_entry
A8: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k no_unit_passes_a_secret_through_its_environment
```

A1 runs the vendored `durable-unit-lint.py lint --root . --format json` and asserts the examined
units and zero blocking findings; A6 runs `scripts/public-scrub.py` over `deploy/`, and over a planted
template the test writes at run time into a `TemporaryDirectory`, carrying a private-range address
assembled from its octets in the test, so no address literal is ever committed.

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
| `scripts/tests/test_deploy_templates.py` | repo | added: A1 to A6, A8 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A7 |
| `.packs/wiring.json` | repo | changed: durable-services enforced with three deferred rows; seven observability rows deferred |
| `docs/schematics/deployment.md` | repo | changed: the budget per unit |
| `docs/decisions/ADR-032-deploy-templates-and-the-host-budget.md` | repo | added |
| `docs/red-first/SPEC-032.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It installs nothing on any host and reloads no Caddy; the first deploy is W2's, with the owner's
  go (#42).
- It writes no deploy or rollback script (#42).
- It provisions no credential and no environment file on the host (#41).
- It builds no daily backup, offsite copy or restore drill (#44).
- It ships no alert unit, SLO, evaluator or memory watch (#24).
- It opens no tunnel for the agent (#43).
- It serves no landing page (#59).

## 6. Risks

- **The budget is wrong for the live host.** The numbers are ADR-032's design, sized beside the
  predecessor's ceiling; the memory watch (SPEC-031) and the first day of the W2 deploy measure them,
  and a resize is an owner decision (ADR-011).
- **A timer's zone is wrong on the host.** The templates carry UTC; the private rail renders the
  owner's zone, and the liveness job's drift check pages on the first maintenance fire that lands
  off its slot (SPEC-027).
- **The Caddy change affects the co-hosted service.** The block is its own site; installing it is the
  owner's gate in W2, and web-security's header rows are read on the box before that.
- **An instance name trips the public scrub.** A literal such as a job template's instance name
  followed by `.timer` matches the scrub's email shape and fails the gate's scrub stage; R10 keeps
  such names out of every committed file, and the scrub itself names the file and line if one slips
  in.
- **A hardening option breaks a role.** `MemoryDenyWriteExecute=` and the system-call filter are
  exercised when W2 first starts the units; a denied call shows as the unit's failure with its
  result, paged through the alert unit.
