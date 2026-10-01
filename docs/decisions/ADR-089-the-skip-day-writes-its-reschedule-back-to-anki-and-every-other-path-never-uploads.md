---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner, at #266), the DeckStreak architect"
---

# The skip day writes its reschedule back to Anki, and every other path never uploads

## Context and Problem Statement

At gate 6 the owner approved reusing the owner's own Anki login on four testable conditions, which
ADR-037 records. The first, condition (a), is no upload path, proven against a fake sync server
that records every request. CHARTER constraint 4 says "The skip day is the ONLY write back to
Anki". The predecessor's skip day reschedules the study day's due review cards and writes the
change back to the collection (`sync.py:AnkiSyncer.skip_day`), and its undo restores them
(`sync.py:AnkiSyncer.undo_skip`), at the predecessor's `27ee2bc`.

ADR-083 laid out the options for DeckStreak's skip day and left the choice to the owner at #266:
- (a) an upload path for the skip day alone;
- (b) no write: DeckStreak records the skip, and the owner reschedules the cards in Anki (the
  recommendation);
- (c) a hand-off: (b), with the search and the steps that reschedule the cards in Anki shown.

The owner chose (a): "Upload the skip day". It "writes the reschedule back to Anki, as the old app
did". The owner chose it knowing that it breaks the no-upload condition on which ADR-037's approval
rested. Which condition gives way, for which path, and what proves that every other path still
never uploads?

## Decision Drivers

- The owner's decision at #266: option (a), taken knowing it breaks condition (a).
- CHARTER constraint 4 stands as written: the skip day is the only write back to Anki.
- The owner's collection stays untouchable by construction on every other path.
- A write to the owner's collection is the riskiest act DeckStreak has, so each guardrail is an
  acceptance criterion whose test goes red first against the recording fake sync server.

## Considered Options (the alternatives it was chosen against)

- (a) The skip day writes its reschedule back to the collection, with an exact undo, under the owner's guardrails — chosen: it is the owner's decision at #266.
- (b) DeckStreak records the skip and the owner reschedules in Anki — rejected by the owner at #266, although it was the recommendation: the owner chose the write, which "writes the reschedule back to Anki, as the old app did", where (b) leaves the owner to move the cards by hand.
- (c) A hand-off: (b), with the search, the day spec and the steps shown to the owner — rejected with (b), because it is (b)'s copy, and the owner still moves the cards by hand.
- Supersede condition (a) for every path — rejected because only the skip day needs a write, and every other path keeps the proof that it records zero uploads.
- Amend CHARTER constraint 4 to "DeckStreak writes nothing back to Anki" — rejected because the owner chose the write, so the constraint stands as written.

## Decision Outcome

Chosen option: "(a) the skip day writes its reschedule back to the collection", because it is the
owner's decision at #266.

- **ADR-037's condition (a) is superseded for the skip-day path only.** On that path the skip day's
  reschedule and its exact inverse are written to the collection. Conditions (b), (c) and (d) stand
  for every path, and condition (a) stands for every other path.
- **The owner's guardrails.** Each becomes an acceptance criterion of SPEC-083 whose test is
  planned red first:
  - (i) "The only writes ever made are the skip day's reschedule of that day's due review cards,
    and its exact inverse. Every other path records ZERO uploads against the recording fake sync
    server."
  - (ii) "Incremental sync only. If the server asks for a full or one-way sync, ABORT, write
    nothing, and tell the owner (a forced full upload could overwrite the collection)."
  - (iii) "The write runs only on the owner's explicit skip declaration, inside ADR-037's
    owner-trigger rule. No new scheduled syncs."
  - (iv) "A preview of the cards to be rescheduled is shown before the write, and their prior due
    dates are recorded so the skip can be undone."
  - (v) "The recording-server proof covers both the skip day's exact changes and the zero-upload
    paths."
- **The undo, with its own criteria, red first.** It restores exactly the recorded prior due dates
  of exactly those cards; it is owner-triggered, incremental only, and aborts on any full-sync
  demand; it carries the same recording-server proof; and it writes only cards whose current state
  still equals what the skip wrote. A card reviewed or changed since the skip is skipped and listed
  to the owner, never overwritten. The Consequences and SPEC-083 §6 name what the engine's sync still lets through while a take or an undo runs, and from a device that syncs only after its push.
- **The recorder is proved too.** The recording layer SPEC-022 built
  (`crates/ingest/tests/support/recording.rs`) keeps every request. SPEC-083 adds the criterion
  that it records a planted upload and a planted local change, because a recorder that sees nothing
  proves nothing.
- **What follows from it.** CHARTER constraint 4 is unchanged. ADR-037 gains a note naming this
  ADR. SPEC-001's gate-6 amendment and SPEC-022's exclusion gain insert-only amendments. ADR-083
  records the option taken and how the write is made, and SPEC-083 specifies the write, the undo
  and their criteria.

