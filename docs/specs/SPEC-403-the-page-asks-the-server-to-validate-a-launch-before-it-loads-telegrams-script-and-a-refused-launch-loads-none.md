# SPEC-403: the page asks the server to validate a launch before it loads Telegram's script, and a refused launch loads none

- **Issue:** #775. It closes the residual that SPEC-400 records (`:211-213`) and ADR-414 accepts
  in its Consequences (`:219-221`).
- **Context(s):** `miniapp` (`web/app/src`, `web/app/tests`), `api` (`crates/api`), and the
  documents that describe the launch (`docs/`).
- **Decided by:** ADR-417: D1 the population, D2 validate first, D3 the server side, D4 FORMAL,
  and D5 drift and the push count. It works under ADR-414, whose D1, D3 and D4 stand and whose D2
  it amends for a launch read from the fragment. It also works under SPEC-024 (the server
  validates the launch data) and SPEC-400 (the launch gate).
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-403.md`.

## 1. The problem, measured

Every fact was read at dev `32f62172`. `git show 32f62172:<path>` with `sed -n` over the named
lines reproduces each one.

### 1.1 How the page decides a launch today

- The app's client `init` hook awaits the gate before the router's first navigation:
  `web/app/src/hooks.client.ts:14`, `await admitLaunch(window);`.
- The gate reads a launch from the presence of `tgWebAppData`, `tgWebAppVersion` or
  `tgWebAppPlatform` in the fragment (`web/app/src/lib/telegram-launch.ts:9`, `:23-32`), or from
  the tab's mark `deck-streak:launched` (`:15`). It writes that mark on presence alone (`:50`).
- On a launch the gate appends Telegram's script (`:54-61`, the append at `:61`). Outside a launch
  it appends a policy that narrows `script-src` to the page's own origin (`:20`, `:42-48`, the
  append at `:46`).
- The append at `:61` is the only place that adds the script: the shell carries none (SPEC-400
  A11).
- No value of the fragment is read, so whether to load the script rests on the browser alone.

### 1.2 Where the server validates launch data today

- `POST /api/session` (`crates/api/src/session_routes.rs:128-155`) takes three guards:
  - the state-change guard (`:182-209`): 403 `cross_site_request` or `not_json`;
  - a handshake slot (`:213-231`, 30 a minute at `:43`): 429 `too_many_handshakes`;
  - a 16 KiB body bound (`:41`, layered on the handler at `:114`).
- It parses `{"init_data": ...}` (`:135`), and calls `OwnerGate::admit` (`:140`;
  `crates/identity/src/owner.rs:148-161`).
- The gate validates the signature in constant time and checks the freshness
  (`crates/identity/src/init_data.rs:88`, `:99`, `:103`). It then pins the owner (`owner.rs:150`).
- A refusal answers 401 `init_data_invalid` or `init_data_stale`, or 403 `not_owner`
  (`crates/identity/src/lib.rs:136-155`), with a body that holds the reason alone (`:158-164`).
- The client reaches that route only from the handshake (`web/app/src/lib/api.ts:113-128`), after
  the first navigation and so after the script has loaded, and from the logout
  (`web/app/src/lib/sync/sign-out.ts:14`).
- The session routes are merged at `crates/api/src/router.rs:258`, and the server's request
  timeout is 10 s (`router.rs:78`).

### 1.3 What pins it today

- Unit tests:
  - `web/app/src/lib/telegram-launch.test.ts:55` (presence and the mark), `:76` (the script's
    append) and `:104` (the narrowed policy);
  - `web/app/src/lib/telegram-launch-hook.test.ts:36` (the hook);
  - `web/app/src/lib/telegram-boundary.test.ts:80-104`: only the gate names a launch parameter,
    and neither the hook nor the gate imports the wrapper.
- Browser tests: `web/app/tests/telegram-launch.spec.ts:34`, `:56`, `:95`, `:122`. The launches
  that `web/app/tests/smoke.spec.ts:32`, `web/app/tests/a11y.spec.ts:75-79` and
  `web/app/tests/streak-calendar.spec.ts:88` open all load the script at dev.
- Route tests: `crates/api/tests/session_routes.rs`, for the guard (`:256`), the bound (`:357`)
  and the body bound (`:515`).
- Mutation rows: none is anchored in `session_routes.rs`, `router.rs`, `telegram-launch.ts` or
  `hooks.client.ts`. No model or proof under `formal/` cites them.
- The threat model's E6 row cites `hooks.client.ts:14` and `telegram-launch.ts:46`, and
  `scripts/tests/test_threat_model.py:231` lists the web surface's row ids.
- CI runs:
  - the unit and browser tests in the `web` check run (`.github/workflows/ci.yml:201`, its run at
    `:253`);
  - the route tests in `rust` (`:30`);
  - the threat-model test in `hygiene` (`:298`);
  - the mutation runs in `mutation-web` (`:704`), `mutation-rust` (`:447`) and `mutation-rows`
    (`:575`).

## 2. Requirements

R1. The gate reads the launch data as the fragment's `tgWebAppData` value, decoded once by
`URLSearchParams`. An absent or empty value is no launch data, and so is `tgWebAppVersion` or
`tgWebAppPlatform` without it.

R2. With launch data, the gate sends `POST /api/launch` with the body `{"init_data": <the launch
data>}`, `credentials: 'same-origin'`, `content-type: application/json` and an abort signal. It
sends before it appends any script or policy, and the router's first navigation waits for the
outcome.

R3. Only a 204 accepts the launch. The gate then marks the tab `deck-streak:launch-accepted`, a
mark that holds no launch data. It appends Telegram's script once, with `referrerpolicy`
`same-origin`, and settles when the script loads or fails.

R4. A 401 or a 403 refuses the launch. Any other status, a failed request, or no answer within
10 000 ms leaves it unanswered. Either way, the gate appends no script, appends the narrowed
policy, and removes the tab's accepted mark.

R5. The outcome settles once. The deadline aborts the request, and an answer after the deadline
changes nothing.

R6. With no launch data in the fragment, the tab's accepted mark loads the script with no request.
The earlier mark `deck-streak:launched` is not read. With neither, the gate appends the narrowed
policy and sends nothing.

R7. The gate imports neither the wrapper nor `web/app/src/lib/api.ts`, and the launch data appears
only in the request's body.

R8. `POST /api/launch` takes three guards: the state-change guard, a slot from the handshake's own
bound, and the handshake's 16 KiB body bound. It validates through `OwnerGate::admit`. On
acceptance it answers 204 with no body and no cookie. It opens and ends no session. It refuses
with the gate's status and reason code alone.

R9. The route is served exactly when the session routes are. No line of
`crates/api/src/session_routes.rs` moves, and no line of `crates/api/src/router.rs` above the
session routes' merge moves.

R10. The threat model's E6 row is re-cited where its line moved. A new S6 row cites the gate's
request and acceptance lines and the route's gate call. The threat-model test lists S6.

R11. SPEC-400's launch tests keep their names and every case they plant. A case whose answer
ADR-417 D2 changes keeps its input and asserts the new answer. Each launch they open is answered
204.

## 3. Acceptance criteria of this delivery

| id | criterion | decided by |
|---|---|---|
| A1 | a crafted launch the server refuses with 401 or 403 loads no Telegram script, on the launch and on its reload; each load sent one validation carrying the decoded launch data, added the narrowed policy and made its own requests | `web/app/tests/launch-validation.spec.ts` (Playwright, CI `web`) |
| A2 | an accepted launch loads Telegram's script exactly once, after the validation request, whose body carries the launch data byte for byte | `web/app/tests/launch-validation.spec.ts` (Playwright, CI `web`) |
| A3 | a launch whose validation fails, or is answered 429, 503 or 200, loads no Telegram script and adds the narrowed policy | `web/app/tests/launch-validation.spec.ts` (Playwright, CI `web`) |
| A4 | a reload after an accepted launch loads Telegram's script again and sends no second validation | `web/app/tests/launch-validation.spec.ts` (Playwright, CI `web`) |
| A5 | outside a launch no route of the routes table sends a validation or requests Telegram's script, while each route makes its own requests | `web/app/tests/launch-validation.spec.ts` (Playwright, CI `web`) |
| A6 | the launch data is the fragment's `tgWebAppData` value decoded once: `+` reads as a space and `%2B` as `+`; an absent or empty value, or another launch parameter alone, is none | `web/app/src/lib/telegram-launch.test.ts` (Vitest, CI `web`) |
| A7 | the gate sends `POST /api/launch` with the JSON body, same-origin credentials and an abort signal, and adds no script and no policy while the answer is pending | `web/app/src/lib/telegram-launch.test.ts` |
| A8 | only a 204 accepts: then the script is added once with no referrer, the tab is marked, and the start settles when the script loads or fails; a 200 adds no script | `web/app/src/lib/telegram-launch.test.ts` |
| A9 | a 401 or a 403 adds no script, adds the narrowed policy and removes the tab's accepted mark | `web/app/src/lib/telegram-launch.test.ts` |
| A10 | a 429 or 500 answer, a failed request, and no answer by 10 000 ms each add no script and add the narrowed policy; at 9 999 ms the start still waits, and at the deadline the request's signal is aborted | `web/app/src/lib/telegram-launch.test.ts` |
| A11 | a 204 that arrives after the deadline adds no script and no second policy | `web/app/src/lib/telegram-launch.test.ts` |
| A12 | with no launch data, the tab's accepted mark loads the script with no request, and the earlier mark alone is not an acceptance | `web/app/src/lib/telegram-launch.test.ts` |
| A13 | the app's start sends the launch through the page's own fetch, and adds the script only after the 204, before the first navigation | `web/app/src/lib/telegram-launch-hook.test.ts` |
| A14 | the owner's launch data at `POST /api/launch` answers 204 with no body and no `Set-Cookie`, and opens no session | `crates/api/tests/session_routes.rs` (cargo, CI `rust`) |
| A15 | a forged, a stranger's, a stale and a malformed launch answer 401 `init_data_invalid`, 403 `not_owner`, 401 `init_data_stale` and 401 `init_data_invalid`, each body exactly the reason | `crates/api/tests/session_routes.rs` |
| A16 | a cross-site or non-JSON launch is refused 403 | `crates/api/tests/session_routes.rs` |
| A17 | a launch body one byte past 16 KiB is refused 413 | `crates/api/tests/session_routes.rs` |
| A18 | launches and handshakes share one bound: after 30 launches in a minute, a handshake is refused 429 | `crates/api/tests/session_routes.rs` |
| A19 | SPEC-400's launch tests keep their names and planted cases and pass with the launch accepted | the files named in the fence |
| A20 | every threat-model control cites a line that holds, with E6 re-cited and S6 added | `scripts/tests/test_threat_model.py` (CI `hygiene`) |

```acceptance
A1: pnpm --dir web/app exec playwright test tests/launch-validation.spec.ts -g "a crafted launch the server refuses loads no Telegram script, and neither does its reload"
A2: pnpm --dir web/app exec playwright test tests/launch-validation.spec.ts -g "an accepted launch loads Telegram's script once, after the server accepted it"
A3: pnpm --dir web/app exec playwright test tests/launch-validation.spec.ts -g "a launch whose validation goes unanswered loads no Telegram script"
A4: pnpm --dir web/app exec playwright test tests/launch-validation.spec.ts -g "a reload after an accepted launch loads Telegram's script again with no second validation"
A5: pnpm --dir web/app exec playwright test tests/launch-validation.spec.ts -g "outside a launch no route sends the launch for validation"
A6: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "the launch data is the fragment's tgWebAppData value, decoded once"
A7: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "a launch is sent for validation before any script is added"
A8: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "only a 204 accepts the launch, and then the script is added once"
A9: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "a refused launch adds no script, narrows the policy and clears the tab's mark"
A10: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "an unanswered launch adds no script: an error answer, a failed request or the deadline"
A11: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "an acceptance after the deadline adds no script"
A12: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "a reload with the accepted mark loads the script with no request, and the earlier mark is not an acceptance"
A13: pnpm exec vitest run web/app/src/lib/telegram-launch-hook.test.ts -t "the app's start validates the launch through the page's fetch before it adds the script"
A14: cargo test -p deck-streak-api --test session_routes -- --exact a_launch_the_gate_admits_is_accepted_with_204_and_opens_no_session
A15: cargo test -p deck-streak-api --test session_routes -- --exact a_forged_foreign_stale_or_malformed_launch_is_refused_with_its_reason_alone
A16: cargo test -p deck-streak-api --test session_routes -- --exact a_cross_site_or_non_json_launch_is_refused
A17: cargo test -p deck-streak-api --test session_routes -- --exact the_launch_body_is_bounded_like_the_handshake
A18: cargo test -p deck-streak-api --test session_routes -- --exact launches_and_handshakes_share_the_handshake_bound
A19: pnpm --dir web/app exec playwright test tests/telegram-launch.spec.ts
A19: pnpm --dir web/app exec playwright test tests/smoke.spec.ts
A19: pnpm --dir web/app exec playwright test tests/a11y.spec.ts
A19: pnpm --dir web/app exec playwright test tests/streak-calendar.spec.ts
A19: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "a launch is read from the fragment's launch parameters and from the tab's mark"
A19: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "Telegram's script is added to the head with no referrer and awaited until it loads or fails"
A19: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "outside a launch the page adds a policy that admits its own scripts and not Telegram's"
A19: pnpm exec vitest run web/app/src/lib/telegram-launch-hook.test.ts -t "the app's start admits the launch before its first navigation"
A19: pnpm exec vitest run web/app/src/lib/telegram-boundary.test.ts
A20: python3 -m unittest discover -s scripts/tests -p test_threat_model.py -k test_every_control_cites_a_line_that_holds
```

A1, A2 and A3 are read red in CI's `web` run at the first push, which carries no fix. A4 and A5
pin behaviour the base already had: the mark's reload, and a plain page that sends nothing. They
guard the gate's rewrite. A6 to A18 are read red locally, against a type-clean stub and against a
tree with no `/api/launch` route. A19 and A20 guard the restatement and the re-citation.

## 4. File manifest

| path | context | change |
|---|---|---|
| `docs/specs/SPEC-403-the-page-asks-the-server-to-validate-a-launch-before-it-loads-telegrams-script-and-a-refused-launch-loads-none.md` | docs | added: this SPEC |
| `docs/decisions/ADR-417-the-start-hook-posts-the-launch-data-to-a-validation-route-and-loads-telegrams-script-only-on-its-204.md` | docs | added: its ADR |
| `docs/specs/SPEC-400-the-page-loads-telegrams-script-only-when-telegram-launched-it-and-refuses-its-origin-otherwise.md` | docs | insert-only: a new last section, `## 10. Amendments by SPEC-403: the launch is validated first`, naming the lines this SPEC supersedes (R2 at `:89-90`, A5's row at `:132`, the residual at `:211-213`) |
| `docs/decisions/ADR-414-telegrams-script-loads-from-the-apps-start-hook-only-on-a-launch-and-the-page-narrows-its-policy-otherwise.md` | docs | insert-only: a new last section, `## Amended by ADR-417`, naming D2's detection (`:78-79`) and the crafted-link consequence (`:219-221`) |
| `docs/schematics/mini-app-launch.md` | docs | insert-only: a note directly after line 21, and a new last section that draws the validated launch |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | docs (threat model) | E6's `telegram-launch.ts` citation re-cited at its new line; a new S6 row after E6 in section 5 |
| `docs/red-first/SPEC-403.md` | docs | added: the red-first record |
| `changelog.d/launch-validate-first-403.md` | docs | added: the delivery's fragment |
| `web/app/src/lib/telegram-launch.ts` | miniapp | the gate validates the launch first (R1 to R7) |
| `web/app/src/lib/telegram-launch.test.ts` | miniapp (test) | A6 to A12; SPEC-400's three tests restated (R11) |
| `web/app/src/lib/telegram-launch-hook.test.ts` | miniapp (test) | A13; SPEC-400's hook test restated (R11) |
| `web/app/tests/launch-validation.spec.ts` | miniapp (test) | added: A1 to A5 |
| `web/app/tests/launch-fragment.ts` | miniapp (test) | a helper that answers the validation and keeps each body it was sent |
| `web/app/tests/telegram-launch.spec.ts` | miniapp (test) | its launches answered 204; its line 56 does not move |
| `web/app/tests/smoke.spec.ts` | miniapp (test) | its launch answered 204 |
| `web/app/tests/a11y.spec.ts` | miniapp (test) | its launch answered 204; still one navigation |
| `web/app/tests/streak-calendar.spec.ts` | miniapp (test) | its launch answered 204 |
| `crates/api/src/session_routes.rs` | api | `POST /api/launch`, after the file's last line (R8, R9) |
| `crates/api/src/router.rs` | api | the launch routes merged directly after line 258 (R9) |
| `crates/api/tests/session_routes.rs` | api (test) | A14 to A18 |
| `scripts/tests/test_threat_model.py` | hygiene (test) | the web surface's id list gains `"S6"` after `"E6"` |
| `scripts/mutation-rows.d/S40300-S40399.json` | mutation | added: five rows (section 8) |

