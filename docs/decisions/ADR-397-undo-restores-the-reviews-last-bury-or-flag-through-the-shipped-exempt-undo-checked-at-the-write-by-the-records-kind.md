---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-397: Undo restores the review's last bury or flag through the shipped exempt Undo, checked at the write by the record's kind

Decides SPEC-383 (issue #753). It is built on the owner-taps ruling, as ADR-382 is: an exempt tap
is reachable only from the UI, and that is proven; it makes one gesture's own write on the card it
names; and the write is shown before it is made, with what it changes. ADR-382 decided how the
review's own last answer is undone under it. This record decides whether and how a bury or a flag
is undone too.

**Amends, by an insert-only amendment section appended to each ADR, after SPEC-371's, with no
existing line changed (the sections' text is at the end of this record):** ADR-382 D2 and its
Consequences; ADR-361 D1, for undo only; ADR-337's Decision Outcome. It adds no table pair and no
exempt row, so ADR-356 stands as SPEC-371 left it, and it adds no export, so ADR-348's boundary
stands.

## Context and Problem Statement

At `dev` `e7ecf10d` the web review's Undo reverts only the review's own last answer (SPEC-371).
After a bury or a flag it offers nothing: the engine's last step is no longer the record's, so
`judge` reads `Changed` (`crates/engine-core/src/undo_answer.rs:71-98`). Three costs follow, each
measured in SPEC-383 section 1:

- A bury made by mistake takes a due card out of today's review, and the review holds no unbury:
  none of the 18 ordinary pairs (`crates/engine-core/src/table.rs:122`) unburies a card.
- A flag pressed on a card that carried a flag other than red loses that flag: `toggled_red` turns
  flags 2 to 7 into red and red into none (`crates/engine-core/src/review.rs:19-21`), so the toggle
  back cannot restore it.
- Either change also takes away the undo of the answer before it.

The engine already restores the card's prior row through (3,8), which the exempt Undo reaches
behind the owner's gesture (`table.rs:337-340`, `crates/engine-core/src/dispatch.rs:260-282`).
What is missing is a record that names its kind and its card, a read of the card's queue, flag and
sync mark, and a check for each kind.

## Decision Drivers

- The shipped rule for answers stays true: the same record, the same `judge`, the same rows.
- Only the review's own change is undone, on the card it changed, and only while it has not synced.
- The learner sees what the undo changes before it is made, and the control says which change it
  would undo.
- The restore is exact: the queue and the flag the card had, not a value computed from its type.
- No new door: the gesture, the target and the containment SPEC-371 proved are reused.
- The shipped row anchors, model variables and witnesses do not move.

## Considered Options (the alternatives each was chosen against)

### D1. Whether Undo reaches bury and flag

- Chosen: build it, because a mistaken bury and a lost flag are each recoverable today only outside the review, and the engine already holds an exact restore behind the shipped door.
- Rejected: no build, keeping Undo to the answer, because "a flag toggles back by itself" holds only for a card with no flag or a red one, and every mistaken bury or flag also costs the answer's undo.
- Rejected: an unbury button in the review instead of an undo, because it restores no flag, needs a write the table does not hold, and puts back a queue computed from the card's type.

What each choice costs. Building costs the code a second record cell, a fixed read, a pure check,
a restore path, ten keys in seven locales and an amended model; it costs the learner nothing they
have today. Not building costs the code nothing, and costs the learner every mistaken bury of a due
card and every flag replaced by red.

Chosen against: no build; an unbury button.

### D2. The undoable set, its scope and the slot

- Chosen: the review's last action only, an answer, a bury or a flag, in one slot held as the shipped `LAST_ANSWER` or a new `LAST_MARK` and never both, because it is the undo a learner expects (the last thing done) and keeps every shipped answer line, row anchor and model variable.
- Rejected: a stack of the review's changes, because an undo itself begins an engine step, so a second undo needs records that outlive the first and a model with no bound.
- Rejected: one cell holding an enum in place of `LAST_ANSWER`, because it rewrites the lines rows `S37113` to `S37116` anchor on and the model's `record` variable, for a rule two cells hold by rows `S38317` to `S38319` and the census.
- Rejected: keeping the answer's record beside a later bury or flag, because the answer can no longer be undone after them (`judge` reads `Changed`), so the control would offer what the check refuses.

The shipped own-last-answer rule stays true for answers: `rate` records as shipped, `judge` is
unchanged, and an answer's record decodes as before (SPEC-383 R3, A15).

Chosen against: a stack; one enum cell; two live records.

### D3. The write, its check and the sync

- Chosen: the shipped exempt Undo, with a record that names its kind and card, a pure check per kind at the write, and then the engine's own (3,8), because the engine's undo restores the card's exact prior row and the door already holds the gesture, the target and the containment.
- Rejected: forward writes, an unbury for a bury and SetFlag back for a flag, because the unbury computes a queue from the card's type, SetFlag back adds a step of its own, and both need new rows in the table.
- Rejected: a second exempt row for the restore, because (3,8) is one method and SPEC-371 decided its one row; two rows would split one door in two.
- Rejected: a new `GestureRefusal` variant per cause, because the page picks its notice by the kind it pressed, and the shipped `NotTheTarget` keeps each cause off the door.
- Rejected: the step and label alone, because a confirmation aimed at another card must be refused, and the card read proves the change is still there and unsynced.

The check sits in the core, at the write, after the gesture is checked and before (3,8): `run_undo`
reads the kind, and a bury or a flag goes to the private `run_restore`, which reads (3,7) and the
card's `CardMark`, runs `judge_change`, and writes only when it admits. A record of another card is
refused `NotTheCard`; another session's or another control's later step is refused `Changed`; a
synced change is refused `Synced`. A normal sync discards the engine's undo queue, so a change made
before it cannot be undone after it. An undo of an unsynced change returns the card to its row
before the change, with nothing left for the next sync to send for it.

Chosen against: forward writes; a second exempt row; a new refusal variant; the step and label
alone.

### D4. The surface and the tests

- Chosen: the web review only, one undo control whose label names the change, and the shipped dialog by kind, because the web review is the one client with an undo, and the native clients have none to extend (#714).
- Rejected: the native clients in this delivery, because they hold no undo of any kind and refuse (3,8); that is the native undo SPEC-358 section 7 moved out (#714).
- Rejected: one "Undo" label for all three, because the learner would confirm without knowing which change goes back.
- Rejected: an undo of a flag with no confirmation, because every exempt undo is shown before it is made, whatever its size.

The control reads "Undo answer", "Undo bury" or "Undo flag" by the view; key `u` and the remote's
button 4 are unchanged; the dialog names the card and what the undo changes, focused on "Keep it".
Ten keys join each of the seven locales. Every acceptance criterion is red first, at a committed
stub, with its red and green in `docs/red-first/SPEC-383.md`. The rows are `S38301` to `S38321`.
The formal decision is REQUIRED by surface (D7), and Lean is not applicable: `judge_change` is
seven comparisons in a fixed order, held operand by operand.

Chosen against: the native clients now; one label; a flag undo with no confirmation.

### D5. Shape

- Chosen: one delivery under SPEC-383 and ADR-397, with insert-only amendment sections on ADR-382, ADR-361 and ADR-337, because its criteria, model properties, rows and seventy strings are a delivery of their own, and SPEC-371 stays as it shipped.
- Rejected: an insert-only amendment of SPEC-371 and ADR-382, because an amendment is counted but not judged for shape, so twenty-eight criteria, a manifest and a band would sit where the probes do not read them.

Chosen against: an amendment of SPEC-371 and ADR-382.

### D6. What `flag` answers the page

- Chosen: `flag` keeps its answer, the card's new flag, and answers an error unless it recorded the change, because a success then means the record exists and the page sets the flag's undo itself.
- Rejected: `flag` answering its flag and its undo view together, because it changes an export's type and nine shipped call sites and fakes across four test files for a value a success already implies.
- Rejected: reading the whole card again after a flag, because it renders the card's text again for one field.

Chosen against: a two-field answer; a second card read.

### D7. The model

- Chosen: amend `formal/tla/UndoOwnAnswer` in place, with new variables, actions and two properties whose witnesses reuse the shipped switches, because its covers are the spans this delivery changes and no shipped property, witness or `TypeOK` conjunct moves.
- Rejected: a new model entry, because two entries would cover the same `run_undo` and `undo` spans, and an entry cannot extend another entry's module.
- Rejected: a third property that the slot holds one record, because that is a fact of the offer, not of what is restored, and A20 and rows `S38317` to `S38319` hold it.

Chosen against: a second entry; a slot property.

### D8. Where the check for a bury or a flag lives

- Chosen: a new pure `judge_change` in its own module, `undo_change.rs`, because `judge` and the six rows anchored inside it stay byte for byte.
- Rejected: one `judge` that reads the kind, because it rewrites every line rows `S37103` to `S37108` anchor on and moves the model's covered span for no new behaviour of answers.

Chosen against: one `judge` for all kinds.

## Decision Outcome

D1 to D8 as chosen above. The core gains `Kind` and three fields of `Recorded`, `undo_change.rs`
(`Mark`, `judge_change`), the fixed read `CardMark`, and the private `run_restore` behind the
shipped `run_undo`. The web adapter gains `LAST_MARK`; `bury`, `flag` and `rate` keep the one slot;
`undo_offer`, `undo` and `current_card` read it by kind. The page labels the control and the dialog
by kind, and seven locales gain ten keys each. No table pair, exempt row, gesture, refusal variant,
export or native line is added.

## Consequences

- Good: a mistaken bury or flag in the review can be undone, exactly, while it has not synced, and
  the answer's undo is unchanged.
- Good: the check is made at the write, so an offer that went stale between the press and the
  confirmation is refused, whatever changed it.
- Good: a learner who replaced a card's flag with red gets that flag back, which no toggle can do.
- Bad: the delivery adds state, `LAST_MARK`, and with it two properties to keep in the model.
- Bad: the native clients still have no undo, so the same mistake is undoable on the web and not
  on a phone (#714).
- Neutral: the undo of a synced change stays unbuilt, and the control says so.

### Confirmation

SPEC-383's A1 to A28, the censuses' plants, the mutation rows `S38301` to `S38321`, and the model
`formal/tla/UndoOwnAnswer` with its two new witnesses,
`witness/an-undo-restores-only-the-offered-change.cfg` and
`witness/a-restore-runs-only-on-a-confirm.cfg`, beside its two shipped ones.

## What would make this wrong

- If the engine keeps the user flag outside the low three bits of `flags`, `judge_change` reads the
  wrong flag; the build measures it first and stops.
- If (3,8) does not restore a buried card's queue or a flag exactly, D3's chosen option falls and a
  forward write returns; A1 and A2 read every column of both card reads and would show it.
- If the native clients gain an undo, they reach the same door with the same record, and D4's
  rejected option returns under #714.
- If a web sync lands that keeps the engine's undo queue, `judge_change`'s `usn` check still
  refuses a synced change.

## The amendment sections

Each section below is appended at the end of its ADR, after SPEC-371's amendment section. No
existing line of that ADR changes.

### Appended to ADR-382

```markdown
## Amendment: Undo also reaches the review's last bury or flag (SPEC-383)

ADR-397 amends D2 and the Consequences. The rest of each stands.

- **D2 (`:59-66`).** D2 chose the review's own last answer only and rejected "Any last change: an
  answer, a bury or a flag", because a bury or a flag leaves no row the core can read back without
  new reads. ADR-397 adds those reads: a record that names its kind and its card, and a fixed read
  of the card's queue, flag and sync mark. The rejected option returns as the review's last action,
  one of the three, in one slot, checked at the write by its kind (ADR-397 D2, D3), as "What would
  make this wrong" (`:171-181`) anticipated. The answer's record and `judge` are unchanged. "After a
  bury or a flag, Undo is therefore not offered; a flag toggles back by itself" no longer holds:
  Undo offers that bury or flag while it has not synced.
- **Consequences (`:158`).** "Bad: undoing a bury or a flag is no longer offered (D2)" no longer
  holds after SPEC-383.
```

### Appended to ADR-361

```markdown
## Amendment: the token for undo also reaches the review's last bury or flag (SPEC-383)

ADR-397 amends D1, for undo only, after SPEC-371's amendment. The rest of D1 stands.

- **D1 (`:37-55`).** SPEC-371's amendment let the undo of the review's own last answer through the
  owner's gesture, and said that Undo no longer reverts a bury or a flag. ADR-397 widens that reach
  to the review's last bury or flag, through the same gesture, checked at the write by the record's
  kind (ADR-397 D3). Bury and flag themselves stay ordinary calls, as D1 decided.
- **D2 (`:57-72`)** is unchanged in effect: a confirmed undo still clears the kept card.
```

### Appended to ADR-337

```markdown
## Amendment: the undo of an unsynced bury or flag (SPEC-383)

ADR-397 amends the Decision Outcome (`:41-44`), as SPEC-371's amendment did. The exempt Undo also
holds the undo of the review's last bury or flag while it has not synced. It meets the owner-taps
ruling's three conditions as the undo of an answer does: it is reachable only from the review, it
writes only on the card the gesture names, and it is shown, with what it changes, before it is
made.
```
