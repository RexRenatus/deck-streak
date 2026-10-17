# SPEC-378: a grade recorded through the press token is an ordinary answer, and no exemption holds it

- **Issue:** #716. SPEC-365's section 5 left it open: "It moves no row into `EXEMPT` and adds no
  never-list exemption. ADR-301 and the owner-taps ruling are untouched (`#716`)." The issue asks
  for that sentence to be held by a test: recording a grade through the press token is an ordinary
  answer, no row joins the exemption list, and the never-list and the owner-taps exemption stay as
  they are. **Context(s):** `deck-streak-engine-core` (`crates/engine-core`).
- **Decided by:** ADR-389 (this SPEC's own). It works under ADR-376 D2 and D5 (the answer has a
  token of its own and a row of its own, each chosen against a row in `EXEMPT`), ADR-301 (a) (the
  never-list) and ADR-337 with the owner-taps ruling, and it changes none of them.
- **Schematic:** none. No code path changes: the delivery adds three tests, its red-first record,
  its mutation rows and its changelog fragment. The press's path to the engine is the one
  `docs/schematics/owner-press-to-recorded-grade.md` draws, unchanged.
- **Status:** one delivery. **Mutation band:** `S37800-S37899` (section 9). **Model:** none
  (section 8).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `f48a977c` by `git show`, `git grep -n` and `grep -n`,
except the one run section 1c names.

### 1a. Where the lists are declared, and how a write is classed

