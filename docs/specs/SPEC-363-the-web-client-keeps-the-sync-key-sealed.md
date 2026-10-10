# SPEC-363: the web client keeps the sync host key sealed under a key the service releases only to the owner's live session, opens it only in the Worker, and drops it by one generation rule both clients share

- **Campaign row:** the app campaign, SPEC-334 row 1.4 (R5, R6) and R19, and the review row 1.3
  names (R9). **Issue:** #654 (SEC01-F12, recorded by SEC-01's review, #618). It binds #631: no
  browser sync is built before this is decided. **Context(s):** `deck-streak-engine-core`
  (`crates/engine-core`), `deck-streak-web-engine` (`crates/web-engine`), `deck-streak-identity`
  (`crates/identity`), `deck-streak-api` (`crates/api`), the daemon's wiring (`crates/daemon`),
  `miniapp` (`web/app/src`) and formal (`formal/tla`).
- **Decided by:** ADR-374 (this SPEC's own: where the browser holds the key, what it holds, how
  the sealing key is made and released, where the store lives, the generation rule and where it
  lives, when a key is dropped, how the key relates to the web session, removal, what each part
  of the client can reach, and the model), under ADR-335 and ADR-358 D2 (the native client's host
  key in the Keychain, the password held only for the call), ADR-347 D13 (the key's class),
  ADR-351 D4 and D7 (no header and no cookie on the sync route), ADR-340 (browser sync is
  same-origin), ADR-024 and ADR-370 (the web session, unchanged), ADR-368 (one rule in the core,
  the precedent) and ADR-352 (the card frame, unchanged).
- **Schematic:** `docs/schematics/web-sync-credential.md` (the components, the key's life from
  obtain to removal, the store's states, and the model's shape; this delivery adds it).
- **Status:** part 1 of 2. This pull request writes the model first, then the one rule both clients
  share, the web engine's exports of it, and the service's release of the sealing key (sections 2
  and 3). Part 2, the Worker's store, its operations, the page's sign-out and the reach census, is
  section 7. **Mutation band:** `S36300-S36399` (section 9; rows `S36301` to `S36316`).
  **Model:** `formal/tla/SyncCredential` (section 8).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `33033b65` (`D` below), or in the engine fork at its pin,
commit `c538de55` (`F` below, the fork's own tree). Nothing was run: every figure is a read.

### 1.1 What is decided, and what is not

| id | measured | figure | command |
|---|---|---|---|
| M1 | No design decides where the web client holds the sync credential, nor how it is obtained, stored, replaced and removed; the issue binds #631 | the issue's body | `gh issue view 654 -R RexRenatus/deck-streak --json body` |
| M2 | The review that found it scoped "where and how the browser holds the sync credential"; no document names the finding | the review's scope, second bullet; `git grep` prints nothing and exits 1 | `gh issue view 618 -R RexRenatus/deck-streak --json body`; `git grep -n F12 D -- docs` |
| M3 | The native client's credential is decided: the sync login and the bearer token sit in the Keychain | SPEC-334 R19 (lines 106-109) | `git show D:docs/specs/SPEC-334-*.md \| grep -n '^R19\.'` |
| M4 | The native client stores the host key alone, in one item for this device only, not synchronizable; the password is held only for the call | ADR-358 D2 (lines 57-77) | `git show D:docs/decisions/ADR-358-*.md \| sed -n '57,77p'` |
| M5 | The host key is a secret of the password's class: derived from the user and the stored hash, it never expires, and only a new hash retires it | ADR-347 D13 (lines 315-331) | `git show D:docs/decisions/ADR-347-*.md \| sed -n '315,331p'` |
| M6 | A client sends the host key in the `anki-sync` request header the engine writes; the client's auth record holds the key and the endpoint | `SYNC_HEADER_NAME` (line 138); `SyncAuth { hkey, endpoint, io_timeout_secs }` (lines 15-19) | `grep -n SYNC_HEADER_NAME rslib/src/sync/request/header_and_stream.rs`; `grep -n -A4 'pub struct SyncAuth' rslib/src/sync/login.rs`, in `F` |
| M7 | The sync server's refusal of a key is the one answer a client can read as "this key is no longer good": a 403 becomes `AuthFailed`, kind `SYNC_AUTH_ERROR` (7); a failed connection is `NETWORK_ERROR` (6) | lines 111-114; lines 31-32 | `grep -n -A3 'S::FORBIDDEN' rslib/src/error/network.rs`; `grep -n -E 'NETWORK_ERROR\|SYNC_AUTH_ERROR' proto/anki/backend.proto`, in `F` |
| M8 | The edge answers no 403 of its own on the sync route: only a 308 from the bare path to the slash form, 404 for a respelling or the health route, and the body bound | the two sync handles, lines 56-86: `redir ... 308` once (57), `respond ... 404` twice (69, 72), `max_size` once (75), and no 403 | `git show D:deploy/caddy/deck-streak.caddy \| sed -n '56,86p' \| grep -n -E 'redir .*308\|respond .*404\|max_size\|403'` |
| M9 | The edge logs the sync route with no request header and no `k` parameter, and passes no cookie either way on it | lines 28, 35-36 and 80-81; ADR-351 D4 (line 72) and D7 (lines 105-107) | `git show D:deploy/caddy/deck-streak.caddy \| grep -n -E 'headers delete\|not_sync\|-Cookie\|-Set-Cookie'` |

### 1.2 What the web client holds

| id | measured | figure | command |
|---|---|---|---|
| M10 | The page runs script from its own origin, from Telegram's origin and from the build's hashed inline scripts, and connects to its own origin only | `script-src` (line 27); `connect-src 'self'` (line 30) | `git show D:web/app/svelte.config.js \| grep -n -E "'script-src'\|'connect-src'"` |
| M11 | Every route of the page loads Telegram's script | `app.html` line 6 | `git show D:web/app/src/app.html \| grep -n telegram-web-app.js` |
| M12 | The engine runs in one dedicated Worker that answers only its own origin's messages | `serve` (lines 39-46) | `git show D:web/app/src/lib/engine/worker.ts \| sed -n '36,47p'` |
| M13 | The Worker's protocol has no sync and no credential operation | `OPS`, 15 operations (lines 8-24) | `git show D:web/app/src/lib/engine/protocol.ts \| sed -n '8,24p'` |
| M14 | The core admits the engine's sync login on the native transport only | `BackendSyncService.SyncLogin` `native: true, web: false` (lines 115-120) | `git show D:crates/engine-core/src/table.rs \| sed -n '115,120p'` |
| M15 | The core already guards the login's endpoint for both transports: `https`, or plain `http` to a loopback literal, with no user or password in the URL | `login_guard.rs`, 79 lines | `git show D:crates/engine-core/src/login_guard.rs \| wc -l` |
| M16 | Web storage today: the input mapping alone uses `localStorage`; nothing stores a credential | 2 lines in `web/app/src/lib/study/input.ts` (18, 27); the other 3 files that match are tests | `git grep -c -i -E 'indexedDB\|localStorage\|sessionStorage' D -- web/app/src` |
| M17 | A card renders in a sandboxed frame with no same-origin, no script and its own policy, reaching neither the app nor the network | SPEC-341 and ADR-352 | `git ls-tree -r --name-only D -- docs/specs docs/decisions \| grep -E 'SPEC-341\|ADR-352'` |

### 1.3 What the service holds

| id | measured | figure | command |
|---|---|---|---|
| M18 | The web session is one host-prefixed cookie; it ends 30 minutes after its last request and 8 hours after it began; 8 live at most; sessions live in memory | lines 36-42; ADR-024 | `git show D:crates/identity/src/session.rs \| grep -n -E 'pub const (SESSION_COOKIE\|IDLE_TIMEOUT\|ABSOLUTE_LIFETIME\|MAX_LIVE_SESSIONS)'` |
| M19 | Logging out ends the session on the server and clears the cookie | `DELETE /api/session` (lines 36, 157-167) | `git show D:crates/api/src/session_routes.rs \| sed -n '157,167p'` |
| M20 | A session opened by a passkey runs no command with stakes; removing a passkey ends the sessions it opened | SPEC-359 R14; ADR-370 D5 | `git grep -n -E '^R14\.' D -- docs/specs/SPEC-359-*`; `git grep -n 'D5. Removing a passkey' D -- docs/decisions/ADR-370-*` |
| M21 | The API process holds no sync credential: its unit loads two credentials, the owner's id and the bot's token; the sync login's two credentials belong to the ingest's sync job | lines 22-23; the job's drop-in, lines 4-5 | `git grep -n 'LoadCredential=' D -- deploy/systemd/deck-streak-api.service 'deploy/systemd/deck-streak-job@sync.service.d/20-sync-login.conf'` |
| M22 | The privacy policy says nothing of what a browser keeps | 0 lines | `git show D:PRIVACY.md \| grep -c -i -E 'browser\|this device'` |
| M23 | No model names a credential; the formal budget file is pinned byte for byte | 8 entries in `budgets.entries`, none a credential; `EXPECTED` in the config test | `git show D:config/formal.json`; `git grep -n 'EXPECTED = ' D -- scripts/tests/test_formal_config.py` |
| M24 | SPEC-357's parts b, c and d wait on this decision | SPEC-357 lines 62 (M23), 198-199, 212-214 and 236 | `git show D:docs/specs/SPEC-357-*.md \| sed -n '62p;198,199p;212,214p;236p'` |

**What follows.** The key the browser would hold never expires (M5) and is a password in all but
name. Every store a page can reach is readable by every script the page runs (M10, M11), and a
copy of the browser's files carries every store with it. The native client answers this with the
Keychain (M4); a browser has no Keychain. What the browser does have is a cookie no script can
read (M18), held by a service that already pins every session to the one owner. So the key is
kept sealed, and the key that opens it stays with the service, released only to the owner's live
session. The order of the key's life (a login in flight, a removal, a refusal after a rotation, a
second tab) is the part a client can get wrong without noticing, so the model comes first and the
rule it covers lives once, in the core both clients share.

### 1.4 One copy for both clients

| lives in | holds | the web client adds | the iOS client adds (#633) |
|---|---|---|---|
| `crates/engine-core/src/credential.rs` | the generation, when a login is kept, when a key may be sent, which answer is a refusal, when a refusal drops a key, removal | nothing | nothing |
| `formal/tla/SyncCredential` | the order both clients run | nothing | nothing |
| the client's store | where the key rests and how it is protected | sealed record in the origin's database, opened in the Worker (part 2) | the Keychain item ADR-358 D2 decided |
| the client's screens | sign-in to sync, sign-out, the status | `web/app/src` (#631 part c) | the iOS app |

## 2. Requirements (part 1)

R1. **The model first.** `formal/tla/SyncCredential` models the owner's login in flight, a
    removal, two Workers, the sync server's refusal after a rotation, a Worker's restart, a
    network failure and the web session's opening and end, over the stored generation, the sealed
    record and each Worker's open key. It states five properties at `ramp=report`, each with a
    witness the formal check must catch, plus `TypeOK` (section 8). The model and its witnesses are
    committed before the rule they cover, and the red-first record says so.

R2. **Its budget.** `config/formal.json` gives the entry a time budget set from its measured run,
    and `scripts/tests/test_formal_config.py`'s `EXPECTED` pins it.

R3. **One rule, in the core.** `crates/engine-core/src/credential.rs` holds `Generation`, a count
    that rises at every kept login and every removal, never falls, and refuses to pass its maximum
    rather than wrap. `on_obtained(started, current)` keeps a login only when no login was kept and
    no removal happened since it started, and names the generation it is kept at.
    `may_send(held, current, sealed)` admits a send only when a sealed record is stored and its
    generation is the one the sender holds. `classify(answer)` reads a sync's answer: only an
    engine error of kind `SYNC_AUTH_ERROR` is a refusal; success is accepted; every other answer,
    a network failure, another sync error, a server message and an answer that does not decode
    included, is a failure that keeps the key. `on_outcome(sent, current, outcome)` drops a key
    only on a refusal of the current generation. `on_removed(current)` names the next generation.
    The module does no I/O, reads no clock and adds no dependency.

R4. **The web engine exports the rule.** `crates/web-engine/src/wasm.rs` exports one function per
    rule of R3, each calling the core and deciding nothing itself, and the boundary census holds
    each export to its call.

R5. **The sealing key.** `crates/identity/src/sync_seal.rs` makes a 32-byte key from the seal
    secret and a 16-byte seal id: HMAC-SHA-256 under the secret, over a fixed label and the id.
    The secret is read once at start from the API's credentials, under the unit credential role
    `sync-seal-secret`; one shorter than 32 bytes refuses start by name; an absent one turns the
    release route off. Its `Debug` prints no byte of the secret. The API unit's `LoadCredential=`
    line for that role ships with the owner's host step (#161), not in this delivery, because a
    unit that names a missing credential fails to start.
    The sealing key is identity's second finalized HMAC, so this extends SPEC-024 A6's finalize
    census by this one site.

R6. **The release.** `POST /api/sync/seal-key` releases the key for one seal id to an owner session
    (proof `telegram` or `linked`); a `link` session and a request with no session are refused
    401. The body is JSON with exactly one field, `seal_id`, 22 base64url characters without
    padding that decode to 16 bytes; anything else is refused 400 `seal_id_invalid`. It is a
    state-changing request in the session routes' sense (JSON and not cross-site, refused 403
    otherwise), bounded by a window of its own of 30 releases a minute per process (the 31st is
    refused 429 `too_many_releases` with `Retry-After`), and it answers 200 with
    `{"key": "<43 base64url characters>"}` and `Cache-Control: no-store`. With no seal secret
    configured it answers 404 `sync_seal_off`. A `linked` session is admitted consistently with
    SPEC-359 R14 and CHARTER 14: the release is none of the four commands with stakes, it writes
    nothing and changes no session, and it carries the owner gate the amended CHARTER 14 asks of
    every surface.

R7. **No secret in a record.** The seal secret, a seal id and a released key never reach a log
    line, a span field, an error or a response body other than the one that releases the key
    (SPEC-359 R13's rule, applied to this route).
    A16's capture is one more routed capture the kernel's capture census counts.

R8. **Nothing else moves.** The web session's cookie, bounds and routes, the ceremony routes, the
    edge's sync route and its log, the card frame and its policy, and the core's table are
    unchanged: `SyncLogin` stays `web: false` until SPEC-357's part b moves it. The core gains no
    dependency.

## 3. Acceptance criteria (part 1)

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | The formal budget file holds the new entry, and the test pins it | `EXPECTED` gains the entry before the file does: the committed text differs | `scripts/tests/test_formal_config.py` `the_committed_file_holds_exactly_the_declared_fields` |
| A2 | A login is kept only at the generation it started at, and names the next one; one that started before a removal or another kept login is discarded | a stub that keeps every login | `crates/engine-core/tests/credential.rs` `a_login_is_kept_only_at_the_generation_it_started` |
| A3 | A send is admitted only when a sealed record is stored at the generation the sender holds | a stub that admits every send | `credential.rs` `a_send_needs_the_held_generation_to_be_current` |
| A4 | Only the sync server's refusal is a refusal: `SYNC_AUTH_ERROR` drops nothing by itself but classifies as refused; network, other sync, server-message and undecodable answers classify as failures | a stub that classifies every error as refused | `credential.rs` `only_the_sync_servers_refusal_is_a_refusal` |
| A5 | A refusal drops the key only when it refused the current generation; a failure never drops it | a stub that drops on every refusal | `credential.rs` `a_refusal_drops_only_the_generation_it_refused` |
| A6 | A removal names the next generation, and the generation never falls and never wraps at its maximum | a stub that returns the generation it was given | `credential.rs` `a_removal_raises_the_generation_and_it_never_wraps` |
| A7 | The core still depends on no crate of this workspace | (unchanged; green at the base) | `crates/engine-core/tests/graph.rs`, whole |
| A8 | Each credential export of the web engine calls its core rule and decides nothing itself | the census names the exports before they exist | `crates/web-engine/tests/boundary.rs` `each_boundary_function_reaches_the_engine_through_the_dispatcher` |
| A9 | The sealing key equals a fixed vector: HMAC-SHA-256 under a known secret over the label and a known seal id | a stub that returns zeros | `crates/identity/tests/sync_seal.rs` `the_seal_key_is_the_labelled_hmac_of_the_seal_id` |
| A10 | A seal secret under 32 bytes refuses start by name, and its `Debug` prints none of it | a stub that accepts any secret | `sync_seal.rs` `a_seal_secret_under_32_bytes_refuses_start` |
| A11 | The key is released to a `telegram` and a `linked` session, and refused 401 to a `link` session and to no session | a stub route that releases without a session | `crates/api/tests/sync_seal_routes.rs` `the_seal_key_is_released_only_to_an_owner_session` |
| A12 | A seal id that is not 16 bytes in unpadded base64url, or a body with another field, is refused 400 `seal_id_invalid` | a stub that releases for any body | `sync_seal_routes.rs` `a_seal_id_that_is_not_sixteen_bytes_is_refused` |
| A13 | The release has a window of its own: 30 in a minute pass, the 31st is refused 429 with `Retry-After`, and the handshake's window is untouched | a stub with no window | `sync_seal_routes.rs` `the_release_has_a_bound_of_its_own` |
| A14 | The release answers `Cache-Control: no-store`, and a cross-site or non-JSON request is refused 403 | a stub with no header and no guard | `sync_seal_routes.rs` `the_release_is_not_stored_and_not_cross_site` |
| A15 | With no seal secret the route answers 404 `sync_seal_off` | a stub that releases a key made from nothing | `sync_seal_routes.rs` `the_release_is_off_without_the_seal_secret` |
| A16 | No seal secret, seal id or released key reaches a captured log line or span, beside a planted line the capture must catch | a stub that logs the seal id | `sync_seal_routes.rs` `no_seal_secret_id_or_key_reaches_a_record` |
| A17 | The daemon starts with the release off when the API's credentials hold no seal secret, and refuses start, naming the credential's role, when it holds one under 32 bytes | a stub wiring that refuses start on a missing secret | `crates/daemon/src/wiring.rs` `a_missing_seal_secret_turns_the_release_off` |
| A18 | A seal id parses only from 22 unpadded base64url characters that decode to 16 bytes | a stub that parses any decoded length | `crates/identity/tests/sync_seal.rs` `a_seal_id_that_is_not_sixteen_bytes_does_not_parse` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k the_committed_file_holds_exactly_the_declared_fields
A2: cargo test -p deck-streak-engine-core --test credential -- --exact a_login_is_kept_only_at_the_generation_it_started
A3: cargo test -p deck-streak-engine-core --test credential -- --exact a_send_needs_the_held_generation_to_be_current
A4: cargo test -p deck-streak-engine-core --test credential -- --exact only_the_sync_servers_refusal_is_a_refusal
A5: cargo test -p deck-streak-engine-core --test credential -- --exact a_refusal_drops_only_the_generation_it_refused
A6: cargo test -p deck-streak-engine-core --test credential -- --exact a_removal_raises_the_generation_and_it_never_wraps
A7: cargo test -p deck-streak-engine-core --test graph
A8: cargo test -p deck-streak-web-engine --test boundary -- --exact each_boundary_function_reaches_the_engine_through_the_dispatcher
A9: cargo test -p deck-streak-identity --test sync_seal -- --exact the_seal_key_is_the_labelled_hmac_of_the_seal_id
A10: cargo test -p deck-streak-identity --test sync_seal -- --exact a_seal_secret_under_32_bytes_refuses_start
A11: cargo test -p deck-streak-api --test sync_seal_routes -- --exact the_seal_key_is_released_only_to_an_owner_session
A12: cargo test -p deck-streak-api --test sync_seal_routes -- --exact a_seal_id_that_is_not_sixteen_bytes_is_refused
A13: cargo test -p deck-streak-api --test sync_seal_routes -- --exact the_release_has_a_bound_of_its_own
A14: cargo test -p deck-streak-api --test sync_seal_routes -- --exact the_release_is_not_stored_and_not_cross_site
A15: cargo test -p deck-streak-api --test sync_seal_routes -- --exact the_release_is_off_without_the_seal_secret
A16: cargo test -p deck-streak-api --test sync_seal_routes -- --exact no_seal_secret_id_or_key_reaches_a_record
A17: cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::a_missing_seal_secret_turns_the_release_off
A18: cargo test -p deck-streak-identity --test sync_seal -- --exact a_seal_id_that_is_not_sixteen_bytes_does_not_parse
```

The model's properties are decided by the formal checker, which this repository's CI does not run;
section 8 names them and their witnesses.

## 4. File manifest

Part 1 of 2; the next pull request adds its own files (section 7).

| path | context | change |
|---|---|---|
| `formal/tla/SyncCredential/SyncCredential.tla` | formal | added |
| `formal/tla/SyncCredential/MCSyncCredential.cfg` | formal | added |
| `formal/tla/SyncCredential/witness/a-login-kept-after-a-sign-out.cfg` | formal | added |
| `formal/tla/SyncCredential/witness/a-send-from-memory-after-a-sign-out.cfg` | formal | added |
| `formal/tla/SyncCredential/witness/a-refusal-that-drops-a-newer-key.cfg` | formal | added |
| `formal/tla/SyncCredential/witness/a-network-failure-that-drops-the-key.cfg` | formal | added |
| `formal/tla/SyncCredential/witness/an-unseal-with-no-session.cfg` | formal | added |
| `config/formal.json` | config | the entry's budget |
| `scripts/tests/test_formal_config.py` | scripts | `EXPECTED` gains the entry |
| `crates/engine-core/src/credential.rs` | engine-core | added |
| `crates/engine-core/src/lib.rs` | engine-core | `pub mod credential` and its line in the module list |
| `crates/engine-core/tests/credential.rs` | engine-core | added |
| `crates/web-engine/src/wasm.rs` | web-engine | the credential exports |
| `crates/web-engine/tests/boundary.rs` | web-engine | one census row per export |
| `crates/identity/src/sync_seal.rs` | identity | added |
| `crates/identity/src/lib.rs` | identity | `pub mod sync_seal` |
| `crates/identity/tests/sync_seal.rs` | identity | added |
| `crates/identity/tests/boundary.rs` | identity | the finalize census admits the sealing key's one site |
| `crates/api/src/sync_seal_routes.rs` | api | added |
| `crates/api/src/router.rs` | api | mounts the release route |
| `crates/api/src/lib.rs` | api | the module and its line in the route list |
| `crates/api/tests/sync_seal_routes.rs` | api | added |
| `crates/kernel/tests/log_capture_class.rs` | kernel | the capture census counts A16's routed capture |
| `crates/daemon/src/wiring.rs` | daemon | reads the seal secret when the API's credentials hold it |
| `docs/specs/SPEC-363-the-web-client-keeps-the-sync-key-sealed.md` | docs | added |
| `docs/decisions/ADR-374-the-web-sync-key-is-sealed-under-a-key-the-service-releases-to-the-owners-session.md` | docs | added |
| `docs/schematics/web-sync-credential.md` | docs | added |
| `docs/red-first/SPEC-363.md` | docs | added |
| `scripts/mutation-rows.d/S36300-S36399.json` | scripts | added |
| `changelog.d/web-sync-seal-363.md` | docs | added |

## 5. What this does NOT do

- It builds no browser sync transport, moves no row of the core's table (`SyncLogin` stays
  `web: false`), and adds no login or sync call: those are SPEC-357's part b, which now consumes
  this SPEC's store (#631).
- It builds no sync screen: sign-in to sync, its status words and the sign-out control on the sync
  screen are SPEC-357's part c (#631).
- It does not move the iOS client's Keychain item: ADR-358 D2 stands, and the iOS client adopts
  the core's generation rule when its sync lands (#633).
- It builds no sign-in screen on the web: outside Telegram an owner session comes from the passkey
  sign-in's screens (#627).
- It gives the host key no lifetime of its own and mints no key per login: the fork's key stays as
  ADR-347 D13 left it, a named exception (#649).
- It changes no host: the seal secret's provisioning and the unit line that loads the
  `sync-seal-secret` role are the owner's steps, since a unit that names a missing credential
  fails to start; until then the route answers 404 `sync_seal_off` (#161).
- It relaxes neither of D13's triggers: a lost synced device still needs a new hash, at the
  cutover runbook's step (#628).
- It proves nothing on a device; the owner's acceptance session does (#637).

## 6. Risks

- **A script in the page during a live session.** Such a script can ask the service for the
  sealing key and read the sealed record, and so open the key, which then works until the owner
  rekeys (M5). Accepted: no browser mechanism keeps a same-origin store from a same-origin script,
  and the two designs that would (a second origin, a relay through the API) are refused by ADR-340
  and by ADR-374 D1 and D7. Narrowed by the page's script sources (M10), and visible in the edge's log
  of the sync route, which records each request's address (ADR-351 D4).
- **A Worker already open after a revocation.** A Worker that opened the key keeps it for its life;
  removing a passkey ends the session, and the next Worker cannot open the record, but the open one
  syncs until its tab closes. Detected by nothing on the client; bounded by the tab's life.
- **A 403 that is not the sync server's.** The rule reads any 403 on the sync route as a refusal
  and drops the key. The edge answers none of its own (M8); a later edge rule that does would
  cost the owner a re-entry of the password, never a lost review.
- **The model and the store drift.** The model covers the core's rule and the release; the
  Worker's store is TypeScript, which no cover can name. Part 2's tests replay each witness's
  interleaving against the store, so a store that drifts from the model fails them.
- **The rows on the web engine's boundary.** The census's owed statements must stay unique per
  export. Detected by the census itself.
- **Telegram's script on every route (M11), the accepted residual.** The page loads Telegram's
  script on every route, the web client outside the Mini App included, so it runs beside the
  page that asks for the release and holds the sync form. Accepted for this delivery (ADR-374,
  Consequences): it is one more same-origin script of the first risk above, and loading it only
  inside Telegram is its own change, outside this SPEC. It never reaches the Worker, the opened
  key or the card frame.

## 7. Delivered by the next pull request (part 2: the Worker's store, its operations, the page's sign-out, the reach census)

R9. The Worker keeps one record per origin in the browser's indexed database
    (`deck-streak-credential`, one object store, keys `generation` and `sealed`); each step reads,
    asks the core and writes in one read-write transaction; only the Worker's credential module
    opens it.
R10. The record is sealed with AES-GCM under the released key, imported non-extractable, with a
    fresh 96-bit nonce, and additional data binding the endpoint (the Worker's own origin and
    `/anki-sync/`, never typed), the user and the generation.
R11. Obtaining asks for the release first; with no owner session it answers `needs-sign-in` before
    the password leaves the page. The password crosses to the Worker for the login call alone and
    is held by nothing after it. The login's key is kept only when the core keeps it.
R12. The open key leaves the Worker's memory only into a request for the Worker's own origin's
    sync route, and an endpoint that is any other refuses before a byte is sent. Before each
    request the Worker re-reads the generation; when the core refuses the send, it sends nothing
    and clears its memory. The opened key lives for the Worker's life (ADR-374 D8).
R13. Each sync's answer is classified by the core; a refusal of the current generation deletes the
    record and raises the generation; every other answer keeps it. A record that does not open
    under its released key is deleted and the generation raised (ADR-374 D11).
R14. `forget` raises the generation and deletes the record in one transaction, clears the Worker's
    memory and tells every Worker of the origin on a broadcast channel. The page's sign-out
    forgets first and then ends the web session, so the key is gone when the device is offline.
R15. No Worker reply carries the host key, the password or the sealing key; the status operation
    answers one of `absent`, `sealed`, `held`, `needs-sign-in` and `offline`; no module of the card
    frame and no page component imports the credential module.
R16. Every study operation answers the same with the credential in each of those five states; a
    refused or unreachable release keeps the record.
R17. `PRIVACY.md` says what this browser keeps (the collection, its media and the sealed sync key),
    that signing out removes the key and clearing the site's data removes all of it, and that none
    of it is the service's store.

## 7a. Acceptance criteria (part 2)

| id | criterion | delivered by |
|---|---|---|
| B1 | a login that lands after a forget stores nothing, and the generation stays raised | `pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "a login that lands after a forget stores nothing"`; red: the stub stores every login |
| B2 | a Worker whose generation is stale sends nothing and clears its memory | `... credential.test.ts -t "a worker whose generation is stale sends nothing"`; red: the stub sends from memory |
| B3 | a refusal of an older generation keeps the newer record | `... credential.test.ts -t "a refusal of an older generation keeps the newer record"`; red: the stub deletes on every refusal |
| B4 | a network failure keeps the record | `... credential.test.ts -t "a network failure keeps the sealed record"`; red: the stub deletes on every failure |
| B5 | a refused or unreachable release keeps the record and reports `needs-sign-in` or `offline` | `... credential.test.ts -t "a refused or unreachable release keeps the record"`; red: the stub deletes on a refused release |
| B6 | the record opens only for its own endpoint, user and generation | `... credential.test.ts -t "the sealed record opens only for its own endpoint and user"`; red: the stub seals with no additional data |
| B7 | no Worker reply carries the host key, the password or the sealing key, beside a planted reply the scan must catch | `pnpm exec vitest run web/app/src/lib/engine/credential-reach.test.ts -t "no worker reply carries a secret"`; red: the stub's status answers the key |
| B8 | only the Worker's modules import the credential module, with the examined count and a planted import the census refuses by name | `... credential-reach.test.ts -t "only the worker imports the credential module"`; red: the census fixture's planted page import |
| B9 | sign-out forgets before it ends the session, and forgets offline | `pnpm exec vitest run web/app/src/lib/sync/sign-out.test.ts -t "sign-out forgets the sync key first, offline too"`; red: the stub ends the session only |
| B10 | every study operation answers the same in each credential state | `... credential.test.ts -t "every study op answers the same in each credential state"`; red: the stub refuses study while the release is refused |
| B11 | the credential operations answer a status word and never a value | `pnpm exec vitest run web/app/src/lib/engine/protocol.test.ts -t "the credential ops answer a status word"`; red: the protocol has no credential op |
| B12 | the key goes only to the Worker's own origin's sync route; a planted foreign endpoint is refused and nothing is sent | `... credential.test.ts -t "the key goes only to its own origin's sync route"`; red: the stub sends to the endpoint it is given |
| B13 | a record that does not open under its released key is deleted, and the status reads `absent` | `... credential.test.ts -t "a record that does not open is deleted"`; red: the stub keeps a record that fails to open |
| B14 | the API role composes the seal secret: absent, the release is off; present, an owner's session gets the release | `cargo nextest run -p deck-streak-daemon --test seal_release_composed`; red: the stub composition ignores the secret |

```acceptance
B1: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "a login that lands after a forget stores nothing"
B2: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "a worker whose generation is stale sends nothing"
B3: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "a refusal of an older generation keeps the newer record"
B4: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "a network failure keeps the sealed record"
B5: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "a refused or unreachable release keeps the record"
B6: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "the sealed record opens only for its own endpoint and user"
B7: pnpm exec vitest run web/app/src/lib/engine/credential-reach.test.ts -t "no worker reply carries a secret"
B8: pnpm exec vitest run web/app/src/lib/engine/credential-reach.test.ts -t "only the worker imports the credential module"
B9: pnpm exec vitest run web/app/src/lib/sync/sign-out.test.ts -t "sign-out forgets the sync key first, offline too"
B10: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "every study op answers the same in each credential state"
B11: pnpm exec vitest run web/app/src/lib/engine/protocol.test.ts -t "the credential ops answer a status word"
B12: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "the key goes only to its own origin's sync route"
B13: pnpm exec vitest run web/app/src/lib/engine/credential.test.ts -t "a record that does not open is deleted"
B14: cargo nextest run -p deck-streak-daemon --test seal_release_composed
```

Part 2's manifest: `web/app/src/lib/engine/credential.ts`, `web/app/src/lib/engine/credential.test.ts`,
`web/app/src/lib/engine/credential-reach.test.ts`, `web/app/src/lib/engine/protocol.ts`,
`web/app/src/lib/engine/protocol.test.ts`, `web/app/src/lib/engine/session.ts`,
`web/app/src/lib/engine/worker.ts`, `web/app/src/lib/engine/client.ts`,
`web/app/src/lib/sync/sign-out.ts`, `web/app/src/lib/sync/sign-out.test.ts`, `PRIVACY.md`,
`docs/red-first/SPEC-363.md` (its rows), and a `changelog.d/` fragment. Its TypeScript is held
by the mutation run over each changed file whole; an equivalent mutant is recorded, never
disabled. Part 2 also composes the seal-secret reader in the API role, so its manifest admits two
Rust paths, `crates/daemon/src/role_api.rs` and `crates/daemon/tests/seal_release_composed.rs`
(B14, held by mutation row `S36317`), beside one doc sentence of `crates/daemon/src/wiring.rs`.

## 8. Formal model

`formal/tla/SyncCredential` is written first, before the rule it covers. Its actors are the
owner (a login, a sign-out), two Workers (an open key, a send, a restart), the sync server (an
accepted key, a refusal after a rotation, a network failure) and the service (a session that opens
and ends, the release). It covers `crates/engine-core/src/credential.rs` at `fn on_obtained`,
`fn may_send`, `fn classify`, `fn on_outcome` and `fn on_removed`, and
`crates/api/src/sync_seal_routes.rs` at `fn release`, and cites #654, #631 and #618. Each
property enters at `ramp=report`:

| property | states | witness the check must catch |
|---|---|---|
| `NoKeyAtRestAfterSignOut` | after a sign-out, until the owner starts a new login, no sealed record is stored | `a-login-kept-after-a-sign-out` |
| `NoSendAfterSignOut` | no Worker starts a send after a sign-out, until a later login is kept | `a-send-from-memory-after-a-sign-out` |
| `ARefusalDropsOnlyItsOwnKey` | a refusal never removes a record of a later generation than the one it refused | `a-refusal-that-drops-a-newer-key` |
| `AFailureKeepsTheKey` | a network failure never removes the record | `a-network-failure-that-drops-the-key` |
| `UnsealOnlyInALiveSession` | a Worker opens the record only while an owner session is live | `an-unseal-with-no-session` |

## 9. Mutation rows (part 1)

Each row names a behaviour of R3 to R7, its mutant and the one test that kills it.

| row | file | mutant | killer |
|---|---|---|---|
| `S36301` | `crates/engine-core/src/credential.rs` | `on_obtained` keeps a login whatever generation it started at | `credential::a_login_is_kept_only_at_the_generation_it_started` |
| `S36302` | `credential.rs` | `may_send` ignores the held generation | `credential::a_send_needs_the_held_generation_to_be_current` |
| `S36303` | `credential.rs` | `may_send` ignores whether a record is stored | `credential::a_send_needs_the_held_generation_to_be_current` |
| `S36304` | `credential.rs` | `classify` reads every engine error as a refusal | `credential::only_the_sync_servers_refusal_is_a_refusal` |
| `S36305` | `credential.rs` | `on_outcome` drops on a refusal of any generation | `credential::a_refusal_drops_only_the_generation_it_refused` |
| `S36306` | `credential.rs` | `on_removed` returns the generation it was given | `credential::a_removal_raises_the_generation_and_it_never_wraps` |
| `S36307` | `credential.rs` | the generation wraps at its maximum instead of refusing | `credential::a_removal_raises_the_generation_and_it_never_wraps` |
| `S36308` | `crates/identity/src/sync_seal.rs` | the label is dropped from the HMAC's input | `sync_seal::the_seal_key_is_the_labelled_hmac_of_the_seal_id` |
| `S36309` | `sync_seal.rs` | the secret's length is not checked | `sync_seal::a_seal_secret_under_32_bytes_refuses_start` |
| `S36310` | `crates/api/src/sync_seal_routes.rs` | the handler drops its owner-session extractor | `sync_seal_routes::the_seal_key_is_released_only_to_an_owner_session` |
| `S36311` | `crates/identity/src/sync_seal.rs` | the seal id's decoded length is not checked | `sync_seal::a_seal_id_that_is_not_sixteen_bytes_does_not_parse` |
| `S36312` | `sync_seal_routes.rs` | the window admits a hundredfold more | `sync_seal_routes::the_release_has_a_bound_of_its_own` |
| `S36313` | `sync_seal_routes.rs` | the response's cache header is not `no-store` | `sync_seal_routes::the_release_is_not_stored_and_not_cross_site` |
| `S36314` | `sync_seal_routes.rs` | the handler drops its state-change guard | `sync_seal_routes::the_release_is_not_stored_and_not_cross_site` |
| `S36315` | `sync_seal_routes.rs` | the route releases when no secret is configured | `sync_seal_routes::the_release_is_off_without_the_seal_secret` |
| `S36316` | `crates/daemon/src/wiring.rs` | a missing seal secret refuses start instead of turning the release off | `lib::wiring::tests::a_missing_seal_secret_turns_the_release_off` |

## 10. What only a device or a person proves

These are read by the owner, not by CI.

| id | criterion | who, and when |
|---|---|---|
| V1 | On Safari on iPhone, as a Home Screen app, and on a desktop browser: a sync after signing in to sync; after sign-out the next sync asks for the sync password again | the owner, in the acceptance session, after SPEC-357's part c lands (#637) |
| V2 | A new tab opened after the web session ended studies at once and asks to sign in before it syncs; once signed in it syncs without the sync password | the owner, in the acceptance session (#637) |
| V3 | After the sync password changes on the server, the next sync drops the old key and asks for the new password, and study is never held | the owner, in the acceptance session (#637) |

## Amendment: SPEC-400 closes M11

M11 (line 49) measured that every route of the page loads Telegram's script, and lines 272-277
accept it as this SPEC's residual. SPEC-400 (ADR-414) closes it: the page loads the script only when
Telegram launched it, and outside a launch the page's policy refuses the script's origin, so the web
client in a plain browser tab runs no script from Telegram beside the page that asks for the release
and holds the sync form.
