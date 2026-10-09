---
status: "accepted"
---

# ADR-385: the sync service states a minimum client level, and the engine core refuses every sync below it

Decides SPEC-374.

## Context and Problem Statement

#671 asks two things. First, the sync service states the oldest client it accepts, and a client
below it shows an update message instead of syncing, stopping before any sync write. Second, a
scheduled check reads the age of the newest internal TestFlight build and starts a re-release
before that build stops working.

What the tree holds, at `6f9ef860` (SPEC-374 section 1 has every citation):

- One engine core runs every client's engine calls, native and web, and already refuses a sync's
  endpoint before the engine, in the engine's own error shape (`dispatch.rs:157-169`,
  `login_guard.rs:79-89`).
- A build number counts `dev` for an internal build and `main` for a release
  (`scripts/ios_lane.py:120-134`); a marketing version is shared by every build between releases
  (`:116-117`); the web client has neither.
- The Swift app may make no request (`test_ios_thin_swift.py:92-94`).
- reqwest's client is made only at allow-listed sites (`clippy.toml:7-15`,
  `request_allow_list.rs:54`).
- At this commit no web client can sync (SPEC-364 R14), and the internal lane archives a scheme
  with no sync (SPEC-352; the move is #625's), so no client that syncs has shipped.
- The internal lane runs on a dispatch or a push to `dev` (`testflight-internal.yml:10-24`).
  Scheduled workflows run from the default branch's copy only (`mutation-weekly.yml:17-18`).
- At `f7d62691`, which holds SPEC-364 part b2, the web client syncs through its Worker, and the
  core's rule refused every web sync pair: CI's web-engine job failed the three sync specs on both
  browsers, because the Worker handed the core no statement (D10).

## Decision Drivers

- The stop must hold before any sync write, local or remote, on every client, and say why.
- A client that cannot read the statement must not sync anyway: a stop that fails open lets
  exactly the stale client the issue names through.
- One rule in one place, so the clients cannot drift apart.
- No guard is quietly routed around: the thin-Swift rule and the request allow-list stand.
- The check must never report a healthy build it did not read, and never tag, touch `main` or
  release.

## Considered Options (the alternatives each decision was chosen against)

### D1. What the service states and what a client compares

- **Chosen: a client level, one integer the engine core declares and the API states at an open route, because every client compiles the core, so one constant is comparable across iPhone, iPad and web.**
- The build number: lost because it is not comparable across lanes (an internal build counts `dev`, a release counts `main` at its tag), and the web client has none, so it lets a web client through unjudged.
- The marketing version: lost because every internal build between two releases shares it, so it cannot stop an internal build that predates a protocol change.
- The engine's own sync protocol version: lost because it is fixed by the engine pin and changing it needs patches to the engine on the server and every client, so it lets through every change made on DeckStreak's side of the engine.
- A setting at the edge or on the host: lost because no test on this box may run the deploy tests, so a value no test reads could strand the clients the same release serves.
- A level carried in the sync endpoint's path and refused by the server: lost because the engine maps a server refusal to its own message, so the client could not say why.

### D2. What a client below the minimum does, and where it stops

- **Chosen: the engine core refuses the sync login, the normal sync and the one-way sync before the engine unless the latest statement admits, on both transports, failing closed when no statement was read, because it is the one place every sync passes before any write.**
- A stop in each client's own code (Swift, TypeScript): lost because Swift may make no request and two copies drift apart.
- A stop only on a statement that reads below, admitting when none was read: lost because a client that cannot reach the statement route but can reach the sync server would sync below the minimum.
- Changing the iPhone and iPad UI test's network-sentence oracle: lost because the unread sentence can begin with the engine's own network sentence, so the oracle stands unchanged.
- The web client out of scope with no rule: lost because once #631's sync lands a tab left open across a release could sync below the minimum; the core's rule refuses it unless the Worker's read admits (D10).

### D3. The re-release check

- **Chosen: a daily scheduled workflow reads the newest valid internal build's expiration from the TestFlight build API and, when it is within 7 days, dispatches the internal lane on `dev`, because only the store knows when a build stops working and a dispatch is the lane's own manual trigger.**
- Reading the lane's own run history for the last upload: lost because a successful run is not a live build, so it would report a healthy build it never read.
- A report or an issue only: lost because #671 asks the check to start a re-release.
- Reusing the lane's upload environment and its credential: lost because the check needs only to read, and a scheduled run starts from the default branch, which a `dev`-only environment would refuse.
- A lead equal to the schedule's interval: lost because one failed or skipped scheduled run would let the build lapse; a 7-day lead gives seven runs.

### D4. The tests and how the box-restricted modules bind them

- **Chosen: part one's every criterion is a Rust or a standard Python test run before the push, so it lands in one push; part two's workflow criteria live in `test_ci_workflows.py`, read in CI, so it takes exactly two pushes, because that module runs only under its own rulings.** Part one's fix round (D10) adds web criteria in one more push: vitest tests run before the push, and three browser specs CI's web-engine job reads.
- A new workflow-reading module run on the box: lost because it would read workflows outside the rulings that restrict the one module that already does.
- A Swift test of the update sentence: lost because the stop lies in the static library before any Swift write, the Rust test reads the sentence the Swift view shows, and Swift runs only in CI.

### D5. Shape

- **Chosen: two deliveries, the handshake and then the re-release check, because they share no code, need different push counts and different models.**
- One delivery: lost because the CI-only workflow criteria would hold the box-run handshake behind two pushes and one larger review.

### D6. Where the native client reads, and the request allow-list

- **Chosen: the static library reads the statement in `Engine::run` before the core, with reqwest's client at one new named site in the request allow-list, because Rust is the only client code that may make a request and the allow-list is the guard's own way to admit a site.**
- The read in Swift: lost because `URLSession` and `URLRequest` are forbidden in every Swift file.
- reqwest's blocking client, or the HTTP stack beneath reqwest: lost because neither is on the allow-list's four paths, so it would route around the guard instead of passing it.
- The read inside the engine core: lost because the core also builds for the web, where the Worker owns the network.

### D7. A due build on an unmoved `dev`

- **Chosen: no dispatch, and the run fails naming the build and the fact that `dev` has not moved, because a rebuild of the same commit gets the same build number, which the upload refuses.**
- Dispatching anyway: lost because every such run would end in a refused upload that names no cause.
- A build number taken from anything but `dev`'s count: lost because SPEC-352 fixes the internal number to that count.

### D8. Formal models, decided by surface

- **Chosen: part one owes a TLA+ model and part two owes none, because the handshake is a read followed by a sync the service can change in between, while the check is one scheduled actor whose only race ends in a refused duplicate upload.**
- No model for part one: lost because a sync between a raise and the next read is exactly the interleaving a test cannot enumerate.
- A model for part two: lost because the check reads the store and `dev`, then dispatches the lane once, under its own concurrency group; the lane holds its own runs one at a time per event, and the one overlap, a push-run building the same commit, ends in the upload's refusal of a duplicate number.

### D9. The census of the two levels

- **Chosen: a census test holding each level defined once and the minimum at or below the level, with a planted control, because no tree may state a minimum its own clients fail.**
- No census: lost because a minimum raised past the level in one release would stop every client that release ships.

### D10. Where and when the web client reads the statement, and in which delivery

- **Chosen: the web Worker reads the statement through its own `fetch` before its sync login and before each normal sync, and hands it to a web engine `handshake` export that answers the core's decision, in part one's own fix round, because the Worker owns the web's network (D6), the core keeps the one rule (D2), and the fix round lands with part one.**
- A separate later delivery for the web read, leaving web sync refused on `dev` between the two lands: lost because SPEC-364's web client already syncs on `dev`, so every web sync there would be refused from part one's land until that delivery's.
- A read only before the sync login: lost because a tab left open across a raise would keep syncing below the minimum until its next login (D2).
- Following a redirect, or sending the page's credentials: lost because the static library's read does neither (D6, SPEC-374 R8), and a statement moved off the origin is not the service's own.
- The decision kept in the Worker's TypeScript: lost because two copies of the rule drift apart (D2); the export answers the core's own decision and sentence.
- A web engine export that fetches the statement itself: lost because the engine owns no network on the web (D6); the Worker's `fetch` is the one network the browser gives it.

### D11. How a refusal reaches the page

- **Chosen: the Worker throws the core's sentence and the session answers it as `engine-failed` with that sentence as its message, because the page's refusal already carries a message and the page's status words are closed over the protocol's codes.**
- A new protocol code for a client below the minimum: lost because the study screens' status map is exhaustive over the codes, so a new code changes screens this delivery does not touch, and the sync screen that would read it is #631's next part.
- Answering `offline`, as a failed login does: lost because it hides the sentence #671 asks the client to show.

## Decision Outcome

- D1: `CLIENT_LEVEL` in `crates/engine-core/src/handshake.rs`; `MINIMUM_CLIENT_LEVEL` in
  `crates/api/src/minimum_client.rs`, at `GET /api/sync/minimum-client`, 200 with
  `{"minimum_client_level":<n>}` and `Cache-Control: no-store`, beside the health routes.
- D2: `Dispatcher::handshake` keeps the latest outcome, shared with private engines; `run` and
  `full_sync` refuse every sync pair unless it is admitted, with SPEC-374 R6's sentences.
- D3, D7: part two's workflow and script (SPEC-374 section 7).
- D4, D5: two deliveries; part one one push and one for its fix round (D10), part two two.
- D6: the read in `crates/ffi/src/engine.rs`, a named allow-list site, no new package.
- D8: `formal/tla/MinimumClientHandshake/`.
- D9: `scripts/tests/test_client_level.py`.
- D10: the Worker's read in `web/app/src/lib/engine/sync.ts`, wired with the Worker's own `fetch`
  in `web/app/src/lib/engine/worker.ts`, handed to the `handshake` export in
  `crates/web-engine/src/wasm.rs`.
- D11: a refusal answered as `engine-failed` with the core's sentence; the protocol's codes
  unchanged.

### Consequences

- Every client that syncs carries the handshake from its first release; no client built earlier
  syncs (#625 ships the first).
- The web client's Worker reads the statement before its sync login and before each normal sync
  (D10), so no web sync is refused for want of a read, and a refused one shows the core's sentence
  (D11).
- A minimum is raised in two releases: the level first, the minimum later.
- The re-release check goes live only when a release carries its workflow to `main`.

### Confirmation

SPEC-374's A1 to A7 and A16 to A22, and A8 to A15 with part two; the TLA+ entry's witnesses
caught and its property clean at its floor; the request allow-list's own test with the new site
named.

## More Information

SPEC-374, its schematic `docs/schematics/minimum-client-handshake-and-re-release-check.md`,
SPEC-352 (the internal lane), SPEC-364 (the web client's sync), SPEC-041 (the request allow-list).

## Amendment: part two lands the re-release check

Part two delivers D3 as it is decided, with the tests D4 names, as the second of the two deliveries D5 chose; D7 holds for a due build on an unmoved `dev`, and it owes no formal model (D8). It adds one decision.

### D10. The check's own credential

- **Chosen: the check reads with a credential of its own, held only in an environment of its own that only the default branch may enter, because a job that runs on a schedule should hold the narrowest credential that does its one read.**
- Reusing the internal lane's upload credential: lost because it is a wider credential than a read needs, held every day by a job that runs on a schedule.

Placing the credential is an owner act outside this delivery. Until it is placed, every run fails closed with a named reason and dispatches nothing (SPEC-374 R14, A11).
