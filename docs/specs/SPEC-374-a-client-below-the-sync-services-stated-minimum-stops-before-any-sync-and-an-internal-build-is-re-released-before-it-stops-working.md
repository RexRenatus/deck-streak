# SPEC-374: a client below the sync service's stated minimum stops before any sync, and an internal build is re-released before it stops working

- **Issue:** #671
- **Decided by:** ADR-385
- **Status:** accepted (part one, the handshake and the web Worker's read of the statement; part
  two, the re-release check, is section 7)
- **Mutation band:** S37400-S37499 (`scripts/mutation-rows.d/S37400-S37499.json`)
- **Model:** opus for part one (three crates, a network read in the static library, a request
  allow-list site, a TLA+ model); sonnet for part one's fix round (the web Worker's read) and for
  part two (one standard-library script and one workflow)

Every `path:line` below was read at `6f9ef860`.

## 1. The problem, measured

Nothing tells a client that the sync service no longer accepts it.

- The sync service is the engine's own sync server, served at the edge under `/anki-sync/` on the
  origin that also serves `/api/*` and the web client (`deploy/caddy/deck-streak.caddy:46-52`,
  `:60-86`, `:88-93`). The API's routes carry their `/api` prefix themselves
  (`crates/api/src/health.rs:23`, `crates/api/src/sync_seal_routes.rs:40`).
- The API serves its health routes to anyone (`crates/api/src/router.rs:216`) and the owner's
  routes only when an owner is configured (`:217-259`). No route states which clients the service
  accepts.
- What a client can read about itself is not comparable across clients. The iPhone and iPad build
  number is the first-parent commit count of `dev` for an internal build and of `main` at the tag
  for a release (`scripts/ios_lane.py:120-134`), and every build between two releases shares one
  marketing version, the workspace version (`scripts/ios_lane.py:116-117`,
  `ios/App/Info.plist:19-22`). The web client has neither.
- One holder runs the engine on every client: the engine core, whose `Dispatcher` runs each call
  on `Transport::Native` (the static library) or `Transport::Web` (the web engine)
  (`crates/engine-core/src/dispatch.rs:134-142`). It already refuses a sync login's or a normal
  sync's endpoint before the engine (`dispatch.rs:157-169`) and checks a one-way sync's auth before
  the engine (`dispatch.rs:249-255`), each refusal in the engine's own error shape
  (`crates/engine-core/src/login_guard.rs:79-89`).
- The sync pairs: the sync login (1,3) is admitted on both transports, the normal sync (1,5) on
  the web alone (`crates/engine-core/src/table.rs:121-134`), and the one-way sync (1,6) runs only
  through the full-sync choice's write (`table.rs:326-332`, `crates/ffi/src/engine.rs:367`,
  `:404`). A private engine starts from the parent's start message with state of its own
  (`dispatch.rs:232-243`).
- **iPhone and iPad.** The first sync contact is `AppModel.signIn`: the login at
  `ios/App/Sources/AppModel.swift:76-77` (`EngineSession.login`, `EngineSession.swift:171-177`,
  the pair (1,3)) and then the first write, the host key's save, at `AppModel.swift:78`. A refusal
  is shown by the catch (`:80-82`) through `sentence` (`:96-98`) in `account-message`
  (`ios/App/Sources/AccountView.swift:29`). The static library runs the call at
  `crates/ffi/src/engine.rs:98-107`: its allow-list check (`:101-103`), then the core (`:104-106`).
- The Swift app may make no request of its own: `URLSession` and `URLRequest` are forbidden in
  every Swift file (`scripts/tests/test_ios_thin_swift.py:92-94`).
- The workspace makes reqwest's client at one allow-listed site: `clippy.toml:7-15` names its four
  paths, and `crates/notifications/tests/request_allow_list.rs:54` holds the named sites, the bot
  transport's (`crates/bot/src/transport.rs:403-407`).
