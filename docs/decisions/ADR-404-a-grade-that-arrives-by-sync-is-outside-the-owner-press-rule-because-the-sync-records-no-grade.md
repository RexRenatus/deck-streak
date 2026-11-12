---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-404: a grade that arrives by sync is outside the owner-press rule, because the sync records no grade

Decides issue `#715`. SPEC-365's section 5 left it open: "It leaves every sync path unchanged
(`#715`)." The issue asks whether, and how, the owner-press rule applies to a grade recorded on
another client that reaches this one by sync, without a press here. This record decides that the
rule does not apply to it, says why, and changes nothing. It amends no ADR and no SPEC, and it
changes no line of ADR-301, ADR-337, ADR-376, ADR-389, SPEC-365 or any ruling.

## Context and Problem Statement

Every figure was read at `dev` `164ac206` by `git show`, `git grep -n` and `git ls-tree`. Nothing
was run.

**The rule.** SPEC-365 records the owner's grading decision at lines 5-6: "The owner-press rule:
only the owner's own tap or press grades a card, held by an engine-core token." ADR-376 states it
as a driver at lines 33-34: "A grade is recorded only for a press that names its card and its
grade: a timer, a script, a model, an import or a default records none (the owner-press rule)." No
signed ruling under `docs/rulings/` names a grade or a press.

**What it binds.** One call records a grade: `SchedulerService.AnswerCard` (13,4), the one row of
`ANSWERED` (`crates/engine-core/src/table.rs:270-277`). `decide` holds it as `NeedsAnswer` on both
transports (`table.rs:355-356`), and `run` refuses it (`crates/engine-core/src/dispatch.rs:226`).
It runs only through `Dispatcher::run_answer` (`dispatch.rs:417-423`), which consumes an
`OwnerAnswer` (`crates/engine-core/src/answer.rs:102-112`) and checks the request's card and rating
against the press's (`answer.rs:116-132`). Two entry files mint one, each from a press in
DeckStreak's own client: the web review's press through `rate` (`crates/web-engine/src/wasm.rs:719`,
`:734`, `:736`), and the native client's press through `Engine::answer`
(`crates/ffi/src/engine.rs:184`, `:202`, `:204`). The census holds the token's names to those two
files (`crates/engine-core/tests/containment.rs:65-67`, `:79-80`, and
`no_non_ui_caller_reaches_an_exempt_function` at `:691`). So the rule binds a press made in
DeckStreak's own client, and it is checked by the engine core that client runs, when the grade is
recorded there.

**How a grade arrives by sync.**

- The web client runs the engine's normal sync, `BackendSyncService.SyncCollection` (1,5), an
  ordinary row that the web admits and the native transport refuses (`table.rs:130-136`). The
  Worker's sync step calls the web engine's `sync_collection` (`web/app/src/lib/engine/sync.ts:120`;
  `wasm.rs:546-564`). `run` guards the endpoint and the client's minimum level, then hands the
  request to the engine whole (`dispatch.rs:209-218`). The core reads nothing of what the sync
  brings in.
- The full download, the one-way write, reaches the engine only through the full-sync choice's own
  write (`crates/engine-core/src/gesture.rs:154-157`; the `OneWaySync` row, `table.rs:330-333`),
  under the owner's gesture.
- The native client admits only the sync login among the sync calls today
  (`crates/ffi/src/allow_list.rs:31`, `:70`). Its normal sync is SPEC-358 R17's (`#633`), through
  the same core `run`.
- The service's own copy runs a normal sync of its own (`crates/ingest/src/engine.rs:320`, `:329`),
  and the engine's `answer_card` appears outside the core only at the held fixture lines that
  SPEC-365 section 1a names.

None of these paths reaches (13,4). A grade that arrives by sync was recorded by the client where it
was pressed: DeckStreak's other client, through that client's own `run_answer`, or a stock client,
which keeps reaching the sync server as the per-review client record ruling's "What stays" bullet
says. What arrives is that record, in the collection's own review log, and no token travels with
it: the token "is a value made and consumed inside one call" (SPEC-365, line 355).

