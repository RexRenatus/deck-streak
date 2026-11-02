# Red-first record: SPEC-516

The SPEC, ADR-534, the schematic and the changelog fragment were installed first (c9b8b378). The tests came next,
whole, over stubs that compile (d85ad2e9): the export route is not merged, the core's back up now refuses every
file, the web engine has no `back_up_now` export and no `backup_id`, the adapter's backup calls list and export
nothing, the web data screen renders an empty page, `/data` is a screen the route table does not list, and the
locales and the policy hold none of the new text. 74c95e02 turns them green, 60b4ce18 makes the SPEC name the tests
and files the build wrote, and 8bf128d3 adds section 7's rows. Each red below is quoted from a run of the named
tests on the tree of d85ad2e9; each green from a run on the tree of 8bf128d3 (the Rust targets, the browser leg and
the policy's row), whose web and Python files are those of 74c95e02.

```red-first
B1: red at d85ad2e9: web/app/src/lib/data/data-screen.test.ts: expected the route table to include '/data'; src/lib/a11y-coverage.test.ts: the route table's 21 paths are not the 22 screens under src/routes
B1: green at 8bf128d3: the coverage test, and tests/a11y.spec.ts for /data in the light and the dark scheme, 2 passed
B2: red at d85ad2e9: crates/api/tests/data_export_route.rs: each of 6 tests answered 404, the route not merged; crates/daemon/tests/data_export_composed.rs:164: left 404, right 200; src/lib/api.test.ts: the export answered unavailable; src/lib/data/data-screen.test.ts: no download button
B2: green at 8bf128d3
B3: red at d85ad2e9: crates/api/tests/data_export_route.rs: the_route_states_its_body_bound answered 404 where 200, the_route_states_its_rate_bound [404 x 6] where [200 x 6]
B3: green at 8bf128d3
B4: red at d85ad2e9: crates/engine-core/tests/backup_now.rs: 3 of 3 failed, back up now answering Unheld where it makes the backup, refuses the open collection and refuses a file with rows; crates/web-engine/tests/boundary.rs:1442: `fn back_up_now(` occurs 0 times and is not an export; crates/web-engine/tests/files.rs:229: backup_id answered none for every name; src/lib/engine/backups.test.ts: '' where 'backup-3', and `unknown operation backup-now`; src/lib/engine/protocol.test.ts: the operations lack backup-now; data-screen.test.ts: no back up now button
B4: green at 8bf128d3
B5: red at d85ad2e9: crates/ffi/tests/backups.rs: 4 of 4 failed, the list answering [] and back up now and the export NotListed; crates/ffi/src/engine.rs:1045 (lib): every copy is listed while the stage is held, left [], right five server copies
B5: green at 8bf128d3
B6: red at d85ad2e9: data-screen.test.ts: no server age shown; api.test.ts: the snapshot's answers read unavailable
B6: green at 8bf128d3
B7: red at d85ad2e9: data-screen.test.ts: the phone's sentence is no message
B7: green at 8bf128d3
B8: red at d85ad2e9: src/lib/data/data-locales.test.ts: each of the 7 locales lacks the 17 data keys, and the screen calls none of them
B8: green at 8bf128d3
B9: red at d85ad2e9: scripts/tests/test_privacy_policy.py: the iPhone and iPad backups and the web data screen's JSON copy, both unstated
B9: green at 8bf128d3
B10: not red: a guard over the base's 27 rows of the per-transport table, passing at d85ad2e9 and at 8bf128d3
```

## What each pair disclosed

- **The red run is the brief's prediction, measured.** At d85ad2e9 the five Rust targets read 17 failed by name and
  1 passed (the B10 guard): the API's 6, the daemon's 1, the core's 3, the web engine's 2 (one in each of
  `boundary` and `files`), and the adapter's 5 (4 in `backups`, 1 in its `lib`). The web unit tests read 20 failed
  of 73 across six files, and the policy's test failed.
- **The green run.** At 8bf128d3 the core's 7 targets read 51 passed, the web engine's 7 binaries 43, the adapter's
  18 binaries 68, the API's 4 targets 34, the daemon's 2 targets 20 and coordination's 2 targets 5, each with 0
  failed; `cargo fmt --all --check` and `cargo clippy` over the six touched crates with `--all-targets -- -D
  warnings` exit 0. svelte-check reads 0 errors and 0 warnings over 1683 files, and the web app's 116 unit test
  files read 737 passed. The policy's 9 tests, the Swift census, the planned-SPEC census, the public scrub and the
  mutation-row tests pass.
- **The web engine compiles for wasm32.** `cargo check --target wasm32-unknown-unknown -p deck-streak-web-engine`
  exits 0. Clippy over that target reads two findings, both at lines of the base (`credential_classify` and
  `handshake`), none in this delivery's export.
- **The browser leg.** The rendered audit of `/data` ran in the pinned Playwright image with no network: 2 passed,
  the light and the dark scheme, with the snapshot route and the engine's module unanswered, so the audited screen
  shows the server's age as unknown and the engine's refusal.
- **Tests changed after the red commit.** rustfmt reformatted the draft's files; `data_screen_pairs` allows its
  shared support's `expect`; the boundary census stopped spelling the exempt entry's name, which the core's
  containment census refuses outside the entry files; the startapp census lists `/data` among the screens opened by
  path, and the card health census's frozen route files gain `data/+page.svelte`. None weakens a red above.

## Mutation rows

S51600 to S51614 read 15 examined, 15 killed, 0 survived, 0 void at 8bf128d3, each killer passing without its mutant
and failing with it: 14 cargo rows and the policy's script row.

## What only a device or a person proves

- The native data screen (R5, R6, R8): `DataModelTests` on the macOS lane, unmeasured on this box.
- Inside Telegram on iOS and on Android, whether the data screen's download saves (R7): unmeasured; the owner's
  acceptance session records it on each device.
