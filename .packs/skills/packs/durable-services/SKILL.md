---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/durable-services

Durable services done right: systemd services and timers on a SMALL host shared with other stacks
(2 vCPU, 1.9 GiB), with their backups and restore drills. The pack has two halves (SPEC-V2-2155,
SPEC-V2-2195):

- **92 portable `tree` rows** judge ANY repository's `deploy/` units offline through `--root`:
  35 blocking and 57 advisory, in nine stages. The first consumer is DeckStreak.
- **22 phoenix-only `live` rows** probe the cron services this box runs, plus the two aol timers.
  They read box state; against another repository they refuse by name.

Which seats consume this pack is its catalog row's `consumes`, the one record of that edge
(ADR-V2-1990), so this body names none.

```
phxd pack probe --pack durable-services --root PATH --format json
```

## Running it

Box-side, against a project's checkout (the tree rows find their script through `{skills}`, so the
project needs no copy of it):

```
phxd pack probe --pack durable-services --scope tree --root /path/to/project --format json
```

On the box, phoenix's own services (what the pack-probe tick runs every quarter hour):

```
phxd pack probe --pack durable-services --scope live --root PATH --format json
```

Directly, or vendored into a project's own CI (one stdlib-only file, no phoenix import):

- `python3 scripts/durable-unit-lint.py lint --root PATH [--format json]` runs all 92 checks and
  exits 1 only on a blocking finding. The text form names every finding, its unit and its line.
- `python3 scripts/durable-unit-lint.py check --id ID --root PATH` runs one check. It exits 1 on
  any finding; the row's severity decides whether that refuses.
- `python3 scripts/durable-unit-lint.py catalog` prints the catalog.

Exits of `phxd pack probe`:

- **0**: every `block` row green. An `advisory` row that found something prints the verdict
  `advisory` and changes neither the card's color nor the exit.
- **4**: a red `block` row, or a refusal (`unknown_pack`, a deny-listed token, catalog drift,
  `--root` not a directory, an unbounded probe).
- **3**: the pack held no row of the `--scope` asked for (`void_probe`).

A card row carries only its child's exit (i1879), so read a red row's reason by running the script
above.

Unscoped on phoenix's own root, the tree rows refuse `units-none`: phoenix ships no `deploy/`
subject, and a guard that examined nothing refuses. Run `--scope live` here and `--scope tree` in a
project.

## Subject layout (the portable contract)

- `deploy/**/*.service`, `*.timer` and `*.slice` are the units. Drop-ins in
  `deploy/**/<unit>.d/*.conf` are read after the unit, as systemd reads them.
- A unit under a directory named `user` (`deploy/user/`) is a user unit. The sandboxing that needs a
  mount namespace does not work there (systemd.exec(5), SANDBOXING), so it is not judged.
- `deploy/**/litestream*.yml` is the Litestream config, in the v0.5 form.
- `deploy/**/*.env*` files are examples: placeholders only for secret-named keys.
- `deploy/host-budget.json` is `{"memory": "1400M", "cpus": 2}`, the share of the host this stack
  may use. Every ceiling must fit it.
- `deploy/**/journald.conf.d/*.conf` is the journald drop-in (`SystemMaxUse=`).
- Scripts under `deploy/` are read when a unit's command names them by basename. That is how a
  restore drill proves it runs `litestream restore` and `PRAGMA integrity_check`.
- A backup job is a timer-activated `*-backup.service`. A restore drill is a timer-activated
  `*-restore-drill.service`, or one whose command runs `litestream restore`.
- Long-running means a service whose `Type=` is not `oneshot`. "Scheduled" means a subject timer
  activates it.

## Severity and waivers