- The iPhone and iPad UI test of a refused login expects a message that begins with a network
  sentence and nothing stored (`ios/AppUITests/ShellFlowTests.swift:21-24`, `:51-81`); the app's
  sync endpoint is a placeholder that cannot answer (`ios/Config/App.xcconfig:13`).
- **Web.** The web client's sync entry obtains its credential: a seal-key release
  (`web/app/src/lib/engine/credential.ts:197`), the login (`:199`), then the first write, the sealed
  credential's store (`:207-213`). At this commit the web engine's network refuses every sync
  request (SPEC-364 R14, replaced by its part b2), so no web client syncs yet.
- **Web, at `f7d62691`.** That commit holds SPEC-364 part b2, so the web client syncs through its
  Worker (`web/app/src/lib/engine/sync.ts`), and the core's rule (R5) refuses every web sync pair,
  because the Worker hands the core no statement. CI's web-engine job (job 113546776315, on a merge
  ref whose tree is `f7d62691`'s) failed the three sync specs on both browsers, six failures: the
  login read `offline` where `held` was expected (`web/app/tests-engine/sync.spec.ts:99`, `:125`),
  and no request reached the moved sync route (`:151`).
- **The internal build.** The internal lane runs on a manual dispatch or a push to `dev`
  (`.github/workflows/testflight-internal.yml:10-24`), one run per event at a time with none
  cancelled (`:32-35`), and uploads for internal testing only (`scripts/ios_lane.py:348`). An
  internal build stops working when its TestFlight lifetime ends, and nothing reads its age; a
  `dev` with no push lets the newest build lapse.
- Scheduled workflows run from the default branch's copy only (`.github/workflows/mutation-weekly.yml:17-18`).

## 2. Requirements (part one: the handshake)

R1. The engine core declares the client level, one positive integer `CLIENT_LEVEL`, defined once,
in a new module `crates/engine-core/src/handshake.rs`. Both clients compile it: the static library
through `crates/ffi`, the web engine through `crates/web-engine`.

R2. The API states the oldest client level it accepts, `MINIMUM_CLIENT_LEVEL`, defined once in a new
module `crates/api/src/minimum_client.rs`, at `GET /api/sync/minimum-client`. The answer is 200,
the body exactly `{"minimum_client_level":<n>}` as JSON, and `Cache-Control: no-store`, to any
caller, with or without an owner configured: the route is merged beside the health routes
(`crates/api/src/router.rs:216`), outside the owner's (`:217-259`), and reaches clients through the
edge's existing `/api/*` proxy with no edge change.

R3. Within one tree the minimum never exceeds the level: `1 <= MINIMUM_CLIENT_LEVEL <= CLIENT_LEVEL`,
held by a census test with a planted control, so no tree states a minimum its own clients fail.

R4. The core keeps the latest statement. `Dispatcher::handshake(statement: Option<&[u8]>)` decides
one of four outcomes: admitted (the body decodes and its minimum is at most `CLIENT_LEVEL`), below
(it decodes and its minimum is above `CLIENT_LEVEL`), undecodable (anything else, an empty body
included), and unread (`None`: no answer was read). A dispatcher starts unread. A later statement
replaces the outcome, so a later below or unread refuses again.

R5. Unless the latest outcome is admitted, the core refuses before the engine sees it, on both
transports: the sync login (1,3) and the normal sync (1,5) in `Dispatcher::run`, after the login
guard (`dispatch.rs:160-165`) and before the engine (`:166-169`); and the one-way sync (1,6) in
`Dispatcher::full_sync`, after its auth check (`:250`) and before the engine (`:252-254`). The
refusal is the login guard's shape, a `BackendError` of kind `INVALID_INPUT` carrying the outcome's
sentence. A refused call changes no row, writes no file and sends no request.

