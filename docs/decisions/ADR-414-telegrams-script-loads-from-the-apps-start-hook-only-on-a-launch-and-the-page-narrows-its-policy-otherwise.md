---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-414: Telegram's script loads from the app's start hook only on a launch, and outside a launch the page narrows its own script policy

Decides SPEC-400 (issue #706). It closes the residual SPEC-363 records as M11 and ADR-374 accepts in
its Consequences.

## Context and Problem Statement

The web client is one static single-page app. The server answers every route with the same
fallback document (`deploy/caddy/deck-streak.caddy:91`, `web/app/svelte.config.js:14`). That
document's shell loads Telegram's Mini App script from telegram.org (`web/app/src/app.html:6`), and
the policy SvelteKit writes into it admits that origin on every route
(`web/app/svelte.config.js:27`). So a page opened in a plain browser tab still runs another party's
script beside the page that asks for the sync key's release and holds the sync form. That tab is
the web client SPEC-363 serves outside the Mini App.

#706 asks for three things:

- the page loads the script only when Telegram opened it;
- the page decides that from the launch parameters, read before anything rewrites the URL;
- the policy admits the origin only where the script can load, or the SPEC records why it cannot.

What is already decided, and stays:

- ADR-005: the official script behind one typed wrapper.
- ADR-007: the policy in a meta element, in hash mode.
- SPEC-028 R2: `web/app/src/lib/telegram.svelte.ts` is the only module that reads the script's
  object, held by `web/app/src/lib/telegram-boundary.test.ts:50-56`.
- SPEC-024: the server validates the launch data on `POST /api/session`.

## Decision Drivers

- The launch parameters arrive in the URL fragment, and no request carries a fragment to the
  server. The first app code that rewrites the URL is the root layout's redirect
  (`web/app/src/routes/+layout.ts:25`), and it drops the fragment.
- The wrapper reads the script's object once, when its module loads
  (`web/app/src/lib/telegram.svelte.ts:150-152`). Every other reader goes through the wrapper.
- SvelteKit hashes only the inline scripts it generates itself (kit.csp in hash mode). The client
  `init` hook runs once when the app starts, and SvelteKit awaits it before the first navigation.
- The launch data keeps its one path: one request body to `POST /api/session`
  (`crates/api/src/session_routes.rs:116`, `:135`), checked by the one validator
  (`crates/identity/src/init_data.rs:88`).

## Decision Outcome

### D1. The population

The build changes or re-proves every item below, and nothing else.

- **The load:** `web/app/src/app.html:6`, the only place the script is loaded.
- **The policy:**
  - `web/app/svelte.config.js:27` is the only policy line that admits the origin.
  - The served header (`deploy/caddy/deck-streak.caddy:19`) names Telegram's web client only in
    `frame-ancestors`. That says who may frame the page, admits no script, and stays.
- **The six readers:**
  - the wrapper (`web/app/src/lib/telegram.svelte.ts:89-98`, at module load `:150-152`);
  - the root layout's load (`web/app/src/routes/+layout.ts:20-26`, its redirect at `:25`);
  - the root layout's `ready()` (`web/app/src/routes/+layout.svelte:26`);
  - the API client's handshake (`web/app/src/lib/api.ts:114-121`, its launch-data source at
    `:239`);
  - the about page's `openLink` (`web/app/src/routes/about/+page.svelte:16`);
  - the review screen's colour scheme (`web/app/src/lib/study/ReviewScreen.svelte:200`).
- **The styles:** the `--tg-*` reads (`web/app/src/routes/layout.css:28`, `:42-46`), each with a
  fallback.
- **The routes:** every path of `web/app/src/lib/routes.ts:10`. All seventeen are served by the one
  fallback document, so the change is the same for every route.

### D2. The detection and the load

A new module, `web/app/src/lib/telegram-launch.ts`, decides the launch. The app's client `init` hook
(`web/app/src/hooks.client.ts:7`) awaits it before the router's first navigation.

- **A launch** is either:
  - a fragment that names `tgWebAppData`, `tgWebAppVersion` or `tgWebAppPlatform` (presence only,
    no value read), or
  - the tab's own launch mark in session storage.
  A launch read from the fragment writes the mark, which holds no launch data. A storage error
  leaves the fragment as the only test.
