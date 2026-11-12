---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-382: Undo reverts only the review's own last answer, as an exempt write behind the owner's gesture, checked at the write after the review shows what it changes

Decides SPEC-371 (issue #714). It is built on the owner-taps ruling: an exempt tap is reachable
only from the UI, and that is proven; it makes one gesture's own write on the card it names; and
the write is shown before it is made, with what it changes. That ruling settled whether; this
record decides how.

**Amends, by an insert-only amendment section appended to each ADR, in the form SPEC-365's
sections take, with no existing line changed (the sections' text is at the end of this record):**
- ADR-356 D2 (the table): Undo leaves the ordinary rows and joins the exempt table; HtmlToTextLine
  joins the ordinary rows, web only.
- ADR-356 D4 (containment): the census names `undo` and holds its four lines outside the core.
- ADR-356 D6 (the first exempt table): the table holds eight rows.
- ADR-361 D1, for undo only: its rejection of a token for undo gives way for the review's own
  last answer; bury and flag stay ordinary.
- ADR-337's Decision Outcome: the exempt table also holds the undo of an unsynced answer.

**Adds** a dated note to ADR-301 at `:107` naming the owner-taps ruling, only if the build's base
holds none (`dev` `2438c4c6` holds none). **Leaves** ADR-348 (the new exports sit inside its
boundary, as SPEC-350's did) and ADR-376 (its `Grade` is reused unchanged).

## Context and Problem Statement

At `dev` `2438c4c6`, which holds SPEC-365 and SPEC-366, four doors reach Undo (3,8): the core's
`run` on both transports, where Undo is an ordinary row (`table.rs:144-150`); the native `run`
(`allow_list.rs:61-65`); the web's `run_method` (`study.rs:82`); and the web's `undo` export
(`wasm.rs:252-259`), which calls (3,8) with an empty request. The engine reverts the front of its
undo queue whatever it is, so after a bury the review's Undo reverts the bury. The review's own
control writes on the press (`review.ts:42`, `:283-284`), and the page reads the engine's undo
label only as a boolean (`review.ts:177`, `:200`), so nothing on the screen names what the press
reverts. No UI yet calls any exempt write: Undo is the first exempt tap built.

## Decision Drivers

- An undo is reachable only from the review's UI, and the census proves it (the ruling's first
  condition).
- An undo reverts one gesture's own write on the card the gesture names, and nothing else (the
  second).
- The undo is shown before it is made, with what it changes, and is made only when the learner
  confirms (the third).
- The guarantee is the compiler's, the census's and the check at the write, not a convention in a
  caller.
- No foreign type changes for a refusal no native tap can reach.

## Considered Options (the alternatives each was chosen against)

### D1. The token

- Undo joins `EXEMPT` as `ExemptWrite::Undo` (3,8) "CollectionService.Undo", `TargetKind::Card`, under the existing `OwnerGesture`, and leaves `ORDINARY` — chosen, because the census already holds `OwnerGesture` to the two entry files, so Undo inherits a containment that is already proven.
- SPEC-365's `OwnerAnswer` — rejected, because it names a grade and a `CardAnswer` for the answered set, so one token would mint two writes.
- A token of its own — rejected, because it needs a second census beside `GESTURE_NAMES`, while the ruling's containment already holds `OwnerGesture` to the two entry files.
- No token, the UI alone gating the call — rejected, because `run_method` admits (3,8) to any script in the Worker, the reach #714 measured.

### D2. What an undo reverts

- The review's own last answer only — chosen, because the core binds an answer to its card by the engine's own review row, which it can read back.
- Any last change: an answer, a bury or a flag — rejected, because a bury or a flag leaves no row the core can read back without new reads.
- The answer exempt, with bury and flag undo kept ordinary — rejected, because (3,8) is one method that reverts whatever is last, so an ordinary row would reach the answer too.

After a bury or a flag, Undo is therefore not offered; a flag toggles back by itself.

### D3. Where the check lives

- A new pure module, `undo_answer.rs`, whose `judge` decides in a fixed order (no review row: `Gone`; another card: `NotTheCard`; a `usn` that is not -1: `Synced`; an empty undo label: `Gone`; a different `last_step` or label: `Changed`), run at the write by a private `run_undo` that `run_exempt` hands the `Undo` arm of `OwnerGesture::checked`, refusing as `GestureRefusal::NotTheTarget` and running (3,8) with an empty request — chosen, because the check sits where the write is made, and no foreign type changes.
- A module named `undo.rs` — rejected, because a `::undo` path in a use tree would be counted once `undo` joins the census's engine names.
- A new `GestureRefusal` variant — rejected, because the native adapter's exhaustive `exempt_refusal` and the foreign `ExemptRefusal` would change for a refusal no native tap can reach.
- A public `run_undo` — rejected, because it would be a second exempt entry for the census to hold.

`OwnerGesture::checked` answers an enum, `Run(service, method, request)` or `Undo { card,
recorded }`, and its `Undo` arm decodes the `Recorded` message (`status: UndoStatus`, tag 1;
`review: i64`, tag 2) or refuses `Undecodable`. Two fixed reads serve it: `Read::NewestReview` and
`Read::Review(i64)`, beside `Read`'s two (`dispatch.rs:89-94`).

### D4. Where the record lives

- A `thread_local!` in `wasm.rs`, `LAST_ANSWER: RefCell<Option<LastAnswer>>`, set by `rate` after `run_answer` succeeds and only when the newest review's card is the rated card, and cleared by a successful undo, `open` and `close` — chosen, because the Worker's entry file is the one place that saw the press and the answer it made.
- The page holding the record — rejected, because the page could read (3,7) after a bury and forge one.
- A record in the core's `Dispatcher` — rejected, because it would change SPEC-365's `run_answer` and share state across every clone, the native one included.
- The name `ANSWERED` — rejected, because SPEC-365's set already holds that name.

`LastAnswer { card, grade, returns, recorded }` lives in `study.rs`, where it is tested natively, and
its `grade` is the study rule's `Grade` (`study.rs:14`, SPEC-365 R7).

### D5. The web exports

- `undo_offer()`, which answers the offer as JSON (card, step, text, grade, the state it returns to) or no offer with its reason (`synced` or `none`), and `undo(card, step)`, which refuses `NotUndoable(Changed)` unless `(card, step)` equal the record's, runs `judge`, mints the gesture for `Target::Card(card)` and calls `run_exempt` with the encoded record, then clears `LAST_ANSWER` and `SHOWN` — chosen, because the page confirms only what it was shown, and the Worker refuses anything else.
- `dev`'s `undo()` with no argument — rejected, because it reverts the front of the queue whatever it is (`wasm.rs:256`), which the model's witness `an-undo-reverts-only-the-offered-answer` (named `a-confirm-with-no-re-check` in the draft) ports.
- Undo as a sixth index of the `run_exempt` export — rejected, because that export takes a write by number from any script in the Worker (`wasm.rs:272-289`), with no record behind it.

`current_card`'s `undo` becomes `"answer"`, `"synced"` or `null`, by `judge`. A refusal is
`StudyError::NotUndoable(UndoRefusal)`, which the session maps to the error codes `undo-synced`
(`Synced`) and `not-undoable` (every other refusal).

### D6. How the card is shown

- The card's question rendered by (27,6), its nodes joined, then (27,9) StripAvTags, then (27,14) HtmlToTextLine with `preserve_media_filenames: true`, with (27,14) joining `ORDINARY` web only and `STUDY_CALLS` — chosen, because it shows the card as one line of plain text from the engine's own renderer, with no second frame.
- The page's kept view in a second sealed frame — rejected, because it displays from the reach the ruling distrusts and adds a second frame-policy surface.
- `faces()` for a card that is not shown — rejected, because it widens ADR-361 D2's kept-card rule.
- No card shown — rejected, because it fails the ruling's third condition: shown, with what it changes.

The index 14 is inferred from the proto's order, not read from a build. The build measures it from
a build first, and a test pins it on known HTML.

### D7. How the review asks

- The press asks the Worker for an offer; an offer moves the review to `confirming`; the Undo action again (remote button 4, key `u`) or the dialog's "Undo answer" confirms; "Keep it", Escape and every other action keep the answer, and that other action is not carried out; a refused confirmation reloads with its notice — chosen, because the offer and its confirmation are how the write is shown before it is made, with what it changes.
- The write on the press, as `dev` does (`review.ts:42`, `:283-284`) — rejected, because the learner never sees what the press reverts before it is reverted.
- Enter or the primary key confirms — rejected, because it is the most-pressed key, so habit would undo.
- Hold to confirm — rejected, because no input reader has a hold gesture, and it adds a clock.
- A typed confirmation — rejected, because it is heavy for an answer that has not synced.

A key repeat is already dropped (`keys.ts:47`), and a press while the offer loads finds no cell in
busy.

### D8. What the screen shows

- In `confirming` the control bar becomes a `role="alertdialog"` region, labelled and described, with focus on "Keep it", showing the card as text, the answer (Again or Good), the state it goes back to in today's queue and the unsynced sentence, with two `min-h-11` buttons, "Undo answer" and "Keep it"; a synced answer disables the bar's button and says why; 15 keys in each of 7 locales — chosen, because each thing the undo changes is named before the learner confirms, and keeping is the default.
- Relabelling `study_undo` — rejected, because the mapping screen names the action by it (`ReviewScreen.svelte:180` is the bar's use), so the bar gains `study_undo_answer`.
- The card line through `{@html}` — rejected, because the review renders card content only inside the sealed `CardFrame` (`ReviewScreen.svelte:155`; the screen holds no `{@html}`; ADR-361), and this line sits outside it.
- Focus on "Undo answer" — rejected, because Enter on the focused button would then confirm, the key D7 rejects as a confirmation.

No text shames the choice to keep.

### D9. How undo is contained

- The Rust census names `undo` (one more engine name) and holds its four lines outside the core by name with their reasons (the bot's habit undo, the bot router's string literal, and two ingest fixture lines, counted 2 and 1), refusing a planted `col.undo();` by name, and holds each owed literal of `undo` in the boundary census that names the gesture under `BOUNDARY_CENSUS_LITERAL`, as the `run_exempt` export's and `rate`'s are; a new TypeScript census, `undo-reach.test.ts`, holds `.undo(` to 2 sites, `.undoOffer(` and `.undo_offer(` to 1 each and the `undo` effect to `confirming`, each population counted and refused at zero; a locale census holds the 15 keys — chosen, because each census's existing matcher already counts the forms an undo can take.
- Crate-aware matching — rejected, because it is a second matcher beside `names()`.

### D10. The native door

- `ALLOW_LIST` drops Undo, the native pair list drops (3,8), SPEC-336 A5 becomes a refusal, `ExemptTap` gains no variant and no Swift line changes — chosen, because no native screen calls (3,8): HarnessWire's `Requests.undo()` only builds bytes.
- A native undo tap, an `ExemptTap` variant — rejected, because no native screen offers undo (no iOS line names undo beyond the bytes builder at `Messages.swift:103-104` and its test at `RequestBytesTests.swift:60-61`), and a door with no caller is reach with no use.

Counts, before SPEC-365 (`dev` `119e4476`), at this record's base (`dev` `2438c4c6`) and after
this record: `ORDINARY` 18, 17, 17; `EXEMPT` 6, 6, 7; the native `ALLOW_LIST` 10, 9, 8;
`STUDY_CALLS` 16, 15, 15; the boundary census's `OWED` 29, 30, 31; the census's `ENGINE_NAMES`
22, 23, 24.

## Decision Outcome

D1 to D10 as chosen above. The core gains `undo_answer.rs` (`Recorded`, `Review`, `UndoRefusal`,
`Returns`, `judge`, `returns_to`), the Undo exempt row, `ExemptWrite::Undo`, the `Undo` arm of
`OwnerGesture::checked`, the private `run_undo` and two fixed reads. The web adapter gains
`LAST_ANSWER`, `undo_offer` and `undo(card, step)`, and (27,14). The native adapter loses (3,8).
The review asks before it writes, and the censuses hold undo to the review.

## Consequences

- Good: no door reverts anything but the review's own last answer, and that only after the
  learner has seen the card, the answer and the state it returns to, and confirmed.
- Good: the check is made at the write, so an offer that went stale between the press and the
  confirmation is refused, whatever changed it.
- Bad: undoing a bury or a flag is no longer offered (D2).
- Bad: the delivery adds state, `LAST_ANSWER`, and with it a formal model to keep.
- Neutral: the undo of a synced answer stays unbuilt, and the control says so.

### Confirmation

SPEC-371's A1 to A28, the censuses' plants, the mutation rows `S37101` to `S37117`, and the
model `formal/tla/UndoOwnAnswer` with its two witnesses,
`witness/an-undo-reverts-only-the-offered-answer.cfg` and
`witness/an-undo-runs-only-on-a-confirm.cfg` (old -> new: `a-confirm-with-no-re-check.cfg` ->
`an-undo-reverts-only-the-offered-answer.cfg`; `an-undo-on-the-press.cfg` ->
`an-undo-runs-only-on-a-confirm.cfg`).

## What would make this wrong

- If (27,14) is not HtmlToTextLine, the text row's index changes, and nothing else does; the build
  measures it first.
- If the owner rules that a bury or a flag is undone too, `Recorded` gains a kind and `judge` reads
  it; D2's rejected option returns.
- If a web sync lands that does not discard the engine's undo queue, `judge`'s `usn` check still
  refuses a synced answer, and the control's synced arm is reached on the web for the first time.
- If the census's `undo` name keeps meeting lines that are not an undo, crate-aware matching (D9's
  rejected option) is revisited.

## The amendment sections

Each section below is appended at the end of its ADR, after SPEC-365's amendment section where the
ADR holds one. No existing line of that ADR changes.

### Appended to ADR-356

```markdown
## Amendment: Undo reverts only the review's own last answer (SPEC-371)

ADR-382 amends D2, D4 and D6. The rest of each stands.

- **D2 (the table, `:150-152`).** Undo (3,8) leaves the ordinary rows and joins the exempt table
  as `ExemptWrite::Undo`, `TargetKind::Card`, deciding `NeedsGesture` on both transports.
  HtmlToTextLine (27,14) joins the ordinary rows, web only, so the review can show the card it
  would undo as one line of text. ADR-382 D1 and D6 decide both, and name what each was chosen
  against.
- **D4 (containment, `:156-160`).** The census's engine names gain `undo`, and its held lines gain
  four entries outside the core, each with its reason, and the boundary census's owed literals of
  `undo` that name the gesture (ADR-382 D9).
- **D6 (the first exempt table, `:164-166`).** The table holds eight rows. The eighth, Undo,
  reverts only the review's own last answer while it has not synced, and is checked at the write
  (ADR-382 D2, D3). The undo after a sync this record anticipated (`:192`) is still unbuilt.
```

### Appended to ADR-361

```markdown
## Amendment: a token for undo, for the review's own last answer only (SPEC-371)

ADR-382 amends D1, for undo only. The rest of D1 stands.

- **D1 (`:37-55`).** D1 rejected a token for undo (`:50-51`). For the undo of the review's own
  last answer that rejection gives way: Undo is an exempt write behind the owner's gesture, checked
  at the write against a record the Worker kept when the answer was made (ADR-382 D1 to D4). Bury
  and flag stay ordinary calls, as D1 decided, and Undo no longer reverts them.
- **D2 (`:57-72`)** is unchanged in effect: a confirmed undo still clears the kept card.
```

### Appended to ADR-337

```markdown
## Amendment: the undo of an unsynced answer (SPEC-371)

ADR-382 amends the Decision Outcome (`:41-44`), which names the exempt functions as the backend
calls that make writes 1 to 8. The exempt table also holds Undo, for the review's own last answer
while it has not synced. Whether that undo is the never-list's entry 1 is read two ways: the
owner-taps ruling's table maps entry 1 to undoing an answer after it has synced, and ADR-361 D1
says the never-list does not name undo. ADR-382 builds to the stricter reading: the undo meets all
three of the ruling's conditions whichever reading holds.
```

### ADR-301's dated note, only if the base holds none

Inserted at `:107`, insert-only, in the record's own note form (`- **Note (<date>, #<issue>,
<subject>):**`, as at `:102`), with the build day's date in place of `<build day>`:

```markdown
- **Note (<build day>, #714, the first exempt tap):** the first exempt tap is built, under the
  owner-taps ruling: Undo of the review's own last answer while it has not synced (SPEC-371,
  ADR-382). It is reachable only from the review, writes only on the card the gesture names, and
  is shown, with what it changes, before it is made.
```

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
