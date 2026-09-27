# Schematic: deployment on the host

Kind: component (deployment). Read at DeckStreak `main` 769ee62 (ADR-007, ADR-010, ADR-015). Every
concrete host name, address and secret name is private configuration; the units are templates in
`deploy/`.

```mermaid
flowchart TB
  internet((internet)) -->|443 TLS, 80 redirect| caddy[Caddy, shared with a co-hosted service]
  subgraph host[the VM]
    caddy -->|/api/* to loopback| apiUnit[deck-streak-api.service]
    caddy -->|static SPA with fallback| web[(release/current/web)]
    botUnit[deck-streak-bot.service] -->|long polling or webhook, outbound| tg((Telegram))
    timers[deck-streak-*.timer] --> jobs[deck-streak-job@.service oneshots]
    apiUnit & botUnit & jobs --> db[(state/deck_streak.db)]
    db --> litestream[litestream.service]
    jobs -->|runner env: device key, base URL| runner[headless agent run]
    runner -->|loopback port| tunnelEnd[reverse tunnel endpoint]
    apiUnit & botUnit & jobs -. OnFailure .-> alert[deck-streak-alert@.service]
    watch[deck-streak-memory-watch.timer] -. memory.events .-> alert
    predecessor[predecessor's units, until cutover]
  end
  tunnelEnd <-->|reverse SSH, opened from the maintainer's machine| proxy[subscription proxy on the maintainer's machine]
  litestream -->|replica| bucket[(offsite bucket, owner-approved)]
  alert -->|sendMessage| tg
```

| unit | type | binds | reads secrets from |
|---|---|---|---|
| `deck-streak-api.service` | notify, watchdog | loopback only | `LoadCredentialEncrypted=` |
| `deck-streak-bot.service` | notify, watchdog | none (outbound) | `LoadCredentialEncrypted=` |
| `deck-streak-job@<name>.service` + `.timer` | oneshot | none | `LoadCredentialEncrypted=` |
| `deck-streak-alert@.service` | oneshot | none | `LoadCredentialEncrypted=` |
| `deck-streak-memory-watch.service` + `.timer` | oneshot | none | none |
| the reverse tunnel | a user unit on the maintainer's machine | the host's loopback | the maintainer's key, never on the host |

Releases live side by side under `releases/<tag>/`, and `current` switches by an atomic rename.
