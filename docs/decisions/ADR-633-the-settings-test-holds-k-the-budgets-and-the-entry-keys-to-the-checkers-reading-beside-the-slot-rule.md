---
status: "proposed"
decision-makers: "the DeckStreak architect"
---

# The settings test holds k, the budgets and the entry keys to the checker's reading beside the slot rule

Decides SPEC-611 (issue #802). Read at the development branch's commit `b065fce`.

## Context and Problem Statement

Issue #802, a follow-up to #703: `config/formal.json` states `k`, the budget fields and per-entry
budgets keyed `tla/<entry>`, and the settings test holds none of them to the reading the formal
checker takes. A settings file the checker refuses can therefore pass the test.

What the tree and the checker hold, measured:

- `config/formal.json` states `k` as 20 (line 2), three budgets, `budgets.tla_seconds`,
  `budgets.lean_seconds` and `budgets.entry_seconds` (lines 4 to 6), and twelve per-entry budgets
  under `budgets.entries` (lines 8 to 19), each keyed `tla/`.
- The test's field table marks `k` and each budget field a positive integer, and `budgets.entries`
  a map of positive integers (`scripts/tests/test_formal_config.py` lines 53 to 57). Its integer
  rule (lines 127 and 128) takes any integer of at least 1, and the test reads no per-entry key.
- The formal checker, outside this repository, reads `k` as an unsigned 32-bit integer, and each
  budget and each per-entry budget as an unsigned 64-bit integer, each at least 1; it reads a
  per-entry key only when it starts `tla/` or `kani/`; and any other value refuses the whole file.
  So a `k` of 4294967296, a budget of 18446744073709551616 and a per-entry key `OpLog` each pass
  the test, and the checker refuses a settings file holding any of them.
- SPEC-404 held `tlc_slot` to the checker's reading by its own rule, `slot_reading`, and left these
  fields open in its section 6 as the same class for other fields (#703).
- SPEC-295's presence harness runs every test of the `FormalConfig` class against each planted
  refusal of the committed file and, for a file that fails to load, requires that exactly the
  tests that load it fail (`scripts/tests/test_formal_config_presence.py` lines 177 and 294); its
  rule for each call of the reader scans the whole module.

## Decision Drivers

- The issue's three lines: a test holds each value to the checker's reading the way the slot test
  holds `tlc_slot`, a per-entry key without the `tla/` prefix is refused, and SPEC-404's exclusion
  cites #802.
- The reader's planted faults and its presence harness are generated from its kinds, and a change
  to the kinds rewrites those populations.
- A red-first test must fail for its own criterion's reason, and the harness must stay green for
  its own.

## Decision Outcome

- **D1. One rule beside the slot rule.** A rule `bounded_reading` stands beside `slot_reading`, and
  `read` gains no wider kinds: the reader's planted faults and its presence harness are generated
  from its kinds, while the bounds are the formal checker's per-field widths, the split SPEC-404
  made for `tlc_slot`. The rule takes its fields from the field table by kind (`posint` and
  `posint-map`) and passes over `tlc_slot`, so a budget the table gains is bounded with no edit
  here.
- **D2. The prefix.** A per-entry key must start `tla/`, as #802 asks. The formal checker also
  reads `kani/` keys, and the file states no Kani entry budget, so this rule refuses them; a Kani
  entry budget widens the prefix in its own change, with its own test.
- **D3. The values are spelled.** The bounds, and the first integer past each, are spelled in the
  tests, never derived from the rule.
- **D4. Where the tests stand.** A1 and A2 stand in a class of their own, `BoundedReading`, before
  `FormalConfig`, and A3 stands beside the slot tests in `FormalConfig`. A1 and A2 build their
  documents from the expected table and read no committed file, while the presence harness runs
  every `FormalConfig` test against each refused file and, for a file that fails to load, requires
  that exactly the tests that load it fail; inside `FormalConfig`, their red-first stub would fail
  them under every plant and turn the harness red at the red commit for a reason that is not its
  own. The harness's check of every call of the reader still covers both, and A3, which loads the
  file, stays in its run.

## Alternatives, and what each decision was chosen against

- D1, chosen: (a) the separate rule beside the slot rule, because the bounds are the checker's per-field widths and the rule reads its fields from the field table by kind.
- D1, (b) per-field bounds inside `read` and its planted faults: rejected, because it rewrites generated populations and their presence harness for a property only some fields have.
- D1, (c) leave the fields open: rejected, because #802 asks for the test, and a file the checker refuses would still pass it.
- D2, also admit `kani/` keys, which the formal checker also reads: rejected, because the file states no Kani entry budget; a Kani entry budget widens the prefix in its own change, with its own test.
- D3, derive the bounds and the first integer past each from the rule's constants: rejected, because a test whose values move with the code under test cannot see that code change.
- D4, (d) the three tests in `FormalConfig` with the harness admitted red at the red commit: rejected, because that red is not the harness's criterion, and it voids the rows the harness kills at that push.

## Consequences

- A settings file whose `k`, budget or per-entry budget lies past the checker's range, or whose
  per-entry key names no TLA+ entry, fails the settings test before any formal check reads it.
- SPEC-404's exclusion bullet cites #802; nothing else of SPEC-404, SPEC-295, ADR-295 or ADR-418
  changes.
- A Kani entry budget, when the file states one, is refused by this rule until its own change
  widens the prefix.

### Confirmation

SPEC-611 A1 and A2, red first and then green; A3 and A4 green throughout; and the rows from
`S61100` killed by their tests.

## What would make this wrong

- The checker's integer widths changing: the bounds spelled in the tests would no longer be the
  reader's, and the rule would refuse a value the checker reads or admit one it refuses.
- The checker reading another per-entry prefix the file comes to use: the rule would refuse a file
  the checker reads.
- The presence harness running every class of the module: A1 and A2 would then be run against each
  refused file, and D4's placement would need review.

## More Information

Issue #802; issue #703; SPEC-404 section 6; SPEC-295; ADR-418; SPEC-611; the schematic
`docs/schematics/formal-bounded-settings.md`.
