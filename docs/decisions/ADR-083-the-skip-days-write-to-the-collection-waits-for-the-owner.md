---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The skip day's write to the collection waits for the owner's decision, and DeckStreak records the skip without one

## Context and Problem Statement

The predecessor's skip day is its one write back to Anki.
`pipeline_layers/skip.py:SkipDaysLayer.take_skip_day` records a row and hands the write to
`sync.py:AnkiSyncer.skip_day`, which converges the collection with the sync server, snapshots every
affected card, reschedules today's due review cards one to three days out with their intervals kept
(`set_due_date` with the day spec of `skip.py:skip_spec`), and uploads the change, all or nothing.
Its undo (`SkipDaysLayer.undo_skip_day`, `AnkiSyncer.undo_skip`) restores the snapshot and uploads
again (predecessor `27ee2bc`).

DeckStreak never uploads (ADR-037). At gate 6 the owner approved reusing the owner's own Anki
login on testable conditions, and the first is that no upload path exists, proven against a fake
sync server that records every request (ADR-037; SPEC-022 R14). A skip day that reschedules cards
needs exactly the path that condition forbids.

The accepted documents already describe the skip day without the write: SPEC-001's gate-6
amendment says the skip day "is therefore recorded in DeckStreak's own database and never written
to the collection", ADR-037 forbids an upload, and SPEC-022's exclusions say DeckStreak "writes
nothing back to Anki, ever". CHARTER constraint 4 still says "The skip day is the ONLY write back to
Anki", and issue #108's title and first two criteria still describe the write. The two readings
disagree on one sentence, and the disagreement is an owner condition, not a builder's choice.

Should DeckStreak write a skip day to the collection, and if so how, without resolving an owner
condition silently?

## Decision Drivers

- The owner's gate-6 condition: no upload path, proven by a recording fake server.
- The owner's collection must stay untouchable by construction, not by convention.
- A skip day's effects on DeckStreak's own game (the streaks bridged, the consistency run held, the
  day's quests voided, its chests paused, the tariff, the undo) do not depend on the write.
- A charter constraint changes only by an owner decision recorded in an ADR before the code.
- Parity: the predecessor's skip moved today's due review cards out of today's queue; without the
  write they stay due in Anki until the owner moves them.

## Considered Options (the alternatives it was chosen against)

- (a) An upload path for the skip day alone, which converges, snapshots, reschedules today's due review cards with the predecessor's day spec and uploads — rejected until the owner decides, because it is the one path gate 6's first condition forbids, it would supersede ADR-037's no-upload rule, and it needs a recording-server proof of its own that nothing else uploads.
- (b) No write: DeckStreak records the skip and applies every effect of it in its own database, and the owner reschedules the cards in Anki — chosen, because ADR-037 and gate 6 hold unchanged, every DeckStreak-side effect works, and it is what SPEC-001's gate-6 amendment and SPEC-022 already say.
- (c) A hand-off: option (b), with the confirm sheet and the bot showing the exact search and day spec to use in Anki's own Set Due Date, and the steps — adopted inside (b) as its copy, because it helps the owner move the cards without any write path, so it is (b) with better words rather than a separate path.
- (c2) Leave the write to the predecessor's own skip day during side by side, and decide at cutover — rejected as the answer, because side by side ends at cutover (#164) with the question still open, and two skip commands in two bots can record two different days.

## Decision Outcome

Chosen option: "(b) no write, with (c)'s copy", because it keeps the owner's condition and
ADR-037 intact while every effect of a skip day on DeckStreak's game works now, whichever option the
owner takes later.

- DeckStreak's skip day writes nothing to the collection, opens no collection for writing, and
  sends no request to the sync server. It records the skip (SPEC-083), prices it with the tariff, and
  every context that the skip affects reads it at the next recompute.
- The preview and the confirmation show the owner the search that selects today's due review
  cards, the configured search wrapped as the predecessor wraps it, and the day spec, which carries
  no `!` so Anki keeps each card's interval (SPEC-083 R3, from the goldens of `skip.py:skip_spec` and
  of the wrap in `sync.py:AnkiSyncer._skip_day_blocking`). The owner reschedules in Anki.
- DeckStreak's undo reverses DeckStreak's record and refunds the tariff. The copy says that a
  reschedule made in Anki is undone in Anki.
- The owner decides at #266. Option (b) is built; (c) is inside it; (a) needs the owner's go,
  supersedes gate 6's first condition and ADR-037's no-upload rule for that one path, and becomes a
  delivery of its own with its own recording-server proof.
- This ADR does not edit the charter. CHARTER constraint 4's sentence "The skip day is the ONLY write
  back to Anki" disagrees with SPEC-001's gate-6 amendment, ADR-037 and SPEC-022. The owner's
  decision at #266 settles which reading stands, and a charter amendment records it: under (b) the
  sentence becomes "DeckStreak writes nothing back to Anki", and under (a) it stays and ADR-037 gains
  its one exception.

### Consequences

- Good, because the owner's collection stays untouchable by construction, and gate 6's condition
  holds with no new proof.
- Good, because every effect of a skip day on DeckStreak's own game works now, and survives the
  owner's later choice unchanged.
- Bad, because the owner reschedules in Anki by hand, so a skip no longer empties today's Anki queue
  by itself.
- Bad, because until #266 is decided, CHARTER constraint 4 and the accepted documents disagree on
  one sentence.

### Confirmation

SPEC-083's criteria: a skip and its undo send no request through a recording layer in front of the
sync endpoint and leave the collection copy's bytes unchanged (A5), and no skip module names an
engine write or a sync call, with a planted one refused (A6). SPEC-022's no-upload census (its A15)
stays the proof for every sync.

## What would make this wrong

- The owner chooses (a) at #266: a delivery then adds the upload path for the skip day alone, with
  its own recording-server proof, and this ADR is superseded.
- The owner rarely moves the cards after a skip, so skipped days pile today's reviews into
  tomorrow's queue: the evidence then reopens #266.

## More Information

CHARTER constraint 4; SPEC-001 §14 (the gate-6 amendment); ADR-011 (side by side); ADR-037; SPEC-022
R14 and §5; SPEC-083; `docs/schematics/skip-day-record-and-effects.md`; #108; #164; #266; #269.
