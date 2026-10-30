---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# FSRS-7 runs live on one preset, rebuilt from review history into the stock fields

## Context and Problem Statement

The owner chose that FSRS-7 schedules live on one experimental preset in the study client, since
this client becomes the owner's primary Anki client (SPEC-334 R10; the owner's answer OQ1), and
that its state is rebuilt from review history (the follow-up answer). FSRS-7 tracks more state
than FSRS-6: a slow and a fast stability beside difficulty, with 34 parameters. It exists only on
the scheduler crate's development branch, in no released crate and in no Anki release, and that
branch also changes FSRS-6's defaults. A stock sync server keeps only the stock card-data fields,
so FSRS-7's extra state does not survive an upload, and a review of that preset on desktop or in
AnkiMobile overwrites the stock fields with FSRS-6's result. How does FSRS-7 run live without
breaking any other client, the sync server or the normal scheduler?

## Decision Drivers

- Every write survives a stock sync server and reads correctly in every stock client.
- The normal scheduler stays stock FSRS-6, on the released crate, unchanged by FSRS-7's branch.
- The review history stays the one source of truth, as ADR-301 (a) entry 1 protects it.
- The switch of a preset to FSRS-7 is a bulk rewrite of memory state, admitted only as the
  owner's tap (ADR-337, entry 5).

## Considered Options (the alternatives it was chosen against)

- Live on one preset, its state rebuilt from each card's review history at open and written only to the stock fields — chosen because the stock fields survive a stock sync server and every stock client, no server patch is carried, and the only writes are the owner's answers and the one switch.
- Shadow-only, FSRS-7 computed beside FSRS-6 and never scheduling — rejected because the owner chose live scheduling on one preset (OQ1).
- A patched sync server that keeps FSRS-7's extra card-data keys — rejected because it forks the server's storage, desktop and AnkiMobile still overwrite the state with FSRS-6's, and the patch must be carried on every Anki bump.
- FSRS-7's parameters stored in the FSRS-6 parameter field — rejected because desktop and AnkiMobile would read them as FSRS-6 parameters and schedule the preset wrongly.

## Decision Outcome

Proposed option: live on one preset, rebuilt from review history, written to the stock fields.

- **The isolated crate.** FSRS-7 is built in its own crate, at a pinned git revision of the
  upstream scheduler crate, admitted by its own `allow-git` entry. It never links into the
  engine, and the engine's FSRS-6 stays on the released crate, pinned by checksum. Both versions
  coexist through cargo's per-version disambiguation, inside the one umbrella library (ADR-335)
  and the one WASM build (ADR-336).
- **Rebuilt, never stored.** When the collection opens, the client replays each card's review
  history on the FSRS-7 preset into FSRS-7 state in memory. It writes only the stock fields: the
  due date, the stability expressed as the 90 percent stability the stock field holds, and the
  difficulty. Nothing else is written, so a stock sync server, desktop and AnkiMobile read nothing
  new. The replay time is measured in Phase 0 against the collection's size.
- **Other clients.** A review of the preset on desktop or in AnkiMobile writes FSRS-6's result
  to the stock fields; the next open replays the history, including that review, so the history
  stays the source of truth.
- **The switch.** Moving a preset to FSRS-7 rewrites every card's memory state on it, which is
  never-list entry 5; it is the owner's tap under ADR-337, shown and confirmed first. The preset
  screen that offers it is a later parity screen; the crate and the replay come first.
- **What the study client shows (SPEC-334 R18).** The study client shows the scheduler's own
  output as Anki does: the next interval on the answer buttons, a card's due date, the browser's
  Due column and the future-due forecast, from FSRS-7 on its preset and from the stock scheduler
  everywhere else. This ADR, with ADR-339, carries the amendment of the PRD's third non-goal and
  the scheduler sentence of CHARTER 4 recorded in
  `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`.

### Consequences

- Good, because FSRS-7 schedules live on the owner's chosen preset with no change to the sync
  server, the stock clients or the normal scheduler.
- Good, because the review history alone determines FSRS-7's state, so nothing can drift from it.
- Bad, because every open replays the preset's history, which costs time the Phase 0
  measurement bounds.
- Bad, because the FSRS-7 crate pins an unreleased revision, which DeckStreak re-pins by hand.

### Confirmation

- The replay measurement in Phase 0, native and on WASM.
- A test that the preset's writes touch only the stock fields, read back through a stock sync
  round trip.
- `cargo deny` with exactly the FSRS-7 crate's `allow-git` entry added.

## What would make this wrong

- The replay takes too long at open for the owner's collection, which the Phase 0 measurement
  reads; a cache of FSRS-7 state keyed to the history would then need its own decision.
- An Anki release carries FSRS-7 natively, which would replace the isolated crate and supersede
  this ADR.

## More Information

- SPEC-334 (R2, R10, R18; stretch row 2.4).
- `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md` (entry 5) and
  `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md` (CHARTER 4, the PRD's third non-goal).
- ADR-301 (a), ADR-337, ADR-339.
