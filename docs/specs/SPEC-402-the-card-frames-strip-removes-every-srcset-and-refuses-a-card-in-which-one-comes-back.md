# SPEC-402: the card frame's strip removes every srcset, and its re-parse check refuses a card in which one comes back

- **Wave:** the app campaign, the card frame on the web (SPEC-341). **Issue:** #771. **Context(s):**
  `miniapp` (`web/app`) and docs.
- **Decided by:** ADR-416 (this SPEC's own: the population, the strip and its check, the layer
  table, the re-plant, FORMAL, the records and the pushes) and ADR-352 (the card frame's layers and
  their planted proof, whose layer table ADR-416 amends).
- **Schematic:** `docs/schematics/card-frame-channels.md`: section 2's strip, section 3's W3 row,
  `img` row and new `srcset` row, W3's "does NOT stop" line, and section 3's new subsection "The
  image forms, from the author's HTML to the frame document".
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-402.md`. **Mutation
  band:** S40200-S40299, unused: the web code is proved by StrykerJS (ADR-416 D9).
- **Amended by:** ADR-498 (section 8: the strip removes every `target` and refuses a card in which
  one comes back, so a clicked card link opens no connection). Its rows A5 to A10 join section 3;
  its band stays the one above, unused.

## 1. The problem, measured

Read at `dev` `1414a982`.

- The strip removes elements only: `STRIPPED = 'link, meta, base, template'`
  (`web/app/src/lib/card/frame-document.ts:17`), each match removed whole (`:39`). No attribute is
  removed, so every `img` and `source` keeps its `srcset`.
- The re-parse check (`frame-document.ts:46-51`) keeps a card only when the head reads as written
  (`:48`), the body holds no stripped element (`:49`) and the body's attributes are the ones written
  (`:50`). Nothing in it reads a `srcset`, so a card whose second parse brings one back is kept.
- The planted `img` card holds seven forms (`web/app/tests-card/planted.ts:94-102`), two of them
  image-set forms: `img srcset` (`:98`, path `/img/2`) and `picture source` (`:99`, path `/img/3`).
  Its layer alone is W2 (`:96`; `docs/schematics/card-frame-channels.md:93`).
- The suite reads a card's arrival by path prefix (`web/app/tests-card/listeners.ts:111-115`), and
  the `img` card names one prefix, `/img/` (`planted.ts:65-66`). So once the strip removes `srcset`,
  the `img` card's `W2 off` variant would still pass on `/img/1` while its two image-set forms never
  arrive: their single-layer reading would end with no test failing.
- In Firefox (#766's first reading of the suite there), the `img` card's `W1 off` and `W4 off`
  variants each read `/img/2` and `/img/3` and nothing else: the engine fetches an image-set
  candidate ahead of its tree builder, typed as an image set, outside the preload types its early
  copy of the meta policy is checked for. W2 holds those two forms there only beside W1 and W4.
- Each engine runs 144 planted-suite tests at `dev`: the census test, 28 pairs, 4 x 28 single-layer
  variants, two scripts-on measurements and the render proof (`web/app/tests-card/card.spec.ts:101-194`;
  28 cards, `planted.ts:93-193`).

## 2. Requirements

- R1. The strip removes the `srcset` attribute from every element of the card that carries one,
  `img` and `source` and any other element, and keeps the element and its other attributes, its
  `src` included.
- R2. The re-parse check refuses a card whose composed document, parsed again, holds a `srcset`
  attribute on any element, the root element included.
- R3. The schematic's W3 row names the `srcset` removal and its refusal; the `img` row no longer
  lists `img srcset` or `picture source`; a new `srcset` channel row lists them, blocked by W3 and
  W2, with layer alone `-`.
- R4. The planted suite plants the two image-set forms as their own card, `srcset`, with no layer
  alone, whose reference frame must reach the path of each form (`/srcset/1` and `/srcset/2`);
  the `img` card keeps its five other forms, their paths unchanged, and its layer alone W2. No
  card other than `srcset` plants a `srcset` attribute.
- R5. No assertion of the planted suite is narrowed in any engine: `card.spec.ts` is unchanged,
  every test the base lists in each engine is listed at the head under the same title, and each
  engine's count rises from 144 to 149.
- R6. The verdicts are read by the check-run names `web` (the unit tests), `card-sandbox` (the
  planted suite), `mutation-web` (StrykerJS over `frame-document.ts`) and the aggregate `ci`.

## 3. Acceptance criteria of SPEC-402

| id | criterion | decided by |
|---|---|---|
| A1 | the frame document holds no `srcset` on an `img`, a `picture` `source` or any other element, every element is kept, and each image keeps its `src` | `web/app/src/lib/card/frame-document.test.ts` "the frame document removes every srcset and keeps each element and its src" |
| A2 | a card whose re-parse brings a `srcset` back, on an `img`, a `picture` `source` or the root element, is refused | `frame-document.test.ts` "a card whose re-parse brings a srcset back is refused" |
| A3 | the schematic's `srcset` channel is planted as its own card with no layer alone, and no other card plants a `srcset` | `web/app/src/lib/card/planted-coverage.test.ts` "every channel in the schematic has a planted card" |
| A4 | in Chromium and WebKit, the `srcset` card reaches both of its paths from the reference frame and nothing from the card frame, and stays closed with each layer off alone; the `img` card still opens with W2 off | the Playwright `web/app/tests-card/card.spec.ts` in `test:card`, read in the `card-sandbox` check-run |
| A5 | the frame document holds no `target` on an `a`, an `area`, an SVG `a`, a `form` or any other element, a `target` whose value holds a line feed and a `<` removed like any other, and every element is kept with its `href` | `web/app/src/lib/card/frame-document.test.ts` "the frame document removes every target and keeps each element and its href" |
| A6 | a card whose re-parse brings a `target` back, on an `a`, an `area` or the root element, is refused | `frame-document.test.ts` "a card whose re-parse brings a target back is refused" |
| A7 | the schematic's web channels, `nav-blank-area`, `nav-blank-svg` and `nav-show-new` among them, are the planted cards, each with its row's layer alone, `nav-top`'s and `nav-blank`'s W4 and `nav-show-new`'s W1 | `web/app/src/lib/card/planted-coverage.test.ts` "every channel in the schematic has a planted card" |
| A8 | the schematic's `BARE in <Engine> with <Wn> off` cells equal the planted suite's declared bare connections both ways | `planted-coverage.test.ts` "the schematic's bare connections equal the declared bare connections both ways" |
| A9 | in Chromium, WebKit and Firefox, `nav-top` and `nav-blank` each stay closed with W1 off and open with W4 off; `nav-show-new` opens with W1 off and stays closed with W2, W3 or W4 off; `nav-blank-area`, `nav-blank-svg` and `nav-show-new` each reach the TCP listener from the reference frame and nothing from the card frame; and each reads with each layer off alone as the schematic gives | the Playwright `web/app/tests-card/card.spec.ts` in `test:card`, read in the `card-sandbox` check-run |
| A10 | in WebKit, `nav-blank-area`'s card frame and its W2-off variant read nothing, and its W3-off variant reads one bare connection and no other key; `nav-blank-svg` reads nothing there at the base and the head | the Playwright `web/app/tests-card/card.spec.ts` in `test:card`, read in the `card-sandbox` check-run |

The tdd probe resolves no Playwright command, so A4 names its Playwright spec in the table and its
fence line runs the Vitest test that proves the suite opens every card in both frames, as
SPEC-341's A9 to A12 do.

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "the frame document removes every srcset and keeps each element and its src"
A2: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "a card whose re-parse brings a srcset back is refused"
A3: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "every channel in the schematic has a planted card"
A4: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the planted suite runs every card in the reference frame and the card frame"
A5: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "the frame document removes every target and keeps each element and its href"
A6: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "a card whose re-parse brings a target back is refused"
A7: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "every channel in the schematic has a planted card"
A8: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the schematic's bare connections equal the declared bare connections both ways"
A9: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the planted suite runs every card in the reference frame and the card frame"
A10: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the planted suite runs every card in the reference frame and the card frame"
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/src/lib/card/frame-document.ts` | `miniapp` | changed: the strip removes every `srcset`; the re-parse check refuses one; the header comment |
| `web/app/src/lib/card/frame-document.test.ts` | `miniapp` | changed: A1 and A2 added |
| `web/app/src/lib/card/planted-coverage.test.ts` | `miniapp` | changed: A3's assertion that only the `srcset` card plants a `srcset`, inside the existing test |
| `web/app/tests-card/planted.ts` | `miniapp` | changed: the `srcset` card added; the `img` card's two image-set forms moved to it |
| `docs/schematics/card-frame-channels.md` | docs | changed: section 2's strip, section 3's W3, `img` and `srcset` rows, W3's "does NOT stop", and section 3's image-forms subsection |
| `docs/specs/SPEC-402-the-card-frames-strip-removes-every-srcset-and-refuses-a-card-in-which-one-comes-back.md` | docs | added |
| `docs/decisions/ADR-416-the-strip-holds-the-image-set-forms-by-removing-every-srcset-and-they-are-planted-as-their-own-card.md` | docs | added |
| `docs/red-first/SPEC-402.md` | docs | added |
| `changelog.d/card-strip-srcset-402.md` | docs | added |
| `web/app/src/lib/card/frame-document.ts` | `miniapp` | changed by ADR-498: the strip removes every `target`; the re-parse check refuses a `target`; the header comment |
| `web/app/src/lib/card/frame-document.test.ts` | `miniapp` | changed by ADR-498: A5 and A6 added |
| `web/app/src/lib/card/planted-coverage.test.ts` | `miniapp` | changed by ADR-498: A8's test added after the unobservable cells' test |
| `web/app/tests-card/planted.ts` | `miniapp` | changed by ADR-498: `nav-top`'s and `nav-blank`'s layer alone W4; the `nav-blank-area`, `nav-blank-svg` and `nav-show-new` cards and their helper; the table of bare connections |
| `web/app/tests-card/card.spec.ts` | `miniapp` | changed by ADR-498: the single-layer variant reads a declared bare connection as `tcp` and no other key, and names it in its title |
| `docs/schematics/card-frame-channels.md` | docs | changed by ADR-498: section 2's strip node, section 3's W3, `nav-top` and `nav-blank` rows, three channel rows, and the subsection "A clicked link's target, from the author's HTML to the frame" |
| `docs/decisions/ADR-498-the-strip-removes-every-target-and-refuses-a-card-in-which-one-comes-back.md` | docs | added |
| `docs/red-first/SPEC-402.md` | docs | changed by ADR-498: A5 to A10 inserted |
| `docs/specs/SPEC-402-the-card-frames-strip-removes-every-srcset-and-refuses-a-card-in-which-one-comes-back.md` | docs | changed by ADR-498, insert-only: the header line, rows A5 to A10, these rows, sections 5 and 6 bullets, and section 8 |
| `changelog.d/card-frame-target-ward-498.md` | docs | added |

