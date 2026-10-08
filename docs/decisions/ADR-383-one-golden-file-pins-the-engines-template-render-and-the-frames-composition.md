---
status: accepted
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-383: one golden file pins the engine's template render and the frame's composition

Decides SPEC-372 (issue `#729`). The web review shows a card through the engine's own render and
the sandboxed card frame, and nothing yet shows that a cloze card, an answer that opens with
`{{FrontSide}}`, or a note type's own CSS reaches the frame as the engine rendered it. This record
decides how two tests in two languages pin one truth, what the fixture is, where newlines are
normalised, what "equal" means across the frame, where the web test enters, how each test is seen
red first, and which rows and model the delivery owes.

It amends no record. It works under ADR-361 (the review shows the faces the engine completed) and
ADR-352 (the frame's document and its check), and changes neither.

## Context and Problem Statement

At `dev` `ebdd1371` the engine completes a face in `crates/engine-core/src/face.rs:168-224`: one
render with the partial flag on, `{{FrontSide}}` filled by the shown question (`:190`), and the
note type's CSS carried through (`:219`). The Worker asks for both faces (`wasm.rs:563-568`) and
takes the head card's ordinal and CSS from a second render with the partial flag off
(`wasm.rs:417-421`, `:442`, `:446`). The review replaces the head card's sides with the faces'
texts (`review.ts:268-274`), the screen passes the side, the head's CSS and the classes
`card card<ordinal + 1>` to the frame (`ReviewScreen.svelte:155-160`), and the frame parses,
strips, normalises the CSS's newlines, composes, parses again and checks
(`frame-document.ts:36-51`). The frame's tests use plain markup and one line of CSS
(`frame-document.test.ts:15-16`); the engine's face tests hold no cloze note and never read a
face's CSS (`crates/engine-core/tests/face.rs`). Criterion 5 asks that a change on either side
fail its own language's test, each seen red first.

## Decision Drivers

- One truth: the web test judges the card the engine renders today, never a copy that drifts.
- Each side fails in its own language: an engine change fails the Rust test, a frame change the
  web test.
- No expectation is computed by the code under test.
- No production file and no frame policy changes; no dependency edge is added.
- The web mutation run and the Rust mutation run can each read what their tests read.

## Considered Options (the alternatives each was chosen against)

### D1. How the two tests pin one truth

- Chosen: one committed golden file, `web/app/src/lib/study/review-templates.golden.json`, holding the fixture's inputs and frame CSS written by hand and the engine's render captured once; the Rust test builds the fixture from its inputs, renders, and compares with it, and the web test reads it, because both tests then judge one render and each fails in its own language.
- Chosen against: two independent expectations, one per test, because they drift apart: the web test would go on judging a card the engine no longer renders, and nothing would say so.
- Chosen against: a golden the Rust test writes on each run, or a bless mode, because a test that writes its own expectation can never fail, so criterion 5's engine half would be unprovable.
- Chosen against: running the engine's wasm build inside the web test, because that build is CI's, and an engine change would then fail the web test instead of the Rust test.
- Chosen against: a golden under `crates/`, because the web mutation sandbox runs inside `web/app` (`web/app/stryker.config.json:21`), where a path that climbs out of the app does not resolve; the Rust test reaches `web/app` through the support module's `workspace()` with no new edge.
- Chosen against: a web test that computes its expectation through the frame's own document builder, because it would compare the frame with itself.

### D2. The fixture

- Chosen: two note types cloned from the stock Cloze and Basic (and reversed card) types, each with its name, templates and CSS set as literals from the golden, one cloze note with deletions c1 and c2 and one reversed note, in the default deck; cards named by their note and ordinal, never by id; the cloze CSS carrying a `.cloze` rule, CR LF line ends, one lone CR and a trailing LF, and the reversed CSS LF only with a trailing LF, because every seed input the render reads is then fixed by the golden, and ids and the clock, which the render does not read, appear nowhere in it.
- Chosen against: the stock note types' own templates and CSS, because an engine upgrade could then move the inputs as well as the render, and the golden would no longer say what was rendered from what.
- Chosen against: a collection file built by hand and committed, because its inputs would be opaque to a reader and to a review of the golden.
- Chosen against: fixing ids and the clock by hand, because the render reads neither; the Rust test instead asserts that no note or card id of its run appears in any rendered text.

### D3. Newline normalisation

- Chosen: only the CSS is normalised, CR LF and a lone CR to LF, in the frame, before the style element is composed (`frame-document.ts:40-41`, unchanged); the engine's render and the golden keep every CR, and the web test's oracle is the frame CSS written by hand with LF line ends, because that is what the frame's check (`:48`) needs and what the frame's parser shows.
- Chosen against: normalising in the engine or in the golden, because it would hide the very step criterion 3 asks about, and the fixture's CR could never reach the frame.
- Chosen against: the web test computing the expected CSS with the frame's own replacement, because it would copy the code under test into its oracle.
- Chosen against: normalising the body's newlines too, because the parser already reads a CR in the markup as LF on both sides of any comparison, so a body step would change nothing the frame shows.

### D4. What "equal" means across the frame

- Chosen: the frame's body, read by parsing the frame's document, has the same serialised children as the engine's text parsed as a body by the same parser, because the frame writes the card again (`frame-document.ts:38`, `:45`): `<hr id=answer>` reaches it as `<hr id="answer">`.
- Chosen against: byte equality between the engine's text and the frame's body, because it cannot hold without changing the frame's composition, which the issue rules out.
- Chosen against: equal text content, because a lost class, a lost `data-ordinal` or a lost rule would pass it.

### D5. Where the web test enters

- Chosen: the review screen, rendered with a fake client that serves each golden card (the head card with the golden ordinal and CSS and two sides of its own; the faces with the golden texts), read through the frame's `srcdoc` and `data-card-refused`, because only that entry covers the faces replacing the head's sides (`review.ts:268-274`), the CSS's source, the class composition (`ReviewScreen.svelte:155-160`) and the frame's document together.
- Chosen against: the frame's document builder alone, because a review that showed the head's own sides, or classes from another ordinal, would pass it.
- Chosen against: the card frame component alone, because the review's choice of text, CSS and classes would pass it unseen.

### D6. Red first

- Chosen: the red commit carries the tests beside planted changes, one on each side: the engine's face returns no CSS (`face.rs:219`); the review keeps the head card's own sides (`review.ts:274`); the frame's composition trims the CSS's trailing newline (`frame-document.ts:41`); and the frame's check admits only `card card1` (`frame-document.ts:29`). The greening commits remove them, so each test is seen red for its own criterion's reason and the net diff of every planted file is empty, because the tests pin behaviour `dev` already has and criterion 5 asks to see each red.
- Chosen against: recording every criterion `not red`, because criterion 5 asks for each test to be seen red first.
- Chosen against: a red from an empty or missing golden, because a missing fixture is the wrong reason for a red.
- Chosen against: one planted change that removes the frame's newline step, because it refuses the cloze card and so turns A1 and A3 red for A4's reason; the class plant refuses only the second cards, after A1 and A3 have failed on the first.
- Chosen against: relying on rows and the web mutation run alone, because they leave no red commit for the record, and the web run mutates no unchanged file.

### D7. Rows, and no model

- Chosen: four rows in band `S37200-S37299` on `crates/engine-core/src/face.rs`, for the face's CSS, a text node, a replacement and the `{{FrontSide}}` guard, each killed by the Rust test; no web rows; no model, because the existing rows on that file pin sound, speech and autoplay and none pins the CSS or the joining, while the web has no production change for its mutation run to judge.
- Chosen against: no rows, because a later edit to `face.rs` that dropped the CSS would then be caught only if the Rust test happened to be selected.
- Chosen against: rows on `crates/web-engine/src/wasm.rs`, because only the wasm build compiles it, and a native killer cannot observe its mutants.
- Chosen against: a TLA+ model, because the delivery adds no actor and no shared state: the golden is read-only test data.

## Decision Outcome

D1 to D7 as chosen above. One golden file under `web/app/src/lib/study` holds the inputs, the frame
CSS and the engine's render; the Rust test renders and compares, the web test drives the review
screen and reads the frame; each is seen red through a planted change that the next commits
remove; four rows pin the face's side.

## Consequences

- Good: an engine change to these cards fails the Rust test, and a frame change fails the web
  test, against one truth.
- Good: no production file, frame policy or dependency edge changes.
- Bad: the golden's render half is captured from the engine, so it is only as good as the anchors
  the Rust test asserts before comparing; an engine upgrade that changes the render needs a
  delivery that captures it again and says why.
- Bad: the red commit turns other existing tests red while its plants stand; the record names them.
- Neutral: the frame keeps taking its CSS from the head card's render with the partial flag off,
  and the Rust test pins that it equals the face's CSS and the inputs.

### Confirmation

SPEC-372's A1 to A5, the red-first record's planted reds, the band's rows proved killed, and the
pull request's `mutation-rust`, `mutation-rows` and `mutation-web` verdicts.

## What would make this wrong

- If the engine is found to rewrite a CR in a note type's CSS, criterion 3's "unchanged apart from
  newline normalisation" is read at the engine as well, and D3 is decided again.
- If the frame starts taking its CSS from the face rather than the head card, the web test's head
  CSS and face CSS stay equal, and D5 still holds; if they ever differ, the test grows a case for
  the difference.
- If a second web screen shows cards, it reads the same golden rather than a copy.
