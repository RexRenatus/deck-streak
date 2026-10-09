# Red-first record: SPEC-389

The vectors test, `crates/progression/tests/formal_vectors_xp_reconciliation.rs`, is committed
after the model it reads and before the proof. `645ec631` (M) commits the Lean entry
`lean/XpReconciliation` with a port of `settle` whose every case answers the request, the rule
ADR-072 replaced: it has no closed-day arm, its theorems are not yet stated, and its writer wrote
`formal/vectors/xp-reconciliation.jsonl` from that port. The commit that adds this record (R) adds
the test and changes nothing else, so the test ran against M's vectors and the shipping `settle`,
and it is red by assertion, not by a compile error. The green commit (G) gives the port the arm of
`settle.rs:117-122`, proves T1 to T4 and writes the vectors again. The test's text is the same at R
and at G, and no Rust source changes.

- **A1, red.** At M the test fails on the first settle vector the shipping rule answers otherwise:
  a closed held 0 under an open Recompute request of 0. The port answers it open, `[0,false]`, and
  `settle` holds it closed: `left: (0, true)`, `right: (0, false)`.
- **A2, red.** At M the test fails at trace `GSRN`'s `score90` close, step 4: the vector holds 0,
  closed, and `settle` holds the 200 that step 3 settled open, now closed: `left: (200, true)`,
  `right: (0, true)`.
- **A3, not red.** The shipping `settle` already holds the larger amount, closed, on both witness
  inputs, so the test guards the two cases T1 and T2 state.
- **A4, not red.** The axes already held every case at M. Its red is a population lacking one
  case: with trace `GSRN`'s line removed from the vectors file for one uncommitted run, it fails at
  `formal_vectors_xp_reconciliation.rs:390` with `GSRN occurs once`, `left: 0`, `right: 1`. The
  file was restored after the run and its sha256 read equal to M's.

```red-first
A1: red at 645ec631: thread 'the_settle_rule_answers_every_lean_vector' (2620416) panicked at crates/progression/tests/formal_vectors_xp_reconciliation.rs:243:9: left: (0, true), right: (0, false)
A2: red at 645ec631: thread 'every_trace_confirms_at_least_the_xp_shown' (2620666) panicked at crates/progression/tests/formal_vectors_xp_reconciliation.rs:207:9: settle [4,"score90",0,true,0,true] of GSRN, left: (200, true), right: (0, true)
A3: not red: the shipping settle already answers the witnesses' inputs as T1 and T2 state; it guards them
A4: not red: the axes already held every case at M; a population lacking one case is the scratch control
```
