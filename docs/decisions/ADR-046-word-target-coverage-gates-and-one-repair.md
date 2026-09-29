---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The reading's form: a word target that scales with the new cards, the predecessor's coverage gates on a persona output, and exactly one named repair

## Context and Problem Statement

The owner decided that a law primer "scales with the topic's new-card count" inside 800 to 1500
words, and left the scale itself open. The predecessor's coverage gates were written for its own
reply format (a card-map roster, then prose, and no list anywhere); DeckStreak's reading is a persona
output whose packs require list-shaped sections (the retrieval prompts, a language reading's
glosses) and Pandoc citations for law. The predecessor specified a single repair naming the failed
gate and never built it. How are the target, the gates and the repair defined here?

## Decision Drivers

- The owner's band (800 to 1500) and "scale by cards", with no daily cap.
- Every new card's note must be engaged by the text, not merely listed.
- One definition per check: the packs' classes stay the packs', and the readings add only what no
  pack judges.
- Fail loud, bounded: never a placeholder, never an unbounded retry.

## Considered Options (the alternatives it was chosen against)

- Target T(n) = min(1500, 800 + 50 × (n − 1)); the roster judged through the packs (law citations over `sources` equal to the seed, language glosses over `x-new-words`); the predecessor's anchor gate kept for law; the list-marker gate applied to the primer's prose sections only; the engine writing every frontmatter declaration; one repair naming the failed gate and its findings — chosen: it scales from the first card, reuses the packs' checks, keeps the predecessor's proof that each card's own text is engaged, and bounds the cost of a failure.
- A fixed target of 800 words for every topic — rejected because the owner chose scaling, and a topic with one new note would be padded.
- A per-note quota such as 100 words a note, clamped to the band — rejected because it stays at the floor until the eighth note, so the common small topics would not scale at all.
- Citations alone as proof of coverage, with no anchor gate — rejected because a citation proves the model named a note, not that the text engages the card's own words; the predecessor's anchor forces each card's opening text into the prose.
- The list-marker gate over the whole output — rejected because the retrieval prompts and the glosses are lists by study-duties' and language-mentors' contracts.
- A blind retry, or retries until a pass — rejected because a retry that does not name the failure repeats it, and an unbounded loop hides a failure the owner must see.

## Decision Outcome

Chosen option. T(1) is 800 and T reaches 1500 at fifteen new cards; the band itself is enforced by
study-duties' `reading-length`, and the scaling by `reading-scale` on the golden readings. The engine
writes `sources`, `x-new-cards` and `x-new-words`, so the roster the gates compare is the seed's by
construction. The anchor, list-marker and completeness gates are the readings' own; every other
check is the packs'. A failed gate earns one regeneration whose instruction names the gate and its
finding lines, then the topic fails with `gate_failed:<gate>`. Three text crates are admitted to
`[workspace.dependencies]`: `unicode-normalization` (NFKC for anchors), `html-escape` (entity
decoding for anchors) and `unicode-segmentation` (word counts for reading minutes).

### Consequences

- Good, because a one-note topic gets an 800-word primer and a heavy topic gets up to 1500, never
  padded to a fixed length.
- Good, because the predecessor's anchor goldens prove the port of its normalisation.
- Bad, because a gate failure costs a second run; each run is capped and every attempt recorded.
- Bad, because the target is a request: the model may miss it, and only the band is enforced.

### Confirmation

SPEC-046's tests, the goldens of `preread.py:anchor_for_note` and `preread.py:is_anchor_usable`, and
the study-duties, learning-science, law-professors and language-mentors rows over the golden readings.

## What would make this wrong

- The owner reads the scaled primers and asks for a different slope or a different ceiling point
  (then T changes by a SPEC amendment and this ADR is superseded).
- `reading-scale` reports the generated readings shrinking as n grows (the target is not being met).

## More Information

SPEC-046; ADR-019; the predecessor's `preread.py:check_coverage`; the study-duties ("The law band"),
law-professors ("The law output contract") and language-mentors ("The output contract") packs;
docs/schematics/readings-generation-flow.md.
