# Review templates: from a note's fields to the card frame

**Kind:** data flow. **Decided by:** ADR-383. **Specified by:** SPEC-372. Every `path:line` below
was read at DeckStreak `dev` `ebdd1371`.

The flow runs from a note's fields, its note type's templates and its CSS, through the engine's two
renders, through the Worker and the review, to the frame's body and style element. The golden file
sits beside the flow: the Rust test checks the engine's half against it, and the web test feeds it
to the review's half.

## The flow

```mermaid
flowchart TD
  Inputs["note fields, templates and CSS<br/>golden inputs, written by hand"]
  Render1["RenderExistingCard, partial flag on<br/>face.rs:176-181"]
  Join["join the nodes; FrontSide filled by the shown question<br/>face.rs:98-110, face.rs:183-190"]
  Face["Face: text and css<br/>face.rs:217-223"]
  Door["Dispatcher::face<br/>dispatch.rs:308-316"]
  Faces["Worker faces, both sides<br/>wasm.rs:552-574"]
  Render2["RenderExistingCard, partial flag off<br/>wasm.rs:417-423"]
  Head["Worker current_card: ordinal and css<br/>wasm.rs:442, wasm.rs:446"]
  View["review view: head card, sides from the faces<br/>review.ts:268-274"]
  Screen["ReviewScreen: side, head CSS, classes card cardN<br/>ReviewScreen.svelte:155-160"]
  Frame["CardFrame<br/>CardFrame.svelte:12-15"]
  Doc["frameDocument: classes, parse, strip, CSS newlines to LF, compose, parse, check<br/>frame-document.ts:36-51"]
  Body["frame body: the card's markup, classes card cardN"]
  StyleElement["frame style element: the CSS, LF line ends"]
  Refused["data-card-refused"]

  Inputs --> Render1 --> Join --> Face --> Door --> Faces
  Inputs --> Render2 --> Head
  Faces --> View
  Head --> View
  View --> Screen --> Frame --> Doc
  Doc --> Body
  Doc --> StyleElement
  Doc --> Refused
```

## Where each test pins it

```mermaid
flowchart LR
  Golden["review-templates.golden.json<br/>inputs, frame CSS, render"]
  RustTest["review_templates.rs<br/>builds from the inputs, renders, asserts anchors, compares"]
  Engine["Dispatcher::face and RenderExistingCard"]
  WebTest["review-templates.test.ts<br/>fake client serves the render"]
  Review["review, ReviewScreen, CardFrame, frameDocument"]
  Observed["srcdoc parsed, data-card-refused"]

  Golden -->|inputs| RustTest
  RustTest -->|calls| Engine
  Engine -->|render| RustTest
  Golden -->|"render, compared"| RustTest
  Golden -->|"render, served"| WebTest
  WebTest -->|drives| Review
  Review -->|frame| Observed
  Golden -->|"frame CSS, the oracle"| WebTest
```

| pin | what it holds | on which edge | criterion |
|---|---|---|---|
| The Rust test's anchors, written by hand | each cloze card hides its own deletion and shows the other; each answer reveals it and adds the extra field; each reversed answer is its question, the rule and the other field; every CSS equals the inputs byte for byte; ordinals 0 and 1; no id in any text | Inputs to Face, Inputs to Head | A5 |
| The Rust test's comparison | the four cards' texts and CSS equal the golden's render | Face, Head | A5 |
| The web test, cloze cards | each side's body equals the golden text as the frame's parser reads it; classes `card card1` and `card card2` | View to Body | A1 |
| The web test, reversed cards | each answer's body equals the golden answer and opens with its question's body | View to Body | A2 |
| The web test, style element | the style element's text equals the frame CSS written by hand, for every card | Head to Style | A3 |
| The web test, refusals | no face is refused; a control card with CSS that ends the style element is | Doc to Refused | A4 |

## Newlines

| where | CR LF and lone CR | why |
|---|---|---|
| the note type's CSS (golden inputs) | kept | the fixture carries them on purpose |
| the engine's render, face and head | kept | the engine passes the CSS through (`face.rs:219`, `wasm.rs:446`) |
| the frame's style element | LF only | `frame-document.ts:40-41`, before the check at `:48` |
| the card's markup | the parser's own reading, on both sides | no step is needed or added |

## The planted reds (red commit only)

| plant | file | turns red |
|---|---|---|
| the face returns no CSS | `crates/engine-core/src/face.rs:219` | A5 |
| the review keeps the head card's own sides | `web/app/src/lib/study/review.ts:274` | A1, A2 |
| the composition trims the CSS's trailing newline | `web/app/src/lib/card/frame-document.ts:41` | A3 |
| the check admits only `card card1` | `web/app/src/lib/card/frame-document.ts:29` | A4 |

The greening commits restore each line, so every planted file's net diff is empty.
