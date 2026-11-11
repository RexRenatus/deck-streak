# SPEC-677: a flaky test is quarantined with the runs that disagreed, and keeps running

Follow-up to SPEC-436 and ADR-450, from #913; tracked by #1396. Decided by ADR-700.

## 1. The problem, measured

No listing in the tree records a flaky test, and no census reads the markers that keep a test
from running or from failing its run. `git grep -n -i -E 'flak|quarantin'` finds no quarantine
listing; each hit is a risk paragraph or a media store. The markers at the base are a handful
of Rust `#[ignore]` attributes, most of them child runs that a parent test starts with
`--ignored`, and at least three Playwright `test.skip` calls; no runner configuration sets a retry. The
whole population is measured by this delivery's own first CI run (the `hygiene` job).

## 2. Requirements

R1. The census reads every test file and test runner configuration of the five stacks (Rust,
Python, web, Swift and the nextest configuration), each chosen by its stack's rule over the
file's path relative to the checkout (`member`); it prints `examined N test file(s)` and
refuses 0, which is VOID.
R2. It finds each marker form by stack: `#[ignore]` and a `cfg_attr` ignore; `skip`, `skipIf`,
`skipUnless`, `expectedFailure`, `self.skipTest` and `SkipTest`; `.skip`, `.skipIf`, `.runIf`,
`.fixme`, `.todo`, `.fail`, `.only` and a `retries` or `retry` setting; `XCTSkip` and
`XCTExpectFailure`; and a nextest `retries` setting.
R3. A quarantine entry names its test as `<file>::<name>`, and the census refuses an entry whose
test is not in a census file.
R4. A quarantine entry's record holds at least two runs on one head, each by run id, attempt, job
and conclusion, whose conclusions are a success and a failure; the census refuses any other.
R5. A marker is not a site when its own condition names a platform, or when it is a Rust
`#[ignore]` whose reason names a test of the same file that runs it with `--ignored`. No other
form is admitted.
R6. Every other marker is a site. The listing holds each site by file, stack and form with its
count and a reason; a listed site with no reason is refused.
R7. At `warn`, an unlisted site is printed as a `WARN quarantine:` line and passes; at `fail` it
fails. A listing entry that names no current site, and every refusal of R3, R4 and R6, fails at
both levels.
R8. The level starts at `warn`, and moves to `fail` only by its own delivery under ADR-450 D6's
promotion rule; a demotion is a weakening.
R9. A quarantined test keeps running: an entry never turns a test off, and no test, runner
configuration or workflow changes in this delivery.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the census reads every test file and runner configuration, five stacks, and holds exactly the listed sites | `test_the_tree_holds_exactly_its_listed_sites` |
| A2 | a quarantined test with two runs on one head that disagreed is clean | `test_a_listed_test_with_a_full_record_is_clean` |
| A3 | an entry with no such record is refused | `test_an_entry_with_no_record_is_refused` |
| A4 | an entry whose test is not in the census is refused | `test_an_entry_whose_test_is_gone_is_refused` |
| A5 | a marker with no entry and no reason is a site | `test_a_marker_with_no_entry_and_no_reason_is_a_site` |
| A6 | a platform condition and a named child run are admitted, nothing else | `test_a_marker_whose_condition_names_a_platform_is_clean` |
| A7 | warn reports an unlisted site, fail fails it, and a stale entry fails at both | `test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does` |
| A8 | an empty population is VOID | `test_an_empty_population_is_void` |
| A9 | a Vitest file under `web/app/src` is read, and its skip is a site | `test_a_vitest_file_under_web_app_src_is_read_and_its_skip_is_a_site` |
| A10 | a root whose own path holds `src` is still read | `test_a_root_whose_own_path_holds_src_is_still_read` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_the_tree_holds_exactly_its_listed_sites
A2: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_a_listed_test_with_a_full_record_is_clean
A3: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_an_entry_with_no_record_is_refused
A4: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_an_entry_whose_test_is_gone_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_a_marker_with_no_entry_and_no_reason_is_a_site
A6: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_a_marker_whose_condition_names_a_platform_is_clean
A7: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does
A8: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_an_empty_population_is_void
A9: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_a_vitest_file_under_web_app_src_is_read_and_its_skip_is_a_site
A10: python3 -m unittest discover -s scripts/tests -p test_quarantine_census.py -k test_a_root_whose_own_path_holds_src_is_still_read
```

## 4. File manifest

| path | change |
|---|---|
| `scripts/tests/test_quarantine_census.py` | added: the census and its tests |
| `scripts/tests/quarantine.json` | added: the listing, at `warn`, with today's sites |
| `scripts/tests/fixtures/quarantine/clean.json.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/flaky.py.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/gone.json.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/markers.py.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/markers.rs.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/markers.ts.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/no_record.json.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/src/scripts/tests/test_planted.py.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/stale.json.fixture` | added: a planted text, read and never run |
| `scripts/tests/fixtures/quarantine/web-src/web/app/src/lib/planted.test.ts.fixture` | added: a planted text, read and never run |
| `docs/specs/SPEC-677-a-flaky-test-is-quarantined-with-the-runs-that-disagreed-and-keeps-running.md` | added: this SPEC |
| `docs/decisions/ADR-700-a-quarantine-listing-binds-each-flaky-test-to-its-runs-and-a-marker-census-starts-at-warn.md` | added: its ADR |
| `docs/red-first/SPEC-677.md` | added: the red-first record |
| `changelog.d/flaky-quarantine.md` | added: the fragment |
| `scripts/mutation-rows.d/S67700-S67799.json` | added: the rows |

## 5. What this does NOT do

- It changes no test, marker, runner configuration or workflow, and cures no site (#1396).
- It reads no CI log: a record is entered from run and job metadata by the delivery that lists it (#1396).
- It does not census a filter in a runner configuration or a stage command (a `testIgnore`, an `-E` filterset): #913 tracks it.

## 6. Risks

- A marker spelt across lines is missed: the planted texts pin each form on one line, and the first criterion's
  examined count shows what was read.
- A child run whose reason elides its parent's name is a site, not admitted: it is listed with
  its reason, and a rewrite of the reason is the owner's choice, never this delivery's.
