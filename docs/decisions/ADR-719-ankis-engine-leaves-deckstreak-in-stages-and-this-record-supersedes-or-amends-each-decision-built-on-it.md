---
status: "proposed"
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-719: Anki's engine leaves DeckStreak in stages, and this record supersedes or amends each decision built on it

## Context and Problem Statement

Every DeckStreak client and the service open the owner's collection through Anki's Rust engine,
pinned to a patched fork (ADR-009, ADR-022, ADR-058). ADR-599 states what is still Anki's beneath
DeckStreak, and lists the engine's replacement as the event that would make that text wrong.
DeckStreak is moving onto a core of its own, and the engine is to leave the clients, the service
and the build. Dozens of accepted and proposed decisions lean on the engine. A record in this
repository is never edited to say something new; a later record supersedes or amends it
(ADR-589). Which of those decisions end, which change, and at which step, so that a reader can
follow the whole removal from one place?

## Decision Drivers

- The owner's study must not stop: reviews keep working at every step of the move.
- Each existing record stays as written, and its successor is easy to find.
- Nothing that speaks Anki's sync protocol can stay once the engine is gone.
- Every change waits for the step that actually ends or moves the seam it describes.
- Import and the package writer are product features, so they survive the engine.

## Considered Options (the alternatives it was chosen against)

- Chosen: one record that names every decision the removal ends or changes, with the step at which each change takes effect, because one pointer then leads a reader through the whole move and every older record keeps its text.
- A new ADR for each affected decision: rejected because some seventy records would each repeat the same removal, and the order between them would be written nowhere.
- Rewrite the status and text of every affected ADR now: rejected because most of them stay true until the cut-over or the removal, and records are kept as written.
- Keep a bridge that speaks Anki's sync protocol after the removal: rejected because the bridge would keep the engine's wire format, and with it the engine, inside the product.

## Decision Outcome

Chosen option: "one record for the whole removal", because it keeps every older record intact and
puts the order of the move in one table.

The removal runs in ordered steps, and a dual run of the old engine and the new core on the same
collection comes before the cut-over. At the cut-over DeckStreak's clients and service switch to
the new core; the removal, after a soak period, takes the engine out of the build.

- **Sync.** No sync with Anki's protocol survives the removal. Until the cut-over the stock Anki
  apps keep syncing the owner's collection, for browsing and authoring; at the cut-over they stop.
- **Tools.** Every tool the owner relies on must exist on the new core before the cut-over may
  happen. Add-ons and the collection's database check are recorded losses, accepted knowingly.
- **Import and packages.** Import and the package writer stay, rebuilt on the new core. ADR-151,
  which made the package the engine's own export and was never accepted, is superseded now.
- **The owner's taps.** The owner's own taps carry onto the new core's operations, under the
  conditions their signed ruling sets.

Superseded by this record, each at the step named:

| when | decisions |
|---|---|
| now | ADR-151 |
| at the cut-over | ADR-009, ADR-340, ADR-347, ADR-362, ADR-368, ADR-388, ADR-404 |
| at the removal | ADR-022, ADR-058, ADR-338, ADR-346, ADR-348, ADR-351 (after the soak), ADR-353, ADR-400, ADR-516 (its golden files stay) |

Amended by this record. None changes now; each amendment takes effect with the step that moves
the engine seam it describes:

| group | decisions | what changes |
|---|---|---|
| clients | ADR-335, ADR-336, ADR-345, ADR-349, ADR-350, ADR-352, ADR-356, ADR-357, ADR-359, ADR-360, ADR-361, ADR-374, ADR-375, ADR-376, ADR-382, ADR-385, ADR-397, ADR-423 | the clients run over the new core; ADR-349's module size is measured again |
| the owner's taps | ADR-337 | its grounds move from the engine to the new core, at the cut-over |
| product rules with an engine seam | ADR-369, ADR-377, ADR-389, ADR-407, ADR-424 | each rule stands and its seam moves: ADR-369, ADR-377 and ADR-389 to the new core's answer path, ADR-407 and ADR-424 to the new card renderer; the device backup in ADR-369 copies DeckStreak's own store instead of the engine's collection |
| data paths | ADR-037, ADR-064, ADR-066, ADR-083, ADR-089, ADR-095, ADR-097, ADR-120, ADR-146, ADR-301, ADR-321, ADR-383, ADR-387, ADR-512, ADR-583 | each reads DeckStreak's store; ADR-064's store is backed up before the cut-over, and ADR-383's golden holds the new renderer too |
| host and CI | ADR-032, ADR-038, ADR-055, ADR-057, ADR-191, ADR-194, ADR-330, ADR-365, ADR-409 | the engine's build and test costs leave the host and CI budgets |
| licence and brand | ADR-018, ADR-014 | ADR-018's reasoning only; ADR-014's notice names no holder |

