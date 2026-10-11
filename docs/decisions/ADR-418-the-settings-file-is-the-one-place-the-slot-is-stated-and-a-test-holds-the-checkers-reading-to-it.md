---
status: accepted
decision-makers: "the DeckStreak architect"
---

# The settings file is the one place the model checker's slot is stated, and a test holds the checker's reading to it

Decides SPEC-404 (issue #703). Read at the development branch's commit `32f6217`.

## Context and Problem Statement

Issue #703: a local formal check of `formal/tla/EveryLegCounted` stopped before the model ran and
read `VOID CONFIG`. The checker compared `tlc_slot` in `config/formal.json` with a value built into
it, the two disagreed, and no property was checked. The model, `MCEveryLegCounted.cfg` and its two
witnesses are not shown wrong.

What the tree and the checker hold, measured:

- `config/formal.json` is the one file that writes the setting: `tlc_slot` names `capacity` and
  `wait_seconds` (lines 26 to 29), the values SPEC-295 R7 set.
- SPEC-295 R7 (lines 229 to 231) and ADR-295's slot addendum (lines 112 to 129, #516) chose that
  capacity because it equalled a value compiled into the checker, which the checker of that time
  compared the file with and refused on any difference.
- The formal checker's settings reader, as it now stands, takes `tlc_slot` from the file as its
  setting. A key the file names is read as the file's value. A key the file omits is read from a
  default compiled into the checker. A value that is not an object, not a positive integer, or a
  capacity past an unsigned 32-bit count is refused whole. That reader holds no comparison of the
  file's capacity with a compiled one, so its compiled values are a second place only for a key the
  file omits.
- This repository's test reads the file through its own reader, which marks both slot keys optional
  and admits any positive integer (`scripts/tests/test_formal_config.py` lines 58 and 59, 125 and
  126). It therefore admits a file that omits a key, whose reading would then come from the
  checker's compiled default, and a capacity of 4294967296 or a wait of 18446744073709551616, each of
  which the checker's reader refuses. Its expected table (line 45) is a copy of the file, not of the
  checker.
- The formal check is a local check: no workflow under `.github/workflows/` names it. The
  settings test runs in the `python` stage of the `hygiene` job (`.github/workflows/ci.yml` line
  351; `scripts/check.sh` lines 178 to 195).

## Decision Drivers

- The issue asks for agreement in one place and for a test that fails when the file and the checker
  drift apart.
- A copy of a number that lives in another tree cannot see that tree change.
- A delivery never changes `k`, the axioms or the signers path, and a mutation row never leaves
  while its target stays without a signed ruling (SPEC-295 R4 and R5).

## Decision Outcome

- **D1. The population.** The slot setting has one writer, `config/formal.json`, and these readers:
  the formal checker's settings reader and the defaults compiled beside it; this repository's
  expected table, field table and reader in `scripts/tests/test_formal_config.py`; the row `S29530`
  (`scripts/mutation-rows.d/S29500-S29599.json`, anchored on the settings file's capacity line); and
  the prose of SPEC-295 R7, ADR-295's slot addendum and `docs/red-first/SPEC-295.md` (lines 172 to
  181). They disagree in three places: the prose names the value as the checker's compiled one,
  which the current reader no longer compares; the test admits an omitted key, which hands the
  reading to the checker's compiled default; and the test admits a value past the reader's range,
  which the checker refuses. From the current reader's source, a file that names both keys within
  range is read as the file's own value, so the issue's `VOID CONFIG` does not reproduce there; it
  reproduces only under a checker that still compares the capacity with a compiled one that differs.
  Chosen against a population drawn from the issue's text alone, because that names the file and
  the checker and misses the test's optional keys and open range, the prose and the row that cite
  the compiled value.
- **D2. One place.** `config/formal.json` is the one place this repository states the slot setting,
  and the formal checker reads it there. The test holds the file to the checker's reading: the file
  names every key the checker reads under `tlc_slot`, so no compiled default is ever the reading,
  and each value lies inside the range the checker's reader takes, so the checker never refuses it.
  Two red-first tests in `scripts/tests/test_formal_config.py` hold it (SPEC-404 A1 and A2), run
  locally for their red and by the `hygiene` job's `python` stage in CI, and mutation rows from
  `S40400` prove each arm. SPEC-295 R7's value stands; its reason, "equal to the formal checker's
  own compiled setting", is superseded by this decision, and the row `S29530` keeps its id and its
  anchor. Chosen against a second copy of the checker's number, deleting the setting, and changing
  nothing (each with its reason below).
- **D3. The local re-run.** The builder runs the EveryLegCounted check locally with the checker
  paired to this tree, at the delivery's committed head, and records its lines in
  `docs/red-first/SPEC-404.md`: each of `PassMeansEveryLegCounted` and `PassMeansThePartition`
  clean, and no `WITNESS_SURVIVED`, `VOID`, `TIMEOUT` or `MODEL_ERROR` line, so both witnesses were
  caught. Chosen against a reading in CI by name and the reviewer's reading from the issue (below).
- **D4. FORMAL, decided by surface.** No model, configuration, witness or proof changes, and
  `config/formal.json` is unchanged. The entry's register lines (`formal/tla/EveryLegCounted/
  EveryLegCounted.tla` lines 68 to 77) do not move, so neither does its streak. The rows the
  delivery adds are additions, so the surfaces ratchet reads no removed row. Chosen against a model
  of the slot's turns and a machine-checked proof of the slot reading (below).
- **D5. Drift and the push count.** No open pull request changes a file this delivery changes;
  #783 adds files under `formal/lean/` and #770 changes `docs/schematics/mutation-testing.md`, and
  neither is in the manifest. The builder re-measures at its cut. The red is read locally, so the
  delivery is pushed once. Chosen against pushing the red commit alone first (below).

## Alternatives, and what each decision was chosen against

- D1, a population drawn from the issue's text alone: rejected, because it names the file and the checker and misses the test's optional keys and open range, the prose and the row that cite the compiled value.
- D2, pin the checker's number in a second copy in this repository: rejected, because a copy cannot see the checker change, which is how SPEC-295 R7's value came to be read as `VOID CONFIG` by a checker built with another value; a test comparing two copies of this repository's own number only compares the file with itself.
- D2, delete the setting so the checker's compiled default rules: rejected, because it changes the slot capacity this repository's checks run at, a checker that compares the capacity reads an omitted key as its absent default and refuses it (ADR-295's slot addendum), and the row `S29530` would lose its anchor while its target stays, a weakening that needs a signed ruling.
- D2, change nothing because the checker now reads the file: rejected, because it leaves no test that fails when the file and the checker drift apart, leaves the test admitting an omitted key and values the checker refuses, and leaves SPEC-295 R7 naming a reason the current reader no longer has.
- D2, chosen: the file is the one place, and a test holds it to the checker's reading rule, because the checker reads the file as its setting, so the file naming every key within the reader's range makes the checker's reading the file's own value with no compiled value in between.
- D2, mirror the whole of the checker's reader for every field in this delivery: rejected, because the issue is the slot setting; the same range rule for `k` and the budget fields is recorded in SPEC-404's exclusions under #703.
- D3, take the EveryLegCounted reading in CI by name: rejected, because this repository's CI has no formal job, so no check-run carries that name.
- D3, cite the reviewer's reading from the issue: rejected, because that reading was `VOID CONFIG`, which is not a pass and checked no property.
- D4, write a model of the slot's turns under `formal/tla/` here: rejected, because the slot is the checker's own protocol outside this tree, and this delivery changes neither the slot's value nor any actor over it.
- D4, write a machine-checked proof of the slot reading under `formal/lean/`: rejected, because the reading is a test-side check over one JSON document, held by planted documents and mutation rows.
- D5, push the red commit alone first: rejected, because the settings test runs locally with no deploy script and no subprocess, so its red is read before any push.

## Consequences

- The slot setting has one statement in this repository, and the test refuses the two ways the
  checker's reading could leave it: an omitted key and a value past the reader's range.
- A checker that still compares the capacity with a compiled value is observed only by a local
  check, which this repository's CI does not run; SPEC-404 records the reading at the head.
- The row `S29530` keeps its id, whose words name the superseded reason; its meaning is read from
  SPEC-404.

### Confirmation

SPEC-404 A1 and A2, red first and then green; the rows from `S40400` killed by their tests; and the
local EveryLegCounted reading recorded in `docs/red-first/SPEC-404.md`.

## What would make this wrong

- The checker's reader comparing the file's capacity with a compiled value again: the file would
  then be refused at any other value, and agreement would again live in the checker. The local
  check would read `VOID CONFIG`, and this decision would be reopened.
- The checker reading a third key under `tlc_slot`: the test's key set would no longer be the
  reader's, and the test would need the key.
- The reader's integer widths changing: the range bounds in the test would no longer be the
  reader's.

## More Information

Issue #703; issue #516 and ADR-295's slot addendum; SPEC-295 R1, R7, A9 and A10; SPEC-404.

## Supersession note for ADR-295

ADR-295 gains an insert-only last section, "Addendum: the slot capacity's reason is superseded by
ADR-418 (#703)", saying that the capacity's value stands and that its reason is now this ADR's D2.
