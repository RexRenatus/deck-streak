# SPEC-611: k, the budgets and the per-entry budget keys are held to the formal checker's reading

- **Issue:** #802, a follow-up to #703. **Context(s):** `repo` (`scripts/tests/` and `docs/`).
- **Decided by:** ADR-633 (this SPEC's own: one bounded rule beside the slot rule, and what that
  was chosen against).
- **Status:** built. It holds `docs/red-first/SPEC-611.md`. **Mutation band:** `S61100-S61199`.

## 1. The problem, measured

Read at the development branch's commit `b065fce`, each file by `git show b065fce:<path>`.

- **The file states bounded values.** `config/formal.json` states `k` as 20 (line 2), three
  budgets, `budgets.tla_seconds`, `budgets.lean_seconds` and `budgets.entry_seconds` (lines 4 to
  6), and twelve per-entry budgets under `budgets.entries` (lines 8 to 19), each keyed `tla/`.
- **The test holds none of them to the checker's reading.** The test's field table marks `k` and
  each budget field a positive integer, and `budgets.entries` a map of positive integers
  (`scripts/tests/test_formal_config.py` lines 53 to 57). Its integer rule (lines 127 and 128)
  takes any integer of at least 1, and the test reads no per-entry key: a key of any spelling under
  `budgets.entries` is admitted when its value is a positive integer.
