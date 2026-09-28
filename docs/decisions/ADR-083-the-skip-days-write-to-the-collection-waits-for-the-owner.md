---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner, at #266), the DeckStreak architect"
---

# The skip day writes its reschedule to the collection, the owner's option (a), on a working copy with incremental syncs only

## Context and Problem Statement

The predecessor's skip day is its one write back to Anki.
`pipeline_layers/skip.py:SkipDaysLayer.take_skip_day` records a row and hands the write to
`sync.py:AnkiSyncer.skip_day`, which converges the collection with the sync server, snapshots every
affected card, reschedules today's due review cards one to three days out, with their intervals kept
when FSRS is off (`set_due_date` with the day spec of `skip.py:skip_spec`), and uploads the change,
all or nothing.
Its undo (`SkipDaysLayer.undo_skip_day`, `AnkiSyncer.undo_skip`) restores the snapshot and uploads
again (predecessor `27ee2bc`).

At gate 6 the owner approved reusing the owner's own Anki login on testable conditions, and the
first is that no upload path exists, proven against a fake sync server that records every request
(ADR-037; SPEC-022 R14). A skip day that reschedules cards needs exactly the path that condition
forbids. CHARTER constraint 4 says "The skip day is the ONLY write back to Anki".

This ADR laid out three options and left the choice to the owner at #266. The owner chose (a):
"Upload the skip day". It "writes the reschedule back to Anki, as the old app did". ADR-089 records
that decision, supersedes ADR-037's condition (a) for the skip-day path only, and states the
owner's guardrails and the undo's rules. Which option does DeckStreak build, and how does SPEC-083
make the write so that the guardrails hold by construction?

## Decision Drivers

