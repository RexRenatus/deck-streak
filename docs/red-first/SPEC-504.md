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

## Fix 1: the causal claim read from the state, and the device crash checked

A frozen population of 30 hand mutants of the model's actions and guards, each run by hand over the committed
configurations in order and stopped at its first violation, left two uncaught at b4d4a4ae. One reads the least clock in
the device's pulled copy as the clock it saw. CausalHlc's first half compares an operation's clock with its own `saw`,
which the step that makes it writes from that same reading, so the property agreed with the wrong clock and all four
configurations read clean. The other drops the device's queue when the device crashes; every configuration took no
device crash, so the crash step never ran. The fix adds CausalHlcAtMake, the claim read from the state (25ca2bac), and
`MCDeviceCrash.cfg`, the main configuration's constants with one crash per device (24f96dc4). CausalHlc is unchanged.

Red first, by hand with the pinned model checker and the check's flags, each on a scratch copy of the fixed model with
one mutant applied, before either commit:

- the least-clock reading: CausalHlcAtMake VIOLATION at the main configuration's constants, the new property alone,
  1,734 distinct states;
- the crash that drops the queue: NoLostReview VIOLATION at `MCDeviceCrash.cfg`, 10 distinct states.

The new witness, a clock past only the oldest clock it saw, switches that least-clock reading on and reads VIOLATION of
CausalHlcAtMake at 1,734 states; each of the 16 witnesses reads VIOLATION of the property it names (16 of 16). The same
switch checked by CausalHlc alone at the main configuration's constants reads no error at 111,862 states: the old
property agrees with the wrong clock. Green, the fixed model at each configuration, every property in one run:

| configuration | distinct states | result |
|---|---|---|
| `MCOpLog.cfg` | 140,340 | no error, 37 s |
| `MCOpLogEdits.cfg` | 12,727 | no error |
| `MCOpLogThree.cfg` | 100,098 | no error, 39 s |
| `MCLiveness.cfg` | 6,974 | no error (Convergence) |
| `MCDeviceCrash.cfg` | 767,800 | no error, 93 s (NoLostReview, ExactlyOnce, CausalHlcAtMake) |

The population, re-applied to the fixed model and run over its five configurations in order: examined 30, caught 30,
escaped 0. The least-clock member is caught by CausalHlcAtMake at `MCOpLog.cfg` (1,734 states), the queue-dropping
crash by NoLostReview at `MCDeviceCrash.cfg` (10 states), and the other 28 at the configurations that caught them
before.

The ledger test, on an extract of b4d4a4ae's tree with only the fixed model in it: `Ran 9 tests`,
`FAILED (failures=1)`, `test_the_rows_hold_o1_to_o17_and_the_planned_entries`: the header declares CausalHlcAtMake and
the ledger does not name it. With the four new rows, one under each obligation that names the entry: `Ran 9 tests`,
`OK`, `examined 145 present artifacts`.

The budget: at 24f96dc4 and 600 s the entry's check spent its budget, 12 of 15 properties clean and six `TIMEOUT` (one
run past its budget, five not started). With a provisional budget on a scratch copy it took 631 s, every property
`clean: true` and no finding, so the entry took 960 s (606720ee); the check at 960 s then took 670 s, clean, so the
entry took 1020 s (71b6bfb4); the check at 1020 s took 839 s on a loaded box (load average near 30), clean, so the
entry takes 1260 s, the worst measurement's (2cd33c7a). The settings test read `FAILED (failures=1)` before each value
moved and `OK` after. At 71b6bfb4 the entry's check (`--kind tla --gate --entry tla/OpLog`, base `d3b09d5e`) read
`FORMAL OK` in 839 s: 15 properties, 16 witnesses and 5 configurations, each property `clean: true` at
`ramp=report`, with no finding. The last commit only raises the entry's budget, so every run that check started still
starts.
