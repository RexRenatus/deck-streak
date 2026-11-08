---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The web card suite counts a loading card's connections at the TCP listener, and no planted card names the HTTP listener

## Context and Problem Statement

#940: the web card suite's HTTP listener counts the requests it serves, by path, and no
connection, so a loading card that opened a connection there and sent no request would not read
red. The suite's TCP listener already counts every connection it accepts under `tcp` and records
each request's path, and every clicked card points at it (SPEC-461, ADR-478 D1 and D7). But
`loads()` still aims the sixteen loading cards it builds at the HTTP listener and declares only
their path (`web/app/tests-card/planted.ts:75-78`, `:121`, at `36efa139`). The HTTP listener
cannot count connections for a card: the harness page's sentinel connects there after every
settle window (`web/app/tests-card/listeners.ts:132-142`). So a loading card's connection that
carries no request reaches no count the suite reads, and neither the card frame's `{}` nor a closed
variant's `{}` can refuse it.

## Decision Drivers

- A loading card's bare connection, out of the card frame or out of a closed variant, must read
  red in Chromium, WebKit and Firefox.
- Each control that proves the counter can read above zero runs in the same test as the zero it
  guards, so no zero is vacuous.
- Nothing a reading proves is loosened: no key is removed from a zero, no arrival is discounted,
  and the sentinel stays where it is.
- The form the clicked cards' controls already take is reused, with nothing new to learn for the
  spec, the harness or the listeners.

## Decisions, and the alternatives each was chosen against

### D1. The observable is the TCP listener's count of accepted connections beside each request's path, and every planted card that loads points at it

`loads()` aims each loading card's markup and CSS at the TCP listener under the card's own path,
and declares `tcp` beside that path as the arrivals that show the card open; `srcset` declares
`tcp` beside its two image-set paths. The TCP listener counts each connection the operating system
hands it, before a byte is read, so it counts a connection that carries a request and one that
carries none, in every engine alike.

Chosen against:

- Counting accepted connections at the HTTP listener: rejected because the sentinel connects there
  after every settle window, so a card's bare connection and the sentinel's share one count
  (ADR-478 D1).
- Discounting the sentinel's own connection there: rejected because it puts the sentinel's
  connection back on the read listener after every settle window and needs a discount a test could
  get wrong; it was refused for the clicked cards (ADR-478 D7).
- Moving the sentinel to a listener of its own: rejected because the sentinel would then prove a
  listener no card uses.
- A fourth listener: rejected for ADR-478 D1's reason, the TCP listener already sees what a fourth
  would.

### D2. No planted card names the HTTP listener, which stays the sentinel's alone

The HTTP listener keeps its request-path count, so a request that reached it would still read; the
sentinel is unchanged. The TCP listener's three facts (SPEC-461's listener criteria, held by
`web/app/tests-card/listeners.spec.ts`) hold for a loading card as for a clicked one: each pair
test's reference half reads that listener above zero, in the same test, before its card frame half
reads zero, so the zero is never vacuous. `webrtc` keeps the loopback host name every listener
shares.

Chosen against:

- Keeping the loading cards at the HTTP listener with a connection count beside the sentinel:
  rejected for D1's first two reasons, the shared count and the discount.

### D3. The test is the suite's own pair and single-layer tests over the sixteen re-aimed cards, with a coverage test and its three refusing controls

The coverage test refuses a loading card that names the HTTP listener, one aimed at the TCP
listener that requests a path without declaring `tcp`, and one that declares `tcp` alone while it
requests a path; a card that only connects declares `tcp` alone.

Chosen against:

- A new browser test per loading card: rejected because it repeats every visit, and the loading
  cards would stay blind to a bare connection in the tests that already run.
- An explicit `tcp` assertion in the spec's loops: rejected because `toEqual({})` already refuses a
  `tcp` key, and the coverage test reads that loop's text.

### D4. Each red fails because no loading card names the counter yet

The declared arrivals and the coverage test are committed over the base's aim unchanged, so each
red is its criterion's assertion: the reference half and each `W2 off` variant that opens wait for
`tcp` and read none.

Chosen against:

- One push: rejected because the browser reds exist only as CI's reading of the red commit.

### D5. One delivery of test code and documents, no rows, no formal entry, the job bound kept, two pushes

The changed files are test code, which StrykerJS does not mutate; the band of SPEC-641's number
stays unused. No state, protocol or pure decision function is added, so no formal entry is
written. The job's bound is unchanged.

Chosen against:

- Mutation rows for the aim: rejected because no rows table can kill a test-code mutant with a test
  it runs, and the red commit is already the aim's mutant, read killed by the suite.

### D6. What is not in this delivery

As SPEC-641 section 5 says: a stock browser build's speculative connection (#941), the iPhone and
iPad suites (#677), a DNS lookup, any change to the UNOBSERVABLE or BARE tables, and anything only a
later CI run can measure.

## Decision Outcome

Every planted card that loads points at the TCP listener through `loads()` and declares `tcp`
beside its own path; no planted card names the HTTP listener. The suite's own pair and
single-layer tests then read each loading card at the connection and its request, in Chromium,
WebKit and Firefox, and a coverage test pins the aim. One pull request, two pushes, and no
production change.

### Consequences

- A loading card's reference load is served an empty response by the TCP listener, as the HTTP
  listener served it.
- Each loading card's reading names `tcp` beside its path.
- The card frame's `{}` and each closed variant's `{}` now refuse a loading card's connection as
  well as its request.
- `planted.ts` changes in five lines and the coverage test gains one test, nothing else.

### Confirmation

SPEC-641's acceptance criteria A1 to A5 and its red-first record.

## What would make this wrong

- A re-aimed card's card frame reads `tcp` at the head: a real connection out of the card frame,
  #940's question answered yes, and the frame needs a cure of its own.
- A closed variant reads one bare connection: a BARE entry needs a later ruled delivery.
- The WebKit `prefetch` pair reads `tcp`: UNOBSERVABLE is no longer true there.
- The suite gains a second worker: the counter is module state, shared by visits that would then
  overlap.

## More Information

- SPEC-341, ADR-352, SPEC-461, ADR-478, #940, #761, #941, #677.
