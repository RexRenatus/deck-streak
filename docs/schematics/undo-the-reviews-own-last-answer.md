# Schematic: from the press to the confirmed undo, and every refused path (SPEC-371)

Kind: **data flow** for the offer, the confirmation and the check at the write; a **state machine**
for the review; and a **component** view of the doors before and after. Every `path:line` was read
at DeckStreak `dev` `2438c4c6`, which holds SPEC-365 and SPEC-366. Names after this delivery are
those SPEC-371 R1 to R15 give.

## 1. The press asks for an offer

```mermaid
flowchart TD
  press["owner's press: the bar's Undo answer button, key u, or remote button 4"] --> act{"review.ts act: view.undo?"}
  act -->|"null"| nothing["nothing happens"]
  act -->|"synced"| synced["announces undo-synced; nothing is sent"]
  act -->|"answer"| busy["question or answer cell undo: busy, effect offer"]
  busy -->|"client.undoOffer()"| session["session.ts: the offer operation"]
  session -->|"engine.undo_offer()"| offer["wasm.rs: undo_offer (entry file)"]
  offer --> record{"LAST_ANSWER holds a record?"}
  record -->|"no"| none["offer null, why none"]
  record -->|"yes"| reads["reads (3,7) now and Review(record.review)"]
  reads --> judge{"undo_answer.rs: judge"}
  judge -->|"Synced"| whysynced["offer null, why synced"]
  judge -->|"Gone, NotTheCard or Changed"| none
  judge -->|"admits"| text["card text: (27,6) question nodes joined, (27,9) StripAvTags, (27,14) HtmlToTextLine"]
  text --> json["offer: card, step, text, grade, returns"]
  json -->|"offered"| confirming["review.ts: confirming, the dialog"]
  none -->|"not-offered"| side["back to the question or the answer, with its notice"]
  whysynced -->|"not-offered"| side
```

The empty queue is one of `judge`'s `Gone` arms: after a normal sync, or when nothing was
answered, the engine's undo label is empty and no offer is made.

## 2. The confirmation reaches the write

```mermaid
flowchart TD
  confirming["confirming: the dialog names the card as text, the answer and the state it returns to"] --> choice{"the next action"}
  choice -->|"Keep it, Escape, or any other action"| kept["back to the side; nothing is sent; the other action is not carried out"]
  choice -->|"the Undo action again, or the Undo answer button"| busy2["busy, effect undo"]
  busy2 -->|"client.undo(card, step)"| session2["session.ts: undo"]
  session2 -->|"engine.undo(card, step)"| undo["wasm.rs: undo (entry file)"]
  undo --> same{"card and step equal LAST_ANSWER's?"}
  same -->|"no: a stale offer"| changed["NotUndoable Changed"]
  same -->|"yes"| judge2{"judge, against (3,7) now"}
  judge2 -->|"Synced"| nsynced["NotUndoable Synced"]
  judge2 -->|"Gone, NotTheCard or Changed"| changed
  judge2 -->|"admits"| mint["OwnerGesture::from_tap(ExemptWrite::Undo, Target::Card(card))"]
  mint -->|"run_exempt(gesture, recorded bytes)"| core["the core's check at the write, section 3"]
  core -->|"refused: NotTheTarget"| changed
  core -->|"(3,8) run"| clear["LAST_ANSWER and SHOWN cleared; the next card is loaded"]
  nsynced -->|"error code undo-synced"| refused["undo-refused: the next card, with its notice"]
  changed -->|"error code not-undoable"| refused
```

While the dialog asks, the focus is on "Keep it", and the page's key reader leaves a key aimed at a
button to that button, so key `u` does not confirm: the keyboard confirms through the dialog's "Undo
answer" button, and remote button 4 confirms from the answer side; on the question side the remote's
reader resolves no key but show answer, so there the bar's "Undo answer" button asks for the offer.

## 3. The check at the write, in the core

