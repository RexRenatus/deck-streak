# Red-first record: SPEC-507

The SPEC, ADR-525, the schematic and the changelog fragment came first (ceee0258). The engine core's tests came next,
whole, over a stub record that keeps every shape and computes nothing (036a22e0): `capped` reads 0, `card_health`
answers zeros, no rows, no mean or median and a provenance of 0, and `to_json` answers an empty object; the dispatcher's
two fixed reads are in place. dc55ab82 turns them green in `health.rs` and adds the web adapter's `card_health` export.
The web tests came over stubs (830023e0): the panel is a dialog holding only its Close button, the panel's lines answer
empty, the census pattern matches nothing, and the review's overlay seam does nothing. a939d705 builds the panel, the
review screen's control, the Worker operation, the clock's pause and the strings, and the study suite's axe step.

```red-first
A1: red at 036a22e0: health.rs: examined 0 review row(s): the population is empty, so nothing was judged
A1: green at dc55ab82
A2: red at 036a22e0: health.rs: assertion `left == right` failed: the panel's capped time and the analytics crate's
A2: red at 036a22e0: health.rs: examined 0 review row(s): the population is empty, so nothing was judged
A2: green at dc55ab82
A3: red at 036a22e0: health.rs: assertion `left == right` failed: the recall rows' part names its state and no number
A3: green at dc55ab82
A3: red at 830023e0: card-health.test.ts: TestingLibraryElementError: Unable to find an accessible element with the role "region" and name "Recall"
A3: green at a939d705
A4: red at 830023e0: card-health.test.ts: AssertionError: examined 0 panel string(s): the population is empty, so nothing was judged
A4: red at 830023e0: card-health.test.ts: TestingLibraryElementError: Unable to find an accessible element with the role "region" and name "Where it came from"
A4: green at a939d705
A5: red at 830023e0: review-clock.test.ts: AssertionError: sequence 1: expected [ '1 31647', '2 260567', …(3) ] to deeply equal [ Array(5) ]
A5: red at 830023e0: card-health.test.ts: TestingLibraryElementError: Unable to find an accessible element with the role "button" and name "Card info"
A5: red at 830023e0: card-health.test.ts: AssertionError: expected '<script lang="ts">\n  // SPEC-350 R7,…' to match /<CardHealthPanel\b/
A5: green at a939d705
A6: red at 830023e0: study-coverage.test.ts: AssertionError: expected [ …(7) ] to deeply equal [ …(8) ]
A6: green at a939d705
```

## What each pair disclosed

- **The web runs:** at 830023e0 the three web test files read 3 failed of 3, 9 tests failed of 9; at a939d705 the whole
  Vitest suite reads 114 files and 694 tests passed, and `check` (svelte-check with `--fail-on-warnings`) reads 1652
  files, 0 errors and 0 warnings.
- **a939d705 edits three web tests the fence does not name.** `session.test.ts` and
  `credential-stand-in.test.support.ts` give their stand-ins for the engine's module the new `card_health`, answering
  `null`, as the module's type now requires; `protocol.test.ts` adds `card-health` to its census of the Worker's
  operations, after `faces`.
