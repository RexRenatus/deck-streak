# The web study screens: components, the review loop and its states

Read at: the base `ac4fbdaeb02f682991a9b26db539dbc20c8ff559` (`dev`), #662's head
`af6da34688467f8866167c4affbb53a45cca9d8b`, #663's head `d13f28f47c7088a22abf76ac7eeb5de86b1c92a7`,
and the engine fork's pinned revision `c538de55a23e695234e794029fce0dafff2d36a9`. Every `path:line`
below is at one of them, as SPEC-350 section 1 names. Decided by ADR-361.

## 1. Components

The page owns the screens, the input, the players and the frame's host element. The Worker owns the
engine and storage. The card frame is a sealed document the page writes once per side and never
hears from. Part 2's units are marked.

```mermaid
flowchart LR
  subgraph inputs["input sources"]
    keys["keyboard and the remote's keyboard mode"]
    pad["Gamepad API: the remote's gamepad mode"]
    touch["pointer and touch"]
  end

  subgraph page["the page (main thread, page policy)"]
    routes["routes: the deck list and the review"]
    input["study/input.ts: readKey, GamepadReader, resolve (from remote/)"]
    machine["study/review.ts: the review's machine"]
    buttons["AnswerButtons: two grades, Again and Good, each with its interval"]
    host["CardFrame: the frame's host element"]
    lock["WakeLockHolder (remote/wake-lock.ts)"]
    audio["one audio element (part 2)"]
    speech["speechSynthesis and the voice picker (part 2)"]
    client["EngineClient"]
  end

  subgraph worker["the Worker (one per tab, Web Lock held)"]
    session["Session: parseRequest, the operation switch"]
    engine["engine module: study.rs shown card, STUDY_CALLS, the core's Dispatcher on the web column"]
    opfs["OPFS: the collection, and the media directory the sync screens fill (part 2)"]
  end

  frame["card frame: srcdoc, empty sandbox, frame policy, no script"]

  subgraph ci["CI gates"]
    cargo["cargo tests: the tables, the shown-card rule"]
    vitest["Vitest and StrykerJS: protocol, machine, input, buttons"]
    guards["census, page policy and frame policy tests, unchanged"]
    size["size gate: 8000000 bytes gzip -9"]
    suite["study suite: Chromium and WebKit, axe on both screens"]
  end

  keys --> input
  pad --> input
  touch --> buttons
  touch --> routes
  input --> machine
  buttons --> machine
  routes --> machine
  machine --> client
  machine --> host
  machine --> lock
  machine --> audio
  machine --> speech
  client -- "request by id" --> session
  session -- "answer or refusal" --> client
  session --> engine
  engine --> opfs
  session --> opfs
  host -- "srcdoc, one way" --> frame

  cargo -.-> engine
  vitest -.-> machine
  guards -.-> frame
  size -.-> engine
  suite -.-> routes
```

### What crosses each edge