**The signed rulings that touch it.** The owner-taps ruling keeps "every repair or reconciliation of
sync" under the never-list, whose first entry is editing review history, and no approval turns such
a path into an exempt tap (its "What stays bound" bullet). The per-review client record ruling has
every review in the log that DeckStreak reads carry which client made it, a stock client included.
It holds that "any signal used to tell clients apart is read in passing and never stored, and
neither the sync server nor the edge keeps anything new for it", and that, besides the export, the
erase and the backups, "only R14's measurement and the owner's own view of the record read it".
That record is not built at `164ac206`: no file but the ruling names it.

**The one interleaving.** On the web, every request to the Worker runs on one queue
(`web/app/src/lib/engine/session.ts:148-156`), the sync among them (`:170`) and the press's `rate`
(`:317`), so a sync never runs inside a press. A sync can still run between the review showing a
card and the owner pressing it. If the review that arrives is of that card, the press here records
against the states the card was shown with, and whether the engine refuses that stale current state
is the engine's own answer, on which ADR-376 D3 relies (ADR-376, line 59). Either way, each grade is
a press on the client that recorded it.

## Decision Drivers

- The rule's words name a press, and its check sits on the one call that records a grade.
- This record changes no signed ruling's text, and reads none of them beyond its words.
- The engine core is one: what it decides for one client, it decides for both.
- Nothing is kept to tell clients apart beyond the per-review client record's one field.

## Considered Options (the alternatives each was chosen against)

### D1. The rule, exactly

- Chosen: the rule is SPEC-365's lines 5-6 and ADR-376's lines 33-34, and it binds a press made in DeckStreak's own client, checked by `OwnerAnswer::checked` inside `run_answer` in the engine core that client runs, when the grade is recorded, because those are the only lines that state it and that is the only door that holds it.
- Chosen against: reading the rule as binding every review row that enters the collection by any route, because its words name a press, and the tree already classes the normal sync as an ordinary call apart from the one that records a grade (`table.rs:130-136` against `:270-277`).
- Chosen against: taking the rule's reading from a signed ruling, because no signed ruling under `docs/rulings/` names a grade or a press, and the owner-taps ruling governs the never-list's exempt taps, which a grade is not (ADR-376 D2, ADR-389).

### D2. The answer to `#715`

- Chosen: a grade that arrives by sync is outside the owner-press rule, because the rule binds the recording of a grade, a sync here records none, and the grade was recorded by the client where it was pressed, under that client's own check when that client is DeckStreak's.
- Chosen against: in scope, with a check on arrival, because nothing arrives to check: the token is made and consumed inside one call (SPEC-365, line 355), so no record of a press travels with a synced review.
- Chosen against: in scope, removing an arrived review that carries no proof of a press, because removing a review edits review history, which the owner-taps ruling keeps under the never-list for every repair or reconciliation of sync, with no approval admitting it.
- Chosen against: refusing the whole normal sync when it would bring a grade that no press here made, because the core hands the sync to the engine whole (`dispatch.rs:215-218`), so refusing it refuses every other change it carries, and the review stays on the server for the next sync.
- Chosen against: in scope, with an attestation the pressing client sends with each review, because a stock client sends none, and the per-review client record ruling keeps nothing to tell clients apart beyond its one field and nothing new on the sync server.
- Chosen against: in scope, read from the per-review client record once it is built, because that ruling limits the record's readers to the export, the erase, the backups, R14's measurement and the owner's own view, so no arrival rule may read it.

### D3. Behaviour

- Chosen: no code changes, and the delivery is this record alone with its changelog fragment, because every fact it rests on is already held by a test whose subject it is: `the_web_column_admits_the_sync_login_and_the_normal_sync` (`crates/engine-core/tests/table.rs:277`) holds (1,5) admitted on the web and refused natively, `every_pair_is_admitted_held_or_refused_by_its_transport` (`tests/table.rs:86`) holds (13,4) as the one pair held for a press, `run_holds_an_answer_for_a_press_on_both_transports_never_for_a_gesture` (`crates/engine-core/tests/answer.rs:484`) holds `run` refusing it, and `no_non_ui_caller_reaches_an_exempt_function` (`tests/containment.rs:691`) holds the token's names to the two entry files.
- Chosen against: a delivery that pins this answer with new tests, as SPEC-378 pinned `#716`, because what it would pin is an absence inside the engine's own sync, which the core never reads, and a later delivery that adds an arrival check changes `run`'s path for (1,5), so it owes its own SPEC and an ADR that supersedes this one.
- Chosen against: waiting for the per-review client record to be built, because its ruling bars an arrival rule from reading it (D2), so its build cannot change this answer.

