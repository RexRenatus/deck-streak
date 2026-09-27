# Schematic: from a failing unit or a burning budget to one Telegram page

Kind: data flow. Read at DeckStreak `main` e05dfa5 (ADR-010, `docs/schematics/deployment.md`, the
observability pack's `slo.template.json`, `alert@.template.service`, `alert-telegram.template.sh`,
`memory-watch.template.sh`). Decided by ADR-031; built by SPEC-031 over SPEC-032's units.

```mermaid
flowchart LR
  api[deck-streak-api.service] -->|TraceLayer response event at INFO: route, status, latency| journal[(journald)]
  slotimer[deck-streak-slo.timer, every 5 min] --> eval[slo-evaluate.py: long and short windows per alert]
  journal --> eval
  eval -->|both windows past the burn, first time this episode| fail1[exit 1]
  mwtimer[deck-streak-memory-watch.timer, every minute] --> watch[memory-watch.sh: memory.events of every unit]
  watch -->|new oom_kill or max| fail2[exit 1]
  watch -->|new high, or above 90 percent of max| journal
  units[api, bot, job units: exit non-zero] --> fail3[unit failed]
  fail1 & fail2 & fail3 -->|OnFailure=deck-streak-alert@%n.service| alert[alert-telegram.sh]
  creds[$CREDENTIALS_DIRECTORY: telegram-bot-token, owner-user-id] --> alert
  alert -->|URL and body through curl --config on stdin| tg((Telegram sendMessage))
```

| page source | fires when | pages how often |
|---|---|---|
| a unit's own failure | the process exits non-zero, a watchdog timeout, an OOM kill | once per failure; a crash loop ends at the start limit |
| the SLO evaluator | both windows of an alert exceed its burn rate | once per burn episode (`$STATE_DIRECTORY`) |
| the memory watch | a new `oom_kill` or `max` event in a unit's `memory.events` | once per event |
| a scheduled job's transition | SPEC-027's paging transitions | once per episode |
