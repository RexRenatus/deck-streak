# Schematic: the owner's sync request, the doorbell and the job

Kind: sequence and component. Decided by ADR-066; built by SPEC-059.

## 1. The sequence

```mermaid
sequenceDiagram
  participant O as owner
  participant B as bot (SyncRequester)
  participant S as store (ingest_state, sync_runs)
  participant P as deck-streak-job@sync (path unit)
  participant J as deck-streak-job@sync (service unit)
  O->>B: /sync
  B->>S: request_rescore(now)
  B->>B: touch request file (at most once per 15 s)
  P-->>J: PathChanged starts the job
  J->>S: load: rescore_pending?
  J->>S: owner cycle (Trigger::Owner, 300 s reuse window)
  J->>S: scheduled run (claimed once per study day)
  B->>S: poll every 2 s, at most 120 s
  B->>O: outcome, or "still running"
```

## 2. The components

The request directory belongs to the service user, mode 0700, and only the bot unit may write
it. The path unit loads no credential. The job unit sees the directory read-only.

## 3. The states of one request

`requested` (flag set) -> `started` (path unit fired) -> `done` (flag cleared, a run row or a
reuse) or `pending` (bound reached: the owner is told it is still running).
