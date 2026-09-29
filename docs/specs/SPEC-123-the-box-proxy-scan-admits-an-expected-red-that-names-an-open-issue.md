# SPEC-123: the box-run proxy scan admits an expected red that names an open issue

- **Wave:** W0. **Issue:** #342 (interim for #341). **Context(s):** `repo` (`scripts/box-packs.sh`).
- **Decided by:** ADR-123, which carries SPEC-030 R12's expectation, a row expected red under the
  open issue that builds its subject, from a pack's entry to the proxy scan's.
- **Status:** judged: written at delivery, because it had no planned copy, and delivered with its
  tests and `docs/red-first/SPEC-123.md` (ADR-016).

## 1. The problem, measured

The box-run driver, `scripts/box-packs.sh`, gives a binary-built pack's wiring entry `expected_red`,
a mapping of a row to the open issue that builds its subject (SPEC-030 R12, ADR-030). It gives the
proxy client scan's entry only `pending` and `note` (`SCAN_KEYS`), and `judge_scan` reads every red
row as unexpected. Read at `dev` cc366e5:

| the scan's entry | what the driver does |
|---|---|
| `pending` with an issue | the scan reads `pending` while it examines no settings document; a scan that does examine one is a stale expectation |
| `expected_red` with a row and an issue | refused: `box.proxy-client-scan takes only ['note', 'pending']` |

Delivery #29 (PR #338) makes the scan examine a settings document for the first time, and one row,
`credential-from-secret-manager`, reads red: it accepts a secret-manager call inside the client,
where ADR-038 loads each credential into systemd's credentials directory at unit start and rejects
a secret-manager read in the application. The maintainer ruled the row substituted and not waived:
#29 carries a test that pins ADR-038's shape, and the run defers the one row to #341 until the scan
learns the socket. The deferral cannot be written, and the two ways left both fail: `pending` is
stale the moment the scan examines a settings document, and an unexpected row holds #29 on a
reading gap in a pinned scanner (ADR-123).

## 2. Requirements

R1. `box.proxy-client-scan` admits `expected_red`, a mapping of a row id to an issue written `#NNN`.
    - The helper's entry, `box.no-apikeyhelper`, does not: it has no rows, and keeps `pending` and
      `note` only.
    - The scan admits no other key beside those three: `expected_red`, `pending` and `note`.
R2. The driver refuses, as VOID, an `expected_red` that is beside `pending`, that is not a mapping,
    that is empty, that names a row that is no name, or that names a value that is not an issue.
    Each refusal names the entry and the reason.
R3. A red row the scan's `expected_red` names is expected: it is not unexpected, and the run does not
    fail for it. A red row it does not name is unexpected and fails the run, beside an expected one.
