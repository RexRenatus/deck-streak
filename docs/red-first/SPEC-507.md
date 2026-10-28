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