R6. The sentences, exact:
- below: `This version of DeckStreak is older than the oldest the sync service accepts. Update DeckStreak to sync.`
- unread: `A network error occurred. The sync service's oldest accepted version could not be read, so nothing was synced.`
- undecodable: `The sync service's statement of the oldest version it accepts could not be read, so nothing was synced.`

R7. A private engine shares its parent's outcome (`dispatch.rs:232-243`), so the fetch of a server
copy obeys the same latest statement as the write that asked for it.

R8. The static library reads the statement before every sync login. `Engine::run`
(`crates/ffi/src/engine.rs:98-107`), for (1,3) alone, after its allow-list check (`:101-103`) and
before the core (`:104-106`), sends `GET <origin>/api/sync/minimum-client`, where the origin is the
scheme, host and port of the login request's endpoint, derived by a core helper only for an
endpoint the login guard's rule admits (`login_guard.rs:101-116`). A login the rule refuses is
never read and keeps the guard's own refusal. The read sends no credential, follows no redirect,
is bounded by a 10-second timeout and a 1024-byte body, and hands the core `None` when nothing
answered and an empty body for any status but 200.

R9. The read is a named request site: one scoped `#[expect]` with its reason at the static
library's read, entered in `crates/notifications/tests/request_allow_list.rs`'s named sites. The
comment at `clippy.toml:7-8` names the allow-list instead of one site, and its four reason strings
stay as they are. SPEC-041's allow-list rule gains an insert-only amendment naming the second site.

R10. The read adds no package: `Cargo.lock` gains no `[[package]]` entry.

R11. On iPhone and iPad the stop lies before the first write: a refused login throws at
`AppModel.swift:76-77`, so the save at `:78` never runs, and the sentence reaches `account-message`.
No Swift file changes. SPEC-347's UI test of a refused login keeps its oracle: the placeholder
endpoint cannot answer, so the refusal begins `A network error occurred.`.

R12. On the web the same core rule (R5) refuses the sync pairs on `Transport::Web` until the web
client hands the core an admitting statement; the web client's read is R22 to R24.

R22. The web Worker reads the statement before its sync login and before each normal sync:
`GET /api/sync/minimum-client` at the Worker's own origin, through the Worker's own `fetch`, with
no credential, no cache and no redirect followed, bounded by a 10-second timeout and a 1024-byte
body. It hands the web engine nothing when no answer was read, and empty bytes for any status but
200 (a redirect included, which the browser answers unopened with status 0), for a body over the
bound, and for a body that could not be read. The read precedes the seal-key release, every request
under the sync route and the first write.

R23. The web engine's `handshake` export hands the Worker's statement to `Dispatcher::handshake`
and answers nothing when the outcome admits, or the outcome's R6 sentence otherwise. The core
decides; the export keeps no rule of its own.

R24. A statement the core does not admit stops the sync call: the Worker throws the sentence, the
credential store is not asked for a key, and the page's client rejects with the code
`engine-failed` and the sentence as its message. An admitting statement lets the login and the
normal sync proceed as SPEC-364 R17 and R18 state.

## 3. Acceptance criteria of part one

