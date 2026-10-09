# Red-first record: SPEC-377

The SPEC, ADR-388 and the schematic were committed first (71c74934), then the censuses alone
(2a90ef07), then every other criterion's test over stubs that keep each input but the missing
behaviour (e8fff72d), before any code the criteria judge. Each red below is quoted from the run at
the commit named. This delivery is part c1 of SPEC-377; section 7 is part c2's.

## The fence, line by line

Each of the 19 lines of SPEC-377 section 3's fence resolves to a test this delivery adds, or to a
test that stands on `dev` and is named as it is.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `web/app/tests-engine/sync.spec.ts` "an upload waits for its snapshot" | added (step 3) |
| 2 | A2 | `web/app/tests-engine/sync.spec.ts` "a download is written after its backup" | added (step 3) |
| 3 | A3 | `web/app/tests-engine/sync.spec.ts` "an evicted collection is restored from the server" | added (step 3) |
| 4 | A4 | `crates/engine-core/tests/one_way.rs` `the_choice_paths_are_judged_by_the_installed_files_port` | added (step 3) |
| 5 | A4 | `crates/engine-core/tests/one_way.rs` `a_path_the_files_port_holds_is_read_for_rows` | added (step 3) |
| 6 | A4 | `crates/engine-core/tests/one_way.rs` `the_open_collection_by_another_spelling_is_refused` | added (step 3) |
| 7 | A4 | `crates/engine-core/tests/one_way.rs` `an_uninstalled_browser_port_refuses_every_choice_path` | added (step 3) |
| 8 | A5 | `crates/web-engine/tests/files.rs`, its four tests | added (step 3) |
| 9 | A6 | `crates/web-engine/tests/boundary.rs`, whole | named: dev's census, grown insert-only by five census tests (step 2) |
| 10 | A7 | `web/app/src/lib/engine/choice.test.ts`, its three tests | added (step 3) |
| 11 | A8 | `web/app/src/lib/engine/protocol.test.ts` "the choice operations carry no path, no id set and no snapshot answer" | added (step 3) |
| 12 | A9 | `web/app/src/lib/study/engine.test.ts` "a session syncs at its start and end" | added (step 3) |
| 13 | A10 | `web/app/src/lib/sync/sync-screen.test.ts` "the sign-in posts once and shows each status word" | added (step 3) |
| 14 | A11 | `web/app/src/lib/sync/sync-screen.test.ts` "the unsynced count is the worker's" | added (step 3) |
| 15 | A12 | `web/app/src/lib/sync/choice-screen.test.ts`, its four tests | added (step 3) |
| 16 | A13 | `web/app/src/lib/sync/sync-screen.test.ts` "a collection the browser lost offers the download alone" | added (step 3) |
| 17 | A14 | `web/app/tests/a11y.spec.ts` "/sync has no WCAG 2.2 AA violation axe can find" | named: dev's run over `ROUTES`, which gains `/sync` (step 3) |
| 18 | A15 | `web/app/src/lib/sync/sync-locales.test.ts` | added (step 2) |
| 19 | A16 | `web/app/src/lib/engine/credential-reach.test.ts` "where the sync key reaches the choice" | named: dev's census, grown insert-only by two tests (step 3) |

## Which red each browser proof saw

SPEC-377 section 3 names the fork's file read, timer and temporary file as A1 to A3's red. At
e8fff72d those calls are not yet reached: the Worker's protocol knows no choice operation, so the
page's first choice request is refused as an unknown operation, and the web engine's four exports
behind it are stubs that refuse every choice. Each test therefore went red on the first choice
request it sent. The fork's sites are reached once the operations and the exports are real.

A1 to A3 and A14 ran in Chromium on this box; WebKit runs in CI.

## The reds and greens

Each line's command is the criterion's line in SPEC-377 section 3's fence, run at the commit named.

