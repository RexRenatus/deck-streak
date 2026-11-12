# ADR-400: The FSRS-7 replay reads one statement through one engine-core seam and writes nothing

| field | value |
|---|---|
| status | accepted |
| spec | SPEC-386 |
| issue | #641 |
| builds on | ADR-338, ADR-353, ADR-356 (amended, insert-only, at D4) |
| measured at | dev `e7ecf10d` |

## Context

Issue #641 asks for FSRS-7 in an isolated crate at a pinned revision, and for a preset's memory state rebuilt from
review history into the stock fields, in the engine only: the live-preset switch is a later parity screen. ADR-338
decided that FSRS-7 runs live on one preset, rebuilt from review history and written only into the stock fields, and
that the switch is the owner's tap. ADR-353 placed the crate beside the engine and SPEC-342 timed its replay. What is
left is the selection of the rows, the projection into stock values, where the replay runs, and how far it reaches.

The facts this decision rests on, each read at `e7ecf10d`:

- `crates/fsrs7` depends on the upstream scheduler alone, at the revision `Cargo.toml:141` pins
  (`crates/fsrs7/Cargo.toml:14-21`), and `scripts/tests/test_fsrs7_pin.py:121` holds that list exact.
- The package registry's newest release of the scheduler has 21 parameters and no FSRS-7; the library documentation
  index has no FSRS-7 API. The pinned revision's source is the only record of the model.
- `crates/fsrs7/src/convert.rs:73` drops ease 0 and kinds 4 and 5, and nothing cuts a card's history at a reset.
- The engine core reads the collection by fixed statements, one per call (`crates/engine-core/src/dispatch.rs:27-83`,
  `:431-451`), and depends on no workspace crate (`crates/engine-core/src/lib.rs:41-42`; `docs/CONTEXT-MAP.md:37`).
- The native engine object is shared and holds no lock of its own (`crates/ffi/src/engine.rs:69-70`, `:87`), and every
  dispatcher method takes `&self`, so two calls from two threads can interleave between statements.
- The scheduler switch is unlisted (`crates/engine-core/src/table.rs:279-285`) and is the owner-taps ruling's
  mass-reschedule entry (`:16`), admitted only from the owner's tap on one preset (`:26-41`).

## D1. The crate and its pin, and what it was chosen against

The crate stays `crates/fsrs7` at the revision `Cargo.toml:141` pins, as ADR-353 D1 placed it; the tree confirms the
placement and nothing refutes it. The pin is recorded where it is read: the workspace manifest's revision, the
lockfile's source line, and the pin test. The registry was read for a release that carries FSRS-7 and holds none, and
the documentation index was read for an FSRS-7 API and holds none, so the build reads the model at the pin itself.

The crate's one dependency stays the pinned scheduler. No browser random-source feature is added to it: the
random-source library reserves that choice for applications, and the web engine's graph already makes it
(`scripts/web-engine-build.sh:10-11`). The standalone browser build of the crate keeps failing as SPEC-342's M4
recorded (#626).

The engine reaches the crate through one seam. The engine core gains one dependency, `deck-streak-fsrs7`, named by
one source file, `crates/engine-core/src/replay.rs`; a census test holds that, with a planted second file it refuses.
`docs/CONTEXT-MAP.md`'s engine-core line becomes `depends on: fsrs7`, and ADR-356 D4's sentence that the core depends on
no workspace crate is amended, insert-only, to name this one edge.

Chosen against:
- Re-pinning to a newer upstream head: it voids SPEC-342's M1 to M5, measured at this revision, and nothing asks for it.
- Waiting for a published release: none carries FSRS-7, so the issue would wait on a release nobody has scheduled.
- Vendoring the upstream source: it copies the scheduler's code into the tree with no revision to compare it against,
  which ADR-353 D1 already refused.
- A browser random-source feature in the crate: the random-source library says a library never selects it, and the
  web engine's graph selects it already.
- Each client adapter depending on the crate, as the XP crate will (ADR-357 D1): two joins of one rule, and the web
  join cannot be tested natively against the engine.
- The crate depending on the engine core: the scheduler's crate would learn the engine's types, which ADR-338's
  isolation keeps out.
- A bridge crate between the core and the crate: a third crate for one module, and the core would still depend on it.

## D2. The replay, and what it was chosen against

**The rows.** A card's rows are taken in id order. Every row up to and including the card's last reset is dropped; a
reset is the engine's own Forget row, kind 4 with factor 0, and A18 confirms the predicate against the engine at its
pin. SPEC-342 R4's drops then apply: kinds 4 and 5 and ease 0. The first kept review has the delta 0 and every later
one the id difference in days. `RevlogRow` gains `factor`, and `CardHistory` gains `last_id`.

**The cards.** The caller passes a deck set. The replay reads the review rows of the cards whose home deck is in the
set (the original deck for a card in a filtered deck), with each card's type, by ONE fixed statement through the
database door. One statement runs under the engine's lock on the collection, so its rows are one state of it. A review
row whose card was deleted is not read. An undone answer's row is gone (SPEC-342's undo measurement), so the replay
after an undo equals the replay before the answer. A card with no kept review gets no entry.

**The fields.** The replay runs the pinned revision's model with the parameters given: none for the revision's
defaults, or a 34-value vector. The stock stability is the interval at which the revision's forgetting curve reads 0.9,
by the revision's own interval function; the difficulty is clamped to 1 to 10. For a card of the review type, `ivl` is
the interval at the preset's desired retention, rounded and clamped to 1 and the maximum interval, and `due` is the
engine day of the last kept review plus `ivl`, with no fuzz. Another card type gets the stability and difficulty only.
`decay`, `dr` and `lrt` are left as they are.

