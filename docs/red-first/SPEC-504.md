# Red-first record: SPEC-504

The SPEC, ADR-522 and the schematic were installed first (d295d857). The ledger's test came next (d05aa53b): it holds
the 48 entries, the five sync entries present under every obligation that names them, before any of them exists.
The model, the four proofs, their vectors and the ledger's update came next (76c2ca48), with two header fixes the
formal checker's registry asked for (2a492ebc: no header key in the model's prose; 25fb795b: the bound lines name the
configurations' numeric constants), and the entry's budget last (c2c5c8fd). The ledger test file is byte-identical at
d05aa53b and at the head.

## The ledger test (A4)

Red, on an extract of d05aa53b's tree (`git archive d05aa53b scripts formal config docs crates`), the test file alone:
`Ran 9 tests`, `FAILED (failures=2)`, both assertions:

- `test_the_rows_hold_o1_to_o17_and_the_planned_entries`: the five entries are still planned in the ledger.
- `test_every_existing_entry_has_its_disposition_row`: the tree holds 43 entries, not the 48 designed.

Green, on the head's tree: `Ran 9 tests`, `OK`, with `examined 22 planned entries`, `examined 29 planned harnesses`
and `examined 141 present artifacts`.

## The model (A1, A3)

Each witness was run forward first, by hand with the pinned model checker and the check's flags, and each read
VIOLATION of the property it names (15 of 15). The four configurations then read clean. Hand runs at the head:

| configuration | distinct states | result |
|---|---|---|
| `MCOpLog.cfg` | 140,340 | no error, every property in one run |
| `MCOpLogThree.cfg` | 100,098 | no error |
| `MCOpLogEdits.cfg` | 12,727 | no error |
| `MCLiveness.cfg` | 6,974 | no error (Convergence) |

The design's main bound with one crash per device and both tokens revocable measured 2,130,199 distinct states and
read no error in 434 s with every property in one run; with one crash per device and one revocable token, 767,800.
The committed main configuration takes no device crash (ADR-522, decision 4).

Vacuity: in a scratch copy, `ReachConflict` (conflicts = {}) and `ReachLateReview` (no review arrives after one that
sorts after it) were each violated by the main configuration. SANY: the committed module parses with no error, and a
copy with one planted unmatched parenthesis is refused with a parse error at that line. None of the three is committed.

The formal check of the entry alone at the default budget (25fb795b) checked 9 of the 14 properties clean and left
13 runs unstarted, each a `TIMEOUT` (`the entry's budget of 300s was spent`). With a provisional budget the entry's
check took 391 s and read every property `clean: true` with no finding, so the entry takes 600 s (c2c5c8fd). At
c2c5c8fd the check of every TLA+ entry (`--kind tla --gate`, base `d3b09d5e`) read `FORMAL OK` in 2646 s: 28
entries, 105 properties, 140 witnesses, 38 configurations; the 14 OpLog properties each `clean: true` at
`ramp=report`. Its only findings are 5 `STALE` on `tla/HabitXpFollowsItsLog`, whose cover
`crates/coordination/src/level_up.rs::announce_level_up` this delivery does not touch.

## The proofs (A2, A3)

Each witness theorem builds, refuting its claim for a named defect at a concrete input. Each theorem's planted
control, the claim stated of the defect and proved by the theorem's own proof, was refused by Lean inside its own
lines (11 of 11). `#print axioms` of the 11 theorems and 11 witnesses lists only `propext`, `Classical.choice` and
`Quot.sound` (22 of 22). The formal check of every Lean entry at c2c5c8fd read `FORMAL OK` in 30 s (56 theorems), the 11 new theorems each
`clean: true` at `ramp=report`, with no finding.