`blocking` rows are firm requirements. Each is one of:
- a statement of the official documentation ("must", "rejected", "deprecated", "only has an
  effect", "is not suitable for passing secrets");
- a recommendation it makes for EVERY long-running service ("recommended to enable this setting
  for all long-running services");
- a standard: NIST SP 800-53 AC-6 least privilege, or CISA's 3-2-1 backups and regular restore
  testing.

`advisory` rows are best practice the docs recommend for "most services" or leave to judgement.
They never refuse.

An advisory finding is waived for one unit by an `X-` key, which systemd itself ignores:
`X-DurableServices-Waive=<reason> <why>` in `[Unit]`. A waiver with no why is not a waiver. A
blocking row cannot be waived.

## The tree rows: 92 checks in nine stages (35 blocking, 57 advisory)

| stage | rows | blocking | advisory |
|---|---|---|---|
| inventory | 2 | 2 | 0 |
| directives | 3 | 3 | 0 |
| service | 19 | 9 | 10 |
| sandbox | 29 | 4 | 25 |
| resources | 8 | 2 | 6 |
| secrets | 5 | 3 | 2 |
| logging | 3 | 0 | 3 |
| timers | 14 | 7 | 7 |
| backup | 9 | 5 | 4 |
| **total** | **92** | **35** | **57** |

Each row's probe is
`python3 {skills}/../scripts/durable-unit-lint.py check --id <id> --root {root} --format json`,
scope `tree`, with a 30-second wall.

### Stage `inventory`: 2 rows (2 blocking, 0 advisory)

inventory — is there a subject.

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `inventory.units-present` | blocking | `units-none` | SPEC-V2-1840: a guard that examined nothing refuses; the subject is deploy/*.service\|timer |
| `inventory.syntax` | blocking | `unit-syntax` | systemd.syntax(7) and systemd.unit(5): sections, Key=Value lines and valid unit names; systemd ignores an unknown section |

### Stage `directives`: 3 rows (3 blocking, 0 advisory)

directives — what systemd 255 will and will not read.

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `directives.known` | blocking | `key-unknown` | systemd 255 load-fragment table: an unknown or misplaced key is ignored with 'Unknown key name ... ignoring' (systemd-analyze verify) |
| `directives.deprecated` | blocking | `directive-deprecated` | systemd NEWS 229/230/240/246/252 and the v258 load-fragment table; systemd-analyze verify warns on each |
| `directives.absolute-paths` | blocking | `path-not-absolute` | systemd.exec(5): EnvironmentFile=, ReadWritePaths=, WorkingDirectory= take absolute paths; systemd-analyze verify ignores or refuses a relative one |

### Stage `service`: 19 rows (9 blocking, 10 advisory)

service — lifecycle, restart, stop, watchdog.

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `service.exec-start` | blocking | `execstart-missing` | systemd-analyze verify: 'Service has no ExecStart=, ExecStop=, or SuccessAction=. Refusing.' |
| `service.type-forking` | blocking | `type-forking` | systemd.service(5) Type=: 'The use of this type is discouraged, use notify, notify-reload, or dbus instead.' |
| `service.restart` | blocking | `restart-missing` | systemd.service(5) Restart=: 'on-failure is the recommended choice for long-running services' |
| `service.oneshot-restart` | blocking | `oneshot-restart-rejected` | systemd.service(5) Restart=: 'always and on-success are rejected' for Type=oneshot; systemd-analyze verify refuses the unit |
| `service.kill-mode` | blocking | `kill-mode-escapes` | systemd.kill(5): KillMode=process 'not recommended!', none 'strongly recommended against!'; systemd 246 warns on none, verify calls it deprecated |
| `service.notify-socket` | blocking | `notify-socket-blocked` | sd_notify(3) sends an AF_UNIX datagram; systemd.exec(5) RestrictAddressFamilies= restricts socket(2), so Type=notify or WatchdogSec= without AF_UNIX can never report READY=1 |
| `service.network-online` | blocking | `network-online-half` | systemd.special(7) network-online.target: pull it in 'via a Wants= type dependency and order themselves after it' |
| `service.on-failure-inert` | blocking | `on-failure-inert` | systemd.unit(5) OnFailure= fires on 'failed', and StartLimit* is the only road there for a restarting unit; a limit that cannot trip leaves OnFailure= provably inert |
| `service.installable` | blocking | `install-missing` | systemctl(1) enable reads [Install]: a long-running service no subject unit pulls in, with no WantedBy=/RequiredBy=, never starts at boot |
| `service.type-exec` | advisory | `type-simple` | systemd.service(5) Type=: 'It is recommended to use Type=exec for long-running services' (notify when the service can report readiness) |
| `service.restart-sec` | advisory | `restart-sec-default` | systemd.service(5) RestartSec= 'Defaults to 100ms': a crash loop hammers a small host |
| `service.start-limit` | advisory | `start-limit-never-trips` | systemd.unit(5) StartLimitIntervalSec=/StartLimitBurst=: size them so a crash loop stops |
| `service.on-failure` | advisory | `on-failure-missing` | systemd.unit(5) OnFailure=: page someone when a daemon or a scheduled job reaches failed |
| `service.timeout-stop` | advisory | `timeout-stop-default` | systemd.service(5) TimeoutStopSec= defaults to DefaultTimeoutStopSec (90s); set the drain |
| `service.timeout-stop-finite` | advisory | `timeout-stop-infinite` | systemd.service(5) TimeoutStopSec=infinity disables the SIGKILL escalation, so a hung stop holds the shutdown |
| `service.watchdog` | advisory | `watchdog-missing` | systemd.service(5) WatchdogSec= with sd_notify WATCHDOG=1: liveness a crashed-but-running daemon cannot fake |
| `service.watchdog-notify` | advisory | `watchdog-without-notify` | systemd.service(5) WatchdogSec=: 'The service must call sd_notify(3) regularly with WATCHDOG=1'; Type=notify is how a service that does so says it is ready |
| `service.documentation` | advisory | `documentation-missing` | systemd.unit(5) Documentation=: 'first reference documentation that explains what the unit's purpose is' |
| `service.description` | advisory | `description-missing` | systemd.unit(5) Description=: the name systemctl and the journal show |

### Stage `sandbox`: 29 rows (4 blocking, 25 advisory)

sandbox — identity and the hardening systemd-analyze security scores (system units only).

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `sandbox.identity` | blocking | `runs-as-root` | systemd.exec(5) User=: 'the default is root' for system services; NIST SP 800-53 AC-6 least privilege; systemd-analyze security weighs User=/DynamicUser= highest |
| `sandbox.protect-system` | blocking | `protect-system-off` | systemd.exec(5) ProtectSystem=: 'recommended to enable this setting for all long-running services' |
| `sandbox.protect-home` | blocking | `protect-home-off` | systemd.exec(5) ProtectHome=: 'recommended to enable this setting for all long-running services (in particular network-facing ones)' |
| `sandbox.syscall-filter` | blocking | `syscall-allowlist-missing` | systemd.exec(5) SystemCallFilter=: 'recommended to enforce system call allow lists for all long-running system services' (@system-service) |
| `sandbox.no-new-privileges` | advisory | `no-new-privileges-off` | systemd.exec(5) NoNewPrivileges=: 'the simplest and most effective way to ensure that a process and its children can never elevate privileges' |
| `sandbox.private-tmp` | advisory | `private-tmp-off` | systemd.exec(5) PrivateTmp=: private /tmp and /var/tmp |
| `sandbox.private-devices` | advisory | `private-devices-off` | systemd.exec(5) PrivateDevices=: no physical devices |
| `sandbox.syscall-architectures` | advisory | `syscall-architectures-unset` | systemd.exec(5): 'recommended to combine this option with SystemCallArchitectures=native' |
| `sandbox.kernel-tunables` | advisory | `protect-kernel-tunables-off` | systemd.exec(5) ProtectKernelTunables=: 'recommended to turn this on for most services' |
| `sandbox.kernel-modules` | advisory | `protect-kernel-modules-off` | systemd.exec(5) ProtectKernelModules=: 'recommended to turn this on for most services' |
| `sandbox.kernel-logs` | advisory | `protect-kernel-logs-off` | systemd.exec(5) ProtectKernelLogs=: 'recommended to turn this on for most services' |
| `sandbox.control-groups` | advisory | `protect-control-groups-off` | systemd.exec(5) ProtectControlGroups=: 'recommended to turn this on for most services' |
| `sandbox.clock` | advisory | `protect-clock-off` | systemd.exec(5) ProtectClock=: 'recommended to turn this on for most services' |
| `sandbox.hostname` | advisory | `protect-hostname-off` | systemd.exec(5) ProtectHostname=; systemd-analyze security scores it |
| `sandbox.namespaces` | advisory | `restrict-namespaces-off` | systemd.exec(5) RestrictNamespaces=; systemd-analyze security scores each namespace |
| `sandbox.realtime` | advisory | `restrict-realtime-off` | systemd.exec(5) RestrictRealtime=: realtime scheduling can starve a small host |
| `sandbox.suid-sgid` | advisory | `restrict-suid-sgid-off` | systemd.exec(5) RestrictSUIDSGID=: 'recommended to restrict creation of SUID/SGID files' |
| `sandbox.personality` | advisory | `lock-personality-off` | systemd.exec(5) LockPersonality=; systemd-analyze security scores it |
| `sandbox.write-execute` | advisory | `memory-deny-write-execute-off` | systemd.exec(5) MemoryDenyWriteExecute= (waive it for a JIT runtime) |
| `sandbox.proc` | advisory | `protect-proc-off` | systemd.exec(5) ProtectProc=invisible; systemd-analyze security scores it |
| `sandbox.proc-subset` | advisory | `proc-subset-off` | systemd.exec(5) ProcSubset=pid; systemd-analyze security scores it |
| `sandbox.umask` | advisory | `umask-permissive` | systemd.exec(5) UMask= 'Defaults to 0022': world-readable files; systemd-analyze security scores it |
| `sandbox.protect-system-strict` | advisory | `protect-system-not-strict` | systemd.exec(5) ProtectSystem=strict: the whole hierarchy read-only but ReadWritePaths= |
| `sandbox.protect-home-all` | advisory | `protect-home-unset` | systemd.exec(5) ProtectHome=: recommended for every service that needs no user data |
| `sandbox.syscall-filter-all` | advisory | `syscall-filter-unset` | systemd.exec(5) SystemCallFilter=@system-service: 'a relatively safe basic choice for the majority of system services' |
| `sandbox.capabilities` | advisory | `capability-bounding-set-unset` | systemd.exec(5) CapabilityBoundingSet=: 'If this option is not used ... no limits on the capabilities of the process are enforced' |
| `sandbox.ambient-capabilities` | advisory | `ambient-capabilities-set` | systemd.exec(5) AmbientCapabilities=: every capability a non-root service holds is one to justify |
| `sandbox.address-families` | advisory | `address-families-unrestricted` | systemd.exec(5) RestrictAddressFamilies=: 'limit exposure of processes to remote access, in particular via exotic and sensitive network protocols' |
| `sandbox.state-directory` | advisory | `state-directory-preferred` | systemd.exec(5) StateDirectory=/LogsDirectory=: 'Using these options is recommended' over hand-made writable paths |

### Stage `resources`: 8 rows (2 blocking, 6 advisory)

resources — ceilings on a small shared host.

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `resources.memory-order` | blocking | `memory-high-not-below-max` | systemd.resource-control(5): 'use MemoryHigh= as the main control mechanism and use MemoryMax= as the last line of defense'; a MemoryHigh= at or above MemoryMax= never throttles |
| `resources.budget` | blocking | `memory-budget-exceeded` | deploy/host-budget.json is the subject's own declared share of a shared host; every ceiling must fit it, or the kernel's global OOM killer decides instead of each unit's own |
| `resources.memory-max` | advisory | `memory-max-missing` | systemd.resource-control(5) MemoryMax=: the last line of defense, inside the unit's cgroup; a scheduled job's spike needs one too (i345: a daily render job with none) |
| `resources.memory-high` | advisory | `memory-high-missing` | systemd.resource-control(5) MemoryHigh=: 'This is the main mechanism to control memory usage of a unit' |
| `resources.tasks-max` | advisory | `tasks-max-missing` | systemd.resource-control(5) TasksMax=: a bounded process count (fork bombs, thread leaks) |
| `resources.cpu-quota` | advisory | `cpu-quota-missing` | systemd.resource-control(5) CPUQuota=: one daemon cannot take a 2-vCPU host from the stacks it shares |
| `resources.batch-priority` | advisory | `batch-priority-missing` | systemd.exec(5) Nice=, CPUSchedulingPolicy=, IOSchedulingClass=: a scheduled job yields to the daemons it shares a small host with |
| `resources.host-budget` | advisory | `host-budget-undeclared` | a shared small host needs its share written down: deploy/host-budget.json |

### Stage `secrets`: 5 rows (3 blocking, 2 advisory)

secrets — never in a unit, an env file or a replica config.

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `secrets.environment-literal` | blocking | `secret-in-environment` | systemd.exec(5) Environment=: 'environment variables are not suitable for passing secrets ... Use LoadCredential='; a literal in a committed unit is a leaked secret |
| `secrets.env-file` | blocking | `secret-in-env-file` | a committed env file under deploy/ holds only placeholders for secret-named keys (the credential lives in the secret store) |
| `secrets.litestream-credential` | blocking | `litestream-credential-literal` | Litestream reference/config: credentials by ${VAR} expansion or ambient identity, never a literal in the committed file |
| `secrets.environment-file-optional` | advisory | `environment-file-optional` | systemd.exec(5) EnvironmentFile=-: a missing file is skipped silently, so the service starts without its configuration |
| `secrets.credentials` | advisory | `secrets-via-environment` | systemd.exec(5): 'Use LoadCredential=, LoadCredentialEncrypted= or SetCredentialEncrypted= ... to pass data to unit processes securely' |

### Stage `logging`: 3 rows (0 blocking, 3 advisory)

logging — journald.

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `logging.journal` | advisory | `output-bypasses-journal` | systemd.exec(5) StandardOutput=file:/append:/truncate: bypasses journald's rotation and rate limits |
| `logging.identifier` | advisory | `syslog-identifier-missing` | systemd.exec(5) SyslogIdentifier=: one stable tag to filter a daemon's journal by |
| `logging.journal-cap` | advisory | `journal-size-uncapped` | journald.conf(5) SystemMaxUse= defaults to 10% of the file system (up to 4G); a small disk wants a drop-in |

### Stage `timers`: 14 rows (7 blocking, 7 advisory)

timers — calendar, catch-up, spread.

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `timers.section` | blocking | `timer-section-missing` | systemd.timer(5): 'Timer unit files must include a [Timer] section' |
| `timers.trigger` | blocking | `timer-no-trigger` | systemd-analyze verify: 'Timer unit lacks value setting. Refusing.' |
| `timers.target` | blocking | `timer-target-missing` | systemd.timer(5) Unit=: 'defaults to a service that has the same name as the timer'; a target the subject does not ship activates nothing |
| `timers.calendar` | blocking | `calendar-invalid` | systemd.time(7) CALENDAR EVENTS; systemd 255 ignores what it cannot parse ('Failed to parse calendar specification, ignoring') |
| `timers.persistent` | blocking | `persistent-without-calendar` | systemd.timer(5) Persistent=: 'this setting only has an effect on timers configured with OnCalendar=' |
| `timers.remain` | blocking | `timer-target-remains` | systemd.timer(5): a RemainAfterExit=yes service is 'only activated once, and then stay around forever' |
| `timers.installable` | blocking | `timer-install-missing` | systemctl(1) enable reads [Install]; a timer with no WantedBy= (timers.target) never starts |
| `timers.catch-up` | advisory | `calendar-not-persistent` | systemd.timer(5) Persistent=: 'useful to catch up on missed runs of the service when the system was powered down' |
| `timers.timezone` | advisory | `calendar-timezone-implicit` | systemd.time(7): a calendar event names UTC or an IANA zone, or follows whatever zone the host is set to |
| `timers.spread` | advisory | `randomized-delay-missing` | systemd.timer(5) RandomizedDelaySec=: 'prevent them from firing all at the same time, possibly resulting in resource congestion' |
| `timers.accuracy` | advisory | `accuracy-coarse` | systemd.timer(5): 'set AccuracySec=1us and RandomizedDelaySec= to some higher value' (AccuracySec defaults to 1min) |
| `timers.target-oneshot` | advisory | `timer-target-long-running` | systemd.timer(5): an active unit 'is not restarted, but simply left running'; a scheduled job is Type=oneshot |
| `timers.target-enabled` | advisory | `timer-target-enabled` | a timer-activated service with its own [Install] WantedBy= also runs at boot |
| `timers.name` | advisory | `timer-unit-name-mismatch` | systemd.timer(5) Unit=: 'recommended that the unit name that is activated and the unit name of the timer unit are named identically' |

### Stage `backup`: 9 rows (5 blocking, 4 advisory)

backup — 3-2-1, a restore drill, Litestream.

| id | severity | reason | why (primary source) |
|---|---|---|---|
| `backup.copies` | blocking | `backup-copies-short` | CISA/US-CERT Data Backup Options (3-2-1): 'Keep 3 copies of any important file: 1 primary and 2 backups' -- here a Litestream replica and a scheduled *-backup job |
| `backup.offsite` | blocking | `backup-not-offsite` | CISA/US-CERT Data Backup Options (3-2-1): 'Store 1 copy offsite' |
| `backup.restore-drill` | blocking | `restore-drill-missing` | CISA #StopRansomware Guide: 'regularly test the availability and integrity of backups'; Litestream docs: test restore with litestream restore -o |
| `backup.litestream-replica` | blocking | `litestream-no-replica` | Litestream reference/config: every dbs entry names its replica |
| `backup.litestream-legacy` | blocking | `litestream-replicas-deprecated` | Litestream v0.5 migration: 'Transition from the deprecated replicas array to the current single replica field' |
| `backup.litestream-unit` | advisory | `litestream-unit-missing` | Litestream guides/systemd: run replication as a systemd service so it restarts with the host |
| `backup.drill-integrity` | advisory | `restore-drill-integrity-unchecked` | Litestream docs: after litestream restore, check the copy with PRAGMA integrity_check |
| `backup.litestream-snapshot` | advisory | `litestream-snapshot-default` | Litestream reference/config snapshot: interval and retention default to 24h; state the restore chain you want |
| `backup.litestream-validation` | advisory | `litestream-validation-off` | Litestream reference/config validation: 'periodic integrity checks for LTX files' |

## The live rows: 22 phoenix-only probes

Every row runs `scripts/durable-service-probe.py check --job <job> --root PATH --format json`,
scope `live`, severity `block`.

- 20 are phoenix-v2 cron services, derived from `ops/cron-jobs.tsv` and `ops/crontab.txt` on four
  citation surfaces (SPEC-V2-2155 R1). `durable-service-probe.py list` prints them.
- 2 are the aol hook deployer and the re-anchor leak watch.

Each phoenix-v2 row runs four sub-checks. Each reads the beat and the log where `ops/cron-wrap.sh`
writes them: the wrapper's `PHX_CRON_STATE` or `PHX_CRON_LOG_DIR` override, else the default it
spells (SPEC-V2-2195).
- `alive`: the beat is younger than `max_age_secs`.
- `succeeds`: the dead-man's own rule over the same beat. The last run did not time out or exit
  non-zero; `rc=-` is a run in progress.
- `speaks`: a non-empty summary line between the wrapper's start and end markers.
- `installed`: the `ops/crontab.txt` line matches the installed crontab.

The two aol rows run their own checks (SPEC-V2-2155 R4):
- `aol-hook-deploy`: `heartbeat`, `deployer_not_stale`, `check_settings`;
- `aol-reanchor-watch`: `heartbeat`, `no_open_episode`.

A root with no `ops/crontab.txt` holds no population and is refused by name, exit 2.

| id | max_age_secs | SPEC | surface |
|---|---|---|---|
| orch-watch | 900 | SPEC-V2-2140 | 1 |
| improvement-metrics | 25200 | SPEC-V2-2141 | 1 |
| model-prices | 90000 | SPEC-V2-2150 R6 | 1 |
| train-service | 2700 | SPEC-V2-2148 | 1 |
| dispatch | 900 | SPEC-V2-2149 | 2 (`ops/dispatch.py` header) |
| user-units-guard | 720 | SPEC-V2-2072 R9 | 3 (manifest-mandated) |
| tracking | 1800 | SPEC-V2-2152 | 1 |
| forge-observe | 1800 | SPEC-V2-2053 LP9 | 1 |
| land-watch | 900 | SPEC-V2-2159 | 1 |
| improvement-cycle | 3600 | SPEC-V2-2151 R-AY | 1 |
| pack-probe | 2700 | SPEC-V2-2161 R4 | 1 |
| ledger-backup | 5400 | SPEC-V2-2157 | 1 |
| formal-soak | 3600 | SPEC-V2-2025 O5 / R-AO(3) | 1 |
| crontab-drift | 5400 | SPEC-V2-2107 i1837 | 2 (`ops/install-crontab.sh` header) |
| loop-watchdog | 1800 | SPEC-V2-195 R8.1 | 2 (`ops/loop-watchdog.sh` header) |
| cron-dead-man | 1200 | SPEC-V2-2107 D3 / SPEC-V2-716 R2 | 4 (rowless, self-citing) |
| budget-governor | 900 | SPEC-V2-2154 | 1 |
| ci-economy | 11700 | SPEC-V2-2169 | 1 |
| context-economy | 3600 | SPEC-V2-2167 | 1 |
| packager | 4200 | SPEC-V2-2156 | 1 |
| aol-hook-deploy | 1800 | SPEC-AOL-HOOK-008 | R4 (bespoke) |
| aol-reanchor-watch | 1800 | SPEC-AOL-HOOK-009 | R4 (bespoke) |

A job with no SPEC citation on any surface is box maintenance, not a durable service, and is left
out: 23 of them as of SPEC-V2-2155, which names each. (The earlier body's `list --excluded` flag
never existed; `list` prints the population only.) The self-hosted CI runner (d2174) is a systemd
service, not a cron job, and sits outside this population by construction (i1793).

## Measured on real subjects (2026-09-27)

- **The DeckStreak-shaped fixture** (`scripts/tests/fixtures/durable-unit-lint/deckstreak`):
  - all 92 rows are green; one advisory is waived with its why (Litestream sends no
    `WATCHDOG=1`);
  - `systemd-analyze verify` finds nothing;
  - `systemd-analyze security --offline=yes` scores each of its services 1.5 OK.
- **v9's own `deploy/`**, the tree DeckStreak ports from:
  - 91 of 92 rows hold. The one blocking finding is no `SystemCallFilter=` allow list on its two
    daemons;
  - 39 advisory reasons, among them `memory-max-missing` on the daily render job (i345),
    `calendar-timezone-implicit` on all three timers, and an unhardened restore drill.
- **phoenix's own user units** (`ops/systemd/`, read as user units): no blocking finding, 12
  advisory reasons. One of them: `phx-subscription-proxy.service`'s default start limit can never
  trip (5 × 5 s > 10 s).
- **The live rows on the box**: before SPEC-V2-2195, 20 of 22 were red for want of the wrapper's
  directories. After it, through `phxd`, 20 of 22 are green. The two reds are true:
  - `loop-watchdog` writes its summary to its own log, not between the cron-wrap markers (i1310);
  - `model-prices` has not run since it was installed.

## What this pack does not do

- It never starts, stops, restarts, installs or configures a unit, a timer or a cron job. The live
  rows read state already on disk, and the tree rows read files.
- It does not judge a host: env-file modes on disk, a timer's last trigger, a replica's freshness
  and a unit's active state are the box's own facts. The SPEC's coverage matrix names each one with
  the idea it belongs to.
- It does not judge application code: `READY=1` placement, the watchdog ping interval, SQLite's
  `busy_timeout` or `VACUUM INTO`. Those belong to the code's own packs.
- It does not decide dispatch order, budget caps or train cadence. Those rules live in the seats
  that own them.
