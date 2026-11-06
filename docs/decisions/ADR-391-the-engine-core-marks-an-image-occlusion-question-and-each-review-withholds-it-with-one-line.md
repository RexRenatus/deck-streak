---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-391: the engine core marks an image occlusion question, and each review withholds it with one line

Decides SPEC-380 (issue `#750`). An image occlusion question hides parts of its image behind
masks drawn by a script when the card is shown, and no review of this workspace runs that script,
so the question would show its answers. This record decides what "the masks are not drawn" is and
where it is decided, what each review shows instead, what happens to the card, how the line is
carried, and how the native review takes it within its budgets.

## Context and Problem Statement

Read at `e7ecf10d6b796eb1f86fe6544e04a96a0583c791` (`dev`). The web review runs no card script: the
card is the source document of an empty sandboxed frame (ADR-352 D1 and D2). The native review
runs none either: its card scripts switch defaults off
(`ios/CardIsolation/Sources/CardIsolation/CardScripts.swift:7`) and the ffi's face document names no
script (`crates/ffi/src/face.rs:69-81`). Neither review loads the engine's reviewer scripts, and the
one mention of occlusion in the tree places it in a later parity phase
(`docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md:159`). Both reviews
read the card through the engine core: the web engine's view (`crates/web-engine/src/wasm.rs:634-710`)
and faces (`:821-861`), and the ffi's face (`crates/ffi/src/engine.rs:155-168`), which is the core's
(`crates/engine-core/src/face.rs:167-224`). The native Swift is counted: the review view holds 3
decisions at its role's ceiling of 3, the review model 6 at 6, and the review session 3 of 4
(`ios/swift-roles.json`, `scripts/tests/test_ios_thin_swift.py:38-48`).

## Decision Drivers

- No learner sees an occlusion question's answers before answering it, on either review.
- No rating is ever recorded for a question the learner did not see.
- Both reviews read one answer, and the engine core gains no workspace dependency (ADR-356 D4).
- No card script runs (ADR-352 D1), and the native Swift stays within every role's ceiling.

## Considered Options (the alternatives each was chosen against)

### D1. What "the masks are not drawn" is, and where it is decided

- Chosen: one rule in the engine core over the rendered question, true when it holds the engine's mask layer or an occlusion shape, by marker names measured on a note the engine builds, because no review of this workspace draws masks, so every occlusion question is undrawn on both reviews, and one rule in the core is the one answer both read.
- Chosen against: each review judging what it painted, because neither review paints anything to judge, and two copies of one rule drift.
- Chosen against: reading the note type's kind through an engine call, because a kind read misses a cloned type and a custom cloze type that use occlusion shapes, and the rendered question already carries the answer.
- Chosen against: scanning the page's document after the frame loads, because the question's markup has then already reached the page, and the native review would need its own scan.

The rule sees: a stock occlusion note with shapes, with no shape, or with a malformed shape field
(each carries the mask layer); a cloned or custom type whose template drops the layer but whose
field still renders occlusion shapes; and either review with its card scripts on or off, since
neither loads the script that paints. It cannot see: an older occlusion note whose masks are
separate images and whose template holds neither marker (a mask image the media rules omit then
leaves the question unmasked), and a type edited to drop both the layer and every shape, which
renders as plain text. SPEC-380 section 5 names both, each with its issue.

### D2. What the review shows instead, and what happens to the card

