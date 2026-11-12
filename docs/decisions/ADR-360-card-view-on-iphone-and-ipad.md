---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The iPhone and iPad card view carries seven named layers from one constructor, and a probe host proves each against planted cards

## Context and Problem Statement

ADR-335 puts a card face in a `WKWebView` with a non-persistent store, no message handler and a
content rule list that blocks the network. #616's harness builds that view and proves two of the
layers through the configuration API; the rule list and the missing handler cannot be observed that
way, and #616 leaves their proof to #619. ADR-352 D1 turns card script off on both platforms.
SEC01-F14 (peer connections from card script) and SEC01-F15 (the card frame's other channels) ask
which channels the three layers leave open: a peer connection, a navigation the card starts, a
window it opens, a file URL it names. This ADR decides the iOS layer set, where it is built, and how
it is proved.

## Decision Drivers

- One constructor, so no screen can build a card view with a layer missing.
- Each layer's job is measured: which channels it alone holds, and which layers are depth.
- The shipped app's transport policy is not relaxed for a test.
- The pure parts are testable on the macOS host with `swift test` and swept by Swift mutants; the
  web-view parts are proved on one iPhone and one iPad simulator.

## Decisions, and the alternatives each was chosen against

### D1. Seven layers

L1 a non-persistent store; L2 page JavaScript off; L3 a compiled rule list blocking `.*` for every
resource type; L4 no script message handler; L5 a navigation gate; L6 a window refusal; L7 no file
access (the card handed over as a string with no base URL, never `loadFileURL`).

Chosen against:

- ADR-335's three layers alone: rejected because a peer connection (not a load), a navigation to a
  `data:` document, a `target=_blank` window and a file URL are channels none of the three is shown
  to close; the suite measures each.
- A frame policy injected into the card on iOS, as the web does: deferred, because L3 already
  blocks loads, and a Swift copy of the web's document builder is a second implementation that
  drifts; a strip in the engine would serve both platforms (#664).

### D2. One public constructor in a host-testable package

`CardWebViewFactory.makeCardWebView(html:)` in `ios/CardIsolation` is the only public way to build a
card view; an `internal` `make(layers:)` builds the probe's reference and single-layer-off views,
reached with `@testable import`. A tree guard refuses any other construction of a `WKWebView` or a
`WKWebViewConfiguration` under `ios/` outside test targets, any script message handler and any
`loadFileURL`.

Chosen against:

- Building the view in the harness app's own target: rejected because the gate and the rule-list
  source could then be tested only on a simulator, and the Swift mutant sweep runs on the host.
- A public `make(layers:)`: rejected because any caller could ship a view with a layer off.

### D3. The navigation gate allows exactly one action, the first main-frame load, and seals

Chosen against:

- Allowing by URL (`about:blank`): rejected because a card can navigate itself to `about:blank` or
  a `data:` document, replacing the card with a document the app did not build.
- Allowing by navigation type: rejected because a `meta refresh` and the app's own string load
  share a type.
- No gate, relying on the rule list: rejected because whether a rule list governs a `data:`
  navigation is not something the design can rely on; the `nav-data` pair measures it either way.

### D4. The rule list is compiled before the view exists, and a failed compile is a refusal

Chosen against:

- Building the view and attaching the list when compilation finishes: rejected because the first
  card would render with no network block.
- A precompiled list shipped in the bundle: rejected because the compiled form belongs to the OS
  that compiles it.

### D5. The probe is loopback listeners in a test-only host app that declares local networking

`CardProbeHost`, an empty app built only for tests, declares local networking in its own Info.plist
so its web views may reach the loopback address; the harness app gains no transport key. The probe
then measures the layers with the transport policy relaxed, which is conservative: in the shipped
app the transport policy would also refuse a plain-HTTP load of the loopback address, so a zero in
the probe host shows the rule list holds without that help. A planted click is the app's own script
activating the element (the stand-in for a tap), and the probe lets script open windows, so that a
zero on `nav-blank` measures L6 and not the popup policy.

Chosen against:

- A custom-scheme handler as the probe: rejected because it adds a channel the shipped view does not
  have, and whether a rule list governs a custom scheme is itself unmeasured, so a zero there would
  say nothing about the network.
- Declaring local networking in the harness app's Info.plist: rejected because it relaxes the
  shipped app's transport policy for a test.
- Reading state with `evaluateJavaScript` alone: rejected because script cannot see whether a load
  left the view; it is used only for markers, text and image widths.

### D6. Each layer's own channels are measured, and depth is declared

A single-layer-off variant per layer shows the channels that layer alone holds. The layers with no
channel of their own under the shipped configuration are declared (expected: L1 and L4, both made
unreachable by L2) and must equal the measured set, so a layer that silently stopped mattering, or
started mattering, is a red.
Measured: the depth is L1, L4, L6 and L7 (the schematic's section 6).

Chosen against:

- Asserting every layer necessary: rejected because with script off, the store and the missing
  handler hold no channel a card can reach, and a claim the suite cannot measure is not evidence.
- Dropping the depth layers: rejected because ADR-335 names them and they hold if script is ever on.

### D7. The card view serves no media, and the rule list keeps no exception

Card media is #632's to bring. #632's design brings it as `data:` URLs inside the document, with a
per-file and a per-face size cap, so this view registers no scheme handler and its rule list keeps
no exception; the planted suite this delivery commits is the one #632 runs again.

Chosen against:

- One scheme handler serving the media directory, as the rule list's one exception: rejected because
  it widens the load layer for every card, and #632 needs no such channel.
- `loadFileURL` with read access to the media directory: rejected because it gives the card file
  reads of everything below that directory and a base URL for relative paths.

### D8. A followed link's one connection is counted for every card and recorded as a residual

A followed link (`nav-self`, `nav-blank`) opens one connection to the link's host with no request
read, and no layer of this delivery holds it. A5 counts the connections from the card view for
every planted card and prints each count: every card must read none except those two, which may
read at most one each, asserted by name with the reading printed. #677 tracks the cure.
Confirmation: A5's count, on the iPhone and the iPad simulator.

Chosen against:

- A5 asserting zero connections for every card now: rejected because it reads red, and no layer
  of this delivery holds the connection, so a new layer is design work outside SPEC-349's
  manifest (#677).
- Leaving A5 blind to connections, as it was: rejected because it hides a measured channel.

## Decision Outcome

The card view is built by one constructor with L1 to L7, its pure parts are tested and swept on the
host, and a probe host on two simulators proves every channel in the schematic's iOS table closed,
with the reach of card script measured in a JavaScript-on variant.

### Consequences

- Good: ADR-335's two unobservable layers are now measured, along with four more.
- Good: no transport exception ships.
- Bad: the project gains a test-only app target and a scheme.
- Bad: preconnect may be a residual a rule list does not govern; the suite measures it and this
  ADR's amendment records it.

### Confirmation

SPEC-349's A1 to A9, on the macOS CI runner's host and its iPhone and iPad simulators.

## What would make this wrong

- The rule list does not block a load the reference view makes: L3 is not the network block
  ADR-335 assumed, and the view does not ship until a layer closes it.
- `make(layers:)` is reachable from a Release build.
- A planted card reaches a probe from the factory's view on either simulator.

## More Information

- ADR-335; ADR-352; SPEC-334 R6; SPEC-339 (#616's Swift harness) for the seam.
- SEC01-F14 and SEC01-F15 are cited by id.

## Amendment: each card load wait starts when the view's page first answers (SPEC-349 section 7), and the alternatives each decision was chosen against

In attempts on pull requests that change no card view path, the planted suite read a card not
loaded in the first simulator session and passed in the second (jobs 125417948615, 125379986494).
Each card view miss followed a start of that view's own WebContent process longer than the whole
10-second load wait: 21.8 seconds (job 125417948615); 11.8, 25.1 and 19.6 seconds (job 125379986494).
D1 to D8 stand, and ADR-372 D11's warm-up and keeper stand.

### D9. Each card load wait starts when the view's page first answers, under the warm-up's bound

The load wait first asks the page a script of the app's own, again whenever an ask comes back
without its answer, one ask at a time, under `Probe.warmUpSeconds`; once the page has answered, it
polls whether the card has loaded under the unchanged `Probe.loadSeconds`. The answer comes from
the view's own page process, so the 10-second wait starts once that process runs, every time, and
judges the card's own load, as ADR-372 D11 meant it to. The wait takes the page as a function
that asks it a script, so a stand-in page whose process starts late can be handed to it, and each
load prints its page's answer time.

Chosen against:

- Raising `Probe.loadSeconds`: rejected because it lengthens every wait that runs to its bound, a card that never loads included, so a slower load would pass (ADR-372 D11).
- A pause before each view: rejected because WebKit does not promise to start the process during it, so it buys nothing certain (ADR-372 D11).
- WebKit's navigation commit as the signal: rejected because the card view's navigation delegate is the factory's gate, and a test that replaces it removes a layer under test.
- A warm of the probe host in the planted step before its first test, as the Harness test steps have: rejected because the warm-up loaded in 3.37 and 5.58 seconds at the first session's start (job 125417948615) and the long starts came all through that session.
- The keeper held across the planted tests: rejected because a GPU process exit preceded a start over 10 seconds no more often than the other starts (8 of 350 against 3 of 97, job 125417948615).
- A retry of the test, the suite or the destination: rejected because it passes a red without curing it.

### Consequences of this amendment

- Good, because a card view's load wait now judges the card, not the start of its process, on both simulators.
- Good, because each load prints its page's answer time, so a session's slow starts show on every run, red or green.
- Bad, because a page whose process never starts holds its test for 60 seconds before it fails, not 10.
- Bad, because the first session's slow process starts are measured, not cured.

### What would make this amendment wrong

- A card view still reads not loaded after its page answered: the load itself is then slow, and its cause needs its own design (#841).
- The printed answer times reach 60 seconds: the start then needs its own measurement (#841).
- SPEC-339's A6 reads red again in one session: the flow tests then need their own design, with the app's console kept (#796).

## Amendment: a card handed over at a URL waits from its own document's first answer (SPEC-349 section 10), and the alternatives it was chosen against

In one attempt on a pull request that changes no card view path, the factory suite read the card
of its `without L7` row, the one row that hands its card over with an https base URL, not loaded
in the first simulator session and passed in the second (job 125835201348). D9's wait had
started: the row's page answered 1.63 seconds into its wait (job 125835201348), and the card
had not loaded when the load wait that followed the answer ended. D1 to D9 stand.

### D10. A card handed over at a URL waits from its own document's first answer

The wait asks the page a script of the app's own that answers only from a document whose
address is the URL the card was handed at; the view's empty first document, which answers as
soon as the page process runs, answers that it is still waiting. Once the handed document has
answered, under the unchanged `Probe.warmUpSeconds`, the wait polls whether the card has loaded
under the unchanged `Probe.loadSeconds`. The answer is read by the page's own script, so the
factory's gate stays the view's navigation delegate. Only the factory suite's base-URL row
takes this wait; D9's wait for every other card is unchanged.

Chosen against:

- D9's wait for that row, kept as it is: rejected because the empty first document answers before the handed card commits, so the load wait counted the commit, which ran past it in the red session (job 125835201348).
- WebKit's navigation commit read by a navigation delegate: rejected because the card view's navigation delegate is the factory's gate, and a test that replaces it removes a layer under test (D9).
- A third warm-up load, a card handed over at an https URL, before any measured wait: rejected because the base-URL row held the first such load of its session (job 125835201348), so whether a warm-up takes the first session's slow commit from it is unmeasured, and the row still paid 0.12 and 0.13 seconds after its answer in second sessions (jobs 125835201348, 125641992874).
- Raising `Probe.loadSeconds`: rejected because it lengthens every wait that runs to its bound, a card that never loads included (D9).
- Handing that row's card over with no base URL, or at another scheme: rejected because the row plants a caller that hands the card over at a web origin, and changing it removes that planted case.
- A retry of the test, the suite or the destination: rejected because it passes a red without curing it (D9).

### Consequences of this amendment

- Good, because the base-URL row judges its card's load from the handed document, on both simulators.
- Good, because the row still prints its answer time, now the handed document's, so a slow commit shows on every run, red or green.
- Bad, because a handed document that never commits holds its row for the answer bound before it fails, not the load wait.
- Bad, because the first session's slow commits are measured, not cured.

### What would make this amendment wrong

- The base-URL row reads not loaded after its handed document answered: the load itself is then slow, and its cause needs its own design (#841).
- Its printed answer times reach the answer bound: the commit then needs its own measurement (#841).
- The planted suite's case that hands a card over at a URL reads not loaded after its page answered: that case then takes this wait too (#841).
