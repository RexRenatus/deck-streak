---
status: "accepted"
date: "2026-10-18"
decision-makers: "the owner"
---

# ADR-339 amendment: DeckStreak's own scheduler may set due dates

It amends ADR-339 and does not replace it. The owner accepts it by the signature on the commit that adds this file, together with `docs/rulings/OWNER-RULING-2026-10-18-rex-enters-deckstreak.md`.

## Context and Problem Statement

ADR-339 is the reorder-only rule: a model outside the engine may emit only reorder scores and never changes a due date. It keeps outside models from scheduling. The owner ruled that DeckStreak's own model may schedule if its training run is better than or equal to the FSRS-6 and FSRS-7 baselines, and that an amendment is owed for that. Rex is that model.

## Decision Outcome

1. **Rex may set due dates.** Rex is DeckStreak's own scheduler, its Rust core in this repository, and it may set due dates. It is the only scheduler from the release it ships in.
2. **ADR-339 still binds every other model.** Any model that is not DeckStreak's own stays reorder-only, never changes a due date, and is never vendored. That covers any outside model and any code, weights or constants derived from one.
3. **Rex schedules only under these conditions:**
   - Rex's state is a deterministic function of model version, review log and links, bit-identical on every platform.
   - The release passed the owner's prediction-ready and scheduling-ready bars.
   - Written permission for its training data is on file.
4. **A Rex upgrade is a delivery.** It re-runs that delivery's evaluation, and a user's history replays through the new version, so no state is migrated by hand.

### Consequences

- FSRS leaves live scheduling. The Anki to Rex and Rex to Anki converters keep a user's history portable: Anki keeps everything it can represent, and only Rex-only state is dropped on the way out.
- The engine's reorder-score path stays for outside models under ADR-339.

### Confirmation

A cross-platform state-hash test is the gate for condition 3's first point: the same model version, review log and links give a bit-identical state on iOS, WASM and the server.
