# SPEC-358: iPhone and iPad reach Phase 1 parity: undo, bury and flag, the iPad's two columns, the remote in both of its modes with its mapping and the screen kept awake only while it drives a review, and the session's sync with the full-sync choice the core decides

- **Issue:** #633. **Campaign row:** SPEC-334 row 1.5 (R4, R8, R15).
- **Decided by:** ADR-342 (layouts, the remote, the screen awake), ADR-337 (the owner's taps),
  SPEC-357 and ADR-368 (the full-sync choice's one rule, #631), and ADR-369 (this delivery's
  decisions).
- **Parts:** three pull requests. Part a: bury, flag and the review's two columns. Part b:
  the remote, its mapping screen and the screen awake. Part c: the session's sync, media and the
  full-sync choice. This SPEC's fence holds part a; section 7 holds parts b and c.
- **Mutation band:** S35800-S35899. **Changelog fragment:** `changelog.d/ios-parity-358.md`.
- **Read at:** `D` = `f9381e14311a0cbe7d812946e33355c13d678e01` (dev, with SPEC-347 part 2 (#684),
  SPEC-348 part 2 (#709) and SPEC-350 part 2 (#690) merged); `PIN` = the engine fork commit
  `c538de5` the workspace pins. The figures were first read at dev `3fe90969` and at the app
  shell's red stub (`73a6bda1`, #684's head); every cite that moved since is given as it reads at
  `D`, and the shell's and the review screen's surfaces are read at `D`.
- **Re-read at:** `C` = `6d71bec078cb2e0792bf7eb2f80c1d9490d042de`, part a's cut (dev, with SPEC-365
  (#725) and the Undo delivery (#730) merged). A cite the cut's re-measure found moved is given
  as it reads at `C`, and says so; every other figure stands as read at `D`.

## 1. The problem, measured

### 1.1 What the native adapter and the core admit

| id | measured | figure | command |
|---|---|---|---|
| M1 | The native column and the adapter's allow-list | at `D`, ten pairs: (1,3) (3,0) (3,8) (7,4) (7,13) (7,22) (13,3) (13,4) (13,24) (27,6). At `C`, eight: (1,3) (3,0) (7,4) (7,13) (7,22) (13,3) (13,24) (27,6), since SPEC-365 and the Undo delivery took (13,4) and (3,8) off; `ALLOW_LIST: [Call; 8]` (`allow_list.rs:30`), `allowed` (`:75-79`); `ORDINARY: [Ordinary; 18]` (`table.rs:122-249`), where (13,4) and (3,8) left it and SPEC-364's (1,5) and SPEC-372's (27,14) joined it | `git show C:crates/ffi/src/allow_list.rs \| sed -n '30,79p'`; `git show C:crates/engine-core/src/table.rs \| sed -n '122,249p'` |
| M2 | The review pairs the native column lacks | (3,7) `CollectionService.GetUndoStatus` (`table.rs:151-157` at `C`), (5,4) `CardsService.SetFlag` (`:158-164` at `C`), (13,14) `SchedulerService.BuryOrSuspendCards` (`:193-199` at `C`), each `native: false, web: true`; the core's test expects `NotAllowed` natively for all three (`tests/table.rs:211-245` at `C`, asserted at `:265-269`) | `git show C:crates/engine-core/src/table.rs \| sed -n '151,164p;193,199p'`; `git show C:crates/engine-core/tests/table.rs \| sed -n '211,270p'` |
| M3 | The sync pairs | the core's one sync row is (1,3); (1,2) `MediaSyncStatus`, (1,5) `SyncCollection` and (1,6) `FullUploadOrDownload` are in neither table; the engine numbers `BackendSyncService` 0 to 8 | `awk '/^service /{s=$2;i=0} /^}/{s=""} s!="" && /rpc /{print s, i++, $2}' proto/anki/sync.proto` at `PIN` |
| M4 | What a sync request carries | `SyncAuth` holds the host key and an optional endpoint (`sync.proto:29-33`); `SyncCollectionRequest` is the auth and `sync_media` (`:51-54`); the answer's `required` is no changes, normal, full, full download or full upload (`:56-64`) | `grep -n -A8 -E 'message (SyncAuth\|SyncCollectionRequest\|SyncCollectionResponse)' proto/anki/sync.proto` at `PIN` |
| M5 | Which calls the endpoint guard covers | the login alone: `SYNC_LOGIN` (`dispatch.rs:26`), the guard's call (`:115-116`); SPEC-347 hands every later `SyncAuth` call the same rule (`:294-295`) | `git grep -n -E 'const SYNC_LOGIN\|login_guard::check' D -- crates/engine-core/src/dispatch.rs` |
| M6 | The core's closed reads hold no flag | `SNAPSHOT_SQL` reads `id, queue, type, due, ivl, reps, lapses` (`dispatch.rs:21`) | `git show D:crates/engine-core/src/dispatch.rs \| sed -n '21p'` |
| M7 | The owner's gesture token | not built: 0 lines name `OwnerGesture` or `run_exempt` under `crates`; SPEC-345 places the native exempt entry in `crates/ffi/src/engine.rs` (`:101-110`) | `git grep -c -E 'OwnerGesture\|run_exempt' D -- crates \| wc -l` |
| M8 | The adapter's surface | `Engine::new` (`engine.rs:82` at `C`), `run` (`:98`), `face` (`:144`), `collection_directory` (`:456`); `VoiceChoices` (181 lines), one file of `language<TAB>identifier` lines written whole and renamed (SPEC-348 R6, `:125`) | `git grep -n -E 'pub fn (new\|run\|face\|collection_directory)' C -- crates/ffi/src/engine.rs` |
| M9 | A native test can sync | the engine's own sync server is ported into the adapter's tests (`crates/ffi/tests/support/sync_server.rs`) | `git ls-tree --name-only D -- crates/ffi/tests/support/` |

### 1.2 Undo, bury and flag

| id | measured | figure | command |
|---|---|---|---|
| M10 | Undo after a sync | a normal sync discards the undo queue (SPEC-345 M15, `:39`), so the never-list's entry 1 has no engine method (`:45`); SPEC-342 U2 measured `UndoEmpty` after an answer and a normal sync (`:128`, its A15) | `git grep -n -E 'M15\|^\| 1 \|\(U2\)' D -- docs/specs/SPEC-345-* docs/specs/SPEC-342-*` |
| M11 | The flag and bury rules live in the web engine's study rule | `RED` (`study.rs:264` at `C`), `toggled_red` (`:269`), `BURY_USER` (`:274`), `BuryOf` (`:278`), `bury_of` (`:289`); rows S35009 to S35013 anchor there, killed by `tests/study.rs:238-258` at `C` | `git grep -n -E 'pub (const RED\|fn toggled_red\|const BURY_USER\|struct BuryOf\|fn bury_of)' C -- crates/web-engine/src/study.rs` |
| M12 | The web engine reaches the core on `wasm32` only | its every dependency, the core included, sits under `target.'cfg(target_arch = "wasm32")'`; a native build compiles the study rule alone (`lib.rs:1-11`) | `git show D:crates/web-engine/Cargo.toml`; `git show D:crates/web-engine/src/lib.rs \| sed -n '1,11p'` |
| M13 | The web's flag reads the queued card's flag | `flag` toggles the flag the kept card carries and keeps the new one (`wasm.rs:707-724` at `C`) | `git show C:crates/web-engine/src/wasm.rs \| sed -n '707,724p'` |
| M14 | The engine's undo status | `UndoStatus` is `undo`, `redo`, `last_step` (`collection.proto:95-99`); an empty `undo` names nothing undoable | `grep -n -A4 'message UndoStatus' proto/anki/collection.proto` at `PIN` |

### 1.3 The app, the remote and the census

| id | measured | figure | command |
|---|---|---|---|
| M15 | The app is on dev | 73 files under `ios`, 18 under `ios/App`: the app shell (#684) and the review screen (#709) are merged | `git ls-tree -r --name-only D -- ios \| wc -l`; `git ls-tree -r --name-only D -- ios/App \| wc -l` |
| M16 | The shell and the review screen at `D` | `NavigationSplitView` with `preferredCompactColumn` and a balanced style, whose detail reads "Choose a deck" until a deck is chosen and then shows its review (`DeckListView.swift:13-28`); one Account button in the sidebar's toolbar (`:60-63`); the model holds the chosen deck (`AppModel.swift:12-13`) and its review (`:15`); `EngineSession` is an actor (`EngineSession.swift:59` at `C`) whose methods make the review's engine calls (`:72-178` at `C`); `ReviewAction` is declared in `AnswerBar.swift:4`, the app's rating type in `ReviewView.swift:13` at `C`, the one entry `perform` at `ReviewModel.swift:63` at `C`, and the review's top row holds replay, stop and voices (`ReviewChrome.swift:18-35`) | `git show D:ios/App/Sources/DeckListView.swift`; `git show D:ios/App/Sources/AppModel.swift`; `git show D:ios/App/Sources/EngineSession.swift`; `git show D:ios/App/Sources/AnswerBar.swift \| sed -n '4p'`; `git show C:ios/App/Sources/ReviewView.swift \| sed -n '13p'`; `git show C:ios/App/Sources/ReviewModel.swift \| sed -n '63p'`; `git show D:ios/App/Sources/ReviewChrome.swift \| sed -n '18,35p'` |
| M17 | The review's seam | every review gesture goes through `ReviewModel.perform(_:)`, its state in the model (SPEC-348 R16, `:182`); one answer per shown card (R10, `:156`); undo, the iPad layout, the remote, keys, the idle timer, sync and media sync are left to #633 (`:327-330`) | `git grep -n -E '^R(10\|16)\.' D -- docs/specs/SPEC-348-*`; `sed -n '327,330p'` of it |
| M18 | The thin-Swift census at `D` | ceilings entry 0, view 3, model 6, session 4, credential 4, config 2, card 2, speech 5 (`test_ios_thin_swift.py:38-47`); the FFI, the codec and `FileManager` are the session's alone (`:75-77`); the Keychain the credential's (`:78-80`), the info dictionary the config's (`:81-84`), WebKit the card's (`:85`), AVFoundation and AVFAudio the speech role's (`:86-87`), the coders the codec's (`:88`); `URLSession`, `URLRequest`, `NWConnection`, `import Network`, `SQLite3`, `sqlite3_`, `UserDefaults`, `@AppStorage` and `NSUbiquitousKeyValueStore` are forbidden everywhere (`:92-102`), the network names admitted only in the card view's sources and tests (`:105-106`); no door names GameController or the idle timer | `git show D:scripts/tests/test_ios_thin_swift.py \| sed -n '38,47p;74,106p'` |
| M19 | The web remote's map | `resolve`: only confirm fires on the question side, and confirm is Good on the answer side (`actions.ts:30-35`); `BUTTONS` by standard index (`mapping.ts:8-17`), the stick (`:20-27`), `KEYS` (`:30-38`), the flag's key with Control or Command (`:41`) | `git show D:web/app/src/lib/remote/actions.ts \| sed -n '30,43p'`; `git show D:web/app/src/lib/remote/mapping.ts` |
| M20 | The default mapping both clients owe | SPEC-343 section 7 (`:329-352`); persisting it and its screen are #630's on the web and #633's on iPad (`:292-293`); GameController, key commands and the idle timer are #633's (`:286-288`) | `git show D:docs/specs/SPEC-343-* \| sed -n '286,293p;329,352p'` |
| M21 | Where the app's tests run | the Apple job's `harness` job (`xcframework.yml:356`): its review screen's tests step, Debug, on the iPhone then the iPad (`:497`), and its report (`:555`) | `git grep -n -E '^  harness:\|name: the review screen.s tests, Debug\|name: the report' D -- .github/workflows/xcframework.yml` |
| M22 | Collected data | SPEC-347 declares none: user content first leaves the device with the sync that uploads it, which declares it (`:310-311`, #633) | `git show D:docs/specs/SPEC-347-* \| sed -n '310,311p'` |

### 1.4 The platform

| id | fact | source |
|---|---|---|
| P1 | An extended gamepad names its face buttons by position: `buttonA` bottom, `buttonB` right, `buttonX` left, `buttonY` top, so they are the standard mapping's 0, 1, 2 and 3; `leftShoulder` and `rightShoulder` are 4 and 5; the d-pad's up, down, left and right are 12, 13, 14 and 15; a direction pad's and a thumbstick's four directions are button inputs with the framework's own threshold | Apple's GameController documentation, `GCExtendedGamepad`, `GCControllerDirectionPad` |
| P2 | A button input's pressed-changed handler fires on press and on release; a controller's arrival and departure are notifications | `GCControllerButtonInput`, `GCControllerDidConnect`, `GCControllerDidDisconnect` |
| P3 | Hardware keyboards are coalesced into one keyboard, so a remote in keyboard mode cannot be told from any other keyboard | `GCKeyboard.coalesced` |
| P4 | A key command's discoverability title lists it in the iPad's command menu; by default text input and focus take a key before a key command does | `UIKeyCommand`, `wantsPriorityOverSystemBehavior` |
| P5 | The idle timer is the app's to disable while needed and to restore after; a controller press does not reset it the way a touch does | `UIApplication.isIdleTimerDisabled` |

**What follows.** Undo, bury and flag lack three pairs and a native call each (M2), and the flag
and bury rules sit in the web engine where the native adapter cannot reach them (M11, M12): the
rule moves to the core both adapters share. Nothing guards a sync call but the login (M5), and
no pair a sync needs is admitted (M3). The full-sync choice's order, counts and checks are the
core's (SPEC-357, #631), so this delivery adds the adapter's storage and the screens, and asks the
core for each call it lacks. The remote's map exists on the web only, in the page (M19), and no
census door names the framework the native client reads it with (M18).

## 2. Requirements

Part a, delivered by the first pull request:

R1. **Two pairs.** (13,14) `SchedulerService.BuryOrSuspendCards` and (5,4) `CardsService.SetFlag`
    join the core's native column and the adapter's allow-list, so #623's parity test holds. The
    allow-list then holds the count measured at the build's cut plus these two: at `D` it holds
    ten (`ALLOW_LIST: [Call; 10]`, `allow_list.rs:28`), SPEC-365's delivery (#711) leaves nine,
    the Undo delivery (#714) leaves eight, and part a's two make ten on that base. The build
    re-measures the count at its cut: eight at `C` (`allow_list.rs:30`). (3,7) stays web-only (section 7). The web column is
    unchanged; no other pair joins.
R2. **One copy of the flag and bury rules.** `RED`, `toggled_red`, `BURY_USER`, `BuryOf` and
    `bury_of` move from the web engine's study rule to `crates/engine-core/src/review.rs`,
    unchanged in behaviour. The web engine's `wasm32` module takes them from the core through its
    existing `wasm32` edge; the web engine gains no native dependency. A census holds each of the
    five defined once in the workspace, in the core.
R3. **The native bury and flag.** The adapter's `Engine` exports `bury(card_id)`, which runs
    (13,14) with the core's `bury_of`, and `flag(card_id, flag)`, which runs (5,4) with
    `toggled_red(flag)` and answers the new flag. `flag` is the flag the queued card carries, as
    on the web (M13): the codec's `QueuedCard` carries it, read from the engine's `Card.flags`
    (field 17 at `PIN`, `cards.proto:49`) and pinned by literal bytes (A40). Swift builds neither
    request and computes no flag.
R4. **Undo.** Moved out of part a: section 7 (#714).
R5. **The actions go through the review's one entry.** `ReviewAction` gains bury and flag
    beside show answer, rate, replay and stop (SPEC-348 R16). Bury and flag act on the shown card
    alone, once per gesture, with the bar's buttons disabled while a call runs; a bury shows the
    next card; the flag shows as an icon with a text label, never a colour alone.
R6. **Two columns.** In regular width the decks and the review sit side by side; in compact width
    (the iPhone, and an iPad window in Split View or Slide Over) the stack shows the review once a
    deck is chosen. The layout follows the window's horizontal size class, never the device. Bury,
    flag and replay sit in the review's toolbar in both widths; the answer bar fills the
    review's width. Hiding and showing the sidebar keeps the shown card and its side.
R7. **Swift stays thin.** Every new Swift file is registered with its role and exact decision
    count; no ceiling rises.
R8. **CI.** The `harness` job gains one step, "the bury and flag tests, Debug, on the iPhone and
    then the iPad", and its report a row. No job, workflow, runner or required context is
    added.

Parts b and c, delivered by the next pull requests (section 7):

R9. **The remote's map lives in the adapter.** `crates/ffi/src/remote.rs` holds the intents
    (confirm, again, hard, good, easy, undo, bury, flag, replay), the side rule as one table
    (only confirm fires on the question side; confirm is Good on the answer side, as M19), and the
    defaults per mode equal to SPEC-343 section 7: gamepad controls named by P1's positions, the
    d-pad's and the left thumbstick's four directions included, and keys by their input, the flag
    with Command or Control and 1. Every other control fires nothing.
R10. **Parity with the web.** A census holds the adapter's defaults and the web's (`mapping.ts`)
     equal to SPEC-343 section 7 under P1's correspondence, and refuses a planted drift on either
     side by name.
R11. **The stored mapping.** `RemoteMapping`, opened on one file outside the collection and its
     media folder and never synced, holds `mode<TAB>control<TAB>intent` lines, written whole to a
     temporary file and renamed; a line that does not parse or names an unknown control or intent
     is skipped; a mode with no valid line takes its defaults; assigning a control moves it off any
     other intent in that mode; restoring a mode's defaults removes its lines.
R12. **Gamepad input.** The app finds an extended gamepad at start and on its arrival, binds each
     element to its control's name by one fixed list, fires on the press and never on the release,
     and hands the name to the review's model, which asks the adapter for the action on the shown
     side and runs it through `ReviewModel.perform(_:)`. A departure unbinds.
R13. **Keyboard input.** Each mapped key of keyboard mode is a key command with a discoverability
     title naming its action, active only while a review is shown, and never taken from a focused
     text field.
R14. **The mapping screen.** The settings sheet, opened by the sidebar's one toolbar button, holds
     the account and the remote. The remote has one screen per mode listing the nine actions in two
     groups, the review's five and the card's four, each with its controls as text. Choosing an
     action asks for a press, with Cancel; restoring a mode's defaults asks for confirmation. Every
     target is at least 44 by 44 pt.
R15. **The screen awake.** `Awake::wanted` in the adapter is true exactly when a review is shown,
     the scene is active, and an extended gamepad is connected or a keyboard is connected and a
     mapped key has driven this review. The app sets the idle timer from it at every change of
     those inputs and nowhere else; with no remote paired the idle timer stays on.
R16. **The census's new role.** The thin-Swift census gains the role `input`, the one role that
     imports GameController and names the idle timer, with its ceiling; no other ceiling rises.
R17. **The sync's pairs.** (1,5) `SyncCollection`, (1,2) `MediaSyncStatus` and (3,1)
     `CloseCollection` join the native column and the allow-list. Every call that carries a
     `SyncAuth` passes the login guard's endpoint rule in the core before the engine sees it: the
     normal sync, the one-way sync and the server copy's download (SPEC-347 R2's rule, one copy).
R18. **The session's sync.** When signed in, the app runs a normal sync with media after the
     collection opens, when a review ends, and on the Sync button, never while another engine call
     runs. Offline or refused, every review stays on the device and the status says so. A
     `new_endpoint` in the answer is not followed: the configured endpoint stays.
R19. **The status.** A line at the foot of the sidebar names the reviews not yet synced and whether
     the collection changed, read from the core's `Dispatcher::unsynced` (SPEC-357 R9) with no
     network, after every rating, bury, flag, undo and sync; while media syncs, it shows (1,2)'s
     progress.
R20. **The choice on iPhone and iPad.** When a sync answers a full sync, a sheet shows each offered
     direction with what each side loses, from the core's counts; no direction is preselected and
     Cancel is first and focused. The owner's tap on one direction confirms it. The sheet then
     shows the backup, the snapshot check (an upload only), the re-check and the write as steps,
     each with progress and its refusal by name, both sides untouched on a refusal, and a designed
     end naming the result.
R21. **The adapter's storage.** The server copy and the backups are files in the app's own
     Application Support folder, outside the collection and its media folder, never synced. A
     download's backup is the device's collection, copied whole while closed by (3,1), written to a
     temporary file and renamed, the collection reopened by the same open request; the copy's ids
     are read by the core and handed to `Confirmed::backed_up`. An upload's backup is the server
     copy. The two newest are kept: a new backup is accepted before the oldest is deleted, by one
     retention function in the core.
R22. **The upload waits for the owner's session on iOS.** The snapshot answer needs the owner's
     session (SPEC-357 R18), which the native client lacks until its sign-in (#627); until then an
     upload is refused by name at the snapshot step, both sides untouched. The download works.
R23. **The one-way write.** The native exempt entry in `crates/ffi/src/engine.rs` builds the owner's
     gesture from the confirm tap and calls the core's one-way call with the core's `Write`. A Swift
     census holds the call to the choice sheet's confirm handler alone.
R24. **Collected data.** The privacy manifest declares the user content the sync uploads, linked to
     the user, not used for tracking, for the app's functionality.
R25. **Swift stays thin** for parts b and c as in R7.
R26. **CI** for parts b and c as in R8: one step each, "the remote's tests" and "the sync tests",
     Debug, on the iPhone and then the iPad.

Stored decks on iPhone and iPad (#633), each in the part its behaviour belongs to:

R27. **Part a: the card sandbox is unchanged.** The card frame still blocks the network, and a
     card's media comes only from the device's own store. Every Swift file part a adds or edits
     keeps SPEC-348 R18's frame (#619), and the census that holds it runs unchanged over them
     (A37). Parts b and c keep it the same way.
R28. **Part c: a deck is stored once.** After the first download, a deck's collection and media
     stay on the device. Opening a stored deck makes no full download; a sync sends only changes,
     and media moves by content hash, so only new or changed files transfer.
R29. **Part c: the first card offline.** With a deck stored and the device offline, a cold launch
     reaches the first card, and no network request comes before it (A38). Time to the first card
     is measured twice, with the deck stored and after a fresh download, by part c's build in the
     Apple job's existing measurements step. Part c fixes the bound in this SPEC from those two
     readings; no bound is written before them (A39).
R30. **Part c: where the files live.** Media lives in the app's Application Support folder, marked
     excluded from backup because it can be downloaded again; the Caches folder is not used, since
     the system may empty it when space runs low. The collection keeps the default backup, so
     unsynced reviews survive a restore. The files' data protection is
     `completeUntilFirstUserAuthentication`, so a sync runs while the device is locked.
R31. **Part c: keep every deck, remove one by hand.** There is no automatic eviction and no size
     cap. A storage screen lists each deck's stored size, its collection and media together, and
     offers to free a deck's media or to remove the deck. A removal never discards unsynced
     reviews: it syncs first, or it refuses and says why.
R32. **Part c: a download continues in the background.** Full and media downloads continue while
     the app is suspended, and resume after an interruption. The census forbids a Swift network
     client (M18), so the mechanism is part c's design question, ruled before part c is built.
R33. **Part c: prefetch.** A sync also runs when the app comes to the foreground, beside R18's
     moments. The media of the next cards in the queue is loaded before those cards show; part c's
     design picks how many, and the review screen never waits on the network.
R34. **Part c: a deck check costs one small response.** A check of a shared or catalog deck is a
     conditional request, so an unchanged deck costs one small response, and a download is
     verified against its content hash before it is used.

## 3. Acceptance criteria (part a)

| id | criterion | red first | decided by |
|---|---|---|---|
| A1 | The two review pairs (13,14) and (5,4) are admitted on the native transport, and the native column holds the count measured at the cut plus two (R1) | `decide(Native, 13, 14)` reads `NotAllowed` | `crates/engine-core/tests/table.rs` `the_review_pairs_are_ordinary_on_the_web` (its `REVIEW` table now expects `Admit` natively for the two) and `every_pair_is_admitted_held_or_refused_by_its_transport` |
| A2 | The adapter's allow-list equals the native column | the column holds (13,14) and the allow-list does not | `crates/engine-core/tests/parity.rs` `each_adapter_table_equals_its_transport_column` |
| A3 | The flag and bury rules are defined once, in the core | the census finds them in the web engine and none in the core | `scripts/tests/test_one_review_rule.py` `test_the_flag_and_bury_rules_live_once_in_the_core`, with a planted second copy refused by name |
| A4 | The flag toggles red, and bury is the user's bury of one card, judged in the core | not red: the rules move unchanged and their behaviour was pinned on the web (SPEC-350 A4, A5); the moved tests guard the move | `crates/engine-core/tests/review.rs` `the_flag_toggles_red`, `bury_is_the_users_bury_of_the_shown_card` |
| A5 | A native bury buries the card as the user's bury, and the queue moves on | a stub that returns without the call: the card's queue is unchanged | `crates/ffi/tests/review_actions.rs` `a_bury_buries_the_card_as_the_users_bury` |
| A6 | A native flag turns the card red, and red to none, and answers the new flag | a stub that answers its input | `crates/ffi/tests/review_actions.rs` `a_flag_toggles_red_and_answers_the_new_flag` |
| A11 | Bury shows the next card; flag shows "Flagged red" and clears it | no bury or flag control | `DeckStreakUITests/ReviewActionsFlowTests/test_a11_bury_moves_on_and_flag_toggles_red` |
| A12 | The review keeps its card and side when the iPad's sidebar hides and returns; on the iPhone, which has no sidebar, the same test's compact-width form asserts that the review stands alone and that the card and its revealed side survive a turn to landscape and back to portrait | not red: SPEC-348 R16 holds the state in the model; this pins it for the layout | `DeckStreakUITests/ReviewActionsFlowTests/test_a12_the_review_survives_the_sidebar` |
| A13 | Every Swift file keeps its role, its doors and its budget | a new file unregistered | `scripts/tests/test_ios_thin_swift.py` |
| A14 | The harness job runs the new step on the iPhone and then the iPad, and reports it | no such step | `scripts/tests/test_ci_workflows.py` `test_the_review_actions_step_runs_on_the_iphone_and_then_the_ipad` |
| A37 | The card frame stays as SPEC-348 R18 holds it, over every Swift file this part adds or edits | not red: an unchanged constraint (R27); the census already holds it, and this pins it for part a's files | `scripts/tests/test_ios_review_screen.py` `the_card_frame_is_the_factorys_alone` (SPEC-348 A13) |
| A40 | The codec decodes the queued card's flag | a decoder that never reads `Card.flags` (17): a red card decodes as unflagged | `ios/HarnessWire` `ResponseDecodingTests/test_a40_the_queued_card_carries_its_flag`, from literal bytes |

```acceptance
A1: cargo test -p deck-streak-engine-core --test table -- --exact the_review_pairs_are_ordinary_on_the_web
A1: cargo test -p deck-streak-engine-core --test table -- --exact every_pair_is_admitted_held_or_refused_by_its_transport
A2: cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_table_equals_its_transport_column
A3: python3 -m unittest discover -s scripts/tests -p test_one_review_rule.py -k test_the_flag_and_bury_rules_live_once_in_the_core
A4: cargo test -p deck-streak-engine-core --test review -- --exact the_flag_toggles_red
A4: cargo test -p deck-streak-engine-core --test review -- --exact bury_is_the_users_bury_of_the_shown_card
A5: cargo test -p deck-streak-ffi --test review_actions -- --exact a_bury_buries_the_card_as_the_users_bury
A6: cargo test -p deck-streak-ffi --test review_actions -- --exact a_flag_toggles_red_and_answers_the_new_flag
A11: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewActionsFlowTests/test_a11_bury_moves_on_and_flag_toggles_red
A12: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -only-testing:DeckStreakUITests/ReviewActionsFlowTests/test_a12_the_review_survives_the_sidebar
A13: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py -k test_every_swift_file_keeps_its_role_its_doors_and_its_budget
A14: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_review_actions_step_runs_on_the_iphone_and_then_the_ipad
A37: python3 -m unittest discover -s scripts/tests -p test_ios_review_screen.py -k the_card_frame_is_the_factorys_alone
A40: swift test --package-path ios/HarnessWire --filter HarnessWireTests.ResponseDecodingTests/test_a40_the_queued_card_carries_its_flag
```

A3's census drops from its listing, by name and before any file is opened, every Rust path whose name holds `drill` or `readings_tree` in any case (the parked `crates/vault/src/readings_tree.rs` among them), and prints how many it dropped; it is red when that count is 0. The drill surface is parked (#158), so those files are not examined: this narrows A3's population, and the pull request's weakening table records it.

## 4. File manifest

Part a's files; parts b and c name theirs in their own pull requests (section 7).

| file | context | change |
|---|---|---|
| `crates/engine-core/src/review.rs` | core | added: `RED`, `toggled_red`, `BURY_USER`, `BuryOf`, `bury_of`, moved; and `bury_request` and `flag_request`, the two requests the native adapter sends, encoded, since the adapter holds no protobuf codec (R3) |
| `crates/engine-core/src/lib.rs` | core | `pub mod review;` |
| `crates/engine-core/src/table.rs` | core | (13,14), (5,4) `native: true` |
| `crates/engine-core/tests/review.rs` | core tests | added: the two moved tests, and `the_bury_and_flag_requests_are_the_engines_bytes`, the two requests' bytes spelled by hand (R3) |
| `crates/engine-core/tests/table.rs` | core tests | `NATIVE` gains two pairs; `REVIEW` expects `Admit` natively for them |
| `crates/engine-core/tests/review_pairs.rs` | core tests | `NATIVE` gains (13,14) and (5,4), insert-only |
| `crates/web-engine/src/study.rs` | web engine | the five items removed |
| `crates/web-engine/src/wasm.rs` | web engine (`wasm32`) | imports the five from the core |
| `crates/web-engine/tests/study.rs` | web engine tests | the two tests and their imports removed |
| `crates/ffi/src/allow_list.rs` | native adapter | two calls; `[Call; N]`, N the count measured at the cut plus two (ten on the base R1 names) |
| `crates/ffi/src/engine.rs` | native adapter | `bury`, `flag`, and one exported function that answers the core's `review::RED`, which `EngineSession` reads once, so Swift holds no copy of red |
| `crates/ffi/tests/review_actions.rs` | native adapter tests | added |
| `crates/ffi/tests/review_pairs.rs` | native adapter tests | `EXPECTED` gains the two pairs, insert-only; its `the_allow_list_carries_the_review_pairs` kills S35804 and S35805 |
| `ios/HarnessWire/Sources/HarnessWire/Messages.swift` | codec | `QueuedCard`'s flag, from `Card.flags` (17 at `PIN`) |
| `ios/HarnessWire/Tests/HarnessWireTests/ResponseDecodingTests.swift` | codec tests | A40, from literal bytes |
| `ios/App/Sources/AnswerBar.swift` | view | `ReviewAction`, declared here at `D` (`:4`): its bury and flag cases |
| `ios/App/Sources/EngineSession.swift` | session | the adapter's new calls, reached here: at `D` the one file that imports the adapter (`:1`; the actor at `:59` at `C`) |
| `ios/App/Sources/ReviewModel.swift`, `ios/App/Sources/ReviewSession.swift` | model, session | `perform`'s bury and flag arms, with replay and stop handed on through `default:` to `ReviewPlayback`; `ReviewSession`'s bury and flag through one shown-card helper that `reveal` shares; no ceiling rises |
| `ios/App/Sources/ReviewChrome.swift` | view | bury and flag beside replay in the toolbar |
| `ios/App/Sources/ReviewPlayback.swift` | model | added: replay and stop, moved from `perform`'s arms |
| `ios/App/Sources/ReviewView.swift` | view | `ReviewPhase` gains `marking`, the phase a bury or a flag holds while it runs |
| `ios/AppUITests/ReviewActionsFlowTests.swift` | UI tests | added: A11 and A12 |
| `ios/swift-roles.json` | register | `ReviewPlayback.swift` as `model` with its count, `ReviewActionsFlowTests.swift` as `test`, and the changed files' counts; no ceiling rises |
| `scripts/tests/test_one_review_rule.py` | census | added |
| `scripts/tests/test_ci_workflows.py` | census | A14 |
| `.github/workflows/xcframework.yml` | the Apple job | R8's step and its report row |
| `scripts/mutation-rows.d/S35000-S35099.json` | rows | S35009 to S35013 re-anchored to the core; S35020 and S35021 re-anchored to `native: true`; S35019 untouched |
| `scripts/mutation-rows.d/S35800-S35899.json` | rows | section 8's part a rows |
| `scripts/mutation-equivalent.d/deck-streak-ffi.json` | equivalent records | added: `red_flag`'s body replaced with 1, which returns the core's `review::RED`, 1, unchanged |
| `docs/specs/SPEC-350-the-web-study-screens-review-a-card-from-the-engine-in-the-browser-in-the-sandboxed-frame-by-touch-keys-or-a-remote.md` | documents | an amendment appended at its end: A4 and A5 are decided in the core |
| `docs/specs/SPEC-358-iphone-and-ipad-reach-phase-1-parity.md`, `docs/decisions/ADR-369-the-remotes-map-the-ipad-layout-the-shared-flag-and-bury-rules-and-the-device-backup.md`, `docs/schematics/iphone-and-ipad-phase-1-parity.md`, `docs/red-first/SPEC-358.md`, `changelog.d/ios-parity-358.md` | documents | added |

## 5. What this does NOT do

- It builds no undo past a sync and no owner gesture for one: a normal sync discards the queue,
  so the never-list's entry 1 has no engine method, and the engine's measurement stands (#620).
- It adds no redo and no undo of an edit, a suspend or any action the review does not make; the
  native review's actions are its own (#666).
- It builds no native sign-in, no owner session and no snapshot request from the app; until they
  exist the upload is refused at the snapshot step and the download works (#627).
- It does not install or change the snapshot answer's route or the offsite store (#161, #617).
- It does not close the window between the re-check and an upload at the server (#617).
- It puts the media folder in no backup: the choice protects review, card and note ids, as the
  shared rule's model states them (#631).
- It builds no abort of a running sync and no sync when the app is sent to the background: the
  session's actor runs one engine call at a time, and a review left unsynced is synced at the next
  start (#666).
- It writes no Swift mutation row into the gate's tables; the codec's rows stay in its own file
  (#650).
- It takes no figure on a device: whether the remote's two modes arrive, whether a key command
  beats the card view to a key, and whether the screen stays lit are read in the owner's device
  session (#629) and accepted at the owner's acceptance session (#637).
- It builds no upload lane, signs nothing and names no team (#634).
- It changes no card view, rule list or card script (#619, #651, #664).
- It shares no mapping between the web and the app: each client stores its own, and the census
  holds their defaults equal (#630).

## 6. Risks

- **A key command loses to the card view.** A focused web view may take Space or an arrow key
  before a key command (P4). Detected by A-criteria on the simulator with a hardware keyboard and
  by the device session (#629); the remedy is the review's commands taking priority while the
  review is shown, never a script in the card.
- **Keyboard mode collides with the system.** ADR-342 names it; the defaults avoid Command except
  for the flag, and the mapping screen changes any key.
- **A keyboard always attached.** An iPad with a keyboard case would hold the screen awake for
  every review if the rule read the keyboard alone; R15's clause needs a mapped key to have
  driven the review.
- **A backup's copy while the engine writes.** Copying an open collection can tear it; R21 closes
  it first and verifies the copy by ids before any write, and a failed reopen is refused by name
  with the copy kept.
- **Storage.** A choice holds the collection, the server copy and two backups at once; a full disk
  stops the choice at the backup step, before any write, and says so.
- **The shared rule moves.** SPEC-357 and ADR-368 are on dev at `D`, with the core's
  `full_sync.rs` and its tests; a name or signature that later changes there is adopted here,
  never re-implemented.
- **The moved rows.** Moving the flag and bury rules re-anchors five rows and their killers' crate;
  an anchor census at the base and the head proves each find occurs once.

## 7. Delivered by the next pull requests

| id | criterion | delivered by |
|---|---|---|
| A15 | The side rule fires only confirm on the question side and confirm as Good on the answer side, over all eighteen cases | part b: `crates/ffi/tests/remote.rs` `the_side_rule_matches_the_default_mapping` |
| A16 | Each mode's defaults equal SPEC-343 section 7, and every other control fires nothing | part b: `remote.rs` `each_modes_defaults_are_the_campaigns` |
| A17 | The adapter's and the web's defaults are equal under the correspondence; a planted drift on either side is refused by name | part b: `scripts/tests/test_remote_mapping_parity.py` |
| A18 | A stored mapping drives its mode, a damaged line is skipped, a mode with none takes its defaults, and an assignment moves a control off its old intent | part b: `remote.rs` `the_stored_mapping_drives_its_mode` and three siblings |
| A19 | The screen is wanted awake only for a shown review in an active scene with a gamepad, or with a keyboard that drove it, over all thirty-two inputs | part b: `remote.rs` `the_screen_is_awake_only_while_a_remote_drives_a_review` |
| A20 | A key command fires its mapped action on the iPad and the iPhone, and is listed by its title | part b: `DeckStreakUITests/RemoteFlowTests` |
| A21 | A gamepad control's name reaches the review's action through the model | part b: `DeckStreakTests/RemoteInputTests` |
| A22 | The mapping screen assigns, cancels and restores, and its targets are at least 44 pt | part b: `DeckStreakUITests/MappingFlowTests` |
| A23 | The idle timer is set only from the adapter's answer, in the input role alone | part b: `scripts/tests/test_ios_thin_swift.py` |
| A24 | The three sync pairs are admitted natively and the allow-list equals the column | part c: `crates/engine-core/tests/table.rs`, `parity.rs` |
| A25 | A normal sync's, a one-way sync's and the server copy's endpoint are refused by the login guard's rule before the engine sees them | part c: `crates/engine-core/tests/login_guard.rs` |
| A26 | After an answer and a normal sync through the adapter, the undo status is empty and undo is the engine's `UndoEmpty` | part c: `crates/ffi/tests/sync.rs` `a_normal_sync_empties_the_native_undo` |
| A27 | A review made offline stays on the device, is counted unsynced, and reaches the server at the next sync | part c: `sync.rs` `an_offline_review_is_kept_and_synced_later` |
| A28 | Each offered direction's counts reach the sheet from the core | part c: `sync.rs` `the_choice_offers_the_engines_directions_with_the_cores_counts` |
| A29 | A download's backup is the closed collection's whole copy, accepted by its ids, and the collection reopens | part c: `sync.rs` `a_download_backs_up_the_closed_collection_and_reopens_it` |
| A30 | The two newest backups are kept, and the oldest is deleted only after the new one is accepted | part c: `crates/engine-core/tests/full_sync.rs` `retention_keeps_two_and_deletes_after_the_new_one` |
| A31 | An upload is refused by name at the snapshot step on iOS, both sides untouched | part c: `sync.rs` `an_upload_waits_for_the_owners_session` |
| A32 | A download replaces the device with the server's collection only after its backup | part c: `sync.rs` `a_download_writes_after_its_backup` |
| A33 | The one-way call is made from the choice sheet's confirm handler alone | part c: `scripts/tests/test_ios_one_way_call.py` |
| A34 | The sheet preselects nothing, focuses Cancel and shows each step | part c: `DeckStreakUITests/SyncChoiceFlowTests` |
| A35 | The status names unsynced reviews offline and clears after a sync | part c: `DeckStreakUITests/SyncFlowTests` |
| A36 | The privacy manifest declares the uploaded user content | part c: `scripts/tests/test_ios_app_tree.py` |
| A38 | With a deck stored and the device offline, a cold launch reaches the first card, and the session makes no network request before it | part c: `DeckStreakUITests/OfflineFirstCardFlowTests/test_a38_a_stored_deck_reaches_its_first_card_offline` |
| A39 | Time to the first card is measured with the deck stored and after a fresh download, on the iPhone and the iPad, and part c's amendment of this SPEC fixes the bound from those readings | part c: the Apple job's measurements step (`xcframework.yml:421`), `DeckStreakUITests/FirstCardMeasureTests` |

### Moved out of part a: native undo (#714)

- It builds no native undo: native Undo leaves the allow-list and the native run refuses (3,8) (#714)

These part a items moved here, kept as drafted for the record; no pull request of this SPEC
delivers them:

| id | item | moved because |
|---|---|---|
| R1, A1, A2 | (3,7) `CollectionService.GetUndoStatus` joining the native column and the allow-list, and the (3,7) half of A1 and A2 | (3,7) stays web-only (#714) |
| R4 | Undo enabled only while (3,7)'s `undo` label is non-empty, carrying the label as its accessibility value; (3,8), then the next queued card's question; the status read after the review opens and after every answer, bury, flag and undo | native Undo leaves the allow-list (#714) |
| R5 | `ReviewAction`'s undo case | native Undo leaves the allow-list (#714) |
| R6, R8, A14 | "undo" in the review's toolbar and in the `harness` step's name | native Undo leaves the allow-list (#714) |
| A7 | The undo status names the answer, and undo restores the card and removes its review (`crates/ffi/tests/review_actions.rs`) | the native run refuses (3,8) (#714) |
| A8 | The codec writes the undo calls and decodes the status's label (`ResponseDecodingTests`) | native Undo leaves the allow-list (#714) |
| A9 | Undo is enabled only while the engine names an action (`ReviewModelTests`) | native Undo leaves the allow-list (#714) |
| A10 | Undo returns the answered card's question, on the iPhone and the iPad (`ReviewActionsFlowTests`) | native Undo leaves the allow-list (#714) |
| codec | the undo status's request and decoder and their literal-bytes tests (`Messages.swift`, `RequestBytesTests.swift`, `ResponseDecodingTests.swift`) | native Undo leaves the allow-list (#714) |
| model, session | `ReviewModel`'s and `ReviewSession`'s status reads | native Undo leaves the allow-list (#714) |
| rows | `S35800-UNDO-STATUS-IS-NATIVE` and `S35803` ((3,7) on the allow-list) are not written; `S35019` is untouched | (3,7) stays web-only (#714) |
| schematic | section 2.4, an undo's sequence: the status after open, answer, bury, flag and undo; (3,8); the next queued card | the native run refuses (3,8) (#714) |

## 8. Mutation rows (part a)

| row | mutates | killed by |
|---|---|---|
| `S35801-BURY-IS-NATIVE` | (13,14) `native: true` to `false` in `table.rs` | `table::the_review_pairs_are_ordinary_on_the_web` |
| `S35802-SET-FLAG-IS-NATIVE` | (5,4) `native: true` to `false` | same |
| `S35804-BURY-IS-ON-THE-ALLOW-LIST` | (13,14)'s allow-list call renumbered in `allow_list.rs` | `review_pairs::the_allow_list_carries_the_review_pairs` |
| `S35805-SET-FLAG-IS-ON-THE-ALLOW-LIST` | (5,4)'s allow-list call renumbered in `allow_list.rs` | `review_pairs::the_allow_list_carries_the_review_pairs` |
| `S35806-NATIVE-BURY-USES-THE-CORES-BURY` | `bury_of(card_id)` replaced by an empty `BuryOf` in `engine.rs` | `review_actions::a_bury_buries_the_card_as_the_users_bury` |
| `S35807-NATIVE-FLAG-TOGGLES` | `toggled_red(flag)` replaced by `flag` in `engine.rs` | `review_actions::a_flag_toggles_red_and_answers_the_new_flag` |
| `S35808-RED-LIVES-ONCE-IN-THE-CORE` | a second `RED` planted back in `crates/web-engine/src/study.rs`, where the five lived before the move | `test_one_review_rule.TheFlagAndBuryRulesLiveOnceInTheCore.test_the_flag_and_bury_rules_live_once_in_the_core` |

S35808 is A3's row: its mutant puts one of the five back where A3 forbids it, so the census is seen
failing for its criterion's reason. Its killer is a Python test module, so CI's `mutation-rows`
job proves it, read by its stem.
S35009 to S35013 keep their ids and properties; their crate, path and killers move to the core.
S35020 and S35021 keep their ids and properties; their finds read `native: true`. S35019, the
(3,7) row, is untouched, and S35800 and S35803 are not written (section 7).
S34501-CLOSE-COLLECTION-IS-THE-WEBS-ALONE keeps its id and its stem when part c re-anchors its
find to (3,1)'s `native: true`. Its stem name then reads the old column: it is stale by design,
since a row is re-anchored and never renamed or retired.

## 9. What only a device or a person proves

| id | what | who, and when |
|---|---|---|
| V1 | Bury and flag on an iPhone and an iPad, by touch | the owner, in the first device session (#629) |
| V2 | The remote in its gamepad mode and its keyboard mode drives every mapped action on both devices, and a key command beats the card view to its key | the owner, in the first device session (#629) |
| V3 | The screen stays lit while the remote drives a review and dims after it disconnects or the review closes | the owner, in the first device session (#629) |
| V4 | A session's sync, an offline review synced later, and a download chosen on the device | the owner, at the acceptance session (#637) |

## 10. Formal models

No new entry. The full-sync choice's order is SPEC-357's `FullSyncChoice`, held by the core's
typed states, which this delivery calls and never re-implements. The side rule and the screen-awake
rule are total functions over small finite inputs, judged exhaustively by A15 and A19.