```mermaid
flowchart TD
  input["run_exempt: an OwnerGesture for Undo and the request bytes"] --> decode{"checked: decodes as Recorded?"}
  decode -->|"no"| undecodable["GestureRefusal::Undecodable; the engine is not called"]
  decode -->|"yes: Undo, card, recorded"| reads2["run_undo: reads (3,7) now and Review(recorded.review) through the backend"]
  reads2 --> row{"a review row?"}
  row -->|"no"| gone["Gone"]
  row -->|"yes"| card{"its cid is the gesture's card?"}
  card -->|"no"| notcard["NotTheCard"]
  card -->|"yes"| usn{"its usn is -1?"}
  usn -->|"no"| syncedr["Synced"]
  usn -->|"yes"| empty{"the undo label is empty?"}
  empty -->|"yes"| gone
  empty -->|"no"| step{"last_step and label equal the record's?"}
  step -->|"no"| changedr["Changed"]
  step -->|"yes"| run["run (3,8) with an EMPTY request: the review row is removed and the card is restored"]
  gone --> target["GestureRefusal::NotTheTarget, write Undo; nothing is reverted"]
  notcard --> target
  syncedr --> target
  changedr --> target
```

## 4. The review's states

```mermaid
stateDiagram-v2
  state "question or answer" as side
  state "busy, effect offer" as asking
  state "confirming" as confirming
  state "busy, effect undo" as writing
  state "next card" as next
  side --> asking: undo, when view.undo is answer
  side --> side: undo, when view.undo is synced or null; nothing sent
  asking --> confirming: offered
  asking --> side: not-offered, with its notice
  confirming --> writing: undo, or the Undo answer button
  confirming --> side: keep, Escape, or any other action, not carried out
  writing --> next: the undo is made
  writing --> next: undo-refused, with its notice
```

No cell answers in either busy state, so a press while the offer loads, or while the write runs,
does nothing. A key repeat is dropped before it reaches the machine (`keys.ts:47`). The done state
holds no cell (`review.ts:59`), so it offers no undo.

## 5. Who can reach Undo (3,8)

| door | at `dev` `2438c4c6` | after SPEC-371 |
|---|---|---|
| the core's `run` (`dispatch.rs:122-141`) | admits (3,8) on both transports (`table.rs:144-150`) | refuses `NeedsGesture` |
| the native `run` | admits it (`allow_list.rs:61-65`) | refuses `EngineRefusal::NotAllowed { 3, 8 }` at the allow-list |
| the web `run_method` (`wasm.rs:650`) | admits it (`study.rs:82`) | refuses: not a study call, and the core refuses it too |
| the web `run_exempt` export (`wasm.rs:272-289`) | indices 0 to 5, no Undo | unchanged: 0 to 5, no Undo |
| the web `undo` (`wasm.rs:252-259`) | runs (3,8) unchecked, on the press | `undo(card, step)`: only after an offer and a confirmation, through the gesture, checked at the write |
| the web `undo_offer` | absent | reads and judges; writes nothing |
| `Dispatcher::run_exempt`'s `Undo` arm | absent | the one door to (3,8), which needs an `OwnerGesture` minted for Undo |
| any other caller of the name `undo` outside the core | the bot's habit undo (`commands.rs:657`), the bot router's literal (`one_router.rs:670`), three ingest fixture lines | held by name in the census, with their reasons; a new one is refused by name |

The gesture for Undo is minted only in `wasm.rs`'s `undo`. The Rust census
(`crates/engine-core/tests/containment.rs`) holds that, and the page's census
(`web/app/src/lib/study/undo-reach.test.ts`) holds `.undo(` to the review and the session, the
offer to one call each, and the `undo` effect to `confirming`.

## 6. What carries state

- `LAST_ANSWER` (new, `wasm.rs`): the review's own last answer, its card, its grade, the state it
  left and the engine's undo status when it was made. `rate` writes it after `run_answer`, only
  when the newest review's card is the rated card. `current_card`, `undo_offer` and `undo` read it.
  A successful undo, `open` and `close` clear it. It lives only on the Worker's one thread, beside
  the four `thread_local!` items the web engine holds today (`wasm.rs:66-72`).
- `SHOWN` (`wasm.rs`): the kept card, unchanged by this delivery. A successful undo clears it, as
  today.
- The engine's undo queue: every begun operation advances its step, and a normal sync discards it.
  It is read through (3,7) at the offer and again at the write. Between the two, the learner, the
  review's other controls and a future web sync can each change it, which is why `judge` runs at
  the write and why the model `formal/tla/UndoOwnAnswer` covers `judge`, `run_undo`, `undo`,
  `undo_offer` and `rate`.
- The offer on the page: the card and the step it was shown, carried back in the confirmation for
  the Worker to compare with its record.
- `OwnerGesture`: minted in `undo` and consumed by `run_exempt` in the same call. It is never
  stored.