| list | where | what it holds |
|---|---|---|
| the never-list | `docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md:105-124` (entries 1 to 8 at `:117-124`, the first exempt tap's note at `:107-110`) | the eight writes no write class makes and no approval admits |
| the owner-taps exemption | `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md` and ADR-337's Decision Outcome (`docs/decisions/ADR-337-the-owners-own-taps-are-exempt-from-the-never-list-and-every-other-path-stays-bound.md:36-67`) | a write the owner makes by one explicit gesture in the study client, through an `OwnerGesture` |
| the exemption list in code | `crates/engine-core/src/table.rs:286-343`, `pub const EXEMPT: [Exempt; 8]`, each row an `ExemptWrite` (`:34-51`) | Forget, set due date, delete a preset, change a note's type, delete a card, delete a note, the one-way sync and Undo |
| the answered set | `crates/engine-core/src/table.rs:273-277`, `pub const ANSWERED: [Answered; 1]` | `SchedulerService.AnswerCard`, (13,4), alone |

`decide` (`table.rs:349-362`) classes a pair: `Admit` when an ordinary row holds it for the
transport, then `NeedsAnswer` when the answered row holds it (`:355-356`), then `NeedsGesture`
when an exempt row holds it (`:357-358`), and `NotAllowed` otherwise. `Dispatcher::run` refuses
the last three before the engine sees the call (`crates/engine-core/src/dispatch.rs:188-190`). An
exempt write runs only through `Dispatcher::run_exempt` (`dispatch.rs:205-217`), which consumes
an `OwnerGesture` whose pair comes from its `EXEMPT` row (`crates/engine-core/src/gesture.rs`
`from_tap` and `checked`). Every other write stays bound by the never-list.

### 1b. How a grade recorded through the press token reaches the engine

| client | path |
|---|---|
| web | `crates/web-engine/src/wasm.rs:695` `rate`, `:710` `OwnerAnswer::from_press(shown.card, pressed(grade))`, `:712` `run_answer` |
| native | `crates/ffi/src/engine.rs:173` `Engine::answer`, `:191` `OwnerAnswer::from_press(card, grade)`, `:193` `run_answer` |
| the core | `crates/engine-core/src/dispatch.rs:377` `run_answer` consumes the answer; `crates/engine-core/src/answer.rs:116` `checked` decodes and checks the `CardAnswer` against the press; `dispatch.rs:379` `let [row] = ANSWERED;` runs (13,4) on the checked bytes |

No gesture and no `EXEMPT` row is on that path. Today the call is classed `NeedsAnswer` on both
transports: it is an ordinary answer held to a press, not an exempt write.

### 1c. What holds it today, and why that is not yet a pin

Three tests on `dev` would fail if the call joined the exemptions:

- `crates/engine-core/tests/table.rs:83` `every_pair_is_admitted_held_or_refused_by_its_transport`
  holds the gesture set equal to its `HELD` literal (`:64`, eight pairs, `:114`) and the answered
  set equal to its `ANSWERED` literal (`:77`, (13,4), `:119`) on both transports.
- `tests/table.rs:131` `each_exempt_write_names_its_engine_call_and_its_target_kind` holds `EXEMPT`
  equal to eight literal rows.
- `crates/engine-core/tests/dispatch.rs:156`
  `an_answer_through_run_is_held_for_the_owner_and_leaves_the_card` holds the native `run`'s
  refusal of (13,4) as `NeedsAnswer`.

Each is a census of the whole table or of one transport, and a delivery that adds an exempt row
edits the census in the same change: SPEC-364 and SPEC-371 grew `EXEMPT` from six rows to eight,
and SPEC-371's A1 and A2 grew `HELD` and the literal rows with it. A delivery that moved AnswerCard
into `EXEMPT` would edit them the same way, and no test would remain whose one subject is that a
grade is not an exempt write. The census's decision reading also cannot see AnswerCard join
`EXEMPT` while it stays in `ANSWERED`, because `decide` answers `NeedsAnswer` first: with row
S37800's mutant planted, `every_pair_is_admitted_held_or_refused_by_its_transport` still passes
(`docs/red-first/SPEC-378.md` quotes the run).

## 2. Requirements

R1. **The answer's class.** A test names the call that records a grade by its literal pair (13,4)
    and its literal name `SchedulerService.AnswerCard`, never read from the core. It holds that
    `ANSWERED` is that call alone and that `decide` classes it `Decision::NeedsAnswer` on both
    transports.

R2. **No exempt row names it.** A test holds that no `EXEMPT` row names that call, by its pair or
    by its name, whatever else `EXEMPT` holds. Three planted tables are each refused by the planted
    row's own name: a row naming the call joins the table, the first row takes the call's pair,
    and the first row takes the call's name.

R3. **`run` holds it for a press.** A test holds that `Dispatcher::run` refuses the call with
    `Refusal::NeedsAnswer { service: 13, method: 4 }` on both transports, before the engine sees
    it, and never with `Refusal::NeedsGesture`.

R4. **Where the pin lives, and what it leaves.** The three tests are added to
    `crates/engine-core/tests/answer.rs`, the token's own test file, after its existing tests,
    with two import lines grown and a paragraph added to its module comment. No existing test, no
    source file, no row of `EXEMPT`, `ORDINARY` or `ANSWERED`, no `ExemptWrite`, no declared write
    class, no line of ADR-301, ADR-337 or ADR-376, and no file under `docs/rulings/` changes.

R5. **Rows.** Three mutation rows, `S37800` to `S37802`, each name one hand mutant and the test of
    this SPEC that kills it (section 9).

## 3. Acceptance criteria of SPEC-378

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | `ANSWERED` is (13,4) `SchedulerService.AnswerCard` alone, and `decide` reads `NeedsAnswer` for it on both transports | not red: it pins `dev`'s classing; row S37801's planted mutant reads it red | `the_answer_is_decided_for_a_press_on_both_transports_never_for_a_gesture` |
| A2 | No `EXEMPT` row names the call by its pair or its name, and the three planted tables are refused by their planted rows' names | not red: it pins `dev`'s table; row S37800's planted mutant reads it red | `no_exempt_row_holds_the_answer_and_a_planted_one_is_refused_by_name` |
| A3 | `run` refuses (13,4) with `NeedsAnswer { 13, 4 }` on both transports | not red: it pins `dev`'s refusal; row S37802's planted mutant reads it red | `run_holds_an_answer_for_a_press_on_both_transports_never_for_a_gesture` |

```acceptance
A1: cargo test -p deck-streak-engine-core --test answer -- --exact the_answer_is_decided_for_a_press_on_both_transports_never_for_a_gesture
A2: cargo test -p deck-streak-engine-core --test answer -- --exact no_exempt_row_holds_the_answer_and_a_planted_one_is_refused_by_name
A3: cargo test -p deck-streak-engine-core --test answer -- --exact run_holds_an_answer_for_a_press_on_both_transports_never_for_a_gesture
```

A pin of behaviour `dev` already has cannot be red at its own commit. Each criterion's red is its
row's mutant, planted by hand on the tree the test was committed to and restored byte for byte,
as SPEC-362's A8 recorded its red; `docs/red-first/SPEC-378.md` quotes each failure.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/engine-core/tests/answer.rs` | `deck-streak-engine-core` | changed: three tests and their two helpers appended, two import lines grown, one module-comment paragraph added |
| `docs/specs/SPEC-378-a-grade-recorded-through-the-press-token-is-an-ordinary-answer-and-no-exemption-holds-it.md` | docs | added |
| `docs/decisions/ADR-389-the-press-token-grade-is-an-ordinary-answer-pinned-in-the-answers-own-test-file.md` | docs | added |
| `docs/red-first/SPEC-378.md` | docs | added |
| `scripts/mutation-rows.d/S37800-S37899.json` | scripts | added |
| `changelog.d/grade-token-not-exempt-378.md` | docs | added |

The rows plant their mutants in `crates/engine-core/src/table.rs` and
`crates/engine-core/src/dispatch.rs` only while a row is proved; both files are unchanged.

## 5. What this does NOT cover

- It edits no never-list entry and no declared write class: ADR-301 (a) and its note stand as
  written (#514).
- It edits neither the owner-taps exemption nor any file under `docs/rulings/`, and ADR-337 stands
  as written; the exempt taps keep their gestures (#714).
- It moves no row of `EXEMPT`, `ORDINARY` or `ANSWERED` and changes no source file: the answer's
  own token and row are SPEC-365's (#711).
- It leaves the table's censuses as they are; a later exempt row still grows them as SPEC-371 did
  (#714).
- It adds no web test: the web's grade surface classes no write, and it stays SPEC-366's (#712).

## 6. Risks

- **A later ruling makes a grade a tap.** That delivery must delete or rewrite these three tests,
  and a deleted or narrowed test is a weakening the reviewer reads by name and that needs a signed
  ruling. That is the friction this SPEC adds on purpose; the three tests detect the change.
- **A row's anchor moves.** Open pull request #737 changes `crates/engine-core/src/table.rs`, and
  #741 and #742 change `crates/engine-core/src/dispatch.rs`. A find span that no longer occurs
  exactly once is refused by the rows battery by the row's stem, which detects it.
- **The default engine start changes.** A3 starts the engine from an empty init on each transport,
  as `tests/answer.rs` already does natively; a start that fails stops A3 at its fixture's own
  message, not at its assertion.

## 7. What only CI proves

The rows' proof on the battery and the whole engine-core suite on the gate are CI's, read by name.

## 8. Formal model

None. `git grep -h '@phx covers'` over the fenced tree at `f48a977c` reads 220 covers; one names
`crates/engine-core/src/dispatch.rs` (anchor `run_undo`), and none names `src/table.rs`,
`src/answer.rs` or `tests/answer.rs`. This delivery changes no source line, and row S37802 plants
in `run`, outside the covered anchor.

## 9. Mutation rows

| stem | file | mutant | killer |
|---|---|---|---|
| `S37800-THE-ANSWER-JOINS-THE-EXEMPTIONS` | engine-core `src/table.rs` | a ninth `EXEMPT` row names (13,4) `SchedulerService.AnswerCard` | `answer::no_exempt_row_holds_the_answer_and_a_planted_one_is_refused_by_name` |
| `S37801-THE-ANSWER-IS-DECIDED-EXEMPT` | engine-core `src/table.rs` | `decide`'s answered arm reads `NeedsGesture` | `answer::the_answer_is_decided_for_a_press_on_both_transports_never_for_a_gesture` |
| `S37802-RUN-HOLDS-THE-ANSWER-FOR-A-GESTURE` | engine-core `src/dispatch.rs` | `run`'s answered arm refuses `NeedsGesture` | `answer::run_holds_an_answer_for_a_press_on_both_transports_never_for_a_gesture` |

cargo-mutants adds no row to a `const` table, so S37800 is the only proof of its mutant. Each row
is killed by hand before it is committed, and the red-first record quotes each failure.
