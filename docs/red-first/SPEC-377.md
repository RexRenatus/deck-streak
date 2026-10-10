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

## Test edits between red and green

The implementation commit fcd6b954 also edits three test files between the reds and the greens.
Each edit grows a population or a fixture, and none changes what a criterion's test asserts:

- `web/app/tests-engine/sync.spec.ts` (A1 to A3): `account()` takes the browser's name, so each
  browser project signs in to a server account of its own (`playwright.engine.config.ts` makes the
  second), and the two browsers no longer share one server collection. No assertion changes.
- `web/app/src/lib/engine/protocol.test.ts`, "the study operations on the wire": the list of every
  operation the Worker admits gains the four choice operations the same commit adds. A8's test is
  unchanged between its red and its green.
- `web/app/src/lib/startapp.test.ts`: `BY_PATH` gains `/sync`, R8's screen, which is opened by its
  path alone.

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

## Part c2 (SPEC-377 section 12)

Part c2's amendment was committed first (b6489dba), then its censuses alone (8ac52ee7), then the
criteria's tests over stubs that keep each input but the missing behaviour, in two commits: the
service, the composition root, the core, the web engine and the wire (5948644e), then the Worker,
the screen and the browser proof (b743e782). No code a criterion judges came before its red.

### The fence, line by line

| # | criterion | test | red at |
|---|---|---|---|
| 1 | B1 | `crates/api/tests/snapshot_route.rs` `a_listed_sealed_snapshot_answers_found_and_its_age` | 5948644e |
| 2 | B1 | `crates/api/tests/snapshot_route.rs` `the_route_refuses_without_an_owner_session` | 5948644e |
| 3 | B1 | `crates/api/tests/snapshot_route.rs` `no_answer_names_an_object` | 5948644e |
| 4 | B1 | `crates/api/tests/snapshot_route.rs` `a_stale_answer_is_listed_again` | 5948644e |
| 5 | B2 | `crates/api/tests/snapshot_route.rs` `an_absent_list_credential_answers_unknown` | 5948644e |
| 6 | B3 | `crates/api/tests/snapshot_route.rs` `an_archive_without_its_manifest_is_not_found` | 5948644e |
| 7 | B3 | `crates/api/tests/snapshot_route.rs` `a_name_outside_the_archive_form_is_ignored` | 5948644e |
| 8 | B3 | `crates/api/tests/snapshot_route.rs` `the_age_is_the_newest_sealed_stamps` | 5948644e |
| 9 | B4 | `crates/engine-core/tests/retention.rs`, its three tests | 5948644e |
| 10 | B5 | `crates/web-engine/tests/files.rs` `only_a_listed_backup_is_exported` | 5948644e |
| 11 | B5 | `crates/web-engine/tests/files.rs` `the_backups_are_listed_newest_first_by_kind` | 5948644e |
| 12 | B5 | `crates/web-engine/tests/boundary.rs` `the_backup_exports_reach_the_pool_only_through_files_and_the_core` | 8ac52ee7 |
| 13 | B5 | `web/app/src/lib/sync/backups.test.ts`, its six tests | b743e782 |
| 14 | B5 | `web/app/src/lib/engine/backups.test.ts`, four of its seven tests | b743e782 |
| 15 | B5 | `web/app/src/lib/engine/protocol.test.ts` "the backup operations carry no path" | 5948644e |
| 16 | B6 | `web/app/tests-engine/sync.spec.ts` "a backup is listed and exported" | b743e782 |
| 17 | B7 | `web/app/src/lib/sync/sync-screen.test.ts` "the storage status is the browser's answer" | b743e782 |
| 18 | B7 | `web/app/src/lib/sync/sync-locales.test.ts` | 8ac52ee7 |
| 19 | B9 | `crates/daemon/tests/snapshot_lister.rs`, its six tests | 5948644e |