## 7. The review's last bury or flag, from the press to the restored card (SPEC-383)

Kind: a **state machine** for the one undo slot, **data flow** for a bury and for a flag from the
learner's gesture to the restored card, and a **sequence** for the sync. Every `path:line` in this
section was read at DeckStreak `dev` `e7ecf10d`. Names after this delivery are those SPEC-383 R1 to
R16 give. Sections 1 to 6 above are unchanged and still describe the undo of an answer.

### 7a. The one slot

The slot is two `thread_local!` cells in `crates/web-engine/src/wasm.rs`, the shipped `LAST_ANSWER`
(`:82`) and the new `LAST_MARK`, and at most one of them is set.

```mermaid
stateDiagram-v2
  state "nothing recorded" as idle
  state "LAST_ANSWER holds the answer" as answered
  state "LAST_MARK holds the bury or the flag" as marked
  [*] --> idle
  idle --> answered: rate records, when the newest review is of the rated card
  idle --> marked: bury or flag records, when the CardMark read shows the change
  answered --> answered: another answer replaces it
  answered --> marked: bury or flag records and clears LAST_ANSWER
  marked --> marked: another bury or flag replaces it
  marked --> answered: rate records and clears LAST_MARK
  answered --> idle: open, close, or a successful undo
  marked --> idle: open, close, or a successful undo
```

### 7b. A bury

```mermaid
flowchart TD
  bury["learner's bury: the bar's Bury button"] --> callB["wasm.rs bury: BuryOrSuspendCards 13,14 with one card in the user's mode"]
  callB --> readB{"CardMark read: is the card user-buried?"}
  readB -- no --> noneB["nothing recorded - the slot is as it was"]
  readB -- yes --> keepB["LAST_MARK: bury, the card, the state it returns to, and the undo status read through 3,7 - LAST_ANSWER cleared"]
  keepB --> viewB["the next card's view: undo is bury"]
  viewB --> pressB["press: the Undo bury button, key u, or the remote's button 4"]
  pressB --> offerB["undo_offer: judge_change on 3,7 and the CardMark read"]
  offerB -- refused --> noticeB["the change's notice - nothing is written"]
  offerB -- admitted --> dialogB["alertdialog: Undo this bury?, the card's text, Goes back to, focus on Keep it"]
  dialogB -- "Keep it, Escape, or any other action" --> keptB["nothing is written"]
  dialogB -- "Undo bury, or the undo action again" --> undoB["wasm.rs undo with the offer's card and step: last_mark_for matches both"]
  undoB --> gestureB["the owner's gesture for Undo on Target Card, checked"]
  gestureB --> kindB["run_undo reads the kind: a bury goes to run_restore"]
  kindB --> checkB{"judge_change at the write"}
  checkB -- refused --> refusedB["NotTheTarget - nothing is written - the page shows the change's notice"]
  checkB -- admitted --> writeB["Undo 3,8: the card's row before the bury returns, its queue included"]
  writeB --> clearB["LAST_MARK and the kept card cleared - the next card request can show the card again"]
```

### 7c. A flag

```mermaid
flowchart TD
  flag["learner's flag: the bar's Flag button"] --> callF["wasm.rs flag: SetFlag 5,4 with toggled_red of the kept flag"]
  callF --> readF{"CardMark read: is the card's user flag the toggled flag?"}
  readF -- no --> failF["flag answers an error - nothing recorded"]
  readF -- yes --> keepF["LAST_MARK: flag, the card, the flag before, the flag after, and the undo status read through 3,7 - LAST_ANSWER cleared"]
  keepF --> viewF["flag answers the new flag - the page sets the view's undo to flag"]
  viewF --> pressF["press: the Undo flag button, key u, or the remote's button 4"]
  pressF --> offerF["undo_offer: judge_change on 3,7 and the CardMark read - the offer says added, removed or replaced"]
  offerF -- refused --> noticeF["the change's notice - nothing is written"]
  offerF -- admitted --> dialogF["alertdialog: Undo this flag?, the card's text, the added, removed or replaced line, focus on Keep it"]
  dialogF -- "Keep it, Escape, or any other action" --> keptF["nothing is written"]
  dialogF -- "Undo flag, or the undo action again" --> undoF["wasm.rs undo with the offer's card and step: last_mark_for matches both"]
  undoF --> gestureF["the owner's gesture for Undo on Target Card, checked"]
  gestureF --> kindF["run_undo reads the kind: a flag goes to run_restore"]
  kindF --> checkF{"judge_change at the write"}
  checkF -- refused --> refusedF["NotTheTarget - nothing is written - the page shows the change's notice"]
  checkF -- admitted --> writeF["Undo 3,8: the card's row before the flag returns, the earlier flag included"]
  writeF --> clearF["LAST_MARK and the kept card cleared - the review asks for its card again"]
```