## 5. What this does NOT cover

- It runs no engine beyond Chromium and WebKit: Firefox joins the card suite in #766, whose last
  push reads Firefox over this strip.
- It does not touch the iPhone and iPad card view or its planted `img` card, whose layers are L1 to
  L7 (#765).
- It does not change CSS `image-set()`, which W2 holds alone as the `css-url` card shows, since the
  strip removes attributes of elements and never rewrites the card's CSS (#771).
- It does not give the `img` card one path per form; its reference reads by prefix as it does at
  `dev` (#771).
- It does not edit the campaign's threat model, whose card-frame citations this delivery leaves in
  place (#653).
- It does not amend SPEC-341 or ADR-352 in place; ADR-416 records the amendment of ADR-352's layer
  table, and the schematic carries it (#771).
- ADR-498 does not remove `href` or `ping` from any link (R2 of #761's route, not taken): a
  followed link stays a navigation of the frame, which W4 refuses (#761).
- ADR-498 does not point the planted suite's other clicked cards at the TCP listener, and does not
  read the `a` shape's WebKit green there: #942 does both after this ward lands, and re-derives its
  rows over it (#942, #761).
- ADR-498 does not cure the host's carriage-return refusal (#943) or census the card's raw-HTML
  sinks (#867).
- ADR-498 runs no stock Chromium or Firefox build: the suite runs Playwright's builds (#941).
- ADR-498 does not remove a submit button's `formtarget`, which W1 and W2 refuse with the form
  (follow-up item WARD-761-F1, #761).
- ADR-498 does not remove an SVG link's `xlink:show`: W1 refuses the new window it asks for, and
  the `nav-show-new` card keeps it as W1's own channel (ADR-498 D9, #761).
- ADR-498 makes no reading on Apple hardware: the reach of WebKit's preconnect to an iPhone, iPad
  or Mac reviewer is inferred from the engine's source, not measured (follow-up item WARD-761-F2, #761).

## 6. Risks

- **Firefox still opens an image form with this strip on `dev`.** Detected by #766's last push,
  whose `card-sandbox` run reads the `img` and `srcset` cards in Firefox over this strip; an opening
  is a new design, never a declaration.
- **The second parse brings a `srcset` back by a shape the tests do not plant.** The check reads
  the whole re-parsed document, so any shape is refused; A2's three plants show the class.
- **StrykerJS finds a survivor in `frame-document.ts`.** Detected by `mutation-web`, which mutates
  the changed file whole and breaks at one survivor; it is cured by a test, never a record.
- **Another delivery edits the schematic's section 3 or `planted.ts`.** Detected by the builder's
  re-measure of each shared path at its cut (#766, #765).
- **A card's only image is in a `srcset`.** It shows no image in the frame; the card's `src` is the
  image the frame shows, as the core writes it.
- **An engine opens a connection from a link with no `target`.** Detected by the new cards'
  WebKit cells in `card-sandbox` at the head; an opening is a new design, never a declaration.
- **A base reading goes against its prediction** (`nav-blank-area` reads `{}` in WebKit's card
  frame, `nav-blank-svg` reads `tcp`, or `nav-show-new` reads otherwise than W1 alone). Detected
  by push 1's `card-sandbox` run; the builder stops, and the population is re-measured (ADR-498 D5,
  D9).
- **A new card's reference frame reaches nothing in an engine** (an engine refusing the shape
  before any connection). Detected by push 1's pair test; the maintainers decide the UNOBSERVABLE
  entry, never the builder.
- **StrykerJS finds a survivor in the new conjunct.** Detected by `mutation-web`, which breaks at
  one survivor; it is cured by a test or by removing an unreachable term, never a record.
- **#942 or #943 lands a change to a shared path first.** Detected by the builder's re-measure of
  `planted.ts`, `planted-coverage.test.ts` and the schematic at its cut.

## 7. The planted suite's assertions, counted per engine

Each engine's tests are listed without running a browser, from `web/app`, by
`playwright test --config playwright.card.config.ts --list`, counted per project. At `dev`, 144 in
`chromium` and 144 in `webkit`; at the head, 149 in each: the `srcset` card's pair and its four
single-layer variants. `card.spec.ts` is unchanged, so every test runs the assertions it ran, and
every title the base lists is listed at the head. The `img` card's five tests keep their titles and
expectations; the two image-set forms, read before only inside the `img` card's prefix, are each
read by their own path in the `srcset` card's reference frame, and each single-layer variant
asserts they stay closed.

## 8. Amendment by ADR-498: the target ward's requirements and readings

### 8.1 Measured

Read at `dev` `36c4783a`.

- The strip removes `link, meta, base, template` (`web/app/src/lib/card/frame-document.ts:18`,
  `:43`) and every `srcset` (`:21`, `:45`), and names no `target`. The re-parse check (`:53-57`)
  reads no `target` and no `href`, and refuses as `escaped` (`:58`).
- #942's push 2 points every clicked planted card at the TCP listener. In its `card-sandbox` run
  the WebKit `nav-blank` card read `{ "tcp": 1 }` from the shipped card frame and from its W2, W3
  and W4 variants, 4 of 4 visits: one accepted connection, with no served request. Chromium and
  Firefox read it `{}`, and the six other clicked cards read `{}` in WebKit.
- WebKit's `HTMLAnchorElement::handleClick` preconnects to a link whose effective target is
  `_blank`, after handing the navigation off and with no sandbox check, and the sandbox refuses the
  popup itself before any load. Its `effectiveTarget()` is the `target` attribute, else the base
  target, passed through `makeTargetBlankIfHasDanglingMarkup`, which receives the target value.
  `HTMLAreaElement` derives from `HTMLAnchorElement`; `SVGAElement`'s handler never preconnects.
  How an href with dangling markup becomes a blank target is not measured.
- At `dev` every clicked card points at the HTTP listener (`web/app/tests-card/planted.ts:69-71`),
  which counts requests only (`web/app/tests-card/listeners.ts:45-51`), so `nav-blank`'s WebKit
  cells read green there: the bare connection reaches an uncounted socket.
- The TCP listener counts each accepted socket and closes it (`listeners.ts:52-55`), as the
  `preconnect` card reads it (`planted.ts:143-150`).
- Each engine lists 150 planted-suite tests at `dev`: the census test, 29 pairs, 4 x 29
  single-layer variants, the host-policy test, two scripts-on measurements and the render proof.
- Two of the 29 planted cards carry a `target`: `nav-top` (`planted.ts:170`, `target="_top"`) and
  `nav-blank` (`:171`, `target="_blank"`). They are the only cards whose layer alone is W1, and with
  the target removed each one's markup is `nav-self`'s (`:169`) with its own path. SPEC-341's census
  (`web/app/src/lib/card/planted-coverage.test.ts:281-288`) requires each layer to hold, alone, a
  channel some engine can observe; W2 holds 6 cards alone, W3 3 and W4 2.
- The reach to Apple platforms is inferred from WebKit's source (cross-platform WebCore code, and
  Cocoa builds enable server preconnect), not measured on that hardware. The iPhone and iPad card
  view closes the channel with L10 (SPEC-392 A7).

### 8.2 Requirements

- R7. The strip removes the `target` attribute from every element of the card that carries one, an
  `a`, an `area`, an SVG `a`, a `form` and any other element, and keeps the element and its other
  attributes, its `href` included; among the planted cards at `dev` that is `nav-top`'s and
  `nav-blank`'s.
- R8. The re-parse check refuses a card whose composed document, parsed again, holds a `target`
  attribute on any element, the root element included.
- R9. The schematic's W3 row names the `target` removal and its refusal; `nav-top`'s and
  `nav-blank`'s rows give W4 alone; `nav-blank-area`, `nav-blank-svg` and `nav-show-new` are
  channel rows, `nav-show-new`'s giving W1 alone; and a cell `BARE in WebKit with W3 off` names
  each declared bare connection.
- R10. The planted suite declares `nav-top` and `nav-blank` with layer alone W4 and plants three
  clicked cards, each pointed at the TCP listener and reading `tcp`: `nav-blank-area` (a full-frame
  image map whose `area` has `target=_blank`), `nav-blank-svg` (a full-frame SVG `a` with
  `target=_blank`, the control) and `nav-show-new` (a full-frame SVG `a` whose `xlink:show` is
  `new`, with no `target`, and layer alone W1), so W1 keeps a channel of its own and SPEC-341's
  census holds unchanged. Its table of bare connections equals the schematic's cells both ways,
  and each entry's variant reads `tcp` and no other key.
- R11. No assertion of the planted suite is narrowed in any engine: every test the base lists in
  each engine is listed at the head under the same title, or under the title `nav-top`'s or
  `nav-blank`'s alone move gives (`W1 off: nav-top stays closed`, `W4 off: nav-top opens`,
  `W1 off: nav-blank stays closed`, `W4 off: nav-blank opens`), and each engine's count rises from
  150 to 165.
- R12. The verdicts are read by the check-run names `web` (the unit tests), `card-sandbox` (the
  planted suite), `mutation-web` (StrykerJS over `frame-document.ts`) and the aggregate `ci`.

### 8.3 The planted readings, per engine

"tcp" is a reading whose `tcp` count is above 0; "path" one that holds the card's own path at the
HTTP listener; `{}` the empty reading. At the head:

| card | reference frame | card frame | W1 off | W2 off | W3 off | W4 off |
|---|---|---|---|---|---|---|
| `nav-top` | path | `{}` | `{}` | `{}` | `{}` | path |
| `nav-blank` | path | `{}` | `{}` | `{}` | `{}` | path |
| `nav-blank-area` | tcp | `{}` | `{}` | `{}` | `{}`; in WebKit tcp and no other key | tcp |
| `nav-blank-svg` | tcp | `{}` | `{}` | `{}` | `{}` | tcp |
| `nav-show-new` | tcp | `{}` | tcp | `{}` | `{}` | `{}` |

At push 1, the re-plant over the old strip, predicted:

| card | card frame | W1 off | W2 off | W3 off | W4 off |
|---|---|---|---|---|---|
| `nav-top` | `{}` | path: red | `{}` | `{}` | `{}`: red |
| `nav-blank` | `{}` | path: red | `{}` | `{}` | `{}`: red |
| `nav-blank-area` | `{}`; WebKit tcp: red | tcp: red | `{}`; WebKit tcp: red | `{}`; WebKit tcp | tcp in WebKit (its preconnect); `{}` elsewhere: red |
| `nav-blank-svg` | `{}` (the control) | tcp: red | `{}` | `{}` | `{}`: red |
| `nav-show-new` | `{}` (W1's own channel; the strip does not move it) | tcp | `{}` | `{}` | `{}` |

A reading against the predictions above, or a reference frame that reaches nothing, stops the
build for the maintainers (ADR-498 D5, D9).

### 8.4 The land

ADR-498 lands on `dev` before #942, which re-derives over it: its clicked cards then read
`nav-blank` at the connection in every engine, and its WebKit W3-off variant is declared in the
bare-connection table with its schematic cell (#942).
