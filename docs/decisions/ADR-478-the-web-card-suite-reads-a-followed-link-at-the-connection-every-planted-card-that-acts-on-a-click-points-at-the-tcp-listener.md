---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The web card suite reads a followed link at the connection: every planted card that acts on a click points at the TCP listener

## Context and Problem Statement

#761: the web review's card frame strips link elements and refuses a followed link, but the
planted card suite cannot see a bare connection that a followed link might open before the
refusal. It asks for a connection-level observable in the web card suite, so that a test can
assert that following a link in a card opens no connection, as the iPhone and iPad planted suite
already asserts for every card.

Measured at `83846ea3`:

- The suite already holds a connection counter. The TCP listener counts every connection it
  accepts, before any byte and whether or not a request follows, and closes it at once, reading no
  request (`web/app/tests-card/listeners.ts:52-55`); a reading names that count `tcp` beside the
  HTTP listener's request paths (`:100-102`), so a reading at the TCP listener holds `tcp` alone.
  Three planted cards point at it: `preconnect`, `dns-prefetch` and `shadow-link`
  (`web/app/tests-card/planted.ts:143-166`).
- No card that acts on a click points at it. `clicks()` aims each clicked card at the HTTP listener
  and expects a request path (`planted.ts:69-72`), and eight cards are built by it: `nav-self`,
  `nav-top`, `nav-blank`, `download`, `ping`, `form`, `external-scheme` and `javascript-url`
  (`:169-174`, `:180`, `:185`). The HTTP listener counts requests by path and nothing else
  (`listeners.ts:45-51`), so a connection that carries no request reaches no count the suite reads.
- The HTTP listener cannot count connections for a card. After every settle window the harness page
  itself requests a sentinel path there (`listeners.ts:126-136`, `web/app/tests-card/card.spec.ts:95-99`),
  so a connection count at that listener would hold the sentinel's connection beside any card's,
  and a connection with no request carries no path to tell them apart.
- The refusal. The card frame is an `iframe` with an empty `sandbox` attribute (W1,
  `web/app/src/lib/card/CardFrame.svelte:19`, `web/app/src/lib/card/policy.js:18`), and the page's
  `frame-src 'none'` (W4, `policy.js:11`) holds the frame's own navigation. The schematic gives
  `nav-self` and `download` to W4 alone, and `nav-top` and `nav-blank` to W1 alone
  (`docs/schematics/card-frame-channels.md:111-114`).
- The suite already holds both controls a connection reading needs. Each card is opened in the
  reference frame, where it must reach its declared arrivals (`card.spec.ts:113-119`), and in the
  card frame, where the reading must be empty (`:124-125`); and with each layer removed alone, where
  the card's own layer opens it and every other reads empty (`:130-141`). The UNOBSERVABLE table
  declares `ping` blind in Firefox and `external-scheme` blind in all three engines
  (`planted.ts:213-214`, `:217`, `:219`).
- The iPhone and iPad suite's listener serves HTTP and counts every connection it accepts
  (`ios/CardProbeTests/Listeners.swift:27-28`, `:152-156`). Its planted test asserts zero
  connections from the card view for every card, with the control that `nav-self` and `nav-blank`
  each open one with the link-activation refusal removed
  (`ios/CardProbeTests/PlantedCardTests.swift:614`, `:623-637`).
- The suite runs in Chromium, as the headless shell the job installs, in WebKit and in Firefox, on
  one worker, one visit at a time (`web/app/playwright.card.config.ts:12-21`,
  `.github/workflows/ci.yml:285-290`). Under the suite, Chromium and Firefox open no `preconnect`
  connection, and WebKit does (`planted.ts:211`, `:215`).

## Decision Drivers

- The observable counts at the socket, outside every engine: a browser's own events report
  requests, never a connection that carries none.
- Its zero is shown able to fail by a control that opens a connection the same counter sees, in the
  same engine and the same run.
- Each red fails every time for its own reason, never by catching a connection that may not come.
- The card frame's production code is what the suite judges, and a test delivery changes none of it.
- The coverage test reads the schematic's section 3 and the spec's structure
  (`web/app/src/lib/card/planted-coverage.test.ts:196-275`), so both stay as they are.