## 5. What this does NOT cover

- The served header's `frame-ancestors`, which names who may frame the page, is unchanged. #775
  keeps it out of scope, as #706 did.
- The session handshake, `POST /api/session`, and its validation are unchanged. The launch route
  reuses them (#775).
- Sign-in on the web outside Telegram is #782's. This change leaves `web/app/src/lib/api.ts`
  untouched (#782).
- A launch that carries launch parameters but no `tgWebAppData` loads no script. No session can
  open from it at dev either, since the handshake sends nothing without launch data (#775).
- No integrity attribute is added to Telegram's script, whose URL names no version (#775).
- The native clients' launches are not the web client's, and are untouched (#664).
- A third browser engine in the test matrix is #652's. A1 to A5 are measured in the suite's
  default browser (#652).

## 6. Risks

- **A slow or unreachable server delays a launch's first screen** by up to 10 s. The launch is
  then unanswered, loads no script, and the app asks to reopen it. Detected by A10's deadline and
  by section 7's launch.
- **A launch while the device is offline loads no script**, even when the browser holds it in its
  cache, so Telegram's theme and buttons are absent until an online launch. Detected by section 7.
- **Telegram's script decoding the fragment differently from `URLSearchParams`** would validate a
  string the session never sees. Two checks detect it: A6's cases, which match the decoding of the
  browser suites' stand-in, and section 7's launch, whose session opens only when the two agree.
- **Each launch takes two of the handshake bound's 30 slots a minute.** A18 pins that the bound is
  shared.
- **A cited line moves.** E6's gate line moves and is re-cited. A20 refuses any citation that no
  longer holds, and `crates/api/src/session_routes.rs` changes only below its last line.

## 7. What only a person proves

After the deploy, the owner opens the Mini App from Telegram on each client they use, and checks
four things:

- the theme applies;
- the first screen renders;
- the day's study shows, so a session opened;
- after a reload from Telegram's menu, the screen still renders.

The owner then opens a route in a plain browser tab, through a link that carries a made-up
`tgWebAppData` value. The browser's network panel lists one `POST /api/launch` answered 401, and
no request to Telegram's origin.

The record is a comment on #775 that names the clients.

## 8. Mutation testing

- StrykerJS mutates `web/app/src/lib/telegram-launch.ts` whole with `thresholds.break: 100`, and
  CI's `mutation-web` check run is the verdict. A survivor gets a test that observes its
  behaviour. Only a survivor proved equivalent is recorded, with its reason, in
  `scripts/mutation-equivalent.d/miniapp.json`. `web/app/src/hooks.client.ts` is unchanged.
- cargo-mutants mutates the diff (`mutation-rust`): `validate_launch`, `launch_routes` and the
  router's new merge.
- The hand rows below cover what cargo-mutants does not list. Each sits in the `MUTATIONS` table of
  `scripts/mutation-rows.d/S40300-S40399.json`, in crate `api`, and is killed by
  `crates/api/tests/session_routes.rs`:

| row | file | the mutant | killer |
|---|---|---|---|
| S40300-THE-LAUNCH-TAKES-A-HANDSHAKE-SLOT | `src/session_routes.rs` | `validate_launch` loses `_slot: HandshakeSlot,` | `launches_and_handshakes_share_the_handshake_bound` |
| S40301-THE-LAUNCH-IS-A-STATE-CHANGE | `src/session_routes.rs` | `validate_launch` loses `_state_change: StateChange,` | `a_cross_site_or_non_json_launch_is_refused` |
| S40302-THE-LAUNCH-BODY-IS-BOUNDED | `src/session_routes.rs` | the handler loses its `DefaultBodyLimit` layer | `the_launch_body_is_bounded_like_the_handshake` |
| S40303-THE-LAUNCH-PATH-IS-API-LAUNCH | `src/session_routes.rs` | `LAUNCH_PATH` names another path | `a_launch_the_gate_admits_is_accepted_with_204_and_opens_no_session` |
| S40304-THE-ROUTER-SERVES-THE-LAUNCH | `src/router.rs` | the launch routes' merge is deleted | `a_launch_the_gate_admits_is_accepted_with_204_and_opens_no_session` |

The rows already anchored in the gate this route reuses, S02401 (`crates/identity/src/init_data.rs`)
and S02404 (`crates/identity/src/owner.rs`), do not move.

## 9. Formal

NOT APPLICABLE (ADR-417 D4).

- The client's race between the answer and the deadline is one `Promise.race` on a
  single-threaded event loop. It is TypeScript, which no formal cover can name, and A10 and A11
  enumerate its orders.
- The route's only shared state is the handshake bound's window, which is read and written inside
  one lock.
- No model or proof under `formal/` cites a file this change touches.
