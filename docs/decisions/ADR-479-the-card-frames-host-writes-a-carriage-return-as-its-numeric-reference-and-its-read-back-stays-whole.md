---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The card frame's host writes a carriage return as its numeric reference, so its read-back carries it and stays whole

Filed as `docs/decisions/ADR-479-the-card-frames-host-writes-a-carriage-return-as-its-numeric-reference-and-its-read-back-stays-whole.md`.
It decides SPEC-462 (#798). Citations are read at DeckStreak `dev` `83846ea3929ab3805c1e95db7291097fde559cd2`.

## Context and Problem Statement

#792 (SPEC-407, ADR-421) put the card document inside a host: `CardFrame`'s frame holds a host
document whose one nested frame carries the card document as its `srcdoc`, and the host is parsed
again as the frame will parse it and refused as `escaped` unless it reads back as written (ADR-421
D2). #798 reports that a card whose note field or template carries a carriage return written as a
numeric character reference (`&#13;` or `&#xD;`) is refused there, so the learner sees an empty
frame where the card rendered before #792. The path, read at `dev` by `git show` and `grep -n`:

| step | what happens to the carriage return | where |
|---|---|---|
| 1 | the engine completes the face: the template's render, the AV tags stripped, the media names rewritten; the tree's own code on that path holds no HTML parse and re-serialisation, so the reference stays six characters | `crates/engine-core/src/face.rs:188-211`, `crates/engine-core/src/media.rs:170` |
| 2 | `frameDocument` parses the card as a body, and the parser resolves the reference to U+000D as it stands (the HTML standard's control-character-reference rule); a raw carriage return in the field never survives this parse, because input-stream preprocessing reads it as a line feed | `web/app/src/lib/card/frame-document.ts:42` |
| 3 | the composed frame document is the parsed body's `innerHTML`, and HTML serialisation escapes `&`, U+00A0, `<` and `>` in text and `&`, U+00A0 and `"` in an attribute value, never U+000D, so the frame document holds a raw carriage return | `frame-document.ts:51` |
| 4 | `frameDocument`'s own read-back compares the head, the stripped elements, the `srcset` attributes and the body's attributes, never the body's text, so it keeps the card (the card's CSS has its carriage returns normalised before its head is compared) | `frame-document.ts:47`, `:53-58` |
| 5 | `CardFrame` hands the frame document and the frame's title to `frameHost` | `web/app/src/lib/card/CardFrame.svelte:15-16` |
| 6 | the host's attribute escaping writes `&` as `&amp;` and `"` as `&quot;` and leaves the carriage return raw in the host | `web/app/src/lib/card/frame-host.ts:15-22`, `:45` |
| 7 | the host is parsed again; preprocessing reads the raw carriage return as a line feed before the attribute value is tokenised, so the one frame gives back a line feed where the card document held a carriage return, `hosted(host) === card` is false, and the host is refused as `escaped` | `frame-host.ts:32-36`, `:46-51` |
| 8 | `CardFrame` renders a frame with no document, marked `data-card-refused="escaped"`; the study screen's status region reads only `frameDocument`'s refusal, so it says nothing | `CardFrame.svelte:19`, `web/app/src/lib/study/review.ts:247-252` |

The refusal is written into the code and its tests: the comment at `frame-host.ts:15-19` says a
carriage return is left as written and refused, and `web/app/src/lib/card/frame-host.test.ts:49-50`
pin it as SPEC-407's A2. Before #792 the frame's `srcdoc` was the frame document itself, carriage
return included, and the frame's own parse read it as a line feed, so the card rendered.

The class the issue names is a character the escaping leaves as it is and the parser changes. By
the HTML standard's input-stream preprocessing and its double-quoted attribute value state, it
holds two characters. U+000D, alone or before a line feed, is read as a line feed. U+0000 is
replaced by U+FFFD, and its reference `&#0;` resolves to U+FFFD too, so no attribute value can
carry it; the parser never leaves one in a document it builds, so no frame document holds one, and
`frame-host.test.ts:73-74` already pin its refusal. No other code point changes inside a
double-quoted attribute value: `&` and `"` are the two the escaping writes.

The iPhone and iPad card view hands the card to its web view as one string
(`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:55-65`), after a strip that
keeps every byte but a link opener (`ios/CardIsolation/Sources/CardIsolation/LinkStrip.swift:17-27`).
It builds no host, escapes no attribute and parses nothing back; its L12 read-back is the handed
string kept with the view (`CardWebViewFactory.swift:51-53`, `:251-260`).

Not measured: the dependency's renderer behind step 1 (not in this tree, not read), and any run.
The reds SPEC-462 plans are the first measurement of the refusal itself.

## Decision Drivers

- The host's read-back is a security control (ADR-421 D2, SPEC-407 R3): nothing here may loosen it, skip it for some cards, or compare less than it does.
- #798's expected behaviour: the card renders as it did before #792, and the escaping and the read-back agree on a carriage return.
- One site that the card document and the title both pass through.
- No file shared with open work on the card frame: the census of the card's sinks (#867) and the card suite (#761).

## Decisions, and the alternatives each was chosen against

### D1. The mechanism: the reference becomes a raw carriage return in the frame document, and the host's escaping leaves it raw for its parse to change

The carriage return enters the frame document at `frame-document.ts:42` (the parse resolves the
reference) and `:51` (the serialisation writes U+000D raw). The host's escaping
(`frame-host.ts:20-22`) leaves it raw, and the host's parse (`:46`) reads it as a line feed before
the comparison at `:49`, which refuses the card. Two characters belong to the class: U+000D, and
U+0000, which no frame document can hold and no attribute value can carry. The engine's in-tree
path (step 1) and the dependency's renderer are named, and the second is not measured.

Chosen against:
- The carriage return entering at the engine's render: rejected because the tree's code on that path parses no HTML (`face.rs:188-211`, `media.rs:170`), and a raw carriage return the engine wrote would be read as a line feed at `frame-document.ts:42` and never reach the host; only the reference survives to the host.
- Counting U+00A0 in the class: rejected because an attribute value carries it unchanged; the policy's refusal of it (`frame-host.test.ts:72`) comes from the head's serialisation writing `&nbsp;`, and the policy is the module's own constant, never a card's text.

### D2. The cure: the escaping writes a carriage return as `&#13;`, after the ampersand and the quote, and the read-back is unchanged

`attribute()` in `frame-host.ts` writes `&` as `&amp;`, then `"` as `&quot;`, then U+000D as
`&#13;`. The parser resolves `&#13;` inside an attribute value to U+000D itself, so the host's one
frame gives back the card document and the title byte for byte, and the three comparisons at
`frame-host.ts:47-50` hold as they are written. The nested frame then parses exactly the document
the frame parsed before #792, carriage return included, and reads it as the line feed it read then,
so the card renders as it did. The ampersand goes first, so the reference the third step writes is
never written again as `&amp;#13;`.

The read-back compares what it compared at `dev`: the whole head, the whole card document, the
whole title. It still refuses a host it cannot carry: a NUL in the card document or the title, and a
double quote in the policy. This option narrows nothing.

Chosen against:
- The refusal stating its reason in the study screen's status region: rejected because the card would still not render, though a carriage return escapes nothing and the card rendered before #792, and `review.ts` and `review.test.ts` are open in #778 and #842.
- Normalising a carriage return to a line feed in the frame document, as `frame-document.ts:47` does for the CSS: rejected because it is a second site that `frameHost` does not share, so `frameHost` would still refuse a carriage return from any other caller (the harness's hosts, `web/app/tests-card/harness/main.ts:66-68`), and the nested frame would no longer hold the document the frame held before #792.
- Comparing the read-back after normalising both sides' line endings, or skipping it for a card that holds a carriage return: rejected because each compares less than the read-back does now, so each narrows the gate ADR-421 D2 set.
- Building the host through DOM calls: rejected because ADR-421 D3 rejected it already, as a second `srcdoc` sink with no string for the read-back to read.
- Writing the reference in hexadecimal, `&#xD;`: rejected because the parser reads both as the same character, and one spelling keeps the host's text and the tests' expectations one string.

### D3. Production or test: one production file, and no cure for the iPhone and iPad card view

One production file changes: `web/app/src/lib/card/frame-host.ts`, its `attribute()` and the
comment above it. It pins the behaviour of SPEC-462's A1 to A4. `CardFrame.svelte`,
`frame-document.ts`, `policy.js` and `review.ts` stay as at `dev`. Web code with no row table is
proved by StrykerJS, so no row guards it (D5).

The iPhone and iPad card view needs no cure and no issue: it has no host, no attribute escaping and
no read-back by parse (Context), so it has no step at which a carriage return could come back
changed.

Chosen against:
- Changing the tests alone, so SPEC-407's A2 accepts a carriage return without the escaping writing it: rejected because the shipped read-back would still refuse the card, and the learner's empty frame would stay.
- A matching criterion or issue for the iPhone and iPad card view: rejected because it holds no host and no attribute read-back, so a criterion there would assert something the code cannot do.

### D4. Shape and split: one delivery, SPEC-407 amended insert-only, ADR-421 unchanged

One delivery: SPEC-462, this ADR, the band `S46200-S46299` (unused, D5), the red-first record
`docs/red-first/SPEC-462.md`, one builder. SPEC-407's A2 names a carriage return in the card
document and one in the title as hosts the read-back refuses; that statement changes, so SPEC-407
gains a new last section, insert-only, after its last line. ADR-421's D2 states the read-back (the
head as written, the card document and the title exactly), and that statement still holds word for
word, so ADR-421 is not amended. `docs/schematics/card-frame-channels.md` owns the card frame and
gains a section 13, insert-only.

Chosen against:
- Two deliveries, the escaping first and the status region's reason second: rejected because #798 is met when the card renders, so the second is no part of it, and it would share `review.ts` and `review.test.ts` with #778 and #842; it is tracked as #938.
- A new schematic file: rejected because `card-frame-channels.md` owns the card frame and SPEC-407's own schematic change was an insert there.
- Amending ADR-421: rejected because no sentence of it states that a carriage return is refused, and its read-back is kept as written.

### D5. Tests, rows, FORMAL, pushes and the model

Four criteria are red first, each a Vitest test under jsdom that fails by an assertion at the base
on every run, since none reads a clock, the network or a random value: A1 (a carriage return in the
card document or the title is carried), A2 (a field's reference passes `frameDocument` and then the
host), A3 (`CardFrame` renders such a card), A4 (among the first 256 code points the host refuses
exactly U+0000, in the card document and in the title, an enumerating test that reports its
examined count). A5 is not red: SPEC-407's A2 test, its two carriage-return plants restated as NUL
plants, guards the refusal the base already makes. Every oracle is a parse the test makes itself
with `DOMParser`, never the module's `hosted()`.

No mutation row: StrykerJS's `mutation-web` mutates the changed production file whole and breaks
at one survivor, and the band `S46200-S46299` stays unused. FORMAL is not applicable to Lean or TLA+
(D5's last two alternatives). The delivery has ONE push, because every red is read locally. The
build's model is sonnet.

Chosen against:
- A planted card in the browser card suite as the red: rejected because `web/app/tests-card/` is the surface #761 is changing and `planted-coverage.test.ts` pins the planted cards, while the unit reds decide each criterion under jsdom every time; a planted card with a carriage return reference is tracked as #939.
- Hand rows in `S46200-S46299`: rejected because web code with no row table is proved by StrykerJS (ADR-421 D7), and a row's killer would be a test that reads the TypeScript as text.
- A Lean proof of the escaping round trip: rejected because no cover can name a TypeScript file and ADR-421 D5 declined the same proof; A4's population, judged by the test's own parse, is the independent oracle.
- A TLA+ model: rejected because the host is one synchronous composition of immutable strings with no second actor and no shared state.
- Two pushes: rejected because no red needs CI to be read.
- Opus for the build: rejected because the change is one escaping line and five tests over a decided design, with no Rust, no formal entry and no census to move.

### D6. Not in this delivery

The connection observable in the card suite (#761), the link strip (#664, #677) and the census of
the card's sinks (#867) are other work on the same frame and stay out. The study screen's quiet
status region on a host refusal, and a browser-engine planted card with a carriage return
reference, have no issue yet: SPEC-462's Risks name them, and each is proposed as an issue of its
own.

Chosen against:
- Folding the connection observable or the link strip in: rejected because each changes the card suite or the iPhone and iPad card view, neither of which this cure touches, and each has its own issue.
- Folding the census of the card's sinks in: rejected because the cure adds no sink and #867 already changes that census.

## Decision Outcome

`attribute()` writes U+000D as `&#13;` after `&` and `"`. A card whose field holds a carriage
return reference passes the host's read-back unchanged and renders as it did before #792. The
read-back compares the whole head, card document and title, as at `dev`, and refuses a NUL in the
card document or the title and a double quote in the policy.

### Consequences

- Good: the card renders again; the host's escaping and its parse agree on every code point an attribute value can carry.
- Good: the read-back's population is stated as a test: among U+0000 to U+00FF it refuses exactly U+0000.
- Neutral: SPEC-407's A2 test changes two plants, from a carriage return to a NUL, under its own title, and SPEC-407's new last section records why; no assertion is removed and the test keeps three refusals, each alone.
- Bad: a host refusal from any future cause still leaves the status region quiet, because `review.ts` reads only `frameDocument`'s refusal; the proposed issue owns it.

### Confirmation

SPEC-462's A1 to A4 read red at the red commit and green at the fix, quoted in
`docs/red-first/SPEC-462.md`; A5 stays green throughout. CI's `web` runs every Vitest file,
`mutation-web` mutates `frame-host.ts` whole, and `card-sandbox` runs the planted suite in every
engine, unchanged.

## What would make this wrong

- An engine whose parser resolves `&#13;` inside an attribute value to anything but U+000D: the read-back would refuse the card there, as it does at `dev`, so the frame fails closed and never renders a changed card; the browser planted card the Risks propose would show it.
- A frame document that holds a U+0000: the host would refuse it as at `dev`, and the parser's rules say no parsed document holds one, so a refusal of that kind would name a parse this ADR did not read.
- A caller of `frameHost` other than `CardFrame` and the harness that passes text the parser changes in another way: A4's population names every code point up to U+00FF; a code point above it would need its own measurement.

## More Information

- Issue #798; #792, SPEC-407 and ADR-421 (the host and its read-back); SPEC-341 and ADR-352 (the card frame's layers).
- The HTML standard's input-stream preprocessing (newline normalisation), its double-quoted attribute value state, its numeric character reference rules and its HTML fragment serialisation algorithm are the rules this ADR's mechanism cites.
