---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The sabbatical clock ports its two primitives into insights, and the peak module stays inert

## Context and Problem Statement

The sabbatical clock (#150) asks how long each course can be left before its cards fall below
their floors. The predecessor computes it in `sabbatical.py:compute_sabbatical`, which calls two
primitives of another module: the elapsed time until a card's retrievability falls to a floor
(`peak.py:elapsed_at_floor`) and the decay clamp beneath it (`fsrs.py:clamped_decay_exponent`),
plus a cold floor for cards with no desired retention (predecessor `27ee2bc`). The rest of the peak
module is the Peak Day taper, which no production path calls: it is inert and its revival is an
owner question (#182). Curriculum already ports the retrievability curve and its clamp for mastery
(SPEC-077), but insights depends only on the kernel and ingest (the context map). Where do the two
primitives live?

## Decision Drivers

- The crate graph is the context map: insights may not depend on curriculum.
- The kernel knows no context (the context map's shared kernel), and a retrievability curve is
  learning-domain logic.
- ADR-012: each number the clock states comes from a golden of the predecessor's own function.
- An inert feature is excluded unless its issue says otherwise (the inert-feature default).
- One predecessor function stays in one context, so its golden has one owner.

## Considered Options (the alternatives it was chosen against)

- Port `elapsed_at_floor` with its decay clamp, the cold floor and the tie epsilon into `crates/insights/src/sabbatical.rs`, each proved by a golden, and nothing else of the peak module — chosen: the clock stays a pure build in one context, and the inert taper stays out.
- The retrievability curve and its clamp in the kernel, shared by curriculum and insights — rejected because it puts learning-domain logic in the shared kernel, which knows no context.
- Coordination computes each card's elapsed days with curriculum's clamp and passes them to insights — rejected because it splits one predecessor function across two contexts and one golden across two owners.
- Port the whole peak module, the taper included — rejected because the taper is inert in the predecessor and its revival is the owner's question (#182).

## Decision Outcome

Chosen option: "Port the two primitives into insights", because the clock is then a pure build over
ingest's reads, proved function by function, and the inert taper stays excluded.

- `crates/insights/src/sabbatical.rs` holds the elapsed time to a floor with its clamp, the cold
  floor 0.5 and the tie epsilon 1e-9, and the clock itself (SPEC-098 R10 to R13).
- The clamp is a second port of `fsrs.py:clamped_decay_exponent` beside curriculum's (SPEC-077);
  each copy is proved by its own golden of the same function, so the two cannot drift apart
  unnoticed.
- Nothing else of the predecessor's peak module is ported. If the owner revives the Peak Day taper
  (#182), its SPEC may move the primitives to wherever the taper lives and cite this record.

### Consequences

- Good, because the clock needs no new crate edge and no domain logic in the kernel.
- Good, because the taper stays excluded under "not revived".
- Bad, because the decay clamp is ported twice; both copies answer to the same predecessor
  function's golden.

### Confirmation

SPEC-098's criteria on the elapsed time to a floor, the constants and the clock, and the mutation row
that holds the elapsed time never negative.

## What would make this wrong

- A third context needs the retrievability curve; the curve then earns a context of its own, and the
  copies move there.
- The owner revives the Peak Day taper (#182); its SPEC decides where the primitives live.

## More Information

The context map; ADR-012; SPEC-077 (curriculum's port of the curve); SPEC-098, which builds the
clock; #150; #182.
