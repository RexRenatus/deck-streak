# The minimum-client handshake and the re-release check (SPEC-374, ADR-385)

Every `path:line` below was read at `6f9ef860`. Part one is the handshake (sections 1 to 3);
part two is the re-release check (section 4). Section 5 says where each test pins each flow.

## 1. The service's statement

```mermaid
flowchart LR
    client["a client's read"] --> edge["edge: the api proxy, unchanged"]
    edge --> router["api router: health routes and the minimum route, served with or without an owner"]
    router --> stmt["GET /api/sync/minimum-client: 200, minimum_client_level, no-store"]
    level["engine core: CLIENT_LEVEL"] -. "census: minimum at most level" .- minimum["api: MINIMUM_CLIENT_LEVEL"]
    minimum --> stmt
```

The route is merged beside `health::routes()` (`crates/api/src/router.rs:216`), outside the owner's
routes (`:217-259`), and the edge already proxies `/api/*` to the API
(`deploy/caddy/deck-streak.caddy:46-52`).

## 2. iPhone and iPad: from the first sync contact to the stop or the sync

```mermaid
sequenceDiagram
    participant View as AccountView
    participant Model as AppModel.signIn
    participant Lib as static library Engine.run
    participant Core as engine core Dispatcher
    participant Api as the API statement route
    participant Sync as the sync server
    View->>Model: sign in
    Model->>Lib: login, the pair 1,3, AppModel.swift 76-77
    Lib->>Lib: allow-list check, engine.rs 101-103
    Lib->>Core: statement URL from the login endpoint, under the guard's endpoint rule
    alt the guard refuses the endpoint
        Lib->>Core: run, no read
        Core-->>Model: the guard's own refusal
    else the endpoint passes the rule
        Lib->>Api: GET the statement, no redirect, bounded time and size
        Api-->>Lib: the statement, or no answer
        Lib->>Core: handshake with the statement or none
        Lib->>Core: run, engine.rs 104-106
        Core->>Core: login guard, dispatch.rs 160-165
        alt the latest outcome is admitted
            Core->>Sync: the engine's login
            Sync-->>Model: host key
            Model->>Model: first write, the host key saved, AppModel.swift 78
        else below, undecodable or unread
            Core-->>Model: refusal in the engine's error shape with the outcome's sentence
            Model->>View: the sentence in account-message, nothing saved
        end
    end
```

The stop sits in the core's `run` after the login guard and before the engine
(`crates/engine-core/src/dispatch.rs:160-169`), reached from `AppModel.swift:76-77`, so the first
write at `AppModel.swift:78` never runs on a refusal. No Swift file changes.

## 3. The web client and every engine on both transports

```mermaid
sequenceDiagram
    participant Page as page client
    participant Worker as web Worker sync, sync.ts
    participant Service as sync service, at the Worker's own origin
    participant Core as engine core Dispatcher, Transport Web
    participant Private as private engine
    participant Engine as the engine
    Page->>Worker: a sync login, or a normal sync
    Worker->>Service: GET the statement, no credential, no cache, redirect manual
    alt answered 200 with a body of at most 1024 bytes
        Service-->>Worker: the body
    else any other status, a redirect included, or an over-long or unreadable body
        Service-->>Worker: an answer read as empty bytes
    else no answer within 10 seconds
        Worker->>Worker: nothing read
    end
    Worker->>Core: the handshake export with what was read, the core decides
    alt admitted
        Core-->>Worker: no sentence
        Worker->>Core: run the pair 1,3 or 1,5
        Core->>Engine: the call
    else below, undecodable or unread
        Core-->>Worker: the outcome's sentence
        Worker-->>Page: refusal engine-failed with the sentence, before the seal-key release and any sync request
    end
    Core->>Private: start, sharing the latest outcome
    Private->>Private: full sync, the pair 1,6, refused unless admitted
```

From part one's fix round the Worker's sync is `web/app/src/lib/engine/sync.ts`: `login()` hands
the credential store a login, whose `obtain()` (`web/app/src/lib/engine/credential.ts`) runs the
seal-key release, the login and then the first write, and `sync()` takes the key for one send.
Each reads the statement first, through the Worker's own `fetch`, and hands it to the web engine's
`handshake` export, which hands it to the core and answers the core's sentence on a refusal, so a
refusal precedes the seal-key release, every request under the sync route and the first write. The
read follows no redirect: the browser hands a redirect back unopened, with status 0, and the Worker
reads it as empty bytes, which the core decides is undecodable, as the static library's read does.
The refusal reaches the page as `engine-failed` with the core's sentence (SPEC-374 R22 to R24). The
core's own rule stays beneath it: a sync pair sent without an admitting statement is still refused
before the engine (R5, R12).

