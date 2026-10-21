# SPEC-334: the app campaign: a web client and a universal iPhone and iPad client study the owner's collection over the engine, against the owner's own HTTPS sync server

- **Wave:** the app campaign (a campaign PRD, as SPEC-001 is for parity). **Issue:** the campaign's
  issues, one per delivery, each citing the row of this SPEC it proves. **Context(s):** every
  context the two clients reach; the engine core, the FFI and WASM crates, the XP crate and the
  FSRS-7 crate join the context map through their own deliveries' ADRs.
- **Decided by:** ADR-335, ADR-336, ADR-337, ADR-338, ADR-339, ADR-340, ADR-341, ADR-342, ADR-343,
  ADR-344, and the owner's two signed rulings,
  `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md` and
  `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`.
- **Status:** judged. This delivery writes documents only and has no behaviour, so it adds no test
  and no red-first record; every row below is proved by the later delivery it names.

## 1. The problem, measured

- **Where the owner studies today.** DeckStreak reads a private copy of the collection and never
  presents a card (CHARTER 4). The owner studies in Anki's own clients, desktop and AnkiMobile,
  which sync to the owner's own sync server. DeckStreak cannot show a card, take an answer or show
  instant XP for one.
- **No client in the tree.** `git ls-files '*.swift' | wc -l` prints 0. `git grep -l wasm32 --
  Cargo.toml 'crates/*/Cargo.toml'` finds no file. The engine is a dependency of two crates only
  (`git grep -l '^anki' -- 'crates/*/Cargo.toml'`: `crates/ingest/Cargo.toml` and
  `crates/readings/Cargo.toml`), and both read.
- **One transport.** The notification router's only outbound port is `BotTransport`
  (`crates/notifications/src/transport.rs:84`), and `git grep -il 'apns\|web.push\|webpush' --
  crates` finds no file. Once Telegram retires, nothing could carry a nudge or a celebration.
- **What the owner decided.** In the design rounds the owner chose a real Anki study client on
  the web and on a universal iPhone and iPad app, over the same engine, as the owner's primary Anki
  client with AnkiMobile parity reached in phases. They also chose to retire the Telegram surfaces
  once native push carries the router, and to move the sync server to the service's own host
  behind HTTPS. The requirement ids below (`ANK-01`, `SCP-03` and the rest) are the ids of those
  answers. Two of the decisions change binding text, and the owner has signed rulings for both:
  the owner's own taps against ADR-301's never-list, and the app's surfaces against CHARTER 2, 4
  and 14, the PRD's third non-goal and the design system's first two principles.

## 2. Requirements

R1. (DS-01) The clients are DeckStreak's new surfaces, built in this repository under its charter,
    its gate and its house order; they are not a separate project.
R2. (ANK-01, ANK-05, ANK-06) The study client is a real Anki client: Anki's engine, pinned to one
    upstream tag, opens the collection, presents cards and records answers. The stock scheduler
    (FSRS-6, through the engine) schedules every card except those on the one preset the owner
    switches to FSRS-7 (R10).
R3. (CORE-01, IOS-01, ANK-02) One engine port has two transports: FFI on iPhone and iPad (ADR-335)
    and WASM in the browser (ADR-336). The isolated FSRS-7 crate and the I/O-free XP crate sit
    beside the engine and never link into it.
R4. (IOS-01, GAM-01, UX-01, DEV-01) The iPhone and iPad client is one universal SwiftUI app with a
    native review screen and adaptive layouts, on current iOS (ADR-335, ADR-342).
R5. (ANK-02, WEB-01) The web client is SvelteKit. Its engine runs in the browser, in a Worker
    over OPFS storage with a request for persistent storage; it syncs at session start and end and
    warns while reviews are unsynced. If the browser engine spike says NO-GO, the server-side
    study collection is the fallback (ADR-336).
