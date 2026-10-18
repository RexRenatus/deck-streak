---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The strip removes every target and refuses a card in which one comes back, so a clicked card link opens no connection

## Context and Problem Statement

ADR-352 gives the web card frame four layers: W1 the sandbox, W2 the frame and host policies, W3
the strip and W4 the page's `frame-src 'none'`. The schematic
`docs/schematics/card-frame-channels.md` gives the `nav-blank` card, a full-frame link with
`target=_blank`, to W1 alone (`:113`), and says W1 does not stop preconnect (`:135-136`).

#942 points every clicked planted card at the suite's TCP listener, which counts each accepted
connection. Its push 2 read the measured channel this ADR closes: in WebKit, a card's clicked
blank-target link opens one TCP connection to the host the card names, from the shipped card
frame, with W1 to W4 on, in 4 of 4 WebKit visits (the `nav-blank` pair and its W2, W3 and W4
variants). Chromium and Firefox, as the suite runs them, read the same card closed. The
connection carries no served request, but it reaches the card's host, and gives that host the
reviewer's network address and the moment of the tap.

The cause is read from WebKit's source and fits every control of that run:
`HTMLAnchorElement::handleClick` hands the navigation off, sends pings, then preconnects to the
link's URL when its effective target is `_blank` (or is a self target in the main frame), with no
sandbox check and whatever the navigation's fate; `PolicyChecker::checkNewWindowPolicy` refuses a
sandboxed popup before any load. So W1 refuses the popup, and the preconnect has already been
started. The handler is cross-platform WebCore code, and Cocoa builds enable server preconnect by
default, so the WebKit view an iPhone, iPad or Mac reviewer runs the web app in is INFERRED to
open the same connection; it is not measured on that hardware. The iPhone and iPad native card
view already closes the channel with L10 (SPEC-392), which cancels a link activation's default
before the engine's click handler runs; the web frame runs no script, so it has no counterpart.

ADR-478, under "What would make this wrong", names this reading as one that needs a cure of its
own. The route adopted for it is a ward in W3 (R1), with `area` and SVG `a` in the test
population, SVG `a` as a control. Every `file:line` below was read at `dev` `36c4783a`.

## Decision Drivers

- A card's markup is untrusted input from a shared deck (ADR-352).
- Every ward is a security control: a layer holds a channel by what it does, in every engine,
  never by an engine's exception.
- No assertion of the planted suite is narrowed in any engine, and every card keeps a reading.
- The change is one layer's: it adds no actor, no state, no page policy and no study-screen
  change.

## Decisions, and the alternatives each was chosen against

### D1. The mechanism: what the strip and its check do, which elements carry a target, and what the suite reads

The strip removes `link, meta, base, template` (`web/app/src/lib/card/frame-document.ts:18`),
each whole (`:43`), and the `srcset` attribute of every element (`:21`, `:45`), from the inert
parse (`:42`). It touches no `target`: the file names none. The re-parse check `kept` (`:53-57`)
keeps a card when the head reads as written (`:54`), the body holds no stripped element (`:55`),
the whole document holds no `srcset` (`:56`) and the body's attributes are the ones written
(`:57`), and refuses it as `escaped` otherwise (`:58`). `CardFrame.svelte:19` then renders a
frame with no document, marked `data-card-refused`, and the review screen shows its message in the
card's place (`web/app/src/lib/study/review.ts:251`).

The elements whose `target` attribute reaches the frame document at `dev` are four: `a`, `area`,
SVG `a` and `form`. `base` would be a fifth, and W3 already removes it, so no base target reaches
the card document. Their effective targets:

- An `a` with `target=_blank` takes `handleClick`'s preconnect, measured (#942's push 2).
- An `area` takes the same handler, because `HTMLAreaElement` derives from `HTMLAnchorElement`:
  inferred from WebKit's source, measured by this delivery's `nav-blank-area` card.
