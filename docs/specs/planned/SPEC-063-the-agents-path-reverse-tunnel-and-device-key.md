# SPEC-063: with the owner's route enabled, the agent reaches the proxy over a supervised reverse tunnel, runs capped under the pinned guards, and fails closed and says so when the path is down

- **Wave:** W2. **Issue:** #43 (epic #3). **Context(s):** `deploy` (`deploy/optional/ai-route/`),
  `agent` (the public runner's guard check); the private rail and the maintainer's machine (the
  tunnel).
- **Decided by:** ADR-015 (headless Claude Code on the host, a reverse SSH tunnel opened from the
  maintainer's machine, a dedicated device key, fail closed), ADR-054 (the route is optional and
  `Absent` by default), ADR-038 (the device key as a credential from the socket), ADR-043 (the
  shell runner and the caps), ADR-061 (the route lands as a drop-in), and this SPEC's ADR-063 (the
  agent's Claude Code comes from the vendor's signed package at one held version).
- **Waits for:** owner gate 3 (#162): the owner's choice of the subscription route over an API key,
  and the maintainer adding the key to the proxy's roster with the owner's go. Also gate 6 (#165) for the key's
  value and gate 2 (#161) for the host changes. It is CONDITIONAL: until gate 3 passes, nothing here
  is installed, and every night runs in no-AI mode (ADR-054).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-063.md` (ADR-016).

## 1. The problem, measured

- **The proxy is loopback-only on the maintainer's machine** (ADR-015), and the host must hold no
  credential to that machine. ADR-015 chose a reverse SSH tunnel opened from the maintainer's machine
  by a supervised user unit, so the proxy appears on the host's own loopback at the same port.
- **The runner is planned against fakes only.** SPEC-043 plans it and tests it only against a fake
  `claude` and a fake `curl` (ADR-043), and ADR-043 names this issue as its live proof (#43).
- **The route is `Absent` by default** (ADR-054): the first deploy (SPEC-062) has no key, no tunnel
  and no proxy. Enabling the route must not need a redeploy: SPEC-053 R9 already names what the rail
  adds, the route setting and the base URL in the environment file and the device key as a
  credential in a drop-in beside the readings unit's template.
- **The box-run packs judge the tunnel's unit.** They refuse a forward that binds anything but
  loopback, and advise `ExitOnForwardFailure` and a positive `ServerAliveInterval`. They run on the
  maintainer's box only (ADR-069), where the rail is.
- **A reverse forward outlives a dead client.** sshd frees a remote forward's listener only when it
  notices the session is gone, and its `ClientAliveInterval` defaults to 0, which never probes
  (sshd_config(5)). A reconnect with `ExitOnForwardFailure=yes` then fails until the old session
  ends.
- **sshd's Match blocks in an included file stay in that file.** OpenSSH restores the match state
  when an `Include` returns, and `sshd -t` checks syntax without evaluating a `Match`; `sshd -T -C`
  shows the effective settings for a given user (OpenSSH `servconf.c`, read through Context7).
- **The CLI is large.** One native build of Claude Code is about 234 MB on disk (234,082,616 and
  234,119,480 bytes for two builds, measured on the maintainer's machine). Its memory for one
  headless, tool-less run is unmeasured, so R10 measures it at gate 3 before the route stays on.
- **The subscription-proxy scanner's credential row** accepts only a secret-manager call inside the
  client, while the runner reads a systemd credential (SPEC-043's risk, ADR-043). That must be
  settled on the box before this lands.

## 2. Requirements

R1. The tunnel is a user unit on the maintainer's machine, kept in the private rail: `ssh -N` with
    one remote forward from the host's `127.0.0.1` to the proxy's loopback address and port,
    `ExitOnForwardFailure=yes`, `ServerAliveInterval=30`, `ServerAliveCountMax=3`, `BatchMode=yes`,
    the host's key pinned with `StrictHostKeyChecking=yes`, `Restart=always` and `RestartSec=10`. The
    box-run packs' tunnel rows judge it on the maintainer's box (ADR-069).
R2. On the host, a dedicated system account accepts the tunnel: no password, `nologin` as its
    shell, and one `authorized_keys` line restricted as
    `restrict,port-forwarding,permitlisten="127.0.0.1:<port>"`, in that order, because the options
    are read left to right and the last forwarding flag wins (OpenSSH `auth-options.c`; sshd(8)
    documents each option). One sshd drop-in holds a `Match User` block for that account alone:
    `AllowTcpForwarding remote`, `PermitListen 127.0.0.1:<port>`, `GatewayPorts no`,
    `PermitTTY no`, `X11Forwarding no`, `AllowAgentForwarding no`, `ClientAliveInterval 30` and
    `ClientAliveCountMax 3`. `sshd -t` passes and `sshd -T -C` shows those values for the account
    before sshd is reloaded.
R3. The host holds no key or credential to the maintainer's machine, and the forward listens on the
    host's loopback only (ADR-015).
R4. Claude Code reaches the host from the vendor's signed package repository, its signing key
    pinned by fingerprint, at one version held against upgrades, with the settings template's
    `DISABLE_AUTOUPDATER` (SPEC-043 R7); it is never bundled into DeckStreak's release and never
    installed by a piped script (ADR-063). The rail installs a new version only after the runner's
    tests have passed with that version on the maintainer's machine.
R5. The device key is the owner's: minted and stored by the owner (gate 6), added to the proxy's
    roster by the maintainer with the owner's go, which restarts the proxy (gate 3). No builder adds it, probes
    the proxy or restarts it. The rail then adds the key's row to the credential map and installs
    `deploy/optional/ai-route/` for the readings unit: the drop-in loads
    `LoadCredential=agent-device-key:/run/deck-streak-credentials/socket` (ADR-038), and the
    environment file gains the route setting and the loopback base URL (SPEC-053 R9). With the
    route, the rail also installs the output gate's probes at the version the box run judges, and
    the environment file names their path (SPEC-043 R11, ADR-069). The key reaches `claude` only as
    `CLAUDE_CODE_OAUTH_TOKEN` in its environment (SPEC-043 R2), never an `apiKeyHelper`, an argv, a
    file or a log line.
R6. Before its preflight, `agent/run-headless.sh` runs `deploy/scripts/guards-check.py` over the
    guards' manifest (SPEC-061 R8), whose path the unit's environment names. A missing manifest, a
    missing or changed guard, or a guard writable by anyone but root exits 2 with one `REFUSE:` line
    naming the file, never its content, and `claude` is not launched; the verdict is
    `refused_shape` (SPEC-043 R12).
R7. The drop-in gives the readings unit a runtime directory, and the runner points the CLI's `HOME`
    there, so the CLI's own files, its session transcript included, live only for the run and are
    removed when the unit stops.
R8. Every run carries SPEC-043's caps (by default 30 turns, 5 USD and 1800 seconds; 620 seconds for
    a daily reading), fails closed and says why; the caps are a per-run safety bound, never a daily
    cap on readings (SPEC-001 §9).
R9. Enabling and disabling need no redeploy. Enabling is R5's drop-in, map row and settings plus a
    systemd reload; the next generation uses the route. Disabling removes them; the route reads
    `Absent` again, and no alert follows (ADR-054).
R10. The gate-3 rehearsal records the readings unit's memory peak (its cgroup's `memory.peak`) for
    one capped run and the host's available memory during it. The drop-in's `MemoryHigh=` and
    `MemoryMax=` for the readings unit are set from that peak, and ADR-032's share is amended with
    the new figure at this SPEC's delivery; a share the host cannot hold is the owner's resize
    decision, with the numbers (ADR-011).
R11. The live proof: with the tunnel up, one capped run delivers one gated reading end to end; with
    the tunnel down, the next run ends `unavailable` with `proxy_unreachable`, raises one alert and
    writes nothing, no reading row and no vault byte (#43, #162).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the AI-route drop-in, merged with the readings unit's template by the units reader (`scripts/tests/_units.py`), sets only `LoadCredential=`, `RuntimeDirectory=`, `MemoryHigh=` and `MemoryMax=`, so it weakens none of the template's settings | `test_ai_route.py` |
| A2 | the drop-in's one credential is `agent-device-key` in the socket form, and nothing under `deploy/optional/ai-route/` sets an `apiKeyHelper`, `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` or a non-loopback base URL | `test_ai_route.py` |
| A3 | enabling the route adds exactly one credential pair, the readings unit's `agent-device-key`, and changes no other unit's pairs | `test_ai_route.py` |
| A4 | the runner refuses to launch when the guard check fails: exit 2, one `REFUSE:` line naming the file, and the fake `claude` records no call | `test_run_headless.py` |
| A5 | the runner points the CLI's `HOME` at the run's runtime directory | `test_run_headless.py` |
| A6 | no file of the AI route names a private value, and a planted one is refused by the public scrub | `test_ai_route.py`; `scripts/public-scrub.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ai_route.py -k test_the_ai_route_drop_in_sets_only_its_credential_runtime_and_ceilings
A2: python3 -m unittest discover -s scripts/tests -p test_ai_route.py -k test_the_ai_route_loads_only_the_device_key_from_the_socket
A3: python3 -m unittest discover -s scripts/tests -p test_ai_route.py -k test_enabling_the_route_adds_exactly_one_credential_pair
A4: python3 -m unittest discover -s agent/tests -p test_run_headless.py -k test_the_runner_refuses_to_launch_when_a_guard_fails
A5: python3 -m unittest discover -s agent/tests -p test_run_headless.py -k test_the_runner_points_the_cli_home_at_the_runtime_directory
A6: python3 -m unittest discover -s scripts/tests -p test_ai_route.py -k test_no_ai_route_file_names_a_private_value
```

A4 and A5 use SPEC-043's fakes and a guard manifest built in a `TemporaryDirectory`. The live proof
(R11) runs on the host at gate 3 and is recorded privately; no test here reaches a proxy. The
box-run packs judge the merged unit and the tunnel's unit as well (ADR-069); that verdict is the
delivery's evidence, not a criterion here.

## 4. The owner's gates and the evidence they record

The exact commands, the port, the account's name and the host's names are in the maintainer's
private gate packet; this SPEC names each step only. No step probes the proxy except the agent's own
preflight during the live proof, run by the maintainer with the owner's go.

| step | gate | what is approved | evidence recorded (privately) | rollback |
|---|---|---|---|---|
| E1 | 3 (#162) | the subscription route over an API key | the owner's recorded choice | none: an API key is a new adapter (ADR-054) |
| E2 | 6 (#165), 3 (#162) | the key minted and stored; the maintainer adds it to the roster | the secret's existence by name; the roster change and the proxy's restart, from the maintainer | the maintainer removes the key from the roster |
| E3 | 2 (#161) | the tunnel account and the sshd drop-in | `sshd -t`; `sshd -T -C` for the account; the reload | remove the key line, the drop-in and the account; reload sshd |
| E4 | 2 (#161) | the tunnel unit on the maintainer's machine | the tunnel rows green over the rail; the forward listening on the host's loopback only | stop and disable the unit |
| E5 | 2 (#161) | Claude Code from the signed repository at one held version | the package's version, its hold and the key's fingerprint; the disk after | remove the package and the repository |
| E6 | 2 (#161) | the guards at their pinned version | `guards-check.py` green over the manifest | remove the guards and the manifest |
| E7 | 3 (#162) | the map row, the drop-in and the settings | `credential-pairs.py --optional ai-route` equal to the map; `effective-check.py` | R9's disabling |
| E8 | 3 (#162) | the live proof | R11 both ways; `find / -xdev` finds no copy of the key; the apiKeyHelper scan over the host's settings; R10's memory peak | R9's disabling |

## 5. File manifest

| file | context | change |
|---|---|---|
| `deploy/optional/ai-route/deck-streak-readings-generate.conf` | deploy | added: the drop-in the rail installs only with the route |
| `deploy/optional/ai-route/ai-route.env.example` | deploy | added: the route setting and a loopback base URL, neutral values |
| `agent/run-headless.sh` | agent (public) | changed: the guard check before the preflight; the CLI's home |
| `agent/tests/test_run_headless.py` | agent (public) | changed: A4, A5 |
| `scripts/tests/test_ai_route.py` | repo | added: A1 to A3, A6 |
| `docs/decisions/ADR-032-deploy-templates-and-the-host-budget.md` | docs | changed: an appended amendment with the measured share |
| `deploy/README.md` | deploy | changed: the AI route's enabling and disabling |
| `docs/specs/SPEC-063-the-agents-path-reverse-tunnel-and-device-key.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-063-the-agents-claude-code-comes-from-the-signed-package-at-one-held-version.md` | docs | changed: status accepted |
| `docs/red-first/SPEC-063.md` | docs | added |
| `changelog.d/` fragment | repo | added |

The tunnel's unit, the host account's key line, the sshd drop-in, the package source, the guards and
their manifest, the output gate's probes, and the map's new row are private: named here, never
committed.

## 6. What this does NOT do

- It enables no route before the owner's choice at gate 3, and builds no API-key adapter (#162).
- It adds no key to the proxy's roster and restarts no proxy; the maintainer does, with the owner's
  go (#162).
- It runs no duty but the daily reading (#46, #53).
- Its forward listens on the host's loopback address only, and it adds nothing reachable from
  outside the host (#161).
- It changes no reading rule and no cap's default (#32).

## 7. Risks

- **A stale forward blocks the reconnect.** The account's `ClientAliveInterval` ends a dead session
  within about 90 seconds, and the tunnel unit retries every 10 seconds; until then a run ends
  `proxy_unreachable`, alerted once (R11).
- **The maintainer's machine is down at the generation's slot.** The run fails closed, alerts once,
  and the owner can regenerate on demand later (SPEC-048); the readings health shows it (SPEC-050).
- **The CLI does not fit DeckStreak's memory share.** R10 measures one capped run's peak before the
  route stays on; the share then changes only by an amendment of ADR-032, or the host by a resize
  the owner decides, with the numbers.
- **The key leaks through the process environment.** Only the unit's user and root can read its
  `/proc` environment; the runner sets `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` (SPEC-043 R2), and the
  daily reading holds no tool (SPEC-043 R8).
- **A CLI update changes behaviour.** The version is held and changes only through the rail after
  the runner's tests pass (R4).
- **The guards drift from their pinned version.** The runner's check refuses the launch (R6).
- **The scanner's credential row still refuses the runner.** SPEC-043's open risk; this SPEC does
  not land until the box run reads it green or a decision waives it.