## Decisions, and the alternatives each was chosen against

### D1. The observable is the TCP listener's count of accepted connections, and every card that acts on a click points at it

The TCP listener counts each connection the operating system hands it on the loopback address,
before a byte is read, so it counts a connection that carries a request, one that carries none, and
one an engine opens ahead of a request and abandons. It sits outside every engine, so it sees such
a connection in Chromium, WebKit and Firefox alike. `clicks()` aims each card's link, form action or
ping at it, and declares `tcp` and the card's own path as the arrivals that show the card open. The
listener keeps its port and its count, and is restated to serve and read each request as well (D7).

What it cannot see is named, never assumed covered. A DNS lookup reaches no listener, and a
followed link here names an address literal, so it needs none (`dns-prefetch` stays UNOBSERVABLE).
A connection an engine never opens under the suite is not exercised: Chromium and Firefox open no
`preconnect` connection under the suite, so a speculative connection that a stock build of either
might open toward a link is not exercised there, and WebKit, which opens one, is the engine where
that path is live. A connection toward any destination other than the link's own is not counted,
and a followed link names no other.

Chosen against:

- Counting accepted connections at the HTTP listener: rejected because the harness page's sentinel
  connects there after every settle window, so a card's bare connection and the sentinel's share
  one count and nothing tells them apart.
- A fourth listener that answers HTTP and counts every connection: rejected because the TCP
  listener already counts every accepted connection, and restated to answer HTTP (D7) it sees what a
  fourth would, without the listener, the query parameter and the `Listener` field the harness and
  the coverage test would both learn.
- The browser's own network events (request events, routing, Chromium's DevTools protocol):
  rejected because they report requests, never a connection without one, the protocol is
  Chromium's alone, and the coverage test refuses them in the spec (`planted-coverage.test.ts:181-185`).
- A packet capture on the loopback interface: rejected because it needs privileges the job does not
  grant, and it reads every process's traffic rather than one visit's.

### D2. The test is the suite's own pair and single-layer tests over the eight re-aimed cards, beside a coverage pin of the aim

With the eight clicked cards aimed at the TCP listener, the suite's existing tests assert, per card
and per engine, beside every observer the suite already holds:

1. The reference frame (`open.html`, every layer off): the click reaches `tcp` and the card's own
   path (`card.spec.ts:118`), the control that the observable sees a followed link's connection and
   the request it carried, unless the card is UNOBSERVABLE there, when it must reach nothing
   (`:114-117`).
2. The card frame (`index.html`, every layer on): after the settle window and the HTTP sentinel the
   reading is `{}` (`:125`). It holds no `tcp` key and no path key, so the follow opened no
   connection, and no request, datagram or bridge counter arrived either.
3. Each layer removed alone (`variant.html?off=W1` to `W4`): the layer that alone holds the card
   opens it, and the follow from the shipped frame with only that refusal removed reaches `tcp` and
   the card's own path (`:136`): `nav-self` and `download` with W4 off, `nav-top` and `nav-blank`
   with W1 off. This is the control that the click lands in the shipped frame's shape, so the card
   frame's zero is the refusal's and not a missed click. Every other layer removed alone reads `{}`
   (`:137`).

Each control reads both keys. `tcp` shows that the counter saw the follow's connection, and the
path shows that the connection carried the card's own request, so no control reads green on a
connection an engine opened ahead of the click and abandoned while the follow itself never left.

The follow is the suite's own: a mouse click at the frame's centre, where each clicked card lays
its full-frame link or button (`card.spec.ts:68-72`, `planted.ts:59-60`), which moves the pointer
over the link, then presses and releases it. `card.spec.ts` is unchanged. A coverage test pins the
aim (SPEC-461 A1): every clicked card names the TCP listener and not the HTTP one, unless it is
UNOBSERVABLE in every configured engine, and expects exactly `tcp` and its own path; a planted card
aimed the old way, or one that declares `tcp` alone, is refused by name.

Chosen against:

- A new table of followed links and a named browser test beside the unchanged clicked cards:
  rejected because the pair and single-layer tests already hold the reference control, the card
  frame's reading and the single-layer control per card and engine, and a second set would repeat
  each follow in every engine while the clicked cards stayed blind to a bare connection.