- An SVG `a` navigates through its own handler, which never preconnects: inferred from WebKit's
  source, measured by this delivery's `nav-blank-svg` card, a control.
- A `form`'s target is read only by a submission, which W1 (no `allow-forms`) and W2
  (`form-action 'none'`) refuse; the `form` card reads nothing in WebKit's card frame. A
  `formtarget` on a submit button is another attribute, outside this ward (D6).

WebKit's `effectiveTarget()` is the `target` attribute, else the document's base target, passed
through `makeTargetBlankIfHasDanglingMarkup`. As the cited source reads, that function receives
the target value, never the href. How an `a` or `area` href holding a newline, a carriage return or
a tab together with `<` becomes a blank target with no `target` attribute is therefore NOT
MEASURED, and no source this ADR cites shows it.

The suite reads per layer and engine through `web/app/tests-card/card.spec.ts:130-140`: a card's
variant with its layer alone off must reach its paths, and every other variant must read `{}`. At
`dev` every clicked card points at the HTTP listener (`web/app/tests-card/planted.ts:69-71`),
which counts requests only (`web/app/tests-card/listeners.ts:45-51`), so at `dev` WebKit's bare
connection from `nav-blank` reaches an uncounted socket and its cells read green. They read red
only on #942's push 2, whose TCP listener counts the socket.

Chosen against:

- Counting the carriers from the planted suite alone: rejected because the suite plants only what it plants, and the parser admits a `target` attribute on every element.
- Reading the dangling-markup path as measured: rejected because the cited source applies the check to the target value, and no reading shows an href becoming a blank target.

### D2. The cure: W3 removes every target and refuses a document holding one

`frameDocument` removes the `target` attribute from each element the selector `[target]` matches
in the inert parse, after the stripped elements go, as it removes `srcset`, and keeps the element
with its other attributes, its `href` included. Every anchor's and area's effective target is then
empty: no `target`, and no base. `handleClick`'s condition is false, since the card frame is never
the main frame, and the follow becomes the frame's own navigation, which W4 refuses before a
request in every engine, as it refuses `nav-self`'s.

The re-parse check gains one conjunct, read over the whole re-parsed document, the root element
included: no element matches `[target]`. A card that fails it is refused as `escaped`.

The refusal states its reason as `escaped`, the strip's one refusal, which `CardFrame` marks on
the frame element (SPEC-341 R4) and the review screen announces (`review.ts:251`, `:258`).

Chosen against:

- R2, removing `href` and `ping` from every `a` and `area`: not taken, because it is wider than the measured defect: it rewrites the layer cells of `nav-self`, `download`, `ping` and `javascript-url` and drops their single-layer controls.
- R3, a WebKit exception recorded in the suite for `nav-blank`'s card frame: refused, because it narrows the gate: it accepts a connection to a card-named host in the engine an Apple reviewer runs, and ADR-478 asks for a cure.
- A selector naming the carriers, `a[target], area[target]`: rejected because the parser admits the attribute on every element and SVG `a` carries it, so a carrier list decides by element where the requirement decides by attribute (ADR-416 D2).
- Setting each target to `_self` or the empty string: rejected because an attribute left is an attribute the re-parse check must tell apart from a dangerous one, where a removed one leaves nothing to read.
- Stripping the re-parsed document again instead of refusing: rejected because the shipped document would be one whose next parse nobody read (ADR-416 D3).
- A new refusal reason for a card whose `target` comes back: rejected because `review.ts:251` reads `escaped` alone, so a new reason would leave the review screen silent over a refused card, a study-screen change for a reason no reader tells apart.
- A sandbox token or a page or frame policy change: rejected because no directive governs the preconnect, which `handleClick` starts before the popup is refused.
- Refusing an `a` or `area` href that holds a newline, a carriage return or a tab together with `<`: withdrawn, because `makeTargetBlankIfHasDanglingMarkup` receives the target value, never the href (D1), and with every target removed and no base that value is empty, so that path is closed by construction; how such an href could become a blank target stays NOT MEASURED.