R6. (UX-05, RND-01) A card face renders as HTML in a sandboxed, no-network frame: an iframe on the
    web and a WKWebView on iPhone and iPad. A test proves that card JavaScript reaches neither the
    native or Worker bridge nor the network.
R7. (ANK-03) The owner's own taps are exempt from ADR-301's never-list exactly as
    `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md` words it, and every other path stays
    bound. The exempt functions are reachable only from the study client's UI layer, a test proves
    that no non-UI caller reaches any of them, and nothing calls one before that ruling is on
    `dev` (ADR-337).
R8. (SYNC-01, BAK-01) A one-way sync, upload or download, is the owner's explicit tap. Before
    either, the client shows what each side loses and writes an on-device backup; before an
    upload it checks that the server's offsite snapshot exists.
R9. (SRV-01, ANK-07, ANK-08, BAK-01, SEC-01) The sync server moves to the service's own host,
    behind HTTPS with hashed passwords. An offsite snapshot exists and its restore is drilled
    before the move, a security review runs before it faces the internet, one cutover repoints
    desktop, AnkiMobile and the app with a new sync password, and the current third-party host
    stays as the rollback until the owner retires it (ADR-340).
R10. (OQ1, the FSRS-7 state answer) FSRS-7 runs live on one preset the owner switches. Its
    memory state is rebuilt from each card's review history when the collection opens, and only
    the stock fields are written, so they survive a stock sync server (ADR-338).
R11. (OQ4, OQ2, OQ3) Every other experimental model only reorders cards the stock scheduler
    already made due; it never changes which cards are due, an interval or a due date. The
    repository carries a generic input of external due-card scores and no code or weights of a
    model whose licence does not admit it (ADR-339).
R12. (SCP-03, NOTIF-01) APNs and web push become the one router's transports: opt-in, at most one
    nudge a day, revocable per device. The Mini App and the bot retire only after native push
    carries the router, and the cutover from the predecessor runs on today's surfaces (ADR-341).
R13. (GAM-01, GAM-02) Per-review XP shows on the device at once, from the XP crate; day-level
    bonuses stay pending until sync; one ledger on the server confirms; and confirmed XP is never
    below the XP shown. The flat multipliers of economy v10 wait on #62 and #64; until then the
    crate computes the predecessor's math, held by the parity oracle.
R14. (GAM-03, UX-02, MET-01) The clients are dark-first and colour-rich at WCAG 2.2 AA in both
    modes, vivid in a session and calm outside one. Whether the new interface worked is measured
    by learning outcomes from the review log only (days studied, true retention, minutes per
    retained card), with no new telemetry.
R15. (HW-01, UX-04) An 8BitDo remote drives studying in both of its modes: the GameController
    framework and UIKeyCommand on iPhone and iPad, the Gamepad API and keyboard events on the web.
    Its buttons map to show answer, the four grades, undo, bury, flag and replay audio, and the
    screen stays awake while it is paired. The web client also takes Anki's desktop keys
    (ADR-342).
R16. (AI-01, AI-02, AI-06, PER-01, PRE-02) A persona tab, and a sheet in review available only
    after reveal with the card timer paused, run on the deployment's AI route: an API key for a
    self-hoster's deployment, and the owner's route behind #162 and #43. A persona created in the
    app speaks only after the persona-core gates pass it (ADR-343). The day's reading sits at
    session start and is skippable.
R17. (REL-01, DIST-01) The internal TestFlight lane runs throughout the build. It starts right
    after the Swift harness, and every later iOS delivery puts a build on internal TestFlight,
    with the owner the only tester. Until the real-data gates clear (both rulings on `dev`, and
    the sync cutover), those builds point at the staging sync user; real data reaches a device
    only from a SemVer tag on `main`. ADR-344 decides the internal builds' trigger and build
    number.
R18. (STAT-01) The study client shows the stock scheduler's output as Anki does: the next interval
    on the answer buttons, a card's due date, the browser's Due column and the future-due
    forecast. The PRD's third non-goal binds DeckStreak's game, public and AI surfaces (ADR-338).
