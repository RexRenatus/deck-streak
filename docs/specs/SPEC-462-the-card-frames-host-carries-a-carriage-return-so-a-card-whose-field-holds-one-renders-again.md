# SPEC-462: the card frame's host carries a carriage return, so a card whose field holds one renders again

Filed as `docs/specs/SPEC-462-the-card-frames-host-carries-a-carriage-return-so-a-card-whose-field-holds-one-renders-again.md`.

- **Wave:** the app campaign, the card frame on the web (SPEC-341). **Issue:** #798. **Context(s):**
  `miniapp` (`web/app`) and docs.
- **Decided by:** ADR-479 (the mechanism, the cure, production or test, the shape, the tests and
  rows, what stays out), beside ADR-421 (the host and its read-back), which it leaves as written.
- **Amends:** SPEC-407, insert-only: a new last section on what A2 names as refused.
- **Schematic:** `docs/schematics/card-frame-channels.md`, a new section 13, "A carriage return from
  a card's field to the host's read-back", insert-only.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-462.md`. **Mutation
  band:** S46200-S46299, unused: the web code is proved by StrykerJS (ADR-479 D5).

## 1. The problem, measured

Citations are read at `dev` `83846ea3929ab3805c1e95db7291097fde559cd2`, each by
`git show <sha>:<path>` with `grep -n` or a line range.