- Re-aiming only the four navigation cards: rejected because `ping`, `form` and `javascript-url`
  act on the same click and count at the same request-only listener, and one helper aims all eight.
- The iPhone and iPad shape, a connection count asserted for every card at one listener: rejected
  for the web because the HTTP listener's sentinel shares that count (D1), and a loading card's
  bare connection is outside #761 (D6).
- An explicit `tcp` assertion added to the spec's pair loop: rejected because `toEqual({})` already
  refuses a `tcp` key, and the coverage test reads that loop's text (`planted-coverage.test.ts:248-262`).
- Controls that read `tcp` alone: rejected because such a control reads green on a connection that
  an engine opens ahead of a follow and abandons, so it cannot tell a follow from a speculative
  connection, and the TCP listener as it stands reads no request to tell them apart (D7).

### D3. Each red fails because no clicked card names the counter yet

The red commit declares `tcp` and the card's own path as each clicked card's arrivals while its
markup still names the HTTP listener, and adds the coverage pin and the listener's spec (D7). Then
the coverage pin fails at once, naming the seven clicked cards whose markup names the HTTP
listener; each reference control reads its path at the HTTP listener but nothing at `tcp` in each
engine where its card is observable (20 pair tests), as does each single-layer control (12 variant
tests); and the served-request fact fails in each engine (3 tests), because the TCP listener there
still closes each connection before its request. The browser reds are read in CI's `card-sandbox`
job. Each red is certain rather than timed: no card names the TCP listener's port, so no connection
can reach it however long the suite waits, the reach timeout only ends a wait that cannot succeed,
and a listener that closes every connection at once can serve no request. The fix commit restates
the TCP listener (D7) and changes the aim, the HTTP listener's origin to the TCP listener's, and
nothing else a test reads. The record names the red commit `red at <sha>` with the failure
observed there and the fix `green at <sha>`, and a criterion that cannot be red first `not red`
with its reason.

The card frame's zero (D2, item 2) is not red first: the shipped frame opens no connection before
the fix or after it, which is the property itself. Its power is the single-layer control in the
same run and engine, read through the same reading of the same counter from the shipped frame with
one refusal removed.

The no-request fact and the sentinel fact (D7) are not red first either: the TCP listener at the
red commit already reads a connection with no request as `tcp` alone and never meets the sentinel.
Each is held to a control at the head in the same run: the served-request fact shows a path key at
that listener, and the no-request fact shows `tcp` there.

Chosen against:

- A red caught from the card frame itself: rejected because whether an engine opens a connection
  there is what the suite measures, so such a red would be a wait that hopes to catch a connection.
- A red planted in the card frame's production code (its frame source or its sandbox removed in the
  red commit): rejected because it reds through code this delivery must not change, and opens every
  card's pair at once.
- A red read at a fixed delay: rejected because its outcome would depend on timing.
- The red and the fix in one commit: rejected because a red and a green at one commit prove nothing,
  and the red-first record refuses them.
- The listener restated in the red commit: rejected because the served-request fact would then be
  green before it was seen red.

### D4. One delivery of test code and documents, with two insert-only amendments

One pull request, one SPEC, one ADR, one band. No file of the card frame changes:
`CardFrame.svelte`, `frame-document.ts`, `frame-host.ts` and `policy.js` stay as they are, and the
only file under `web/app/src` that changes is the coverage test. If the card frame is read opening
a connection, the delivery stops there, and the frame's cure is a delivery of its own with its own
criterion.

Two amendments, each insert-only and each its own final. SPEC-392, which cites #761 as an exclusion
(its section 5), gains a last section recording that the web suite now reads a followed link at the
connection. The schematic `docs/schematics/card-frame-channels.md` gains a last section that places
the counter and the controls on a followed link's path, per engine. Section 3's table, which the
coverage test reads, is unchanged: its cells say "at the listener", which still holds.

Chosen against:

- Two deliveries, the aim first and the controls after: rejected because the aim is one helper's
  line and the controls are the suite's own, so neither half proves anything alone.