- **On a launch:** the hook adds the script to the head with `referrerpolicy="same-origin"` and
  awaits its `load` or `error`.
- **Outside a launch:** the hook adds no script, and adds D3's policy instead.
- **The wrapper is untouched.**
  - The hook never imports it (SPEC-400 A10), and it stays the only reader of the object
    (SPEC-028 R2).
  - Its module loads after the hook, so it reads the object the script left, or none.
  - Outside a launch it says it is outside. Every Mini App call then answers "reopen from
    Telegram", as it does today for a page with no launch data.
- **Tests:**
  - A1 and A2 are red first in CI's `web` check run. Playwright observes every request with
    `page.on('request')`.
  - A5 to A11 are red first locally, in Vitest, against a stub of the launch module that keeps
    its inputs and does nothing.
  - A3, A4, A12 and A13 are not red: each pins behaviour the base already has.

### D3. The policy

`svelte.config.js` keeps the origin. Outside a launch, the hook adds a second policy to the head,
`script-src 'self' 'wasm-unsafe-eval'`.

- Every policy a document holds applies, so once the second policy is in the head, a script element
  naming the origin is refused before any request goes out.
- The app's own scripts, its Worker and WebAssembly still load.
- What holds: inside a launch the origin is admitted, where the script must load; outside, it is
  refused for the page's life.
- SPEC-400 §1.3 records why the static policy itself cannot be narrowed.

### D4. FORMAL, decided by surface

NOT APPLICABLE, for both a model and a proof. The launch data's handoff is unchanged, and the gate
adds no actor.

- The handoff keeps the same bytes, one request body and the same validator.
- The gate is one page's sequential start: one `await` in the start hook, before the router runs.
- The tab's mark is written and read by that one page, at most one page at a time.
- The detection is a disjunction of two inputs, whose whole domain SPEC-400 A5 enumerates.

### D5. Drift and the push count

The build cuts only after #748 lands, and pushes twice.

- The first push holds the documents and the end-to-end tests alone. Their red is read in CI's
  `web` check run while the unit suites stay green, so the check reaches its end-to-end stage.
- The second push holds the unit tests, the code and the red-first record.

## The alternatives each decision was chosen against

### D1. The population

- Chosen: the load, the policy line, six readers, the styles and every route, because the build
  changes or re-proves each one and nothing else.
- Chosen against a list of routes (only the web client's own pages): the routes do not decide
  inside or outside, because every route is reachable both ways through the one fallback document.
- Chosen against taking the population from the routes table alone: that misses the policy line,
  the readers and the shell, which carry the change.

### D2. The detection and the load

- Chosen: the app's start hook, through `telegram-launch.ts`, because it is the one place SvelteKit
  runs and awaits before the first navigation, while the fragment is still whole.
- Chosen against loading the script on every route (the base): it runs another party's code in
  every page, the web client's included, which #706 refuses.
- Chosen against a user-agent check: no Telegram client is bound to name itself there, and the
  script reads the fragment, so the check could disagree with the script it gates.
- Chosen against an inline script in the shell: SvelteKit hashes only the inline scripts it writes,
  so a hand-written one needs a hand-pinned hash beside the policy (SPEC-028 A13 holds the shell
  to no inline script), and an element it appends would load with nothing holding the app back.
- Chosen against writing a cross-origin script into the document as it is parsed: a browser may
  refuse a parser-blocking script written that way, so the launch would depend on the connection.
- Chosen against a separate same-origin entry script before the app: it has the same ordering
  problem, and it is one more file for the policy and the boundary census to name.
- Chosen against a lazy load inside the wrapper: every reader (the layout's load, the API client,
  the about page, the review screen) would turn asynchronous, and the wrapper's
  read-once-at-module-load contract (SPEC-028 R4) would end.
- Chosen against detecting a reload from Telegram's own session-storage key: that key is another
  party's private storage, with no published contract, so the app keeps a mark of its own.

### D3. The policy

- Chosen: the static policy keeps `https://telegram.org`, and outside a launch the start hook adds a
  second, narrower policy, because every policy a document holds applies.
- Chosen against a per-route policy: one static document answers every route, and the routes do
  not decide inside or outside.
- Chosen against a nonce: it needs a server that renders each response, and SvelteKit refuses a
  nonce for a prerendered page.
- Chosen against a hash or subresource integrity for the script: it pins bytes that Telegram
  updates in place at one URL, so the script would stop loading at its next update.
- Chosen against a build per mode (two builds or two hosts): it changes the Mini App's launch URL
  and the deploy, for a fact the client already knows at start.
- Chosen against recording the residual only: it leaves the origin admitted on pages that never
  load it, so an injected element naming any script there would still run.

### D4. FORMAL, decided by surface

- Chosen: NOT APPLICABLE, because the launch data's handoff is unchanged and the gate adds no
  actor over shared state.
- Chosen against a TLA+ model of the start sequence: it would hold one actor and one ordered step,
  which no interleaving can reorder.
- Chosen against a Lean proof of the predicate: a formal cover names only a Rust, Python or shell
  file, so nothing could keep a proof attached to TypeScript code.

### D5. Drift and the push count

- Chosen: the build cuts only after #748 lands, and pushes twice, because #748 changes the threat
  model this build adds a row to.
- Chosen against cutting now: the threat model is a shared file, and the build also moves the line
  the threat model's T2 row cites in `web/app/src/lib/csp.test.ts`.
- Chosen against one push: A1 and A2 are red only in CI, and a red read in CI needs its red commit
  pushed alone first.
- Chosen against a third push for the unit reds: each one is read locally, against its stub,
  before the code exists, so no CI run is needed to see it.

## Consequences

- **Outside a launch:**
  - no route requests telegram.org;
  - `window.Telegram` is undefined, and the wrapper's `inside` is false (the base's real script
    set the object with empty launch data). No reader reads `inside`;
  - the `--tg-*` variables are unset, and every read of one falls back.
