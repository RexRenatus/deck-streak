# SPEC-404: the slot setting is stated once in the settings file, and a test holds the checker's reading to it

- **Issue:** #703. **Context(s):** `repo` (`scripts/tests/` and `docs/`).
- **Decided by:** ADR-418 (this SPEC's own: the settings file is the one place the slot is stated,
  and what that was chosen against).
- **Status:** built. It holds `docs/red-first/SPEC-404.md`.

## 1. The problem, measured

Read at the development branch's commit `32f6217`, each file by `git show 32f6217:<path>`.

- **The check stopped before the model ran.** A local formal check of `formal/tla/EveryLegCounted`
  read `VOID CONFIG` and checked no property: the checker compared `tlc_slot` in
  `config/formal.json` with a value compiled into it, and the two differed (#703).
- **One file writes the setting.** A search for `tlc_slot` at `32f6217` lists six files:
  `CHANGELOG.md`, `config/formal.json`, ADR-295, `docs/red-first/SPEC-295.md`, SPEC-295 and
  `scripts/tests/test_formal_config.py`. Only the settings file writes it (lines 26 to 29).
- **The prose names a compiled value as the reason.** SPEC-295 R7 (lines 229 to 231) and ADR-295's
  slot addendum (lines 112 to 129) set the capacity "equal to the formal checker's own compiled
  setting", because the checker of that time refused any other (#516). The checker's settings
  reader as it now stands reads `tlc_slot` from the file as its setting and compares it with no
  compiled value; its compiled values are read only for a key the file omits.
- **The test admits what the checker would not read from the file.** The test's reader marks both
  slot keys optional (`scripts/tests/test_formal_config.py` lines 58 and 59) and admits any positive
  integer (lines 125 and 126). A file without `tlc_slot.capacity` passes it, and the checker would
  then read its compiled default; a capacity of 4294967296 and a wait of 18446744073709551616 pass
  it too, and the checker's reader refuses each as past its unsigned 32-bit and 64-bit ranges.
- **The formal check is a local check.** A search of `.github/workflows/` at `32f6217` for `formal`
  finds no line. The settings test runs in the `hygiene` job's `python` stage
  (`.github/workflows/ci.yml` line 351; `scripts/check.sh` lines 178 to 195).

## 2. Requirements

- **R1.** `config/formal.json` is the one place this repository states the model checker's slot
  setting. It names both keys the formal checker reads under `tlc_slot`, `capacity` and
  `wait_seconds`, so the checker reads the file's own values and never a default compiled into it.
  The values are those SPEC-295 R7 set, unchanged.
- **R2.** Each slot value the test admits is one the formal checker's reader takes: `capacity` a
  positive integer no greater than 4294967295, and `wait_seconds` a positive integer no greater
  than 18446744073709551615. The first integer past each bound is refused, and each bound is
  admitted.
- **R3.** SPEC-295 R7's value, A9 and A10 stand, and the row `S29530` keeps its id and its anchor.
  R7's reason, "equal to the formal checker's own compiled setting", is superseded by ADR-418 D2,
  and SPEC-295 and ADR-295 each gain an insert-only last section that says so.
- **R4.** The EveryLegCounted entry's local check at this delivery's committed head reads clean for
  `PassMeansEveryLegCounted` and `PassMeansThePartition`, with both witnesses caught, and the
  reading is recorded as section 4 says. No model, configuration, witness or register line changes.
- **R5.** The new tests keep SPEC-295's presence harness whole: each call of the test's reader in
  them is inside a handler of its refusal, a test that loads the committed file fails by assertion
  under every planted refusal of that file, and no planted refusal makes either new test error.

## 3. Acceptance criteria of SPEC-404

| id | criterion | decided by |
|---|---|---|
| A1 | the committed file's slot reading is its own two values, read from the loaded file and never from the expected table (the presence control); the keys the rule judges are exactly the keys the reader's field table lists under `tlc_slot`, spelled in the test as `capacity` and `wait_seconds`; a document without `tlc_slot`, one without `tlc_slot.capacity` and one without `tlc_slot.wait_seconds`, each admitted by the test's reader, is each refused by the rule's own arm, the refusal naming exactly what the file does not name (`tlc_slot`, `tlc_slot.capacity`, `tlc_slot.wait_seconds`); the test prints how many documents it examined and refuses zero | `test_formal_config.py` `the_checker_reads_the_slot_from_the_file_alone` |
| A2 | for each slot key, the document holding its bound (4294967295 for `capacity`, 18446744073709551615 for `wait_seconds`) is admitted and read as that value, and the document holding the first integer past it is refused by the rule's range arm naming the key; every value is spelled in the test, never derived from the rule; the test prints how many documents it examined and refuses zero | `test_formal_config.py` `each_slot_value_is_one_the_checkers_reader_takes` |
| A3 | SPEC-295's presence harness holds with the new tests counted: every call of the reader is guarded, and every planted refusal of the committed file fails each test that loads it by assertion and makes none error | `test_formal_config_presence.py` `PresenceControls` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k the_checker_reads_the_slot_from_the_file_alone
A2: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k each_slot_value_is_one_the_checkers_reader_takes
A3: python3 -m unittest discover -s scripts/tests -p 'test_formal_config*.py' -k PresenceControls
```

R1 and R2 are decided by A1 and A2, R5 by A3. R3 is a rule of the documents, read in review. R4 is
decided by the local formal check of section 4, which no repository test runs.

## 4. The local formal reading, decided outside the test suite

- **Who and when.** The builder, at the delivery's committed head after the rows are committed,
  because the check reads the committed tree; then the verifier, and the check at landing.
- **What runs.** The paired checker's `formal check --entry tla/EveryLegCounted`, with the
  development branch's live commit as its base, and its ratchet-only check at the same head.
- **The record.** A section of `docs/red-first/SPEC-404.md` quoting the check's verdict line, each
  property row's `clean` and `ramp`, and the absence of `WITNESS_SURVIVED`, `VOID`, `TIMEOUT` and
  `MODEL_ERROR` lines. A `STALE` line is quoted, and it is the base's when a check of the base
  reports it too. A `VOID CONFIG` line is a stop, never a reading.

## 5. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_formal_config.py` | repo | changed: the slot rule, A1 and A2 |
| `docs/specs/SPEC-404-the-slot-setting-is-stated-once-in-the-settings-file-and-a-test-holds-the-checkers-reading-to-it.md` | repo | added |
| `docs/decisions/ADR-418-the-settings-file-is-the-one-place-the-slot-is-stated-and-a-test-holds-the-checkers-reading-to-it.md` | repo | added |
| `docs/schematics/formal-check-settings.md` | repo | added: the settings' data flow, agree and disagree |
| `docs/specs/SPEC-295-the-repository-declares-its-formal-check-settings.md` | repo | changed: an insert-only last section (R3) |
| `docs/decisions/ADR-295-the-repository-declares-its-formal-check-settings.md` | repo | changed: an insert-only last section (R3) |
| `docs/red-first/SPEC-404.md` | repo | added |
| `scripts/mutation-rows.d/S40400-S40499.json` | repo | added: the rows of section 8 |
| `changelog.d/formal-slot-agree-404.md` | repo | added |

## 6. What this does NOT cover

- It changes no value in `config/formal.json`: the slot setting stands as SPEC-295 R7 set it
  (#703).
- It changes no model, configuration, witness or register line under `formal/`, and adds no formal
  entry (#703).
- It changes neither the formal checker nor how a checker is paired with this tree; the checker is
  not in this repository (#703).
- It adds no CI job that runs the formal check, so the EveryLegCounted reading stays a local one
  (#703).
- It does not hold `k`, the budget fields or the per-entry budget names to the checker's reader the
  same way; that is the same class for fields other than `tlc_slot` (#703).
- It retires and renames no mutation row: `S29530` keeps its id, whose words name the superseded
  reason (#516).
- It changes no Rust, no web surface, no workflow and no existing test (#703).

## 7. Risks

- **A checker that compares again.** A checker whose reader compares the capacity with a compiled
  value refuses the file at any other value. Only the local check sees it, and it reads
  `VOID CONFIG`; ADR-418 names this as what would make it wrong.
- **The reader changes shape.** A third key under `tlc_slot`, or another integer width, makes the
  test's literal key set or bounds no longer the reader's. Nothing in this repository sees that
  change before the local check refuses the file or reads a default.
- **The presence harness runs every settings test under each planted refusal.** A new test that
  errors where it should fail reddens A3, which is why R5 is stated.
- **Anchors.** The new code repeats no line an existing row of `S29500` to `S29530` anchors on, and
  the builder's anchor census proves each anchor occurs exactly once.

## 8. The mutation rows

Band `S40400` to `S40499`, `scripts/mutation-rows.d/S40400-S40499.json`, target
`scripts/tests/test_formal_config.py`, each killed by A1's test or A2's test alone. One row per arm,
key and bound: an omitted `tlc_slot` read as a value; an omitted key passed over; only the first key
judged; the capacity's bound widened to the wait's width; the wait's bound widened; the bound itself
refused; the first integer past the bound admitted; the reading taken from the bound instead of the
file. The record commit lists each row's id, its find, its replacement and its killer here.

## 9. References

Issue #703; issue #516; SPEC-295 R1, R7, A9 and A10; ADR-295's slot addendum; ADR-418; the
schematic `docs/schematics/formal-check-settings.md`.
