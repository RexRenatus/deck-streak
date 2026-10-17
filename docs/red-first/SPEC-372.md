# Red-first record: SPEC-372

The SPEC, ADR-383 and the schematic were committed first (265e5b70). The red commit then carries
the two tests and the golden file beside four planted changes, one on each side of the render path
(ADR-383 D6), because the tests pin behaviour `dev` already has and criterion 5 asks to see each
test red:

- PE, `crates/engine-core/src/face.rs`: the face's `css: rendered.css,` becomes
  `css: String::new(),`, so a face carries no CSS. Restored in the engine's greening commit
  (ba90af44).
- PW1, `web/app/src/lib/study/review.ts`: the review keeps the head card's own sides instead of the
  faces' texts. Restored in the web's greening commit (c6598071).
- PW2, `web/app/src/lib/card/frame-document.ts`: the composition trims the CSS's trailing newline
  after normalising it. Restored in the web's greening commit (c6598071).
- PW3, `web/app/src/lib/card/frame-document.ts`: the class check admits only `card card1`.
  Restored in the web's greening commit (c6598071).

Each red below is quoted from the run of that criterion's fence line at the red commit, from the
worktree root, as SPEC-372 section 3 writes it; each is its test's first failing assertion. A3's
received style text is the expected text without its last newline.

While the plants stood, they also turned these existing tests red, each green again once its plant
was restored; the engine suite has no other red, as no other test there reads a face's CSS:

| test | plant |
|---|---|
| `web/app/src/lib/study/review.test.ts` "the frame shows the faces the engine completed" | PW1 |
| `web/app/src/lib/study/review-screen.test.ts` "the review screen plays the face, and offers its replay and its voices" | PW1 |
| `web/app/src/lib/study/review-screen.test.ts` "the review screen shows the card, reveals it and rates it" | PW3 |
| `web/app/src/lib/card/frame-document.test.ts` "the frame body carries the card's classes and nothing else" | PW3 |
| `web/app/src/lib/card/card-frame.test.ts` "the card's classes reach the frame body and the frame gains no attribute" | PW3 |
| `web/app/src/routes/study.test.ts` "the review route reviews the deck studied through the same engine" | PW3 |

## The fence, line by line

Each of the 5 lines of SPEC-372 section 3's fence resolves to one test: the web lines by the `-t`
filter, the engine line by `--exact`.

| # | criterion | test file | added or named |
|---|---|---|---|
| 1 | A1 | `web/app/src/lib/study/review-templates.test.ts` | added |
| 2 | A2 | `web/app/src/lib/study/review-templates.test.ts` | added |
| 3 | A3 | `web/app/src/lib/study/review-templates.test.ts` | added |
| 4 | A4 | `web/app/src/lib/study/review-templates.test.ts` | added |
| 5 | A5 | `crates/engine-core/tests/review_templates.rs` | added |

```red-first
A1: red at 4443b048: AssertionError: cloze ordinal 0 question: the body: expected '<p>the head card question</p>' to be 'The <span class="cloze" data-cloze="d…' // Object.is equality
A2: red at 4443b048: AssertionError: reversed ordinal 0 answer: the body: expected '<p>the head card answer</p>' to be 'der Hund\n\n<hr id="answer">\n\nthe d…' // Object.is equality
A3: red at 4443b048: AssertionError: cloze ordinal 0 question: the style element: expected '.card {\n  font-family: arial;\n  fon…' to be '.card {\n  font-family: arial;\n  fon…' // Object.is equality
A4: red at 4443b048: AssertionError: cloze ordinal 1 question: data-card-refused, srcdoc and status: expected { refused: 'escaped', …(2) } to deeply equal { refused: null, srcdoc: true, …(1) }
A5: red at 4443b048: assertion `left == right` failed: cloze ordinal 0: the question face's CSS left: "" right: ".card {\r\n  font-family: arial;\r\n  font-size: 20px;\r\n  text-align: center;\r\n}\r\n\r\n.cloze {\r\n  font-weight: bold;\r\n  color: blue;\r\n}\r.nightMode .cloze {\n  color: lightblue;\n}\n"
A5: green at ba90af44
A1: green at c6598071
A2: green at c6598071
A3: green at c6598071
A4: green at c6598071
```