R19. (AUTH-01, AUTH-02, AUTH-03) Sign-in on the web and on iPhone and iPad pins to the one owner
    account by ADR-131's and ADR-132's methods, and no surface opens a session before its gate is
    built. The native sync login and the bearer token sit in the Keychain, and native calls reach
    the service through the Rust core over FFI (ADR-335).
R20. (CI-01, CI-02) A macOS CI job builds the Apple and FFI paths when they change and on every
    tag. The team id, the app ids, the associated-domains file and the upload key never enter this
    repository; they are rendered or read from the private deploy rail.
R21. The order holds: economy v10 after #62 and #64; the Telegram retirement after native push
    carries the router; the v1.0.0 cutover is the owner's go (section 8).

## 3. Acceptance criteria

The campaign commits to five rows. Each is proved by the deliveries its last column names: their
own SPECs state the row's criteria, fence their commands and record them red first, and the
campaign closes against this table with each row's evidence. None of them is run by this delivery.
This delivery has no behaviour (it writes documents only), so it states no criterion of its own
and its fence is empty, as SPEC-037's is.

| row | the campaign row | delivered by |
|---|---|---|
| 1.1 | Campaign documents: this SPEC, ADR-335 to ADR-344 and the schematic are on `dev`; ADR-335 and ADR-336 carry the decision outcome their spikes measured; ADR-337 is accepted once the owner-taps ruling is on `dev` | this delivery for the documents; the iOS spike and the browser engine spike for ADR-335's and ADR-336's outcomes |
| 1.2 | Phase 0 measured and recorded. Spikes: the engine on WASM (gzipped size, OPFS persistence, eviction), the engine through FFI on iPhone and iPad (size, cold start, memory), FSRS-7 replay time, the card HTML sandbox, APNs and web push, the 8BitDo remote in both modes, the full-sync choice path, and whether undo deletes the review-log row. Measurements: the collection's and its media's size, and the host's memory and disk with the sync server added | the browser engine spike; the FFI umbrella spike and the Swift harness; the scheduler, undo and full-sync measurements; the card sandbox spike; the push and remote spike; the owner's first device session |
| 1.3 | The sync server runs on the service's own host behind HTTPS, with hashed passwords and a new sync password; an offsite snapshot exists and its restore has been drilled; desktop and AnkiMobile are repointed; the current third-party host stays the rollback until the owner retires it | the sync-server packaging; its security review; the first deploy and the sync cutover, on the owner's go |
| 1.4 | The web client at Phase 1 parity: deck list; review with the card HTML in a sandboxed iframe; undo; templates; media; Web Speech TTS; sync with Anki's full-sync choice, its backups and loss counts; Anki's desktop keys and the Gamepad API for the 8BitDo remote in both modes; installable to the Home Screen; the engine in the browser, or ADR-336's fallback | the browser engine (or its fallback); the web study screens; the web sync screens; the owner's acceptance session |
| 1.5 | The universal iPhone and iPad client at Phase 1 parity on internal TestFlight: deck list; native review screen; card in a WKWebView; undo; media; AVSpeech TTS; sync with the full-sync choice; iPad split layouts; the 8BitDo remote through GameController and UIKeyCommand with the screen kept awake while paired; a build from a SemVer tag on `main` on internal TestFlight with the owner the only tester, after the internal lane has carried every iOS delivery | the engine core; the FFI umbrella crate; the app shell; the review screen; the iPad layouts and remote; the TestFlight pipeline; the milestone build; the owner's acceptance session |