### D3. Production in `frameDocument` alone; the proof in Vitest, the census and the planted suite; the native view unchanged

The production change is `web/app/src/lib/card/frame-document.ts` alone: its header comment
(`:3-10`), a `target` selector beside `STRIPPED` and `SRCSET` (`:18-21`), one removal loop after
`:45`, and one conjunct in `kept` (`:53-57`). `frame-host.ts`,
`CardFrame.svelte`, `policy.js` and `review.ts` are unchanged. The iPhone and iPad card view
already closes the channel (SPEC-392's L10, read on both simulators by its A7), so no native file
changes.

The tests:

- Vitest, read on a builder's machine and in the `web` check-run: the strip removes every target
  (an `a`, an `area`, an SVG `a`, a `form` and a plain element, and a `target` whose value holds a
  line feed and a `<`, removed like any other), keeps each element and its href; and a re-parse
  that brings a target back is refused.
- The planted suite, read in the `card-sandbox` check-run by name: `nav-top`'s and `nav-blank`'s
  cells with W1 off and W4 off in every engine, and three new cards pointed at the TCP listener:
  `nav-blank-area`, `nav-blank-svg` and `nav-show-new` (D9).
- `mutation-web` mutates `frame-document.ts` whole and breaks at one survivor
  (`web/app/stryker.config.json:7-18`).

Chosen against:

- A test or harness change alone: rejected because the connection is the engine's, from the shipped frame, and only a ward closes it.
- A native change: rejected because L10 already holds the channel in the card view, read on both simulators.
- A change in `CardFrame.svelte` or the host: rejected because no script runs in either frame, so neither can cancel a click, and the strip is where the card's markup is decided.

### D4. One SPEC-402 amendment under this ADR; the ward lands first and #942 re-derives after

SPEC-402 is the strip's own SPEC, and the ward is the same shape as its `srcset` rule: an attribute
removed by name and a re-parse that refuses its return. So this delivery amends SPEC-402 insert
only: a header line naming this ADR, rows A5 to A10 in its section 3 table and fence, rows in its
manifest, bullets in its sections 5 and 6, and a new last section 8 holding the measured problem,
R7 to R12 and the readings per engine. Its band S40200-S40299 stays unused.

The ward lands on `dev` first, with its own tests, and #942 re-derives its rows after it. They
share three paths, each an OPEN-sibling overlap with no shared hunk:

- `web/app/tests-card/planted.ts`: the ward changes `nav-top`'s and `nav-blank`'s lines (`:170`,
  `:171`) and adds its three cards, a helper and the bare-connection table; #942 changes the
  `clicks` helper (`:69-71`) and the listener's comment (`:18`).
- `web/app/src/lib/card/planted-coverage.test.ts`: the ward inserts a test after `:360`; #942
  appends one after `:486`.
- `docs/schematics/card-frame-channels.md`: the ward changes `:38`, `:56`, `:88`, `:112` and `:113`,
  inserts rows after `:113` and a subsection after `:153`; #942 appends a section after `:836`.
  #943 also changes this file.

After the ward lands, #942's TCP listener reads `nav-blank` in every engine at the connection:
its card frame `{}`, its W4-off variant `tcp` and its path, and in WebKit its W3-off variant one
bare connection, which #942 declares in the ward's bare-connection table with its schematic cell;
its SPEC-461 section 7 row and its red-first lines follow. `nav-top` reads there as `nav-self`
does: #942's run read its card frame `{}` in WebKit with its `target=_top` in place, so its W3-off
variant holds no bare connection. The ward lands before the release's
closing run, so the release does not ship the channel, and #942 waits on it.

Chosen against:

- A new SPEC-481: rejected because an insert-only amendment holds every requirement and criterion, and the strip's attribute rules then stay in one SPEC.
- #942 landing first and the ward after: rejected because #942's own WebKit cells would read red until the ward lands, and the ward is the cure #942's reading asks for.
- One pull request carrying the ward and #942: rejected because #942 would grow to carry a product ward, and the suite's reading and the cure each keep their own red-first.
- The ward pointing `nav-blank` itself at the TCP listener: rejected because at `dev` that listener counts sockets only, so the card's reference and W4-off readings would drop their path, a reading that compares less, and it would pre-empt #942's change.
- Leaving `nav-top`'s or `nav-blank`'s layer alone at W1: rejected because once the strip removes the card's target, its markup is `nav-self`'s with its own path, so its W1-off variant reads `{}` and its W4-off variant opens, in every engine.

### D5. Red first per criterion, each new card's base reading predicted, no rows, no formal entry, two pushes, opus

Each criterion is red at the base for its own reason, or `not red` with the reason. The Vitest
criteria read red on a builder's machine over the old strip. The census criteria read red with the
schematic's new rows committed before the planted cards. The planted criteria read red in
`card-sandbox` on push 1, the re-plant over the old strip: `nav-top`'s and `nav-blank`'s W1-off
and W4-off cells in every engine, and in WebKit `nav-blank-area`'s card frame and W2-off cells.
#942's push 2 is the red-first evidence for the `a` shape in WebKit.

Each new card's base reading is predicted before it is read: `nav-blank-area`'s WebKit card frame
`tcp`, `nav-blank-svg`'s `{}`, and `nav-show-new` held by W1 alone in every engine (D9). A base
reading against its prediction is a stop, and so is a
reference leg that reaches nothing in an engine: the maintainers decide it, never an UNOBSERVABLE
entry the builder writes.

No mutation row: StrykerJS proves `frame-document.ts`, as ADR-416 D9 records. Every mutant of the
new lines dies: an empty selector throws in every test; an emptied attribute name leaves a target
the re-parse refuses; and the conjunct's `=== null` dies by the benign cards. The check carries no
literal, guard or fallback that its selector makes unreachable, since each would be an equivalent
survivor.

FORMAL is not applicable by surface: `frameDocument` is a total function of the card's text, its
CSS and its classes, with one caller and no shared state, so it is no protocol for a model; and a
proof would rest on the HTML parser's tree construction, which no entry under `formal/` models.
No entry can cover it, and 0 of the 280 `@phx covers` lines in the tree name it. It is closed by
a hostile population judged by independent oracles: the Vitest plants, read back by the parser,
and the planted cards, read at the listeners in three engines.

Two pushes: the planted reds are read only in CI, in the browsers, so push 1 carries the documents,
the tests and the re-plant over the old strip, and push 2 the strip and the record. Opus: the
delivery reads push 1's measurement against predictions written before it, in three engines and
four layers, and stops on any reading against them; it moves two cards' layer alone in the planted
table, the schematic and the census together; and its mutation screen must leave no survivor in
the new conjunct.

Chosen against:

- One push with the fix: rejected because the planted reds would never be read, and a test whose red nobody saw proves nothing about the strip.
- Hand rows in the band S40200-S40299: rejected because web code with no row table is proved by StrykerJS, and a row would mutate what StrykerJS already mutates.
- A model of the engine's click handler and its preconnect: rejected because that interleaving is the engine's, not this code's, and the planted suite reads its outcome in the engine.

### D6. What is not in this delivery

- R2's `href` and `ping` strip: not taken (D2; #761).
- The suite's own connection reading, which points every clicked card at the TCP listener (#942,
  #761).
- The carriage-return cure of the host (#943).
- The census of the card's raw-HTML sinks (#867).
- Stock Chromium and Firefox builds, which the suite does not run (#941).
- The `a` shape's WebKit green at the TCP listener, read by #942's run after it re-derives (#942).
- A `formtarget` on a submit button, and a measured reading on Apple hardware: each is a follow-up
  of its own, which SPEC-402 section 5 cites.

Chosen against:

- Widening the strip to `formtarget` here: rejected because a form submission is refused by W1 and W2 and never reaches the anchor's handler, so it is wider than the measured defect.

### D7. The suite reads WebKit's bare connection as its own cell, bound to the schematic both ways

With W3 off, a link's `target` returns, W1 refuses the popup, and WebKit's handler preconnects:
the variant reads one bare connection and nothing else. That cell is the control that W3 holds
WebKit's bare connection alone, while W4 holds the navigation. `planted.ts` gains a table of bare
connections, each an engine, a card, a layer and its measured reason. `card.spec.ts`'s variant
test reads a declared entry as reaching `tcp` and holding no other key, and names it in its title.
`planted-coverage.test.ts` holds the table equal to the schematic's `BARE in <Engine> with <Wn>
off` cells, both ways, as SPEC-399 holds the UNOBSERVABLE cells. At this delivery the table holds
`nav-blank-area` in WebKit with W3 off.

Chosen against:

- Keeping "stays closed" for that cell: rejected because the variant restores the target and the engine preconnects, so the cell would read red at every head.
- An UNOBSERVABLE entry: rejected because the channel is observable there; the entry would read the card's reference as blind.
- Skipping the cell in WebKit: rejected because a skip narrows the suite.
- A per-engine layer alone on the card: rejected because the bare connection and the navigation are two readings of one card, held by two layers, and a second alone would hide which.

### D8. The schematic's section 3 rows change in place where the census reads them

`planted-coverage.test.ts:197-212` reads each channel's layer alone from its row, so `nav-top`'s and
`nav-blank`'s rows (`:112`, `:113`) and W3's row (`:88`) change in place, as SPEC-402 changed W3's
and `img`'s rows, and section 2's strip node and its edge (`:38`, `:56`) name the target removal.
The three new channels' rows and a new subsection, "A clicked link's target, from the author's HTML
to the frame", are inserted.

Chosen against:

- An insert-only schematic amendment: rejected because the census would read `nav-top`'s and `nav-blank`'s rows at W1 while their cards declare W4.
- A new schematic: rejected because the census reads this file's section 3.

### D9. W1 keeps a channel of its own: an SVG link that asks for a new window with no target

At `dev` two planted cards carry a `target`, `nav-top` (`web/app/tests-card/planted.ts:170`,
`target="_top"`) and `nav-blank` (`:171`), and they are the only two whose layer alone is W1.
With every target removed, each one's markup is `nav-self`'s (`:169`) with its own path, so W4
holds each alone, and no planted card is left that W1 holds alone. SPEC-341's census
(`planted-coverage.test.ts:281-288`, its A10) requires each layer to hold a channel alone that
some engine can observe. W1's own definition (the schematic's `:86`) names what is left to it.
Forms, scripts and plugins are W2's too, and so is the script-borne reach the opaque origin limits
(`:116`, `:119-125`); the download link is W4's (`:114`); and a popup or a top navigation asked for
by a `target` or a `base` is now W3's too. What remains is a popup the card asks for with no
`target` at all. Such a window would open the card's URL as a page of its own, giving the host the
card names the reviewer's network address and the moment of the tap, as the measured channel does,
so it is a channel, and W1 is the one layer that refuses it.

So the suite plants `nav-show-new`: a full-frame SVG `a` whose `xlink:show` is `new`, the SVG
link attribute that asks for a new window, with no `target`, pointed at the TCP listener and
clicked. The strip leaves `xlink:show`, since it is not a `target`. Its cells are predicted from
`nav-blank`'s at `dev` and are NOT MEASURED: each engine is predicted to give the link the blank
target a `target` would, so the sandbox refuses its popup as it refuses `nav-blank`'s. The card
then reads `tcp` from the reference frame and with W1 off, and `{}` from the card frame and with
W2, W3 or W4 off, at the base and the head, in all three engines. In WebKit it is also a standing
control: SVG `a`'s handler starts no preconnect (D1), so its card frame reads `{}` at every head,
where `nav-blank-svg` carries a target only at the base.

Chosen against:

- Re-deriving an existing card as W1's: rejected because 0 cards at `dev` are W1's alone without a `target`, and the strip moves no card's cells but the two carriers'.
- A modifier click on a plain link: rejected because that window is the reviewer's gesture, not the card's markup, and it adds a click mode to the suite's visit (`web/app/tests-card/card.spec.ts:58-74`) and to the card type (`planted.ts:25-41`).
- Leaving `nav-top`'s `target=_top`, or any one target value: rejected because it narrows R7 to a list of values, which D2 refuses.
- Changing SPEC-341's census so that W1 may hold nothing alone: rejected because it narrows the gate: a layer whose removal opens nothing is a layer no test reads.
- Stripping `xlink:show` too: rejected because it is wider than the measured defect, since SVG `a`'s handler starts no preconnect, and it would leave W1 nothing alone.
- Pointing the card at the HTTP listener: rejected because that listener counts requests only (`web/app/tests-card/listeners.ts:45-51`), so a WebKit preconnect from the SVG link would go uncounted.

## Decision Outcome

W3 removes every `target` and refuses a frame document that holds one; `nav-top`'s and
`nav-blank`'s layer alone move from W1 to W4, and W1 keeps a channel of its own in `nav-show-new`;
`area` and SVG `a` are planted, SVG `a` as a control; WebKit's bare connection is read as its own
cell; and no assertion of the suite is narrowed.

### Consequences

- Good: a clicked card link opens no connection from the shipped frame in WebKit, Chromium or
  Firefox, by what W3 does, whatever an engine's click handler starts.
- Good: the layer table states what holds each reading: W4 the navigation, W3 WebKit's bare
  connection.
- Bad: a card whose link opened a new window opens nothing; its follow is the frame's own
  navigation, which W4 refuses, as it refuses every other link's.
- Good: every layer still holds a channel of its own, W1 in `nav-show-new`, so SPEC-341's census
  reads the four single-layer variants as before.
- Neutral: each engine runs 165 planted tests at `dev`, 15 more: three pairs and twelve variants.

### Confirmation

`web` runs A5 to A8's Vitest tests, `card-sandbox` runs A9 and A10 in Chromium, WebKit and
Firefox, and `mutation-web` mutates `frame-document.ts` whole; `docs/red-first/SPEC-402.md`
records each red and green.

## What would make this wrong

- An engine opens a connection from a link with no `target` in the shipped frame: some path other
  than the effective target reaches a preconnect (a modifier click, a link preview), and D1's
  population is incomplete.
- `nav-blank-area`'s WebKit card frame reads `{}` at the base: `area` does not take the anchor's
  handler in the suite's build, and the population is re-measured.
- A reference leg of a new card reaches nothing in an engine: that engine refuses the shape before
  any connection, and the maintainers decide its UNOBSERVABLE entry.
- The Apple reach, inferred here, reads otherwise when measured on that hardware.
- `nav-show-new` reads otherwise than W1 alone in an engine: that engine gives `xlink:show` no
  blank target, or its SVG link handler starts a connection, and W1's channel and D1's population
  are re-measured before any build goes on (D9).

## More Information

- #761 (the question this answers); #942 (the suite's connection reading, whose WebKit red is this
  delivery's red-first evidence); #943 and #867 (sibling deliveries on the card frame).
- ADR-352 D4 and D7; ADR-416 D2, D3, D6 and D9; ADR-478; SPEC-341 R4 and A10; SPEC-392 (L10).
- SPEC-402 section 8; the schematic `docs/schematics/card-frame-channels.md`, sections 2 and 3.
- WebKit's source: `HTMLAnchorElement::handleClick` and `effectiveTarget`, `HTMLAreaElement`,
  `SVGAElement`, `PolicyChecker::checkNewWindowPolicy`, the `LinkPreconnect` preference and
  `ENABLE_SERVER_PRECONNECT` for Cocoa.
