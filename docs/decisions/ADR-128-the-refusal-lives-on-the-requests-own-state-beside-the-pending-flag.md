---
status: "accepted"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The refusal lives on the request's own state, beside the pending flag

## Context and Problem Statement

The sync job refuses the owner's request in two places (the recompute's setup cannot load, or the
owner cycle returns a refusal) and only logs it (SPEC-059, issue #323). The flag stays set, so
the owner is told the sync is still running and every job run refuses the request again. Where
does a refusal live so that the owner's answer can read it and the job stops retrying?

## Decision Drivers

- The owner's answer already reads `ingest_state.rescore_pending` and the latest owner run.
- SPEC-022 R9 fixes the closed set of a sync's failure codes, and the scheduled sync uses it too.
- A refusal is not a sync: it has no attempts, no download and no start instant of a sync.
- SQLite cannot widen a `CHECK` on an exported table without rebuilding it.

## Considered Options (the alternatives it was chosen against)

- Two columns on `ingest_state`, the reason and the instant, cleared by a new request and by R6's erase, and by nothing else. Chosen because the record sits beside the flag it explains, one write sets both, and `ADD COLUMN` with a `CHECK` needs no rebuild.
- A `sync_runs` row with SPEC-022 R9's closed set widened. Rejected because a refusal is not a sync (its `attempts`, `full_download` and start instant mean nothing), widening a `CHECK` needs a rebuild of an exported table, and R9's set is the scheduled sync's too.
- Keep logging only, as today. Rejected because the owner is told the sync is still running and every job run refuses the same request again, which is the issue.
- A new table of refused requests. Rejected because only the latest request is ever answered, and a table adds a data-rights row and a census for one value.

## Decision Outcome

Chosen option: "two columns on `ingest_state`", because the request's state then reads in one
row: pending, refused (reason and instant) or served.

### Decision 2 (fix round 1): a refusal after the owner's run is answered beside the run

A refusal recorded at or after the request, by the same cycle, after the owner's run row (the codes
`obligations_unreadable` and `recompute_failed`) is answered beside the run's outcome. Line 1 is the
run's own sync line, Synced or Failed, unchanged. Line 2 is "Your scores were not recomputed
(CODE), so they stand." A refusal with no run since the request keeps "No sync ran (CODE)."
Every request and every owner cycle clears the record first, so a refusal beside an owner run was
recorded after that run. The type change is `Progress::RefusedAfterRun` in the daemon and
`Scores::Refused { reason }` in the bot; the reply goes through the bot's `send_html`, not the router.

- Answering it as "No sync ran". Rejected because it is false: a sync ran and refreshed the copy.
- Answering it as "Ran". Rejected because it hides the refusal, which is the defect this issue names.
- Answering it as the sync "Failed". Rejected because it is false: the copy was refreshed.

### Consequences

- Good, because the refusal and the cleared flag are one write, so no run sees one without the other.
- Good, because the row is already exported and reset in place, so the data-rights port grows two columns.
- Bad, because only the latest refusal is kept; the log holds the history.
- Bad, because a transient database error inside the cycle also clears the flag, so the owner sends `/sync` again.

### Confirmation

SPEC-128 A1 to A13; the band `S12800-S12899`.

## More Information

Issue #323; SPEC-128; SPEC-059 section 6; ADR-066; SPEC-022 R9.