```acceptance
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md` | campaign | added |
| `docs/schematics/app-clients-engine-and-sync.md` | campaign | added: the clients, the engine's two transports, the sync server, the one router and its push transports |
| `docs/decisions/ADR-335-the-iphone-and-ipad-client-is-swiftui-over-the-engine-through-ffi.md` | campaign | added |
| `docs/decisions/ADR-336-the-web-client-runs-the-engine-in-the-browser-on-wasm-with-a-server-side-fallback.md` | campaign | added |
| `docs/decisions/ADR-337-the-owners-own-taps-are-exempt-from-the-never-list-and-every-other-path-stays-bound.md` | campaign | added |
| `docs/decisions/ADR-338-fsrs-7-runs-live-on-one-preset-rebuilt-from-review-history-into-stock-fields.md` | campaign | added |
| `docs/decisions/ADR-339-every-other-experimental-model-only-reorders-cards-already-due.md` | campaign | added |
| `docs/decisions/ADR-340-the-sync-server-moves-to-the-services-own-host-behind-https.md` | campaign | added |
| `docs/decisions/ADR-341-native-push-replaces-the-bot-through-the-one-router.md` | campaign | added |
| `docs/decisions/ADR-342-one-universal-iphone-and-ipad-app-driven-by-touch-keys-or-an-8bitdo-remote.md` | campaign | added |
| `docs/decisions/ADR-343-a-persona-created-in-the-app-speaks-only-after-the-persona-core-gates.md` | campaign | added |
| `docs/decisions/ADR-344-internal-testflight-builds-come-from-a-dispatch-on-dev-numbered-by-its-first-parent-count.md` | campaign | added |
| `CHARTER.md` | campaign | changed: three amendment notes in its last section, for constraints 2, 4 and 14 |
| `docs/PRD.md` | campaign | changed: an amendment note for the third non-goal |
| `docs/DESIGN_SYSTEM.md` | campaign | changed: an amendment note for the first two principles |
| `changelog.d/docs-app-campaign-334.md` | campaign | added |

## 5. What this does NOT do

