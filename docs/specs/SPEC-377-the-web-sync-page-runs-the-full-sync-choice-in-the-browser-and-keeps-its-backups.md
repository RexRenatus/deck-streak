# SPEC-377: the web sync page runs the full-sync choice in the browser, its files move through SQLite on `wasm32`, and the owner's backups and the archive's snapshot answer reach the page

- **Campaign row:** the app campaign, SPEC-334 row 1.4 (R5, R8) and its section 9. **Issue:** #631,
  part c (SPEC-357 section 5: "the session's sync, the unsynced status, the choice screen, where
  the backups are stored and how many are kept, and the snapshot answer an upload asks for").
  **Context(s):** `deck-streak-engine-core` (`crates/engine-core`), `deck-streak-web-engine`
  (`crates/web-engine`), `miniapp` (`web/app/src`), and in part c2 `deck-streak-api` (`crates/api`)
  and the daemon's composition (`crates/daemon`); the engine fork (its pin, ADR-058, ADR-348).
- **Decided by:** ADR-388 (this SPEC's own), under ADR-375 (D2: the full sync's files move through
  SQLite on `wasm32`; D6: each delivery pins the fork patch it proves; D7: no time driver on
  `wasm32`; D8: nothing the choice makes is deleted by the choice; D19: the request's total
  timeout), ADR-368 (D6: the unsynced read; D5 and D10 left to this part), ADR-374 (D10: the sign-in
  form posts to the Worker once), ADR-337 (a one-way sync is the owner's tap), ADR-058 and ADR-348
  (the fork and its pin).
- **Schematic:** `docs/schematics/web-sync-page.md` (this delivery adds it: the page, the Worker,
  the web engine, the core and the fork in sequence).
- **Status:** part c1 of two. This pull request (c1) carries SPEC-364 section 7's part b3 (the
  full-sync files on `wasm32`, the web engine's choice exports, the restore after eviction) and
  builds the page's sync screen: the session's sync, the sign-in, the unsynced count and the choice
  screen. Part c2 (the snapshot route, the backups' list, export and retention, and the storage
  status) is section 7. **Mutation band:** S37700-S37799 (section 10). **Model:**
  `formal/tla/FullSyncChoice`, unchanged (section 8).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `f48a977c` (`D` below) or in the engine fork at its pin,
commit `2cfa7047` (`F` below, the fork's own tree). Nothing was run: every figure is a read.

### 1.1 The full sync's file-system calls on `wasm32`

| id | measured | figure | command |
|---|---|---|---|
| M1 | A full upload reads the collection's file with the standard library | `let col_data = fs::read(&col_path)?;` (line 49), after `before_upload` and `close(Some(SchemaVersion::V18))` | `git show F:rslib/src/sync/collection/upload.rs \| sed -n '44,49p'` |
| M2 | A full download writes a temporary file, opens it, and renames it over the collection | `new_tempfile_in_parent_of` (line 37), `write_file` (38), a builder with `set_check_integrity(true)` (39-41), `update col set ls=mod` (42), `atomic_rename(temp_file, &col_path, true)` (44) | `git show F:rslib/src/sync/collection/download.rs \| sed -n '27,46p'` |
| M3 | Those helpers are the standard library's file calls | `std::fs::write` (lines 52-53), `NamedTempFile::new_in` (242-247), `sync_all`, `persist` and the parent's `sync_all` (256-284) | `git show F:rslib/io/src/lib.rs \| sed -n '50,55p;240,286p'` |
| M4 | Both directions' transfer ticks a timer | `interval(Duration::from_millis(100))` (line 30) in `full_sync_progress_monitor`, which `download_with_progress` (56) and `upload_with_progress` (69) run | `git show F:rslib/src/sync/http_client/full_sync.rs \| sed -n '21,82p'` |
| M5 | Every choice reaches M2 before the owner taps: the counted server copy and an upload's fresh copy are fetched by a private engine's full download | `fetched` runs `private.full_sync(&request)`; `count` and `recheck` call it | `git show D:crates/engine-core/src/one_way.rs \| grep -n -E 'fn (fetched\|count\|recheck)\|full_sync\('` |
| M6 | The file calls sit inside the engine's own functions, with no hook a caller can supply | `full_upload_with_server` and `full_download_with_server` are `pub(super)` (upload line 44, download line 27); `before_upload`, eleven statements an upload runs first, is `pub(crate)` | `git show F:rslib/src/collection/mod.rs \| sed -n '192,205p'` |
| M7 | The transport an in-repository path would need is public | `HttpSyncClient` and `new` are `pub` (lines 27, 37); `SyncProtocol::upload` and `download` are trait methods (lines 90-91) | `git show F:rslib/src/sync/http_client/mod.rs \| grep -n pub`; `git show F:rslib/src/sync/collection/protocol.rs \| grep -n -E 'async fn (upload\|download)'` |
| M8 | The core probes the file system itself on the choice's paths | `path.exists()` (line 190) in `fillable`; `open.canonicalize()` and `path.canonicalize()` (line 342) in `opens` | `git show D:crates/engine-core/src/one_way.rs \| grep -n 'exists('`; `git show D:crates/engine-core/src/dispatch.rs \| grep -n canonicalize` |
| M9 | On `wasm32` the standard library has no file system, so M1 to M3 fail and M8 answers "absent" and "no other spelling" | ADR-375's rejected alternative, line 67: "`wasm32-unknown-unknown` has no file system; the calls fail" | `git show D:docs/decisions/ADR-375-the-web-client-syncs-from-its-worker.md \| grep -n 'has no file system'` |
| M10 | The pool knows its own files | `pool.list()` decides `existed` in `open` (lines 170-173) | `git show D:crates/web-engine/src/wasm.rs \| sed -n '165,186p'` |
| M11 | The pin and its rules | `[patch]` takes `anki` and `anki_proto` by `rev` `2cfa7047` (lines 171-173); the manifest's comment names the browser's sync transport and its two companion `wasm32` patches, one commit each (line 170), and ADR-058's notes record each patch as a row | `git show D:Cargo.toml \| sed -n '165,173p'` |

### 1.2 What the web client holds

| id | measured | figure | command |
|---|---|---|---|
| M12 | The web engine exports no choice call and no unsynced read | 27 `#[wasm_bindgen]` items; none names `full_sync` or `unsynced` | `git show D:crates/web-engine/src/wasm.rs \| grep -c '^#\[wasm_bindgen'` |
| M13 | The core's unsynced read exists and answers offline | `Dispatcher::unsynced` (line 441), one statement (ADR-368 D6) | `git show D:crates/engine-core/src/dispatch.rs \| grep -n 'pub fn unsynced'` |
| M14 | The Worker's protocol has 19 operations and none for the choice | `OPS` (lines 13-33) | `git show D:web/app/src/lib/engine/protocol.ts \| sed -n '13,33p'` |
| M15 | No page module calls the sync or the sign-in | 0 lines | `git grep -n -E '\.sync\(\)\|syncLogin\(' D -- web/app/src/routes web/app/src/lib/study web/app/src/lib/sync` |
| M16 | The page has no sync screen and no settings screen | `ROUTES` holds 16 paths, neither among them (lines 10-27) | `git show D:web/app/src/lib/routes.ts \| sed -n '10,27p'` |
| M17 | The sign-out exists and no screen calls it | `signOut` (line 28) | `git show D:web/app/src/lib/sync/sign-out.ts \| grep -n 'export async function signOut'` |
| M18 | Every session start asks for persistent storage and keeps no answer | `void this.#persist().catch(...)` (line 56) | `git show D:web/app/src/lib/study/engine.ts \| sed -n '50,63p'` |
| M19 | A collection the browser evicted opens as new and says so | `open` answers `{"existed": false, ...}` | M10's command |
| M20 | The accessibility run covers every path `ROUTES` lists | `for (const route of ROUTES)` (line 295) | `git show D:web/app/tests/a11y.spec.ts \| sed -n '295,300p'` |

### 1.3 What the archive holds (part c2)

| id | measured | figure | command |
|---|---|---|---|
| M21 | The archive job copies a sealed archive and its sealed manifest per stamp, by a command and a destination its settings name | `ARCHIVE_NAME` (line 54), `STAMP_FORMAT` (47), the sealed suffix (236-240), the copy (290) | `git show D:deploy/scripts/backup.py \| sed -n '45,63p;250,300p'` |
| M22 | The archive's destination and credential are filled at deployment, never held in the tree | the unit is a template with neutral values (lines 12-13) | `git show D:deploy/systemd/deck-streak-sync-snapshot.service \| sed -n '1,14p'` |
| M23 | The service's owner-session route has a shape to follow | the off arm, the state-change guard, `OwnerSession`, a rate window of its own and a body limit (the module comment's numbered list from line 9; `release` takes `StateChange` and `OwnerSession`, lines 89-92) | `git show D:crates/api/src/sync_seal_routes.rs \| sed -n '9,20p;89,92p'` |

**What follows.** A full sync on the web fails at its first file call (M1, M2, M3), and its timer
panics before that (M4); every choice reaches the download's calls before the owner's first tap
(M5). No path inside this repository moves the bytes in memory without a fork edit: the calls sit
inside the engine's own functions (M6), and a web-engine path over the public transport (M7) would
skip the core's one-way request and `before_upload`, which this repository cannot call. ADR-375 D2
already decided the cure, a fork patch; this part is the delivery that proves it. Beside the fork,
the core's own two probes (M8) answer wrongly on `wasm32` (M9) without failing, so the rule "a
server copy is fetched only into an empty file, never into the open collection" would hold natively
and silently not in the browser; the pool knows its files (M10). And the page has nothing to show:
no choice export (M12), no operation (M14), no caller (M15), no screen (M16, M17).

## 2. Requirements (part c1)

R1. **The files on `wasm32`.** A fork patch, `browser-full-sync-files`: on `wasm32` an upload reads
    the closed collection through SQLite's serialize, never the standard library's file read; a
    download deserializes the received bytes into memory, checks their integrity, sets `ls` to
    `mod`, and replaces the collection at its path with SQLite's backup interface in one
    transaction, through the pool the web engine installs as the default file system; the full
    sync's progress monitor never ticks. The binding's serialize and backup features are on for
    `wasm32` alone, and the native engine is unchanged (its own tests equal before and after).
R2. **The pin.** R1 is one commit on the fork's pin branch over `2cfa7047`, tagged with a new tag;
    every earlier tag stays (ADR-375 D6, ADR-348). The root manifest's `[patch]` `rev` moves to it,
    `Cargo.lock` moves only by `cargo update -p anki -p anki_proto`, and ADR-058 gains its row and
    a note in its earlier notes' shape. The fork write is made only under the grant ADR-388 D6
    names.
R3. **The population is measured before it is frozen.** Before the fork commit, one wide-open
    trial runs A1 to A3 in the browser and records every standard-library call that fails on their
    path, in the fork or in this repository. R1 and R4 cover exactly what it records; a site beyond
    M1 to M4 and M8 stops the delivery for a decision.
R4. **Where files live is the adapter's.** The core reads whether a path holds a file, and whether
    two paths name one file, through a `Files` port the dispatcher keeps. The native default is the
    standard library's, with today's behaviour. On `wasm32` the default refuses every choice path
    as the open collection until an adapter installs its own port, and the web engine installs the
    pool's at start: a file the pool lists, compared by its one name.
R5. **The web engine's choice exports.** `full_sync_count`, `full_sync_confirm`,
    `full_sync_cancel` and `unsynced`. They hold one choice's stage between the owner's taps, call
    only `one_way`'s functions and the dispatcher's unsynced read, and take no path, id set or
    snapshot answer from the page. `full_sync_confirm` is the owner's tap: it builds the one-way
    gesture and runs the backup and, for a download, the write; for an upload, the snapshot answer
    the Worker read, the re-check and the write. Each answers the counts, a status word, or that
    the counts changed and why. Every path is a new pool file the web engine names, never the
    collection's and never one the pool already lists, and the pool is reserved before each. A
    Worker that ends drops the stage, which is the model's `Cancel`.
R6. **The Worker's choice operations.** `choice-count`, `choice-confirm` (`direction`),
    `choice-cancel` and `unsynced`. Each choice send takes its key from the credential store and
    settles it, as the normal sync does (ADR-375 D17). Before an upload's confirm, the Worker reads
    the snapshot answer from the service's same-origin route with the owner's session; an absent
    route, a refusal or a network failure is `unknown`, and `unknown` refuses the upload as a
    snapshot not found does. No reply carries a key, a password, a host key, a path or an id.
R7. **The session's sync.** The study engine posts one `sync` when a session starts (the
    collection opened) and one when it ends (before its close), and none between them; it awaits
    neither before it sends a study request (the Worker's one queue orders them, bounded by the
    request's timeout, ADR-375 D19).
R8. **The sync screen.** A route of its own, `/sync`: a form that posts the sync user and password
    to the Worker once and keeps neither (ADR-374 D10), the five status words in the owner's words,
    the sign-out, the unsynced count from the Worker's reply (offline too), and, when a sync answers
    a full sync required, the choice.
R9. **The choice screen.** The counts per offered direction (reviews, cards and notes the replaced
    side loses), a direction not offered absent, neither direction preselected, one tap to confirm
    a direction and a cancel at every step before the write; for an upload, the snapshot's age
    before the tap, or that it is unknown and the upload waits; a changed server or a new device
    review returns to the counts with a sentence saying why.
R10. **The web client's storage promise.** Every session start
    keeps asking for persistent storage; the server's copy stays authoritative, so a collection the
    browser evicted is offered the download alone and is never uploaded over the server; and the
    sync screen says the browser lost its copy and restores it by a download the owner taps.
R11. **Locales.** Every string the screens show is a message key in each of the seven locale files.

## 3. Acceptance criteria of part c1

The browser proofs run against the engine's own sync server through `playwright.engine.config.ts`,
as SPEC-364 part b2's do. An upload's snapshot answer comes from a test route that stands in for
part c2's.

| id | criterion | red at its tests commit | decided by |
|---|---|---|---|
| A1 | In Chromium and WebKit, an upload with no found snapshot is refused and with one is written; a review made in the browser is on the server afterwards (SPEC-364 section 17 (i)) | red: M1's file read and M4's timer | `sync.spec.ts`, "an upload waits for its snapshot" |
| A2 | In Chromium and WebKit, a download replaces the browser's collection with the server's after a backup that opens with every device review | red: M2's temporary file | `sync.spec.ts`, "a download is written after its backup" |
| A3 | In Chromium and WebKit, a collection the browser evicted is offered the download alone and restores every server review on the owner's tap | red: M2 | `sync.spec.ts`, "an evicted collection is restored from the server" |
| A4 | The core judges a choice path by the installed `Files` port: a port that names the path as the open collection refuses it, a port that lists it gets its rows read, the standard port finds the open collection by another spelling, and the fail-closed default refuses every choice path | red: a stub port the core never asks, while it reads `exists()` and `canonicalize()` (M8) | the four `one_way` tests the fence names |
| A5 | The pool's port holds only the names the pool lists, and every choice file is a new name, never the collection's | red: a stub port that holds every name | `crates/web-engine/tests/files.rs` |
| A6 | The choice exports reach the engine only through `one_way` and the dispatcher, take no path, id set or snapshot answer, and the web engine installs the pool as the core's port | red: the census's entries before the exports | the `boundary` census |
| A7 | The Worker's choice settles every send, reads the snapshot answer itself, and refuses an upload on `unknown` | red: a stub that passes the page's answer | `choice.test.ts` |
| A8 | The choice operations carry no path, id set or snapshot answer, and anything else is refused before any engine call | red: the operations are unknown | `protocol.test.ts`, "the choice operations carry no path, no id set and no snapshot answer" |
| A9 | A session syncs once at its start and once at its end, and sends its study requests without awaiting either | red: no caller (M15) | `engine.test.ts`, "a session syncs at its start and end" |
| A10 | The sign-in posts the user and password once and keeps neither, and each status word reads as the owner's words | red: no screen (M16) | `sync-screen.test.ts`, "the sign-in posts once and shows each status word" |
| A11 | The unsynced count is the Worker's reply, shown offline | red: a stub that counts in the page | `sync-screen.test.ts`, "the unsynced count is the worker's" |
| A12 | The choice screen shows only offered directions with their counts, preselects neither, shows an upload's snapshot age or that it is unknown, cancels at every step, and returns to the counts with a sentence | red: a stub that shows both directions | `choice-screen.test.ts` |
| A13 | A collection the browser lost is said so on the sync screen and offered the download alone | red: the screen says nothing of it | `sync-screen.test.ts`, "a collection the browser lost offers the download alone" |
| A14 | `/sync` passes the page's accessibility run | red: the stub form's unlabelled fields | `a11y.spec.ts`, its `/sync` row |
| A15 | Each of the seven locales holds every key the screens use | red: six locales lack the keys | `sync-locales.test.ts` |
| A16 | No reply carries a key, a password or a host key across the new operations | not red: an absence census, held by its planted controls | `credential-reach.test.ts` |

```acceptance
A1: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "an upload waits for its snapshot"
A2: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "a download is written after its backup"
A3: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "an evicted collection is restored from the server"
A4: cargo test -p deck-streak-engine-core --test one_way -- --exact the_choice_paths_are_judged_by_the_installed_files_port
A4: cargo test -p deck-streak-engine-core --test one_way -- --exact a_path_the_files_port_holds_is_read_for_rows
A4: cargo test -p deck-streak-engine-core --test one_way -- --exact the_open_collection_by_another_spelling_is_refused
A4: cargo test -p deck-streak-engine-core --test one_way -- --exact an_uninstalled_browser_port_refuses_every_choice_path
A5: cargo test -p deck-streak-web-engine --test files
A6: cargo test -p deck-streak-web-engine --test boundary
A7: pnpm --dir web/app exec vitest run src/lib/engine/choice.test.ts
A8: pnpm --dir web/app exec vitest run src/lib/engine/protocol.test.ts -t "the choice operations carry no path, no id set and no snapshot answer"
A9: pnpm --dir web/app exec vitest run src/lib/study/engine.test.ts -t "a session syncs at its start and end"
A10: pnpm --dir web/app exec vitest run src/lib/sync/sync-screen.test.ts -t "the sign-in posts once and shows each status word"
A11: pnpm --dir web/app exec vitest run src/lib/sync/sync-screen.test.ts -t "the unsynced count is the worker's"
A12: pnpm --dir web/app exec vitest run src/lib/sync/choice-screen.test.ts
A13: pnpm --dir web/app exec vitest run src/lib/sync/sync-screen.test.ts -t "a collection the browser lost offers the download alone"
A14: pnpm --dir web/app exec playwright test tests/a11y.spec.ts -g "/sync has no"
A15: pnpm --dir web/app exec vitest run src/lib/sync/sync-locales.test.ts
A16: pnpm --dir web/app exec vitest run src/lib/engine/credential-reach.test.ts
```

Existing tests grow insert-only, each population printed and no assertion weakened:
`crates/engine-core/tests/one_way.rs`, `crates/engine-core/tests/containment.rs` (if the exports
add an entry call), `crates/web-engine/tests/boundary.rs`, `web/app/src/lib/engine/session.test.ts`,
`web/app/src/lib/engine/client.test.ts` and `web/app/src/lib/engine/credential-reach.test.ts`.

## 4. File manifest (part c1)

| path | change |
|---|---|
| `docs/specs/SPEC-377-the-web-sync-page-runs-the-full-sync-choice-in-the-browser-and-keeps-its-backups.md` | this SPEC |
| `docs/decisions/ADR-388-the-web-sync-page-drives-the-choice-from-the-worker-and-the-service-answers-the-snapshot.md` | its ADR |
| `docs/schematics/web-sync-page.md` | the schematic (new) |
| `docs/red-first/SPEC-377.md` | the red-first record |
| `docs/specs/SPEC-364-the-web-client-syncs-from-its-worker.md` | one line, insert-only, at the end of section 17 ("What part b3 owes after part b2"): part b3 is delivered by SPEC-377 part c1 |
| `docs/decisions/ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md` | the new patch's row and pin note, insert-only |
| `Cargo.toml` | the `[patch]` `rev` and its comment |
| `Cargo.lock` | by `cargo update -p anki -p anki_proto` alone |
| `crates/engine-core/src/files.rs` | the `Files` port, the standard library's port, the `wasm32` default (new) |
| `crates/engine-core/src/lib.rs` | `pub mod files;` |
| `crates/engine-core/src/dispatch.rs` | the dispatcher keeps the port and takes an installed one; `opens` (its name and signature kept) and a new `holds` ask it |
| `crates/engine-core/src/one_way.rs` | `fillable` asks the port |
| `crates/engine-core/tests/one_way.rs` | A4 |
| `crates/engine-core/tests/containment.rs` | grows insert-only if the exports add an entry call |
| `crates/web-engine/src/files.rs` | the pool's names: whether the pool holds a name, one name per file, and new choice names (new; compiled natively, like `study.rs`) |
| `crates/web-engine/src/lib.rs` | `pub mod files;` |
| `crates/web-engine/src/wasm.rs` | the pool's port installed; the four exports; no line above the pool install moves |
| `crates/web-engine/tests/files.rs` | A5 (new) |
| `crates/web-engine/tests/boundary.rs` | A6 |
| `web/app/src/lib/engine/protocol.ts`, `web/app/src/lib/engine/protocol.test.ts` | the four operations; A8 |
| `web/app/src/lib/engine/choice.ts`, `web/app/src/lib/engine/choice.test.ts` | the Worker's choice and its snapshot read (new); A7 |
| `web/app/src/lib/engine/session.ts`, `web/app/src/lib/engine/session.test.ts` | the operations reach the choice |
| `web/app/src/lib/engine/worker.ts` | the choice composed beside the sync |
| `web/app/src/lib/engine/client.ts`, `web/app/src/lib/engine/client.test.ts` | the client's four calls |
| `web/app/src/lib/engine/credential-reach.test.ts`, `web/app/src/lib/engine/credential-stand-in.test.support.ts` | A16 grows over the new operations |
| `web/app/src/lib/study/engine.ts`, `web/app/src/lib/study/engine.test.ts` | the session's two syncs; A9 |
| `web/app/src/lib/sync/status.ts` | the page's sync state, held from the Worker's replies (new) |
| `web/app/src/lib/sync/SyncScreen.svelte`, `web/app/src/lib/sync/ChoiceScreen.svelte` | the screens (new) |
| `web/app/src/lib/sync/sync-screen.test.ts`, `web/app/src/lib/sync/choice-screen.test.ts`, `web/app/src/lib/sync/sync-locales.test.ts` | A10 to A13, A15 (new) |
| `web/app/src/routes/sync/+page.svelte` | the route (new) |
| `web/app/src/lib/routes.ts` | `/sync` in `ROUTES` |
| `web/app/src/lib/startapp.test.ts` | `/sync` joins the screens opened by their path alone (`BY_PATH`); no startapp token opens it |
| `web/app/messages/en.json`, `web/app/messages/es.json`, `web/app/messages/fr.json`, `web/app/messages/ja.json`, `web/app/messages/ko.json`, `web/app/messages/zh-Hans.json`, `web/app/messages/zh-Hant.json` | the screens' keys |
| `web/app/tests-engine/sync.spec.ts`, `web/app/engine-harness/main.ts` | A1 to A3 and the harness's choice calls |
| `web/app/vite.engine.config.ts` | the snapshot route's stand-in beside the release's, with a test mode that sets its answer |
| `web/app/playwright.engine.config.ts` | the engine's sync server keeps a second synthetic account, so each browser project syncs against a server collection of its own |
| `scripts/mutation-rows.d/S37700-S37799.json` | rows S37700 to S37711 (new) |
| `scripts/mutation-rows.d/S36400-S36499.json` | `S36413` re-anchored under its own id on the holds arm's new line, its killer unchanged (section 10) |
| `changelog.d/web-sync-page-377.md` | the fragment (new) |

The fork's own files, outside this repository: `rslib/src/sync/collection/upload.rs`,
`rslib/src/sync/collection/download.rs`, `rslib/src/sync/http_client/full_sync.rs` and the
manifest that turns the binding's features on for `wasm32`.

## 5. What this does NOT cover

- It fills no media directory and syncs no media: the one-way request keeps `server_usn` empty, so
  no background media sync starts (part d, #631).
- It builds no iOS screen; the iOS client installs its own `Files` port when its sync lands (#633).
- It provisions nothing: the archive's list-only grant is the owner's step (#161).
- It builds no snapshot route, no backup list, export or retention, and no storage status: part c2,
  section 7 (#631).
- It does not close the window between the re-check and an upload at the server (#617).
- It counts no edit to a row both sides hold (#620).
- It does not drop the engine fork; its removal conditions stand (#233).
- It proves nothing on a device; the owner's acceptance session does (#637).

## 6. Risks

- **A further unsupported site.** A1 to A3's path may meet a standard-library call M1 to M4 and M8
  do not name, as part b2's met the sync meta's size read. R3's wide-open trial records them before
  the fork commit, and a new site stops the delivery for a decision, so no patch population is
  frozen on a guess.
- **The pool's room.** Each choice file is a new pool file. R5 reserves the pool before each, and A2
  asserts the backup opens with every device review, so a full pool fails before any write.
- **A full sync past the request's bound.** The total timeout stays the engine's stall duration
  (ADR-388 D12). A transfer past it fails as a timeout and writes nothing; the screen says so, and
  the counts are read again.
- **The native engine.** R1 is a `cfg` on the target. The fork's native tests run before and after
  the commit with equal results; a difference stops the delivery.
- **The threat model's cited lines.** A pull request that cites `worker.ts`, `wasm.rs` or
  `protocol.ts` line by line breaks when those lines move. The builder keeps new code below the
  cited lines, and re-measures the citations if that record is on `dev` at the cut.
- **Rows on the files this part edits.** `S36413` moves with its line (section 10). The new exports
  map an engine refusal through the existing function, so `S36438`'s find stays unique, and the
  builder's census at its cut proves every row on `dispatch.rs`, `one_way.rs` and `wasm.rs` keeps
  one occurrence before the rows commit.
- **A port left uninstalled.** The `wasm32` default refuses every choice path, so a web engine that
  forgets the pool's port fails A1 to A3 loudly instead of writing over a file.

## 7. Delivered by the next pull request

Part c2 is built after c1, on the same SPEC and ADR, and amends both insert-only.

R12. **The snapshot answer.** `GET /api/sync/snapshot`, behind the owner's session (401 without
    one), in a rate window of its own, `Cache-Control: no-store`, answers `{"found": true,
    "age_seconds": n}`, `{"found": false}` or `{"found": null}` (unknown). It never names an
    object, a path or a prefix, in an answer or a log line.
R13. **What found means.** A sealed archive and its sealed manifest at one stamp, in the archive's
    listing (M21); the age is now less the newest such stamp, and no bound is set (ADR-388 D2).
R14. **The listing.** The route reads the listing through a port; the composition root wires a
    configured list command, run with arguments and no shell, bounded in time and output, under a
    list-only credential named only by its role. No command or credential configured, a refusal,
    or a timeout answers `unknown`. The answer is cached briefly.
R15. **The backups' list.** The sync screen lists the device backups and the counted server copies
    by kind and age, newest first, from the web engine's read of the pool.
R16. **The export.** One backup leaves the browser as a file the owner keeps, named by kind and
    age; the bytes reach the page by transfer; the page names a backup only by an id the Worker
    listed, and the web engine refuses any other.
R17. **Retention.** One rule in the core: the newest three of each kind are kept and the newest of a
    kind is never removed. It runs in the Worker only when no choice is held, after a write or a
    cancel, and each removal is named on the screen.
R18. **Storage status.** The sync screen shows whether persistent storage was granted, from the
    answer the session start already asks for, and that an evicted collection is restored from the
    server by a download the owner taps.
R19. **The policy.** `PRIVACY.md` says the browser keeps its collection and its backups on the device
    alone, at most three of each kind, and that they leave it only by the owner's export.

| id | criterion | delivered by |
|---|---|---|
| B1 | The route answers found and an age from a listing, refuses without an owner session, and names no object | `cargo test -p deck-streak-api --test snapshot_route`; red: a stub that answers found always |
| B2 | With no list command or credential configured, the route answers unknown, and the upload stays refused | `... --test snapshot_route -- --exact an_absent_list_credential_answers_unknown`; red: the stub answers found |
| B3 | An archive without its manifest is not found, and the age is the newest sealed stamp's | `... --test snapshot_route`; red: the stub |
| B4 | Retention never removes the newest of a kind, keeps three of each, and keeps the kinds apart | `cargo test -p deck-streak-engine-core --test retention`; red: a stub that removes the oldest, the newest included |
| B5 | The backups are listed by kind and age, newest first, and only a listed backup is exported | `cargo test -p deck-streak-web-engine --test files`; `vitest run src/lib/sync/backups.test.ts`; red: the stub exports any name |
| B6 | In Chromium and WebKit, a download leaves its backup listed, and its export is a file the engine opens with every device review | `playwright ... sync.spec.ts -g "a backup is listed and exported"`; red: the list is absent |
| B7 | The storage status reads the persistence answer, and the sync screen's backup list passes the accessibility run | `vitest run src/lib/sync/sync-screen.test.ts -t "the storage status is the browser's answer"`; the `/sync` accessibility row; red: no status |
| B8 | The policy names the browser's copies and their bound | the repository's privacy probe over `PRIVACY.md`; red: no line names them |

## 8. Formal model

`formal/tla/FullSyncChoice` is unchanged. Part c1 changes no `@phx covers` anchor: `full_sync.rs` is
untouched, and the core's edits are in `fillable` and `opens`, which the model abstracts as the
places a backup and a server copy are made. The web engine calls `snapshot_found` with the answer
the Worker read, which the model's `SnapCheck` takes as `found` alone; `unknown` is not found, so
`SnapshotBeforeUpload` states it already. A Worker that ends drops the stage before the write,
which is the model's `Cancel`. Part c2's retention removes places `NoReviewLost` counts, but never
while a choice is held and never the newest of a kind, so the last choice's places hold over the
model's one-choice horizon. No freshness bound is set (ADR-388 D2); a bound would enter the model
first, with a witness of an upload past it.

## 9. What only a device or a person proves

SPEC-364's V1 and V2, SPEC-357's V1 to V3, and a full sync of the owner's own collection from the
owner's own browser, by the owner in the acceptance session (#637).

## 10. Mutation rows (part c1)

Band `S377`, rows `S37700` to `S37711` in `scripts/mutation-rows.d/S37700-S37799.json`, in the
`MUTATIONS` table. Each find is spelt from the committed text after `cargo fmt`, keeps every item
used, and occurs once in its file. The TypeScript and Svelte are held by StrykerJS with `break` 100
over each changed production file, with no hand rows. The fork carries no rows of this repository.
The `wasm32`-only code of `wasm.rs` is held by the `boundary` source census, its one native killer.

One existing row moves with its code, under its own id (the rule: a row the change makes wrong is
re-anchored, never retired): `S36413-A-COPY-GOES-ONLY-INTO-AN-EMPTY-FILE`, whose find
`if path.exists() && holds_a_row(&read(dispatcher, path)?) {` becomes the holds arm's new line,
asking the dispatcher's port, with its replacement the same line ending `&& false {` and its killer
`one_way::a_server_copy_is_fetched_only_into_an_empty_file` unchanged. `S36412`'s find,
`if dispatcher.opens(path) {`, is kept byte for byte: `opens` keeps its name and asks the port
inside it. No other row at `dev` anchors on a line this part rewrites (23 rows on `dispatch.rs`, 9 on
`one_way.rs`, 11 on `wasm.rs`, none on `opens`' body); each keeps one occurrence after the edit.

| row | crate, file | mutant | killer |
|---|---|---|---|
| `S37700-THE-STANDARD-PORT-HOLDS-NOTHING` | engine-core `src/files.rs` | the standard port's `holds` ends `&& false` | `one_way::a_server_copy_is_fetched_only_into_an_empty_file` |
| `S37701-THE-STANDARD-PORT-KNOWS-ONE-SPELLING` | engine-core `src/files.rs` | the standard port's canonical arm reads `false &&` before it | `one_way::the_open_collection_by_another_spelling_is_refused` |
| `S37702-THE-BROWSER-DEFAULT-ADMITS-A-PATH` | engine-core `src/files.rs` | the fail-closed port's `same` answers `false` | `one_way::an_uninstalled_browser_port_refuses_every_choice_path` |
| `S37703-THE-POOL-HOLDS-AN-UNLISTED-NAME` | web-engine `src/files.rs` | the pool's `holds` ends `\|\| true` | `files::the_pool_holds_only_the_names_it_lists` |
| `S37704-THE-POOL-HOLDS-NOTHING` | web-engine `src/files.rs` | the pool's `holds` ends `&& false` | `files::the_pool_holds_only_the_names_it_lists` |
| `S37705-THE-POOL-NEVER-FINDS-THE-COLLECTION` | web-engine `src/files.rs` | the pool's `same` ends `&& false` | `files::the_pool_finds_the_collection_by_its_one_name` |
| `S37706-A-TAKEN-NAME-IS-REUSED` | web-engine `src/files.rs` | the new name's check against the listed names reads `false &&` before it | `files::a_choice_file_is_never_a_name_the_pool_holds` |
| `S37707-A-CHOICE-FILE-IS-THE-COLLECTION` | web-engine `src/files.rs` | the new name's check against the collection's name reads `false &&` before it | `files::a_choice_file_is_never_the_collection` |
| `S37708-THE-POOL-PORT-IS-NEVER-INSTALLED` | web-engine `src/wasm.rs` | the install becomes `let _ = &port;` | `boundary::the_web_engine_installs_the_pool_as_the_cores_files_port` |
| `S37709-AN-UPLOAD-FINDS-A-SNAPSHOT-ALWAYS` | web-engine `src/wasm.rs` | the confirm passes `found: found \|\| true` to the snapshot check in place of the Worker's answer | `boundary::the_snapshot_answer_reaches_the_core_as_the_worker_read_it` |
| `S37710-A-CANCEL-KEEPS-THE-STAGE` | web-engine `src/wasm.rs` | the cancel's clearing of the held stage becomes `let _ = &stage;` | `boundary::a_cancel_drops_the_held_stage` |
| `S37711-A-CHOICE-FILE-IS-NAMED-UNRESERVED` | web-engine `src/wasm.rs` | the pool reserve before a choice file becomes `let _ = (&pool, storage("the pool is not reserved"));` | `boundary::each_choice_file_is_reserved_before_it_is_named` |

## 11. Requirements (part c2), as built

Section 7's R12 to R19 stand; this section says how part c2 builds each, and adds nothing to them.

R12. `GET /api/sync/snapshot` is merged beside the seal route and keeps its order: an off arm while
    no lister is wired, answering 200 `{"found": null}` (never 404, so an upload stays refused and
    the page can say why); then `OwnerSession` (401 `no_session`); then a rate window of its own
    (429 with `Retry-After`). Every answer carries `Cache-Control: no-store`. A GET changes no
    state, so the route takes no state-change guard. No answer and no log line names an object, a
    path or a prefix: the only log lines are the refusal's reason word and the found word.
R13. The route reads a listing's names by their last path segment. A stamp is found only when the
    listing holds both `sync-<stamp>.tar.age` and `sync-<stamp>.sha256.age`, the stamp spelt
    `YYYYMMDDTHHMMSSZ` with a real date and time; any other name is ignored. `age_seconds` is now
    less the newest found stamp, never below 0.
R14. The service's crate holds a `SnapshotLister` port and answers `unknown` when none is wired, or
    when the port answers nothing. The last answer is kept for a window of 60 seconds and listed
    again after it. The daemon wires a lister only when both the list command's setting and the
    archive's list-only credential are present: the setting's words are the program and its
    arguments, split on whitespace and run with no shell; the credential is passed as the path of
    its file in the service's credential directory, as the last argument, never as an environment
    line. The command is killed at 8 seconds, its output is read to 65536 bytes, and a refusal, a
    non-zero exit, a kill or an excess answers `unknown`. Nothing is configured in the tree.
R15. The web engine's `backups` export lists the pool's backups and server copies newest first, each
    by an id that is its pool name's stem (`backup-2`), its kind and its age in seconds. The pool
    keeps no file times, so the web engine records when it first lists each file, as an empty pool
    entry beside it (ADR-388 D17); the order is that record, then the minted number.
R16. `export_backup` answers the bytes of the listed backup an id names, and refuses any id the pool
    does not list as a backup. The Worker posts the bytes to the page in the reply's transfer list;
    the page makes a Blob and an anchor whose `download` is `deck-streak-<kind>-<age>s.anki2`.
R17. `deck_streak_engine_core::retention` is the one rule: `KEEP` is 3, a file is removed once
    `KEEP` newer files of its kind exist, the newest of a kind is never removed, and the kinds are
    counted apart. The web engine's `retain` export applies it and refuses to while a choice's stage
    is held. The Worker runs it before each backup list, which the sync screen reads at its start
    and after every write or cancel (ADR-388 D18), and the list's reply names each removal, which
    the screen shows.
R18. The sync screen reads `navigator.storage.persisted()` itself (a Worker cannot ask `persist()`)
    and says whether the browser keeps this site's storage, that it does not, or that it cannot
    tell, beside one sentence: an evicted collection is restored from the server by a download the
    owner taps.
R19. `PRIVACY.md` names the backups and the server copies the browser keeps, at most three of each
    kind, on the device alone, leaving it only by the owner's export. The seven locales carry every
    new message.

## 12. Acceptance criteria of part c2

| id | criterion | red at its tests commit | decided by |
|---|---|---|---|
| B1 | The route answers found and an age from a listing, refuses without an owner session, names no object, and lists again after its window | red: a stub that answers found always | `snapshot_route` tests the fence names |
| B2 | With no lister wired, the route answers unknown with 200 | red: the stub answers found | `snapshot_route`, `an_absent_list_credential_answers_unknown` |
| B3 | An archive without its manifest is not found, a name outside the archive's form is ignored, and the age is the newest sealed stamp's | red: the stub | `snapshot_route` tests the fence names |
| B4 | Retention keeps three of each kind, never removes the newest of a kind, and keeps the kinds apart | red: a stub that removes the oldest, the newest included | `crates/engine-core/tests/retention.rs` |
| B5 | The backups are listed newest first by kind and age, only a listed backup is exported, the backup operations carry no path, and the Worker posts the export's bytes by transfer | red: the stub exports any name | `files`, `protocol.test.ts`, the two `backups.test.ts` |
| B6 | In Chromium and WebKit, a download leaves its backup listed, and its export is a file the engine opens with every device review | red: the list is absent | `sync.spec.ts`, "a backup is listed and exported" |
| B7 | The storage status reads the page's own persistence answer, and every new message is in the seven locales | red: no status; the locales lack the keys | `sync-screen.test.ts`, `sync-locales.test.ts` |
| B9 | The list command runs with no shell, is killed at its time bound, is read to its byte bound, and answers unknown on any refusal | red: a lister wired to nothing | `crates/daemon/tests/snapshot_lister.rs` |

```acceptance
B1: cargo test -p deck-streak-api --test snapshot_route -- --exact a_listed_sealed_snapshot_answers_found_and_its_age
B1: cargo test -p deck-streak-api --test snapshot_route -- --exact the_route_refuses_without_an_owner_session
B1: cargo test -p deck-streak-api --test snapshot_route -- --exact no_answer_names_an_object
B1: cargo test -p deck-streak-api --test snapshot_route -- --exact a_stale_answer_is_listed_again
B2: cargo test -p deck-streak-api --test snapshot_route -- --exact an_absent_list_credential_answers_unknown
B3: cargo test -p deck-streak-api --test snapshot_route -- --exact an_archive_without_its_manifest_is_not_found
B3: cargo test -p deck-streak-api --test snapshot_route -- --exact a_name_outside_the_archive_form_is_ignored
B3: cargo test -p deck-streak-api --test snapshot_route -- --exact the_age_is_the_newest_sealed_stamps
B4: cargo test -p deck-streak-engine-core --test retention
B5: cargo test -p deck-streak-web-engine --test files -- --exact only_a_listed_backup_is_exported
B5: cargo test -p deck-streak-web-engine --test files -- --exact the_backups_are_listed_newest_first_by_kind
B5: cargo test -p deck-streak-web-engine --test boundary -- --exact the_backup_exports_reach_the_pool_only_through_files_and_the_core
B5: pnpm --dir web/app exec vitest run src/lib/sync/backups.test.ts
B5: pnpm --dir web/app exec vitest run src/lib/engine/backups.test.ts
B5: pnpm --dir web/app exec vitest run src/lib/engine/protocol.test.ts -t "the backup operations carry no path"
B6: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "a backup is listed and exported"
B7: pnpm --dir web/app exec vitest run src/lib/sync/sync-screen.test.ts -t "the storage status is the browser's answer"
B7: pnpm --dir web/app exec vitest run src/lib/sync/sync-locales.test.ts
B9: cargo test -p deck-streak-daemon --test snapshot_lister
```

## 12a. The policy line, decided outside this fence

| id | criterion | decided by |
|---|---|---|
| B8 | `PRIVACY.md` names the browser's backups and server copies, their bound of three of each kind, and the owner's export as their only way out | the privacy pack's `policy-published` row on CI; no repository test in this part's manifest reads the line, so the fence names none |

## 13. File manifest (part c2)

| path | change |
|---|---|
| `docs/specs/SPEC-377-the-web-sync-page-runs-the-full-sync-choice-in-the-browser-and-keeps-its-backups.md` | sections 11 to 16, insert-only |
| `docs/decisions/ADR-388-the-web-sync-page-drives-the-choice-from-the-worker-and-the-service-answers-the-snapshot.md` | D17 and D18, insert-only |
| `docs/red-first/SPEC-377.md` | part c2's red and green lines, appended |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | a citation's line number, where an edit shifts the cited line |
| `crates/api/src/snapshot_routes.rs` | new: the route, the lister port, the answer and its cache |
| `crates/api/src/router.rs` | `with_snapshot`, and the route merged beside the seal route |
| `crates/api/src/lib.rs` | the module |
| `crates/api/tests/snapshot_route.rs` | new: B1 to B3 |
| `crates/daemon/src/snapshot_lister.rs` | new: the list command, bounded |
| `crates/daemon/src/role_api.rs` | the lister wired when configured |
| `crates/daemon/src/lib.rs` | the module |
| `crates/daemon/tests/snapshot_lister.rs` | new: B9 |
| `crates/engine-core/src/retention.rs` | new: the rule |
| `crates/engine-core/src/lib.rs` | the module |
| `crates/engine-core/tests/retention.rs` | new: B4 |
| `crates/web-engine/src/files.rs` | the backups by kind and age, and the listed export |
| `crates/web-engine/src/wasm.rs` | the `backups`, `export_backup` and `retain` exports |
| `crates/web-engine/tests/files.rs` | B5 |
| `crates/web-engine/tests/boundary.rs` | the census of the three exports, insert-only |
| `web/app/src/lib/engine/protocol.ts`, `web/app/src/lib/engine/protocol.test.ts` | the `backups` and `backup-export` operations |
| `web/app/src/lib/engine/backups.ts`, `web/app/src/lib/engine/backups.test.ts` | new: the Worker's backups |
| `web/app/src/lib/engine/session.ts`, `web/app/src/lib/engine/client.ts`, `web/app/src/lib/engine/worker.ts` | the operations, the client's calls, the transfer |
| `web/app/src/lib/sync/BackupList.svelte`, `web/app/src/lib/sync/backups.test.ts` | new: the list and the export |
| `web/app/src/lib/sync/SyncScreen.svelte`, `web/app/src/lib/sync/sync-screen.test.ts`, `web/app/src/lib/sync/sync-locales.test.ts` | the list, the storage status, the locales |
| `web/app/messages/en.json`, `web/app/messages/es.json`, `web/app/messages/fr.json`, `web/app/messages/ja.json`, `web/app/messages/ko.json`, `web/app/messages/zh-Hans.json`, `web/app/messages/zh-Hant.json` | the new messages |
| `web/app/tests-engine/sync.spec.ts`, `web/app/engine-harness/main.ts` | B6 |
| `PRIVACY.md` | the policy line (R19) |
| `scripts/mutation-rows.d/S37700-S37799.json` | rows `S37712` to `S37722`, appended |
| `changelog.d/web-sync-page-377-b.md` | the fragment |

## 14. What this does NOT cover (part c2)

- It syncs no media and fills no media directory (part d, #631).
- It builds no iOS backup list, export or storage status (#633).
- It provisions nothing and names no store: the archive's list-only grant and the list command's
  setting are filled at deployment (#161).
- It does not close the window between the snapshot answer and an upload at the server (#617).
- It proves nothing on a device; the owner's acceptance session does (#637).

## 15. Risks (part c2)

- **A slow listing.** The list command runs inside a request bounded by the stack's timeout. Its own
  bound is shorter, and B9's `a_command_past_its_time_bound_answers_unknown` kills a command that
  outlives it.
- **A stray name read as a snapshot.** Only the exact sealed forms count, and a stamp needs both
  files; B3's tests feed a near-miss name with its manifest.
- **A removed backup's number reused.** Part c1 mints the first free number, so after a removal the
  newest file can carry the lowest number. The order reads the first-listed record first, and B5's
  order test lists a reused number as the newest.
- **The pool's room.** Each first-listed record is an empty pool entry; the list reserves the pool
  before it records one, and retention removes a record with its file.

## 16. Mutation rows (part c2)

Rows `S37712` to `S37722`, appended after `S37711` in `scripts/mutation-rows.d/S37700-S37799.json`.
Each find is spelt from the committed text after `cargo fmt`, keeps every item used, and occurs once
in its file. The killers of `S37716`, `S37721` and `S37722` spell their inputs as literals, never
from the constant. The TypeScript and Svelte are held by StrykerJS with `break` 100; the
`wasm32`-only exports by the `boundary` census.

| row | crate, file | mutant | killer |
|---|---|---|---|
| `S37712-AN-ARCHIVE-WITHOUT-ITS-MANIFEST-IS-FOUND` | api `src/snapshot_routes.rs` | the manifest check reads `true` | `snapshot_route::an_archive_without_its_manifest_is_not_found` |
| `S37713-THE-AGE-IS-THE-OLDEST-STAMP` | api `src/snapshot_routes.rs` | the newest stamp's `max` becomes `min` | `snapshot_route::the_age_is_the_newest_sealed_stamps` |
| `S37714-AN-ABSENT-LISTER-FINDS` | api `src/snapshot_routes.rs` | the off arm answers `found` | `snapshot_route::an_absent_list_credential_answers_unknown` |
| `S37715-A-STRAY-NAME-COUNTS` | api `src/snapshot_routes.rs` | the archive name check ends `\|\| true` | `snapshot_route::a_name_outside_the_archive_form_is_ignored` |
| `S37716-THE-CACHE-NEVER-EXPIRES` | api `src/snapshot_routes.rs` | the cache's window comparison reads `false` | `snapshot_route::a_stale_answer_is_listed_again` |
| `S37717-RETENTION-KEEPS-ONE-LESS` | engine-core `src/retention.rs` | `KEEP` minus one | `retention::three_of_each_kind_are_kept` |
| `S37718-RETENTION-REMOVES-THE-NEWEST` | engine-core `src/retention.rs` | the newest's exemption reads `false &&` before it | `retention::the_newest_of_a_kind_is_never_removed` |
| `S37719-RETENTION-MIXES-KINDS` | engine-core `src/retention.rs` | the kind's grouping key becomes one constant | `retention::each_kind_is_kept_apart` |
| `S37720-AN-EXPORT-TAKES-ANY-NAME` | web-engine `src/files.rs` | the listed check ends `\|\| true` | `files::only_a_listed_backup_is_exported` |
| `S37721-THE-LISTER-WAITS-UNBOUNDED` | daemon `src/snapshot_lister.rs` | the time bound's comparison reads `false` | `snapshot_lister::a_command_past_its_time_bound_answers_unknown` |
| `S37722-THE-LISTER-READS-PAST-ITS-BOUND` | daemon `src/snapshot_lister.rs` | the output bound's comparison reads `false` | `snapshot_lister::output_past_its_bound_answers_unknown` |
