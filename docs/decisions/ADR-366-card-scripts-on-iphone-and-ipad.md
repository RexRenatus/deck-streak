---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Card scripts run on iPhone and iPad behind one switch that needs every control, with peer connections removed in every frame and every connection held at the app's own proxy

## Context and Problem Statement

ADR-352 D1 turned card scripts off on both platforms, because a peer connection is outside every
network control either platform enforces on a card, and named what would make it wrong: an owner
ruling that scripted cards are needed. The owner has ruled (#651): card scripts go on, behind
controls, by the route that weakens nothing, with no residual signed; scripts stay off until the
delivery lands. ADR-360 built the iPhone and iPad card view with seven layers and measured that
page JavaScript off (L2) alone holds four channels, one of them a peer connection that reached the
UDP listener with JavaScript on. #677 measured that a followed link opens one connection before
the gate refuses it. This ADR decides how scripts turn on for iPhone and iPad without reopening
any of those channels. It is carried by SPEC-355.

## Decision Drivers

- Every channel SPEC-349 refuses today stays refused, with its test (#651's non-weakening rule).
- A control the engine does not enforce is a comment; one that cannot be observed is a promise.
- Scripts must never run because a control silently failed to install.
- A proof observes arrivals at listeners the test owns, with a reference beside every absence
  that is itself proved not blind (SPEC-349's note on the simulator's ICE agent).
- The scripts-off view stays exactly as proved, for every case in which a control is missing.

## Decisions, and the alternatives each was chosen against

### D1. One switch, one pure decision, read from the view the factory built

`CardScripts.switchedOn` is the only switch. `decide(switchedOn:present:)` returns `run` only when
it is on and L1, L3 to L9 are all present; `present` is read back from the built configuration,
never from what the factory meant to do. Page JavaScript (L2) is set from the verdict.

Chosen against:

- A compile-time flag: rejected because one build holds one value, so no test can prove the
  scripts-off branch of the build that ships.
- A per-deck or per-card choice offered to the learner: rejected because a learner cannot judge a
  shared deck's script, and every scripted card would still need every control.
- Scripts on whenever the switch is on: rejected because a control can fail at run time (the
  hold's listener not ready), and scripts would then run without it.
- `present` taken from the factory's intent: rejected because a read-back is what proves an
  installed layer; an intention passes when the install was skipped.

### D2. L8: a document-start user script in every frame, in the page's world, deletes every peer-connection global

It deletes every global whose name matches `^(webkit)?RTC`, and `WebTransport`, before any card
script runs; a card can then construct no peer connection and open no transport outside the load
path.

Chosen against:

- The engine's private peer-connection preference: rejected because a private interface is
  refused at review and can change without notice.
- Lockdown Mode for the card's web page: rejected because an app that is not a browser cannot set
  it on a web view; the engine refuses the change.
- The CSP `webrtc 'block'` directive: rejected because the engine does not implement it (ADR-352
  D1's reason, unchanged).
- A network control alone (the rule list, or the hold of D3): rejected because a peer connection
  is not a load and its UDP traffic does not pass through an HTTP CONNECT proxy.
- The same script in an isolated content world: rejected because a deletion there removes
  nothing from the page's own globals.
- Main frame only: rejected because a frame the card makes has its own globals; every frame is
  injected, and A8 measures each nested frame a card can make.

### D3. L9: the card view's store sends every connection to a loopback hold the app owns, which refuses it (#677)

One proxy configuration, an HTTP CONNECT proxy at the app's `ConnectionHold` listener, failover
off, no excluded domain. The hold answers no byte and closes each connection. A link the card
follows, a script's navigation and a preconnect all reach only the hold; the engine resolves no
name for a connection it hands to a proxy.

Chosen against:

- A change to the navigation gate: rejected because the gate already refuses; #677's connection
  opens before the gate is asked, so no decision can withdraw it.
- Stripping links from the card (#664): rejected as the cure because a card script adds links at
  run time; the strip stays a separate depth layer.
- A proxy address with no listener: rejected because another process could bind that port and
  read the names the card connects to.
- Failover allowed: rejected because a refused proxy would fall back to a direct connection and
  reopen #677.
- A per-domain exclusion for the app's own hosts: rejected because the card view loads nothing
  from any host; an exclusion would be the one route out.

### D4. The second origin is the opaque origin of a string-loaded card, in its own store, with no bridge

No second host is served on iPhone and iPad. A card handed over as a string with no base URL has
an opaque origin; its non-persistent store is not the default store; it has no message handler.
A planted card that tries to read the app's state proves it (SPEC-355 R6).

Chosen against:

- A custom URL scheme serving the card: rejected because the scheme handler is a channel back
  into Swift, and the card would gain a stable origin with storage.
- A second web host loaded over the network: rejected because it needs an exception in L3 and L9
  for that host, which is a route out.

### D5. Script dialogs, media capture and motion requests are refused by the window refusal (L6)

Chosen against:

- The engine's default for a delegate that does not implement them: rejected because the default
  is implicit, can change with the SDK, and no test names it.

### D6. The proof is a scripted variant of the planted suite whose every reference is proved not blind

Each scripted card has a reference, scripts on with its own control off, that must reach its
listener on both simulators; the scripted view must reach nothing, and the card's marker must show
its script ran. The peer connection is read twice, by STUN to the UDP listener and by TURN over
TCP to the TCP listener, so the ICE agent skipping the loopback interface reads red. A lookup is
read by a multicast DNS witness counting queries for a `.local` name the card asks for.

Chosen against:

- Reading the configuration only: rejected for ADR-352 D7's reason; it proves no behaviour.
- Reusing SPEC-349's one measured datagram: rejected because it measured the main frame only, and
  a nested frame is the case D2 must survive.
- Leaving `dns-prefetch` UNOBSERVABLE: rejected because a card script can write data into a name;
  an unobserved channel is then a residual nobody signed.

### D7. One delivery for iPhone and iPad, #677 closed in its first green step

Chosen against:

- #677 as its own delivery first: rejected by default because the hold is the control the switch
  needs and its proof is the same suite; the seat may rule the split.
- One delivery with the web: rejected for ADR-352 D8's reasons (different runners, proof media and
  cut points), and because the web needs owner acts this half does not.

## Decision Outcome

Card scripts run in the iPhone and iPad card view only when the switch is on and the factory reads
back every control from the view it built; peer connections are removed in every frame, every
connection the card starts is held at the app's own refusing proxy, and the suite proves each
control with a reference that is not blind. Any missing control yields today's view.

### Consequences

- Good: scripted cards work on iPhone and iPad; #677 closes; the suite's DNS blind spot closes.
- Good: the scripts-off view keeps its own proof, so a regression reads red there first.
- Bad: a card script that needs a host library or a peer connection still does not work.
- Bad: L8 is a script, not an engine setting; its proof per frame is what carries it.
- Neutral: the app owns one loopback listener while a card view exists.

### Confirmation

SPEC-355's A1 to A12, in the `card-isolation` and `harness` jobs on every pull request that
touches `ios/`.

## What would make this wrong

- A planted card in any frame reaches the UDP or TCP listener through a peer connection from the
  scripted view: D2 does not hold, and the switch does not ship on.
- The engine connects around the proxy (for loopback or any destination), or resolves a name for
  a proxied connection: D3 does not hold as stated.
- The engine gains a public per-view peer-connection setting: D2 should move to it.
- The witness cannot see a lookup the reference makes: D6's DNS reading is blind, and the seat
  rules before scripts go on.

## More Information

- ADR-352 (D1 amended for iPhone and iPad), ADR-360 (L1 to L7), SPEC-349, #651, #677, #664.
- SEC-01 findings by id: SEC01-F14, SEC01-F15.