## 4. The scheduled re-release check (part two)

```mermaid
flowchart TD
    trigger["schedule once a day, or a manual dispatch"] --> pre["presence step: four credential parts"]
    pre -->|none placed| failnone["fail: not placed, no build was read"]
    pre -->|some placed| failhalf["fail: names each absent part"]
    pre -->|all placed| sign["sign a short-lived token, key in a private file"]
    sign -->|signing fails| failread["fail closed, names what failed"]
    sign --> read["read the app's builds, newest upload first"]
    read -->|not 200 or not the expected body| failread
    read --> pick["newest valid, unexpired, internal-only build"]
    pick -->|a newer build is processing| processing["report processing, dispatch nothing"]
    pick -->|none live| due["due"]
    pick -->|expires within the lead| due
    pick -->|expires after the lead| healthy["report healthy after the read, dispatch nothing"]
    due --> moved{"dev's first-parent count above the build's number"}
    moved -->|yes| dispatch["dispatch the internal lane on dev with the job token"]
    moved -->|no| unmoved["fail: dev has not moved since the build"]
    dispatch --> lane["the internal lane builds dev's tip under its own triggers and concurrency"]
```

The lane accepts a manual dispatch on `dev` (`.github/workflows/testflight-internal.yml:10-24`,
`scripts/ios_lane.py:99-135`) and numbers the build by `dev`'s first-parent count
(`scripts/ios_lane.py:134`). The check never tags, never names `main`, and never releases.

## 5. Where each test pins each flow

| flow step | pinned by |
|---|---|
| the statement's route, body and cache header, served with no owner | SPEC-374 A6, `crates/api/tests/minimum_client_route.rs` |
| the minimum never above the level | A7, `scripts/tests/test_client_level.py` |
| the core's refusal before the engine, both transports, every sync pair | A1, A2, `crates/engine-core/tests/handshake.rs` |
| the latest statement decides; a private engine obeys its parent | A3, `crates/engine-core/tests/handshake.rs` |
| the native read before the login, nothing under the sync path below the minimum, the sentence | A4, `crates/ffi/tests/handshake.rs` |
| the read's order, the unanswered read's network sentence, no redirect, no read for a refused endpoint | A5, `crates/ffi/tests/handshake.rs` |
| the stop before the first Swift write | `AppModel.swift:76-78` order, unchanged; SPEC-347's UI test of a refused login, unchanged |
| the read followed by the sync, with a raise in between | `formal/tla/MinimumClientHandshake/` |
| the Worker's read: its own origin, no credential, no cache, no redirect followed, its bounds | A19, `web/app/src/lib/engine/sync.test.ts` |
| a refused statement stops the login and the normal sync before the store, with the core's sentence | A20, `web/app/src/lib/engine/sync.test.ts` |
| the Worker's own `fetch`, and the statement's bytes handed on before each sync | A21, `web/app/src/lib/engine/worker.test.ts` |
| the export reaches the core's dispatcher and keeps no rule of its own | A22, `crates/web-engine/tests/boundary.rs` |
| the Worker's login and normal sync, a refused key and a redirected sync, each after an admitting statement | A16 to A18, `web/app/tests-engine/sync.spec.ts` |
| a below and a moved statement in a browser: the sentence, no sync request, the redirect not followed | the new spec in `web/app/tests-engine/sync.spec.ts` (coverage beside A20, not a criterion) |
| the schedule, the token's permissions, the admitted credential reads | A8, `scripts/tests/test_ci_workflows.py` |
| a build due before the next run reported and dispatched on dev | A9, `scripts/tests/test_testflight_age.py` |
| healthy only after a read | A10, `scripts/tests/test_testflight_age.py` |
| absent or half-placed credential, unreadable answer | A11, A12, `scripts/tests/test_testflight_age.py` |
| unmoved dev | A13, `scripts/tests/test_testflight_age.py` |
| no credential part in argv, environment or log | A14, `scripts/tests/test_testflight_age.py` |
| the lead covers the schedule | A15, `scripts/tests/test_ci_workflows.py` |