Read and kept as they stand: ADR-002, ADR-020, ADR-026, ADR-045, ADR-051, ADR-071, ADR-076,
ADR-087, ADR-092, ADR-150 (the GUID rule, which the new core keeps), ADR-342, ADR-354, ADR-371,
ADR-498, ADR-515, ADR-339 and the ADR-339 amendment.

The same record decides the rulings, the CHARTER and the SPECs that rest on the engine:

- The engine half of ANK-01 and all of ANK-08 are superseded at the cut-over; ANK-01's identity
  wording was already superseded by the signed identity ruling that ADR-599 carries.
- ANK-03 is amended at the cut-over: the owner's taps stay as DeckStreak features, and AnkiMobile
  stops being their measure. The owner-taps ruling and STAT-01 are kept; STAT-01's comparison
  wording changes at the cut-over.
- The deck-writes ruling and its CHARTER amendment are amended at the cut-over. The converters
  ruling is kept; its licence line has nothing to apply to after the removal.
- CHARTER constraint 4, which allows no write back to Anki except the skip day's, is superseded
  at the cut-over; CHARTER's mission sentence is amended when it stops being true; CHARTER item
  22 is kept.
- SPEC-334's R2 is superseded, its scheduler half at the scheduler switch and its engine half at
  the cut-over; its R3 and R5 are amended at the cut-over, and its R9 is superseded then.
- Superseded at the cut-over or the removal: SPEC-022, SPEC-055, SPEC-335 to SPEC-338, SPEC-340,
  SPEC-342, SPEC-345, SPEC-351, SPEC-357, SPEC-363, SPEC-364, SPEC-374, SPEC-377, SPEC-386,
  SPEC-395 (its sync-server rows), SPEC-498 (its golden files stay), SPEC-565 and the planned
  SPEC-151.
- Amended at the step that moves their seam: SPEC-001, SPEC-021, SPEC-023, SPEC-038, SPEC-039,
  SPEC-045, SPEC-059, SPEC-061, SPEC-066, SPEC-083, SPEC-301, SPEC-334, SPEC-343, SPEC-344,
  SPEC-346, SPEC-347, SPEC-348, SPEC-350, SPEC-352 to SPEC-354, SPEC-358, SPEC-365, SPEC-368,
  SPEC-371, SPEC-372, SPEC-376, SPEC-378, SPEC-383, SPEC-393, SPEC-394, SPEC-409, SPEC-410,
  SPEC-494 and SPEC-496.

### Consequences

- Good, because a reader of any affected record finds its successor here, and every older record
  keeps its text.
- Good, because the owner keeps studying through the move: the stock apps sync until the cut-over,
  and the dual run comes before it.
- Bad, because add-ons and the collection's database check are lost at the cut-over.
- Bad, because the tables must be read with the step each row names: most rows do not take effect
  until a later step lands.

### Confirmation

When a step lands, the pull request that lands it sets the status line of each ADR this record
supersedes at that step to `superseded by` this record, and cites this record in each SPEC it
amends. ADR-151's status line changes with this record. Review holds each step to its row.

## What would make this wrong

- The dual run shows the new core treating the owner's collection differently from the engine,
  and the difference cannot be closed before the cut-over.
- A tool the owner relies on cannot be carried onto the new core.
- The owner chooses to keep a sync with the stock Anki apps after the cut-over.

## More Information

ADR-599 (the identity text and the signed identity ruling it carries), ADR-589 (records are kept
and later ones supersede them), ADR-009, ADR-022 and ADR-058 (the engine and its pin), and the ADR
titled "The iPhone and iPad app ships no Anki code, and no App Store build carries the engine",
which applies this removal to the iPhone and iPad target.
