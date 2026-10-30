# SPEC-389: a Lean entry proves a study day's confirmed XP is never below the XP the device showed, at every settle point, and the shipping settle rule answers its vectors

- **Wave:** the app campaign (SPEC-334 R13 and section 9).
  **Issue:** #755. **Context(s):** `formal` (the Lean entry, its writer and its vectors),
  `deck-streak-progression` (one integration test; no source change).
- **Decided by:** ADR-403, ADR-072 (the settle rule this entry ports), ADR-371 (the XP crate; its
  "More Information" names #639 for instant XP and the reconciliation, and this delivery builds the
  entry's server side, leaving the client tie to #639).
- **Status:** judged by this delivery, with its tests, `docs/red-first/SPEC-389.md` and the formal
  check quoted in section 8.

## 1. The problem, measured

Every figure below was read at `dev` `3ca06142923a7c93ea3298c6b9c17987e4ca9531` (BASE) with
`git show BASE:<path>` and `git grep` at BASE.

- **The requirement.** SPEC-334 R13 (`docs/specs/SPEC-334-*.md:79-82`): per-review XP shows on the
  device at once, from the XP crate; day-level bonuses stay pending until sync; one server ledger
  confirms; confirmed XP is never below shown XP. Its section 9 (`:235-237`) asks for "a Lean entry
  for XP reconciliation": for any sequence of reviews and day-level bonuses, the XP the server
  confirms is never below the XP the client showed. `docs/schematics/app-clients-engine-and-sync.md:91-96`
  draws the same edge.
- **Where XP is confirmed.** `crates/progression/src/settle.rs` keeps one `xp_settlement` row per
  study day, source and track. `settle` (`:96-139`) decides the row at `:117-122`: a Recompute over
  a held row keeps the larger amount and closes the row when either the held row or the request is
  closed; every other case writes the request (`:124-128`). `SettleCause` has two arms, Recompute
  and OwnersCorrection (`:52-57`). `settled_of_day` (`:159`) reads a day's rows back.
  `docs/schematics/xp-grants-and-settlement.md:34-49` draws the row's states: an open day's row
  follows the record and can fall; a closed day's row only rises, except by the owner's correction.
- **What the fold settles.** `crates/coordination/src/recompute/xp.rs::evaluate` (`:95`) sums a
  day's review XP per track (`:105-116`): an answer whose card is unknown counts on the language
  track with no tier (`.unwrap_or((Track::Language, None))`), each total is a saturating sum, and the
  tier applies to the law track alone (`:80`). It settles `reviews` and `reviews_law` (`:117-136`)
  with the Recompute cause and `closed` true for any day but the current one (`:102`).
- **Who lowers a row.** The one OwnersCorrection caller is `crates/coordination/src/habits/minutes.rs`
  (`:26`, `:268`), on habit sources; no caller passes it on a review source.
- **The price.** `crates/xp/src/review_xp.rs::review_xp` (`:59`) prices one answer, 0 for an answer
  that is not a study event (`:60-62`), the tier's multiplier or the untagged one (`:79`), rounded
  half to even (`:80`). `crates/progression/src/review_xp.rs:15-17` translates progression's review
  into it. `economy.json:38-46`: the untagged multiplier (1.0) is the least of the tier multipliers
  (1.0, 2.0, 3.0, 5.0).
- **Where XP is shown.** No client computes XP at BASE: a case-sensitive search of the web engine,
  the engine core, the FFI crate, the native app and the web study and engine modules names no XP
  function, and only progression depends on the XP crate. The web app reads the server's rows
  (`crates/api/src/xp_routes.rs:3-6`; `web/app/src/lib/level/level.ts:11-12`). SPEC-360
  (`:36-40`) leaves instant XP on the device, pending bonuses and the reconciliation to #639.
- **Undo and sync.** An answer is undone only before it syncs, and a sync discards the undo queue
  (`docs/schematics/undo-the-reviews-own-last-answer.md:46`, `:52`).
- **Two folds overlap.** `crates/coordination/src/recompute/mod.rs::run` reads its facts once, at
  its start (`:512-513`), and writes the current day from them later (`:574-577`) with no re-read;
  the cursor is re-read inside each owed day's write (`:549-552`, ADR-313).
  `formal/tla/FoldSettlesOnce/FoldSettlesOnce.tla:14-16`: the scheduled cycle and the owner's
  recompute each run a fold, and their folds overlap. So a write from facts read before the last
  arrival can land after a write from newer facts.
- **The formal tree.** `formal/lean/lean-toolchain:1` pins the Lean toolchain;
  `formal/lean/lakefile.toml:6-8` builds one library, `Formal`, from every `Formal.*` module, with no
  package dependency. Ten Lean entries and 24 TLA+ models are in the tree. Each entry is tied to the
  code by `-- @phx covers <path> anchor=<item> digest=sha256:<...>` lines and by vectors its writer
  emits through a one-line arm in `formal/lean/Formal/Vectors.lean` (`:26-37`), which a Rust test of
  the covered crate answers (`crates/progression/tests/formal_vectors_exchange.rs`). Of 219 `covers`
  lines, two name `settle.rs` (`formal/tla/HabitXpFollowsItsLog/HabitXpFollowsItsLog.tla:7` and
  `formal/tla/MintReadsTheFinalBase/MintReadsTheFinalBase.tla:7`); none names `recompute/xp.rs` or
  either `review_xp.rs`. No entry or vector set is about the floor.
- **The formal stage.** `.github/workflows/ci.yml` names no formal job (0 matches for `lean`, `tla`,
  `lake`, `formal` or `phxd` in the workflows). The formal check runs outside CI, by the builder at
  its head, the verifier and the orchestrator at land. CI's `rust` job (`ci.yml:70`) runs the
  progression tests, so a vectors test there runs on every pull request.
- **The census.** `crates/progression/tests/xp_census.rs:13` accepts progression's own callers of
  `settle`, and `:73` admits test, bench and example targets as callers.

## 2. Requirements

**The claim.**

- R1. *Shown XP* of study day d is the saturating sum, at the `u32` maximum, of the XP the device
  shows for each of its own study-event answers of d that is not undone (SPEC-334 R13). Day-level
  bonuses are never in it.
- R2. *Confirmed XP* of d is the sum of every settled row of d: `reviews`, `reviews_law` and each
  bonus source.
- R3. A *settle point* of d is a state in which every answer the device holds for d has synced, and
  the last write to d's rows came from a fold whose facts held every answer the server holds for d.
- R4. The entry proves, over every finite trace of the events of R6, that at every settle point of
  d, shown XP of d is at most confirmed XP of d. It holds under two stated premises: each answer's
  shown XP is at most the server's price for it, and no event removes an answer from the server's
  answers of d.

**The model.**

- R5. `formal/lean/Formal/XpReconciliation.lean` ports, branch for branch:
  - the settle rule (`settle.rs:117-122`) over naturals that never exceed the `u32` maximum;
  - the per-track fold of `evaluate` (`xp.rs:105-116`): an unknown card counts on the language
    track, and each total is a saturating sum, parametric in each answer's XP.
- R6. The protocol's events are:
  - the device's grade of an answer, and its undo of its last unsynced answer;
  - the device's sync, which delivers every unsynced answer;
  - another client's answer reaching the server;
  - a fold's read of the server's answers;
  - a fold's write of d's rows, from the answers it read, with the Recompute cause and d's closed
    flag;
  - d's close.
  A grade after the close belongs to the next day. A restart is the identity: the device derives
  shown XP from its own log.
- R7. The entry declares four theorems at `ramp=report`, each with one witness: a mutated port that
  violates it at one input.
  - T1 `a_closed_row_is_never_lowered_by_a_recompute`;
  - T2 `the_close_keeps_the_last_provisional_amount`;
  - T3 `a_days_review_xp_never_falls_as_answers_arrive`;
  - T4 `confirmed_xp_is_never_below_the_xp_shown`.
- R8. Its header covers `crates/progression/src/settle.rs` anchor `settle`,
  `crates/coordination/src/recompute/xp.rs` anchor `evaluate`, `crates/xp/src/review_xp.rs` anchor
  `review_xp` and `crates/progression/src/review_xp.rs` anchor `review_xp`. It declares its vectors
  and cites #755 and #639.
- R9. The proofs use no `sorry`, no new axiom and no native evaluation. `#print axioms` of every
  theorem and witness lists only the axioms `config/formal.json` allows.

**The vectors and their test.**

- R10. `formal/lean/Formal/XpReconciliationVectors.lean` writes `formal/vectors/xp-reconciliation.jsonl`
  through one import and one arm in `formal/lean/Formal/Vectors.lean`. The file is regenerated,
  never edited by hand, and holds 1699 lines:
  - a header in the house form, naming the axes;
  - 144 settle vectors (9 held rows by 8 requests by 2 causes): a held row of none, or of an amount
    in {0, 30, 50, 4294967295}, open or closed; a request of those amounts, open or closed; and
    either cause;
  - 1554 trace vectors (6 + 36 + 216 + 1296): every word of length 1 to 4 over six letters.
- R11. The six letters are:
  - G, a language answer the device shows at 7 and the server prices at 7;
  - H, a law answer the device shows at 7 and the server prices at 14;
  - U, an undo;
  - S, a sync;
  - R, one fold's read and write while d is open, which settles `score90` at 200 when the server
    holds an odd number of d's answers, else 0;
  - N, d's close, then one fold's read and write with `score90` at 0.
  Each trace vector lists every settle, with its step, source, request and the row held after it,
  and lists each settle point, with its shown and confirmed XP.
- R12. `crates/progression/tests/formal_vectors_xp_reconciliation.rs` reads the vectors with
  `include_str!`. It answers every settle vector, and replays every trace's settles, through the
  shipping `settle` on a scratch ledger, one study day per vector. At each settle point it reads the
  day's rows back with `settled_of_day` and asserts that their sum equals the vector's confirmed XP
  and is at least its shown XP. It prints the examined count, refuses zero, and checks the header's
  count against it.

**What does not change.**

- R13. No product code changes: no source under `crates/*/src`, `web/` or `ios/`, no `Cargo.toml`,
  no migration and no `config/formal.json`.
- R14. Rows S38900 to S38904 mutate the settle rule in `settle.rs`, and A1's test kills each.
- R15. `docs/schematics/xp-grants-and-settlement.md` gains one final section, insert-only.

## 3. Acceptance criteria of this delivery

| id | criterion | decided by |
|---|---|---|
| A1 | the shipping `settle` answers every settle vector: the amount it returns and the row's closed flag read back equal the vector's | `cargo test -p deck-streak-progression --test formal_vectors_xp_reconciliation -- --exact the_settle_rule_answers_every_lean_vector` |
| A2 | every trace's settles, replayed through the shipping `settle`, hold the rows the vector says, and at every settle point the day's confirmed XP read back equals the vector's and is at least its shown XP | `cargo test -p deck-streak-progression --test formal_vectors_xp_reconciliation -- --exact every_trace_confirms_at_least_the_xp_shown` |
| A3 | the witnesses' settle inputs answer on the shipping `settle` as T1 and T2 state: a closed row of 50 under a Recompute request of 30 holds 50, closed; an open row of 50 under a closing Recompute request of 30 holds 50, closed | `cargo test -p deck-streak-progression --test formal_vectors_xp_reconciliation -- --exact the_xp_reconciliation_counterexamples_answer_as_proved` |
| A4 | the vectors hold every case of the axes the test declares exactly once, and the header's count equals the count examined | `cargo test -p deck-streak-progression --test formal_vectors_xp_reconciliation -- --exact the_vectors_cover_every_case_of_the_axes` |

```acceptance
A1: cargo test -p deck-streak-progression --test formal_vectors_xp_reconciliation -- --exact the_settle_rule_answers_every_lean_vector
A2: cargo test -p deck-streak-progression --test formal_vectors_xp_reconciliation -- --exact every_trace_confirms_at_least_the_xp_shown
A3: cargo test -p deck-streak-progression --test formal_vectors_xp_reconciliation -- --exact the_xp_reconciliation_counterexamples_answer_as_proved
A4: cargo test -p deck-streak-progression --test formal_vectors_xp_reconciliation -- --exact the_vectors_cover_every_case_of_the_axes
```

**The red each criterion shows first.** The red commit carries the Lean package with a settle port
that lacks the closed-day arm (every settle writes the request, the rule ADR-072 replaced), its
writer, the vectors it wrote, and the test. The shipping `settle` answers differently, so A1 and A2
are red by assertion, not by a compile error. The green commit gives the port the arm, proves the
theorems and writes the vectors again. The test's text is the same at both commits, and no Rust
source changes.

| id | red first, at which commit, for which reason |
|---|---|
| A1 | at the red commit: the first settle vector with a closed held row and a smaller request, where the port answers the request and the shipping `settle` keeps the held amount, closed |
| A2 | at the red commit: trace `HSRN`, whose close settles `score90` at 0 over a held 200; the port's vector holds 0 and the shipping `settle` 200 |
| A3 | not red: the shipping `settle` already answers the witnesses' inputs as the theorems state; the test guards them |
| A4 | not red: the axes already held every case at the red commit; a population lacking one case is the control, measured in a scratch copy and never committed |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-389-a-lean-entry-proves-confirmed-xp-is-never-below-the-xp-shown.md` | docs | added: this SPEC |
| `docs/decisions/ADR-403-the-xp-floor-is-proved-over-the-settle-rule-and-tied-by-vectors-the-server-answers.md` | docs | added: the decision |
| `docs/schematics/xp-grants-and-settlement.md` | docs | changed, insert-only: a final section, the floor and its proof |
| `formal/lean/Formal/XpReconciliation.lean` | `formal` | added: the ports, the protocol, T1 to T4 and their witnesses |
| `formal/lean/Formal/XpReconciliationVectors.lean` | `formal` | added: the vector writer |
| `formal/lean/Formal/Vectors.lean` | `formal` | changed: one import and one arm in `main` |
| `formal/vectors/xp-reconciliation.jsonl` | `formal` | added: 1699 lines the writer emits |
| `crates/progression/tests/formal_vectors_xp_reconciliation.rs` | `deck-streak-progression` | added: A1 to A4 |
| `scripts/mutation-rows.d/S38900-S38999.json` | rows | added: S38900 to S38904 |
| `docs/red-first/SPEC-389.md` | docs | added: the red-first record |
| `changelog.d/lean-xp-floor-389.md` | docs | added: the changelog fragment |

## 5. What this does NOT cover

- The device's shown-XP code on the web and native clients, the track and tier it files an answer
  under, the display of pending bonuses, and the `covers` lines and vectors that would tie a client's
  shown total to this entry are #639's. Until #639 lands, shown XP is the model's R1, and the
  premise that each answer shows at most its server price is #639's to hold.
- An answer that leaves the server's answers of a day, because its card is deleted, its deck leaves
  the ingested scope, or the collection is replaced, can lower an open day's review rows. The model
  has no such event; the full-sync choice is #631's, and a shown total derived over the same scope
  is #639's.
- Two folds whose writes land out of the order of their reads can leave an open day's row below the
  XP shown until the next fold. The claim holds at a settle point as R3 defines it, and this
  delivery changes no fold (#756 is that lapse's own model-first fix).
- A change to the price or the tier table, which could break the premise that the untagged
  multiplier is the least, is #62's and #64's.

## 6. Risks

- **A moved span reads STALE in report mode.** A change to `settle`, `evaluate` or either
  `review_xp` stales every theorem of the entry, and report mode still exits 0. The formal check is
  read by each theorem row's `clean`, never by its exit code; section 8 records it.
- **The fold is tied by its digest alone.** No progression test can call coordination's `evaluate`.
  A change to its per-track sum reads STALE at the next formal check, and fails no Rust test.
- **The census.** If `xp_census.rs` stops admitting test targets as callers of `settle`, it refuses
  the new test. The whole progression suite runs at every step, so this would fail before the push.
- **The overlap.** The proof does not exclude the lapse that section 5's third bullet names. A
  reader who drops R3's last clause states a claim the code does not keep.
- **Size.** The vectors file holds 1699 lines. The writer must run within `config/formal.json`'s
  budgets, and the test within the `rust` job's time; the builder measures both at its head.

## 7. Mutation rows

`S389` is this delivery's row stem: `S` and the SPEC's number. Every row has crate `progression`,
file `src/settle.rs`, and killer
`formal_vectors_xp_reconciliation::the_settle_rule_answers_every_lean_vector`. Each `find` occurs
exactly once in the file.

| row | what it mutates |
|---|---|
| `S38900-THE-CLOSE-READS-THE-REQUESTS-FLAG` | `if held.closed \|\| request.closed =>` to `if held.closed =>` |
| `S38901-A-RECOMPUTE-KEEPS-THE-LARGER` | `held.amount.max(request.amount)` to `held.amount.min(request.amount)` |
| `S38902-A-HELD-CLOSED-ROW-STAYS-CLOSED` | `(held.amount.max(request.amount), true)` to `(held.amount.max(request.amount), request.closed)` |
| `S38903-A-FIRST-SETTLE-KEEPS-ITS-FLAG` | `_ => (request.amount, request.closed),` to `_ => (request.amount, false),` |
| `S38904-ONLY-THE-RECOMPUTE-KEEPS-THE-LARGER` | `(Some(held), SettleCause::Recompute) if` to `(Some(held), _) if` |

## 8. The formal check, outside CI

The formal check of `lean/XpReconciliation`, and the ratchet over the whole tree, are run by the
builder at its head, re-run by the verifier, and re-run by the orchestrator at land. The pull
request's hand-back quotes the result. The check must read:

- no `HOLE`, `MODEL_ERROR`, `DERIVED_DRIFT`, `STALE` or `WITNESS_SURVIVED`;
- `clean` true on each of T1 to T4;
- each witness built.

A theorem moves from `ramp=report` to `ramp=block` only by the streak of clean landings the checker
counts, never by this delivery.