| # | criterion | decided by |
|---|---|---|
| A1 | Below the minimum, the core refuses the sync login on both transports, the normal sync on the web and the one-way write, each before the engine, with the below sentence, and no row or file changes | `crates/engine-core/tests/handshake.rs` |
| A2 | An unread, an undecodable and a never-read statement each refuse every sync pair, with the unread or undecodable sentence | `crates/engine-core/tests/handshake.rs` |
| A3 | The latest statement decides: an admitting one (minimum equal to the level included) lets the call reach the engine, a later below refuses again, and a private engine obeys its parent's latest outcome | `crates/engine-core/tests/handshake.rs` |
| A4 | A native login below the minimum sends only the statement's `GET`, no request under the sync path, and refuses with the below sentence | `crates/ffi/tests/handshake.rs` |
| A5 | A native login reads the statement first: an admitting one is followed by the login's request, an unanswered one refuses with a sentence that begins `A network error occurred.`, a redirect is not followed, and an endpoint the guard refuses is never read | `crates/ffi/tests/handshake.rs` |
| A6 | The service states its minimum to any caller: 200, the exact body and `Cache-Control: no-store`, with no owner configured | `crates/api/tests/minimum_client_route.rs` |
| A7 | Each level is defined once and the minimum never exceeds the level, and a planted tree that breaks it is refused | `scripts/tests/test_client_level.py` |
| A16 | With an admitting statement, a login and a normal sync from the web Worker reach the server, each through the page's own origin | `web/app/tests-engine/sync.spec.ts` |
| A17 | With an admitting statement, a refused key is dropped and a lost network keeps it | `web/app/tests-engine/sync.spec.ts` |
| A18 | With an admitting statement, a redirected sync answer is refused and nothing is synced | `web/app/tests-engine/sync.spec.ts` |
| A19 | The Worker reads the statement at its own origin with no credential, no cache and no redirect followed, and hands on a 200 body of at most 1024 bytes, empty bytes for any other answer and for an over-long or unreadable body, and nothing when no answer was read | `web/app/src/lib/engine/sync.test.ts` |
| A20 | A statement the core refuses stops the login and the normal sync before the store is asked for a key and before the engine syncs, with the core's sentence; an admitting one lets both through | `web/app/src/lib/engine/sync.test.ts` |
| A21 | The Worker's sync reads the statement through the Worker's own `fetch` and hands the engine its exact bytes before the sync login and before the normal sync | `web/app/src/lib/engine/worker.test.ts` |
| A22 | The `handshake` export hands the statement to the core's dispatcher and answers the core's own decision and sentence, keeping no rule of its own | `crates/web-engine/tests/boundary.rs` |

```acceptance
A1: cargo test -p deck-streak-engine-core --test handshake -- --exact a_client_below_the_minimum_is_refused_every_sync_before_the_engine
A2: cargo test -p deck-streak-engine-core --test handshake -- --exact an_unread_or_unreadable_statement_refuses_every_sync
A3: cargo test -p deck-streak-engine-core --test handshake -- --exact the_latest_statement_decides_and_a_private_engine_obeys_it
A4: cargo test -p deck-streak-ffi --test handshake -- --exact a_login_below_the_minimum_stops_before_any_sync_request_and_says_why
A5: cargo test -p deck-streak-ffi --test handshake -- --exact the_statement_is_read_first_and_only_an_admitting_one_reaches_the_login
A6: cargo test -p deck-streak-api --test minimum_client_route -- --exact the_service_states_its_minimum_client_level_to_any_caller
A7: python3 -m unittest discover -s scripts/tests -p test_client_level.py -k test_the_minimum_never_exceeds_the_level_a_tree_builds
A16: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "a login and a normal sync from the worker reach the server"
A17: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "a refused key is dropped and a lost network keeps it"
A18: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "a redirected sync answer is refused"
A19: pnpm --dir web/app exec vitest run src/lib/engine/sync.test.ts -t "the statement is read at the same origin with no credential, no cache and no redirect followed"
A20: pnpm --dir web/app exec vitest run src/lib/engine/sync.test.ts -t "a statement the core refuses stops the login and the sync before the store and says why"
A21: pnpm --dir web/app exec vitest run src/lib/engine/worker.test.ts -t "the worker reads the statement through its own fetch and hands the engine its bytes before each sync"
A22: cargo test -p deck-streak-web-engine --test boundary -- --exact each_boundary_function_reaches_the_engine_through_the_dispatcher
```

