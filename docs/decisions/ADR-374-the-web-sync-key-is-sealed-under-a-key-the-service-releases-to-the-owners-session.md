---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-374: the web sync key is sealed under a key the service releases only to the owner's live session, opened only in the Worker, and dropped by one generation rule in the core

Decides SPEC-363 (issue #654).

## Context and Problem Statement

#654 (SEC01-F12, found by SEC-01's review, #618) asks where the web client holds the sync
credential, and how it is obtained, stored, replaced and removed. #631's browser sync waits on the
answer (SPEC-357 M23 and section 5).

What is already decided:

- The native client keeps the host key alone in one Keychain item for this device only, not
  synchronizable, and holds the password only for the login call (ADR-358 D2, under ADR-335 and
  SPEC-334 R19).
- The host key is a secret of the password's class. It is derived from the user and the stored
  hash, it never expires, and only a new hash retires it (ADR-347 D13).
- Browser sync is same-origin (ADR-340), under one path on the web origin (ADR-347 D4). The edge
  logs that route with no request header and passes no cookie either way on it (ADR-351 D4, D7).
- The web session is one host-prefixed, script-unreadable cookie that ends 30 minutes after its
  last request and 8 hours after it began, held in memory (ADR-024). A passkey sign-in opens a
  `linked` session, and removing a passkey ends the sessions it opened (ADR-370 D5).
- A card face runs no script, in a sandboxed frame that reaches neither the app nor the network
  (ADR-352).
- The full-sync choice is one rule in the core both clients share (ADR-368 D1). That is the
  precedent for any rule both clients need.

None of them decides:

- where a browser keeps the host key, and what it keeps beside it;
- what protects the key at rest, and who holds what opens it;
- how the key relates to the web session;
- when the key is replaced and when it is dropped, and which side of a race wins;
- what each part of the client (the page, the Worker, the card frame) can reach;
- how long the opened key lives;
- whether the lifecycle is modelled.

## Decision Drivers

- **The key is a password in all but name.** Whatever holds it at rest holds a credential that
  works until the owner rekeys (ADR-347 D13).
- **Four threats, named.** A script in the page, a script in a card, a copy of the browser's
  profile, and a browser more than one person uses.
- **Offline first.** Study never waits on the network or on the credential, and a session started
  offline syncs once it is online (SPEC-334 R5).
- **One owner, one rule.** A rule both clients need lives once, in the core (ADR-368 D1).
- **No secret on anything public, and none in a row** (CHARTER 15). A secret reaches a process as
  a credential its unit loads.
- **No new surface on the sync route.** Its edge rules and its log stand as ADR-351 left them.

## Considered Options (the alternatives each was chosen against)

### D1. Where the browser keeps the host key

- A record sealed with AES-GCM, kept in the origin's indexed database, opened only in the dedicated Worker, under a sealing key the service releases only to the owner's live session — chosen, because
  a record at rest is then useless without a live session, which no script and no copied profile
  can mint without the owner. The key never enters the page, a reply or the card frame. The
  session the service already pins to the owner is the gate (CHARTER 14).
- The host key in plain form in the origin's storage — rejected, because a copied profile or
  a shared browser would sync for as long as the key lives, which is until the owner rekeys
  (ADR-347 D13).
- The key in the Worker's memory only, with the sync password typed at every session — rejected, because
  a sync at each session's start and end (SPEC-334 R5) would ask for a password every time.
- The sync password kept, so the client can log in again silently — rejected, because ADR-358 D2
  refused it: the password mints new keys, and the host key alone serves.
- A relay through the API, so the browser holds no sync credential at all — rejected, because
  the internet-facing API process would hold a password-class secret. It would also break the
  direct same-origin route ADR-347 D4 and ADR-351 built and bounded, carry every sync's bytes
  twice, and tie sync to the API's session bounds. D7 records the session side of it.
- A non-extractable browser key with no server part — rejected, because any same-origin
  script can use the key to decrypt, and the key sits in the same profile as the record, so a
  copied profile carries both.
- A key derived from a passkey's pseudo-random-function extension — rejected, because
  not every authenticator and browser offers it, and it would prompt the owner at every unlock.
  It would also reopen the sign-in ceremony ADR-370 settled.
- Telegram's secure or device storage — rejected, because it exists only inside Telegram's
  clients. The web client also runs outside Telegram, and the value would be read by page script
  through Telegram's script, so the Worker could not keep it.
- Telegram's cloud storage — rejected, because the key would leave the device for a third
  party's servers.
- An `HttpOnly` cookie carrying the key to the sync route — rejected, because ADR-351 D7 strips
  cookies on that route. The protocol's own header carries the key, and the engine writes it.

### D2. What the sealing key is

- A 32-byte key derived as HMAC-SHA-256, under a seal secret of at least 32 bytes that the API's unit loads as the credential `sync-seal-secret`, over a fixed label and a 16-byte seal id the Worker mints at each kept login — chosen, because
  the service keeps no per-browser state and no row. A key released for one record opens no
  other, since each kept login gets a new seal id. Rotating the seal secret retires every
  browser's record at once.
- One sealing key for every browser — rejected, because a key read once by a page script would
  then open every record any browser seals later, until a rotation.
- A random key per browser, kept by the service — rejected, because it is a secret in a row
  (CHARTER 15) and a new store with rights duties, and it must outlive restarts the in-memory
  session store does not.
- A key derived from a secret the service already loads (the bot's token, say) — rejected, because
  rotating one would rotate the other, and units that need the bot's token would also hold what
  opens sync keys.
- A key tied to the web session's id — rejected, because a record sealed in one session could
  not be opened in the next. That brings back the password at every session D1 rejected.

### D3. How the sealing key is released

- `POST /api/sync/seal-key` with `{"seal_id": …}`, taking the owner-session extractor (a `telegram` or `linked` session; a `link` session and no session are refused 401) and the state-change guard, under a window of its own of 30 a minute, answered `no-store` and never logged — chosen, because
  it reuses the two extractors every owner route already trusts and the bound shape ADR-024 and
  ADR-370 D4 use. The Worker alone calls it, and only when it opens a record. Admitting a
  `linked` session is consistent with SPEC-359 R14 and CHARTER 14: the release is none of the
  four commands with stakes (panic, pardon, contracts, the agent's on-demand run), it writes no
  row and changes no session, so it has no destructive effect, and the owner-session extractor is
  the owner gate the amended CHARTER 14 asks of every surface. Passkey removal, R14's one
  destructive route, still needs a fresh `telegram` proof.
- `GET` — rejected, because the state-change guard is the session routes' cross-site defence and
  needs a JSON request with a body.
- Sharing the handshake's window — rejected, because a flood of releases would then lock the
  owner out of the Telegram way in, the recovery path (ADR-370 D4's reason).
- A fresh `telegram` proof only — rejected, because a web client outside Telegram signs in by
  passkey and could then never sync, and a fresh proof lapses within minutes.
- The sealing key inside the session handshake's answer — rejected, because every page load
  would receive it in the page, whether or not a sync was wanted.

### D4. Where the sealed record lives

- One database of the origin's indexed storage, holding one object store with two keys, `generation` and `sealed` (nonce, ciphertext, seal id, user), each step read, decided and written in one read-write transaction — chosen, because
  a Worker can reach it and a transaction is atomic across both keys. Two Workers then never see
  a record and a generation that disagree.
- A file in the origin's private file system beside the collection — rejected, because two files
  cannot be written atomically, and the engine's pool already holds its directories exclusively.
- A table inside the collection — rejected, because the collection is what an upload sends and
  what a backup copies, so the key would travel to the server and into every backup.
- `localStorage` — rejected, because a Worker cannot reach it, and it is synchronous and
  readable from the page by design.
- A cookie — rejected, because a cookie travels with every request to the origin, and the edge
  strips it on the one route that needs it.

### D5. Where the rule that drops a key lives

- One pure module in the core, `crates/engine-core/src/credential.rs`, holding a generation and five functions (keep a login, admit a send, classify an answer, settle an outcome, remove), with the web engine exporting each and the iOS client adopting it when its sync lands — chosen, because
  the race between a removal and a login in flight is the same on both clients, and ADR-368 D1
  placed the full-sync choice in the core for the same reason. The rule decides; each client's
  store only keeps.
- The rule in the Worker's TypeScript — rejected, because the iOS client would write it a second
  time, which ADR-368 D1 refused for the full-sync choice.
- The rule in the web engine's crate — rejected, because the native client does not link it.
- A crate of its own — rejected, because it adds an edge for five functions the core can hold,
  as ADR-368 D1 found for the full-sync choice.
- The rule in Swift beside the Keychain item — rejected, because Swift decides nothing (ADR-358
  D6), and the web client would need its own.

### D6. When a key is dropped

- Only on the owner's sign-out or forget, on a record that does not open under its released key, and on the sync server's refusal (`SYNC_AUTH_ERROR`) of the generation that is still current — chosen, because
  the server's refusal is the one answer that says the key no longer works (a new hash or a new
  password). Every other answer leaves the key good, and dropping it would cost the owner a
  password for nothing. A network failure, a timeout, another sync error, and a release refused
  401, 404 or 429 all keep it.
- Dropping on any failed sync — rejected, because a flaky network would ask the owner for the
  password again and again, and the key would still work.
- Dropping when the web session ends — rejected, because a session ends 30 minutes after its last
  request, and a record that cannot be opened without a session costs nothing to keep. No client
  hook runs when the server ends a session offline.
- A lifetime of the client's own — rejected, because the server's key would outlive it
  (ADR-347 D13), so it retires nothing. It adds a password prompt and protects nothing a
  sealed record does not.
- Dropping on any refusal, whatever generation it refused — rejected, because a second Worker's
  refusal of an older key would delete the newer key the owner had just obtained. The model's
  witness `a-refusal-that-drops-a-newer-key` is that race.

### D7. How the sync key relates to the web session

- They are separate credentials. The session gates the record's opening and never carries the key. Sign-out forgets the key first, then ends the session. A session that ends by its bounds, by a passkey's removal or by a sign-out elsewhere leaves the record sealed and unopenable until a new session — chosen, because
  sign-in stays exactly as ADR-024 and ADR-370 decided, and the sync route stays as ADR-351
  decided. The forget runs offline, so a sign-out never leaves the key behind for want of a
  network.
- The key held by the API inside the session — rejected, because that is D1's relay. It would
  also put a password-class secret behind a session SPEC-359 R14 keeps from every command with
  stakes.
- The session's end deleting the record — rejected, because nothing tells an offline client that
  its session ended, and an idle session ends every 30 minutes.
- Sign-out ending the session first and forgetting after — rejected, because a failed or offline
  logout request would then leave the key in place.

### D8. How long the opened key lives in the Worker

- For the Worker's life: opened at the first sync after the Worker starts, held only in the Worker's memory, and cleared by a forget, by a refusal of its generation, or when the generation it holds is no longer the stored one — chosen, because
  SPEC-334 R5 syncs at each session's end too. Study makes no request to the service, so the web
  session can idle out during a long study session, and a key opened at the start still syncs at
  the end.
- Opening the record for each sync and dropping it after — rejected, because the end-of-session
  sync after a long study session would then ask for a sign-in. A revoked session would stop
  syncing one sync sooner, and that is the whole gain. The consequences below accept the tab's
  life instead.
- Holding the key in the page and passing it to the Worker for each sync — rejected, because every
  page script, Telegram's included, could then read it from the page.

### D9. What each part of the client can reach

- Only the Worker's credential module opens the store, holds the opened key, calls the release, or puts the key in a request, and only to its own origin's sync route. The Worker's replies carry status words (`absent`, `sealed`, `held`, `needs-sign-in`, `offline`), never a value. The card frame stays as ADR-352 built it — chosen, because
  the key then has one holder and one destination a census can hold. A page script never
  receives it in a reply, and a card's frame has an opaque origin that reaches neither the
  store, the session's cookie nor the Worker.
- A status operation that also returns the user name — rejected, because the user is part of the
  credential (the sync job loads it as one), and the page has no use for it beyond a label the
  owner typed.
- The page owning the store and the Worker only syncing — rejected, because the page runs
  Telegram's script and every other page script beside it.

### D10. How the key is obtained

- The owner types the sync user and password into a form on the sync screen. The page posts both to the Worker in one message. The Worker asks for the release first, refusing `needs-sign-in` before the login when there is no session, then runs the engine's login against its own origin's sync route through the core's endpoint guard, and seals what the core keeps — chosen, because
  the password leaves the page only for the Worker and lives only for the call (ADR-358 D2's
  rule). The endpoint is derived, never typed. A login with no session to seal under never sends
  the password at all.
- The user name built into the web bundle — rejected, because the sync user is a credential
  (the sync job loads it as one), and the bundle is public. ADR-358 D3 renders it into a
  private build for the native app, which a public web bundle cannot do.
- The endpoint typed by the owner — rejected, because the same-origin route is the only one the
  page's policy admits, and a typed endpoint is the hazard the core's guard exists for (ADR-358
  D4).
- A password form inside the Worker — rejected, because a Worker has no document to draw one in.

### D11. Replacement and removal

- Re-login keeps the newer key by the core's generation. A password change or new hash on the server shows up as the next sync's refusal, which drops the key and asks for the new password. Rotating the seal secret makes every record fail to open, which deletes it. Account removal on the service is those two levers; a browser drops its record at its next refused open or sync — chosen, because
  every replacement and removal is then one of the core's transitions or the server's own
  rekey. No lever needs the browser to be online when it is pulled.
- A list of live records kept by the service, revoked one by one — rejected, because it is a row
  per browser (D2's rejected option) and a new revocation surface.
- Keeping a record that does not open — rejected, because it can never open again, and it would
  report `sealed` forever.

### D12. Whether the lifecycle is modelled

- A TLA+ entry, `formal/tla/SyncCredential`, written first. It holds two Workers, the owner's login and sign-out, the sync server's refusal after a rotation, network failures, a Worker's restart and the session's opening and end, with five properties, each with a witness — chosen, because
  the failures here are interleavings: a sign-out racing a login in flight, a refusal racing a
  newer login, a removal racing a Worker that holds the key. A unit test of one order says nothing
  about the others.
- No model — rejected, because the rule's worth is the order of these steps, and the formal pack
  requires a model for two actors over shared state.
- A Lean proof of the rule — rejected, because the rule is four comparisons that unit tests and
  mutation rows hold; the risk is in the order, which a proof of the functions does not state.
- Extending `formal/tla/FullSyncChoice` — rejected, because its actors are different, an entry
  cannot extend another's module, and every edit would move its covers.

## Decision Outcome

D1 to D12 as chosen above. The browser keeps the host key as D1 chose: an AES-GCM sealed record
in the origin's IndexedDB, opened only in the dedicated Worker, under a sealing key the service
releases only to a live owner session. The build is two pull requests:

- The first carries the model first, then the core's rule and its exports, and the service's
  release.
- The second carries the Worker's store, its operations, the page's sign-out and the reach census.

SPEC-357's part b proceeds as designed and plugs its login and sync into this store's obtain,
send and settle. Its rows that change:

- §9 V1 now reads "once online and signed in";
- M23 is decided;
- the waits in its section 5 on #654 are met;
- part c gains the `needs-sign-in` state.

## Consequences

- Good, because a record at rest is useless without a live owner session, so a copied profile
  with no live session, or a shared browser after a sign-out, cannot sync.
- Good, because the sync route, the sign-in and the card frame are unchanged, and the key has one
  holder and one destination.
- Good, because the rule both clients need is written once, modelled, and held by mutation rows.
- Good, because study never waits on the credential, and a network failure never costs a key.
- Bad, because a script running in the page during a live session can obtain the sealing key and
  read the record, and so obtain a key that works until the owner rekeys. No same-origin store is
  closed to a same-origin script. The page's script sources narrow who can run there, and the
  sync route's log records each request's address.
- Bad, because a Worker that opened the key keeps syncing after its session is revoked, until its
  tab closes.
- Bad, because a copied profile that carries a live session (at most 8 hours old) can open the
  record. The owner's answers are a sign-out, a seal-secret rotation and a new hash.
- Bad, because outside Telegram a new tab needs a passkey sign-in before its first sync. That
  rests on the web sign-in screens (#627).
- Bad, because the password is typed into a page form, which page script can read while it is
  typed. A Keychain prompt has no web equivalent.
- Bad, and accepted as this decision's residual: Telegram's script loads on every route of the
  page (`web/app/src/app.html:6`), the web client outside the Mini App included, so it runs beside
  the page that asks for the release and draws the sync form. It is one more same-origin script of
  the first Bad consequence above, and it never reaches the Worker, the opened key or the card
  frame. Loading it only inside Telegram is its own change, not this delivery's.
- Neutral: a lost device that has synced still needs a new hash (ADR-347 D13 is unrelaxed).

## What would make this wrong

- A second origin for sync, or a sync route that sets or reads cookies: D1 and D3's release would
  then need to be rethought.
- An edge rule that answers 403 on the sync route: D6 would read the edge's 403 as the server's
  refusal and drop a good key.
- A key with a lifetime, or one minted per login, at the sync server (#649): D6's refusal rule
  stands, but the generation would then also need the server's expiry.
- A browser that offers a hardware-bound, non-exportable key a same-origin script cannot use:
  D1's sealed record could then drop the service's part.
- More than one API process behind the edge: the release's window would need sharing as the
  session store's would.

## Amendment: ADR-414 closes the residual of Telegram's script on every route

Lines 286-290 accept, as this decision's residual, that Telegram's script loads on every route of
the page. ADR-414 (SPEC-400) closes it: the script loads only when Telegram launched the page, and
outside a launch the page adds a policy that refuses the script's origin. The web client outside
the Mini App therefore runs no script from Telegram beside the release and the sync form. The rest
of this decision is unchanged.