- **The formal checker refuses what the test admits.** The formal checker, outside this
  repository, reads `k` as an unsigned 32-bit integer, and each budget and each per-entry budget as
  an unsigned 64-bit integer, each at least 1; it reads a per-entry key only when it starts `tla/`
  or `kani/`; and any other value refuses the whole file. So a `k` of 4294967296, a budget of
  18446744073709551616 and a per-entry key `OpLog` each pass the test, and the checker refuses a
  settings file holding any of them (#802).
- **SPEC-404 left these fields open.** SPEC-404 held `tlc_slot` to the checker's reading and, in
  its section 6, left `k`, the budget fields and the per-entry budget names open as the same class
  for other fields (#703). The settings test runs in the `hygiene` job of CI, which runs
  `scripts/tests`.

## 2. Requirements

- **R1.** A rule beside the slot rule, `bounded_reading`, reads `k`, each budget and each per-entry
  budget, keyed by its dotted name, and reads every value within its range as stated. It refuses a
  `k` above 4294967295, and a budget or a per-entry budget above 18446744073709551615, by its range
  arm, the refusal reading `<dotted name> is past its range`. It takes its fields from the reader's
  field table by kind (`posint` and `posint-map`) and passes over `tlc_slot`, which the slot rule
  alone judges; `k` is bounded by the unsigned 32-bit range and every other field by the unsigned
  64-bit range.
- **R2.** The rule refuses a per-entry budget whose key does not start `tla/` by its entry arm, the
  refusal reading `budgets.entries.<key> names no TLA+ entry`.
- **R3.** The committed file's bounded reading is its own values, read from the loaded file and
  never from the expected table.
- **R4.** The new tests keep SPEC-295's presence harness whole: each call of the test's reader in
  them is inside a handler of its refusal that fails the test by assertion, the one new test that
  loads the committed file stands in the class the harness runs and fails by assertion under every
  planted refusal, and no planted refusal makes a new test error. The two tests that build their
  documents from the expected table, and read no committed file, stand in a class of their own
  before the settings tests. The reader, its field table, its expected table, the slot rule, every
  existing test and `config/formal.json` are unchanged.
- **R5.** SPEC-404's exclusion bullet for these fields cites #802 in place of #703, and no other
  byte of SPEC-404 changes.

## 3. Acceptance criteria of SPEC-611

| id | criterion | decided by |
|---|---|---|
| A1 | for `k`, each of the three budgets and one per-entry budget, the document holding the bound (4294967295 for `k`, 18446744073709551615 for each budget) is admitted by the test's reader and read by the rule as that value, and the document holding the first integer past it is refused by the rule's range arm naming the field; every value is spelled in the test, never derived from the rule; the test prints how many values it examined and refuses any it did not reach | `test_formal_config.py` `each_bounded_value_is_one_the_checkers_reader_takes` |
| A2 | a per-entry budget keyed `OpLog`, `lean/OpLog`, `TLA/OpLog` or `tla-OpLog`, its value within the range, is admitted by the test's reader and refused by the rule's entry arm naming the key; every key is spelled in the test; the test prints how many keys it examined and refuses any it did not reach | `test_formal_config.py` `a_budget_entry_key_that_names_no_tla_entry_is_refused` |
| A3 | the committed file's bounded reading is its own `k`, each of its budgets and each of its per-entry budgets, read from the loaded file and never from the expected table (the presence control); the rule admits every per-entry key the file names, and the file names at least one | `test_formal_config.py` `the_bounded_reading_is_the_files_own_values` |
| A4 | SPEC-295's presence harness holds with the new tests counted: every call of the reader is guarded, and every planted refusal of the committed file fails each test that loads it by assertion and makes none error | `test_formal_config_presence.py` `PresenceControls` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k each_bounded_value_is_one_the_checkers_reader_takes
A2: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k a_budget_entry_key_that_names_no_tla_entry_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k the_bounded_reading_is_the_files_own_values
A4: python3 -m unittest discover -s scripts/tests -p 'test_formal_config*.py' -k PresenceControls
```

R1 is decided by A1, R2 by A2, R3 by A3 and R4 by A4. R5 is a rule of the documents, read in review.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_formal_config.py` | repo | changed: the bounded rule beside the slot rule, two tests in a class of their own before the settings tests and one test beside the slot tests, inserts only (R1 to R4) |
| `docs/specs/SPEC-404-the-slot-setting-is-stated-once-in-the-settings-file-and-a-test-holds-the-checkers-reading-to-it.md` | repo | changed: one line, its exclusion bullet cites #802 in place of #703 (R5) |
| `scripts/mutation-rows.d/S61100-S61199.json` | repo | added: the eleven rows of section 7 |
| `docs/specs/SPEC-611-k-the-budgets-and-the-per-entry-budget-keys-are-held-to-the-formal-checkers-reading.md` | repo | added |
| `docs/decisions/ADR-633-the-settings-test-holds-k-the-budgets-and-the-entry-keys-to-the-checkers-reading-beside-the-slot-rule.md` | repo | added |
| `docs/schematics/formal-bounded-settings.md` | repo | added: the bounded values' data flow, agree and disagree |
| `docs/red-first/SPEC-611.md` | repo | added |
| `changelog.d/formal-bounded-settings-611.md` | repo | added |

## 5. What this does NOT cover

- It changes no value in `config/formal.json` (#802).
- It leaves the test's reader `read`, its field table, its expected table and the slot rule
  unchanged (#802).
- It changes no formal model, entry, witness or register line (#802).
- It leaves the formal checker unchanged; the checker is not in this repository (#802).
- It admits no per-entry prefix but `tla/`, and a Kani entry budget would widen the rule in its
  own change, with its own test (#802).
- It changes no CI job, no Rust, no web surface and no workflow (#802).

## 6. Risks

- **A checker whose reader takes another width.** The rule then disagrees with the formal checker,
  and nothing in this repository sees it; the bounds are re-measured against the checker's reader
  when it changes.
- **A field the reader's table gains.** A budget field of kind `posint` is bounded by the unsigned
  64-bit range through the field table with no edit here; a field of another width needs its own
  bound.
- **The presence harness runs every settings test under each planted refusal.** A test in
  `FormalConfig` that fails for a reason that is not the file's reddens A4, which is why R4 places
  the two tests that read no committed file in a class of their own.
- **Anchors.** The new code repeats no line an existing row on `scripts/tests/test_formal_config.py`
  anchors on, and the builder's anchor census proves each anchor occurs exactly once.

## 7. The mutation rows

Band `S61100` to `S61199`, `scripts/mutation-rows.d/S61100-S61199.json`, target
`scripts/tests/test_formal_config.py`. One row per bound, arm, boundary and reading: the 32-bit
bound of `k`, the 64-bit bound of a budget, the bound `k` takes, the bound admitted, the first
integer past the bound refused, a per-entry budget past its bound, the per-entry key refused, the
prefix itself, the reading of a field and of a per-entry budget taken from the file, and the slot
left to its own rule.

| id | find | replacement | killer |
|---|---|---|---|
| `S61100-K-IS-BOUNDED-BY-THE-READERS-32-BIT-WIDTH` | `K_BOUND = 2**32 - 1` | `K_BOUND = 2**64 - 1` | `test_formal_config.BoundedReading.test_each_bounded_value_is_one_the_checkers_reader_takes` |
| `S61101-A-BUDGET-IS-BOUNDED-BY-THE-READERS-64-BIT-WIDTH` | `BUDGET_BOUND = 2**64 - 1` | `BUDGET_BOUND = 2**65` | `test_formal_config.BoundedReading.test_each_bounded_value_is_one_the_checkers_reader_takes` |
| `S61102-K-TAKES-ITS-OWN-BOUND` | `bound = K_BOUND if path == ("k",) else BUDGET_BOUND` | `bound = BUDGET_BOUND` | `test_formal_config.BoundedReading.test_each_bounded_value_is_one_the_checkers_reader_takes` |
| `S61103-A-VALUE-AT-ITS-BOUND-IS-ADMITTED` | `if value > bound:` | `if value >= bound:` | `test_formal_config.BoundedReading.test_each_bounded_value_is_one_the_checkers_reader_takes` |
| `S61104-THE-FIRST-VALUE-PAST-ITS-BOUND-IS-REFUSED` | `if value > bound:` | `if value > bound + 1:` | `test_formal_config.BoundedReading.test_each_bounded_value_is_one_the_checkers_reader_takes` |
| `S61105-AN-ENTRY-BUDGET-PAST-ITS-BOUND-IS-REFUSED` | `if seconds > BUDGET_BOUND:` | `if seconds > BUDGET_BOUND + 1:` | `test_formal_config.BoundedReading.test_each_bounded_value_is_one_the_checkers_reader_takes` |
| `S61106-AN-ENTRY-KEY-WITHOUT-THE-PREFIX-IS-REFUSED` | `if not key.startswith(ENTRY_PREFIX):` | `if False:` | `test_formal_config.BoundedReading.test_a_budget_entry_key_that_names_no_tla_entry_is_refused` |
| `S61107-THE-ENTRY-PREFIX-NAMES-A-TLA-ENTRY` | `ENTRY_PREFIX = "tla/"` | `ENTRY_PREFIX = ""` | `test_formal_config.BoundedReading.test_a_budget_entry_key_that_names_no_tla_entry_is_refused` |
| `S61108-THE-READING-IS-THE-FILES-OWN-VALUE` | `reading[name] = value` | `reading[name] = bound` | `test_formal_config.FormalConfig.test_the_bounded_reading_is_the_files_own_values` |
| `S61109-AN-ENTRY-READING-IS-THE-FILES-OWN-VALUE` | `reading[entry] = seconds` | `reading[entry] = BUDGET_BOUND` | `test_formal_config.FormalConfig.test_the_bounded_reading_is_the_files_own_values` |
| `S61110-THE-SLOT-IS-LEFT-TO-ITS-OWN-RULE` | `if path[0] == "tlc_slot" or not present:` | `if not present:` | `test_formal_config.FormalConfig.test_the_bounded_reading_is_the_files_own_values` |

## 8. References

Issue #802; issue #703; SPEC-404 section 6; SPEC-295; ADR-418; ADR-633.