**Where.** `crates/engine-core/src/replay.rs` adds `Dispatcher::replay` and the pure mapping from a result to a card's
stock fields. The projection without engine terms (`stock`) is a pure module of the crate.

**The proof.** The replay's values equal the pinned revision's own test values on that test's fixed inputs; each of the
four first ratings replays to its initial stability, typed from the revision's defaults; the selection equals the Lean
entry's vectors.

Chosen against:
- SPEC-342 R4 without a reset cut: a reset card would replay the reviews from before its reset, which the engine's own
  history for FSRS drops.
- FSRS-7's raw stability in `s`: a stock client reads `s` as the 90-percent stability under the card's own decay, and
  only the 90-percent point reads the same under every decay.
- Resolving the preset's decks inside the replay: a second engine read beside the history read, so the two would not
  be one state, and the caller that chose the preset already names its decks.
- Paging the read by card: several statements, and another thread of the native client can answer, undo or sync
  between them, so the pages would not be one state.
- A fuzzed interval: fuzz belongs to the switch, and a fuzzed replay is no longer a function of its rows.
- Writing `decay`, `dr` or `lrt`: decay and the desired retention are the preset's, and the engine's last-review time
  already equals the last kept review (A19).
- Replaying the released FSRS-6 model with FSRS-7's rows: ADR-338 chose FSRS-7, and the released model is the stock
  one the engine already runs.

## D3. Its reach, and what it was chosen against

No client calls the replay in this delivery. The native adapter and the web engine export nothing new, and a census
test (A22) refuses any of their sources that names it. The replay writes nothing: the stamp, the undo status and every
row are unchanged after it (A16). The write it prepares is the scheduler switch, the owner's tap on one preset, made by
the preset screen (#611); here the mapping is proved by a test that writes through the engine's own card update and
reads the card back after a sync round trip (A17), as ADR-338's confirmation asks.

The web engine compiles the crate because the core depends on it, and no export reaches it, so the bundle's reachable
code does not change. The read-only design could not run a build; the builder runs the web build and its size script
before the push and records both readings beside dev's, and CI's `web-engine` job decides. The compute costs at most
0.2329 µs a review in the browser target and 0.1937 µs natively (SPEC-342 M1, M2); the one-statement read's cost is
unmeasured and is the first caller's to time.

Chosen against:
- Exporting the replay to both clients now: an export with no screen to call it, which would also bring the crate's
  code into the web bundle before anyone can reach it.
- Writing the stock fields from the engine core now: an unlisted mass-reschedule write with no owner tap to admit it,
  which the owner-taps ruling refuses for every batch and background job.
- A stored replay cache: ADR-338 decided rebuilt, never stored, and a cache is a second copy a sync would carry.
- Replaying at every collection open: no preset is switched yet, so every open would pay for a result nobody reads.

## D4. The tests, and what it was chosen against

Every criterion of SPEC-386 section 3 is a test, red first for the reason section 3 states, except A1 and A22, which
guard what the base already holds and are recorded `not red`. The fixtures' seed inputs are fixed the way the pinned
revision's own test fixes them. The rows come from S38600, with a row for every new literal constant and every branch
of the selection, the projection and the statement.

FORMAL: Lean is REQUIRED for the history selection, a total pure function over integer rows that decides which reviews
count. The entry `lean/ReplayHistory` ports `histories` and its predicates, proves the reset cut, the drops, the first
delta 0, non-negative deltas and order independence, and its witness shows the merge-base's `histories` keeping a
review from before a reset. TLA+ is NOT APPLICABLE: the replay reads one statement, writes nothing and adds no actor.

Chosen against:
- Vectors computed by running the crate: the code under test would build both sides of its own comparison.
- The released FSRS-6 values as the reference: a different model with 21 parameters cannot decide FSRS-7's values.
- A Lean port of the model's floating-point arithmetic: that arithmetic is the pinned revision's code, held by its own
  test values, and a port would prove a copy.
- A TLA+ model of the read: one statement, no write and no new actor leave no interleaving to model until the switch
  acts on the result (#611).

## D5. The shape, and what it was chosen against

ONE delivery under SPEC-386 and ADR-400: 23 criteria, beside SPEC-342's 22, over a crate that already exists.

Chosen against:
- Two deliveries, the crate and then the replay: the crate landed with SPEC-342, and what it still needs is two fields,
  a cut and a module.
- An insert-only amendment of SPEC-342 and ADR-353: they decided a measurement, and this adds a dependency edge, a seam
  and a read that the context map must record; an amendment would hide a new edge inside an old decision.

## Consequences

- The engine core depends on the crate, so both client builds compile it; no export reaches it, and the linkers drop
  what nothing calls.
- `docs/CONTEXT-MAP.md`, `crates/engine-core/src/lib.rs:41-42` and ADR-356 D4 name the one edge.
- The crate's measurement workflow runs on the change and reads SPEC-342's checksums, since the generator writes no
  reset.
- The preset screen (#611) inherits a tested replay, a tested mapping and the write it must admit by the owner's tap.

## Confirmation

- SPEC-386 A1 to A23, each red first or recorded `not red`, green at the head.
- The Lean entry `lean/ReplayHistory` checks clean, its witness built, its vectors byte-equal.
- CI's `web-engine` job passes its size check, and the `fsrs7-measure` workflow's checksums match SPEC-342's.

## What would make this wrong

- A release of the scheduler carries FSRS-7: re-pin to the release, as ADR-353 says.
- The engine's own Forget row is not kind 4 with factor 0: A18 reads red, and the predicate follows the engine.
- The first caller finds the one-statement read too slow or too large for a real collection: page the read with a
  modification-stamp bracket, and model that bracket, since pages interleave.
- The pinned revision's tests hold no FSRS-7 values: A5 has no oracle, and the build stops rather than invent one.
