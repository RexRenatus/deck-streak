# Schematic: from the owner's press to the recorded grade (SPEC-365)

Kind: **data flow**, with one **component** view of the doors before and after. Every `path:line`
was read at DeckStreak `dev` `dee337bc`. Names after this delivery are those SPEC-365 R1 to R13
give.

## 1. The web client

```mermaid
flowchart TD
  press["owner's press: a grade button, a key, the remote or the stick"] --> review["review.ts: the answer phase's grade cell, effect rate"]
  review -->|"client.rate(card, wire, ms)"| client["client.ts: rate"]
  client -->|"postMessage op rate"| session["session.ts: case rate"]
  session -->|"engine.rate(card, wire, ms)"| rate["wasm.rs: rate (entry file)"]
  rate -->|"study::grade(wire)"| wire{"wire 1 or 3?"}
  wire -->|"2 or 4"| notagrade["refused: NotAGrade"]
  wire -->|"0 or above 4"| range["refused: RatingOutOfRange"]
  wire -->|"1 Again, 3 Good"| shown{"SHOWN keeps this card?"}
  shown -->|"no"| notshown["refused: NotShown (ADR-361 D2)"]
  shown -->|"yes"| build["CardAnswer from the kept states: grade.pick(again, good), grade.rating()"]
  build --> mint["OwnerAnswer::from_press(shown.card, grade)"]
  mint -->|"run_answer(answer, bytes)"| check
  check["dispatch.rs: run_answer, answer.rs: checked"] --> recorded
  recorded["engine: AnswerCard (13,4) on the checked message, encoded again"] --> clear["SHOWN cleared"]
```

## 2. The native client

```mermaid
flowchart TD
  tap["owner's tap: the harness's Good button"] --> model["HarnessModel.answerGood"]
  apptap["owner's tap: the review screen's Again or Good, its two grade buttons"] --> appsess["the app's session: the shown card's states, as the queue gave them"]
  appsess -->|"engine.answer(card, grade, states, ms)"| entry
  model --> sess["EngineSession.answerGood: the head card's states, as the queue gave them"]
  sess -->|"engine.answer(card, .good, states, ms)"| entry["engine.rs: Engine::answer (entry file)"]
  entry -->|"states undecodable"| sentence
  entry --> pick2["CardAnswer from the shown states: grade.pick(again, good), grade.rating()"]
  pick2 --> mint2["OwnerAnswer::from_press(card, grade)"]
  mint2 -->|"run_answer(answer, bytes)"| check2["dispatch.rs: run_answer, answer.rs: checked"]
  check2 --> recorded2["engine: AnswerCard (13,4) on the checked message, encoded again"]
  check2 -->|"refusal"| sentence["PressRefusal: its own sentence"]
```

## 3. The check, shared by both clients

```mermaid
flowchart TD
  input["request bytes and an OwnerAnswer, consumed"] --> decode{"decodes as CardAnswer?"}
  decode -->|"no"| undecodable["AnswerRefusal::Undecodable; the engine is not called"]
  decode -->|"yes"| card{"card_id equals the pressed card?"}
  card -->|"no"| notcard["AnswerRefusal::NotTheCard pressed, named"]
  card -->|"yes"| grade{"rating equals the grade's rating, 0 or 2?"}
  grade -->|"no: Hard 1, Easy 3 or the other grade"| notgrade["AnswerRefusal::NotTheGrade pressed, named"]
  grade -->|"yes"| run["run AnswerCard on the message encoded again"]
  run -->|"engine error"| engine["AnswerRefusal::Engine error"]
  run -->|"ok"| ok["the grade is recorded"]
```

## 4. Who can reach AnswerCard

| door | at `dev` `dee337bc` | after SPEC-365 |
|---|---|---|
| the core's `run` (`dispatch.rs:112-130`) | admits (13,4) on both transports (`table.rs:184-190`) | refuses `NeedsAnswer { 13, 4 }` |
| the native `run` (`engine.rs:94-103`) | admits it (`allow_list.rs:54-58`) | refuses `NotAllowed` at the allow-list |
| the web `run_method` (`wasm.rs:656-659`) | admits it (`study.rs:88`) | refuses: not a study call, and the core refuses it too |
| the web `answer` (`wasm.rs:251-270`) | answers the head card, any of four ratings | removed |
| the web `rate` (`wasm.rs:485-502`) | answers the kept card, any of four ratings | answers the kept card with Again or Good, through `run_answer` |
| the native `Engine::answer` | absent | answers the pressed card with Again or Good, from the states it was shown with, through `run_answer` |
| `Dispatcher::run_answer` | absent | the one door, which needs an `OwnerAnswer` |

