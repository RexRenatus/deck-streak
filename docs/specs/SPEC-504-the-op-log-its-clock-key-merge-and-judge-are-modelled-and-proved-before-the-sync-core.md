# SPEC-504: The op log, its clock, its key, its merge and its judge are modelled and proved before the sync core

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | #15 (the sync engine), until the pull request names its own |
| decided by | ADR-522 |
| measured at | `d3b09d5e` |
| schematic | `docs/schematics/the-op-log-model-and-its-four-proofs.md` (new) |
| mutation band | S50400-S50499 (no row: section 6) |

## 1. The problem, measured

DeckStreak's sync is to become one server-ordered log of operations that every device pushes to and pulls from. The
code that will run it, the sync core, is not in the tree: `git ls-tree -d --name-only d3b09d5e crates/ | grep -c -x
crates/sync` prints 0. The formal ledger (SPEC-496) names the protocol's model and its four proofs as planned:
`grep -c -E '"entry": "(tla/OpLog|lean/(SyncClock|ReviewKey|FieldMerge|PushJudge))", "state": "planned"'
formal/obligations.json` prints 12 at `d3b09d5e`, under obligations O1, O2, O3, O11 and O16. A protocol modelled after
its code is modelled against what was built; this delivery writes the model and the proofs first, so the sync core is
built against them.

## 2. Requirements

R1. `formal/tla/OpLog` models the op log: devices that make reviews, edits and undos of their own writes, push them,
    read verdicts and pull by cursor; a judge keyed by (origin, seq) that reads the device's token; a writer that
    commits at the next position and records the loser of a concurrent edit; device and server crashes, a base
    replace and a token revocation. It declares 14 properties, each `ramp=report`: NoLostReview, ExactlyOnce,
    MonotoneLog, NoSkip, Convergence (liveness), CanonicalCardFold, NoSilentOverwrite, FieldConverges, CausalHlc,
    RebootstrapKeepsPending, RevokedDeviceRefused, RetractOnlyOwn, LocalIsPrefix and UndoOwnOnly.
R2. Each property has a witness configuration with one recorded defect switched on, and each witness is caught: 15
    witnesses, among them the 10 the design names (an answer written before its commit, a pending operation dropped on
    send, a judge with no (origin, seq) key, a position taken outside the writer's transaction, a field won by arrival
    order, a loser not recorded, a card folded in server order, a clock that ignores what it saw, a replace that drops
    pending, a revoked token that still pushes).
R3. Four configurations: `MCOpLog.cfg` (2 devices, 2 operations each, 1 server crash, MaxHlc 4, 1 note),
    `MCOpLogThree.cfg` (3 devices, 1 operation each, a report), `MCOpLogEdits.cfg` (2 devices, 2 fields, values
    {a, b, c}, edits and an undo) and `MCLiveness.cfg` (Convergence under weak fairness of push, commit, verdict and
    pull, at most one lost answer per operation). No configuration names SYMMETRY, CONSTRAINT, VIEW or
    CHECK_DEADLOCK.
R4. Two vacuity traps, ReachConflict (a conflict record is reached) and ReachLateReview (a review that sorts before a
    pulled one arrives), are each violated by the main configuration, and a SANY parse of the module with one planted
    syntax error is refused. Both are scratch controls and are never committed.
R5. `formal/lean/Formal/SyncClock.lean` proves `tick_gt_seen`, `tick_ge_pt`, `run_strict_mono` and `causal`;
    `ReviewKey.lean` proves `fold_sorted_perm_invariant`; `FieldMerge.lean` proves `merge_comm`, `merge_assoc`,
    `merge_idem` and `apply_perm_invariant`; `PushJudge.lean` proves `judge_idempotent` and `no_review_mutation`.
    Each theorem has a witness theorem that refutes the claim for a named defect at a concrete input.
R6. Each Lean entry declares a vectors file, `formal/vectors/{sync-clock,review-key,field-merge,push-judge}.jsonl`,
    written by its arm of `Formal/Vectors.lean` and never edited by hand. No entry covers code: none exists.
