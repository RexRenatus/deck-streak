---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A card face runs no script, and each platform closes every other channel with a named layer that a planted card proves

## Context and Problem Statement

SPEC-334 R6 (UX-05, RND-01) puts a card face, HTML a shared deck's author wrote, in a sandboxed,
no-network frame on the web and in an isolated web view on iPhone and iPad, and asks for a test
that proves card JavaScript reaches neither the app's bridge nor the network. ADR-335 names three
layers for the iOS view. The security review's three card-frame findings, SEC01-F13 (the card
frame's navigation), SEC01-F14 (peer connections from card script) and SEC01-F15 (the card frame's
other channels), say the layers named so far leave channels open or unnamed. This ADR decides what
runs in a card, what each platform's layers are, how the proof is made, and how the work is cut.
It is carried by the web delivery (SPEC-341); the iPhone and iPad delivery's own ADR cites it.

## Decision Drivers

- A card's markup is untrusted input from a shared deck, and a learner opens decks from strangers.
- Every channel a card can open is named, with the layer that closes it and what that layer does
  NOT stop (the schematic's tables). A channel nobody named is open by default.
- The page policy stays as strict as `csp.test.ts`'s `unsafe()` judgement demands: no unsafe
  source enters `script-src`.
- A proof observes arrivals at a listener the test owns, with a positive control beside every
  absence, because an absence-only test passes when the probe is blind.
- No Swift is compiled outside CI; the iOS half's proof is read from its macOS CI run.

## Decisions, and the alternatives each was chosen against

### D1. No card script runs, on either platform

The web frame carries no sandbox token, so no script runs in it; the iOS view sets
`allowsContentJavaScript` to false. A peer connection is outside every network control either
platform enforces on a card (a CSP directive or a content rule list governs loads, and a peer
connection is not a load), and a STUN or TURN request can carry data. With no card script, no card
code can open one. The planted suite measures the reach a card script would have (the scripts-on
variant), so the cost of turning scripts on later is a number, not a guess.

Chosen against:

- Scripts on, the peer connection accepted as a residual the owner signs: rejected until the owner
  rules on it, because the residual is a data channel from every shared deck to any host.
- Scripts on, with the `webrtc 'block'` directive and a nested-frame check: rejected because the
  design cannot count on both target engines enforcing it, and a policy an engine ignores is a
  comment; the suite would have to prove it per engine, and per nested frame, before it could
  carry the risk.
- Scripts on in the `srcdoc` frame, by adding `'unsafe-inline'` to the page policy: rejected
  because a `srcdoc` document inherits the page's policy, so the page itself would admit inline
  script, which `csp.test.ts`'s judgement refuses.
- Scripts on in a URL-loaded frame from a second origin: rejected for this spike because it needs
  a second served origin, an exception to the edge's `frame-ancestors` and still the peer-connection
  control above.

Cost: a card that needs script (a hint toggle, MathJax, a typed-answer helper) renders without it.
The study screens (#630, #632) show such a card as markup.

### D2. The web card frame is an `iframe` with an empty `sandbox` attribute and the card in `srcdoc`

Chosen against:

- A URL-loaded frame from the app's own origin: rejected because the edge's `frame-ancestors`
  admits only the Telegram web client, so the app cannot frame itself, and a framing exception
  would widen what can frame the app.
- A `data:` URL frame: rejected because it needs `frame-src data:` in the page policy, which
  reopens the frame navigation D5 closes.
- A `blob:` URL frame: rejected for the same reason (`frame-src blob:`), and a blob URL outlives
  the card unless revoked.
- Rendering the card into the page's own DOM through a sanitizer: rejected because the card would
  then share the page's origin, storage and session, and one sanitizer miss is a full compromise.

### D3. The web frame carries its own policy, a meta element placed first in its head

`default-src 'none'; img-src data:; media-src data:; font-src data:; style-src 'unsafe-inline';
form-action 'none'; base-uri 'none'`. The inherited page policy still applies on top of it.

Chosen against:

- The inherited page policy alone: rejected because the page sets no `default-src` and must not
  (the app loads its own images, fonts and API), so a card's `img`, `font` or `media` would reach
  any host.
- The `csp` attribute on the `iframe` (embedded enforcement): rejected because one of the two
  target engines does not enforce it.
- A policy header on a URL-loaded frame: rejected with D2's URL frames.

### D4. The web frame document strips the card's `link`, `meta`, `base` and `template` elements, and refuses a card whose re-parse brings one back

Chosen against:

- Leaving them to the frame policy: rejected because `link rel=preconnect` and
  `link rel=dns-prefetch` are not fetches, so no policy governs them; a card's `meta refresh` is a
  navigation; and a card's `base` changes where every relative URL in it points.
- A general HTML sanitizer library: rejected because its job is removing script, which D1 already
  holds, its allow-list would strip markup cards legitimately use, and it is a dependency to keep
  current for no channel the strip does not close.

### D5. The page policy sets `frame-src 'none'` (SEC01-F13)

A frame's own navigation, including one the frame starts itself, is checked against the embedding
page's `frame-src`. A `srcdoc` document is not fetched, so the card frame itself still renders.

Chosen against:

- `frame-src` naming the card frame's source: rejected because a `srcdoc` frame has no source to
  name.
- `navigate-to` in the frame's own policy: rejected because no engine ships it.
- The sandbox alone: rejected because a sandbox with no token still lets the frame navigate itself.
- Watching the frame's navigation from the page: rejected because the page cannot observe a
  cross-origin frame's navigation before it happens.

### D6. The page has no window message listener; one added later checks `event.source`

Chosen against:

- Checking `event.origin`: rejected because every opaque-origin frame posts with origin `"null"`,
  so the origin identifies no frame; the planted scripts-on measurement shows it.
- Handing the card a `MessageChannel` port: rejected because with D1 there is no card code to hold
  one.

### D7. Each platform's layer set is named, and its proof is a planted suite with reference controls

The web layers are W1 (the sandbox), W2 (the frame policy), W3 (the strip) and W4 (the page's
`frame-src`); the iOS layers L1 to L7 are the iPhone and iPad delivery's ADR's. For every channel
in the schematic, the suite opens one planted card in a reference frame with every layer off,
where it must reach a listener the test owns, and in the shipped frame, where it must reach
nothing. A single-layer-off variant per layer shows which channels that layer alone holds.
Listeners count real arrivals: requests, TCP connections and UDP datagrams. Engines: Chromium and
WebKit on the web; one iPhone and one iPad simulator on iOS.

Chosen against:

- Asserting the configuration only (attributes, preferences): rejected because a configuration
  test cannot see a rule list or a missing handler, and proves no behaviour; #616's own design says
  so and leaves this proof here.
- Counting the browser's request events: rejected because an engine may report a request a policy
  then blocks, and a request event is not an arrival.
- Treatment-only absence tests: rejected because a listener that went deaf passes them.

### D8. Two deliveries: the web now, iOS after the Swift harness

Chosen against:

- One delivery: rejected because every iOS file sits on #616's harness, which is cut only after
  #645 lands; the two halves run on different runners with different proof media (Playwright on
  Linux, XCTest on macOS); and one delivery's population freezes at its first round, so the web half
  would wait on the slowest pipe while #648 and the web study screens (#630) need its component now.

## Decision Outcome

Card scripts are off on both platforms; the web frame is a `srcdoc` iframe with an empty sandbox,
its own policy and the strip, under a page policy with `frame-src 'none'`; the iOS view is the
iPhone and iPad delivery's ADR's; and a planted suite on each platform proves every named channel
closed, with the reach of card script measured, not assumed.

### Consequences

- Good: every channel a card can open is named with its closing layer, and CI fails when one opens.
- Good: the page policy gains one directive and loses nothing; the app frames nothing by URL today.
- Bad: scripted cards lose their script until D1 is revisited.
- Bad: a card's media must reach the frame as `data:` until the study screens decide a media path;
  any media path added later re-runs the planted suite.
- Neutral: #648's harness and its WEB-1 test must compose with `frame-src` (SPEC-341 R13).

### Confirmation

`card-sandbox` (web) and the `CardProbe` tests (iOS) run on every pull request that touches them;
their acceptance criteria are SPEC-341's A1 to A13 and the iPhone and iPad delivery's SPEC's.

## What would make this wrong

- An engine refuses to render a `srcdoc` frame under `frame-src 'none'` (the render-proof card
  would show it); D5 would then need a different mechanism.
- Both target engines enforce a peer-connection control a card frame can carry; D1's cost could
  then be reconsidered.
- The owner rules that scripted cards are needed at Phase 1 parity; D1 then needs the second-origin
  frame and a signed residual or control.
- A planted card reaches a listener from the shipped frame in any engine: the layer table is wrong
  and the frame does not ship until it is right.

## More Information

- SPEC-334 R6 and row 1.2; ADR-335 (card faces in a web view); ADR-336 (the engine in the browser).
- The schematic `docs/schematics/card-frame-channels.md` holds the channel tables this ADR's D7
  runs.
- The SEC-01 findings are cited by id: SEC01-F13, SEC01-F14 and SEC01-F15.

## Amendments by ADR-412

ADR-412 adds Firefox to D7's web engines: the planted suite runs in Chromium, WebKit and
Firefox, in the same `card-sandbox` job. D7's decision and its alternatives are otherwise
unchanged; ADR-412's D2 and D3 record what adding the engine was chosen against.
