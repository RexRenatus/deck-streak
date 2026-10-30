The owner amends ADR-301 (a), the never-list, in the owner's own words: "Exempt your taps" (owner, 2026-10-04, ANK-03, answering "Which rule covers your own taps?"), "send the commits to me on telegram for signature", "i gove you peision for the rulings" and "try again" (owner, 2026-10-04): a write the owner makes by an explicit UI gesture in DeckStreak's study client is exempt from never-list entries 1-8 (`docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md:112-119`), and every other path, every write class included, stays bound by those entries verbatim.

# OWNER RULING 2026-10-04: the owner's own taps in the study client

## What was held

At dev `df2a4cdb`, ADR-301 (a) (lines 105-119) lists eight writes that "No write class makes ... at any rung, and no
owner approval admits one" (line 107):

| # | the never-list entry (ADR-301:112-119) | the study-client tap that meets it |
|---|---|---|
| 1 | edit review history | undo an answer after it has synced |
| 2 | forget or reset a card | Forget |
| 3 | delete a preset | delete a preset in deck options |
| 4 | force a full sync | the one-way sync choice, upload or download (SYNC-01) |
| 5 | mass-reschedule | switching a preset's scheduler, which rewrites the memory state of every card on it (OQ1) |
| 6 | set a per-card due-date policy | set due date |
| 7 | change a reviewed note's type | change note type |
| 8 | delete a reviewed card or note | delete a reviewed card or note |

The study client is a real Anki client (ANK-01) and the owner's primary one (OQ1), and the owner chose full AnkiMobile
parity for their own taps (ANK-03). Each tap above meets an entry. Under ADR-301 as written, none can be built.

## What replaces it

- **The exemption.** A write is exempt from entries 1-8 only when ALL of these hold:
  - it is made from a UI gesture the owner performs in DeckStreak's study client (web or iPhone/iPad);
  - it is one gesture's own write, on the card, note, preset or collection the gesture names;
  - it is shown before it is made, with what it changes, and it asks for confirmation where Anki asks for one.
- **What stays bound, verbatim.** Every path that is not such a gesture stays under entries 1-8 exactly as ADR-301
  states them: every write class, every agent, the deck optimizer, every batch, every background or scheduled job, and
  every repair or reconciliation of sync. No approval, owner approval included, turns any of those into an exempt tap.
- **Containment the build must prove.** The exempt functions are reachable only from the study client's UI layer. A
  test proves that no non-UI caller reaches any of them: no write class, agent route, server job or sync repair.
- **The one-way sync (entry 4)** keeps SYNC-01's guards: before an upload or a download the client shows what each
  side loses, writes an on-device backup first, and an upload first checks that the server's offsite snapshot exists.
- **The scheduler switch (entry 5)** is the owner's tap on one preset (OQ1, FSRS-7 rebuilt from review history and
  written to stock fields only). Every other experimental model only reorders cards the stock scheduler already made
  due (OQ4) and stays bound by entries 5 and 6.
- **Order.** Nothing calls an exempt function before this ruling is signed and on dev. ADR-301 keeps its text; the
  delivery that builds the first exempt tap adds a dated note at line 107 naming this ruling.

## Why it is admitted

It is a weakening: ADR-301 says no owner approval admits a never-list write, and this admits eight kinds of write.
Its reach is the owner's own hand. Each entry protects the record and the scheduler from writes the owner did not
make one at a time: a batch, an agent, a standing rule. A tap the owner makes, sees and confirms in their own client
is what AnkiMobile already lets them do to the same collection today (ANK-08: both clients sync to the owner's own
server).

The rejected alternatives:
- **Main's recommendation (an owner-acts category that keeps the never-list's entries).** It leaves the client short
  of AnkiMobile parity, which the owner declined at ANK-03.
- **Owner approval of a write class's batch.** ADR-301 (a) refuses it, and this ruling keeps that refusal: approval
  is not a tap.