| edge | carries | never carries |
|---|---|---|
| input to the machine | an intent (`confirm`, a grade, `undo`, `bury`, `flag`, `replay`), read against the side | a key or button the map does not name; a key typed into a control; a repeat |
| the machine to `EngineClient` | `decks`, `study`, `card`, `rate`, `bury`, `flag`, `undoOffer`, `undo` with the card and step the offer named (SPEC-371 R9, R12) and `open` with the languages | `next`, `answer`, `seed`, `snapshot` (the harness's) or any pair |
| `EngineClient` to the Session | a numbered request; the reply settles it by id | a message from any origin but the page's own |
| the Session to the engine | one named export per operation, each crossing the Dispatcher on the web column | `run_method`, `run_exempt`, SQL |
| the host to the frame | the composed `srcdoc`: the frame policy, the card's CSS, the card's body with its classes, and (part 2) `data:` URLs in `src` | a script, a `blob:` or network URL, a token, a message channel |
| the frame to anything | nothing | everything: no listener exists, and the frame runs no script |

## 2. The review loop

Deck, next card, show, reveal, rate, next. The page keeps only which side is shown; the engine side
keeps the shown card.

```mermaid
sequenceDiagram
  participant L as learner
  participant P as page
  participant W as Worker
  participant E as engine

  P->>W: open with the app's languages
  W->>E: init, open the collection
  P->>W: decks
  W->>E: DeckTree (7,4) with now
  E-->>W: the tree with new, learning and review counts
  W-->>P: decks
  L->>P: choose a deck
  P->>W: study with the deck id
  W->>E: SetCurrentDeck (7,22)
  loop each card
    P->>W: card
    W->>E: GetQueuedCards (13,3), the head
    W->>E: RenderExistingCard (27,6), full render
    W->>E: StripAvTags (27,9), each side
    W->>E: DescribeNextStates (13,24) for the head's states
    W->>E: GetUndoStatus (3,7)
    Note over W,E: the engine side keeps the card id, its states and its flag
    W-->>P: the view, or none when the deck is done
    P->>P: question side into the card frame
    L->>P: show answer (Space, Enter, a face button, a tap)
    P->>P: answer side into the card frame, intervals on the buttons
    L->>P: a grade (Again or Good: a key, the d-pad, the stick, a tap)
    P->>W: rate with the card id, the rating and the milliseconds
    W->>E: is this the kept card
    alt the kept card
      W->>E: AnswerCard (13,4) with the kept states
      W-->>P: done, the kept card cleared
    else another card
      W-->>P: refused, not-shown
    end
  end
  opt undo, the review's own last answer only (SPEC-371 R9, R10)
    L->>P: u, the remote's undo, or the bar's Undo answer button
    P->>W: undoOffer
    W->>E: GetUndoStatus (3,7) and the newest review, judged against the Worker's record
    W-->>P: the offer with the card as text, the answer and the state it returns to, or none and why
    P->>P: confirming, the dialog with focus on Keep it
    alt the Undo action again, or the dialog's Undo answer button
      P->>W: undo with the offer's card and step
      W->>E: the owner's gesture for Undo, checked at the write, then Undo (3,8)
      Note over P,W: the record and the kept card cleared, and the next card request shows the undone card again
    else Keep it, Escape or any other action
      P->>P: back to the side, nothing sent, the other action not carried out
    end
  end
  opt bury or flag
    L->>P: minus, Control or Command with 1, or the remote
    P->>W: bury or flag with the card id
    W->>E: BuryOrSuspendCards (13,14) user bury, or SetFlag (5,4) red toggled
  end
```

## 3. The review's states

`review.ts` is a table, as #663's wake lock is: every event passes through one step function, and a
gesture while a request is in flight fires nothing.

```mermaid
stateDiagram-v2
  [*] --> loading
  loading --> question: a view
  loading --> done: no view
  loading --> refused: a refusal
  question --> answer: show answer
  answer --> busy: a grade
  question --> busy: bury, or undo asking for the offer
  answer --> busy: bury, or undo asking for the offer
  busy --> confirming: offered
  busy --> question: not-offered from the question, with its notice
  busy --> answer: not-offered from the answer, with its notice
  confirming --> busy: undo, or the Undo answer button
  confirming --> question: keep, Escape or another action, from the question
  confirming --> answer: keep, Escape or another action, from the answer
  busy --> loading: undo-refused, with its notice
  answer --> answer: flag settled
  question --> question: flag settled
  busy --> loading: settled
  busy --> loading: not-shown
  busy --> busy: any gesture, dropped
  refused --> loading: retry
  done --> [*]
```

A card the frame refuses (`escaped`) stays in `question` or `answer` with a message in place of the
frame: its controls stay usable, so the learner can still rate, bury or flag it.

## 4. Media, sound and speech (part 2)

```mermaid
flowchart TD
  view["the card view: each side's text"] --> names["ExtractMediaFiles (41,8): the names"]
  names --> read["Worker reads each name from the OPFS media directory"]
  read --> allow{"type on the allow-list, within the per-file and per-card bounds"}
  allow -- "yes" --> url["data: URL"]
  allow -- "no" --> skip["left as named: the frame policy loads nothing"]
  url --> doc["frameDocument replaces the matching src in its inert parse"]
  doc --> frame["card frame"]
  view --> tags["ExtractAvTags (27,3): sound and speech tags"]
  tags --> sound["sound: the page's audio element, after user activation"]
  tags --> tts["speech: speechSynthesis, BCP 47, stored voice, named voice, default"]
```

The seams the web sync screens (#631) need: the OPFS media directory's name and layout, which their
media transport fills; the deck list's empty state, whose one action opens their entry; and the
Worker, whose lock their sync must hold while it writes the collection.

## 5. Media, sound and speech through the core's face (part 2)

Section 4's Worker-built `data:` URLs are superseded (ADR-361 D12). The Worker asks the engine for
the shown card's two faces twice, inside the one queued operation, so no second actor appears: the
first ask answers the media names the core wanted, and the second carries their bytes. The page
plays the face's clips; the frame receives only the face's text. The install surface and the
mapping screen sit beside the review.

```mermaid
flowchart TD
  review["Review: the card is shown"] -->|"faces of the shown card"| worker["Worker: one queued operation"]
  worker -->|"first ask, no files"| engine["Engine: the core's face call over a reader"]
  engine -->|"both faces and the wanted names with limits"| worker
  worker -->|"each wanted name, first limit bytes"| media["OPFS directory deck-streak-media"]
  media -->|"bytes, or absent"| worker
  worker -->|"second ask, with the files"| engine
  engine -->|"both faces: text with data URLs, clips, omitted"| worker
  worker -->|"the second answer"| review
  review -->|"face text"| frame["Card frame: policy and sandbox unchanged"]
  review -->|"autoplay on show and reveal, replay on demand"| player["Clip player: one page audio element"]
  player -->|"sound: a page URL from bytes and type, revoked after"| audio["Page audio element"]
  player -->|"speech: language and rate over 0.5"| speaker["speechSynthesis with the stored voice"]
  voices["Voice picker: one key, per language"] --> speaker
  mapping["Mapping screen at /study/mapping: one key, per mode"] -->|"keys and buttons"| input["Study input: readKey and GamepadReader"]
  input -->|"one handler"| review
  install["manifest.webmanifest and icons, linked from app.html"] --> app["The app, installable, no service worker"]
```

## 6. The late line: the due day from the engine to the review (SPEC-376)

Read at the base `5fe4a48a09bc72700f434e7747ad64916af76902` (`dev`). Decided by ADR-387. This
section is appended to `docs/schematics/web-study-screens.md`; no line above it changes.

The due day has one source, the shown card's own fields in the engine's day, and one rule, the
engine core's `past_due_day`. The web engine carries the rule's answer to the page as the card
view's `late`; the page draws one line from it. The native review reads nothing yet (`#738`).

### 6a. Data flow, per client

```mermaid
flowchart LR
  subgraph engine["the engine (Worker, or the native process)"]
    queued["GetQueuedCards (13,3): the head card, with its queue, due, home deck due and home deck"]
    timing["the scheduler's timing of today: the day count and the next rollover"]
  end
  subgraph core["the engine core"]
    dayread["Dispatcher: the core's own read of the engine's day, behind no adapter pair"]
    rule["late::past_due_day(card, day)"]
  end
  subgraph web["web engine (wasm32)"]
    view["current_card: the view, with late"]
  end
  subgraph page["page"]
    proto["CardView.late"]
    screen["ReviewScreen: one line above the card when late"]
    msgs["study_late_review, in each of the 7 locales"]
  end
  subgraph native["native review"]
    none["no line: its own issue"]
  end
  timing --> dayread
  dayread --> rule
  queued --> rule
  rule --> view
  view --> proto
  proto --> screen
  msgs --> screen
  rule -. "the same rule, when its issue lands" .-> none
```

### 6b. The rule

| the card's queue | its due day | past its due day when |
|---|---|---|
| review, or day learning | its home deck due when it sits in a filtered deck and that due is set, else its own due | that day is earlier than the engine's day count |
| intraday learning | its due instant | that instant is earlier than the engine's next rollover less one day |
| new, preview, or any other | none | never |

### 6c. What crosses each new edge

| edge | carries | never carries |
|---|---|---|
| the engine to the core's day read | the day count and the next rollover, decoded from the engine's own reply | a clock read by the core itself; an adapter's request |
| the core's rule to the web engine | one boolean for the card the view shows | the due day, the day count or a date |
| the web engine to the page | `late` on the card view, built when the card is shown | a number, a date or a count of days |
| the page to the learner | one line in the app's locale, on both sides of the card | the due date, the days overdue, a word of blame |

### 6d. When the day turns

The view is built when the card is shown. A card shown before the rollover and answered after it
keeps the view it was shown with: a card due today shows no line and is then answered past its due
day (the line is absent, never false); a card past its due day when shown is still past it when
answered (the line stays true).

## 7. The withheld occlusion question: from the engine to both reviews (SPEC-380)

Read at the base `e7ecf10d6b796eb1f86fe6544e04a96a0583c791` (`dev`). Decided by ADR-391. This
section is appended to `docs/schematics/web-study-screens.md`; no line above it changes. The native
review's withheld phase is drawn in `docs/schematics/ios-review-screen.md` section 6.

An image occlusion question has its masks drawn by a script, and no review runs card scripts
(ADR-352 D1). The question has one rule, the engine core's `occlusion::masks_not_drawn`, read over
the rendered question. A marked question is withheld on both sides, on both reviews: the web
review shows one line in every locale, and the native review shows the ffi's English line in the
card's place. Neither records a rating, and the card stays due.

### 7a. Data flow, per client

```mermaid
flowchart LR
  subgraph engine["the engine (Worker, or the native process)"]
    render["RenderExistingCard (27,6): the question and the answer"]
  end
  subgraph core["the engine core"]
    rule["occlusion::masks_not_drawn(question)"]
    coreface["face::complete: a Face, withheld on both sides when marked"]
  end
  subgraph web["web engine (wasm32)"]
    view["current_card: the view, with withheld"]
    faces["faces: the core's faces"]
  end
  subgraph page["page"]
    proto["CardView.withheld"]
    phases["review.ts: the withheld phase, with undo, bury and flag"]
    screen["ReviewScreen: no frame, no late line, the status line"]
    msgs["study_card_withheld, in each of the 7 locales"]
  end
  subgraph ffi["ffi"]
    cardface["CardFace.withheld, and a document holding the English line"]
  end
  subgraph native["native review"]
    session["ReviewSession.next: the withheld phase"]
    model["ReviewModel.perform: bury and flag only"]
    shown["CardFaceView: the ffi's document in the card's place"]
  end
  render --> rule
  render --> coreface
  rule --> coreface
  render --> view
  rule --> view
  coreface --> faces
  view --> proto
  proto --> phases
  phases --> screen
  msgs --> screen
  coreface --> cardface
  cardface --> session
  session --> model
  cardface --> shown
```

### 7b. The rule

| the rendered question holds | marked | the notes it meets |
|---|---|---|
| the engine's mask layer | yes | a stock occlusion note, with shapes, with no shape, or with a malformed shape field |
| an occlusion shape, without the mask layer | yes | a cloned or custom type whose template dropped the layer |
| neither | no | a plain card, a text cloze, an image; and, unseen, an older note whose masks are separate images (SPEC-380 section 5) |

The marker names are measured by the build on a note the engine builds, and named in the rule's
doc comment.

### 7c. The web review's withheld state

```mermaid
stateDiagram-v2
  [*] --> Loading
  Loading --> Question: a card, not withheld
  Loading --> Withheld: a card, withheld
  Withheld --> Busy: bury, or undo of the previous answer
  Withheld --> Withheld: flag
  Busy --> Loading: the next card
```

- In `Withheld`, show-answer, replay and every rating have no row in the review's table: a key,
  button, remote or stick press there is not a transition, and no rating is sent.
- The status region shows `study_card_withheld` where the escaped card's line is shown; the card
  frame and the late line are not drawn.
- The card stays due; only the learner's own bury moves it.

### 7d. What crosses each new edge

| edge | carries | never carries |
|---|---|---|
| the engine's render to the core's rule | the rendered question | the note's fields, its type's kind |
| the core's rule to the core's face and the web engine's view | one boolean for the card | the question's text, the markers |
| the core's face to the web engine's faces and the ffi | a withheld face: no text, no note CSS, no clip | the image, the shapes |
| the web engine to the page | `withheld` on the card view, with an empty question, answer and CSS | the image, the shapes, a reason code |
| the ffi to the native review | `withheld` on the card face, and a document holding the English line | the image, the shapes, a script |
| either review to the learner | one line, and the learner's own bury and flag | a rating, an automatic bury |
