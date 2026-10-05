---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The iPhone and iPad client is a SwiftUI app over the engine through FFI

## Context and Problem Statement

The owner chose a native study client for iPhone and iPad that is a real Anki client (SPEC-334
R2, R4; the owner's answer IOS-01). It has to run Anki's engine on the device, so it can study
offline and sync like any Anki client, and it shares that engine with the web client (ADR-336).
It needs a native review screen, the platform's input, Keychain and push, and card faces that
render Anki's own HTML templates. DeckStreak's house stack is Rust with a SvelteKit front end, and
its mobile shell would be Tauri 2. How is the iPhone and iPad client built, how does it reach the
engine, and how do its builds reach the owner's devices?

## Decision Drivers

- The engine is Rust and stays one engine on every surface: no second implementation of the
  scheduler or the collection (CHARTER 1's parity rule, applied to Anki's behaviour).
- A native review screen with the platform's input: the GameController framework and UIKeyCommand
  for an 8BitDo remote (ADR-342), haptics, AVSpeech text to speech.
- Credentials sit in the Keychain, and sign-in pins to the one owner (CHARTER 14 as the
  app-surfaces ruling amends it).
- Card faces are HTML from Anki's templates, and card JavaScript must reach neither the bridge nor
  the network (SPEC-334 R6).
- Builds reach the owner's devices without a deploy, and nothing private enters the public tree.

## Considered Options (the alternatives it was chosen against)

- A SwiftUI app over the engine through FFI — chosen because the owner chose it (IOS-01), and it gives a native review screen, GameController, UIKeyCommand, Keychain and APNs with no bridge layer between the screen and the platform, while the engine stays the shared Rust crate.
- Tauri 2, the house stack's mobile shell — rejected because the owner declined it at IOS-01, and its review screen is a web view end to end, so remote input, Keychain and APNs each cross a plugin and the invoke bridge.
- Capacitor — rejected because it also puts the whole client in a web view behind a JavaScript bridge, and the engine would still need a native plugin, so it adds a layer without removing the FFI.
- The web client alone, installed to the Home Screen — rejected because the owner asked for a native iPhone and iPad app, and the browser's storage can be evicted, which the native client's own files avoid; the web client is built as well (ADR-336).

## Decision Outcome

Proposed option: a SwiftUI app over the engine through FFI. **The decision outcome comes from the
iOS spike.** The spike's Rust half builds one umbrella FFI crate as an arm64 XCFramework for
device and simulator in CI; its Swift half is a harness that opens a synthetic collection, lists
its decks, renders a card and answers it on the iPhone and iPad simulators, and records the app's
size, cold start and memory. When both halves pass, the delivery that records them sets this ADR
to `accepted` with the measurements; if either fails, this ADR is rejected and the owner chooses
again among the options above.

**The spike's outcome.** Both halves passed. The Rust half built the XCFramework in CI (SPEC-336,
section 7). In `xcframework` run 37233368979, the Swift half opened the synthetic collection on the
iPhone and iPad simulators, listed its decks, rendered the queued card in the isolated web view and
answered it Good (SPEC-339, A4 to A8, on both simulators), and each of its codec's 23 mutants was
killed. SPEC-339 section 7 holds that run's figures:

- the Release simulator app is 35286275 bytes, of which 35145000 are the executable, with the
  engine linked and dead-stripped;
- the median cold start to the first responsive frame is 3.47 s on the iPhone simulator and
  10.57 s on the iPad simulator;
- the median from the harness's first line to the deck names shown is 1.96 s and 3.03 s;
- the largest peak physical memory over open, list, render and answer is 58251.9 kB and
  70244.9 kB.

These are a simulator's figures, not a device's, and the iPad simulator's passes spread widely:
the run before read 4.00 s to its first frame. None of them shows the engine too large, too slow
to start or too heavy in memory for the spike to stop on.

Proposal: accept this option. What could still make it wrong is a device's size, cold start and
memory, which no simulator measures. This records the outcome only; the status is the
decision-makers' to set.

- **The engine through FFI.** One umbrella crate exposes an allow-listed `run(service, method,
  bytes)` over the engine's protobuf backend, through UniFFI at a pinned version. It is the app's
  one Rust static library, because two Rust static libraries in one app clash; the engine core,
  then the XP crate and the FSRS-7 crate, link into it. The allow-list is the engine core's
  (ADR-337).
- **Card faces.** A card renders in a WKWebView with a non-persistent store, no message handler on
  the card's frame and a content rule list that blocks the network, inside the native review
  screen.
- **Sign-in, and CHARTER 14 as the app-surfaces ruling amends it.** Sign-in on iPhone and iPad
  pins to the one owner account by ADR-131's and ADR-132's methods, with passkeys through
  associated domains. The native sync login and the bearer token sit in the Keychain, and calls
  to the service go through the Rust core over FFI. No surface opens a session before its gate is
  built, and the Telegram `initData` gate stays while the Mini App and the bot exist. This ADR
  carries the amendment of CHARTER 14 recorded in
  `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`.
- **Builds.** A macOS CI job builds the Apple and FFI paths when they change and on every tag. The
  team id, the app ids and the associated-domains file are rendered from the private deploy rail
  and never enter this repository.
- **A TestFlight upload is artifact publication, not a deploy.** `RELEASING.md` (lines 3 to 5)
  says that nothing deploys from a branch or from CI, and CHARTER 3 governs the binaries a host
  runs. An upload to TestFlight publishes a signed build to the owner's own devices; it changes no
  host and serves no one else, so CI may perform it, from `dev` as from a tag on `main`.
- **Dev builds come from a manual dispatch on `dev`.** They install under a separate dev app id
  and sync to the staging sync user; real data reaches a device only from a SemVer tag on `main`
  (SPEC-334 R17). ADR-344 decides the dispatch and the build number.

### Consequences

- Good, because the review screen, the remote, haptics and speech are the platform's own, with no
  bridge between the screen and them.
- Good, because the engine stays one Rust crate shared with the web client, so the scheduler and
  the collection behave as Anki's do on every surface.
- Bad, because DeckStreak gains a second UI codebase, in Swift, a language new to this
  repository; a Swift practice pack judges Swift work before the first Swift build.
- Bad, because every Apple build needs a macOS CI runner, which costs more per minute than the
  Linux jobs.
- Bad, because the house stack's mobile shell is set aside for this client, so its patterns do
  not carry over.

### Confirmation

- The iOS spike's record: the XCFramework built in CI, and the harness's open, list, render and
  answer on both simulators with the size, cold start and memory it measured.
- The engine core's containment test (ADR-337) runs in Rust, under the gate, for every caller the
  FFI exposes.
- The card-sandbox test proves that card JavaScript reaches neither the bridge nor the network.

## What would make this wrong

- The engine through FFI is too large, too slow to start or too heavy in memory on the device,
  which the spike measures.
- UniFFI cannot generate a usable module for the umbrella crate, which the spike's Rust half
  checks before any Swift is written.

## More Information

- SPEC-334 (rows 1.1, 1.2 and 1.5; R4, R6, R17, R19, R20).
- `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md` (CHARTER 14).
- ADR-336 (the browser's transport), ADR-337 (the allow-list and the owner's gestures), ADR-342
  (universal layouts and the remote), ADR-344 (internal builds), ADR-131 and ADR-132 (sign-in).
