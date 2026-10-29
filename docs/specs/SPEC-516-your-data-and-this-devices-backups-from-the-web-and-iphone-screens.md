# SPEC-516: your data and this device's backups from the web and iPhone screens: a JSON copy, a backup made now, and the server's backup age

- **Context(s):** `deck-streak-api` (`crates/api`, the export route), `deck-streak-coordination`
  (`crates/coordination`, the one export document), `deck-streak-daemon` (`crates/daemon`, the
  `data` role), `deck-streak-engine-core` (`crates/engine-core`, back up now), `deck-streak-web-engine`
  (`crates/web-engine`), `deck-streak-ffi` (`crates/ffi`), the web app (`web/app`), the iPhone and
  iPad app (`ios/App`), and `repo` (`PRIVACY.md`).
- **Decided by:** ADR-534 (this SPEC's own: the owner's export is reachable from the study clients
  through one owner-session route), under ADR-008 (one database), ADR-020 (the data-rights port),
  ADR-024 (owner sessions behind a host cookie) and ADR-388 (the web sync page drives the choice
  from the Worker and the service answers the snapshot).
- **Cites:** SPEC-021 (export and erase over every context's port; `deckstreakd data export`) and
  SPEC-377 (the web sync page keeps its backups: R15 the list, R16 the export, R17 retention).
- **Beside:** SPEC-503 (the iPhone and iPad client syncs its collection; its download's backup is a
  device backup this SPEC lists).
- **Schematic:** `docs/schematics/your-data-screens.md` (data flow: screen to route to
  `export_all`; screen to Worker to pool to file; screen to adapter to folder to the save dialog).
- **Mutation band:** S51600-S51699. **Models:** none; no model covers a line this delivery changes.

## 1. The problem, measured

At the base:

- The owner's export reaches two surfaces only: the host's `deckstreakd data export`, which writes
  `export_all`'s document as one line of standard output (`crates/daemon/src/role_data.rs`), and
  the bot's `/export`. Neither study client can fetch it: `crates/api/src/router.rs` merges no data
  route, and the web app's route table (`web/app/src/lib/routes.ts`) lists 21 screens, none of them
  a data screen.
- The web app keeps backups (SPEC-377 R15 to R17), but a backup is made only by a full-sync
  choice's write, and the list lives on `/sync` alone. The web engine exports `backups`,
  `export_backup` and `retain` (`crates/web-engine/src/wasm.rs`); none makes a backup on the
  owner's word.
- The iPhone and iPad app makes a backup before a full sync's download (SPEC-503 R4) and a server
  copy for each count, as `backup-<n>-<p>.anki2` and `server-copy-<n>-<p>.anki2` in the
  collection's folder (`crates/ffi/src/engine.rs`, `choice_file`), but lists none, exports none and
  never removes one: SPEC-377 R17's rule runs in the web engine only.
- The server's backup age is answered by `GET /api/sync/snapshot` (SPEC-377 R12 to R14), and no
  screen shows it as an age the owner reads.

## 2. Requirements

- **R1. The data screen is a route.** `/data` is in `ROUTES` and is a screen under
  `web/app/src/routes`, so the coverage test holds the audit over it and the rendered audit runs
  axe over it in both of Telegram's colour schemes.
- **R2. One owner-session export route.** `POST /api/data/export` answers the owner's export.
  Its extractors run in this order, each refusing before the next reads anything: the session
  routes' state-change guard (403 `cross_site_request` or `not_json`); identity's `OwnerSession`
  (401 `no_session`, a `link` session included); the route's own window (429
  `too_many_exports` with `Retry-After`); the body, at most `EXPORT_BODY_LIMIT_BYTES` (413 past
  it); the database (503 `database_not_open`). It then answers 200 with the bytes
  `deckstreakd data export` writes over the same database, the document's one line and its
  newline, from one function both call (`coordination::data_rights_registry::export_document`),
  with `Content-Type: application/json`, `Cache-Control: no-store` and
  `Content-Disposition: attachment; filename="deck-streak-export.json"`. No line it logs names the
  document or a row: an export is logged by the fact alone, and a refusal by its reason code.
