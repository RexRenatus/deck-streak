---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-376: only the owner's press records a grade, through an engine-core `OwnerAnswer` held apart from the gesture, and the engine records two grades

Decides SPEC-365 (issue `#711`). It is built on the owner's grading decision, which settles the
question issue #175 raised: grading is two buttons everywhere, only the owner's own tap or press
grades a card (held by an engine-core token), and a late review is told plainly that it does not
count toward the streak. This record decides how; that decision settled whether.

**Amends, by an insert-only amendment section appended to each ADR, with no existing line changed:**
- ADR-356 D2 (the table): a third set, `ANSWERED`, joins the ordinary and exempt tables.
- ADR-356 D4 (containment): the census holds the answer's names as it holds the gesture's.
- ADR-361 D1: it rejected a token on every grade, and AnswerCard leaves the ordinary pairs.
- ADR-348's JS boundary: the `answer` export is removed, and the wire's four ratings become two
  grades.
- ADR-359 D5 (which side picks a native answer's next state): the adapter picks it, from the
  states the card was shown with, and the codec's `Rating` names two grades.

## Context and Problem Statement

At `dev` `dee337bc`, three doors record a grade with no press behind it. The first is the core's
`run`, on both transports: AnswerCard (13,4) is an ordinary row (`table.rs:184-190`), and the native
allow-list admits it too (`allow_list.rs:54-58`). The second is the web's `run_method` (`study.rs:88`).
The third is the web's `answer` export, which answers the queue's head card (`wasm.rs:251-270`). Five
engine entry points admit four grades. The owner's gesture (`gesture.rs`; ADR-356 D5) already holds
the never-list's exempt writes to a tap. Nothing holds a grade to a press.

## Decision Drivers

- A grade is recorded only for a press that names its card and its grade: a timer, a script, a
  model, an import or a default records none (the owner-press rule).
- Every engine entry point offers exactly two grades (the two-button rule).
- The guarantee is the compiler's and the census's, not a convention in a caller.
- The never-list and the owner-taps ruling stay as they are.
- A rating still reaches only the card the learner saw (ADR-361 D2).

## Considered Options (the alternatives each was chosen against)

### D1. Where the guarantee lives

- An engine-core token, consumed by the one call that records a grade — chosen, because every adapter reaches the engine only through the core's dispatcher (ADR-356 D1), so a token checked there binds both clients at once.
- The UI alone (only the buttons call the engine) — rejected, because a page script or a native caller reaches `run` (13,4) directly: three such doors were measured.
- Each adapter's own table — rejected, because the core would still admit the pair, two copies would drift, and ADR-356 D3 makes the core decide after each adapter.
- A nonce the UI holds and the engine checks — rejected, because the page and any script in it share what the page holds, and a stored nonce is new shared state with its own interleavings.

### D2. What the token is

- A new `OwnerAnswer` (one card and one grade) with its own row and decision — chosen, because answering is not a never-list write, so it needs no exemption.
- A new `ExemptWrite` for AnswerCard in `EXEMPT`, as ADR-361's "What would make this wrong" foresaw — rejected, because `EXEMPT` is the never-list's exemption under ADR-301, and joining it would put every grade under the owner-taps ruling's conditions.
- A token that names the card and not the grade — rejected, because the two-button rule needs the engine to refuse a third grade, which it can do only if the press names the grade.
- A token that may be cloned or kept for a session — rejected, because one press records one grade, and a copy would let one press record two.

### D3. What the check reads

- The request's card id and rating, against the token's card and grade, with the checked message encoded again — chosen, because both are values the call owns, so nothing else can change them between the check and the act.
- Also the next state, against the engine's own states read at answer time — rejected, because it adds a read and then a write on the collection, which another caller can change in between. The engine already refuses a stale current state, and on the web the next state is picked in the entry file by a two-armed pick the boundary census holds.
- The core building the whole request from the card and the grade — rejected, because it gives up ADR-361 D2's kept states: the card answered would be the card as it is at answer time, not the card that was shown.

### D4. How many grades, and which

- Two, Again (rating 0) and Good (rating 2) — chosen, because they are the pass and the fail whose next states the engine gives for every card. The decision to offer two buttons names no pair, so this record picks one.
- Four in this delivery and two in the next — rejected, because the two-button rule covers the engine's entry points, and they would offer four grades for a delivery.
- Again and Easy — rejected, because Easy skips the learning steps, so every pass would overshoot its interval.
- Hard and Good — rejected, because there would be no fail, so a forgotten card could not be marked.

### D5. Where the answer row lives in the table

- A third set, `ANSWERED`, decided as `Decision::NeedsAnswer` on both transports — chosen, because the table test's arms and `run`'s refusal then tell "only by a press" apart from "never".
- AnswerCard kept in `ORDINARY` with both transport marks false — rejected, because `run` would then refuse it as not allowed, and a reader could not tell a held answer from a forbidden call.
- `EXEMPT` — rejected for D2's reason.

### D6. The native door

- `Engine::answer(card, grade, states, milliseconds_taken)`, with its own foreign types `PressedGrade` and `PressRefusal` beside `ExemptTap` and `ExemptRefusal`, building the answer from the states the card was shown with by the two-armed pick — chosen, because the native caller names the press's card and grade and passes what it was shown, and the adapter picks the next state and mints the token, as `run_exempt` mints a gesture for a tap.
- `Engine::answer(card, grade, input)` taking the caller's own `CardAnswer` bytes — rejected, because the native code would then choose the next state, which the token does not check (SPEC-365 section 6).
- A shown card kept in the adapter, as the web's `rate` keeps one — rejected, because it adds state to the adapter that D11 rules out, and the native screen already holds the card it shows.
- The adapter re-reading the card's states at answer time — rejected, because the states depend on the time they are computed (ADR-359 D5), and a read before the write is D3's rejected interleaving.
- `run_exempt` with a new `ExemptTap` — rejected for D2's reason.
- Keeping `run` (13,4) and checking in the native code — rejected, because the native code is the caller the token guards against.

### D7. The web door

- `rate` mints the answer from the kept card, and the queue-head `answer` export is removed — chosen, because only `rate` answers a card a press named, and ADR-361 D2 (`:67-68`) already turned `answer` away from the screens.
- `answer` minting a token for the head card — rejected, because the engine chooses that card and no press names it, which is the grade by script that the owner-press rule forbids.
- `answer` built only under a test feature — rejected, because the engine suite would then test a module the page never loads.
- `answer` kept but always refusing — rejected, because a dead export is a surface the boundary census must still carry.

### D8. The wire

- 1 is Again and 3 is Good, and 2 and 4 are refused by name as `NotAGrade` — chosen, because the page already sends 1 and 3 for Again and Good, and a Hard or Easy press is refused aloud.
- Renumbering the wire to 1 and 2 now — rejected, because the page's protocol and its stored data would change in the engine delivery, outside the surfaces that own them.
- Mapping 2 to Again and 4 to Good — rejected, because a Hard press would silently record another grade.

### D9. How the answer is contained

- The census that holds the gesture also holds the answer: new names, entry calls, held lines and plants, with no gesture row, name or assertion removed or narrowed — chosen, because the population and the walker are the same. The change is additive, so the owner-taps ruling's containment (ADR-337) holds as it did.
- A second census file for the answer — rejected, because it copies the walker, and two walkers must then be kept equal.
- The crate graph alone — rejected, because ADR-356 D4 found that the graph lets any crate that names the core mint a token, and only the census holds the entry files.

### D10. Test fixtures that seed a scratch collection

- They call the engine's own `answer_card` directly, and each line is held by name in the census — chosen, because a fixture builds a history in a collection the test owns, and no product path runs it.
- Routing fixtures through a token — rejected, because a token minted outside the entry files is exactly what the census refuses, and a test-only constructor would be a second door.

### D11. Whether the token is modelled

- No TLA+ model and no Lean proof — chosen, because no new actor or state is added and the check compares two owned values (SPEC-365 section 8).
- A TLA+ model — rejected, because the doors reaching AnswerCard shrink to the owner's presses and the token lives inside one call, so there is no interleaving to explore.
- A Lean proof — rejected, because the rule is two comparisons and one table row, which tests and mutation rows hold, as ADR-374 D12 decided for four.

### D12. The order of the work

- Three deliveries, landing as the web half of the surfaces (D2), then the engine token with the native half of the surfaces (this record's delivery, D1, D14), with the late-review copy (D3) independent — chosen, because the web half stands alone on the engine as it is (the engine accepts four wire ratings and the web then sends two), so no pull request leaves a window in which a Hard or Easy press is refused, and the native half needs `Engine::answer`.
- The engine first, then the surfaces — rejected, because the web would send ratings 2 and 4 that the engine already refuses, so a press would show a refusal until the surfaces landed.
- D1 and the web half composed into one merge — rejected, because the web half needs nothing from the engine change, so composing them only enlarges one review.
- One delivery — rejected, because it crosses four contexts and seven locales' messages, more than a review can hold.

### D13. The native callers already on `dev`

- Moving every native caller of AnswerCard to `Engine::answer` in this delivery — chosen, because the allow-list loses the pair here, so a caller left on `run` would be refused on `dev`.
- Leaving them for the native half — rejected, because the native review would answer through a refused pair from this delivery until the native half landed.
- Landing this delivery before the native review screen — rejected, because that screen's rounds are open, and its builder would then rebuild its answer path against a door its own design does not name.

### D14. The native half of the surfaces

- Folding the native half into this delivery, after the native review screen (#709) lands unchanged: the codec's `Rating`, the app's rating type and the answer bar narrow to Again and Good, and the screen answers through `Engine::answer` — chosen, because it adds no scope to that open pull request, and a screen that offers Hard or Easy cannot answer through a door that names two grades.
- Narrowing #709 to Again and Good before it lands — rejected, because it adds scope to an open pull request whose rounds are in flight and on whose landing this delivery waits.
- A separate native half after this delivery — rejected, because this delivery must move the screen's answer path to the two-grade door, and it cannot move a caller that offers four grades without narrowing it.
- Leaving the screen on the native `run` until a native half lands — rejected, because the allow-list loses AnswerCard here (D13), so the screen would answer through a refused pair.

### D15. Where a native press's codec lives

- The core's `answer.rs` carries the two codec steps of a native press: `shown_states` decodes the states the card was shown with, refusing `Undecodable`, and `answer_request` encodes the engine's `CardAnswer` from the next state the adapter picked. The native adapter keeps the decode's refusal, the grade's pick `grade.pick(states.again, states.good)`, its clock and the mint, so rows `S36515` and `S36516` anchor in `crates/ffi/src/engine.rs` — chosen, because the adapter depends on the engine core and the bindings runtime alone, and the core already holds the engine's messages and their codec (ADR-356 D1), so no dependency edge is added.
- A dependency edge from the native adapter to the engine's message crate and its codec — rejected, because the edge is the design: it would give the adapter a second route to the engine's messages beside the core that holds them, added only to make code compile.
- The core building the whole request from the card, the grade and the encoded states — rejected, because the pick would then leave the adapter, which D6 places there and rows `S36515` and `S36516` hold.
- The native caller encoding its own `CardAnswer` — rejected for D6's reason: the native code would choose the next state.

## Decision Outcome

D1 to D15 as chosen above. The core gains `answer.rs` (`Grade`, `OwnerAnswer`, `AnswerRefusal`, and a native press's codec steps `shown_states` and `answer_request`),
the `ANSWERED` row, `Decision::NeedsAnswer`, `Refusal::NeedsAnswer` and `Dispatcher::run_answer`.
The native adapter gains `Engine::answer`, which picks the next state from the states the card was
shown with, and AnswerCard leaves its allow-list; the native client offers Again and Good alone. The
web adapter's
`rate` answers through the token, its wire names two grades, and `answer` is gone. The census holds
the answer to the two entry files.

## Consequences

- Good: no door records a grade without a press that names its card and its grade, and no door
  records Hard or Easy.
- Good: both clients' surfaces have one thing to call, and the engine refuses anything else by
  name.
- Bad: the delivery depends on the web half having landed, so its build stops when the web still sends
  ratings 2 or 4 (D12).
- Bad: the native review screen offers four grades on `dev` from its own merge until this
  delivery's, recorded through the native `run` until then (D14).
- Neutral: the engine suite answers by showing and rating, which is the screens' own path.

### Confirmation

SPEC-365's A1 to A16, the census's plants, and the mutation rows `S36501` to `S36516`.

## What would make this wrong

- If the two grades are not Again and Good, `Grade`'s two values and the wire's two numbers
  change, and nothing else does.
- If a next state forged by a native caller is observed, the core reads the engine's own states (D3's
  rejected option) and accepts the interleaving model that comes with it.
- If the owner-taps ruling's containment is found to govern the census's lists, D9's second census
  file replaces the additive form.
