# Red-first record: SPEC-338

The SPEC, its schematic, ADR-348 and ADR-349 were committed first (53561380), and the manifests,
the lockfile and the crate's skeleton next (6754bbc0). Each criterion's tests were then committed
before the code that turns them green, in four red and green pairs, and each red below is quoted
from the run at the red commit.

```red-first
A14: red at f535806: the patch replaces ['anki', 'anki_proto'], not anki alone
A14: green at ea34f36
A15: red at f535806: ADR-058's Confirmation does not name the pinned commit `fedcba98...`
A15: green at ea34f36
A1: red at f4cd373: wire rating 2 left: Ok(Again) right: Ok(Hard)
A1: green at a9d7de2
A2: red at f4cd373: wire rating 0 left: Ok(Again) right: Err(RatingOutOfRange(0))
A2: green at a9d7de2
A17: red at f4cd373: schedule_cards_as_new left: Ok("unlisted") right: Err("run_method refuses service 13 method 17: not a study call")
A17: green at a9d7de2
A3: red at 07a76d2: AssertionError: 0 != 1
A3: green at b3d1156
A4: red at 07a76d2: AssertionError: 'web-engine-size: module deck_streak_web_engine_bg.wasm: raw 128008, gzip-9 20564, brotli-11 20058' not found in ''
A4: green at b3d1156
A5: red at 07a76d2: AssertionError: 0 != 2 : module is missing:
A5: green at b3d1156
A6: red at 59aead3: AssertionError: expected [] to deeply equal [ 'script-src wasm-unsafe-eval' ]
A6: green at 591d81f
A7: red at 59aead3: AssertionError: expected [] to deeply equal [ { id: 1, op: 'open' }, …(2) ]
A7: green at 591d81f
A8: red at 59aead3: AssertionError: expected [ null, …(1) ] to deeply equal [ null, { id: null, ok: false, …(2) } ]
A8: green at 591d81f
A9: red at 59aead3: AssertionError: expected { id: +0, ok: true, value: null } to deeply equal { id: 1, ok: false, …(2) }
A9: green at 591d81f
A10: red at 59aead3: AssertionError: expected { id: +0, ok: true, value: null } to deeply equal { id: 1, ok: false, …(2) }
A10: green at 591d81f
A11: red at 59aead3: AssertionError: expected null to deeply equal { existed: false, notes: +0 }
A11: green at 591d81f
A12: red at 59aead3: AssertionError: expected [ 'already persisted', 'unsupported' ] to deeply equal [ 'already persisted', 'persisted' ]
A12: green at 591d81f
A13: red at 59aead3: AssertionError: expected [] to deeply equal [ { id: 4, ok: false, …(2) }, …(1) ]
A13: green at 591d81f
A16: not red: test_ci_workflows.py is never run locally (ruling 189); CI by name decides it
A18: red at 64530a2: AssertionError: expected { id: 2, ok: false, …(2) } to deeply equal { id: 2, ok: true, value: 1114112 }
A18: green at ab2d6e8
A19: red at 64530a2: assertion `left == right` failed: note 0's fields left: (17, 16) right: (200, 200)
A19: green at ab2d6e8
```

## What each pair disclosed

- **A14 and A15 were re-keyed inside their green commit.** At f535806, `test_engine_pin.py`'s
  planted defects of ADR-058's appended note sat in one table with the Confirmation's own planted
  defects, all judged at one commit. The green commit ea34f36 moved the four note defects into a
  table of their own, judged at the note's commit (the planted tip), beside the judge that reads
  the note, so the green commit changed the test as well as the code it judges. The reds at
  f535806 are each criterion's reason (a manifest that patches `anki_proto` beside `anki`, and a
  Confirmation that names no pinned commit); the re-key changed which commit a note defect is
  judged at, not what the note must hold. Run by name at this record's commit, the two tests read
  `examined 4 planted note defect(s)`, `examined 5 planted defect(s)`, `examined 8 planted
  defect(s)` and `examined 5 engine package(s) in Cargo.lock`, and `OK`.
- **A1, A2 and A17 were red over stubs.** f4cd373 committed the tests with a `study.rs` that
  answered every wire rating with `Again` and admitted a call outside the table as `unlisted`
  rather than refusing it; a9d7de2 replaced the stubs. `cargo test` read 3 failed at the red and
  3 passed at the green.
- **A3 to A5 were red over a stub.** 07a76d2 committed the tests with a size script that printed
  nothing and exited 0; the run read `Ran 4 tests`, `FAILED (failures=4)` (the fourth is the
  absent-compressor case beside A5). b3d1156 read `Ran 4 tests` and `OK`.
- **A6 to A13 were red over stub modules.** 59aead3 committed the Vitest files with `protocol.ts`,
  `client.ts`, `session.ts`, `persistence.ts` and `worker.ts` as stubs that type-check and do
  nothing, so each red is an assertion on the missing behaviour, not an import error. The run read
  `Tests  15 failed (15)`; 591d81f read `Tests  18 passed (18)`, and `svelte-check` read 0 errors
  and 0 warnings.
- **A6's red was seen twice.** It was first seen before 59aead3 was committed, and seen again with
  the engine's modules implemented in the tree and `svelte.config.js` not yet changed, reading
  `Tests  2 failed | 16 passed (18)`: the policy test and the exact-directives test, the two that
  read `script-src`.
- **A9's first draft was re-ordered before 59aead3.** It opened the first tab before asserting the
  second tab's refusal, and so read red on the first tab's open, which is not A9's reason. The test
  was re-ordered so the second tab's refusal is asserted first, and the red quoted above is the
  re-ordered run.
- **A18 and A19 joined the SPEC with their own commit (cad8c4e).** The Worker's peak memory needed
  a reading the page could take, and section 7's realistic collection needed ADR-022's note shape,
  so the SPEC gained both criteria before their tests. At 64530a2, A18 asked an open session for
  `memory`, which the protocol did not yet hold, so the reply was `bad-request`; the fake engine's
  `memory_pages` named a key `EngineModule` did not have yet, which `svelte-check` reads as a type
  error, while Vitest strips types and ran the assertion. A19 read 250,001 notes over a stub that
  wrote the old short fields. At ab2d6e8 `cargo test` read 4 passed across the crate's two test
  files, and Vitest read `Tests  19 passed (19)` across the engine's files and the policy's.
- **A16 has no local red.** Its judge, `web_engine_job_problems`, and its test are in
  `scripts/tests/test_ci_workflows.py`, which is never run on a builder's machine; CI runs it by
  name, over the real `ci.yml` and a planted copy for each refusal.
