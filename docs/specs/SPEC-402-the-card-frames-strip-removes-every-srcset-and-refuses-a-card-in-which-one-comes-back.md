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

The tdd probe resolves no Playwright command, so A4 names its Playwright spec in the table and its
fence line runs the Vitest test that proves the suite opens every card in both frames, as
SPEC-341's A9 to A12 do.

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "the frame document removes every srcset and keeps each element and its src"
A2: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "a card whose re-parse brings a srcset back is refused"
A3: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "every channel in the schematic has a planted card"
A4: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the planted suite runs every card in the reference frame and the card frame"
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

## 7. The planted suite's assertions, counted per engine

Each engine's tests are listed without running a browser, from `web/app`, by
`playwright test --config playwright.card.config.ts --list`, counted per project. At `dev`, 144 in
`chromium` and 144 in `webkit`; at the head, 149 in each: the `srcset` card's pair and its four
single-layer variants. `card.spec.ts` is unchanged, so every test runs the assertions it ran, and
every title the base lists is listed at the head. The `img` card's five tests keep their titles and
expectations; the two image-set forms, read before only inside the `img` card's prefix, are each
read by their own path in the `srcset` card's reference frame, and each single-layer variant
asserts they stay closed.