B5 and B7 hold fence lines whose reds were measured at different commits, so each of their lines
below names every commit it cites, earliest first. B8, the policy line, sits outside the fence
(section 12a): the privacy pack's `policy-published` row decides it in CI, so it has no line here.

Three tests of `web/app/src/lib/engine/backups.test.ts` pass at b743e782 and are mutation
coverage, not red-first evidence: the export answers the engine's bytes, a Worker that keeps no
backups refuses each operation by name, and an engine's refusal answers `engine-failed`. Beside
the fence, `sync-screen.test.ts` "the backup list is read at the start and after every write or
cancel" is red at b743e782 too: `AssertionError: expected +0 to be 1 // Object.is equality`.

### Which red the browser proof saw

B6 ran in Chromium on this box; WebKit runs in CI. At b743e782 the upload, the download and its
backup are real, so the test reaches the list: the Worker's list is a stub that answers no backup,
and the assertion on the list's kinds fails.

### Edits to part c1's tests

`web/app/src/lib/sync/sync-screen.test.ts` grows insert-only at b743e782 (the two tests above and
the fake's two backup operations, whose calls it records apart from the calls the existing tests
read), except one line: the client a pending sign-in never answers is typed
`SyncClient & BackupsClient`, as the screen's client now is. No assertion changes.

### The reds and greens