R4. The expectation is stale, and fails the run, when its issue is closed (the rule
    `closed_expectations` already applies to a pack's), or when its row does not read RED: green,
    void or absent from the scan's rows. A closed issue and a row that is not red are one stale
    expectation, never two.
R5. The runner compares the scan's red total with the expected count. The scan's line names
    how many rows are expected, in the words a pack's line uses, and its parseable shape is
    unchanged. The red total may not exceed the number of expected rows: a total beyond them
    fails the run, and a total within them, with no unexpected row, does not.
    A scan that examined no settings document is VOID and fails the run whatever `expected_red`
    names; its detail names the expectation's issues.
R6. Nothing else changes: `pending` reads as SPEC-030 R13 states it, the helper's judgment, the
    packs' judgment and the private wiring's schema name.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | an expected red row passes, and the scan's line counts it as expected | `test_box_scan_expected.py` `an_expected_red_row_passes_and_the_line_counts_it` |
| A2 | an unexpected red row beside an expected one still fails the run, by name | `test_box_scan_expected.py` `an_unexpected_red_row_beside_an_expected_one_still_fails` |
| A3 | a red total beyond the expected rows fails, and one that does not exceed the expected rows does not | `test_box_scan_expected.py` `a_red_total_beyond_the_expected_rows_fails` |
| A4 | an expected row that reads green, void or is absent is stale, and with no settings document examined the detail names the expectation's issues | `test_box_scan_expected.py` `an_expected_row_that_does_not_read_red_is_stale`, `an_expectation_with_no_settings_document_names_its_issues` |
| A5 | an expectation whose issue is closed is stale, once, on a red row and on a green one | `test_box_scan_expected.py` `an_expectation_whose_issue_is_closed_is_stale` |
| A6 | `expected_red` beside `pending` is refused | `test_box_scan_expected.py` `an_expected_red_beside_pending_is_refused` |
| A7 | an expectation with a value that is no issue, a row that is no name, an empty mapping or a list is refused | `test_box_scan_expected.py` `an_expectation_that_names_no_issue_is_refused` |
| A8 | the scan takes no key beyond `expected_red`, `pending` and `note`, and the helper takes no `expected_red` | `test_box_scan_expected.py` `only_the_scan_takes_an_expected_red_and_no_other_key` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_box_scan_expected.py -k an_expected_red_row_passes_and_the_line_counts_it
A2: python3 -m unittest discover -s scripts/tests -p test_box_scan_expected.py -k an_unexpected_red_row_beside_an_expected_one_still_fails
A3: python3 -m unittest discover -s scripts/tests -p test_box_scan_expected.py -k a_red_total_beyond_the_expected_rows_fails
A4: python3 -m unittest discover -s scripts/tests -p test_box_scan_expected.py -k an_expected_row_that_does_not_read_red_is_stale -k an_expectation_with_no_settings_document_names_its_issues
A5: python3 -m unittest discover -s scripts/tests -p test_box_scan_expected.py -k an_expectation_whose_issue_is_closed_is_stale
A6: python3 -m unittest discover -s scripts/tests -p test_box_scan_expected.py -k an_expected_red_beside_pending_is_refused
A7: python3 -m unittest discover -s scripts/tests -p test_box_scan_expected.py -k an_expectation_that_names_no_issue_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_box_scan_expected.py -k only_the_scan_takes_an_expected_red_and_no_other_key
```

The tests drive `scripts/box-packs.sh` as SPEC-030's A9 to A13 do: a planted wiring, a stand-in
scan that prints planted rows, and the runner's closed-issue seam, so no test reaches the network,
the maintainer's box or a real scan. Their rows and issues are synthetic. `test_box_packs.py` gains
three planted scan rows the new module reads.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/box-packs.sh` | repo | changed: R1 to R5 (`SCAN_KEYS`, `HELPER_KEYS`, the entry's checks, `named_issues`, `judge_scan`) |
| `scripts/tests/test_box_scan_expected.py` | repo | added: A1 to A8 |
| `scripts/tests/test_box_packs.py` | repo | changed: three planted scan rows |
| `scripts/mutation-rows.d/S12300-S12399.json` | repo | added: the scan's expectation invariants, as hand-proved rows (section 7) |
| `docs/decisions/ADR-123-the-scan-carries-an-expected-red-as-a-packs-entry-does.md` | repo | added |
| `docs/specs/SPEC-123-the-box-proxy-scan-admits-an-expected-red-that-names-an-open-issue.md` | repo | added |
| `docs/decisions/ADR-030-the-box-pack-runner-uses-each-packs-own-verb.md` | repo | changed: one amendment line at its end (insert-only) |
| `docs/specs/SPEC-030-repository-hygiene-and-wiring-honesty.md` | repo | changed: one dated amendment line at its end (insert-only) |
| `docs/red-first/SPEC-123.md` | repo | added |
| `changelog.d/fix-box-scan-expected-123.md` | repo | added |

## 5. What this does NOT do

- It does not teach the scan the credential socket. The scan's row keeps reading red until it does,
  and #341 tracks that; the expectation this delivery admits comes off when #341 closes (#341).
- It does not change delivery #29's client or its test. The test that pins ADR-038's shape is #29's
  own, and this delivery only lets the box run defer the one row (#342).
- It does not write the deferral. The wiring is private and the maintainer's; naming the row in it
  is the maintainer's step after this delivery merges (#342).
- It does not change how a pack's `expected_red` is read, and it does not admit the key on the
  helper's entry, which has no rows (#342).
- It adds no workflow and no required check. The box run stays outside CI, as SPEC-056 decides (#342).

## 6. Risks

- **An expectation outlives its reason.** It cannot: a closed issue and a row that is not red are
  each stale, and a stale expectation fails the run (R4).
- **A second red row hides behind the first.** It cannot: a red row the expectation does not name
  is unexpected, and the red total is held to the expected count (R3, R5).
- **The scan's summary changes shape.** The scan's own output does not change, so `SCAN_ROW` and
  `SCAN_ALL` read it as before; the expected count is appended to the runner's pack line, which
  keeps its shape (A1).

## 7. Mutation rows

`scripts/mutation-rows.d/S12300-S12399.json` holds twelve rows on `scripts/box-packs.sh`, each
proved killed by its full id and each naming one test as its killer:

| row | the invariant it pins | killer |
|---|---|---|
| S12301 | an expected row is not unexpected | A1 |
| S12302 | the red total may not exceed the expected count | A3 |
| S12303 | a row that does not read red is stale | A4 |
| S12304 | a closed expectation is not counted twice | A5 |
| S12305 | the scan's expected issue is read | A5 |
| S12306 | `expected_red` beside `pending` is refused | A6 |
| S12307 | the value is an issue | A7 |
| S12308 | an empty expectation is refused | A7 |
| S12309 | the scan takes no other key | A8 |
| S12310 | the scan takes a note | A8 |
| S12311 | the helper takes no `expected_red` | A8 |
| S12312 | a scan with no settings document and an expectation fails, whatever it names | A4 |

## References

SPEC-030 (R10 to R14, whose expectation this carries), SPEC-054, SPEC-056, ADR-016, ADR-030,
ADR-038, ADR-123; #29, #341, #342.