R7. Each theorem's planted control, the claim stated of its defect and proved by the real theorem's own proof, is
    refused by Lean at that control's own line; `#print axioms` of every theorem and witness lists at most `propext`,
    `Classical.choice` and `Quot.sound`. Both are scratch controls.
R8. The ledger records the five entries as present, each named property under every obligation that named the entry,
    and gives each entry its row (`re-stamp`: stamped when the sync core lands); the ledger's tests hold the 48
    entries.

## 3. Acceptance criteria of SPEC-504

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1-R3: every OpLog property clean at its configurations, every witness caught | each witness VIOLATION | `phxd formal check --root . --base <base> --kind tla --gate --format json` (a local check, outside the tree) |
| A2 | R5, R6: every Lean theorem built, every witness built, vectors byte-equal, no hole | each witness theorem | `phxd formal check --root . --base <base> --kind lean --gate --format json` (local) |
| A3 | R4, R7: the vacuity traps violated, the SANY control and each planted control refused, the axiom audit within the three | the planted controls | the red-first record's scratch runs |
| A4 | R8: the ledger names the five entries present and their rows | the ledger test before the entries exist | the fence below |

```acceptance
A4: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_the_rows_hold_o1_to_o17_and_the_planned_entries
A4: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_every_existing_entry_has_its_disposition_row
A4: python3 -m unittest discover -s scripts/tests -p test_formal_obligations.py -k test_the_ledger_exists_in_schema_v1
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `formal/tla/OpLog/OpLog.tla` | formal | new: the model |
| `formal/tla/OpLog/MCOpLog.cfg`, `MCOpLogThree.cfg`, `MCOpLogEdits.cfg`, `MCLiveness.cfg` | formal | new |
| `formal/tla/OpLog/witness/*.cfg` | formal | new: 15 witnesses |
| `formal/lean/Formal/SyncOrder.lean` | formal | new: the key and edit orders (support) |
| `formal/lean/Formal/SyncClock.lean`, `ReviewKey.lean`, `FieldMerge.lean`, `PushJudge.lean` | formal | new |
| `formal/lean/Formal/SyncVectors.lean` | formal | new: the four vector writers (support) |
| `formal/lean/Formal/Vectors.lean` | formal | changed: one import and four arms |
| `formal/vectors/sync-clock.jsonl`, `review-key.jsonl`, `field-merge.jsonl`, `push-judge.jsonl` | formal | new |
| `formal/obligations.json` | formal | changed: R8 |
| `scripts/tests/test_formal_obligations.py` | guard tests | changed: the 48 entries |
| `config/formal.json`, `scripts/tests/test_formal_config.py` | formal | changed only if the entry's measured time needs a budget of its own |
| `docs/specs/SPEC-504-...md`, `docs/decisions/ADR-522-...md` | docs | new |
| `docs/schematics/the-op-log-model-and-its-four-proofs.md` | docs | new |
| `docs/red-first/SPEC-504.md` | docs | new: the red-first record |
| `changelog.d/formal-ahead-code-oplog-504.md` | docs | new |

## 5. What this does NOT do

- No sync code, and no `covers` line: the sync core lands later (#15) and stamps each entry when it does.
- No Rust test reads the vectors yet: `crates/sync/tests/formal_vectors_<name>.rs` needs the sync core's crate.
- No property is promoted: each enters at `ramp=report`.

## 6. Risks

- The model is checked at small bounds, not universally; the ledger's gap lines say so. The main configuration takes
  no device crash, because the per-property runs at one crash per device do not fit the entry's budget (the red-first
  record has the measurement); a device crash here loses only unread answers, which the server crash also loses.
- The proofs prove the ports; until the sync core's tests read the vectors byte-equal, nothing ties them to code.
- Mutation band: no row. The delivery changes no Rust or Python source; its one Python change is a test.