- **On a launch:**
  - the script loads before the first navigation, as it did when it was the shell's first script;
  - the first paint waits on the script, as it did when the script blocked the parser.
- **Restated tests:**
  - **The shell check moves out of the first test** (`web/app/src/lib/csp.test.ts:44-50`). It
    becomes its own test, which is stronger: the shell holds no script, where the base held
    exactly one.
  - **The referrer test** (`:81-88`) now holds the meta ahead of `%sveltekit.head%`, and holds the
    added element to `referrerpolicy="same-origin"`.
  - **The smoke test** (`web/app/tests/smoke.spec.ts:27-31`) says where the script is: present on
    a launch, absent outside one.
  - **The palette audits** (`web/app/tests/a11y.spec.ts`, `web/app/tests/streak-calendar.spec.ts`)
    open the page as a launch and prove that their stand-in ran.
- **"First in the head" is retired.** It was in SPEC-028 R1 and R4, ADR-005, and the launch
  schematic's first section. What it was for, the script running before any navigation, is now
  SPEC-400 A3. Each of those documents takes an insert-only note.
- **A crafted link.** A link that carries launch parameters loads the script in a plain tab, as
  every page did before. The server validates every session, so the script gains no session
  there.

## What would make this wrong

- **A Telegram client that opens the Mini App with none of the three parameters in the fragment.**
  The page would treat itself as outside, and the wrapper would answer "reopen from Telegram".
  This is detected by SPEC-400's person-proved launch, on each client the owner uses.
- **A browser that ignores a policy meta inserted after the document was parsed.** The narrowing
  would then not hold. SPEC-400 A2 detects this in the default test browser, and the study suite
  runs the narrowed page in two engines.
- **Telegram's script stops working when it is added after the document is parsed.** This is
  detected by the person-proved launch: no theme, no `ready()`, and a "reopen" answer inside
  Telegram.
- **SvelteKit stops awaiting `init` before its first navigation.** SPEC-400 A3 goes red: the
  redirect drops the fragment before the stand-in reads it.

## Amended by ADR-417

ADR-417 (SPEC-403, #775) amends D2's detection for a launch read from the fragment (lines 78-79). The start hook reads the fragment's `tgWebAppData` value, sends it to `POST /api/launch`, and adds Telegram's script only when that answers 204. A 401 or a 403, any other answer, a failed request, or no answer within 10 s adds no script and narrows the policy. The tab's mark is written only after a 204, under a new key. D1, D3 and D4 stand. The crafted-link consequence (lines 219-221) is closed by it.