#671's first acceptance bullet is A1, A4 and A20: a client below the minimum stops before any sync
write and says why, in the core for both transports, in the static library the iPhone and iPad app
runs, and in the web Worker.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/engine-core/src/handshake.rs` | engine core | new: `CLIENT_LEVEL`, the outcome, its decision and sentences, the statement's URL helper |
| `crates/engine-core/src/lib.rs` | engine core | the new module |
| `crates/engine-core/src/dispatch.rs` | engine core | the shared outcome, `handshake`, the refusal in `run` and `full_sync`, `private` sharing it |
| `crates/engine-core/tests/handshake.rs` | engine core | new: A1 to A3 |
| `crates/engine-core/tests/login_guard.rs`, `crates/engine-core/tests/full_sync.rs`, `crates/engine-core/tests/one_way.rs`, `crates/engine-core/tests/containment.rs` | engine core | an admitting statement as setup where a sync pair must reach the engine; no assertion changes |
| `crates/ffi/src/engine.rs` | static library | the statement's read before a sync login, at its named site |
| `crates/ffi/Cargo.toml`, `Cargo.lock` | static library | the HTTP client and runtime the engine already links; no new package |
| `crates/ffi/tests/handshake.rs` | static library | new: A4, A5 |
| `crates/ffi/tests/support/sync_server.rs`, `crates/ffi/tests/support/mod.rs` | static library | the loopback server's origin also answers an admitting statement; no assertion changes |
| `crates/api/src/minimum_client.rs` | API | new: `MINIMUM_CLIENT_LEVEL`, the route |
| `crates/api/src/lib.rs` | API | the new module |
| `crates/api/src/router.rs` | API | the route merged beside the health routes |
| `crates/api/tests/minimum_client_route.rs` | API | new: A6 |
| `scripts/tests/test_client_level.py` | scripts | new: A7 |
| `crates/notifications/tests/request_allow_list.rs` | request allow-list | the static library's site named |
| `clippy.toml` | workspace | the comment names the allow-list; reasons unchanged |
| `crates/web-engine/src/wasm.rs` | web engine | the `handshake` export (R23) |
| `crates/web-engine/tests/boundary.rs` | web engine | the export's census row: A22 |
| `web/app/src/lib/engine/sync.ts` | web Worker | the statement's read and its hand-off before each sync login and normal sync (R22, R24) |
| `web/app/src/lib/engine/worker.ts` | web Worker | the Worker's own `fetch` handed to the sync |
| `web/app/src/lib/engine/sync.test.ts` | web Worker | new: A19, A20; the stub engine's `handshake` and the sync's `fetch` as setup |
| `web/app/src/lib/engine/worker.test.ts` | web Worker | new: A21; the stub engine's `handshake` as setup in the existing sync test |
| `web/app/src/lib/engine/credential-reach.test.ts` | web Worker | the stub engine's `handshake` and the sync's `fetch` as setup; no assertion changes |
| `web/app/vite.engine.config.ts` | engine tests | a stand-in statement at the same origin, admitting unless a test sets it, and the paths it was asked at |
| `web/app/tests-engine/sync.spec.ts` | engine tests | A16 to A18 unchanged; a new spec of a refused and a moved statement |
| `docs/specs/SPEC-041-*.md` (the file at the cut) | specs | an insert-only amendment naming the second site |
| `formal/tla/MinimumClientHandshake/MinimumClientHandshake.tla`, `formal/tla/MinimumClientHandshake/MCMinimumClientHandshake.cfg`, `formal/tla/MinimumClientHandshake/witness/*.cfg` | formal | new: section 8 |
| `scripts/mutation-rows.d/S37400-S37499.json` | rows | new band |
| `docs/specs/SPEC-374-a-client-below-the-sync-services-stated-minimum-stops-before-any-sync-and-an-internal-build-is-re-released-before-it-stops-working.md` | specs | this SPEC |
| `docs/decisions/ADR-385-the-sync-service-states-a-minimum-client-level-and-the-engine-core-refuses-every-sync-below-it.md` | decisions | its ADR |
| `docs/schematics/minimum-client-handshake-and-re-release-check.md` | schematics | the flows |
| `docs/red-first/SPEC-374.md` | records | new |
| `changelog.d/min-client-rerelease-374.md` | changelog | new fragment |
| `.github/workflows/testflight-rerelease-check.yml` | workflows | unchanged in part one (section 7) |
| `scripts/testflight_age.py` | scripts | unchanged in part one (section 7) |
| `scripts/tests/test_testflight_age.py` | scripts | unchanged in part one (section 7) |
| `scripts/tests/test_ci_workflows.py` | scripts | unchanged in part one (section 7) |

## 5. What this does NOT cover

- A web sync screen that tells a refused client from a failed engine by a code of its own and shows
  the sentence in the app's own words: the Worker's refusal carries the core's sentence as
  `engine-failed` (R24), and the sync screen is #631's next part.
- The iPhone and iPad sync screens and the full-sync choice on them, whose every sync entry reads
  the statement first (#633).
- Moving the internal lane to the app's scheme, which first ships a client that syncs (#625).
- A refusal by the sync server itself of a client below the minimum: the engine's sync protocol
  carries no client level, and a client built before this delivery cannot be stopped by it (#671
  records the scope; no client that syncs has shipped, #625).
- Raising the minimum: a later release raises `CLIENT_LEVEL` first and `MINIMUM_CLIENT_LEVEL` only
  in a later one, each its own delivery (#671).

## 6. Risks

- A statement route that is not deployed answers 404, so every client refuses with the undecodable
  sentence. Detected by A6 before a release and by the client's sentence after one; the route
  ships with the API in the same release as the clients that read it (#625 ships the first).
- A minimum raised between a client's read and its sync is obeyed at the next read, not the
  current call. The model in section 8 states the window; every sync entry reads first (R8, R22,
  #633).
- The web Worker reads the statement only where its own origin serves it, through the edge's
  existing `/api/*` proxy, so a deployment that serves the web app without the API refuses every
  web sync with the undecodable or unread sentence. Detected by A6 before a release and by the
  sentence after one, as for the native read.
- The browser specs read a stand-in statement that the engine tests' server answers at the same
  path as the API's route. Detected by A6 and A19, which each name the route's path as a literal,
  and by A16 to A18, which fail if the Worker reads anywhere else.
- A widened request allow-list could admit a request that is not the statement's. Detected by
  `request_allow_list.rs`, which names the site, and by the census of what the site requests (A4,
  A5 record every request).

## 7. Delivered by the next pull request (part two: the re-release check)

R13. A workflow `.github/workflows/testflight-rerelease-check.yml` runs on `schedule` (one cron,
once a day) and `workflow_dispatch`, and on nothing else. Its token is read-only by default; its
one job runs on a pinned hosted runner image in an environment of its own, holds `contents: read`
and `actions: write` and nothing more, and has its own concurrency group, cancelling nothing. Its
checkout is pinned by the full commit sha the internal lane uses, with the whole history and no
persisted credential.

R14. Its credential has four parts the owner places in that environment, each read by name in the
one step that uses it: the API key, its id, its issuer, and the app's id. A presence step reads
only whether each is placed. None placed fails the run with `the check's credential is not placed;
no build was read`; some placed fails it naming each absent part.

R15. `scripts/testflight_age.py` (standard library only) signs a short-lived ES256 token with
`openssl` from a key written to a new 0600 file in a 0700 directory removed on every exit, and
reads the app's builds, newest upload first, with `curl` taking its authorization header from a
0600 file. No credential part reaches a child's argv or environment, or the log.

R16. It decides on the newest build that is valid, unexpired and for internal testing only: healthy
when its expiration is more than the lead, 7 days, after now; otherwise due. No such build is due
(`no internal build is live`). A newer build still processing is reported and dispatches nothing.

R17. The lead covers the schedule's interval (7 days against one day), so a build that would stop
working before the next scheduled run is reported by this one.

R18. A due build starts a re-release: when `dev`'s first-parent count is above the build's number,
`gh workflow run testflight-internal.yml --ref dev` with the job's own token, and the lane builds
`dev`'s tip under its own triggers and concurrency. When `dev` has not moved past the build's
number, nothing is dispatched and the run fails with `dev has not moved since build <n>; a
re-release needs a new commit on dev`.

R19. It never creates or moves a tag, never names `main` as a ref, never dispatches any workflow but
the internal lane, and never releases.

R20. Every failure to read fails closed: an answer other than 200, a body that is not the expected
JSON, a signing failure or a git failure fails the run naming what failed. `healthy` is printed
only after a build was read.

R21. The run's summary names the build's number, its upload and expiration as the API gave them,
the verdict and the dispatch's outcome.

| # | criterion | delivered by |
|---|---|---|
| A8 | The check runs on a schedule and a manual dispatch only, its token read-only by default, its job writing only actions, and its credential reads admitted and no other | part two |
| A9 | A build that would stop working before the next scheduled run is reported and the internal lane dispatched on `dev` | part two |
| A10 | A build beyond the lead is reported healthy, only after it was read, and nothing is dispatched | part two |
| A11 | An absent or half-placed credential fails closed, reads nothing and dispatches nothing | part two |
| A12 | An unreadable answer fails closed and dispatches nothing | part two |
| A13 | A due build on an unmoved `dev` is reported and not dispatched | part two |
| A14 | No credential part reaches a child's argv or environment, or the log | part two |
| A15 | The lead covers the schedule's interval | part two |

## 8. Formal model

Part one is a check followed by an act on state another actor changes in between: the client
reads the stated minimum, then syncs, and the service can raise it between the two; a private
engine acts on its parent's outcome. A TLA+ entry, `formal/tla/MinimumClientHandshake/`, models a
service that raises its minimum, an adapter that reads it or fails to, and two engines (a parent
and a private one) that start syncs. Its property, `EverySyncFollowsAnAdmittingRead`, holds that
every sync started was preceded, in its engine's shared outcome, by a read whose minimum was at
most the level. Three witnesses each switch one defect on and must be caught: a sync with no gate,
a comparison that admits a minimum above the level, and a private engine that starts admitted. It
covers `handshake.rs` and `dispatch.rs`'s handshake, refusal and private-engine items and the
static library's `run`, and cites #671. Part two adds no interleaving the lane does not already
hold: ADR-385 D8 records why it owes no model.

The web Worker's read (R22) is the model's adapter on the web: it reads the statement or fails to,
and the core's shared outcome decides. The web engine's `handshake` export keeps no rule of its own
(R23, A22), so the entry's covered items stay the core's and the static library's.

## 9. Amendments: the lines part one's cut reads

Every `path:line` above was read at `6f9ef860`. Part one is cut at `6d71bec0`, after #730 changed
`crates/engine-core/src/dispatch.rs` and `crates/engine-core/src/table.rs`; the same items, read
at the cut, are these, and no requirement changes:

- `Dispatcher::start` is `dispatch.rs:146-154` (section 1's `:134-142`).
- In `Dispatcher::run`, the login guard is `dispatch.rs:172-177` and the engine's call `:178-181`
  (R5's `:160-165` and `:166-169`, section 1's `:157-169`).
- `Dispatcher::private` is `dispatch.rs:296-307` (R7's `:232-243`).
- `Dispatcher::full_sync` is `dispatch.rs:313-319`, its auth check `:314` and the engine's call
  `:316-318` (R5's `:249-255`, `:250` and `:252-254`).
- The sync login's and the normal sync's rows are `table.rs:123-136`, and the one-way sync's
  `:329-335` (section 1's `:121-134` and `:326-332`).

Three files section 4 names need no setup line, and part one leaves them as they are; each passes
at the implementation commit with no statement handed to its dispatcher:

- `crates/engine-core/tests/full_sync.rs` is unchanged: its dispatcher reads the collection's ids
  and drives no sync pair that must reach the engine.
- `crates/engine-core/tests/containment.rs` is unchanged: it is a census of the source, and drives
  no sync.
- `crates/ffi/tests/support/mod.rs` is unchanged: the admitting statement is answered by the
  support module's sync server, the file section 4 names beside it.

The diff's own mutants add one file to section 4's list:

- `scripts/mutation-equivalent.d/deck-streak-engine-core.json` gains one record: deleting the
  `kind` field of the refusal `admits` builds leaves the same bytes, because that kind is the
  proto3 default, so no test can tell the two apart.

## 10. Amendments, part two: the re-release check lands

Part two delivers section 7 as it is written: R13 to R21, decided by A8 to A15. Section 7 and its table stay as they are; section 11 restates the criteria with the commands that decide them.

- The check reads with a credential of its own, held in an environment of its own that only the default branch may enter. ADR-385's amendment records the decision and the alternative it was chosen against.
- Placing that credential is an owner act outside this delivery. Until it is placed, every run fails closed, says the credential is not placed, reads nothing and dispatches nothing (R14, A11).
- Files: `.github/workflows/testflight-rerelease-check.yml` (new), `scripts/testflight_age.py` (new), `scripts/tests/test_testflight_age.py` (new), `scripts/tests/test_ci_workflows.py`, `docs/red-first/SPEC-374.md`, `scripts/mutation-rows.d/S37400-S37499.json`, `changelog.d/min-client-rerelease-374-b.md`.

## 11. Acceptance criteria of the part-two amendment

| # | criterion | decided by |
|---|---|---|
| A8 | The check runs on a schedule and a manual dispatch only, its token read-only by default, its job writing only actions, and its credential reads admitted and no other | `scripts/tests/test_ci_workflows.py`, in CI |
| A9 | A build that would stop working before the next scheduled run is reported and the internal lane dispatched on `dev` | `scripts/tests/test_testflight_age.py` |
| A10 | A build beyond the lead is reported healthy, only after it was read, and nothing is dispatched | `scripts/tests/test_testflight_age.py` |
| A11 | An absent or half-placed credential fails closed, reads nothing and dispatches nothing | `scripts/tests/test_testflight_age.py` |
| A12 | An unreadable answer fails closed and dispatches nothing | `scripts/tests/test_testflight_age.py` |
| A13 | A due build on an unmoved `dev` is reported and not dispatched | `scripts/tests/test_testflight_age.py` |
| A14 | No credential part reaches a child's argv or environment, or the log | `scripts/tests/test_testflight_age.py` |
| A15 | The lead covers the schedule's interval | `scripts/tests/test_ci_workflows.py`, in CI |

```acceptance
A8: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a8_the_rerelease_check_runs_scheduled_with_admitted_reads_only
A9: python3 -m unittest discover -s scripts/tests -p test_testflight_age.py -k test_a9_a_build_due_within_the_lead_dispatches_the_lane_on_dev
A10: python3 -m unittest discover -s scripts/tests -p test_testflight_age.py -k test_a10_a_build_beyond_the_lead_is_healthy_and_dispatches_nothing
A11: python3 -m unittest discover -s scripts/tests -p test_testflight_age.py -k test_a11_an_absent_or_half_placed_credential_fails_closed
A12: python3 -m unittest discover -s scripts/tests -p test_testflight_age.py -k test_a12_an_unreadable_answer_fails_closed
A13: python3 -m unittest discover -s scripts/tests -p test_testflight_age.py -k test_a13_a_due_build_on_an_unmoved_dev_is_not_dispatched
A14: python3 -m unittest discover -s scripts/tests -p test_testflight_age.py -k test_a14_no_credential_part_reaches_a_child_or_the_log
A15: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a15_the_rerelease_lead_covers_the_schedule_interval
```
