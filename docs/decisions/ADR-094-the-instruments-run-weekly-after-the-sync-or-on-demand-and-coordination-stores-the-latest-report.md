---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The instruments run weekly after the sync or on demand, one at a time, and coordination stores each one's latest report

## Context and Problem Statement

W4 ports about twenty research instruments (#137 to #151, #142, #144): read-only studies of the
collection, each with its own reads and one report. In the predecessor some run inside the weekly
report and some answer a command (`pipeline_layers/read_api.py:ReadApiLayer`, `bot.py:CommandBot`,
predecessor `27ee2bc`). The weekly report itself is W5 (#130), and several instruments the
predecessor never called are excluded as inert (#173 to #182). Each instrument reads more of the
collection than a sync's recompute does. When does an instrument run, which context runs it, and
where does its report live?

## Decision Drivers

- A full read is slow: instruments must not run inside every sync's recompute, and two must not
  read at once.
- The weekly report (#130) and the Mini App read reports without waiting on a read.
- An instrument's inputs come from several contexts (ingest's reads, analytics' rollups,
  curriculum's desired retention), and ADR-002 lets only coordination join them.
- An inert instrument must be revivable by the owner without a redesign (#173 to #182).

## Considered Options (the alternatives it was chosen against)

- A frame: insights defines pure instruments and one registry row each; coordination runs a weekly instrument after a sync's recompute when its report is 7 study days old, runs an on-demand one when asked, one at a time through the offload, and stores the latest report in `instrument_reports` — chosen: one read at a time, a report always ready, and reviving an inert instrument is one registry row.
- Every instrument inside each sync's recompute — rejected because it multiplies the sync's reads for studies that change weekly at most.
- Every instrument on demand with nothing stored — rejected because the weekly report and the Mini App would each wait on a full read.
- One table per instrument — rejected because each would owe six files for a report the frame never queries inside.
- Insights owning the table — rejected because the run, its inputs from other contexts and its failure record are the use case's, and insights stays a set of pure builds testable without a database.

## Decision Outcome

Chosen option: "A frame with a weekly and an on-demand cadence, and one latest report per
instrument in coordination", because it bounds the reads and keeps every report ready.

- An instrument has an id, a cadence, the reads it needs and a pure build to a serialisable report
  that carries its failed reads (`crates/insights/src/instrument.rs`).
- After each sync's recompute, coordination runs each weekly instrument whose report is absent or
  at least 7 study days older than the sync's study day; a failure of one is its report's failed
  read and never stops the next.
- An on-demand run answers a route or a command; a request while any instrument runs in the same role
  process is answered that a run is in progress and starts nothing; role processes each hold their own
  guard, and a run in another process may overlap under the collection lock's shared side.
- `instrument_reports` holds the latest report per instrument (id, study day, schema version, JSON),
  replaced in one write, in the `research-instruments` category.
- A registry row marked inert never runs; reviving it is that row.

### Consequences

- Good, because at most one instrument reads at a time, and never inside the recompute.
- Good, because W5's weekly report reads stored reports and sends them through the one router.
- Bad, because a weekly report is up to 7 study days old; its section shows its study day.
- Bad, because one latest report per instrument keeps no history; a trend over reports would need
  a new table.

### Confirmation

SPEC-094's frame criteria (the weekly rule, the in-progress answer, the store replaced in one
write, export and erase) and each later SPEC's registry rows.

## What would make this wrong

- An instrument's report must be exact when asked and cannot be weekly or on demand; it then moves
  into the fold (ADR-091).
- The owner wants a report's history; `instrument_reports` then gains a history table in a later
  SPEC.

## More Information

ADR-002; ADR-012; SPEC-020 (the offload); SPEC-094, which builds the frame; SPEC-095 to SPEC-098.
