---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Card scripts on iPhone and iPad are contained by refused navigations, content-blocking rules and a refused link activation, proved by the planted suite before the switch turns on

## Context and Problem Statement

ADR-366 put card scripts on iPhone and iPad behind one switch that needs every control, read back
from the built view. The switch defaults off on iOS pending a measured containment layer. This
ADR decides that layer: which parts it has, where each lives, when each is installed, what
happens when one is missing, how the existing planted suite proves it, and the order in which
the build turns the switch on. It is carried by SPEC-361.

Two platform facts shape it, both from primary sources. A content-blocking rule list reaches
every load the engine makes for a card, and a link hint as a `ping`, but not the early connection
WebKit opens when a link is activated: `HTMLAnchorElement::handleClick` asks the navigation
delegate and then calls `preconnectTo`, which no rule list, no Content Security Policy and no
delegate consults, and whose only per-view switch is a private interface. And the navigation
delegate is asked before any content loads, but a refusal there does not withdraw that
connection. A link activation must therefore be refused before `handleClick` runs.

## Decision Drivers

- Every channel SPEC-349 and SPEC-355 refuse stays refused, with its test (#651's non-weakening
  rule).
- A control the engine does not enforce is a comment; one that cannot be observed is a promise.
- Scripts never run because a control silently failed to install.
- A public interface only: a private one is refused at review and changes without notice.
- The switch turns on only after the measurement holds on both simulators in CI.

## Decisions, and the alternatives each was chosen against

### D1. The layer is refused navigations plus content-blocking rules, with the link activation refused before the engine follows it

- Chosen: L3 (the compiled rule list, one rule blocking every URL) and L5 (the navigation gate)
  stay as ADR-360 built them, and four parts join them: L10 refuses a link's activation, L11
  guards the page's activation and rewrite entry points, L12 is the document's own policy, and
  L13 turns the link preview off.
- The app's own proxy: rejected: not measured to hold every connection.
- The engine's private network-host allow-list: rejected because it is a private interface.
- A second compiled rule list: rejected because it runs on the same engine path as L3, so it adds
  no independent hold.
- Lockdown Mode for the card view: rejected for this delivery because what it turns off for link
  activation is not documented; the seat may rule a probe of it.

### D2. L10 cancels a link's activation from a content world the app owns

One user script, at document start, in every frame, in a `WKContentWorld` the app owns, listens on
the frame's window for `click` and `auxclick` in the capture phase. It cancels the default action
when the composed path holds a link (`a` or `area` with `href`, or an SVG `a`), or when the target
is an element that can host a shadow root, so a link inside a closed shadow root is refused
through its host. Default actions are what follow links; a card's own listeners still run.

- Chosen: an app-owned content world, the capture phase on the window, cancel the default only.
- The same listener in the page's own world: rejected because a card script shares that world and
  can replace the intrinsics the listener calls before any click.
- The navigation delegate cancelling the action: rejected because the delegate is asked before the
  early connection and its refusal does not withdraw it.
- Cancelling only links seen on the composed path: rejected because a closed shadow root hides its
  link from every outside listener, and its click is retargeted to the host.
- Cancelling every click in the document: rejected because it breaks every native control a card
  may use (a checkbox, a details toggle) when the host rule already reaches every hidden link.
