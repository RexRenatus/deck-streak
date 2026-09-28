# Schematic: deployment on the host

Kind: component (deployment). Read at DeckStreak `main` 769ee62 (ADR-007, ADR-010, ADR-015); the
budget per unit is SPEC-032's (ADR-032). Every
concrete host name, address and secret name is private configuration; the units are templates in
`deploy/`. Credentials reach every unit from the private rail's credential socket at each start
(ADR-038), and the agent's path exists only when the owner enables the proxy route (ADR-054).

```mermaid
flowchart TB
  internet((internet)) -->|443 TLS, 80 redirect| caddy[Caddy, the host's reverse proxy]
  subgraph host[the VM]
    caddy -->|/api/* to loopback| apiUnit[deck-streak-api.service]
    caddy -->|static SPA with fallback| web[(release/current/web)]
    botUnit[deck-streak-bot.service] -->|long polling or webhook, outbound| tg((Telegram))
    timers["deck-streak-*.timer"] --> jobs["deck-streak-job@.service oneshots"]
    apiUnit & botUnit & jobs --> db[(state/deck_streak.db)]
    db --> litestream[litestream.service]
    socket[credential socket and fetch helper, the private rail's, root only] -->|LoadCredential at each start| apiUnit & botUnit & jobs & alert
    jobs -.->|only with the proxy route: device key as a credential, base URL| runner[headless agent run]
    runner -.->|loopback port| tunnelEnd[reverse tunnel endpoint]
    apiUnit & botUnit & jobs -. OnFailure .-> alert["deck-streak-alert@.service"]
    watch["deck-streak-memory-watch.timer"] -.->|memory.events| alert
    predecessor[predecessor's units, until cutover]
  end
  socket -->|the host's identity, each named secret only| secrets[(secret manager)]
  tunnelEnd <-->|only with the proxy route: reverse SSH, opened from the maintainer's machine| proxy[subscription proxy on the maintainer's machine]
  litestream -->|replica| bucket[(offsite bucket, owner-approved)]
  alert -->|sendMessage| tg
```

| unit | type | binds | reads secrets from | `MemoryHigh` / `MemoryMax` (ADR-032) |
|---|---|---|---|---|
| `deck-streak-api.service` | notify, watchdog | loopback only | `LoadCredential=` from the credential socket (ADR-038) | 96M / 128M |
| `deck-streak-bot.service` | notify, watchdog | none (outbound) | `LoadCredential=` from the credential socket (ADR-038) | 64M / 96M |
| `deck-streak-job@<id>.service` + `.timer` | oneshot | none | `LoadCredential=` from the credential socket (ADR-038) | 320M / 384M |
| `deck-streak-alert@.service` | oneshot | none | `LoadCredential=` from the credential socket (ADR-038) | 32M / 48M |
| `deck-streak-slo.service` + `.timer` | oneshot | none | none | 48M / 64M |
| `deck-streak-memory-watch.service` + `.timer` | oneshot | none | none | 32M / 48M |
| the credential socket and its fetch helper | the private rail's (#41), root only | the socket path | the secret manager, with the host's identity | the rail's |
| the reverse tunnel, only with the proxy route (ADR-054) | a user unit on the maintainer's machine | the host's loopback | the maintainer's key, never on the host | none on the host |

Releases live side by side under `releases/<tag>/`, and `current` switches by an atomic rename.

## The budget per unit (SPEC-032, ADR-032)

DeckStreak's share of the host is `"memory": "640M"` and `"cpus": 2`, in `deploy/host-budget.json`,
which records each unit's `memory_high` and `memory_max`; every unit's `MemoryHigh=` and
`MemoryMax=` equal its entry. The file holds the units SPEC-032 ships, and SPEC-031 adds the alert
unit, the SLO evaluator and the memory watch with the numbers above. The worst case the share must
hold is every long-running unit at its ceiling while the largest oneshot runs:

```mermaid
flowchart LR
  api["deck-streak-api.service: 128M"] --> daemons["long-running: 224M"]
  bot["deck-streak-bot.service: 96M"] --> daemons
  job["deck-streak-job@.service: 384M, the largest oneshot"] --> worst["worst case: 608M"]
  daemons --> worst
  worst -->|fits| share["the share: 640M"]
```

Each unit throttles at its own `MemoryHigh=` and is killed at its own `MemoryMax=`, so a job's
spike fails the job alone, and `OnFailure=` pages it through the alert unit.