### Consequences

- Good, because a skip moves the study day's due review cards in Anki by itself, as the
  predecessor's did (#108), except a card in a filtered deck, which stays due because the undo could
  not put it back exactly (SPEC-083 R3).
- Good, because every other path keeps condition (a)'s proof: zero uploads against the recording
  fake sync server.
- Bad, because DeckStreak can change the owner's collection on one path, so a defect there reaches
  the owner's cards. The guardrails bound it: incremental syncs only, only the previewed cards, the
  prior state recorded before the write, and an undo that never writes a card whose change reached the server before its own converge. A card changed on another client in the seconds between the undo's converge and its
  push can still lose that change to the restore, which is newer: the undo lists such a card when
  the change was a review, and cannot see any other change (SPEC-083 R32, §6). A review or another change made on a device that syncs only after a take's or an undo's push can be lost the same way, and nothing lists it (SPEC-083 §6).
- Bad, because the review-log rows the engine's Set Due Date writes stay after an undo: an
  incremental sync removes no review-log row. The read never counts them as study events
  (SPEC-023 R2).

### Confirmation

SPEC-083's criteria for guardrails (i) to (v) and for the undo, each recorded red then green in
`docs/red-first/SPEC-083.md`, run against the recording layer in front of the engine's own sync
server, together with the recorder's own control. SPEC-022's census (A15) stays the proof for every
sync of the private copy.

## What would make this wrong

- The owner withdraws option (a): (b) returns, and condition (a) holds for every path.
- A proof shows a write outside the skip set, or an upload on any other path: the skip's write is
  withheld until that proof is green.
- The owner's sync server demands a full sync so often that most skips abort: the skip day goes
  back to the owner, and guardrail (ii) stands until the owner decides.

## More Information

CHARTER constraint 4; ADR-037 (conditions (a) to (d)); ADR-083; SPEC-083; SPEC-022 R14 and A15;
SPEC-023 R2; SPEC-001 §14; #266; #108.

Amendment (2026-10-01): guardrails (i), (ii), (iii) and (iv) are amended by ADR-301, the owner's
decision at #514, and their text above is kept as it was. DeckStreak writes to the collection only
through declared write classes, each with its own ADR in ADR-089's form. The skip day is the
first such class, at the approval rung. (i): the only writes ever made are each declared write
class's exact changes and their exact inverses, and every other path records zero uploads against
the recording fake sync server. (ii) binds every class: incremental sync only, and on any full or
one-way sync demand the write aborts and writes nothing. (iii): the owner's explicit declaration
becomes the approval rung; at the autonomous rung, promotion by a passed trial, the guard metric's
own undo and the kill switch replace it, and the batch rides the study day's one scheduled sync.
(iv): each batch records the prior state and is previewed, and its undo writes only cards whose
current state still equals what the batch wrote. (v) stands, and each class's ADR extends it to
that class's exact changes and their inverse.

Note (2026-10-01, #518, the skip day's rung): in the amendment above, "at the approval rung" reads
more exactly as a ceiling. Its ceiling is the approval rung, because the owner's skip declaration
approves each batch. The sources are ADR-301's skip day paragraph under "The amended clauses" and
guardrail (iii) above, which runs the write only on the owner's explicit skip declaration.

Note (2026-10-01, #518, the skip day's budget): ADR-301 (c) requires each write class's ADR to state
its change budget, its dwell time between changes and the band a result must clear. ADR-301 grants the
skip day no exemption: part (c) binds it as it binds every class. This note states what a delivered
document on dev decides for each of the three, and what it leaves open.

- Change budget. The skip's reschedule is bounded to the study day's due review cards (guardrail (i)
  above; ADR-301 (a): "ADR-089 (i) bounds it to those cards"), and its preview names every card it
  moves (guardrail (iv) above). The planned SPEC-083 R21 refuses a set of more than `SKIP_MAX_CARDS`,
  the planned golden constant (5,000), with `too_many_cards` before any change.
- Dwell. No dwell time between changes is decided. The nearest planned rule is the planned SPEC-083 R2:
  a study day that already holds a pending or applied skip, not undone, refuses a second take with
  `already_skipped`. It bounds a day to one skip that is not undone; it is not a dwell between changes.
  The same rule ends: "After an undo the day can be skipped again."
- Band. No band a result must clear is decided. ADR-301 (c) has a class reach the autonomous rung only
  after a pre-registered n-of-1 trial passes, and the skip day's ceiling is the approval rung, so no
  trial of the skip day is decided, and with it no band. The dwell and the band are left to #108,
  "Skip / cheat day (the one Anki write path)", the issue of the planned SPEC-083.