- It lists nothing on the App Store and opens no external TestFlight testing: the client stays on internal TestFlight with the owner the only tester until the owner's review of a listing (the engine's licence on the store, a name without "Anki", the store's guidelines, account deletion and an honest age rating) is settled (#612).
- It builds no parity beyond Phase 1: browse and search, adding and editing notes, Forget, set due date, in-app import, stats and the forecast screen, deck options and the FSRS-7 preset screen, Apple Pencil and image occlusion are the later parity phases, whose owner taps the owner-taps ruling admits (#611).
- It does not cut over from the predecessor or release v1.0.0; that work keeps its own issues and the owner's go (#61, #62, #63, #64, #164).
- It does not build economy v10's flat multipliers, which wait on side-by-side verification and the v1.0.0 release (#62, #64).
- It does not retire the Mini App or the bot: they stay until native push carries the router, and the cutover runs on them (#612, #63).
- It does not put the day's reading in the clients, which waits on the readings going live (#45).
- It does not build the owner's subscription route for the clients' AI, which waits on its owner gates (#162, #43).
- It ships no RWKV-Instant code, weights or scores pipeline: its licence admits none, written permission is sought first, and the repository carries only the generic due-card scores input (#612).
- It does not build in-app persona creation; ADR-343 records the design a later delivery builds (#51).
- It builds no voice conversation, on-device AI or home-screen widget (#51, #121).

## 6. Risks

- **The engine does not build for wasm32, or builds too large.** The browser engine spike decides
  GO or NO-GO on a measured gzipped size and a measured hot path; on NO-GO, ADR-336's fallback is
  built instead, and row 1.4 still holds.
- **Two Rust static libraries in one app clash.** One umbrella FFI crate links the engine core,
  and later the XP and FSRS-7 crates, as a single static library (ADR-335).
- **Experimental scheduler state breaks another client's sync.** FSRS-7 writes only the stock
  fields, so a stock sync server, desktop and AnkiMobile read nothing new (ADR-338); the full-sync
  measurement runs against a scratch sync server in CI.
- **The browser evicts its storage.** The client asks for persistent storage, syncs at session
  start and end, and warns while reviews are unsynced (R5).
- **An exempt function is reached by something other than the owner's tap.** The containment
  test refuses any non-UI caller (R7, ADR-337).
- **A one-way sync loses reviews.** Loss counts, the on-device backup and the server-snapshot
  check come before every upload or download (R8).
- **The sync move loses data.** The offsite snapshot and its drilled restore come first, and the
  current third-party host stays as the rollback (R9, ADR-340).
- **Card JavaScript escapes its frame.** The sandbox test proves it reaches neither the bridge
  nor the network (R6).
- **A private value reaches the public repository.** Team ids, app ids, the associated-domains
  file and the upload key come from the private deploy rail; the public scrub and the secrets
  scan run before every push (R20).
- **A TestFlight build expires while the owner relies on it.** The minimum-client handshake and a
  re-release check are part of the TestFlight pipeline's delivery.
- **Swift is new to this repository.** A Swift practice pack judges Swift work before the first
  Swift build, and the macOS CI job builds every Apple change (R20).
- **The sync server outgrows DeckStreak's host budget.** It runs as its own unit with its own
  budget entry (ADR-032), sized from a measurement taken before the move, under the memory watch.

## 7. The ordered stretch

Beyond the five committed rows, five more are built in this order if capacity allows, and dropped
from the bottom first.

| row | the stretch row | decided by |
|---|---|---|
| 2.1 | Passkey sign-in on the web and natively through associated domains, a passkeys-only slice of #58 (ADR-132); rows 2.2 and 2.3 need it | ADR-335, ADR-132 |
| 2.2 | The I/O-free XP crate and instant per-review XP on both clients, on the predecessor's math, with the reconciliation test that confirmed XP is never below shown XP | R13 |
| 2.3 | Native push on the one router: APNs and web push transports, opt-in, at most one nudge a day, revocable per device | ADR-341 |
| 2.4 | The isolated FSRS-7 crate and review-history replay in the client; the engine only, since the live-preset switch is a later parity screen | ADR-338 |
| 2.5 | The persona tab and the after-reveal sheet, on the API-key route | R16, ADR-054 |

## 8. The order

- DeckStreak's current backlog keeps its place ahead of the campaign's work.
- Phase 0 comes first: the spikes, the measurements and the sync-server move. ADR-335 and
  ADR-336 take their decision outcomes at the ends of the iOS spike and the browser engine spike.
- Foundations, then the web client, then the iPhone and iPad client; the internal TestFlight lane
  starts right after the Swift harness and carries a build of every later iOS delivery (R17).
- Nothing calls an exempt function before the owner-taps ruling is on `dev` (R7).
- Economy v10 follows #62 and #64.
- The Telegram retirement follows native push carrying the router, and the cutover from the
  predecessor (#63).
- The v1.0.0 cutover is the owner's go (#64, #164).

## 9. Formal models this campaign owes

This delivery touches no code, so it adds no model. Two candidates are recorded here for the
deliveries that build their surfaces:

- **A TLA+ model of the full-sync choice's ordering.** Actors: the owner's tap, a normal sync and
  a second client. The property: an upload or a download writes only after the on-device backup
  exists, and an upload only after the server's offsite snapshot was found, with no normal sync
  between that check and the write. It is built, model first, by the delivery that builds the
  full-sync choice.
- **A Lean entry for XP reconciliation.** For any sequence of reviews and day-level bonuses, the
  XP the server confirms is never below the XP the client showed. It is built by the delivery of
  row 2.2.

## 10. References

- `docs/rulings/OWNER-RULING-2026-10-04-owner-taps.md` and
  `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`: the owner's signed rulings.
- SPEC-001 (the parity campaign whose shape this follows), ADR-301 (the declared write classes
  and the never-list), ADR-058 (the patched engine fork), ADR-054 (the optional AI route),
  ADR-131 and ADR-132 (sign-in), ADR-041 (the one router), ADR-032 (the host budget).
- `docs/schematics/app-clients-engine-and-sync.md`: the components and the data flow.