- The owner's choice of (a) at #266, and its guardrails (i) to (v) and the undo's rules (ADR-089).
- The owner's collection stays untouchable by construction on every path but the skip day's.
- The private copy stays SPEC-022's: its syncer never sends a local change (R6, R14).
- A skip day's effects on DeckStreak's own game (the streaks bridged, the consistency run held, the
  day's quests voided, its chests paused, the tariff, the undo) work as SPEC-083 specifies.
- Parity: the predecessor's skip moved today's due review cards out of today's queue.

## Considered Options (the alternatives it was chosen against)

- (a) An upload path for the skip day alone, which converges, snapshots, reschedules today's due review cards with the predecessor's day spec and uploads — chosen by the owner at #266 (ADR-089), under the guardrails ADR-089 states.
- (b) No write: DeckStreak records the skip and applies every effect of it in its own database, and the owner reschedules the cards in Anki — rejected by the owner at #266: the owner chose the write, which "writes the reschedule back to Anki, as the old app did" (ADR-089).
- (c) A hand-off: option (b), with the confirm sheet and the bot showing the exact search and day spec to use in Anki's own Set Due Date, and the steps — rejected with (b), because it is (b)'s copy, and the owner still moves the cards by hand.
- (c2) Leave the write to the predecessor's own skip day during side by side, and decide at cutover — rejected, because side by side ends at cutover (#164) with the question still open, and two skip commands in two bots can record two different days.
- Write on the private copy and revert it in place when the push fails, as the predecessor does (`sync.py:AnkiSyncer._restore_cards`) — rejected because a revert in place leaves the reverted cards as local changes, which the next scheduled sync would send: an upload outside the skip path, against guardrail (i).
- Swap the pushed working copy in for the private copy — rejected because the private copy would gain a second writer beside SPEC-022's syncer, and nothing the skip needs reads the moved cards before the next sync downloads them.
- Resolve a full-sync demand inside the skip by a full download, as the predecessor's converge does (`sync.py:AnkiSyncer._converge`) — rejected by guardrail (ii): any full or one-way sync demand aborts the skip.
- Restore every snapshotted card on an undo, as the predecessor does (`sync.py:AnkiSyncer.undo_skip`) — rejected by the undo's rule, in the owner's words: "A card reviewed or changed since the skip is skipped and listed to the owner, never overwritten."
- A confirm that names only the due count, as the predecessor's preview does (`SkipDaysLayer.skip_preview`) — rejected because the take could then move cards the owner never saw, against guardrail (iv).
- Take the server's acceptance of the push as the outcome, with no read-back — rejected because a review made on another client between the converge and the push either loses its schedule to the push or keeps its card from moving, since the sync's merge keeps the card with the newer modification time and adds every review-log row (the pinned engine's `rslib/src/sync/collection/chunks.rs:168-193`), and only reading each card back finds it, to list it to the owner and keep the undo off it.
- Record the take's and the undo's syncs in `sync_runs` beside the private copy's — rejected because those syncs are of a working copy, and a row there would let the five-minute reuse (SPEC-022 R17) answer the owner's next `/sync` from them, leaving the private copy unsynced.
- Wrap a configured search as the predecessor does and no more (`sync.py:AnkiSyncer._skip_day_blocking`, `-is:new -is:learn`) — rejected because that wrap tests neither the due day nor suspension nor burial, so a custom search could move review cards due on another day, suspended or buried, against guardrail (i).
- Drop the configurable search — rejected because the predecessor's search is configurable (`config.py:Settings.skip_deck_search`), and SPEC-083 R3's holds keep any configured search to the study day's due review cards.
- Move a review card in a filtered deck too, as the predecessor does, and put it back on an undo — rejected because the engine's Set Due Date moves it to its home deck and the engine's card update writes a deck without checking that it exists, so the undo could put the card into a filtered deck emptied, rebuilt or deleted since, and the inverse would not be exact (guardrail i).
- Compose the wrapped search from the configured search's parsed terms — rejected because the engine renders parsed terms in its own form (the pinned engine's `rslib/src/search/writer.rs:45-47`), not as the golden's wrap that SPEC-083's A7 pins, while refusing a configured search that does not parse as one expression keeps every other search the golden's own wrap followed by SPEC-083 R3's holds.
- Run the engine in the study day's zone and rollover — rejected because the engine's day in client mode is the collection's rollover hour in the process's own zone (the pinned engine's `rslib/src/scheduler/mod.rs:90-108`), so it holds only when the collection's rollover and configured UTC offset already equal the study day's, and otherwise needs a setting changed, which the push would carry whole (SPEC-083 R23), against guardrail (i); refusing covers the same cases and writes nothing.
- Record a take whose push fails as `failed`, like any other failure — rejected because the engine's sync server commits a push only at `finish` (the pinned engine's `rslib/src/sync/collection/finish.rs:31-37`), so a failure after the push's first request may follow a committed push, and a `failed` row would free the study day and tell the owner that nothing was written while the moved cards stand, where no undo could reach them (SPEC-083 R1, R5, R25).
- Refuse only near a transition of the zone's offset: rejected, because a write already running is never cancelled mid-sync (SPEC-083 R26), so the moment between a check and the engine's day computation has no bound that a window around a transition could cover.
- Accepting the daylight-saving moment as a named risk: rejected, because a push could then carry a changed setting, which guardrail (i) forbids, and only the owner may relax an owner clause.
- Read the process's zone from `TZ` alone, counting an unset `TZ` as UTC: rejected, because the engine reads the zone through chrono's `Local`, which falls back to the host's zone when `TZ` pins none, so such a check could pass while the engine counts from another zone; the checks read the zone through `Local`, and the pin refuses a `TZ` that is not a POSIX rule (SPEC-083 R3, A44).
- Pin the zone by a zone name in `TZ`: rejected, because chrono opens a zone name's file again on each new thread and falls back to the host's zone when it cannot, so a running process could still read another zone; a POSIX rule is parsed from the string alone, and the pin accepts only a rule (SPEC-083 §1, R3, A44).

## Decision Outcome

Chosen option: "(a) an upload path for the skip day alone", the owner's decision at #266
(ADR-089), made as follows.

- **A working copy.** Each take and each undo copies the private collection copy beside it, under
  the exclusive collection lock (SPEC-022 R7), and syncs, writes and pushes only that working copy,
  which is discarded when the run ends, whether it applied or not. The private copy is written by
  SPEC-022's syncer alone, so it never holds a local change for another sync to send.
- **Incremental syncs only.** The converge before the write and the push after it are normal syncs.
  A full or one-way sync demand at either aborts the run: nothing is pushed, no card is recorded as
  moved, no tariff is charged, and the owner is told to sync first (guardrail ii).
- **The preview binds the take.** The preview lists the cards the write would move, read from the
  private copy with the predecessor's wrapped search, held to the study day's due review cards
  outside a filtered deck (SPEC-083 R3), and carries a digest of their ids. The take moves only
  previewed cards that are still due after its converge; when the confirm carries no digest, or
  the list has changed, it writes nothing and answers the new preview (guardrail iv).
- **The prior state first.** Each card's prior due date and the rest of its prior scheduling state
  are committed before the reschedule, and the state the reschedule left it in is committed before
  the push (guardrail iv).
- **The engine's own reschedule.** The engine's Set Due Date runs with the predecessor's day spec,
  which carries no `!`, so with FSRS off each review card keeps its interval; with FSRS on the
  engine sets the interval by its own rule, and the snapshot records it either way. The push
  carries those cards and the review-log rows the engine writes for them, beside the settings and
  the creation stamp that a sync carries whole once the working copy is newer, each as the server
  held it at the converge, the engine's own last-unburied day aside, and nothing else (guardrails i
  and v). It runs only when the engine's own day, the collection's rollover hour in the process's
  zone, is the study day, the collection's configured UTC offset is present and is the process's
  zone, and the process's zone observes no daylight saving and is pinned, checked before any request (the daylight-saving condition and the pin at every preview, take and undo, not only at the service's start) and, for the day and the offset, again on the converged working copy: otherwise the engine would count from another day or rewrite that offset (SPEC-083 R3, R23). The zone is pinned for the service as a fixed rule: its environment sets `TZ` to a POSIX rule, and a preview, a take or an undo in a process whose `TZ` is not a POSIX rule refuses before any request or write, so a change of the host's zone cannot reach a running process (SPEC-083 R3, A44). The checks cannot hold a rollover in the moment after one of them (SPEC-083 §6); a change of the zone's offset there is prevented, a daylight-saving change by the refusal and a change of the host's zone by the pin.
