---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-417: the start hook posts the launch data to a validation route, and loads Telegram's script only on its 204

Decides SPEC-403 (#775). It amends ADR-414 D2 for a launch read from the fragment: the start hook
no longer loads Telegram's script because a launch parameter is present, but because the server
accepted the launch data. ADR-414 D1, D3 (the narrowed policy) and D4 stand.

## Context and Problem Statement

Every fact was read at dev `32f62172`.

- The app's client `init` hook awaits the launch gate before the router's first navigation:
  `web/app/src/hooks.client.ts:14` (`await admitLaunch(window);`).
- The gate decides a launch from the fragment's keys alone. `web/app/src/lib/telegram-launch.ts:9`
  lists `tgWebAppData`, `tgWebAppVersion` and `tgWebAppPlatform`, and `isLaunch` (`:23-32`) reads
  their presence through `URLSearchParams`, or else the tab's mark (`:15`, `deck-streak:launched`).
- On a launch, the gate writes the mark (`:50`) and appends Telegram's script (`:54-61`). That
  append is the only place in the tree that adds the script: the shell holds none (SPEC-400 R7).
  Outside a launch it appends the narrowed policy (`:42-48`, the append at `:46`).
- The server validates launch data only in the session handshake. `POST /api/session`
  (`crates/api/src/session_routes.rs:128-155`) takes a state-change guard (`:182-209`), a
  handshake slot (`:213-231`, 30 a minute at `:43`) and a 16 KiB body bound (`:41`, layered at
  `:114`). It parses `{"init_data": ...}` (`:135`) and calls `OwnerGate::admit` (`:140`;
  `crates/identity/src/owner.rs:148-161`), which validates the signature in constant time and the
  freshness (`crates/identity/src/init_data.rs:88`, `:99`, `:103`), then pins the owner
  (`owner.rs:150`). Its refusals answer 401 `init_data_invalid` or `init_data_stale`, or 403
  `not_owner` (`crates/identity/src/lib.rs:136-155`), and the body holds the reason alone
  (`:158-164`).
- The client calls that route from the handshake (`web/app/src/lib/api.ts:113-128`), after the
  first navigation, and from the logout (`web/app/src/lib/sync/sign-out.ts:14`).
- SPEC-400 records the residual this closes (`:211-213`), and ADR-414 accepts it (`:219-221`): a
  link that carries a launch parameter loads Telegram's script in a plain tab, and the server's
  validation comes only at the session call, after the script has run.

#775 asks that the page have the server validate the launch first, load Telegram's script only
after the server accepts it, and load no script for a refused launch. The served header's
`frame-ancestors` stays out of scope, as in #706.

## Decision Drivers

- No code from Telegram's origin runs in a page whose launch the server has not accepted.
- The decision is still taken before the router's first navigation, while the fragment is whole
  (ADR-414 D2's reason holds unchanged).
- There is one implementation of launch validation, `OwnerGate::admit`.
- The launch data travels only in a request body: never in a URL, a header, storage or a log line
  (`web/app/src/lib/api.ts:38-39`).
- No line that another document cites moves.

## Decision Outcome

### D1. The population

Every place the page reads the launch parameters: `web/app/src/lib/telegram-launch.ts:9` and
`:23-32` (the gate), and Telegram's script itself once loaded. The wrapper
(`web/app/src/lib/telegram.svelte.ts:89-98`) reads only the script's object. The boundary test
(`web/app/src/lib/telegram-boundary.test.ts:80-90`) holds the gate as the one source file that
names a launch parameter.

Every path that loads the script: `web/app/src/lib/telegram-launch.ts:61`, reached only from
`web/app/src/hooks.client.ts:14`.

The server's validation: `POST /api/session` and the refusals listed in the Context.

The tests that pin them:
- `web/app/src/lib/telegram-launch.test.ts:55`, `:76`, `:104`;
- `web/app/src/lib/telegram-launch-hook.test.ts:36`;
- `web/app/src/lib/telegram-boundary.test.ts:80-104`;
- `web/app/tests/telegram-launch.spec.ts:34`, `:56`, `:95`, `:122`;
- the launches that `web/app/tests/smoke.spec.ts:32`, `web/app/tests/a11y.spec.ts:75-79` and
  `web/app/tests/streak-calendar.spec.ts:66`, `:88` open;
- the route's tests in `crates/api/tests/session_routes.rs` (`:256`, `:357`, `:515`).

The rows: none anchors `session_routes.rs`, `router.rs`, `telegram-launch.ts` or
`hooks.client.ts`. S02401 (`crates/identity/src/init_data.rs`) and S02404 (`owner.rs:150`) anchor
the gate this change reuses, and this change does not touch their lines.

The threat model's E6 row cites `hooks.client.ts:14` and `telegram-launch.ts:46`, and
`scripts/tests/test_threat_model.py:231` lists the web surface's row ids. No model or proof under
`formal/` cites any of these files.

**Chosen against** taking SPEC-400's population alone, and against a census by the script's URL.

### D2. Validate first

The gate reads the launch data as the fragment's `tgWebAppData` value, decoded once by
`URLSearchParams`. That is the decoding Telegram's script applies, so the page sends the same
string the wrapper later reads as the script's launch data. An absent or empty value is no launch
data. `tgWebAppVersion` or `tgWebAppPlatform` without it is no launch data either.

With launch data, the gate sends `POST /api/launch` with the body `{"init_data": <the launch
data>}`, `credentials: 'same-origin'`, `content-type: application/json`, and an abort signal. It
sends through an injected `send`, whose default is `(input, init) => globalThis.fetch(input,
init)`, as `web/app/src/lib/api.ts:107-109` does. The hook's call stays
`await admitLaunch(window);`. The gate does not import `api.ts`, because `api.ts` imports the
wrapper, which would read the script's object before the script exists. The gate also never names
the launch data's field in the wrapper's spelling: the boundary test reserves that name to the
wrapper.

The gate races the answer against a deadline of 10 000 ms, the server's own request timeout
(`crates/api/src/router.rs:78`). It uses one `Promise.race` between the answer and a `setTimeout`
that aborts the request, so exactly one outcome settles and a late answer changes nothing:

| the answer | outcome | what the page does |
|---|---|---|
| 204 | accepted | marks the tab `deck-streak:launch-accepted`, appends the script once with `referrerpolicy` `same-origin`, and settles when it loads or fails |
| 401 or 403 | refused | appends the narrowed policy, removes the tab's accepted mark, appends no script |
| any other status (a 200 document included), a failed request, or no answer by the deadline | unanswered | the same as refused |

With no launch data in the fragment, the tab's accepted mark loads the script with no request.
That is a reload after the router's redirect dropped the fragment. The mark is written only after
a 204, holds no launch data, and only the page's own code can write it. With neither, the gate
appends the narrowed policy and sends nothing. The mark the earlier gate wrote on presence alone
(`deck-streak:launched`) is never read.

The red-first tests, and the check runs the seat reads by name:
- a crafted launch the server refuses loads no script, on the launch and on its reload
  (Playwright, CI `web`, red at the first push);
- an accepted launch loads the script exactly once, after the validation request (Playwright, CI
  `web`, red at the first push);
- an unanswered validation (a failed request, 429, 503, or a 200 document) loads none
  (Playwright, CI `web`, red at the first push);
- the decoding, the request's shape, each outcome, the deadline, a late answer and the mark
  (Vitest, CI `web`, red locally against a stub);
- the route's answers (cargo, CI `rust`, red locally while the route is absent).

Vitest's `vi.stubGlobal` reaches the page's `fetch`, and fake timers drive the deadline through
`vi.advanceTimersByTimeAsync`. Playwright's `page.route` answers the validation with
`route.fulfill({ status })` or fails it with `route.abort()`, because the suites' web server is
`vite build && vite preview` and serves no API.

**Chosen against** keeping the browser-only decision, loading the script first and validating
after, validating only at the first session call, reusing `POST /api/session`, re-validating a
reload through `GET /api/me`, keeping the launch data to send it again, reading Telegram's own
storage key, `AbortSignal.timeout`, a shorter deadline, reusing the earlier mark's key, and
treating `tgWebAppVersion` or `tgWebAppPlatform` alone as a launch.

### D3. The server side

There is a new route, `POST /api/launch`. It is defined after the last line of
`crates/api/src/session_routes.rs`, so no line of that file moves. `LAUNCH_PATH` and
`pub(crate) fn launch_routes(access: OwnerAccess) -> Router` mirror `routes()`, with the handler
behind the same 16 KiB body bound (`DefaultBodyLimit::max(HANDSHAKE_BODY_LIMIT_BYTES)`, the
`Handler::layer` that `:114` uses).

The handler is `validate_launch(_state_change: StateChange, _slot: HandshakeSlot,
State(access): State<OwnerAccess>, body: Bytes) -> Response`. It parses the same `Handshake` body,
answering `Refusal::InitDataInvalid` when the body is not one. It calls
`access.gate.admit(&init_data, access.clock.now())`, answers `StatusCode::NO_CONTENT` on `Ok`, and
answers `refusal.into_response()` on `Err`.

It opens and ends no session, sets no cookie, reads no `HeaderMap`, and logs by reason alone, as
the gate already does (`owner.rs:157-158`). It takes a slot from the handshake's own bound: every
clone of `OwnerAccess` shares one `Arc<HandshakeBound>`, so launches and handshakes count against
the same 30 a minute.

`crates/api/src/router.rs` merges it directly after the session routes' merge (`:258`), so it is
served exactly when the session routes are. Nothing above that line moves, and nothing cites a
line below it.

No migration is needed, and no secret is used. The tests reuse the synthetic signing token and
payloads of `crates/api/tests/session_routes.rs:29-52`, and an answer carries a reason code alone.

**Chosen against** reusing `POST /api/session`, a second validator, a route in a module of its
own, a bound of its own, and adding the route inside `routes()`.

### D4. FORMAL, decided by surface

FORMAL: NOT APPLICABLE, for TLA+ and for Lean. The census, by actor:

- **The client.** The start hook, a timer and the answer race on one single-threaded event loop.
  Their only shared state is the outcome of one `Promise.race`, which settles once, so there is no
  check-then-act a second actor can interleave. The surface is TypeScript, and a formal cover can
  name only a `.rs`, `.py` or `.sh` file. The race is enumerated by unit tests instead: the answer
  first, the deadline first, and a late answer.
- **The server.** The new route is a new caller of `OwnerGate::admit` and `HandshakeBound::admit`.
  The gate is a pure function of the launch data, the signing key and the clock. The bound's
  window is read and written inside one lock (`crates/api/src/session_routes.rs:243-257`), with no
  check-then-act across it. The route reads and writes no session.
- **The step after the check.** The script load grants nothing: the session handshake validates
  the launch data again (`session_routes.rs:140`), so launch data that grows stale between the two
  calls is refused there.
- **The models.** No model or proof under `formal/` cites `session_routes.rs`, `router.rs`,
  `owner.rs`, `init_data.rs`, `telegram-launch.ts` or `hooks.client.ts`.

**Chosen against** a TLA+ model of the client's race, and a model of the shared bound.

### D5. Drift and the push count

Open work shares these paths:
- the threat model's schematic, with three open pull requests (#783, #778, #769);
- `web/app/src/lib/api.ts`, with #782, the web client's sign-in. This change leaves that file
  untouched.
- `scripts/mutation-equivalent.d/miniapp.json`, with #782, and only if a StrykerJS survivor is
  proved equivalent.

#782 adds routes to `web/app/src/lib/routes.ts`. Both SPEC-400's walk of the routes table and
this change's walk read the table at run time, so a new route is covered without an edit. The
builder re-measures every shared path at its cut.

Which threat-model citations move:
- E6's `telegram-launch.ts:46` moves when the gate is rewritten, and is re-cited at the narrowed
  policy's new append line.
- E6's `hooks.client.ts:14` holds, because the hook is unchanged.
- A new row, S6 ("a link from outside Telegram poses as a launch to load Telegram's script"),
  cites the gate's request and acceptance lines and the route's gate call. The refused-launch
  test and the route's refusal test pin it.
- `scripts/tests/test_threat_model.py:231` gains `"S6"` after `"E6"`.

TWO pushes. The browser reds can be read only in CI's `web` run, so push 1 holds the docs and the
Playwright tests alone, and its red is read there. Push 2 holds the Vitest and Rust tests, whose
red is read locally against a type-clean stub, then the implementation, the record, the rows and
the fragment.

**Chosen against** one push and three pushes.

## The alternatives each decision was chosen against

### D1. The population

- **SPEC-400's population alone**: rejected because it holds no server side, and the server's
  answer is now the decision.
- **A census by the script's URL alone**: rejected because it finds the one append and misses the
  readers of the launch parameters and the tab's mark that decide whether the append runs.

### D2. Validate first

- **Keep the browser-only decision**: rejected because it is the residual SPEC-400 records
  (`:211-213`): a crafted link still loads the script, since presence is all the gate reads.
- **Load the script first and validate after**: rejected because Telegram's code has already run
  in the page by the time the answer arrives, and an answer cannot unload it.
- **Validate only at the first session call**: rejected because that call is made after the first
  navigation (`web/app/src/lib/api.ts:113-128`), after the script has loaded and run.
- **Reuse `POST /api/session` as the validation**: rejected because it opens a session and sets
  its cookie before any script has run, and doubles the session churn of every launch.
- **Re-validate a reload through `GET /api/me`**: rejected because sessions live in the server's
  memory and end at a restart, so a reload from Telegram's menu after a restart would load no
  script and answer "reopen".
- **Keep the launch data in the tab and send it again on a reload**: rejected because the launch
  data never goes in storage (`web/app/src/lib/api.ts:38-39`).
- **Read Telegram's own storage key on a reload**: rejected because ADR-414 already rejected it
  (`:158`): the key is the script's private format and can change without notice.
- **`AbortSignal.timeout` for the deadline**: rejected because a fake clock is not guaranteed to
  drive it, and `setTimeout` with an `AbortController` is what the unit tests' fake timers do drive.
- **A deadline shorter than the server's 10 s timeout**: rejected because a slow but valid launch
  would then load no script and answer "reopen", while the server was still able to accept it.
- **Reuse the earlier mark's key**: rejected because the earlier gate wrote that key on presence
  alone, so a tab that carried it across the deploy would count a crafted link as accepted.
- **Treat `tgWebAppVersion` or `tgWebAppPlatform` alone as a launch**: rejected because it carries
  no launch data, so the server has nothing to accept, and no session can open from it
  (`web/app/src/lib/api.ts:115`).

### D3. The server side

- **Reuse `POST /api/session`**: rejected because it opens a session and sets a cookie before the
  script; the launch's question needs no session.
- **A second validator for the launch**: rejected because two implementations of one check can
  drift, and `OwnerGate::admit` already holds the signature, the freshness and the owner pin.
- **A route in a module of its own**: rejected because `HandshakeSlot` and `StateChange` are
  private to `session_routes.rs`; reaching them would widen their visibility for no gain.
- **A bound of its own**: rejected because it adds shared state, and the owner's launches are
  well inside 30 a minute when each one costs a launch and a handshake.
- **Add the route inside `routes()`**: rejected because it moves the handshake's lines
  (`:128-155`), which `docs/schematics/passkey-sign-in-on-the-web.md:96` cites.

### D4. FORMAL, decided by surface

- **A TLA+ model of the client's answer-or-deadline race**: rejected because the surface is
  TypeScript, which no cover can name, and the race is one `Promise.race` that settles once; unit
  tests enumerate its three orders. This is recorded as a gap of the formal packs: a web surface
  cannot be covered.
- **A model of the shared handshake bound**: rejected because its window is read and written in
  one critical section (`session_routes.rs:243-257`), with no check-then-act across the lock.

### D5. Drift and the push count

- **One push**: rejected because the browser criteria's red can be read only in CI, and a push
  that carries the fix beside them shows them green.
- **Three pushes (the unit and Rust tests alone, then the fix)**: rejected because those reds are
  read locally against a type-clean stub, so a CI run of them adds a run and no evidence.

## Consequences

- Good: a link from outside Telegram that carries a launch parameter loads no Telegram code. The
  server's refusal decides it, and the narrowed policy then refuses Telegram's origin for the
  page's life.
- Good: the validation reuses the shipped gate, its bound and its refusals, so the launch and the
  session cannot disagree on what a valid launch is.
- Bad: a launch now makes one more request before the first screen. A slow server delays that
  screen by up to the deadline, after which the launch is unanswered.
- Bad: a launch while the device is offline loads no script, even when the browser holds it in its
  cache, so Telegram's theme and buttons are absent until an online launch.
- Bad: each launch takes two of the 30 handshake slots a minute.
- Neutral: the session handshake still validates the launch data, so an accepted launch whose data
  grows stale before the first call answers "reopen", as before.

## What would make this wrong

- Telegram's script decoding `tgWebAppData` differently from `URLSearchParams` for a fragment
  Telegram sends. The launch would then be validated on a string the session never sees. The unit
  test's decoding cases and the person-proved launch would show it.
- A launch kind that carries launch parameters but no `tgWebAppData` turning out to need
  Telegram's script. No session can open from it at dev either (`web/app/src/lib/api.ts:115`).
- Any other code path that adds a script from Telegram's origin. SPEC-400's A1 (no route requests
  the script outside a launch) and A11 (the shell holds no script) hold the gate as the only one.
