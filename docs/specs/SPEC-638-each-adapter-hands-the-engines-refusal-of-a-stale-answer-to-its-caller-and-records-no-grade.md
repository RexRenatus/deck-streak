# SPEC-638: each adapter hands the engine's refusal of a stale answer to its caller and records no grade

Status: ready to build, one delivery. Issue: #937. Decided by ADR-661.
Amends, insert-only: docs/schematics/owner-press-to-recorded-grade.md (section 7 appended).

## 1. The problem, measured

Every figure was read at `dev` `36efa139`, by the command named beside it.

- The core's answer door already holds the engine's refusal of a stale current state: SPEC-460's
  two tests in `crates/engine-core/tests/stale_answer.rs` call `run_answer` directly, on a native
  dispatcher, and spell the refusal's message head `card was modified` (`:47`, by `grep -n`).
- Neither adapter's own handover of that refusal is run by any test. A fenced `git grep -n -E
  'PressRefusal::Engine|card was modified|engine refused the answer'` with the drill and stand-in
  excludes finds 29 lines: 8 under `crates/` and 0 under `web/`. Under `crates/ffi/tests` the only
  hits are `refusal_text.rs:71` and `:80`, which build `PressRefusal::Engine` by hand to read its
  sentence; no test reaches it through a press the engine refused. Under `web/app/tests-engine`
  no test reads the core's sentence for an engine refusal.
- The native handover, by `grep -n` over `crates/ffi/src/engine.rs`: `Engine::answer`
  (`:203-232`) calls the core's door at `:223-226` and returns its refusal through
  `press_refusal` (`:226`), whose arm at `:739` (`press_refusal`, `:734-741`) hands the engine's
  error bytes on as `PressRefusal::Engine { error }`.
- The web handover, by `grep -n` over `crates/web-engine/src/wasm.rs`: `rate` (`:874-912`) calls
  the core's door at `:890-892` and throws its refusal by `.map_err(refuse)?` at `:892`, before it
  forgets the shown card at `:893`. The Worker codes a thrown message that opens with no known
  prefix as `engine-failed` (`web/app/src/lib/engine/session.ts`, `refusalCode`).
- The schematic `docs/schematics/owner-press-to-recorded-grade.md` says so in its section 6
  table: "the adapter's own lines are not run here" (`grep -c`: 2, one per adapter). Its line
  numbers there (`engine.rs:184-206`, `:266`; `wasm.rs:818-856`, `:837`) are older than this
  `dev`, as are the issue's own (`engine.rs` lines 261-268, `wasm.rs` line 837).
- The web boundary census does not hold the web mapping. `cargo test -p deck-streak-web-engine
  --test boundary -- --nocapture --test-threads=1` prints `examined 39 boundary function(s) of
  src/wasm.rs` and then `examined 129 owed statement(s)`; `rate`'s entry in `OWED` owes 8
  statements, and none of them is the refusal's mapping. A program that removes every blank from
  `rate`'s body counts the statement `&request.encode_to_vec()).map_err(refuse)?` once there, and
  0 times in the unsqueezed text.
- The web module runs only in a browser: `crates/web-engine/Cargo.toml` puts the engine core
  behind `cfg(target_arch = "wasm32")` (`grep -n`: `:30`), so no native test can call `rate`.

## 2. Requirements

R1. A native press whose states were shown before another answer changed the card reaches its
caller as `PressRefusal::Engine { error }`, whose `error` decodes as the engine's `BackendError`
of kind `INVALID_INPUT` (field 2 reads 0) with its message (field 1) headed `card was modified`
up to its first colon.

R2. That refused press records nothing: `Engine::answer` returns the refusal and no bytes, and
the queue read after it (`SchedulerService.GetQueuedCards` (13, 3) through `Engine::run`,
`fetch_limit` 1: its head card id, its head queue and the new and learning counts) equals the
queue read right after the first press.

R3. In the browser, a rating through the Worker client on the card the page shows, after a sync
brought in another browser's answer of that card, reaches the page as the client's refusal with
code `engine-failed` and a message matching `^the engine refused the answer \(\d+ bytes\)$` (the
core's sentence for `AnswerRefusal::Engine`, `crates/engine-core/src/answer.rs:90-91`, thrown by
`rate`'s `.map_err(refuse)?` and coded by `session.ts`' `refusalCode`).

R4. That refused rating records nothing: the card's review count in the page's collection
(`client.snapshot(card).reps`, read by the browser test's `reps` helper) reads 1 after the sync
and still 1 after the refusal.

R5. The web boundary census owes `rate` the statement `&request.encode_to_vec()).map_err(refuse)?`
(its `OWED` entry for `rate`, compared with every blank removed), so a `rate` that swallows or
replaces the refusal fails the census by name.

R6. Three hand rows (section 8), each killed by its criterion's test and proved by hand
(section 10).

R7. No product code changes: the delivery's diff names no path under `crates/*/src`,
`web/app/src` or `web/app/engine-harness`; the schematic is insert-only; SPEC-460, ADR-477 and
ADR-376 are unchanged.

## 3. Acceptance criteria of SPEC-638

| A | criterion | decided by |
|---|---|---|
| A1 | R1 and R2: a native press on a card changed since it was shown is the engine's refusal and records nothing | `stale_press::a_native_press_on_a_card_changed_since_it_was_shown_is_the_engines_refusal_and_records_nothing` |
| A2 | R3 and R4: a rating on a card a sync changed since it was shown is the engine refusal and records nothing, in Chromium and WebKit | the Playwright test titled `a rating on a card a sync changed since it was shown is the engine refusal and records nothing` in `web/app/tests-engine/sync.spec.ts`, decided in CI's `web-engine` job by name |
| A3 | R5: the web boundary census owes `rate` the line that throws the refusal | `boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher`, existing, with R5's statement added |

```acceptance
A1: cargo test -p deck-streak-ffi --test stale_press -- --exact a_native_press_on_a_card_changed_since_it_was_shown_is_the_engines_refusal_and_records_nothing
A2: pnpm --dir web/app test:engine -g "a rating on a card a sync changed since it was shown is the engine refusal and records nothing"
A3: cargo test -p deck-streak-web-engine --test boundary -- --exact each_boundary_function_reaches_the_engine_through_the_dispatcher
```

A2 needs the wasm module built by `scripts/web-engine-build.sh` with its pinned tools, and runs in
both browser projects of the engine's Playwright configuration.

## 4. File manifest

| path | change |
|---|---|
| `docs/specs/SPEC-638-each-adapter-hands-the-engines-refusal-of-a-stale-answer-to-its-caller-and-records-no-grade.md` | added: this SPEC |
| `docs/decisions/ADR-661-each-adapters-handover-of-the-engines-answer-refusal-is-held-by-a-test-through-the-adapter.md` | added: the decision |
| `docs/schematics/owner-press-to-recorded-grade.md` | amended: section 7 appended, no line changed |
| `changelog.d/adapter-stale-answer-refusal-638.md` | added: the changelog fragment |
| `crates/ffi/tests/stale_press.rs` | added: the test of A1 |
| `crates/web-engine/tests/boundary.rs` | changed: one statement added to `rate`'s `OWED` entry (A3) |
| `web/app/tests-engine/sync.spec.ts` | changed: the test of A2 appended after the file's last test |
| `docs/red-first/SPEC-638.md` | added: the red-first record (section 10) |
| `scripts/mutation-rows.d/S63800-S63899.json` | added: rows S63801 to S63803 (section 8) |

## 5. What this does NOT cover

- The core's answer door and the engine's comparison, which SPEC-460 and ADR-477 already hold at
  the core (#760).
- A grade that arrives by sync, which ADR-404 decides (#715).
- The issue's own line citations, which are older than this `dev`; section 7 of the schematic
  states the current ones and the issue is left as filed (#937).
- What only this delivery's CI run can measure: A2 in both browsers, and the rows' verdicts by id
  (#937).

## 6. Risks

- An engine bump that rewords the message head turns A1 red. That is the pin working: the bump's
  delivery re-reads the head from the engine's source and re-pins it, never loosens it.
- The browser test needs two browsers on one sync account, so a test that runs beside it on the
  same account could change the server's collection between its upload and its sync. The engine's
  Playwright configuration gives each browser project its own account, and the test makes the
  server's collection itself by a full upload first, as the other full-sync tests do.
- A census that reads test sources for the engine's or the gesture's names
  (`crates/engine-core/tests/containment.rs`) would refuse a new test line that named one: neither
  the new native test nor the census's new statement names any, and the containment suite runs
  green unchanged.
- An open pull request that changes the browser test file's helpers could move a helper this test
  calls: the test is appended after the file's last test and calls only helpers the file defines.

## 7. What decides R6 and R7

| R | decided by |
|---|---|
| R6 | each row's hand proof (its mutant installed once, its killer run by `--exact`, the failing assertion read, the file restored by sha256), recorded in section 10's record, and CI's rows job by id |
| R7 | `git diff --name-only` of the delivery names no `crates/*/src`, `web/app/src` or `web/app/engine-harness` path, and `git diff --numstat` of the schematic reads `<n> 0` |

## 8. Mutation rows, band `S63800-S63899`

Each row is in the `MUTATIONS` table, and each find occurs once in its file at `dev` `36efa139`
(a program over `git show`: 1, 1 and 1).

| id | crate and file | find | replacement | killer |
|---|---|---|---|---|
| `S63801-THE-NATIVE-PRESS-NAMES-THE-ENGINES-REFUSAL` | `ffi`, `src/engine.rs` | `AnswerRefusal::Engine { error } => PressRefusal::Engine { error },` (`:739`) | `AnswerRefusal::Engine { .. } => PressRefusal::NotTheCard,` | A1's test |
| `S63802-A-REFUSED-NATIVE-PRESS-RECORDS-NOTHING` | `ffi`, `src/engine.rs` | `.map_err(press_refusal)?;` after the door's call (`:225-226`) | `.unwrap_or_default();` | A1's test |
| `S63803-THE-WEB-RATE-THROWS-THE-ENGINES-REFUSAL` | `web-engine`, `src/wasm.rs` | `.map_err(refuse)?;` after the door's call (`:891-892`) | `.unwrap_or_default();` | A3's test |

## 9. Formal

Not required (ADR-661 D6). The delivery adds tests, rows and documents; it adds no state, protocol,
concurrency or pure decision function. `formal/tla/AnswerOnTheShownState` covers the core's door
(`crates/engine-core/src/dispatch.rs` and `crates/engine-core/src/answer.rs`), and neither file
changes.

## 10. Red-first

Both handovers are on `dev`, so no criterion can be red at the base for its reason without
changing what is pinned (ADR-661 D5). `docs/red-first/SPEC-638.md` carries one line per criterion
in its `red-first` fence, each `not red`: A1's reds are rows S63801 and S63802, A3's red is row
S63803, and A2 has no row, since no row's killer can run it, while its text is held by A3's row.
Below the fence, each row's proof by hand: its mutant installed, its killer's failing line, the
file restored by sha256, and the killer green again.
