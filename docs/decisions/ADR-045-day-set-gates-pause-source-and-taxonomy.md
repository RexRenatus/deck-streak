---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The day set's gates: the last sync first, then the pause computed by the readings from ingest's reviews, from a private taxonomy, into closed states

## Context and Problem Statement

ADR-019 recorded the owner's reading decisions: a reading generates whenever the last sync
succeeded, and after two or more days without study the daily readings pause. It did not say in
which order the gates run, who computes the pause, where the owner's deck taxonomy lives in a
public repository, or how a topic's end state is held so that nothing to read, could not tell and
failed can never blur. The predecessor answered the first by copying the whole collection before it
refused, and the others with deck constants in code.

## Decision Drivers

- Honest states: no new cards, could not tell and failed stay distinct, and a collapse is a test failure.
- The repository is public; deck names and the subjects run are private (constraint 11).
- The readings context depends on the kernel and ingest only; the governor lives in streaks.
- No work, and no collection copy, on a night that is refused anyway.

## Considered Options (the alternatives it was chosen against)

- Gates in the order last sync, then pause, then resolve; the pause computed by the readings from ingest's reviews; the taxonomy in a private file with the predecessor's parse rule as the public algorithm; the end state as one enum whose variants carry their class and reason — chosen: cheap refusals first, no new edge, no private name in the tree, and a type that cannot hold two states.
- Keep a freshness gate on the collection file's age — rejected by the owner (ADR-019): it refuses whenever the file has not changed, as it does after a day without study, although the last sync succeeded.
- Take the pause from the governor's state — rejected because it needs a readings-to-streaks edge the context map does not declare, the governor knows three silent days and not two, and the pause (two days) must begin before the lapse (three days) does.
- Keep the predecessor's deck constants in code — rejected because they are the owner's private deck names and subject list.
- Record could-not-tell as a failure with a reason — rejected because it collapses a broken rail into a failed generation, which the owner's honest-states rule forbids.
- A second hand-written SHA-256 for the digest, beside the vault's — rejected at acceptance because `sha2` is already in the workspace, identity's (ADR-006, on ADR-024's 0.11 line), and this record admits it to readings; a second implementation is a second place for the digest to go wrong.
- A synthetic collection built for A1 by raw SQL — rejected at acceptance because a deck's row holds the engine's protobuf and a collation only the engine registers; ingest's own builder, which drives the engine, builds it instead.
- The queue's adapter in coordination — rejected at acceptance because A1 could then prove the resolution only through a copy of it; the adapter uses ingest's engine port and the kernel's offload alone, so it lives in readings beside the rules, and coordination composes.

## Decision Outcome

Chosen option. The resolution refuses before any collection work when the last sync failed
(`could_not_tell`, `rail_broken`, `sync_failed`) or when the two study days before today have no
qualifying review (`paused`). Only then does it query the scheduler's queue per root. The taxonomy
file names the law roots and bands, the language decks and their term fields, and the writing roots;
the predecessor's parse rule reads it and is proved by goldens generated through a synthetic
adapter. The six states and their closed reasons are one Rust enum stored with its class and reason.

At acceptance:
- **The last sync** is ingest's sync record's last run: `ok` or `skipped` succeeded; `error`, or no
  run at all, did not.
- **Before the pause,** a missing taxonomy (`config_fault`, `taxonomy_missing`) and a private copy
  the read cannot open (`rail_broken`) refuse the whole run, because without them no topic exists to
  pause. Such a run records its refusal and no topic.
- **The pause's reviews,** the resolution's deck names and each queued card's decks and note come
  from one read-only read of the private copy by ingest, made before the gates. The copy the queue
  needs, because the engine selects a deck to answer it, is the collection work the gates hold back:
  a throwaway copy of the private copy, taken under the shared collection lock, queried on the
  offload within the 30-second budget, and removed. The read comes before the last-sync gate, chosen
  against reading after it, because a night whose sync failed would then record no topic, and
  SPEC-045 R5 ends every topic of the day in one state. The read copies nothing and asks no engine,
  so the driver "no work, and no collection copy, on a night that is refused anyway" holds for the
  copy and the engine, and SPEC-045 R4 now says its gates run after the read and before any
  collection copy or engine query.
- **Attribution:** ingest's queue answers card ids, which are paired with the read's cards, and a
  card the read did not return goes to deck 0, so it is reported unmapped. Chosen against ingest's
  queue answering each card's decks and note, a new engine call beside SPEC-023's port, which is used
  as it stands, and against dropping an unread card, which would hide a queued card.
- **The copy's lifecycle** is one blocking closure on the offload that owns the shared lock: it takes
  the copy, releases the lock once the copy is whole, queries the engine, and removes the copy by a
  guard that neither an early return nor a panic skips. Chosen against the split lifecycle (the copy
  in one offload call, its removal in a later one), because the budget cancels by dropping the
  future (`tokio::time::timeout`: "Otherwise, an error is returned and the future is canceled"), and
  tokio stops neither half of the work: "`spawn_blocking` tasks cannot be aborted once they start
  running", and "If a `JoinHandle` is dropped, then the task continues running in the background and
  its return value is lost" (tokio 1.53, `task::spawn_blocking` and `task::JoinHandle`). Split, a
  copy the budget passed ran on detached with nothing left to remove it, and the lock was released
  as the future dropped, so the rest of the copy ran outside it. One closure turns the budget into a
  bound on the wait, never on the work.
- **The states** are six, `ai_route_absent` among them (ADR-054). `failed` carries the closed reasons
  SPEC-046 names, so the table's checks hold every state from its first migration. Chosen against
  SPEC-046 adding its reasons in its own migration, because SQLite cannot alter a CHECK in place and
  changing the list would rebuild `reading_topic_days`. The gate "no list markers" is spelled
  `no_list_markers` here, and SPEC-046 inherits the spelling.
- **Two dependencies.** `sha2` entered the workspace for identity: ADR-006 admits it for `initData`'s
  HMAC, and ADR-024 holds it on the 0.11 line. This record admits it to readings, where it computes
  the day-set digest. Readings' tests take `anki` (ADR-022) as a dev-dependency to build A1's and
  A5's collection with ingest's own synthetic builder, included by path; readings' code names no
  engine type. The include was chosen against ingest exporting its builder behind a test-support
  feature, which would need no `anki` dev-dependency in readings, because that adds a module and a
  feature to another context's library, outside SPEC-045's manifest, and cargo unifies the feature
  into the one ingest library a workspace test build compiles, so ingest's own tests would run
  against a library carrying test code.

### Consequences

- Good, because a paused or unsynced night costs a query of the sync ledger and of the reviews, not a
  collection copy.
- Good, because the pause and the lapse stay two concepts with two names, as the context map says.
- Bad, because the readings and the governor each count silent days; the counts differ by design
  (two and three), and each is tested against its own rule.

### Confirmation

SPEC-045's tests, the goldens of `prereading.py:resolve_day_sets`, `leeches.py:_law_subject` and
`prereading.py:_digest_for_card_ids`, the public scrub over the example taxonomy, and the hand-proved
rows of the gates' order, the pause, the saturation threshold, the digest's sort, the slug's refusal,
the reason-to-class map and the copy's removal past the budget (S04513, killed by
`a_budget_passed_during_the_copy_leaves_no_copy_behind`) in
`scripts/mutation-rows.d/S04500-S04599.json`.

## What would make this wrong

- The owner redefines the pause in terms of the governor's lapse (then the pause reads the governor
  through coordination).
- A deck layout the parse rule cannot express appears in the owner's collection (then the taxonomy
  learns an explicit deck-to-topic map).
- Two ports share one scratch directory, in one process or two (then the throwaway's name carries
  the process id as well as the call, and a sweep at start removes a dead process's copies: today a
  port's first call reuses the name `-0`, so it overwrites and removes a copy a killed run left).

## More Information

SPEC-045; ADR-019; docs/CONTEXT-MAP.md "Overloaded words"; docs/schematics/reading-lifecycle.md.
