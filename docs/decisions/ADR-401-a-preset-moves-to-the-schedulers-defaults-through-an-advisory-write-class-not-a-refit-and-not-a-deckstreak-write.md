---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-401: a preset moves to the scheduler's defaults through an advisory write class, not a refit and not a DeckStreak write

## Context and Problem Statement

The collection's main preset is to move to the current scheduler generation's default parameters.
It is a preset change, not a refit. A full refit is later work. Five other required behaviours
were handed to the same design, each to be stated by the SPEC that carries it. SPEC-387 section 1
holds the measurements, each with its `file:line` at `dev` `3ca06142`. In short:

- **The current generation.** It is FSRS-6. Its defaults are the released package's
  `DEFAULT_PARAMETERS`, 21 values (`crates/fsrs7/tests/coexistence.rs:48-52`). FSRS-7 is
  unreleased and isolated in its own crate (ADR-338).
- **DeckStreak and presets today.** DeckStreak neither reads nor writes a preset. The engine core
  refuses `UpdateDeckConfigs` (ADR-356). Ingest's one write port belongs to the skip day
  (ADR-321 D14).
- **The write rule.** ADR-301 counts a preset's parameters as a write, starts every class on the
  advisory rung, and puts mass-reschedule on its never-list. The no-dwell ruling (#744) gives a
  parameters-only class a dwell of none.
- **What a save does.** Saving new parameters makes the engine recompute the memory state of every
  non-new card in the preset's decks, even with "Reschedule cards on change" off.
- **The privacy page.** It pins a paragraph that says DeckStreak fits nothing (SPEC-368), and it
  must change in any delivery that changes what DeckStreak does with those parameters.

The questions:

- What carries each of the six behaviours?
- Which delivery carries the preset move, and in what shape?
- What does the move reach: which clients, which store, which history?
- How are its tests ordered, and does it need a formal model?

## Decision Drivers

- ADR-301's rule binds every write to the collection. A class starts advisory, and it climbs only
  when its own ADR and SPEC are accepted.
- The never-list binds at every rung, so no path may reschedule a preset's cards.
- The move must be the same on every client that schedules the preset's cards.
- The move must be undoable from a record, not from memory.
- A test must decide each requirement red first, and a guard must carry a plant.
- The FSRS-7 replay's delivery depends on the unreleased crate, and this change must not.

## Considered Options (the alternatives each was chosen against)

### D1. What carries each of the six behaviours

- One SPEC restating all six: rejected because four already have a SPEC or a planned build whose text must state them, and a second record of one rule splits it across two files that can drift.
- One docs delivery per behaviour: rejected because the three text-only amendments (the AI route, handwriting, and nothing for the MCP grant) are insert-only and small, so one docs delivery reviews them together; only the two that change code take deliveries of their own.

### D2. Which delivery carries the preset move

- The FSRS-7 replay's delivery: rejected because it depends on the unreleased FSRS-7 crate and its replay, while the move needs only the released package, so tying them would hold a small change behind a larger one.
- An approval-rung class in which DeckStreak writes the vector itself: rejected because ADR-301 starts every class on the advisory rung until its own ADR and SPEC are accepted, and the engine core refuses the only engine call that saves a preset.
- A note to the owner with no code: rejected because it keeps no undo values, records no change point for the later refit, and leaves the move unverified.

### D3. The reach of the move

- Turning "Reschedule cards on change" on so that intervals follow the defaults at once: rejected because it writes a review-log row and a new due date for every card in the preset, which is a mass-reschedule on ADR-301's never-list.
- Moving every preset at once: rejected because the move is asked of the main preset, and each preset is its own unit of change under ADR-301's dwell rules.

### D4. The tests and the formal decision

- Tests written after the code: rejected because a test that never failed proves nothing about the behaviour it names.
- A TLA+ model of the proposal's states: rejected because every check and its act sit in one immediate transaction behind a partial unique index or a guarded update, so the database serialises the only interleaving, and A5 and A6 drive it directly.

### D5. The shape

- An insert-only amendment of an existing SPEC: rejected because no SPEC on `dev` holds a manifest that admits a new ingest module, a migration, a host role and the privacy page together; SPEC-368 changes no code, and SPEC-342 and SPEC-345 change no ingest file.
- Two deliveries, the read first and the proposal second: rejected because the read alone changes no behaviour a user sees, and the privacy amendment belongs with the record it discloses.

### D6. Where the default vector comes from

- A literal copy of the 21 values in ingest: rejected because ADR-338 records that the upstream branch changes FSRS-6's defaults, so a literal drifts silently from the engine's own when the pin moves.
- FSRS-7's crate: rejected because it is the next generation on an unreleased revision, not the generation the engine schedules with.
- A normal dependency of ingest on workspace `fsrs6`: rejected because it widens the FSRS-7 pin's released set (SPEC-342 R2), which is presumed a weakening, when the engine ingest already depends on reports the same package's defaults through its deck-options read.

### D7. An explicit vector, not a cleared box

- Clearing the preset's parameters box, so that each engine uses its own defaults: rejected because each client's engine then schedules with the defaults of its own scheduler package, and ADR-338 records that those defaults are changing, so two clients could schedule the same cards differently.

### D8. A record of each proposal

- Printing the proposal and keeping nothing: rejected because the prior vector, which is the only undo, would live only in a terminal, and the later refit would have no change point.
- A file in the owner's vault: rejected because the vault sits outside the database's export, erase and backup, and so outside the declared data rights.

### D9. A read port of its own

- A method on `AnkiEngine`: rejected because ten implementors would each need it, most of them test fakes that have no use for presets.
- A method on `CollectionWrite`: rejected because that is the write port, census A24 holds it to the skip day's write module, and a read must never carry a write capability.

### D10. A host role

- A web or iOS screen: rejected because the clients' preset screens belong to later parity phases (#611), and a host role is the owner's own act, as the `data` role is.
- A bot command: rejected because a preset's parameters are not daily study, and a command would widen the bot's surface for a one-off change.

### D11. The privacy paragraph rides this delivery

- A separate docs delivery for the page: rejected because SPEC-368 R3 binds a change in what DeckStreak does with the parameters to an amendment in its own delivery, and this delivery adds the record the page must disclose.
- Keeping "does not fit a model": rejected because it does not say which kind of model is never trained, and it reads as denying that the parameters are a fit at all, when they are the user's own scheduler's fit or its defaults.

### D12. The class's form under ADR-301

- No class at all, since the advisory rung writes nothing: rejected because a later rung then starts from no accepted ADR, and ADR-301 counts a preset's parameters as a write whoever saves them.

## Decision Outcome

### D1. The carriers

| behaviour | carrier | how |
|---|---|---|
| The AI feature runs on the one AI route; no second arm keyed by its own credential, interim or fallback | SPEC-043 (R15) and SPEC-334 (R16, stretch row 2.5), with a note on ADR-054 | insert-only amendments in one docs delivery. The code already has no such arm (`crates/agent/src/route.rs:13-18`) |
| Retention rewards are keyed to each preset's own desired retention | SPEC-073 (R6), amended by a delivery of its own | it changes code and the badge constants' golden, which depart from parity. This SPEC's preset read gives it each preset's desired retention |
| The privacy page says that no neural network or language model is trained, and that the scheduler's parameters are a fit of the user's own scheduler | SPEC-387 R10, which amends SPEC-368 | this delivery |
| The MCP server offers no export or erase tool, and its grant is read or write | SPEC-119 section 17 (T19-T21) and SPEC-369 | none: both already state it, and the code holds it |
| The main preset moves to the current generation's defaults, as a preset change and not a refit | SPEC-387 | this delivery |
| The tablet joins the device family with the pen and handwriting input; the phone gets draw-and-compare with no machine learning, and a deterministic stroke matcher in Rust comes later | SPEC-334, by an insert-only amendment in the same docs delivery as the first row | the later handwriting build's SPEC carries the behaviour. The device family already includes the tablet (ADR-342) |

Chosen against: D1 above.

### D2. The carrier

The move has its own delivery, SPEC-387. It declares a write class, "a preset's parameters", on
ADR-301's advisory rung:

1. DeckStreak reads every preset.
2. It proposes the released defaults for the preset the owner names.
3. It records the proposal with the values it would replace.
4. The owner applies it in their own Anki app, under Anki's own guards.
5. DeckStreak settles the proposal once the change has synced back.

DeckStreak writes nothing to the collection.

Chosen against: D2 above.

### D3. The reach

| what | across the move |
|---|---|
| the store | the preset's deck config in the collection on the owner's sync server |
| who changes it | the owner's Anki app, on save |
| how it travels | the next sync from the owner's app is incremental. It carries the deck config and the card rows whose memory state the save recomputed. No full sync is asked |
| when other clients see it | at each one's next sync. DeckStreak's private copy sees it at its daily sync, and the web and iOS clients and any other Anki app see it at theirs. Deck configs merge last writer wins, so an edit of the same preset elsewhere before that sync replaces the move whole |
| review history | unchanged: no review-log row is added, changed or removed |
| due dates, intervals, queues | unchanged ("Reschedule cards on change" stays off) |
| desired retention and every other option | unchanged |
| the parameter vector | replaced by the 21 released defaults |
| memory state | recomputed by the engine for every non-new card in the preset's decks, from that card's own reviews under the new vector. Each card's next interval follows from it at its next review |
| the undo | the owner pastes the prior vector from the record, or clears the box when the record says it was empty. Memory state is recomputed again under the prior vector. It equals the state before the move whenever that state was itself a recompute from the full history under the same vector |

Chosen against: D3 above.

### D4. The tests and the formal decision

- **Red first.** Every criterion of SPEC-387 section 3 is red at a commit that holds stubs. Each
  stub keeps every input except the missing behaviour:
  - the read returns no preset;
  - propose records every call, with the stored vector as the proposed one;
  - the red migration has no partial index;
  - verify settles nothing;
  - the text and the listing are empty;
  - the role parses no `preset` command;
  - the data-rights port declares no new table.
- **Guards.** A7 (zero uploads against the recording fake sync server, with the copy's sha256
  unchanged), A8 (a census of write methods) and A14 (the unchanged pin) are guards. Each has a
  planted control that is refused first.
- **Rows.** They run from S38700 (SPEC-387 section 8).
- **Formal.** None, by surface (SPEC-387 section 7). No `@phx covers` line names a file in its
  manifest.

Chosen against: D4 above.

### D5. The shape

One delivery under SPEC-387 and this ADR.

Chosen against: D5 above.

### D6. The default vector's source

Ingest reads the released package's `DEFAULT_PARAMETERS`, the ones the engine schedules with,
through the engine it already depends on: the engine's deck-options read,
`Collection::get_deck_configs_for_update`, returns them as its `defaults`. Ingest names no
scheduler package, so the FSRS-7 pin's released set is unchanged. SPEC-342 records this in an
insert-only section, and `PINNED_USERS` is unchanged.

Chosen against: D6 above.

### D7. An explicit vector

The proposal gives all 21 values, printed so that each parses back to the same value.

Chosen against: D7 above.

### D8. The record

`preset_proposals` holds each proposal with its undo values. It is declared in `privacy.json` as
`preset-proposals`, named on the privacy page, and exported and erased by ingest's data-rights
port. Its partial unique index holds one open proposal per preset. Verify's observed time is the
preset's change point, which a later refit's held-out window reads.

Chosen against: D8 above.

### D9. The read port

`PresetRead` is a trait in ingest's new preset module, implemented by `RslibEngine` only. It reads
under the collection's shared lock.

Chosen against: D9 above.

### D10. The host role

The role is `deckstreakd preset list | propose <preset id> | verify <proposal id>`.

Chosen against: D10 above.

### D11. The privacy paragraph

SPEC-387 R10's paragraph replaces SPEC-368 R2's, in this delivery.

Chosen against: D11 above.

### D12. The class, in ADR-301's form

| what ADR-301 asks | this class |
|---|---|
| the exact change, and its exact inverse | one preset's parameter vector replaced by the released defaults, after which the engine recomputes its non-new cards' memory state. The inverse restores the recorded prior vector, or clears the box |
| what it may move | the memory state of the preset's non-new cards. Review rows move by 0, and due dates by 0 |
| its guardrails, each an acceptance criterion planned red first against the recording fake sync server | on this rung DeckStreak's own change is zero: SPEC-387 A7 and A8 |
| its undo | the record's prior vector (D3) |
| its rung | advisory. The approval rung needs its own SPEC to be accepted, with ADR-301's backup and restore drill, counts and kill switch |
| its dwell | none (#744) |
| its formal decision | not applicable, by surface (D4) |
| the never-list | not touched. "Reschedule cards on change" stays off, so no card is rescheduled |

Chosen against: D12 above.

### Consequences

- Good:
  - The main preset can move to the released defaults now, on the owner's own save, with a
    recorded undo and a recorded change point.
  - Every client schedules with the same 21 values.
  - The privacy page says what is and is not trained.
  - The retention-rewards delivery gains each preset's desired retention.
- Bad:
  - Published benchmarks find that a fitted vector beats the defaults for most collections, so the
    move can worsen predicted recall until the later refit.
  - The owner pastes the values by hand.
  - The defaults come through an engine read built for the deck options screen, so an engine
    upgrade that changes that read changes the preset module with it.
- Neutral:
  - The class sits on the advisory rung until its approval-rung SPEC is accepted.

### Confirmation

- SPEC-387's fifteen criteria, red first, recorded in `docs/red-first/SPEC-387.md`.
- The band's rows, killed by CI's own runner.
- `scripts/tests/test_privacy_policy.py`, as amended, and `scripts/tests/test_fsrs7_pin.py`,
  unchanged.

## What would make this wrong

- A measurement that the engine, with "Reschedule cards on change" off, still moves a due date or
  writes a review row when it saves parameters. The move would then be a reschedule, and it stops.
- A measurement that the engine reads the parameter fields in another order than SPEC-387 R2
  says.
- An owner ruling that brings a parameters-only save under the never-list's mass-reschedule entry
  even when no due date moves.
- A released scheduler package whose defaults differ from the ones the owner's Anki app uses. The
  explicit vector would still hold every client to one set, but "the defaults" would then need
  naming by package.

## More Information

- ADR-301, parts (b) and (c), the never-list and the mass-reschedule definition.
- ADR-321 D14.
- ADR-338.
- ADR-356.
- SPEC-342 R2.
- SPEC-345.
- SPEC-368 R2, R3, R6 and R7.
- The owner's no-dwell ruling (#744).
- `docs/schematics/scheduler-presets-and-their-parameters.md`.
- #514, #611, #623, #641, #721, #722.

## Amendments

### D13. A read that would rewrite the copy's timing

Chosen: the preset read refuses, before any engine read, when the copy's configured UTC offset differs from the process zone or its rollover hour is unset, as the skip's own facts check refuses a copy it cannot read without a write.

Rejected: (b) open the copy in the engine's server mode, which skips the rewrite but needs a second engine entry point that the manifest does not admit; (c) let the read rewrite the timing and disclose it, which breaks R8's promise that every preset path leaves the copy unchanged.