- **a939d705 edits one fence test's reading, not its expectations.** `the panel shows the counts, the capped times and
  the newest reviews first` reads each table row cell by cell, joined by a space, since a table holds no whitespace
  between its cells; the expected rows are unchanged.
- **The Rust runs:** at 036a22e0 the health target's four tests read 4 failed of 4 (exit 101): A2's differential and A3
  on their own assertions, and A1 and A2's real-answers test on the population guard, since the stub's record carries
  no review row. At dc55ab82 they read 4 passed of 4 (exit 0), measured at 3ad7f0b0, whose Rust tree is dc55ab82's.
  Each run was `cargo test --locked --offline --no-fail-fast -p deck-streak-engine-core --test health` with the four
  test names.
- **50207a62 renames one binding, after green.** `cargo clippy -p deck-streak-engine-core --all-targets --locked
  --offline -- -D warnings` denied `similar_names` on the rows' lapse count beside the card's `lapses` counter; it is
  now `logged_lapses`, and no test changed. At 50207a62 clippy reads clean, `cargo fmt --all --check` reads clean and
  the four tests read 4 passed of 4.
- **The `card_health` export builds for wasm32.** At 50207a62, `cargo build --release --locked --offline --target
  wasm32-unknown-unknown -p deck-streak-web-engine` (step 1 of `scripts/web-engine-build.sh`, with clang-18 and
  llvm-ar-18) exits 0, and the module names `card_health`. Its bindings and size steps (wasm-bindgen, wasm-opt) run in
  CI's web-engine job.
- **The study suite's axe step is not measured here.** It runs in Chromium and WebKit over the built module, in CI.

## Fix 1: tests for the mutants a review left alive

A review of the delivery at f0050af3 applied 61 hand-written mutants to the lines it adds, and 24 survived the tests.
d75e821a (the engine core's health tests) and 041ff071 (the panel's and the clock's tests) change tests only: no
production line changed, so each mutant still applies where it did. Each changed test was seen red with each mutant it
answers applied on a scratch tree at 041ff071, on its own assertion, and green at 041ff071 itself.

| mutant | requirement | the test that fails | its red with the mutant applied |
|---|---|---|---|
| `capped` reads a negative time as the cap | R3 | A1 | the capped figures: total 189,999 and mean 31,667, not 129,999 and 21,667 |
| the mean truncates | R3 | A1 | mean 21,666, not 21,667 (the odd sum's 21,666.5 rounds half up) |
| the median of an even count rounds up | R3 | A1 | median 16,001, not 16,000 (the middle pair's 16,000.5 is floored) |
| the record keeps every row | R2 | A1 | 207 rows carried, not the newest 200 |
| the record keeps the oldest 200, reversed | R2 | A1 | the rows begin with the oldest entries set by hand, not the newest fixture row |
| a lapse is any Again | R5 | A1 | the log's lapses read 2, not 1: a learning Again counted |
| a lapse is any review row | R5 | A1 | the log's lapses read 2, not 1: a review Good counted |
| a state spells another's word (four mutants) | R6 | A3, Rust | the scheduler part reads `fixture` for absent, `absent` for fixture, `stale` for unready and `unready` for stale |
| the JSON carries the created instant in seconds | R7 | A1 | `created_ms` 1793218510, not the card's id 1793218510377 |
| the clock keeps an overlay still open at the grade | R9 | A5, the clock property | in the first sequence, card 4's grade under an open overlay sends 82,745 ms, not 39,488 |
| an overlay open as a new card shows keeps its earlier start | R9 | A5, the clock property | in the first sequence, card 5's grade sends 9,956 ms, not 53,213 |
| the Worker reads Ready without both figures | R6 | A3, Vitest | a Ready part with no `days_to_90` reads Ready, not absent |
| the Worker reads every state as absent | R6 | A3, Vitest | the core's `fixture` record reads absent |
| the Worker drops the created instant | R7 | A4, where the card came from | `createdMs` 0, not 1712345678901 |
| the panel shows the log's answers as its reviews | R5 | the counts test | "Reviews 4 Lapses 1" where the card's counter reads 6 |
| the panel shows the log's lapses as its lapses | R5 | the counts test | "Lapses 1" where the card's counter reads 3 |
| the log line only when the answers differ | R5 | the counts test | no log line when only the lapses differ |
| the newest-of line at the bound | R2 | the counts test | "The newest 5 of 5 reviews." shown when every row is |
| a fetch reached through `globalThis` | R10 | A5, the census | `global globalThis` and `member fetch` refused |
| a navigation through `location` | R10 | A5, the census | `global location` and `member assign` refused |
| an endpoint route under another name | R10 | A5, the census | 27 route files against the base's frozen 26 |

- **What the tests now hold.** A1's card has 200 older entries set by hand and seven rows: a negative time, an hour's
  answer, an odd capped sum, an even answer count whose middle pair's sum is odd, a learning Again and a review Good,
  neither a lapse; its counters differ from the log one count at a time across the two cards, and the record's JSON
  carries the card's id as its created instant, a millisecond instant of the run. A3 writes each state's word out
  rather than reading it back from the type. The Vitest recall and provenance tests feed the core's own JSON through a
  Worker session over a stand-in module; the counts test sets each counter apart from the log in turn and passes the
  row bound; the clock property grades under open overlays (830 of its 1,970 grades) and shows cards under them.
- **The census is default-deny.** It parses the panel's two modules with Svelte's parser and refuses any import,
  undeclared name, property read by name, indexed value, kind of syntax, element or attribute that its closed lists do
  not hold; names resolve through the scopes that declare them, so a global a parameter shadows elsewhere is still
  refused, and every list entry must be one the modules use. Ten planted reaches are refused (a direct fetch, one
  through `globalThis`, `location`, a shadowed `location`, a fetch through an element's document, a dynamic import, a
  navigation import, a link, an indexed value and a shorthand `fetch`). The base's 26 route files are frozen, so a
  route under any name is refused.
- **The differential's domain is non-negative times.** R4 now says so: the analytics crate's function keeps a negative
  time negative where the panel reads it as 0 (R3). A2's generator draws no negative time; A1's fixture holds it.
- **All 61 mutants ran again at 041ff071.** Examined 61, killed 61, none survived and none failed to apply or build:
  the 29 Rust mutants each against the health target's four tests (4 passed before and after), and the 32 web mutants
  each against the panel's, the clock's, the session's and the protocol's test files (38 passed before and after),
  each judged by a test named for it failing.

## Part b: the panel on iPhone and iPad

Section 8 of the SPEC came first (f5ac4b77). The adapter's tests came next, whole, over a stub that keeps every shape
and computes nothing (345ae8bf): the native record's types are in place, its conversion from the core's record answers
zeros, `card_health_lines` answers no section, and `Engine::card_health` answers no record. The same commit adds the
card health collection's builder and its `health/` beside the review fixture. 989c88cb fills the conversion, the
panel's words and the export. The app's tests came over Swift stubs (d40886e7): the clock holds no overlay's time, the
session reads no section, the panel's model reads nothing, and the chrome has no control. 12d22421 builds the clock,
the model, the sheet, the chrome's control and the session's read.

```red-first
B1: red at 345ae8bf: card_health.rs: the card exists
B1: green at 934f96b2
B2: red at 345ae8bf: card_health.rs: assertion `left == right` failed: the native record and the core's
B2: green at 934f96b2
B3: red at 345ae8bf: card_health.rs: the card exists
B3: green at 934f96b2
B4: red at 345ae8bf: card_health.rs: the card exists
B4: green at 934f96b2
```

### What each pair disclosed

- **The Rust runs:** at 345ae8bf the card health target's four tests read 4 failed of 4 (exit 101): B2 on its own
  assertion, every one of its 300 generated cards a mismatch, and B1, B3 and B4 on the stub's missing record, after
  B1's queue read had found the health card at the head of its deck. At 934f96b2 they read 4 passed of 4 (exit 0).
  Each run was `cargo test --locked --offline --no-fail-fast -p deck-streak-ffi --test card_health` with the four test
  names.
- **989c88cb's first run read 3 passed of 4.** B4's planted census control expected two catches in "This card is a
  leech at risk." where the census reads three: the word `leech`, the word `risk` and the phrase `at risk`.
  934f96b2 corrects the control's expectation alone; the census, the panel's words and the other three tests are
  unchanged.
- **a3cf501f answers clippy, after green.** `cargo clippy -p deck-streak-ffi --all-targets --locked --offline -- -D
  warnings` denied `doc_markdown` on one doc line of `health.rs` and `too_many_lines` on B4; the line now says "the
  screen reader", and B4 calls two functions that hold its quiet, named and missing records and its census. No
  assertion changed. At a3cf501f clippy reads clean and `cargo fmt --all --check` reads clean.
- **The whole adapter package at 12d22421:** `cargo test --locked --offline --no-fail-fast -p deck-streak-ffi` reads
  18 binaries, 67 passed and 0 failed, the fixture's own test and the review actions' among them. The built
  `review-fixture` example writes `health/` beside `parity/` and `sound-video/` and exits 0.
- **The Swift census holds at each Swift commit.** `test_ios_thin_swift.py`, `test_ios_review_screen.py` and
  `test_ios_answer_door.py` read OK at d40886e7 and at 12d22421, with the register recording the clock at 2 decisions,
  the panel's model at 1 and the sheet at 0; the chrome stays at 3 of its 3, the review model at 6 of its 6, and the
  two session files at 4 and 3.
- **B5 to B7 are not measured here.** They are XCTest and XCUITest cases, which run only on macOS with the iPhone
  and iPad simulators; on Linux the Swift is neither compiled nor run, so neither their red at d40886e7 nor their
  green at 12d22421 is claimed. They wait for the Apple lane.