### 7d. The check at the write

`judge_change` (`crates/engine-core/src/undo_change.rs`) answers the first refusal that holds, in
this order. `judge` (`crates/engine-core/src/undo_answer.rs:71-98`) still judges an answer, unchanged.

```mermaid
flowchart TD
  judgeIn["judge_change: the record, the undo status read through 3,7, the CardMark read, the target card"] --> row{"does the card have a row?"}
  row -- no --> gone1["Gone"]
  row -- yes --> cardCheck{"is the record's card the target?"}
  cardCheck -- no --> other["NotTheCard"]
  cardCheck -- yes --> still{"bury: is the card user-buried? flag: is its user flag the recorded flag?"}
  still -- no --> changed1["Changed"]
  still -- yes --> synced{"is the card's usn -1?"}
  synced -- no --> sent["Synced"]
  synced -- yes --> queue{"does the engine have anything to undo?"}
  queue -- no --> gone2["Gone"]
  queue -- yes --> step{"is the last step the record's?"}
  step -- no --> changed2["Changed"]
  step -- yes --> labelCheck{"is the undo label the record's?"}
  labelCheck -- no --> changed3["Changed"]
  labelCheck -- yes --> admit["admitted: run_restore runs Undo 3,8"]
```

### 7e. The sync

```mermaid
sequenceDiagram
  participant L as learner
  participant P as page
  participant W as Worker
  participant E as engine
  L->>P: bury or flag
  P->>W: bury or flag with the card
  W->>E: the change, then GetUndoStatus 3,7 and the CardMark read
  Note over W: LAST_MARK records the change, and the card's usn is -1
  alt a normal sync before the undo
    W->>E: SyncCollection
    Note over E: the change is sent, the card's usn leaves -1, and the undo queue is discarded
    L->>P: Undo
    P->>W: undoOffer
    W->>E: GetUndoStatus 3,7 and the CardMark read
    W-->>P: no offer, why synced
    Note over P: the control is disabled and says the last change has synced
  else no sync before the undo
    L->>P: Undo, then the dialog's confirmation
    P->>W: undo with the offer's card and step
    W->>E: the owner's gesture, judge_change, then Undo 3,8
    Note over E: the card's row before the change returns, its usn included, so the next sync has nothing to send for it
  end
```

### 7f. Who can reach it

| client | bury and flag | undo of a bury or a flag |
|---|---|---|
| the web review | `wasm.rs` `bury` and `flag`, which keep the slot | through `undo_offer` and `undo`, the owner's gesture and `run_restore` (SPEC-383) |
| the native clients | `crates/ffi/src/engine.rs` `bury` (`:214-218`) and `flag` (`:226-231`) | none: the native `run` refuses (3,8), and no native undo exists (#714) |
| the server and the bot | no crate writes either | none; the bot's own habit undo is unchanged (#714) |

### 7g. What carries state, added to section 6

- `LAST_MARK` (new, `wasm.rs`): the review's last bury or flag, its card, its kind, the engine's
  undo status when it was made, and, for a bury, the state the card returns to, for a flag, the
  flag before and after. `bury` and `flag` write it after their write, only when the `CardMark`
  read shows the change, and clear `LAST_ANSWER`; `rate` clears it when it records an answer.
  `current_card`, `undo_offer` and `undo` read it; a successful undo, `open` and `close` clear it.
- `Recorded` (`undo_answer.rs`): gains `kind`, `flag` and `card`. A record with no kind is an
  answer, so the bytes of the undo of an answer are read as before.
- The card's row: its queue, flags and usn, read through the new fixed read `CardMark` when the
  change is recorded, at the offer, and again at the write.
- The model `formal/tla/UndoOwnAnswer` gains `mark`, `markOffers`, `markConfirmed` and `restored`,
  and covers `judge_change`, `run_restore`, `bury`, `flag` and `current_card` beside its shipped
  covers.
