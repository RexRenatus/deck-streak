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
