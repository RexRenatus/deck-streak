---
status: proposed
decision-makers: "the DeckStreak architect"
---

# The native column admits the undo status and the native adapter undoes through the core's judges

## Context and Problem Statement

ADR-382 made Undo (3,8) an exempt write that only the owner's gesture reaches, checked at the
write against the review's record of its own last answer, after the review shows what the undo
changes; ADR-397 restored a bury or a flag through the same door by the record's kind. The web
engine keeps that record in its `wasm32` module and asks the core's two rules about it. The native
column refuses the undo status (3,7), the native adapter has no undo, and the adapter links no
protocol crate, so it can neither read an undo status nor write the record the exempt Undo decodes.
SPEC-515 asks that the iPhone and iPad app undo the review's own last answer, bury or flag, as the
web does. This ADR decides where the native record, its judge and its words live.

## Decision Drivers

- One rule for both clients: `undo_answer::judge`, `undo_change::judge_change` and the door's
  judge at the write are the core's; the native side must reach them, never copy them.
- The adapter names no protocol type, and a new dependency would change the lock file and the
  graph census (ADR-521's driver).
- The app's Swift stays thin: the session alone imports the adapter, and every file keeps its
  decision budget; the review model is at its ceiling.
- The gesture on an exempt write is minted only in an adapter's entry file (ADR-356 D4).

## Considered Options

### Where the native record and its judge live

- In the core, as a module the adapter calls with plain values (`native_undo`): chosen, because it
  reads the newest review, the card's mark and the undo status through the core's own fixed reads
  and decoders, and judges with the same two rules the web engine calls.
- A Swift copy of the rule over the codec: rejected, because the rule would live twice, and Swift
  would judge.
- In the adapter, with the protocol crates as new dependencies: rejected, by the lock file and the
  graph census.

### How the native client reads the undo status

- Admit (3,7) in the native column and the allow-list: chosen, because the adapter's offer and the
  app's undo control both read it through `run`, as the web engine reads it through its column.
- A status read the core makes past the table: rejected, because the column would then not say
  what the native client calls.

### Where the dialog's words live

- In the adapter, beside the sync choice's sentences (ADR-521's neutral consequence): chosen,
  because the app then lays out lines it is given and decides nothing about an offer's kind.
- In Swift views: rejected, because each kind's line is a branch the views' budgets cannot hold.

## Decision Outcome

- **D1. The column.** (3,7) is `native: true`; the allow-list carries it (twelve pairs), and the
  parity test holds the two equal. (3,8) stays exempt on both transports.
- **D2. The record and its judge.** `crates/engine-core/src/native_undo.rs` records the review's
  last answer, bury or flag from the status the adapter read and the core's fixed reads, as the
  web engine's `rate`, `bury` and `flag` record theirs, and judges a record now with
  `undo_answer::judge` or `undo_change::judge_change`. It keeps the record as the core's
  `Recorded`, which the exempt Undo decodes, and hands the adapter its encoded bytes.
- **D3. The adapter's undo.** `crates/ffi/src/engine.rs` keeps one slot. `undo_offer` judges the
  slot now and answers the offer with its words or the sentence that says why none was made.
  `undo(card, step)` checks the confirmation against the slot, judges again, mints the gesture on
  the exempt Undo and runs it through `run_exempt`, where the core's `run_undo` or `run_restore`
  judges once more at the write.
- **D4. Containment.** The gesture on the exempt Undo is minted once in each of the two review
  doors; outside the core `.undo(` appears only at the adapter's own tests' held lines; in the app,
  only the session calls the adapter's undo, and only the review's undo control sends its gestures.

### Consequences

- Good: both clients judge an undo with the same rules, and the app adds one model, one control
  and one dialog within its budgets.
- Bad: the record's words and the flag's change (added, removed or replaced) are stated in the
  adapter beside the web's own, which the web engine's study rule keeps for the page.
- Neutral: the adapter's sentences are English, as the app's other review words are.

### Confirmation

SPEC-515's A1 to A6 and its mutation rows in S515.

## What would make this wrong

- The web engine's record moves into the core: the native slot and the web's then share one module.
- The app gains translated words: the adapter's words move to the app's catalogue, with the kind
  passed through.

## More Information

ADR-356, ADR-382, ADR-397, ADR-521; SPEC-358, SPEC-371, SPEC-383, SPEC-503.
