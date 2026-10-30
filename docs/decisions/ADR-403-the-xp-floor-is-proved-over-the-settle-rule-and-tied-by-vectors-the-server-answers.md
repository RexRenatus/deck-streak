---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# The XP floor is proved over the settle rule and a trace protocol, and tied to the code by covers and by vectors the shipping settle answers

## Context and Problem Statement

SPEC-334 R13 says confirmed XP is never below shown XP, and its section 9 asks for a Lean entry
that proves it. SPEC-389 section 1 records what the tree holds at `3ca06142`. The server confirms
XP in `xp_settlement` rows, which `settle` decides (`crates/progression/src/settle.rs:117-122`)
from the totals the fold's `evaluate` sums (`crates/coordination/src/recompute/xp.rs:105-116`). No
client computes XP yet: shown XP is SPEC-334 R13's definition, and #639 builds it. Two folds can
overlap (`formal/tla/FoldSettlesOnce/FoldSettlesOnce.tla:14-16`), and each writes the current day
from facts it read at its start (`crates/coordination/src/recompute/mod.rs:512-513`, `:574-577`).

This record answers five questions. What exactly is claimed? What does the entry model, and how is
it held to the code? Where is it checked? Which tests show it red first? Does it ship as one
delivery?

## Decision Drivers

- The claim must be true of the code at the base, or it is a question, never a guess.
- A model that drifts from the code must be caught by a check, not by a reader.
- The delivery changes no product code; the client half is #639's.
- DeckStreak's formal verdict comes from one checker run outside CI; CI runs the Rust tests.
- Each criterion is red first for its own reason, or recorded `not red` with the reason.

## Considered Options (the alternatives it was chosen against)

### D1, the invariant

- Chosen: shown XP is the device's saturating sum of its own per-review XP for a study day, with no day-level bonus, because SPEC-334 R13 defines it so.
- Chosen: confirmed XP is the sum of the day's settled rows, because those rows are what the server confirms.
- Chosen: the claim holds at every settle point (SPEC-389 R3), over grades, undos, syncs, other clients' answers, fold reads and writes, and the close, because those are the events the tree has.
- Chosen: a restart is the identity, because the device derives shown XP from its own log.
- Chosen against: the web app's provisional row as shown XP: lost, because an open day's row falls by design (`docs/schematics/xp-grants-and-settlement.md:44-46`), so the claim would be false at the base.
- Chosen against: the floor at every step: lost, because between a grade and its sync the device shows XP the server has not seen, as R13 intends.
- Chosen against: a settle point as "a fold settled after the last arrival": lost, because a write from older facts can land after a newer one (`mod.rs:512-513`, `:574-577`), so that claim is false at the base.
- Chosen against: no premise on price or scope: lost, because the server prices an unknown card as untagged on the language track (`xp.rs:105-116`), and the tree can drop an answer from the scope it ingests; neither is the device's.

### D2, the model and its tie to the code

- Chosen: the entry ports the settle rule and the per-track sum branch for branch, and models the protocol over traces, because the settle rule is the one step that keeps or lowers a row, and the sum is what each settle requests.
- Chosen: `covers` lines on `settle`, `evaluate` and both `review_xp`, because a moved span then stales every theorem, including a change to the price that the premise rests on.
- Chosen: vectors the shipping `settle` answers in a progression test, because the test calls the real function on a real ledger.
- Chosen against: a port of `review_xp`: lost, because its multipliers are not dyadic and it rounds in `f64`, while the floor needs only the premise that each answer shows at most its price, which SPEC-360's goldens already hold.
- Chosen against: covers alone: lost, because a digest says a span moved, never that the port answers what the code answers.
- Chosen against: a pure settle-rule function the test could call without a ledger: lost, because it changes product code the census and the rows hold, for a tie the scratch ledger already gives.
- Chosen against: a coordination test that answers the fold's vectors: lost, because `evaluate` is not public, and a test of it needs a new seam in coordination; its digest stales the entry instead.

### D3, the formal stage

- Chosen: the entry sits at `formal/lean/Formal/XpReconciliation.lean` in the one `Formal` library, built by the pinned toolchain. The packs' checker runs `formal check` on `lean/XpReconciliation`, and the ratchet over the tree, against live dev: by the builder, the verifier and the orchestrator at land. This is chosen because DeckStreak's formal verdict has one checker, run outside CI.
- Chosen: the vectors test runs in CI's `rust` job (`ci.yml:70`), because it is a progression test.
- Chosen against: a CI job that builds the Lean library: lost, because a second formal gate would split the verdict, and its toolchain would be a second pin.

What the check reports: a proof that does not elaborate reads `MODEL_ERROR`; a `sorry`, an axiom or
a native evaluation reads `HOLE`; a witness that does not build reads `WITNESS_SURVIVED`, a fail;
vectors that differ from the writer's output read `DERIVED_DRIFT`; a moved covered span reads
`STALE`.