### D4. FORMAL, decided by surface

- Chosen: no new and no amended model, because this record adds no actor, no state and no step, and edits no covered span; two models cover the surface's code, and neither needs a change: `formal/tla/MinimumClientHandshake` covers `crates/engine-core/src/dispatch.rs` at `run`, the door the normal sync passes, and models whether a sync waits for an admitting read, not what it brings, and `formal/tla/UndoOwnAnswer` covers `crates/web-engine/src/wasm.rs` at `rate`, `undo` and `undo_offer`, and its `Sync` action marks the queue's rows synced, outbound only.
- Chosen against: a new TLA+ model of an arriving grade beside a press, because the press's check compares two values its one call owns (SPEC-365 section 8; ADR-376 D11), the Worker runs a sync and a press one after the other (`session.ts:148-156`), and the only state the two share is the card's current state, which the engine decides inside its own sync and answer, where no item of DeckStreak's code exists for a cover to name.
- Chosen against: amending `UndoOwnAnswer` with an inbound sync, because its properties judge the undo of the review's own last answer (`#714`), this record changes no path that model covers, and a new cover restarts that entry's promotion count with no change in code.

### D5. The native client's normal sync

- Chosen: the answer holds unchanged when the native client gains its normal sync (SPEC-358 R17, `#633`), because both transports reach the engine through one `decide` and one `run` (`table.rs:349-362`, `dispatch.rs:203`), so the answer cannot differ by client.
- Chosen against: a second decision when that sync lands, because nothing in this answer depends on the transport.

### D6. Shape and closing

- Chosen: one delivery, docs only, of this record and `changelog.d/synced-grade-owner-press-390.md`, with `#715` closed by a neutral comment once it lands, because the decision changes no behaviour and SPEC-365's section 5 cites `#715` for a decision, which this record is.
- Chosen against: an insert-only amendment to ADR-376 naming this record, because this record changes nothing ADR-376 decided, and an amendment section records a changed decision.
- Chosen against: closing `#715` with a comment and no record, because a comment is not a decision the tree carries, and the exclusion that opened it is in a SPEC.

## Decision Outcome

D1 to D6 as chosen above. A grade that arrives by sync is outside the owner-press rule. The rule
holds the one call that records a grade to a press in DeckStreak's own client, checked there when
the grade is recorded. Sync records no grade: it brings in reviews recorded elsewhere, which stay as
they were recorded. No code, test, table row, census, model or ruling changes.

## Consequences

- Good: the rule keeps one meaning and one door, and both clients and the service's copy keep the
  reviews made on every client, a stock client's included, as the per-review client record ruling
  expects.
- Good: nothing new is kept to tell clients apart.
- Bad: a grade made on a stock client is kept and read like any other review, and DeckStreak cannot
  hold it to a press; once the per-review client record is built, the owner can see which client
  made it, and nothing else reads that.
- Neutral: a press after an arriving grade for the same shown card is two presses, each on its own
  client; whether the engine refuses the later one as stale stays ADR-376 D3's.

### Confirmation

The four tests D3 names, unchanged at this record's delivery, and `#715` closed once it lands.

## What would make this wrong

- If the owner rules that the owner-press rule also governs grades recorded outside DeckStreak's
  clients, an arrival check becomes a design question that D2's rejections still bound: it may
  neither remove a review nor keep anything new to tell clients apart.
- If a DeckStreak path is found that records a synced grade through (13,4), or through
  `answer_card` outside the held fixture lines, that path is under the rule, and the census refuses
  it.
- If the engine's sync gains a per-review field that a pressing client can set, D2's attestation
  option is read again against the per-review client record ruling.

## More Information

- SPEC-365 section 5 (`#715`), ADR-376 D2, D3 and D11, ADR-389 (`#716`), SPEC-364 (the web's
  normal sync), SPEC-358 R17 (`#633`), the owner-taps ruling and the per-review client record
  ruling.