- **R3. The route states its bounds.** `EXPORT_BODY_LIMIT_BYTES` is 64 (the request carries `{}`
  or nothing) and sits on the handler, inside the shell's 2 MiB (`ws.request-body-limit`);
  `EXPORTS_PER_MINUTE` is 6 per process in a minute of the kernel's clock, in a window of the
  route's own, so a flood of exports never closes the handshake or the seal release
  (`ws.rate-limit`). The shell's timeout, concurrency bound and request id hold it as every route.
- **R4. Web: back up now.** The data screen's "Back up now" asks the Worker's new `backup-now`
  operation, which takes no argument; the web engine's `back_up_now` export names a new
  Backup-kind pool file through `files.rs` (`choice_file(Kind::Backup)`), and the core's
  `backup_now::back_up` writes the open collection into it with the core's fixed statement and
  reads it back through a private engine, refusing the open collection's own name, a file that
  already holds a row, and a backup that lacks a device id. It refuses while a choice's stage is
  held. It answers the new backup's id alone. The screen then lists the backups again, which runs
  SPEC-377 R17's retention first, so at most three of a kind are kept; the list and each backup's
  export are SPEC-377 R15 and R16's, shown on `/data` by the sync screen's own list.
- **R5. iPhone and iPad: the device backups.** The adapter lists the backups and server copies in
  the collection's folder, newest first by the stamp their names carry, each by an id that is its
  file's stem, its kind and its age in seconds, after the core's retention
  (`deck_streak_engine_core::retention`) removes what the rule removes; it refuses to remove while
  a full-sync choice's stage is held. `back_up_now` makes a backup in that folder through the same
  core function as the web's. `backup_bytes` answers the bytes of a listed backup an id names and
  refuses any other id. The app's data screen lists them, exports one through SwiftUI's
  `fileExporter`, and backs up now. The JSON copy is offered where an owner session exists: the
  native app holds none until a later delivery gives it a device credential, so its screen names
  the web screen and the bot's `/export` instead.
- **R6. The server's backup age, on both screens.** The web screen reads `GET /api/sync/snapshot`
  under the owner's session: found with an age shows that age; not found says the server holds no
  backup yet; `{"found": null}`, a refusal or a failed read shows the age as unknown, in words,
  never as a number. The native screen shows unknown, the same words, while it holds no owner
  session.
