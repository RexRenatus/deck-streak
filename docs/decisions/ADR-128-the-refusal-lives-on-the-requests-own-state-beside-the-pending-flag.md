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

- Two columns on `ingest_state`, the reason and the instant, cleared by a new request and by nothing else. Chosen because the record sits beside the flag it explains, one write sets both, and `ADD COLUMN` with a `CHECK` needs no rebuild.
- A `sync_runs` row with SPEC-022 R9's closed set widened. Rejected because a refusal is not a sync (its `attempts`, `full_download` and start instant mean nothing), widening a `CHECK` needs a rebuild of an exported table, and R9's set is the scheduled sync's too.
- Keep logging only, as today. Rejected because the owner is told the sync is still running and every job run refuses the same request again, which is the issue.
- A new table of refused requests. Rejected because only the latest request is ever answered, and a table adds a data-rights row and a census for one value.

## Decision Outcome

Chosen option: "two columns on `ingest_state`", because the request's state then reads in one
row: pending, refused (reason and instant) or served.

### Consequences

- Good, because the refusal and the cleared flag are one write, so no run sees one without the other.
- Good, because the row is already exported and reset in place, so the data-rights port grows two columns.
- Bad, because only the latest refusal is kept; the log holds the history.
- Bad, because a transient database error inside the cycle also clears the flag, so the owner sends `/sync` again.

### Confirmation

SPEC-128 A1 to A9; the band `S12800-S12899`.

## More Information

Issue #323; SPEC-128; SPEC-059 section 6; ADR-066; SPEC-022 R9.