- **The undo compares before it writes.** It restores a card's recorded prior state only when the
  card, as its converge brings it, still carries the state the skip wrote and has no study event
  since the skip began. It lists every other card to the owner and never writes one. A change made
  on another client after the undo's converge can still lose to the restore, which is newer; the
  undo reads each restored card back and lists a card reviewed there (SPEC-083 R32). It is
  owner-triggered and incremental only, and it aborts on any full-sync demand (the undo's rules).
- **Owner triggers only.** The take and the undo run only from the owner's confirm in the bot or the
  Mini App, as owner triggers under ADR-037. Their syncs are recorded on the skip's own row, never
  in `sync_runs`, and no job, recompute step, startup path, retry or spawned task runs them
  (guardrail iii).
- **CHARTER constraint 4 stands as written:** the skip day is the only write back to Anki.

### Consequences

- Good, because a skip moves the study day's due review cards in Anki, as the predecessor's did,
  except a card in a filtered deck, which stays due because the undo could not put it back exactly
  (SPEC-083 R3), and every effect of it on DeckStreak's own game works as SPEC-083 specifies.
- Good, because a run that aborts, or fails before its push's first request, leaves the private
  copy and the owner's collection as they were, and every other sync stays free of uploads.
- Bad, because a review made on another client in the seconds between a take's converge and its
  push can lose its schedule to the reschedule, since the newer change wins the sync's merge. The
  take reads each moved card back and lists such a card to the owner, and the undo leaves it alone
  (SPEC-083). A suspension, a flag, a deck move or a setting made in that window is overwritten the
  same way when the reschedule is newer, and the read-back cannot see it (SPEC-083 §6).
- Bad, because a review made on another client between the undo's converge and its push can lose
  its schedule to the restore; the undo lists that card to the owner (SPEC-083 R32, A42) but
  cannot keep the review's schedule, since the sync keeps the newer card. A suspension, a flag, a
  deck move or a setting made in that window is overwritten the same way when the restore is
  newer, and the read-back cannot see it (SPEC-083 §6).
- Bad, because a review or another change made on a device that syncs only after a take's or an undo's push can be lost to the newer card that push left, and nothing lists it: it reaches the server after
  the read-back (SPEC-083 §6).
- Bad, because the private copy shows the moved cards as due until its next sync downloads the
  change.
- Bad, because the reschedule's review-log rows stay after an undo; the read never counts them as
  study events (ADR-089, SPEC-023 R2).
- Bad, because a review card due that study day in a filtered deck stays due: Set Due Date would
  move it to its home deck, and the undo could not put it back exactly once the filtered deck is
  emptied, rebuilt or deleted (SPEC-083 R3). The owner can empty the filtered deck before taking the
  skip.
- Bad, because a skip writes only when R3's checks find the engine's own day to be the study day
  and the collection's configured UTC offset present and equal to the process's zone. Otherwise the
  preview, the take and the undo refuse before any request, and a take or an undo whose converge
  brings another client's setting that fails either check ends before its push, because the engine
  would count from another day or rewrite that offset, which the push would carry (SPEC-083 R3,
  R23). A rollover in the moment after a check still gets through: the cards then land a day later
  (SPEC-083 §6). A change of the zone's offset in that moment is prevented: R3 refuses a zone that
  observes daylight saving, and a zone the service's environment does not pin as a POSIX rule,
  before any request or write, so neither a daylight-saving change nor a change of the host's zone
  reaches a run in flight (SPEC-083 R3, A40, A44).
- Bad, because a push whose answer is lost may already be committed, so its outcome is not known at
  once. The take's row stays `pending` until the private copy's next sync settles it, and the undo
  leaves its skip `applied` until a later undo, which counts the cards already restored as restored;
  neither answer says that nothing was written (SPEC-083 R25, R26, R32).
- Bad, because the service skips nothing in a zone that observes daylight saving; such a deployment refuses every skip until its zone is changed. An owner whose devices observe daylight saving has the configured offset rewritten at each change, so no fixed zone matches it all year, and for some offsets no fixed zone exists in the tz database at all; ADR-020 assumes an owner without daylight saving (SPEC-083 R3).
- Bad, because a deployment whose environment does not pin the zone skips nothing: the preview, the take and the undo refuse until its `TZ` holds a POSIX rule with no daylight period (SPEC-083 R3, A44).

### Confirmation

SPEC-083's criteria, each red first: the take pushes exactly the previewed cards and their
review-log rows, and no setting changed but the engine's own last-unburied day (A5), and a
configured search moves only the study day's due review cards outside a filtered deck, or is
refused when it closes the wrap's group (A38); a take or an undo holds to the study day and changes
no setting when the engine's day or the collection's configured UTC offset differs or is missing,
in the private copy or as its converge brings it, and refuses before any request or write in a test
process whose zone observes daylight saving (A40), and in a process whose `TZ` is not a POSIX rule,
reading the zone only through chrono's `Local` (A44); every other path
records zero uploads (A6), and only the skip's take and undo reach an engine write (A24); a
full-sync demand aborts a take or an undo, writing nothing (A25, A26, A32); only the owner's
confirm reaches the take and the undo, once per confirm (A27); the preview lists the cards and
binds the take (A28, A39); the prior state is recorded before any card changes (A29); a take lists
a card reviewed on another client during its own window (A34); an undo restores exactly the prior
state of exactly the moved cards (A30) and never writes a card whose change reached the server before its converge (A31),
and lists a card reviewed on another client during its own window (A42); a take or an undo whose
push's answer is lost says that its outcome is not
known yet, never that nothing was written (A35, A41); and the recording layer records a planted
upload, a planted local change and a planted setting change (A33). SPEC-022's no-upload census (its
A15) stays the proof for every sync of the private copy.

## What would make this wrong

- A measured engine or server behaviour breaks a guardrail's proof: for example a normal sync that
  sends a card, note, deck or tag beyond the moved cards, or a setting whose value differs from the
  server's, or a Set Due Date that changes a field the snapshot does not record. The write is then
  withheld until the proof is green again.
- The owner withdraws option (a) (ADR-089): the write is removed, and (b) returns with this ADR
  superseded.

## More Information

CHARTER constraint 4; SPEC-001 §14 (the gate-6 amendment); ADR-011 (side by side); ADR-037;
ADR-089; SPEC-022 R6, R7 and R14; SPEC-023 R2; SPEC-083;
`docs/schematics/skip-day-record-and-effects.md`; #108; #164; #266; #269.