- Amending SPEC-341 instead of SPEC-392: rejected because SPEC-341's R8 already names TCP
  connections among the listeners' arrivals, and an open pull request changes SPEC-341.
- Rewriting section 3's rows in place: rejected because the coverage test reads that section, and
  an edit in place is not insert-only.

### D5. Red first per criterion, no rows, the bound kept, no formal entry, two pushes, sonnet

- Rows: none from S46100. The changed code is test code (the planted table, the TCP listener, its
  spec and a coverage test). StrykerJS mutates `src` and excludes test files (`web/app/stryker.config.json`),
  and no rows table runs a Vitest or Playwright killer (`scripts/mutation_rows.py:81-91`). The red
  commit is itself the mutant of the aim, the clicked cards pointed back at the HTTP listener, and
  A1 to A3 read it killed; and of the listener, a TCP server that serves no request, which A6 reads
  killed.
- Bound: kept (`.github/workflows/ci.yml:273`). Five completed runs on `dev` read the
  `card-sandbox` job, each under that bound:
  965 s (job 117379697051);
  1043 s (job 117228531110);
  1037 s (job 116852594522);
  1062 s (job 116852564311);
  1048 s (job 116828774378).
  The delivery adds no card visit: the listener's spec adds three tests per engine, one of them
  loading one page from the HTTP listener, and its red push ends 32 tests at the suite's reach
  timeout (`card.spec.ts:19`) instead of their usual visits and fails A6's three at once.
- Formal: NOT APPLICABLE for TLA+ and for Lean, by surface. The counter is one worker's module
  state, written by its accept handler and read and reset by that worker between visits it runs one
  at a time, each closing its browser context before the next reset; no pure function here decides
  a verdict; and no formal cover can name a TypeScript file. At `83846ea3`, 0 of the 251
  `@phx covers` lines under `formal/` name a path under `web/` or `docs/`.
- Pushes: two. The browser reds exist only as CI's reading of the red commit, so the first push
  carries the red commit, and the second the fix and the record.
- Model: sonnet. Every edit is named to its line, the reds are read in CI by name, and nothing in
  the build calls for a design judgement.

Chosen against:

- A hand row on the planted table killed by a new Python reader: rejected because it would
  duplicate the coverage pin in a second language only to fill a table.
- A raised job bound: rejected because the measured times sit under the bound, and a raise is
  main's ruling.
- A TLA+ model of the counter and the visits: rejected because no cover can name the TypeScript it
  would model, and the visits are sequential.
- One push: rejected because the browser reds exist only as CI's reading of the red commit.
- Opus for the build: rejected because the build is a line-named edit and a CI reading.

### D6. What is not in this delivery