- **R7. Inside Telegram on a phone.** Where the web screen runs inside Telegram on iOS or Android
  (the client's `platform` is `ios` or `android`), it says that Telegram may not save a download
  there and names `/export`, which sends the same document in the chat. Whether a download saves
  is measured on the owner's devices (section 9).
- **R8. Locales.** Every string the web data screen shows is a message key in each of the seven
  locale files. Every string the native data screen shows is one entry of its words table
  (`ios/App/Sources/DataWords.swift`), which the view reads and nothing else.
- **R9. The policy.** `PRIVACY.md` states the iPhone and iPad backups: at most three of each kind,
  in the app's own folder on the device, never sent to the service, leaving it only by the
  owner's export through the system's save dialog; and the web screen's JSON copy, the same
  document as `/export`, fetched under the owner's session.
- **R10. No new engine pair.** The per-transport table's rows (`ORDINARY`, `EXEMPT`, `ANSWERED`)
  are the base's, by pair and name: back up now reaches the engine through the core's fixed
  statement on the database door, as the full-sync choice's backup does, never through a pair.

## 3. Acceptance criteria

| id | criterion | red at its tests commit | decided by |
|---|---|---|---|
| B1 | `/data` is routed, so the coverage test and the rendered audit cover it in both schemes | red: `ROUTES` lacks `/data` | `data-screen.test.ts`; `tests/a11y.spec.ts` for `/data` |
| B2 | the route answers the data role's bytes over one database; 401 without a session; 403 cross-site or not JSON before the session is read; 503 without its database | red: no route (404) | `data_export_route`, `data_export_composed` |
| B3 | the body bound and the rate bound are the route's own | red: no route (404) | `data_export_route` |
| B4 | back up now makes one Backup-kind file holding every device id, never into the open collection or a file with rows; the screen lists again after it | red: the stub makes nothing | `backup_now`, `boundary`, `backups.test.ts`, `protocol.test.ts`, `data-screen.test.ts` |
| B5 | the native backups are listed newest first by kind, kept three of each kind by the core's rule, backed up now, and exported only by a listed id | red: the stubs list nothing | `crates/ffi/tests/backups.rs`; `DataModelTests` on the macOS lane |
| B6 | the server's backup age is shown, unknown as unknown | red: no screen | `data-screen.test.ts`; `DataModelTests` on the macOS lane |
| B7 | inside Telegram on a phone the screen names `/export` | red: no screen | `data-screen.test.ts` |
| B8 | every web string is in the seven locales; every native string is in the words table | red: the locales lack the keys | `data-locales.test.ts`; `DataModelTests` on the macOS lane |
| B9 | the policy states the native backups and the JSON copy | red: no line | `test_privacy_policy.py`; the privacy pack's `policy-published` |
| B10 | the per-transport table's rows are the base's, by pair and name (examined 27) | not red: a guard over the base | `data_screen_pairs` |

```acceptance
B1: pnpm --dir web/app exec vitest run src/lib/data/data-screen.test.ts -t "the data screen is routed and audited"
B1: pnpm --dir web/app exec vitest run src/lib/a11y-coverage.test.ts
B1: pnpm --dir web/app exec playwright test tests/a11y.spec.ts -g "/data has no WCAG 2.2 AA violation axe can find"
B2: cargo test -p deck-streak-api --test data_export_route -- --exact the_export_is_the_owners_document_and_is_never_stored
B2: cargo test -p deck-streak-api --test data_export_route -- --exact the_route_refuses_without_an_owner_session
B2: cargo test -p deck-streak-api --test data_export_route -- --exact a_cross_site_or_non_json_request_is_refused_before_the_session
B2: cargo test -p deck-streak-api --test data_export_route -- --exact without_its_database_the_route_answers_unavailable
B2: cargo test -p deck-streak-daemon --test data_export_composed -- --exact the_route_answers_the_bytes_the_data_role_writes
B3: cargo test -p deck-streak-api --test data_export_route -- --exact the_route_states_its_body_bound
B3: cargo test -p deck-streak-api --test data_export_route -- --exact the_route_states_its_rate_bound
B4: cargo test -p deck-streak-engine-core --test backup_now
B4: cargo test -p deck-streak-web-engine --test boundary -- --exact the_backup_now_export_reaches_the_pool_through_files_and_the_core
B4: pnpm --dir web/app exec vitest run src/lib/engine/backups.test.ts -t "back up now"
B4: pnpm --dir web/app exec vitest run src/lib/engine/protocol.test.ts -t "backup-now"
B4: pnpm --dir web/app exec vitest run src/lib/data/data-screen.test.ts -t "back up now"
B5: cargo test -p deck-streak-ffi --test backups
B6: pnpm --dir web/app exec vitest run src/lib/data/data-screen.test.ts -t "the server's backup age"
B7: pnpm --dir web/app exec vitest run src/lib/data/data-screen.test.ts -t "inside Telegram on a phone"
B8: pnpm --dir web/app exec vitest run src/lib/data/data-locales.test.ts
B9: python3 -B -m unittest discover -s scripts/tests -p test_privacy_policy.py -k the_policy_states_the_iphone_backups_and_the_json_copy
B10: cargo test -p deck-streak-engine-core --test data_screen_pairs
```

The native half (R5, R6 and R8's words table) is held on the macOS lane by
`xcodebuild test ... -only-testing:AppTests/DataModelTests`, which no Linux host runs; its rows read
unmeasured until that lane runs them, and the adapter's half is held here by `backups`.

## 4. File manifest

| path | change |
|---|---|
| `docs/specs/SPEC-516-your-data-and-this-devices-backups-from-the-web-and-iphone-screens.md` | new: this SPEC |
| `docs/decisions/ADR-534-the-owners-export-is-reachable-from-the-study-clients-through-one-owner-session-route.md` | new |
| `docs/schematics/your-data-screens.md` | new: the data flow |
| `docs/red-first/SPEC-516.md` | new: the red and green lines |
| `changelog.d/your-data-device-backups-516.md` | new: the fragment |
| `crates/api/src/data_export_routes.rs`, `crates/api/src/lib.rs`, `crates/api/src/router.rs` | the route, its module, its merge |
| `crates/api/tests/data_export_route.rs` | new: B2, B3 |
| `crates/coordination/src/data_rights_registry.rs` | `export_document`, the one line both surfaces answer |
| `crates/daemon/src/role_data.rs` | the role writes `export_document` |
| `crates/daemon/tests/data_export_composed.rs` | new: B2's byte equality |
| `crates/engine-core/src/backup_now.rs`, `crates/engine-core/src/lib.rs`, `crates/engine-core/src/one_way.rs` | new: back up now, over the one-way module's fill check, read and statement |
| `crates/engine-core/tests/backup_now.rs`, `crates/engine-core/tests/data_screen_pairs.rs` | new: B4, B10 |
| `crates/web-engine/src/wasm.rs`, `crates/web-engine/tests/boundary.rs` | the `back_up_now` export; its census, insert-only |
| `crates/ffi/src/backups.rs`, `crates/ffi/src/lib.rs`, `crates/ffi/src/engine.rs` | new: the device backups; the module; the stage's read |
| `crates/ffi/tests/backups.rs` | new: B5 |
| `web/app/src/lib/routes.ts`, `web/app/src/routes/data/+page.svelte` | the route |
| `web/app/src/lib/data/DataScreen.svelte`, `web/app/src/lib/data/data.ts` | new: the screen and its reads |
| `web/app/src/lib/data/data-screen.test.ts`, `web/app/src/lib/data/data-locales.test.ts` | new: B1, B4, B6 to B8 |
| `web/app/src/lib/api.ts` | the export call |
| `web/app/src/lib/engine/protocol.ts`, `protocol.test.ts`, `backups.ts`, `backups.test.ts`, `session.ts`, `client.ts` | the `backup-now` operation |
| `web/app/messages/en.json`, `es.json`, `fr.json`, `ja.json`, `ko.json`, `zh-Hans.json`, `zh-Hant.json` | the new messages |
| `ios/App/Sources/DataModel.swift`, `DataView.swift`, `DataWords.swift` | new: the native data screen |
| `ios/App/Sources/AccountView.swift`, `ios/App/Sources/EngineSession.swift` | the screen's entry and its engine calls |
| `ios/AppTests/DataModelTests.swift`, `ios/swift-roles.json` | new: the macOS lane's tests; the files' roles |
| `PRIVACY.md`, `scripts/tests/test_privacy_policy.py` | R9 and its test |
| `scripts/mutation-rows.d/S51600-S51699.json` | new: section 7's rows |

## 5. What this does NOT cover

- No device credential for the native app: the JSON copy on iPhone and iPad waits for the delivery
  that gives the native app a device token with a closed scope list; until then its screen names the
  web screen and `/export`.
- No Anki package and no CSV: an export in Anki's format is its own delivery, and this one adds no
  engine pair (R10).
- No restore: a backup leaves the device as a file; bringing one back is its own delivery.
- No media in a backup: back up now copies the collection file, as the full-sync choice's backup does.
- No erase from the screens: `/delete` and the host's erase stay the only erase.

## 6. Risks

- **A large export inside the shell's timeout.** The document is built in one read transaction, as
  the data role builds it; the shell's 10-second bound answers 408 past it, and the bot's
  `/export` and the host's role remain.
- **A download the Telegram client refuses.** The screen cannot observe a refused save, so on a
  phone inside Telegram it always names `/export` beside the download.
- **Retention while a choice is held.** Removing the counted server copy under an open stage would
  make the confirm read a missing file; both the web's and the native listing refuse to remove
  while a stage is held, and the native list then lists without removing.

## 7. Mutation rows

Rows `S51600` onward in `scripts/mutation-rows.d/S51600-S51699.json`, each find spelt from the
committed text after `cargo fmt`, once in its file. The table is written with the rows at green.

## 8. Formal models

None: no model's cover names a function this delivery changes. The full-sync choice's model covers
the adapter's `full_sync_count` and `full_sync_confirm`, which this delivery reads beside and does
not change.

## 9. What only a device or a person proves

- Inside Telegram on iOS and on Android, whether the data screen's download saves a file (R7): the
  owner's acceptance session on each device records saves or refused.
- The native data screen, its `fileExporter` and its back up now (R5, R6, R8): the macOS lane's
  `DataModelTests` and a run on a device.