```red-first
A1: red at e8fff72d: in Chromium, at tests-engine/sync.spec.ts:308 the count answered `{"code": "bad-request", "message": "unknown operation choice-count"}` where `{"status": "held", "snapshot": {"found": false}}` was expected
A2: red at e8fff72d: in Chromium, at tests-engine/sync.spec.ts:283 the upload that seeds the server answered `{"code": "bad-request", "message": "unknown operation choice-confirm"}` where `{"status": "held", "outcome": "written"}` was expected
A3: red at e8fff72d: in Chromium, at tests-engine/sync.spec.ts:283 the upload that seeds the server answered `{"code": "bad-request", "message": "unknown operation choice-confirm"}` where `{"status": "held", "outcome": "written"}` was expected
A4: red at e8fff72d: panicked at crates/engine-core/tests/one_way.rs:1292:5: left: (Ok(()), Err(HoldsRows), [], true); and panicked at crates/engine-core/tests/one_way.rs:1334:5: left: [(true, Err("HoldsRows"), []), (false, Err("HoldsRows"), [])]; and panicked at crates/engine-core/tests/one_way.rs:1363:5: left: (Err(OpenCollection), []); and panicked at crates/engine-core/tests/one_way.rs:1395:5: left: three Err(Engine(..)), right: ([Err(OpenCollection), Err(OpenCollection), Err(OpenCollection)], false)
A5: red at e8fff72d: panicked at crates/web-engine/tests/files.rs:29:5: left: [("/deck-streak/collection.anki2", true), ("/deck-streak/backup-1.anki2", true), ("/deck-streak/backup-2.anki2", true), ("/deck-streak/Collection.anki2", true), ("/deck-streak/collection.anki2-journal", true), ("", true)]; and panicked at crates/web-engine/tests/files.rs:72:5: left: [Some("/deck-streak/backup-1.anki2"), Some("/deck-streak/backup-1.anki2"), Some("/deck-streak/collection.anki2")]
A6: red at 2a90ef07: panicked at crates/web-engine/tests/boundary.rs:896:5: left: ["full_sync_count: `fn full_sync_count(` occurs 0 times, not once", "full_sync_confirm: `fn full_sync_confirm(` occurs 0 times, not once", ...]; and panicked at crates/web-engine/tests/boundary.rs:784:5: left: ["create_backend installs the pool as the core's files port when it starts the dispatcher, and its body lacks `let port: Arc<dyn CoreFiles> = Arc::new(PoolPort);`", ...]
A7: red at e8fff72d: AssertionError: expected { found: true, age: +0 } to deeply equal { found: true, age: 90 }; and AssertionError: expected { status: 'held', outcome: 'written' } to deeply equal { status: 'held', …(2) }
A8: red at e8fff72d: AssertionError: 1: expected { id: 1, …(1) } to deeply equal { request: { id: 1, …(1) } }
A9: red at e8fff72d: AssertionError: expected [ 'open' ] to deeply equal [ 'open', 'sync' ]
A10: red at e8fff72d: TestingLibraryElementError: Unable to find an accessible element with the role "status"
A11: red at e8fff72d: TestingLibraryElementError: Unable to find an element with the text: Reviews waiting to sync: 7
A12: red at e8fff72d: AssertionError: both: expected false to be true // Object.is equality; and TestingLibraryElementError: Unable to find an element with the text: Counting what each choice replaces…
A13: red at e8fff72d: TestingLibraryElementError: Unable to find an element with the text: This browser holds no copy of your collection. Download the server's copy to restore it.
A14: red at e8fff72d: in the light and the dark theme, axe's `label` rule, impact critical: "Form elements must have labels"
A15: red at 2a90ef07: AssertionError: expected { en: [ …(30) ], es: [ …(30) ], …(5) } to deeply equal { Object (en, es, ...) }
A16: not red: an absence census over the new operations, held by its planted controls
A1: green at 82c0f0a3
A2: green at 82c0f0a3
A3: green at 82c0f0a3
A4: green at fcd6b954
A5: green at fcd6b954
A6: green at fcd6b954
A7: green at fcd6b954
A8: green at fcd6b954
A9: green at fcd6b954
A10: green at fcd6b954
A11: green at fcd6b954
A12: green at fcd6b954
A13: green at fcd6b954
A14: green at fcd6b954
A15: green at fcd6b954
```