### D4, the tests, red first, and the controls

- Chosen: the red commit's port lacks the closed-day arm, so A1 and A2 are red by assertion. The green commit adds the arm, proves the theorems and writes the vectors again. A3 and A4 are recorded `not red`. This is chosen because the delivery adds no behaviour to the code, so the only red a criterion can show is the model disagreeing with the code.
- Chosen: three controls in a scratch clone, never committed, because each proves the check can refuse: a broken proof must read `MODEL_ERROR`, a `sorry` must read `HOLE`, and a one-line vector edit must read `DERIVED_DRIFT`.
- Chosen against: red from an absent vectors file: lost, because a missing fixture is not the criterion's reason.
- Chosen against: all four criteria `not red`: lost, because then nothing shows that the test can tell a wrong model from a right one.

### D5, the shape

- Chosen: one delivery, with no product change, because the server's half is the entry, its writer, its vectors and one test file in one crate, and no client computes XP at the base.
- Chosen against: the model first and the code tie in a later delivery: lost, because the tie is one test file that the same round writes.
- Chosen against: the client tie now: lost, because no client computes XP at the base, and #639 builds it.

### D6, no TLA+ model

- Chosen: Lean alone, with each fold's read and write as separate events of the trace, because induction over traces proves the floor for every interleaving, of every length.
- Chosen against: a TLA+ model of the two cycles: lost, because TLC checks a bounded instance of a claim that is over every trace. The lapse the overlap allows is a product question, and its fix would carry its own model.

### D7, the schematic

- Chosen: amend `docs/schematics/xp-grants-and-settlement.md` with one final section, insert-only, because it owns the settle rule and the rows' states.
- Chosen against: a new schematic: lost, because it would draw the rows' state machine a second time.
- Chosen against: amending `docs/schematics/app-clients-engine-and-sync.md`: lost, because it owns the clients and the sync, not the rows.

### D8, rows on code the delivery does not change

- Chosen: rows S38900 to S38904 on the settle rule, each killed by A1, because the vectors test is the delivery's only new killer, and the rows show that it fails on the code it ties.
- Chosen against: no rows, since no product code changes: lost, because then nothing proves A1 observes the code rather than the file.

### D9, the witnesses

- Chosen: each witness is a mutated port that violates one theorem at one input, named `..._violates`, as `formal/lean/Formal/Exchange.lean:7-18` names its witnesses, because the shipping code already satisfies every claim.
- Chosen against: witnesses ported from the predecessor's rule alone: lost, because only T1 has a predecessor rule to port (ADR-072's replace of a closed row).

## Decision Outcome

The chosen options are D1 to D9, as each "Chosen:" bullet states. Together they prove the floor
SPEC-334 R13 names, at the points where it holds at the base, over every trace. They hold the port
to the shipping settle rule by vectors in CI, and the fold and the price by their digests. No
product code changes.

### The witnesses

| witness | kills | the input |
|---|---|---|
| `a_recompute_that_replaces_a_closed_row_violates` | T1 | a closed row of 50, a Recompute request of 30, open: the port holds 30 |
| `a_close_that_reads_only_the_held_flag_violates` | T2 | an open row of 50, a closing Recompute request of 30: the port holds 30 |
| `a_total_that_wraps_at_the_top_violates` | T3 | answers of 4294967295, then 4294967295 and 1: the wrapping total falls to 0 |
| `a_device_that_shows_a_bonus_at_the_grade_violates` | T4 | trace `GGSR`: the device shows 214, and the day confirms 14 |
| `a_shown_total_that_keeps_an_undone_answer_violates` | T4 | trace `GUR`: the device shows 7, and the day confirms 0 |

### Consequences

- Good, because a change to the settle rule that the port does not follow fails A1 in CI.
- Good, because a change to the fold or the price stales the entry at the next formal check.
- Good, because #639's client inherits a stated premise (shown at most the price) and a stated
  scope (the server's answers of the day).
- Bad, because the fold's tie is a digest, not a vector, so its drift is read at the formal check,
  outside CI.
- Bad, because the theorems start in report mode, and a STALE row does not move the exit; a reader
  must read each row's `clean`.
- Bad, because the floor is proved only at settle points as SPEC-389 R3 defines them. The lapse
  that two overlapping folds allow is left to a product decision.

### Confirmation

SPEC-389 A1 to A4 in CI; the red-first record `docs/red-first/SPEC-389.md`; rows S38900 to S38904;
and the formal check SPEC-389 section 8 describes, quoted at the head.

## More Information

#755, #639, #631, #62, #64; SPEC-334 R13 and section 9; SPEC-360; SPEC-072 and ADR-072 (the
settle rule); ADR-313 (the fold re-reads its cursor in each owed day's write); ADR-371 (the XP
crate, whose "More Information" names #639 for the reconciliation).
