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
- Restore every snapshotted card on an undo, as the predecessor does (`sync.py:AnkiSyncer.undo_skip`) — rejected by the undo's rule: a card reviewed or changed since the skip is never overwritten.
- A confirm that names only the due count, as the predecessor's preview does (`SkipDaysLayer.skip_preview`) — rejected because the take could then move cards the owner never saw, against guardrail (iv).
- Take the server's acceptance of the push as the outcome, with no read-back — rejected because a review made on another client between the converge and the push either loses its schedule to the push or keeps its card from moving, since the sync's merge keeps the card with the newer modification time and adds every review-log row (the pinned engine's `rslib/src/sync/collection/chunks.rs:168-193`), and only reading each card back finds it, to list it to the owner and keep the undo off it.
- Record the take's and the undo's syncs in `sync_runs` beside the private copy's — rejected because those syncs are of a working copy, and a row there would let the five-minute reuse (SPEC-022 R17) answer the owner's next `/sync` from them, leaving the private copy unsynced.
- Wrap a configured search as the predecessor does and no more (`sync.py:AnkiSyncer._skip_day_blocking`, `-is:new -is:learn`) — rejected because that wrap tests neither the due day nor suspension nor burial, so a custom search could move review cards due on another day, suspended or buried, against guardrail (i).
- Drop the configurable search — rejected because the predecessor's search is configurable (`config.py:Settings.skip_deck_search`), and SPEC-083 R3's holds keep any configured search to the study day's due review cards.
- Move a review card in a filtered deck too, as the predecessor does, and put it back on an undo — rejected because the engine's Set Due Date moves it to its home deck and the engine's card update writes a deck without checking that it exists, so the undo could put the card into a filtered deck emptied, rebuilt or deleted since, and the inverse would not be exact (guardrail i).

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
  carries those cards and the review-log rows the engine writes for them, and nothing else
  (guardrails i and v).
- **The undo compares before it writes.** It restores a card's recorded prior state only when the
  card still carries the state the skip wrote and has no study event since the skip began. It lists
  every other card to the owner and never overwrites one. It is owner-triggered and incremental
  only, and it aborts on any full-sync demand (the undo's rules).
- **Owner triggers only.** The take and the undo run only from the owner's confirm in the bot or the
  Mini App, as owner triggers under ADR-037. Their syncs are recorded on the skip's own row, never
  in `sync_runs`, and no job, recompute step or startup path runs them (guardrail iii).
- **CHARTER constraint 4 stands as written:** the skip day is the only write back to Anki.

### Consequences

- Good, because a skip moves the study day's due review cards in Anki, as the predecessor's did, and
  every effect of it on DeckStreak's own game works as SPEC-083 specifies.
- Good, because a failed or aborted run leaves the private copy and the owner's collection as they
  were, and every other sync stays free of uploads.
- Bad, because a review made on another client in the seconds between a take's converge and its
  push can lose its schedule to the reschedule, since the newer change wins the sync's merge. The
  take reads each moved card back and lists such a card to the owner, and the undo leaves it alone
  (SPEC-083).
- Bad, because the private copy shows the moved cards as due until its next sync downloads the
  change.
- Bad, because the reschedule's review-log rows stay after an undo; the read never counts them as
  study events (ADR-089, SPEC-023 R2).

### Confirmation

SPEC-083's criteria, each red first: the take pushes exactly the previewed cards and their
review-log rows (A5), and a configured search moves only the study day's due review cards outside a
filtered deck (A38); every other path records zero uploads (A6), and only the skip's take and
undo reach an engine write (A24); a full-sync demand aborts a take or an undo, writing nothing
(A25, A26, A32); only the owner's confirm reaches the take and the undo (A27); the preview lists
the cards and binds the take (A28, A39); the prior state is recorded before any card changes (A29);
an undo restores exactly the prior state of exactly the moved cards (A30) and never overwrites a
card changed since (A31); and the recording layer records a planted upload and a planted local change
(A33). SPEC-022's no-upload census (its A15) stays the proof for every sync of the private copy.

## What would make this wrong

- A measured engine or server behaviour breaks a guardrail's proof: for example a normal sync that
  sends more than the moved cards, or a Set Due Date that changes a field the snapshot does not
  record. The write is then withheld until the proof is green again.
- The owner withdraws option (a) (ADR-089): the write is removed, and (b) returns with this ADR
  superseded.

## More Information

CHARTER constraint 4; SPEC-001 §14 (the gate-6 amendment); ADR-011 (side by side); ADR-037;
ADR-089; SPEC-022 R6, R7 and R14; SPEC-023 R2; SPEC-083;
`docs/schematics/skip-day-record-and-effects.md`; #108; #164; #266; #269.
