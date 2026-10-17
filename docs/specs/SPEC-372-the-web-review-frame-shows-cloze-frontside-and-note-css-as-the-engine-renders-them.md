# SPEC-372: the web review frame shows cloze, FrontSide and note CSS as the engine renders them

- **Issue:** `#729`. It depends on `#630` (the web study screens) and `#626` (the web engine), whose
  render path and sandboxed card frame are on `dev`. The issue asks for proof, not for a change: a
  cloze card, an answer that opens with `{{FrontSide}}` and a note type's own CSS reach the frame
  exactly as the engine rendered them, and the frame's document check refuses none of them. No
  frame policy changes.
- **Context(s):** `engine-core` (`crates/engine-core/tests`) and `miniapp`
  (`web/app/src/lib/study`).
- **Decided by:** ADR-383 (this SPEC's own). It works under ADR-361 (the review shows the faces the
  engine completed) and ADR-352 (the frame's document and its check), and changes neither.
- **Schematic:** `docs/schematics/review-templates-render-to-frame.md` (the data flow from a note's
  fields and templates to the frame's body and style element, and where each test pins it).
- **Status:** one delivery. **Mutation band:** `S37200-S37299` (section 9). **Model:** none
  (section 8).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `ebdd1371` by `git show`, `git grep -n` and `git ls-tree`.
Nothing was run: every figure is a read.

### 1a. The render path, from the engine to the frame

| step | where | at `dev` |
|---|---|---|
| The engine completes one face of a card: one render with the partial flag on, the question's nodes joined, the answer's nodes joined with `{{FrontSide}}` filled by the shown question, sound and speech tags stripped, media inlined | `crates/engine-core/src/face.rs` | `complete` `:168-224`; the request `:176-180`; `join` `:98-110`; the answer's front side `:190`; the face's CSS `:219` (`css: rendered.css,`) |
| The dispatcher's door to it | `crates/engine-core/src/dispatch.rs` | `Dispatcher::face` `:308-316` |
| The render call is admitted on both transports | `crates/engine-core/src/table.rs` | `RenderExistingCard` (27, 6) `:221-227`, native and web |
| The Worker completes both faces of the shown card | `crates/web-engine/src/wasm.rs` | `faces` `:552-574`, the two `engine.face` calls `:563-568`; `face_value` writes `text` and `css` `:582-591` |
| The Worker's head card carries the ordinal and a CSS from a second render with the partial flag off | `crates/web-engine/src/wasm.rs` | `current_card` `:396-459`; the request `:417-421`; `"ordinal"` `:442`; `"css"` `:446` |
| The review keeps the head card and replaces its two sides with the faces' texts | `web/app/src/lib/study/review.ts` | `:268-274` |
| The screen draws the shown side with the head's CSS and the classes `card card<ordinal + 1>`, and the night-mode pair in a dark palette | `web/app/src/lib/study/ReviewScreen.svelte` | `:155-160` |
| The frame builds its document from the side, the CSS and the classes, and marks a refusal | `web/app/src/lib/card/CardFrame.svelte` | `:12`; `data-card-refused` `:15` |
| The document: the classes checked; the markup parsed and stripped; the CSS's newlines normalised; the document composed, parsed again and kept only when its head, its stripped set and its body's attributes read back as written | `web/app/src/lib/card/frame-document.ts` | `frameDocument` `:36-51`; `STRIPPED` `:17`; `CLASSES` `:29`; the class check `:37`; the normalisation `:40-41`; the composition `:45`; the check `:47-50` |
| The review's own refusal message reads the same document check, without the classes | `web/app/src/lib/study/review.ts` | `escaped` `:156-161` |

### 1b. What the tests use today

| test | fixture | what it shows, and what it does not |
|---|---|---|
| `web/app/src/lib/card/frame-document.test.ts` | plain markup `:15` and one line of CSS `:16` | the body is kept `:41`; a CR LF and a lone CR in CSS read as LF `:43-46`; the classes `:96-128`. No cloze, no `{{FrontSide}}`, no note type's CSS |
| `web/app/src/lib/card/card-frame.test.ts` | a plain card, classes `card card2 nightMode night_mode` `:42` | the frame's attributes. No engine render |
| `web/app/src/lib/study/review-screen.test.ts` | `view()` `:24-36`: `<p>question N</p>`, CSS `.card { color: navy; }` | the classes `card card1`, `card card3` and night mode `:268-305`. No engine render |
| `web/app/src/lib/study/review.test.ts` | "the frame shows the faces the engine completed" `:423` | the review takes the faces' texts. Plain texts |
| `crates/engine-core/tests/face.rs` | Basic notes, and a cloned Basic whose answer is `{{FrontSide}}<hr id=answer>{{custom:Back}}` `:517-552` | expected values written by hand `:1-11`. No cloze note, no reversed card, and no test reads a face's `css` |

So no test carries an engine render of a cloze card, a reversed card or a note type's own CSS to
the frame, and none would fail if either side of that path changed.

### 1c. Three facts that shape the pin

- **The frame writes the card again.** `frame-document.ts:38` parses the markup and `:45` composes
  the document from `card.body.innerHTML`, so `<hr id=answer>` reaches the frame as
  `<hr id="answer">`. Byte equality between the engine's text and the frame's body cannot hold, and
  equality as the frame's parser reads both can (ADR-383 D4).
- **Only the CSS's newlines are normalised, and only in the frame.** `:40-41` turns CR LF and a lone
  CR into LF before the style element is composed, because the check at `:48` compares the parsed
  head with the composed one and the parser reads both forms as LF. The body needs no such step:
  the parser normalises it on both sides of any comparison (ADR-383 D3).
- **The Rust side can read a shared file with no new edge.** `crates/engine-core/Cargo.toml:24`
  carries `serde_json` among its dependencies (`:16`), and the test support module's
  `workspace()` (`crates/engine-core/tests/support/mod.rs:29-30`) names the repository root.
  `web/app/stryker.config.json:21` runs the web mutation sandbox inside `web/app`, so a file the web
  test reads must sit there (ADR-383 D1).

## 2. Requirements

R1. **One golden file.** `web/app/src/lib/study/review-templates.golden.json` holds the fixture's
    inputs written by hand (two note types with their templates and CSS, two notes with their
    fields), the CSS each note type should show in the frame written by hand with LF line ends, and
    the engine's render of each of the four cards: its note, its ordinal, its question and answer
    as `Dispatcher::face` completes them, the CSS that call returns, and the CSS of the render with
    the partial flag off. It holds no id and no clock value. No code writes it.
R2. **The fixture.** A cloze note type cloned from the stock one, its CSS carrying a `.cloze` rule,
    CR LF line ends, a lone CR and a trailing LF; a reversed note type cloned from the stock Basic
    (and reversed card), its CSS with LF line ends and a trailing LF, each answer template opening
    with `{{FrontSide}}`. One cloze note with deletions c1 and c2, and one reversed note. Every
    template and CSS is a literal in the golden, never the stock note type's own (ADR-383 D2).
R3. **The Rust test renders the fixture and compares.** `crates/engine-core/tests/review_templates.rs`
    builds a collection from the golden's inputs, renders each card through `Dispatcher::face` for
    both sides (as the Worker's `faces` does) and through `RenderExistingCard` with the partial flag
    off (as the Worker's `current_card` does for the CSS), asserts the anchors written in the test,
    and then asserts the render equals the golden's. It never writes the golden; a mismatch prints
    the render it computed.
R4. **The web test drives the review screen.** `web/app/src/lib/study/review-templates.test.ts`
    renders `ReviewScreen` with a fake client that serves each golden card: the head card carries
    the golden ordinal and CSS and two sides of its own, and the faces carry the golden texts. For
    each card and side it reads the frame's document as the frame reads it.
R5. **Newline normalisation stays where it is.** Only the CSS is normalised, CR LF and a lone CR to
    LF, in the frame, before the style element is composed. The engine's render and the golden keep
    every CR (ADR-383 D3).
R6. **No production file changes.** The delivery's net diff touches no file under
    `crates/*/src` and no production file under `web/app/src`. The frame's policy, sandbox,
    stripped set, class set and check are unchanged.
R7. **Each test is seen red first.** The red commit carries the tests beside planted changes to the
    engine's render and the frame's composition; the greening commits remove them (ADR-383 D6).
R8. **Rows pin the engine's side.** Three rows in `scripts/mutation-rows.d/S37200-S37299.json` mutate
    the face's CSS and its text joining in `crates/engine-core/src/face.rs`, each killed by R3's
    test (section 9).

## 3. Acceptance criteria

| # | criterion | red at the red commit, for this reason | test |
|---|---|---|---|
| A1 | For the cloze note's cards c1 and c2, each card's question and answer reach the frame's body equal to the engine's render of that card, as the frame's parser reads both, and the body carries `card card1` and `card card2` respectively | the planted review keeps the head card's own sides, so the frame's body is not the engine's render | `web/app/src/lib/study/review-templates.test.ts` "a cloze card's question and answer reach the frame as the engine rendered them, with its card's classes" (added) |
| A2 | For the reversed note's two cards, each answer opens with its question where the template says `{{FrontSide}}`, equal to the engine's render | the planted review keeps the head card's own sides, so the answer is not the engine's | same file, "a reversed card's answer opens with its question where its template says FrontSide" (added) |
| A3 | Each note type's CSS, a `.cloze` rule included, reaches the frame's style element unchanged apart from newline normalisation | the planted composition trims the CSS's trailing newline | same file, "the note type's css reaches the frame's style element with only its newlines normalised" (added) |
| A4 | The frame's document check refuses none of these cards, on either side | the planted check admits only `card card1`, so each second card is refused | same file, "the frame's document check refuses none of the template cards" (added) |
| A5 | A change to the engine's render of these cards fails the Rust test, and a change to the frame's composition fails the web test; each test is seen red first | the planted face returns no CSS, so the Rust test's anchor fails; the web test's reds are A1 to A4's | `crates/engine-core/tests/review_templates.rs` `the_engine_renders_the_template_cards_as_the_golden_file_records_them` (added) |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/study/review-templates.test.ts -t "a cloze card's question and answer reach the frame as the engine rendered them, with its card's classes"
A2: pnpm exec vitest run web/app/src/lib/study/review-templates.test.ts -t "a reversed card's answer opens with its question where its template says FrontSide"
A3: pnpm exec vitest run web/app/src/lib/study/review-templates.test.ts -t "the note type's css reaches the frame's style element with only its newlines normalised"
A4: pnpm exec vitest run web/app/src/lib/study/review-templates.test.ts -t "the frame's document check refuses none of the template cards"
A5: cargo test -p deck-streak-engine-core --test review_templates -- --exact the_engine_renders_the_template_cards_as_the_golden_file_records_them
```

Each enumerating assertion reports what it examined and refuses an empty population: two cloze
cards (A1), two reversed cards (A2), four cards' style elements and the inputs' CR LF and lone CR
(A3), eight faces (A4) and four rendered cards (A5). A4 carries a positive control: the same card
with CSS that ends the style element is refused, so the observation can see a refusal.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-372-the-web-review-frame-shows-cloze-frontside-and-note-css-as-the-engine-renders-them.md` | docs | added |
| `docs/decisions/ADR-383-one-golden-file-pins-the-engines-template-render-and-the-frames-composition.md` | docs | added |
| `docs/schematics/review-templates-render-to-frame.md` | docs | added |
| `docs/red-first/SPEC-372.md` | docs | added |
| `changelog.d/review-templates-372.md` | docs | added |
| `crates/engine-core/tests/review_templates.rs` | `engine-core` | added (R3) |
| `web/app/src/lib/study/review-templates.golden.json` | `miniapp` | added (R1, R2) |
| `web/app/src/lib/study/review-templates.test.ts` | `miniapp` | added (R4) |
| `scripts/mutation-rows.d/S37200-S37299.json` | scripts | added (R8) |
| `crates/engine-core/src/face.rs` | `engine-core` | planted in the red commit and restored in the next; unchanged at the push (R6, R7) |
| `web/app/src/lib/study/review.ts` | `miniapp` | planted in the red commit and restored in a later one; unchanged at the push (R6, R7) |
| `web/app/src/lib/card/frame-document.ts` | `miniapp` | planted in the red commit and restored in a later one; unchanged at the push (R6, R7) |

## 5. What this does NOT cover

- Editing a template: a template or note type is edited nowhere in this delivery; the fixture's
  templates are literals in its golden file (`#611`).
- Card scripts: no script in a template runs or is shown, and the frame's policy is unchanged
  (`#651`).
- Typed answers: no fixture template asks for a typed answer (`#611`).
- The iPhone and iPad gaps: the native client's render is not pinned here (`#666`).
- The face's fallback for a replacement other than FrontSide (a field with a filter the engine leaves
  to the reviewer): this golden's inputs never reach it, so it is not pinned here (`#729`).

## 6. Risks

- **The golden is captured from the engine.** The render half of the golden is read from the
  engine at `dev`, so on its own it would pin whatever the engine renders. Detection: the Rust
  test's anchors are written by hand before the capture and run before the comparison (the cloze
  deletion shown and hidden by ordinal, the reversed answer composed from its question, the CSS
  equal to the inputs byte for byte), and the web test's frame CSS is written by hand.
- **An engine upgrade changes the render.** The Rust test fails, which is criterion 5 working. The
  golden's render half is then captured again in a delivery that says why, and the web test reads
  the new truth with no edit of its own.
- **The golden cannot be read in a mutation copy.** A mutation run whose copy lacks the golden
  would fail every mutant for the wrong reason. Detection: a run's unmutated baseline reads the
  same file and fails first, which stops the run.
- **The engine rewrites a CR in the CSS.** Then the inputs and the render differ and the anchor
  fails at the capture; the build stops and the question goes to the seat.
- **A sibling delivery moves a path this one reads.** `crates/engine-core/tests/support/mod.rs` and
  `crates/engine-core/src/dispatch.rs` are edited by an open pull request, and the review's files
  by a coming one. Detection: the build re-measures each at its cut.

## 7. Delivered by the other pull requests

None. This delivery is whole.

## 8. Formal model

None. No actor, store or shared state is added: the golden is test data with no runtime reader or
writer, the tests read it, and every production file's net diff is empty. 0 of the 208
`@phx covers` lines under `formal/` name a file this delivery plants or reads.

## 9. Mutation rows

Three rows in `scripts/mutation-rows.d/S37200-S37299.json`, table `MUTATIONS`, crate `engine-core`,
file `src/face.rs`, each killed by `review_templates::the_engine_renders_the_template_cards_as_the_golden_file_records_them`:

| row | mutates | what a reader would lose |
|---|---|---|
| `S37201-A-FACE-KEEPS-ITS-NOTE-TYPES-CSS` | the face's `css: rendered.css,` to an empty string | the note type's CSS, `.cloze` rule included, never reaches the frame (A3, A5) |
| `S37202-A-FACE-KEEPS-ITS-TEMPLATES-TEXT` | a text node joined as empty | the template's own markup between fields vanishes, and the cloze question stops asking for deletion 1 (A1, A5) |
| `S37204-FRONTSIDE-FILLS-ONLY-FRONTSIDE` | the `{{FrontSide}}` guard negated | the reversed answer no longer opens with its question, and other fields read as the question (A2, A5) |

The existing rows on `src/face.rs` (`S34816` to `S34822`) are unchanged. No web rows: the web
mutation run mutates each changed production file under `web/app`, and this delivery's net diff
has none.

## 10. What only CI or a device proves

| # | what | where |
|---|---|---|
| C1 | The Worker's glue carries a face's `text` and `css` to the page, and the head card's `css` and `ordinal`, as the native door returns them | the wasm build and its browser suite, in CI on the pull request; this delivery changes neither |
| C2 | Every changed file's mutants and the band's rows are killed | `mutation-rust`, `mutation-rows` and `mutation-web`, on the pull request |
| V1 | On the device, a cloze card and a reversed card look as the desktop engine shows them, in light and dark | the owner, in an acceptance session |