- Chosen: the card withheld on both sides, one plain line where the card's lines are shown, the learner's own bury and flag (and the web's undo of the previous answer, as the question side offers it), no rating, and the card left due, because a rating for an unseen question records a recall that never happened, and leaving the card due loses nothing the learner did not choose.
- Chosen against: burying the card automatically, because that writes to the collection on an act the learner did not take.
- Chosen against: rating it Again automatically, because that records a grade for a question nobody saw.
- Chosen against: skipping it within the session, because the queue answers one head card and has no skip, so the same card would return at once.
- Chosen against: ending the review with a refusal, as an engine refusal does on the native review today, because every later card of the deck would be unreachable behind it.
- Chosen against: showing the card under a warning, because a warning cannot hide the answers the image shows.

### D3. The line and its locales

- Chosen: a new key, `study_card_withheld`, in each of the 7 web locales, modelled on the escaped card's line and holding each locale's own bury word, because the escaped line is the line a learner already reads for a card that cannot be shown, and the new one keeps its shape.
- Chosen against: reusing `study_card_escaped`, because it tells the learner they can still answer, which this card must not offer.
- Chosen against: English in every locale, because every line of the web review is translated, and the census refuses an English copy.

### D4. The native review

- Chosen: in scope now, because the issue names both reviews, and the native review on dev reads the same core face through the ffi.
- Chosen against: naming it out of scope with its own issue, because an occlusion question would then stay visible, answers and all, on the native review.

### D5. Where the native line is written

- Chosen: by the ffi, into the withheld face's document, in English, because the native review shows the document the ffi answers in the card's place, and its Swift then gains one decision, in the session, within its ceiling.
- Chosen against: a branch in the native review view that draws the line, because that view holds 3 decisions at its role's ceiling of 3, and raising a ceiling weakens the census.
- Chosen against: a notice the review model computes, because the model holds 6 decisions at its ceiling of 6.
- Chosen against: a native string catalog in this delivery, because it is a new surface for every native string, larger than this issue (#738).

### D6. Where "no rating" is enforced

- Chosen: in each review's own action table (the web's withheld phase row and the native model's patterns), with the question never shown, because a rating is reachable only from the answer side, which a withheld card never reaches.
- Chosen against: a refusal inside the web engine's rating call, because that call is a span the undo model covers, and the native adapter keeps no shown card to refuse against, so it would guard one review only.

### D7. The tests and the model

- Chosen: red-first tests per criterion; the rule's arms on literal questions copied from an engine-built note's render; the core's face, the web view by a source census, the ffi's face, the web page and its locales, the native session and model on the simulator step; rows from S38000; StrykerJS for the web, because each layer is tested where it runs.
- Chosen against: a TLA+ model, because no actor, shared state, timer or write path is added: the rule runs inside one synchronous call, and the withheld phase is one more arm in two transition tables.
- Chosen against: a Lean proof of the rule, because no recorded failure motivates it, and its two arms are few enough for the tests to cover each from both sides.
- Chosen against: hand rows for the Swift, because no mutation table takes Swift at the base (#650).

### D8. The fixture

- Chosen: a second collection beside the review fixture's first, from its own builder, because the first collection's own test holds four cards and two media files, and the native tests choose their deck by name in it.
- Chosen against: a deck added to the first collection, because that changes the fixture's own test and the deck list the native flow tests read.

### D9. The shape

- Chosen: one delivery under SPEC-380 and ADR-391, because the rule, the faces and both reviews' withheld state are one behaviour no existing SPEC governs.
- Chosen against: an insert-only amendment of SPEC-376 and ADR-387, because those decide the late line, a different behaviour for a different issue.
- Chosen against: an amendment of SPEC-358, because its criteria are the native parity phase's, and this guard serves both reviews.

## Decision Outcome

D1 to D9 as chosen above. The engine core marks an occlusion question by its rendered markers, and
withholds its face; the web engine carries the same answer on the card view; the web review shows
one line in every locale and offers undo, bury and flag; the native review shows the ffi's English
line in the card's place and offers bury and flag; neither records a rating, and the card stays due.

## Consequences

- Good: no occlusion question shows its answers on either review, whatever its card scripts switch
  reads.
- Good: one rule, in the core both reviews reach, with no new dependency, table pair or store.
- Good: the learner keeps the card: it stays due, and only the learner's own bury moves it.
- Bad: an occlusion card cannot be studied on either review until a review draws its masks.
- Bad: the native line is English until the native string catalog lands (#738).
- Neutral: older occlusion notes built from mask images are not seen, and show as they do today.

### Confirmation

SPEC-380's A1 to A15, the censuses' plants, the band's rows by their killers, the `mutation-web`
verdict on the pull request, and A13 and A14 read by name on the simulator step.

## What would make this wrong

- If a review of this workspace gains a mask painter, the rule's true no longer means "not
  drawn" for that review: the rule takes the review's answer, and D1 is decided again.
- If the engine's template stops carrying both markers, A4 goes red and the markers are measured
  again; if it stops carrying any marker, D1 is decided again.
- If the owner rules that a withheld card is buried or suspended by the review itself, D2 is decided
  again, with that write named.
- If a native string catalog lands, D5's line moves to it, and the Swift budgets are read again.