Firefox for the engine and study suites (#764); the iPhone and iPad suites, whose planted test
already counts every connection for every card (#677); the simulator harness (#839); a bare
connection that a loading card might open at the HTTP listener, which counts requests by path
(#940); a speculative connection that only a stock browser build would open (#941); and anything
only a later CI run can measure (#761).

Chosen against:

- Counting a loading card's connections here: rejected because the HTTP listener's sentinel shares
  that count (D1), and #761 asks for the followed link.
- A full Chromium build beside the headless shell: rejected because it is a browser matrix decision
  (ADR-412's), not an observable, and it changes the job's install step.

### D7. The TCP listener is restated as a request-reading listener on its own port, without the sentinel

The TCP listener keeps its own port, the one the system chooses for it at start, and the
`Listener.tcp` origin the harness and the cards already name, and becomes an HTTP server
(`node:http`'s `createServer`, whose server is a `net.Server`). Its `connection` handler counts
every connection it accepts under `tcp`, before a byte is read and whether or not a request
follows, as the TCP server's did, and no longer closes it. Its `request` handler records each
request's path under that path's own key, in the map the HTTP listener writes, and answers as the
HTTP listener does: an empty response, never stored, that closes its connection. It has no sentinel
branch: the sentinel's address is the HTTP listener's (`listeners.ts:127`), and a sentinel that
reached this listener would read as `tcp` and a path key, which the card frame's `{}` refuses.
`reading()` is unchanged (`listeners.ts:100-109`): `tcp` is the connection count, and every path
key comes from the one map both servers write. `close()` closes its open connections as it closes
the HTTP listener's (`listeners.ts:139`), since a connection that sends no request stays open.
Every `listeners.ts` line cited in this decision is the cut's, at `83846ea3`.

It states three facts, and its own spec, `web/app/tests-card/listeners.spec.ts`, proves each by its
own test in each engine (SPEC-461 A5 to A7):

1. A connection that sends no request reads `tcp` and no path key.
2. A served request reads `tcp` and its path key.
3. The harness's sentinel never reaches it.

Each fact's absence is held to a presence in the same run: the second shows that a path key from
this listener reaches the reading, so the first's missing path key is not a blind reading; the
first shows that `tcp` counts a connection, so the third's missing `tcp` is not either; and the
third is read only once the sentinel has arrived at the HTTP listener, which `sentinel()` waits for.

Chosen against:

- The TCP server that closes each connection at once, read as `tcp` alone: rejected because a
  control at it reads the connection only, never the request the follow carried.
- A fourth listener beside the TCP listener that serves HTTP and counts connections: rejected because
  it adds a port, a query parameter and a `Listener` field the harness and the coverage test must
  learn, while the restated listener keeps the `tcp` origin every card and the harness name.
- Reading the request line by hand on the TCP server's socket: rejected because the server would
  then parse and answer HTTP itself, which `node:http` does under the same `connection` event.
- Path keys spelt apart per listener (`tcp:/nav-self/1`): rejected because `Listeners.holds`
  matches a request path by its leading `/` (`listeners.ts:111-115`) and every planted card
  declares its paths so; the coverage test pins the aim instead, and a path read without `tcp` is
  the red commit's reading.
- A sentinel at the TCP listener as well: rejected because its connection would share `tcp` with
  every card's, which is D1's reason against counting at the HTTP listener.
- The facts tested in `card.spec.ts`: rejected because that file stays unchanged (D2), and a
  sibling spec under `tests-card/` is collected by the card configuration as it stands
  (`playwright.card.config.ts:10`, which sets no `testMatch`).
- The facts tested under Vitest: rejected because the third needs a browser page to send the
  sentinel, and a Vitest file beside the coverage test would be another change under `web/app/src`.

## Decision Outcome

Every planted card that acts on a click points its link, form action or ping at the TCP listener,
which counts every connection it accepts and, restated, serves and reads each request's path, and
expects `tcp` and its own path. The suite's own pair and single-layer tests then read each followed
link at the connection and its request, in Chromium, WebKit and Firefox: seen from the reference
frame and from the shipped frame with its refusal alone removed, and absent from the card frame. A
coverage test pins the aim, and the listener's own spec proves its three facts. One pull request,
two pushes, and no production change.

### Consequences

- A clicked card's reference follow is served an empty page by the TCP listener, as the HTTP
  listener served it before; the suite already reads a page that has left the harness
  (`listeners.ts:96-99`, `:117-120`).
- A clicked card's reading still names its path, now beside `tcp`.
- Every request the HTTP listener counted for a clicked card rode on a connection, and the TCP
  listener answers each request as the HTTP listener did, so no control that opened before the aim
  can read blind after it; the card frame's `{}` now refuses a connection as well as a request.
- `listeners.ts` changes in the TCP listener's server, its handlers and its close, the listener's
  spec is added beside it, and `card.spec.ts`, the harness and the configuration are unchanged.

### Confirmation

SPEC-461's acceptance criteria A1 to A7 and its red-first record.

## What would make this wrong

- A re-aimed card's reference follow reads its path but nothing at `tcp` in an engine: its request
  rode on a connection accepted before the visit's reset, an engine carried a connection across
  browser contexts, and the suite needs a fresh listener per visit.
- The card frame's reading holds `tcp` at the head: a followed link opens a connection before its
  refusal, #761's question is answered yes, and the frame needs a cure of its own.
- An engine build the suite does not run opens a speculative connection toward a link: the suite's
  zero says nothing about it.
- The suite gains a second worker: the counter is module state, shared by visits that would then
  overlap.

## More Information

- SPEC-341, ADR-352, SPEC-392, ADR-406, ADR-412, ADR-413, #761, #664, #677, #764, #839.
