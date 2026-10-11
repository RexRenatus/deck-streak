# SPEC-407: the card document inherits the frame policy from a host, so every engine holds the image-set forms

Filed as `docs/specs/SPEC-407-the-card-document-inherits-the-frame-policy-from-a-host-so-every-engine-holds-the-image-set-forms.md`.

- **Wave:** the app campaign, the card frame on the web (SPEC-341). **Issue:** #787. **Context(s):**
  `miniapp` (`web/app`) and docs.
- **Decided by:** ADR-421 (the cause, the host, the census and the reads, the red-first record,
  FORMAL, the order and the pushes, no row), beside ADR-416 (the strip of every `srcset`) and ADR-352
  (the card frame's layers), whose Firefox reading ADR-421 amends.
- **Schematic:** `docs/schematics/card-frame-channels.md`: section 2's components, section 3's opening
  line and W2 row, the image forms' Firefox cells, a new section 3 subsection "A variant visit, from
  the page load to the assertion, in each engine", and section 9's Firefox paragraph.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-407.md`. **Mutation
  band:** S40700-S40799, unused: the web code is proved by StrykerJS (ADR-421 D7).

## 1. The problem, measured

Citations are read at `dev` `a68db18aed95338ebdebbec0374ba320be9f4471`.

The planted card suite runs every planted card in the reference frame, in the card frame, and with
each layer off alone (`web/app/tests-card/card.spec.ts:107-140`). Its Firefox case
`W3 off: srcset stays closed` (`card.spec.ts:133`) reads arrivals on `dev` and on pull request runs.

Each job's log was read with `gh api repos/RexRenatus/deck-streak/actions/jobs/<id>/logs`, with
escape sequences stripped, by `grep`:

| job | the run's head | verdict | the arrivals the failure prints |
|---|---|---|---|
| 114191481550 | `dev` `32f62172` | 447 passed | none |
| 114224746266 | `dev` `a68db18a` | 1 failed, 446 passed | `/srcset/1` 1, `/srcset/2` 1 |
| 114200425555 | a pull request head | 1 failed, 446 passed | `/srcset/1` 1, `/srcset/2` 1 |
| 114216666394 | the same head, its second attempt | 1 failed, 446 passed | `/srcset/1` 1, `/srcset/2` 1 |
| 114215917120 | a pull request head | 1 failed, 446 passed | `/srcset/1` 1, `/srcset/2` 1 |
| 114225289308 | a pull request head | 1 failed, 446 passed | `/srcset/1` 1, `/srcset/2` 1 |
| 114225399072 | a pull request head | 1 failed, 446 passed | `/srcset/1` 1, `/srcset/2` 1 |
| 114225529870 | a pull request head | 1 failed, 446 passed | `/srcset/1` 1 |
| 114227082352 | a pull request head | 1 failed, 446 passed | `/srcset/1` 1 |

- Every log reads `Running 447 tests using 1 worker`: 149 tests in each of three engines.
- In each of the eight failing logs the one failure is `[firefox] › tests-card/card.spec.ts:133:5 › W3 off: srcset stays closed`, failing by `srcset opened with W3 off in firefox` at `card.spec.ts:137`; Chromium's and WebKit's same case pass in every log.
- Each failing run uploads `card-sandbox-results` with one entry, `card-W3-off-srcset-stays-closed-firefox/error-context.md`, read with Python's `zipfile`: the same test source and the same page snapshot in all eight; the passing run uploads none.
- `git rev-parse 32f62172:web` and `git rev-parse a68db18a:web` both read `8f0f97c33ce722603f8d7313469febd01cb9bffe`, `.github` is equal at both, and every job's install step names the same browser builds.
- The case takes the same time in passing and failing logs; in job 114224746266 the `srcset` card's pair ran minutes before it, with 87 tests between, each resetting the counts (`web/app/tests-card/listeners.ts` `reset()`, called first by `visit()` at `card.spec.ts:58-74`) and each reading nothing or its own card's paths.
- The two paths are named only by the `srcset` card (`web/app/tests-card/planted.ts:101-111`), whose W3-off variant is the raw card under the frame policy meta (`web/app/tests-card/harness/main.ts:73`).

So the arrivals are this visit's own, and whether they arrive is Firefox's fetch order: its parser
fetches an image-set candidate ahead of its tree builder, and the frame policy's meta element is not
yet part of the document's policy when it does (ADR-412 D6). W1 and W4 stop no fetch
(`docs/schematics/card-frame-channels.md:130-135`), and neither the page policy
(`web/app/svelte.config.js:24-31`) nor the edge header (`deploy/caddy/deck-streak.caddy:19`) names an
image source. With W3 off, no layer holds the two forms in Firefox by what it does (ADR-421 D1).

A local reproduction was not run; the CI record above is the evidence.

## 2. Requirements

- R1. `CardFrame`'s frame keeps exactly the attributes `sandbox` (empty), `srcdoc` and `title`, and its `srcdoc` is a host: a document whose head holds a `Content-Security-Policy` meta element carrying the host policy, then the host's style, and whose body is exactly one `iframe` with the frame's `title` and, as its `srcdoc`, the frame document `frameDocument` writes, unchanged; the host's frame carries no `sandbox` attribute.
- R2. The host policy is `img-src data:; script-src 'none'; object-src 'none'; base-uri 'none'`: each directive one the frame policy already enforces, and no `default-src`, `frame-src` or `child-src`.
- R3. The host is parsed again as the frame will parse it and refused as `escaped` unless its head reads back as written and its one frame gives back exactly the card document and the title; a host with an empty policy carries no meta element. A refused card renders a frame with no document, marked as refused, as today.
- R4. With the strip and the card document's own policy meta both off and every other layer on, neither image-set form of the `srcset` card arrives in any engine, after its reference visit reaches both.
- R5. Every test of the planted suite at `dev` stays, unchanged, in every engine, and passes; the suite runs 150 tests per engine, `W3 off: srcset stays closed` in Firefox included.
- R6. The sink census stays an exact equality and names the host's frame as the second `srcdoc` sink.
- R7. The harness builds every single-layer variant through the host, and a host the harness cannot build fails the visit; the reference frame and the scripts-on measurement stay single frames.
- R8. The threat model's citations into the card files hold at their lines: `policy.js:11`, `:18`, `:26`, `card-frame.test.ts:19`, `policy.test.ts:36`.
- R9. The schematic records W2's two points, the image-set forms' Firefox cells, and a variant visit from the page load to the assertion in each engine.

## 3. Acceptance criteria of SPEC-407

| id | criterion | decided by |
|---|---|---|
| A1 | the host holds the card document as its one frame under the host policy: the whole host for a card document and a title each holding `&` and `"`, read back byte for byte, and no meta element for an empty policy | `web/app/src/lib/card/frame-host.test.ts` "the host holds the card document as its one frame, under the host policy" |
| A2 | a host that cannot carry the card document, the title or the policy byte for byte is refused as `escaped`: a carriage return in the card document, one in the title, a double quote in the policy, each alone | `frame-host.test.ts` "a host that cannot carry the card document or its title byte for byte is refused" |
| A3 | the host policy is exactly its four directives, each enforced by the frame policy, with no frame source | `web/app/src/lib/card/policy.test.ts` "the host policy holds only what the frame policy already holds, and no frame source" |
| A4 | `CardFrame`'s `srcdoc` is the host, whose one frame, with no sandbox attribute, holds the frame document | `web/app/src/lib/card/card-frame.test.ts` "the card frame's document is the host, whose one frame holds the frame document" |
| A5 | the sink census names exactly two `srcdoc` sinks, `CardFrame.svelte` and `frame-host.ts`, once each | `web/app/src/lib/card/card-sinks.test.ts` "card HTML reaches the page only through the card frame" |
| A6 | in Chromium, WebKit and Firefox, the `srcset` card reaches both paths from the reference frame and nothing under the host policy alone (the strip and the card document's meta off) | the Playwright case `web/app/tests-card/card.spec.ts` "the host policy alone holds the image-set forms in every engine", read in `card-sandbox`; its fence line runs the Vitest test that pins the case and its harness variant |
| A7 | the whole planted suite passes in every engine, 150 tests each, `W3 off: srcset stays closed` in Firefox included | the Playwright suite `web/app/tests-card/card.spec.ts`, read in `card-sandbox`; its fence line runs the Vitest test that pins the suite's three engines |

The tdd probe resolves no Playwright command, so A6 and A7 name their Playwright spec in the table
and their fence lines run Vitest tests over the suite's text, as SPEC-402's A4 does.

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/frame-host.test.ts -t "the host holds the card document as its one frame, under the host policy"
A2: pnpm exec vitest run web/app/src/lib/card/frame-host.test.ts -t "a host that cannot carry the card document or its title byte for byte is refused"
A3: pnpm exec vitest run web/app/src/lib/card/policy.test.ts -t "the host policy holds only what the frame policy already holds, and no frame source"
A4: pnpm exec vitest run web/app/src/lib/card/card-frame.test.ts -t "the card frame's document is the host, whose one frame holds the frame document"
A5: pnpm exec vitest run web/app/src/lib/card/card-sinks.test.ts -t "card HTML reaches the page only through the card frame"
A6: pnpm exec vitest run web/app/src/lib/card/frame-host.test.ts -t "the planted suite holds the host policy case, and the harness builds it through the host"
A7: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the card config runs the planted suite in Chromium, WebKit and Firefox"
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/src/lib/card/frame-host.ts` | `miniapp` | added: `frameHost` and `hosted` |
| `web/app/src/lib/card/frame-host.test.ts` | `miniapp` | added: A1, A2 and A6's pin |
| `web/app/src/lib/card/policy.js` | `miniapp` | changed: `HOST_POLICY` appended after line 26 |
| `web/app/src/lib/card/policy.test.ts` | `miniapp` | changed: the import names `HOST_POLICY`; A3 appended |
| `web/app/src/lib/card/CardFrame.svelte` | `miniapp` | changed: the frame's `srcdoc` is the host |
| `web/app/src/lib/card/card-frame.test.ts` | `miniapp` | changed: A4 appended; lines 27 and 47 read the card document through the host |
| `web/app/src/lib/card/card-sinks.test.ts` | `miniapp` | changed: the census names two sinks (A5) |
| `web/app/src/lib/study/late-line.test.ts` | `miniapp` | changed: line 133 reads the card document through the host |
| `web/app/src/lib/study/review-screen.test.ts` | `miniapp` | changed: line 177 reads the card document through the host |
| `web/app/src/lib/study/review-templates.test.ts` | `miniapp` | changed: line 242 reads the card document through the host |
| `web/app/src/routes/study.test.ts` | `miniapp` | changed: line 67 reads the card document through the host |
| `web/app/tests-card/card.spec.ts` | `miniapp` | changed: A6 inserted after the variant loop, no line removed |
| `web/app/tests-card/harness/main.ts` | `miniapp` | changed: the variants through the host; the `W3meta` variant |
| `docs/schematics/card-frame-channels.md` | docs | changed: sections 2, 3 and 9 as the schematic amendment names |
| `docs/specs/SPEC-407-the-card-document-inherits-the-frame-policy-from-a-host-so-every-engine-holds-the-image-set-forms.md` | docs | added |
| `docs/decisions/ADR-421-the-frame-policy-is-in-force-from-the-card-documents-creation-through-a-host-document-it-inherits-from.md` | docs | added |
| `docs/red-first/SPEC-407.md` | docs | added |
| `changelog.d/card-w3-srcset-407.md` | docs | added |

## 5. What this does NOT cover

- The iPhone and iPad card view, whose own layers are measured by its own suite (#616).
- The planted suite's per-engine unobservable cells, which the held follow-up decides (#661).
- Firefox in the engine, study, end-to-end and accessibility suites (#652).
- The study screen's occlusion mask guard, which shares four of the study tests this delivery edits and re-measures after it (#778).
- A per-visit token in the arrival counter: no counting race was found (#787).
- A retry, a longer settle window, a blind cell or a narrowed expectation for the case: #787 refuses each.
- The page policy and the edge header, which stay as they are (#787).

## 6. Risks

- An engine that does not put an inherited policy in force before its fetch-ahead: A6 reads an arrival in that engine at the fix, which stops the delivery and reopens the layer table (ADR-421, "What would make this wrong").
- The host changing what the learner sees: the render proof compares the shipped frame with the reference frame pixel for pixel, unchanged.
- The host moving a channel's layer alone (the card frame's own navigation is checked against the host, which inherits the page's `frame-src`): every single-layer variant of the planted suite decides it, unchanged.
- A study screen read that still parses the host as the card: Vitest in `web` and the study suite in `web-engine`.
- A mutant of `frame-host.ts` no test kills: `mutation-web` breaks at one survivor.
- A conflict with #778's four study tests: #778 re-measures after this lands.

## 7. The planted suite's assertions, counted per engine

| | at `dev` | after |
|---|---|---|
| tests per engine (Chromium, WebKit, Firefox) | 149 | 150 |
| tests in all | 447 | 450 |
| `expect` call sites in `card.spec.ts` | 17 | 19 |
| lines removed from `card.spec.ts` | | 0 |
| planted cards, layers, unobservable entries in `planted.ts` | 29, 4, 13 | 29, 4, 13 (file unchanged) |

No assertion of the planted suite is narrowed in any engine: every existing test runs in every engine
as at `dev`, and the one new test adds a reach and an absence.