A card whose note field or template carries a carriage return written as a numeric character
reference (`&#13;` or `&#xD;`) is refused by the card frame's host, and the learner sees an empty
frame (#798). The path:

| step | the carriage return | where |
|---|---|---|
| the engine completes the face | kept as the six characters of its reference: the tree's code on this path parses no HTML | `crates/engine-core/src/face.rs:188-211`, `crates/engine-core/src/media.rs:170` |
| `frameDocument` parses the card as a body | the reference is resolved to U+000D; a raw carriage return would be read as a line feed here | `web/app/src/lib/card/frame-document.ts:42` |
| the frame document is composed | the body's serialisation writes U+000D raw | `frame-document.ts:51` |
| `frameDocument`'s read-back | compares the head, the stripped elements, `srcset` and the body's attributes, not the body's text, so the card is kept | `frame-document.ts:53-58` |
| `CardFrame` builds the host | passes the frame document and the frame's title to `frameHost` | `web/app/src/lib/card/CardFrame.svelte:15-16` |
| the host's escaping | writes `&` and `"` only, so U+000D stays raw in the host | `web/app/src/lib/card/frame-host.ts:20-22`, `:45` |
| the host's read-back | the parse reads the raw carriage return as a line feed, the one frame gives back a line feed, and the host is refused as `escaped` | `frame-host.ts:46-51` |
| what the learner sees | a frame with no document, marked `data-card-refused="escaped"`; the status region reads only `frameDocument`'s refusal, so it is quiet | `CardFrame.svelte:19`, `web/app/src/lib/study/review.ts:247-252` |

The refusal is pinned at `dev`: `web/app/src/lib/card/frame-host.test.ts:49-50` expect a carriage
return in the card document and one in the title to be refused (SPEC-407's A2), and the comment at
`frame-host.ts:15-19` says so. No test at `dev` passes a field's reference through `frameDocument`
and then the host: `frame-host.test.ts:63` passes the literal text `&#13;` to the host alone, which
its escaping writes as `&amp;#13;`.

The characters the escaping leaves as they are and the parser changes, by the HTML standard's
input-stream preprocessing and its double-quoted attribute value state:

| character | what the parser makes of it | can an attribute value carry it | reaches the host from a card |
|---|---|---|---|
| U+000D, alone or before a line feed | a line feed | yes, as the reference `&#13;`, which resolves to U+000D | yes, through `frame-document.ts:42` and `:51` |
| U+0000 | U+FFFD | no: `&#0;` resolves to U+FFFD too | no: the parser leaves none in a document it builds; `frame-host.test.ts:73-74` pin its refusal |

No other code point changes inside a double-quoted attribute value. The frame's title is one of two
messages in seven locales, and none holds a carriage return or a NUL
(`web/app/messages/*.json`, `study_question_frame` and `study_answer_frame`). The iPhone and iPad
card view builds no host and parses nothing back
(`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:55-65`).

The refusal itself is not yet measured by a run: A1 to A4 are its first measurement.

## 2. Requirements

- R1. `attribute()` in `web/app/src/lib/card/frame-host.ts` writes `&` as `&amp;`, then `"` as `&quot;`, then U+000D as `&#13;`; nothing else in the host changes.
- R2. A card document or a title holding a carriage return (alone, before a line feed, after a line feed, inside an attribute value of the card, or beside an ampersand and a double quote) is kept, and the host's one frame gives back the card document and the title byte for byte, read by a parse the test makes itself.
- R3. A card whose field carries a carriage return as a numeric reference (decimal or hexadecimal, alone or before a line feed's reference, in text or in an attribute value) is kept by `frameDocument` and then by `frameHost`, and the host's one frame holds the frame document byte for byte.
- R4. `CardFrame` renders such a card: its frame carries a `srcdoc` and no `data-card-refused`, and the card document it carries reads the carriage return as the line feed the frame read before SPEC-407.
- R5. The host's read-back is unchanged, and among the code points U+0000 to U+00FF it refuses exactly U+0000, in the card document and in the title, as an enumerating test that reports its examined count; a double quote in the policy is still refused.
- R6. Every test at `dev` stays and passes, except that SPEC-407's A2 test restates its two carriage-return plants as a NUL in the card document and a NUL in the title, under its own title, with its policy plant and its kept control unchanged.
- R7. The planted card suite runs unchanged in every engine and passes, and the census of the card's sinks still names `frame-host.ts` once.
- R8. No other production file changes: `CardFrame.svelte`, `frame-document.ts`, `policy.js`, `review.ts` and the iPhone and iPad card view stay as at `dev`.

## 3. Acceptance criteria of SPEC-462

| id | criterion | decided by |
|---|---|---|
| A1 | a carriage return in the card document or the title, alone, before or after a line feed, in an attribute value, or beside an ampersand and a quote, is kept and read back byte for byte, and the host's text holds its reference | `web/app/src/lib/card/frame-host.test.ts` "a carriage return in the card document or the title is carried byte for byte" |
| A2 | a field holding `&#13;`, `&#xD;` or `&#13;&#10;`, in text or in an attribute value, passes `frameDocument`, whose document holds U+000D, and then the host, whose one frame holds that document byte for byte | `frame-host.test.ts` "a carriage return reference in a card field reaches the nested frame as the frame document wrote it" |
| A3 | `CardFrame` renders a card whose field holds a carriage return reference: a `srcdoc`, no refusal mark, the carriage return carried, and the card's text read with a line feed | `web/app/src/lib/card/card-frame.test.ts` "a card whose field holds a carriage return reference renders through the host" |
| A4 | among U+0000 to U+00FF the host refuses exactly U+0000, in the card document and in the title, and carries every other code point byte for byte, with 256 of 256 examined for each | `frame-host.test.ts` "among the first 256 code points the host refuses exactly a NUL, in the card document and in the title" |
| A5 | the read-back still refuses a host it cannot carry byte for byte: a NUL in the card document, one in the title, a double quote in the policy, each alone, and keeps a plain card | `frame-host.test.ts` "a host that cannot carry the card document or its title byte for byte is refused" |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/frame-host.test.ts -t "a carriage return in the card document or the title is carried byte for byte"
A2: pnpm exec vitest run web/app/src/lib/card/frame-host.test.ts -t "a carriage return reference in a card field reaches the nested frame as the frame document wrote it"
A3: pnpm exec vitest run web/app/src/lib/card/card-frame.test.ts -t "a card whose field holds a carriage return reference renders through the host"
A4: pnpm exec vitest run web/app/src/lib/card/frame-host.test.ts -t "among the first 256 code points the host refuses exactly a NUL, in the card document and in the title"
A5: pnpm exec vitest run web/app/src/lib/card/frame-host.test.ts -t "a host that cannot carry the card document or its title byte for byte is refused"
```

A1 to A4 are red at the base by an assertion: the host refuses each card or title they plant. A5
is not red: it guards the refusal the base already makes.

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/src/lib/card/frame-host.ts` | `miniapp` | changed: `attribute()` writes U+000D as `&#13;` after `&` and `"`; its comment (lines 15-19) |
| `web/app/src/lib/card/frame-host.test.ts` | `miniapp` | changed: the import of `frameDocument`; A1, A2 and A4 appended inside the `describe`; A5's two carriage-return plants (lines 49-50) restated as NUL plants |
| `web/app/src/lib/card/card-frame.test.ts` | `miniapp` | changed: A3 appended inside the `describe`, after line 70 |
| `docs/specs/SPEC-462-the-card-frames-host-carries-a-carriage-return-so-a-card-whose-field-holds-one-renders-again.md` | docs | added |
| `docs/decisions/ADR-479-the-card-frames-host-writes-a-carriage-return-as-its-numeric-reference-and-its-read-back-stays-whole.md` | docs | added |
| `docs/specs/SPEC-407-the-card-document-inherits-the-frame-policy-from-a-host-so-every-engine-holds-the-image-set-forms.md` | docs | changed: a new last section, insert-only |
| `docs/schematics/card-frame-channels.md` | docs | changed: section 13 appended, insert-only |
| `docs/red-first/SPEC-462.md` | docs | added |
| `changelog.d/card-frame-carriage-return-462.md` | docs | added |

## 5. What this does NOT cover

- The connection observable in the card suite (#761).
- The link strip on the web and on iPhone and iPad (#664, #677).
- The census of the card's sinks, which this delivery adds no sink to (#867).
- The iPhone and iPad card view, which builds no host and reads nothing back by parse, so it needs no cure; its own suite measures its layers (#616).

## 6. Risks

- An engine whose parser resolves `&#13;` inside an attribute value to anything but U+000D: the read-back refuses the card there as at `dev`, so the frame fails closed. A1 to A4 measure jsdom's parser; no planted card in the browser card suite carries a carriage return reference, so a browser engine's reading is not yet measured, and a planted card for it is tracked as #939.
- A host refusal from any other cause leaves the study screen's status region quiet, because `review.ts:247-252` reads only `frameDocument`'s refusal. After this change no card reaches such a refusal by the parser's rules (section 1's table), so the gap is latent; announcing it is tracked as #938.
- The escaping's order: an ampersand written after the reference would turn `&#13;` into `&amp;#13;`. A1's card beside an ampersand and a quote decides it.
- A mutant of `attribute()` no test kills: `mutation-web` breaks at one survivor.
- The census of the card's sinks counts the `srcdoc` sink pattern in the text of `frame-host.ts`, comments included: the new comment must not add one, or `card-sinks.test.ts` reds.
- A5's restated plants: the test keeps three refusals, each alone, and A4 refuses U+0000 in both places, so the restatement removes no refusal the read-back makes.

## 7. The host's read-back, counted

| | at `dev` | after |
|---|---|---|
| code points among U+0000 to U+00FF the host refuses in the card document | U+0000 and U+000D | U+0000 |
| the same, in the title | U+0000 and U+000D | U+0000 |
| a double quote in the policy | refused | refused |
| the comparisons of the read-back (`frame-host.ts:47-50`) | three | the same three, unchanged |
| tests in `frame-host.test.ts` | 7 | 10 |
| tests in `card-frame.test.ts` | 3 | 4 |
| lines of `frame-host.test.ts` changed in place | | 2 (A5's two plants) |
| production files changed | | 1 |

No comparison of the read-back is narrowed, and no card is exempted from it.