```red-first
B1: red at 5948644e: panicked at crates/api/tests/snapshot_route.rs:190:5: left: Object {"age_seconds": Number(0), "found": Bool(true)} right: Object {"age_seconds": Number(10), "found": Bool(true)}; and at :229:9: no cookie is refused left: 200 right: 401; and at :260:5: left: [200, 200, 200, 200] right: [200, 401, 200, 200]; and at :331:5: left: Object {"age_seconds": Number(0), "found": Bool(true)} right: Object {"age_seconds": Number(10), "found": Bool(true)}
B2: red at 5948644e: panicked at crates/api/tests/snapshot_route.rs:384:9: the owner's session is told unknown left: Object {"age_seconds": Number(0), "found": Bool(true)} right: Object {"found": Null}
B3: red at 5948644e: panicked at crates/api/tests/snapshot_route.rs:420:9: left: Object {"age_seconds": Number(0), "found": Bool(true)} right: Object {"found": Bool(false)}; and at :444:9: left: Object {"age_seconds": Number(0), "found": Bool(true)} right: Object {"found": Bool(false)}; and at :476:5: left: Object {"age_seconds": Number(0), "found": Bool(true)} right: Object {"age_seconds": Number(11), "found": Bool(true)}
B4: red at 5948644e: panicked at crates/engine-core/tests/retention.rs:24:5: left: [3] right: [1, 3]; and at :38:5: left: [4] right: [0, 2, 4]; and at :55:5: left: [0] right: [2]
B5: red at 8ac52ee7: panicked at crates/web-engine/tests/boundary.rs:1025:5: left: ["backups: `fn backups(` occurs 0 times, not once", "export_backup: `fn export_backup(` occurs 0 times, not once", "retain: `fn retain(` occurs 0 times, not once", ...]; and at 5948644e: panicked at crates/web-engine/tests/files.rs:115:5: left: [("backup-1", Some("/deck-streak/backup-1.anki2")), ("server-2", Some("/deck-streak/server-2.anki2")), ...]; and panicked at crates/web-engine/tests/files.rs:145:5: left: [Backup { id: "backup-2", kind: Backup, number: 2, made: 300, ... }, ...]; and AssertionError: backup-export's backup is malformed: expected { request: { id: 1, …(2) } } to deeply equal { id: 1, …(1) }; and at b743e782: AssertionError: expected [] to deeply equal [ …(3) ]; and AssertionError: expected { backups: [], removed: [] } to deeply equal { backups: [ { …(3) }, …(2) ], …(1) }
B6: red at b743e782: in Chromium, at tests-engine/sync.spec.ts:407 `expect(received).toContain(expected)`: Expected value: "backup" Received array: []
B7: red at 8ac52ee7: AssertionError: expected { en: [ …(10) ], es: [ …(10) ], …(5) } to deeply equal { Object (en, es, ...) } with "en lacks sync_backups_title"; and at b743e782: AssertionError: kept: expected [] to deeply equal [ …(2) ]
B9: red at 5948644e: panicked at crates/daemon/tests/snapshot_lister.rs:81:5: left: None right: Some(["sync-a.tar.age", "sync-a.sha256.age"]); and at :112:5: left: None; and at :126:5: a command inside its bound is listed left: None; and at :161:5: 64 bytes are read whole left: None; and at :180:5: a command that exits 0 is listed left: None; and at :224:5: no credential, no lister
```

### The greens

Every fence line of part c2 passes at 4edd078f, the commit that builds section 11 whole and changes
no test file. B6 passed in Chromium locally; WebKit runs in CI. The record names 4edd078f rather
than the pushed tip: the tests and the code they exercise are byte-identical between the two,
which the pull request proves with `git diff --quiet` over their paths. B8 has no line here: the
privacy pack's `policy-published` row decides it in CI.

```red-first
B1: green at 4edd078f
B2: green at 4edd078f
B3: green at 4edd078f
B4: green at 4edd078f
B5: green at 4edd078f
B6: green at 4edd078f
B7: green at 4edd078f
B9: green at 4edd078f
```

### After the greens

The record above names 4edd078f. Its sentence that the tests and the code they exercise are
byte-identical to the pushed tip holds for B1, B2, B3 and B4 only, and this section says what
changed after it, without editing that sentence.

- 3fa2cc8d changed the order in which the daemon reads the archive lister after a start refusal.
  The daemon whole suite was red before it and passed with 104 tests at it. No test changed.
- cb0eb1de added a census of the backup helpers that only the wasm32 build compiles. It exists
  for mutation coverage, and no criterion's red or green moves with it.
- 817417c1 added four tests, again for mutation coverage: the listing command past its time bound
  is stopped and not left running (B9), the alert clears and no removal list is shown (B5), and the
  storage status waits for the browser's answer (B7).
- c63b0a27 dropped two guards no answer reaches, one in the backups helper and one in the sync
  screen's text choice. Behaviour is unchanged.

B5, B6, B7 and B9 were re-run green at c63b0a27: the files test 8 passed with the listed-backup
export test among them, the boundary test 12 passed, the backups vitest file 8 passed, the browser
test for B6 1 passed, the storage status test for B7 1 passed, and the lister test file for B9 7
passed. The pull request proves by `git diff --quiet` that the tests and code of B1 to B4 are
byte-identical between 4edd078f and the pushed head, and that the seven paths changed since are
byte-identical between c63b0a27 and the pushed head.

## Census fix round

Two checks of the pull request's own reds were watched failing in CI before any fix, in run
38021001105 over 526852d6.

- Job 114121749093 (hygiene), the setting-shape pin test, read:
  `daemon::ListCommand (src/snapshot_lister.rs) "an absolute program and its arguments"` as
  unpinned (`1 unpinned`, 30 impls examined).
- Job 114121749032 (rust), the capture census, read:
  `assertion `left == right` failed: capture population: 736 file(s) read; 0 raw capture(s), 24 routed, 1 global default(s)`
  with `left: 24` and `right: 23`, at `crates/kernel/tests/log_capture_class.rs:1588:5`.

The fix pins the lister command's shape in a test of its crate, and raises the census's routed
figure from 23 to 24 for the one capture this delivery added, the route test's
`hold_capture` call. The raw figure stays 0 and no assertion is removed.

Both are green at c940cb43.

- The setting-shape pin test: `Ran 76 tests`, `OK`, with the lister command's shape now spelled by
  a test of its own crate.
- The capture census: `capture population: 736 file(s) read; 0 raw capture(s), 24 routed, 1 global default(s)`
  and `test result: ok. 1 passed`. The kernel crate's whole suite and the lister test file (8
  passed) are green as well.