The token's constructor is named outside the core only in the two entry files. The census
(`crates/engine-core/tests/containment.rs`) holds this, and it holds the engine's `answer_card`
outside the core to four fixture lines in `crates/ingest/tests`.

## 5. What carries state

- `SHOWN` (`wasm.rs:70`): the web's kept card and its states. It is written when a card is shown,
  read and cleared by `rate`, and only on the Worker's one thread. It is unchanged by this delivery.
- The collection: written only by the engine, behind its own lock, which also refuses a stale
  current state. It is unchanged.
- `OwnerAnswer`: made in an entry file and consumed by `run_answer` in the same call. It is never
  stored.
- The states a native press passes: written by the codec from the queue's reply, held by the app
  while the card shows, and consumed by `Engine::answer` in one call. The adapter never stores them.

## 6. The engine's refusal of a stale current state (SPEC-460)

Kind: **data flow**. Every `path:line` was read at DeckStreak `dev` `83846ea3`; engine paths are
in the engine, the workspace's pinned git dependency, at the rev `Cargo.lock` pins. It
draws the answer's path from the shown state to the engine's comparison and its refusal, each
DeckStreak call path, the test's seam and the two rows (dashed).

```mermaid
flowchart TD
  queue["the queue shows the card: GetQueuedCards (13,3), with its states"] --> webkeep["web: SHOWN keeps the card and its states (wasm.rs:821-824)"]
  queue --> nativekeep["native: the app holds the encoded states; Engine::answer decodes them (engine.rs:192)"]
  queue --> testkeep["stale_answer.rs: the test keeps the states (its seam)"]
  other["another caller in between: an answer of the card (A1) or a due date set by the owner's gesture (A2)"] --> cardrow["the card's row in the collection"]
  webkeep --> webreq["wasm.rs:826-833: CardAnswer, current_state from the kept states"]
  nativekeep --> nativereq["answer.rs:149-166: answer_request, current_state from the shown states"]
  testkeep --> nativereq
  webreq --> door["dispatch.rs:496-502: run_answer"]
  nativereq --> door
  door --> checked["answer.rs:116-132: checked, card id and rating; encoded again, current state as sent"]
  checked -.-> rowone["row S46001: the engine's own state read here, at answer time"]
  checked --> answercard["engine: AnswerCard (13,4), one transact (answering/mod.rs:317-319)"]
  cardrow --> computed["engine: the card's current state from its row (current.rs:19-60)"]
  answercard --> computed
  computed --> compare{"answering/mod.rs:335-339: equal to the request's current state?"}
  compare -->|"yes"| recorded["apply_study_state (mod.rs:341): the grade is recorded"]
  compare -->|"no"| refused["invalid input, card was modified: a BackendError of kind INVALID_INPUT"]
  refused --> mapped["dispatch.rs:501: AnswerRefusal::Engine with the engine's bytes"]
  mapped -.-> rowtwo["row S46002: the refusal read as a recorded answer"]
  mapped --> nativeout["engine.rs:266: PressRefusal::Engine"]
  mapped --> webout["wasm.rs:837: refuse, its sentence"]
  mapped --> asserted["A1 and A2 assert: INVALID_INPUT, head card was modified; snapshot and newest review unchanged"]
```

| who reaches the refusal | through | held by |
|---|---|---|
| the web `rate` (`wasm.rs:818-856`) | `run_answer`, on the kept states | the core's door, run by A1 and A2; the adapter's own lines are not run here |
| the native `Engine::answer` (`engine.rs:184-206`) | `answer_request`, then `run_answer` | the core's door and codec, run by A1 and A2; the adapter's own lines are not run here |
| `crates/engine-core/tests/stale_answer.rs` | `answer_request`, then `run_answer`, as the native adapter calls them | A1 and A2 themselves |
| the core's `run` | none: the table holds (13,4) as `NeedsAnswer` (`table.rs:355-356`) | the existing table tests |

The engine compares the card's computed scheduling state, not its row: a bury or a flag between
show and answer leaves a new or review card's state equal, and is not refused (ADR-477 D1).
`formal/tla/AnswerOnTheShownState` models this section's two presses, the other caller and the
comparison as one action.
