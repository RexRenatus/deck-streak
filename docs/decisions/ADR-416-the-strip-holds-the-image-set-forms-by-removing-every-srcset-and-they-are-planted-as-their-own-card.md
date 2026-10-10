---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The strip holds the image-set forms by removing every srcset, and they are planted as their own card

## Context and Problem Statement

ADR-352 gives the web card frame four layers: W1 the sandbox, W2 the frame policy, W3 the strip and
W4 the page's `frame-src` (ADR-352 D7), and its D4 makes the strip remove the card's `link`, `meta`,
`base` and `template` elements and refuse a card whose re-parse brings one back. The schematic
`docs/schematics/card-frame-channels.md` holds the table D7 runs. In it the `img` channel, seven
forms on one planted card, two of them image-set forms (`img srcset` and `picture source`), is held
by W2 alone. In Firefox an engine fetches an image-set candidate ahead of its tree builder, typed as
an image set, outside the preload types its early copy of the meta policy is checked for, so W2
holds those two forms there only beside W1 and W4 (#766's first reading). #771 asks for a layer
built for them: W3 removes the `srcset` attribute from every element that carries one, and its
re-parse check refuses a card in which one comes back. Every `file:line` below was read at `dev`
`1414a982`.

## Decision Drivers

- A card's markup is untrusted input from a shared deck (ADR-352).
- A layer holds a form in every engine by what it does, never by an engine's fetch order.
- No assertion of the planted suite is narrowed in any engine, and every form keeps a reading.
- The change is one layer's: it adds no actor, no state and no page policy.

## Decisions, and the alternatives each was chosen against

### D1. The population: what the strip removes, where a candidate can reach the frame, how the check refuses, and which cards plant a form

The strip removes `link, meta, base, template` (`web/app/src/lib/card/frame-document.ts:17`),
each element whole (`:39`), from the inert parse (`:22-24`, `:38`), and no attribute.

A `srcset` candidate is fetched from an `img` and from a `source` inside a `picture`; a `link`
`imagesrcset` preload is the third carrier, and W3 already removes every `link`. The parser admits
a `srcset` attribute on any element.

The re-parse check (`:46-51`) keeps a card when the head reads as written (`:48`), the body holds
no stripped element (`:49`) and the body's attributes are the ones written (`:50`), and refuses it
as `escaped` otherwise (`:51`); `web/app/src/lib/card/CardFrame.svelte:15` then renders a frame
with no document.

The planted cards that hold an image-set form are the `img` card's `img srcset` and
`picture source` (`web/app/tests-card/planted.ts:98-99`). The `css-url` card's `image-set()`
(`:108`) is CSS, which the strip never reads.

Chosen against:

- Counting the carriers from the planted suite alone: rejected because the suite plants only what
  it plants, and the parser admits the attribute on every element.
- Counting `link` `imagesrcset` as a carrier still to close: rejected because W3 removes every
  `link` whole (ADR-352 D4), so no `link` reaches the frame document.

### D2. W3 removes the `srcset` attribute from every element of the inert card, by the attribute's name

`frameDocument` removes the attribute from each element the selector `[srcset]` matches in the
inert parse, after the stripped elements are removed, and keeps the element with its other
attributes, its `src` included. No image-set candidate then reaches the frame document in any
engine, the element's `src` still shows the image, and W2 holds every image form left, alone.

Chosen against:

- An image source list in the page policy: rejected because it constrains every image the app shows, and it makes the inherited page policy, which the card frame does not own, hold what the frame's own layers should.
- The frame's `csp` attribute: rejected because ADR-352 D3 already rejected it, since one target engine does not enforce it.
- Rewriting each `srcset` to keep only `data:` candidates: rejected because it must parse candidate lists as each engine does, commas inside `data:` URLs included, where removing the attribute leaves nothing to parse and the `src` still shows the image.
- A selector naming the carriers, `img[srcset], source[srcset]`: rejected because the parser admits the attribute on every element, so a carrier list decides by element where the requirement decides by attribute.
- Removing each element that carries a `srcset`: rejected because the card's `src` image would go with it, and the issue keeps the `src` showing.
- Setting each `srcset` to the empty string: rejected because an empty attribute is still an attribute the frame parses, and the re-parse check would have to tell an empty one from a full one.

### D3. The re-parse check refuses a frame document that holds a `srcset` anywhere, the root element included

The check gains one conjunct: the re-parsed document holds no element matching `[srcset]`, read
over the whole document. A card whose markup the first parse reads as a style's text and the
second as an element with a `srcset` is refused, as D4 of ADR-352 refuses a stripped element that
comes back. A `<html>` start tag the second parse meets inside the body adds its attributes to the
root element, outside the body, the same way the body's own attributes arrive in the mutation
`frame-document.test.ts:136-137` plants.

Chosen against:

- Reading the body alone, as the stripped-element conjunct does: rejected because the root element is outside the body, and a `srcset` there is still a `srcset` in the frame document.
- Stripping the re-parsed document a second time instead of refusing: rejected because the shipped document would then be one whose next parse nobody read, which is why ADR-352 D4 refuses rather than repairs.
- No check, the strip alone: rejected because the strip reads the first parse, and a mutation that the second parse reads differently brings the attribute back unseen.

### D4. The layer table: W3 holds the image-set forms beside W2, and no layer holds them alone

This amends ADR-352's layer table. W3's row names the `srcset` removal and its refusal. The
image-set forms become the `srcset` channel: blocked in the card frame by W3 and W2, layer alone
`-`. With W2 off, W3 has removed the attribute, so nothing is fetched; with W3 off, W2 refuses the
fetch in Chromium and WebKit, and in Firefox the forms stayed closed in #766's reading with W1, W2
and W4 on. The `img` channel keeps its five other forms, `img src`, `input type=image`, SVG
`image` and `use` and `video poster`, and its layer alone W2: W3 keeps every one of them, and W2
is the one layer that governs their fetch.

Chosen against:

- The image-set forms left in the `img` card: rejected because the suite reads a card's arrival by prefix (`web/app/tests-card/listeners.ts:111-115`), so the `img` card's `W2 off` variant would pass on `/img/1` while the two forms never arrive, a reading ended with no test failing.
- The `srcset` channel's layer alone set to W3: rejected because W2 also holds the forms in Chromium and WebKit, so the `W3 off` variant opens nothing there, and an "opens" expectation would fail.
- Per-engine expectations for the image-set forms: rejected because they narrow the single-layer assertion (`web/app/tests-card/card.spec.ts:137`) with an engine exception, a weakening.

### D5. The re-plant: the `srcset` card names each form's path, and the `img` card keeps its paths

The `srcset` card plants `<img alt="" srcset="<listener>/srcset/1 1x">` and
`<picture><source srcset="<listener>/srcset/2"><img alt=""></picture>`, clicks nothing, and names
the paths `/srcset/1` and `/srcset/2`, so its reference frame must reach each form. The `img` card
loses those two forms and keeps `/img/1` and `/img/4` to `/img/7` as they are. No card other than
`srcset` plants a `srcset` attribute, and the coverage test says so.

Chosen against:

- One prefix path for the `srcset` card, as `loads()` gives: rejected because the reference would pass when one of the two forms arrived, and each form is read alone nowhere else.
- Renumbering the `img` card's forms from 1 to 5: rejected because it changes paths the suite and #766's records name, for no reading it adds.
- One path per form on the `img` card too: rejected because no reading at `dev` shows each of those forms arriving in every engine, so it is a new measurement, not this change.

### D6. FORMAL is not applicable, by surface

`frameDocument` is a total function of the card's text, its CSS and its classes: one caller, no
shared state, no step another actor can interleave, so it is no protocol for a model. Its claim,
that a kept document holds no `srcset` as the parser reads it, is checked by the function itself
on every card, since D3 refuses any document that holds one, so a proof would restate the check.
No model or proof under `formal/` cites the strip's attribute list: 0 of the 208 files there name
it.

Chosen against:

- A model of an engine's speculative fetch beside its tree builder: rejected because that interleaving is the engine's, not this code's, and the planted suite reads its outcome in the engine.
- A proof that the composed document holds no `srcset`: rejected because the claim rests on the HTML parser's tree construction, which no proof in `formal/` models, and the run-time check already refuses every document that breaks it.

### D7. ADR-416 records the amendment; ADR-352 and SPEC-341 are not edited

ADR-352 names the schematic as holding the tables D7 runs (ADR-352's More Information), so the
amendment is this ADR and the schematic's rows.

Chosen against:

- An insert-only amendments section appended to ADR-352 and SPEC-341: rejected because #766 appends its own section to each after the same last line, so a second appended section conflicts with it in both files.
- Editing ADR-352's D4 text in place: rejected because an accepted decision's text is not edited.

### D8. Two pushes: the red alone first, then the fix and the record

The suite reads the `srcset` card's single-layer red only in CI, in the browsers. Push 1 carries
the tests, the documents and the re-plant over the old strip, so `card-sandbox` reads `W2 off:
srcset stays closed` failing in Chromium and WebKit and `web` reads A1 and A2 failing. Push 2
carries the strip and the record.

Chosen against:

- One push with the fix: rejected because the `srcset` card's single-layer red would never be read, and a test whose red nobody saw proves nothing about the strip.
- Three pushes, the tests first without the re-plant: rejected because A1 to A3 read red locally, and that push's `card-sandbox` run would read nothing new.
- The strip before the re-plant: rejected because the `img` card's `W2 off` variant would then pass on its prefix while the two forms went unread.

### D9. No mutation row: StrykerJS proves `frame-document.ts`

A pull request's `mutation-web` job mutates each changed production file whole and breaks at one
survivor (`web/app/stryker.config.json:7-18`). Every mutant of the new lines dies by A1 or A2: an
empty selector throws, an emptied attribute name leaves the `srcset` that the check then refuses,
and each operand of the new conjunct is decided alone by A2's plants or the benign cards. No row
of `scripts/mutation-rows.d` anchors in the strip or the check, and none moves.

Chosen against:

- Hand rows in the band S40200-S40299: rejected because web code with no row table is proved by StrykerJS, and a row would mutate what StrykerJS already mutates.
- An equivalence record: rejected because no mutant of the change is equivalent, and a record excuses only a recorded survivor.

## Decision Outcome

W3 removes every `srcset` and refuses a frame document that holds one; the image-set forms are the
`srcset` channel, held by W3 and W2 with no layer alone, planted as their own card that reads each
form's path; the `img` channel keeps five forms under W2 alone; and no assertion of the suite is
narrowed.

### Consequences

- Good: the image-set forms are closed by what a layer does, in every engine, whatever its fetch
  order.
- Good: each image-set form is read by its own path, where before it hid in the `img` card's prefix.
- Bad: a card's responsive candidates are dropped; the frame shows the `src` image only.
- Neutral: each engine runs 149 planted tests, five more than at `dev`.

### Confirmation

`web` runs A1 to A4's Vitest tests, `card-sandbox` runs the planted suite in Chromium and WebKit,
and `mutation-web` mutates `frame-document.ts` whole; `docs/red-first/SPEC-402.md` records each
red and green.

## What would make this wrong

- An engine fetches an image-set candidate from a kept frame document: some attribute other than
  `srcset` carries one, and D1's population is incomplete.
- Firefox opens the `srcset` card or the `img` card in a single-layer variant with this strip on
  `dev`: the cause #766 read is incomplete, and the layer table is reopened.
- A card needs its responsive candidates for parity: the frame then needs a media path that keeps
  them, decided with the study screens' media.

## More Information

- Issue #771; #766 (Firefox in the card suite), whose last push reads Firefox over this strip.
- ADR-352 D3, D4 and D7; ADR-361 D12 (media reach the frame as `data:` in `src`, unchanged here).
- SPEC-402; the schematic `docs/schematics/card-frame-channels.md`, sections 2 and 3.