- Stripping links from the card (#664): rejected as the cure because a card script adds links at
  run time; the strip stays a separate depth layer.
- Stopping propagation as well: rejected because a card's own click listeners are its feature, and
  the default action is the only part that reaches the network.

### D3. L11 refuses activation of a detached node and every document rewrite, in the page's world

A detached element's click reaches no window, so L10 never sees it, and `handleClick` still runs
for an `a` that is not connected. `document.open()` removes every event listener of the document,
its window and its nodes in every world, L10's included. L11 closes both from inside the page's
world, where those entry points live: it wraps `HTMLElement.prototype.click` and
`EventTarget.prototype.dispatchEvent` to refuse a target that is not connected, and wraps
`Document.prototype.open`, `write` and `writeln` to refuse always; every wrapper is non-writable,
non-configurable, and calls only intrinsics captured at install.

- Chosen: page world, document start, every frame, beside L8; refuse by throwing
  `NotAllowedError`, so a card's script sees the refusal.
- Letting `document.open` run and re-installing L10 after it: rejected because a page-world script
  cannot add a listener in the app's world, so L10 could not be restored.
- Allowing `document.write` while the parser runs: rejected because a script cannot tell a
  parser-inserted call from one that opens the document, and refusing both is the one rule a
  test can hold.
- Refusing every `click()` and `dispatchEvent`: rejected because a connected element's activation
  is L10's to judge, and scripted cards use both.
- Watching for new links with a mutation observer: rejected because it acts after the fact, when a
  click may already have run.

### D4. L12 is a policy the factory places first in the card's document

The factory hands the view `DocumentPolicy.prefix` and then the card: a `Content-Security-Policy`
`meta` admitting `data:` images, media and fonts, inline style and inline script, and nothing
else (`default-src 'none'`, `object-src 'none'`, `form-action 'none'`, `base-uri 'none'`). A policy
a card adds can only narrow it. It holds every load and form a second time, on a check separate
from the rule list; it does not hold the two link hints, which L3 holds alone.

- Chosen: a `meta` policy in CardIsolation, prefixed by the factory, read back from the string
  handed, removable by a probe variant.
- No policy, the rule list alone: rejected because one control would then hold every load, and no
  variant could show a second hold.
- A policy header from a custom URL scheme handler: rejected because ADR-359 and SPEC-348 R18 keep
  the card view free of scheme handlers, a channel back into Swift.
- The policy composed into the face document in the shared core: rejected because the planted
  suite builds its own documents, so it would never see the policy, and no variant could remove it.
- Admitting `'unsafe-eval'`: rejected because a card's ordinary script needs no evaluation of
  strings; a card that does is #651's.

### D5. L13 turns the link preview off, and L6 answers the context menu with nothing that opens a link

- Chosen: `allowsLinkPreview = false` on the card view, read back; `WindowRefusal` gains a
  context-menu arm that offers no preview and no open.
- The defaults: rejected because a long-press preview or a menu's open loads the link outside the
  card's document, where no layer above reaches.
- Overriding the view's gestures: rejected because it reaches into private view internals.

### D6. Every card view gets every layer; the verdict sets page JavaScript alone

- Chosen: one configuration for the scripts-off view and the scripted view, so L10 to L13 hold
  #677's channel in both, and the read-back judges both.
- L10 to L13 in the scripted view only: rejected because the scripts-off view would keep #677's
  channel open.

### D7. The proof is the existing planted suite, widened and never loosened

Each new channel has a scripted card whose reference, its own controls removed, must reach its
listener on both simulators, and whose scripted view must reach nothing. Every planted card is
also shown in the scripted view, with its click. A new variant removes one layer from the
scripts-off view, so the planted control for #677's channel is that view without L10. A
`permitted` card proves the rules leave the card's own `data:` loads. Declared sets are
predictions; a contradicting measurement is a STOP for the seat.

- Chosen: the existing listeners, variants, controls and counts, plus new cards, one new variant
  and one positive control; a person's reading on a device for a real tap and long press (D1).
- Reading the configuration only: rejected for ADR-352 D7's reason: it proves no behaviour.
- A UI test tapping the card in the simulator: rejected for this delivery because the listeners
  live in the test bundle and the app under test cannot report to them; #616 carries device and
  real-input proof.
- Loosening a count or adding a tolerance to absorb a reading: rejected because a weakened suite
  proves nothing about the layer.

### D8. The build orders its commits so the switch turns on last

- Chosen: the tests that judge the layer with scripts on are committed red first over stubs that
  compile; then the layer; then, at a head whose CI run reads every criterion green by name on
  both simulators, one commit changes `switchedOn` to `true`.
- The switch and the layer in one commit: rejected because no run would show the layer holding
  before the switch shipped it.
- The switch in a later delivery: rejected because this build is the one that measures the
  layer, and the switch turns on only once that measurement holds.

### D9. No formal model of the layer's ordering

The orderings that matter are all sequential in the app's code or the engine's: the rule list
compiles before any view exists, every user script and the rule list are in the configuration
before the view is made, and the verdict is computed before the card is loaded. The one
interleaving, a card's script against a user script's injection in a new frame, is the engine's,
and the suite measures it in every frame kind a card can make.

- Chosen: no TLA+ or Lean entry; A1's exhaustive cases prove the decision, and the planted suite
  proves the injection order.
- A TLA+ model of "no card script runs before every guard is installed": rejected because it would
  state the engine's behaviour as an assumption, not prove code the app owns.

### D10. The `harness` job's bound rises to 150 minutes, inside a band of 150 to 180

The planted suite is the containment evidence, and it grew with the layer: the card view's run
alone takes about fifty minutes on a hosted macOS runner, and the whole job projects to 85 to 98
minutes. The bound that held before the suite grew now cancels the job at its own limit.

- Chosen: the job's `timeout-minutes` is 150, held inside 150 to 180 by a band test beside the
  engine jobs' bands. It is about one and a half times the projected run and at most half the
  hosted job's limit, so a hung run still ends well inside it.
- Re-running at 90: rejected because the measured run overruns that bound, so a re-run is a coin
  flip, and every later pull request inherits the suite and the overrun.
- The planted suite in a job of its own: rejected because it would put a second hosted macOS
  runner on every pull request and a new check name for the land bar to learn.
- A shorter suite: rejected because it changes the containment evidence that D7 holds
  unloosened, and a cheaper check would prove less than the channels it names.

### D11. The planted suite starts WebKit once per test process, before any measured load wait

Two `harness` runs at one head read one red, and only there: the suite's first test, on the first
simulator, read its first row not loaded at the 10-second wait (#681). The test builds every
row's view and starts every load before it mounts the first, in a host process that has not yet
started WebKit's GPU and networking processes. Those two took 6.3 and 3.7 seconds in the first
run and 7.3 and 12.7 seconds in the second, against 1.4 to 4.3 seconds on the second simulator,
where the same test passed both times. The cost is the host's first start of WebKit, paid once
per process, and whichever wait runs first pays it.

- Chosen, because the cost is paid once per process, so one load that absorbs it under its own
  bound leaves every measured wait as it was: `Probe.warmUp()` loads one planted card in a view
  the factory builds, under `Probe.warmUpSeconds` (60 seconds), once per process, and every test
  class that loads a card asserts it from `setUp`. A host where WebKit never starts fails every
  test with the warm-up's message, so the warm-up hides nothing.
- Rejected, because it is a bound edit (ruling 530) and lengthens every wait that runs to its
  bound, a card that never loads included: raising the 10-second wait.
- Rejected, because it changes what the factory test measures, every row live at once: starting
  each row's load only when its view is mounted.
- Rejected, because it lets the red pass without curing it (ruling 530): an expected-failure mark
  or a retry.
- Rejected, because XCTest's order is no contract: reordering the suite so that another test runs
  first.

## Decision Outcome

Card scripts on iPhone and iPad run only when the switch is on and the factory reads back every
control from the view it built. Loads are refused twice, by the rule list and by the document's
policy; navigations and windows by the gate and the window refusal; a link's activation is
cancelled before the engine follows it, from a world the card cannot reach; and the page's own
entry points that would bypass that refusal are closed in every frame. The `harness` job that
runs the planted suite is bounded at 150 minutes, so the proof finishes inside its limit (D10).
The planted suite starts WebKit once per test process before any measured load wait, so a cold
first test's 10-second wait measures the card and not the host's first start (D11).

### Consequences

- Good: every part is a public engine interface or a script the app installs, and each control has
  a planted card that shows it holding.
- Good: the scripts-off view gains the same activation refusal, so #677's channel closes there too.
- Bad: a card cannot follow a link, open a document for writing, or use `eval`.
- Bad: a tap on an inline element inside a `summary` or `label` no longer toggles it.
- Neutral: the context-menu arm and the preview switch are proved on a device, not in CI.
- Neutral: each simulator's run gains the warm-up's one load and A17's whole 10-second wait.

### Confirmation

SPEC-361's A1 to A17, in the `card-isolation` and `harness` jobs on every pull request that
touches `ios/`; D1, a reading on a device, is held.

## What would make this wrong

- A planted card reaches a listener from either card view: the layer does not hold, and the
  switch stays off.
- An app-world user script does not run while page JavaScript is off: L10 does not hold the
  scripts-off view as D6 claims.
- The engine gains a public per-view switch for link preconnection: L10 and L11 should defer to it.
- A real tap reaches the network on a device while the stand-in click does not: D7's stand-in is
  insufficient, and the seat rules.

## More Information

- ADR-352, ADR-359, ADR-360, ADR-366, SPEC-348, SPEC-349, SPEC-355, #651, #664, #677, #616.
- SEC-01 findings by id: SEC01-F14, SEC01-F15.
