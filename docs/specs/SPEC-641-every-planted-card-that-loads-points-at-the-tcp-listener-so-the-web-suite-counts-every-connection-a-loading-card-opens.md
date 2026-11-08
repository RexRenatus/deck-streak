# SPEC-641: every planted card that loads points at the TCP listener, so the web suite counts every connection a loading card opens

- **Issues:** #940, closed by this delivery. **Context(s):** `miniapp` (`web/app`).
- **Decided by:** ADR-664, one line per decision:
  D1 the observable is the TCP listener's count of accepted connections beside each request's path, and `loads()` aims every loading card at it, declaring `tcp` beside the card's own path;
  D2 no planted card names the HTTP listener, which stays the sentinel's alone with its request-path count;
  D3 the test is the suite's own pair and single-layer tests over the sixteen re-aimed cards, with a coverage test and its three refusing controls;
  D4 each red fails because no loading card names the counter yet, so the declared arrivals and the coverage test come before the aim;
  D5 one delivery of test code and documents, no rows, no formal entry, the job bound kept, two pushes;
  D6 what is not in this delivery, as section 5 says.
  It builds on ADR-352 D7, ADR-412, ADR-413 and ADR-478, and amends none of them.
- **Schematic:** `docs/schematics/card-suite-loading-connections.md`, new: a loading card's
  arrivals, the listener each card names, and the key that proves the counter can read above zero.
- **Amends:** nothing.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-641.md`.

## 1. The problem, measured

Every figure below was read at `36efa139`, by `git show 36efa139:<path>` with `grep -n` or a line
range.

- **The loading cards point at the HTTP listener.** `loads()` aims the markup and CSS of every
  card it builds at the HTTP listener and declares only the card's own path as its arrival
  (`web/app/tests-card/planted.ts:75-78`); `srcset` overrides that arrival with its two image-set
  paths, again without `tcp` (`:121`). It builds sixteen cards: `img`, `srcset`, `css-url`,
  `css-import`, `font`, `media`, `stylesheet`, `preload`, `prefetch`, `meta-refresh`, `base`,
  `nested-frame`, `object`, `script-inline`, `script-src` and `event-handler`.
- **The HTTP listener counts no connection.** It counts each request by its path and nothing else
  (`web/app/tests-card/listeners.ts:45-51`). After every settle window the harness page requests a
  sentinel there (`listeners.ts:132-142`, called by `card.spec.ts:95-99`), so a connection count
  there would mix a card's connection with the sentinel's.
- **The TCP listener counts every connection.** It counts each connection it accepts under `tcp`,
  with a request or without one, and each request under its path, and the sentinel never reaches it
  (`listeners.ts:52-61`; SPEC-461's three listener facts, held by
  `web/app/tests-card/listeners.spec.ts`).
- **The other cards already name it.** Every clicked card points at the TCP listener and declares
  `tcp` beside its path (`planted.ts:80-83`). The three connection-only cards, `preconnect`,
  `dns-prefetch` and `shadow-link`, name it and declare `tcp` alone (`:154-177`). `webrtc` names
  only the HTTP listener's host name, the loopback host every listener shares, for its STUN server
  (`:224`), and `bridge` names no listener.
- **So a loading card's bare connection is read by nothing.** A connection that a loading card
  opens and that carries no request reaches no count the suite reads, and the card frame's
  `toEqual({})` (`card.spec.ts:125`) cannot refuse it; nor can a closed single-layer variant's.

## 2. Requirements

- **R1.** `loads()` aims each loading card's markup and CSS at the TCP listener under the card's own
  path, and declares `tcp` and the card's own path as the arrivals that show it open; `srcset`
  declares `tcp` beside its two image-set paths.
- **R2.** No planted card names the HTTP listener's origin: it is the harness sentinel's alone and
  keeps its request-path count, so a request that reached it would still read. `webrtc` keeps the
  loopback host name it shares with every listener.
- **R3.** A coverage test (A1) holds R1 and R2 for every planted card that loads. It refuses a
  loading card that names the HTTP listener; one aimed at the TCP listener that requests a path and
  declares it without `tcp`, whose control would never read the count above zero; and one that
  declares `tcp` alone while it requests a path, whose control would read a bare connection as the
  request (the clicked cards' controls already read both keys). A card that only connects declares
  `tcp` alone, as at the base. Each refusal is a planted card refused by name, and the test prints
  its `examined` counts after its behaviour assertion.
- **R4.** Nothing else in the suite changes: `web/app/tests-card/card.spec.ts`, the harness,
  `web/app/tests-card/listeners.ts`, `web/app/tests-card/listeners.spec.ts`, both card
  configurations, every workflow, the card frame's production files, the UNOBSERVABLE table (13
  pairs) and the BARE table (2 entries) are unchanged.
- **R5.** Red first: the declared arrivals and A1's test are committed before the aim, over the
  base's aim unchanged, so A1, A2 and A3 read red by their assertions, and the red-first record
  comes with the reds.
- **R6.** The documents: this SPEC, ADR-664, the schematic, `docs/red-first/SPEC-641.md` and the
  changelog fragment.

## 3. Acceptance criteria of SPEC-641

| id | criterion | decided by | red first by |
|---|---|---|---|
| A1 | every planted card that loads names the TCP listener and not the HTTP listener; one that requests a path declares `tcp` first and then only its own paths, and one that only connects declares `tcp` alone; a card aimed the old way, one declaring its path without `tcp`, and one declaring `tcp` alone while it requests a path are each refused by name | Vitest, `planted-coverage.test.ts` "every planted card that loads points at the TCP listener, which counts every connection" | the red commit's markup, still aimed at the HTTP listener, read locally: the test names the sixteen loading cards |
| A2 | each loading card's reference frame reaches `tcp` and its own path in each engine where the card is observable, and its card frame reaches nothing, no connection included | Playwright, the pair tests "`<id>`: the reference frame reaches its listener and the card frame reaches nothing" for the sixteen loading cards | the red commit, read in CI: 47 of the 48 read nothing at `tcp`; the WebKit `prefetch` pair is UNOBSERVABLE there |
| A3 | the shipped frame with W2 alone removed opens a connection the TCP listener counts, carrying the card's own path, for the six W2-alone loading cards `img`, `css-url`, `css-import`, `font`, `media` and `nested-frame` | Playwright, the variant tests "W2 off: `<id>` opens" | the red commit, read in CI: 18 of 18 read nothing at `tcp` |
| A4 | the whole planted suite passes in Chromium, WebKit and Firefox at the head | Playwright, `test:card` whole | not red: the kept suite of SPEC-341, SPEC-398 and SPEC-461 |
| A5 | every single-layer variant of a loading card that stays closed reads nothing, no connection included | Playwright, the variant tests "W1 off" to "W4 off: `<id>` stays closed" for the sixteen loading cards | not red: a zero at both commits; A3 is its control in the same run |

Each browser criterion is decided in CI's card-sandbox job, in Chromium, WebKit and Firefox in one run; on the build machine the suite is only listed.

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "every planted card that loads points at the TCP listener, which counts every connection"
A2: pnpm --dir web/app exec playwright test --config playwright.card.config.ts -g " (img|srcset|css-url|css-import|font|media|stylesheet|preload|prefetch|meta-refresh|base|nested-frame|object|script-inline|script-src|event-handler): the reference frame reaches its listener and the card frame reaches nothing"
A3: pnpm --dir web/app exec playwright test --config playwright.card.config.ts -g "W2 off: (img|css-url|css-import|font|media|nested-frame) opens"
A4: pnpm --dir web/app test:card
A5: pnpm --dir web/app exec playwright test --config playwright.card.config.ts -g "W[1-4] off: (img|srcset|css-url|css-import|font|media|stylesheet|preload|prefetch|meta-refresh|base|nested-frame|object|script-inline|script-src|event-handler) stays closed"
```

Listed on the build machine at the base, A2's selector picks 48 tests, A3's 18 and A5's 174 (the
sixteen cards by four layers, less the six that open, is 58, in each of three engines).

## 4. File manifest

| path | change |
|---|---|
| `web/app/tests-card/planted.ts` | changed in five lines: the declared arrivals of `loads()` and of `srcset` (R1, the red commit); the aim of `loads()`, its two parameter names, its comment and the `Listener.http` comment (R1, R2, the green commit) |
| `web/app/src/lib/card/planted-coverage.test.ts` | changed: A1's test inserted before the clicked-card test (R3) |
| `docs/specs/SPEC-641-every-planted-card-that-loads-points-at-the-tcp-listener-so-the-web-suite-counts-every-connection-a-loading-card-opens.md` | added: this SPEC (R6) |
| `docs/decisions/ADR-664-the-web-card-suite-counts-a-loading-cards-connections-at-the-tcp-listener-and-no-planted-card-names-the-http-listener.md` | added (R6) |
| `docs/schematics/card-suite-loading-connections.md` | added (R6) |
| `docs/red-first/SPEC-641.md` | added (R5, R6) |
| `changelog.d/card-suite-loading-connections-641.md` | added (R6) |

## 5. What this does NOT cover

- A speculative connection that only a stock browser build would open: the suite runs no such
  build (#941).
- The iPhone and iPad suites, whose planted test already counts every connection (#677).
- A DNS lookup, which reaches no listener: `dns-prefetch` stays UNOBSERVABLE (#940).
- Any change to the UNOBSERVABLE or BARE tables: a reading at the head that contradicts one is a
  finding for a later delivery, never an edit here (#940).
- Anything only a later CI run can measure, such as the job's time after the land (#940).

## 6. Risks

- **A re-aimed card's card frame reads `tcp` at the head.** A real connection out of the card
  frame: #940's question is answered yes. A2 reds by name, and the frame needs a cure of its own,
  never a declared cell.
- **A closed variant reads one bare connection.** A5 reds by name; a BARE entry needs a later ruled
  delivery.
- **The WebKit `prefetch` pair reads `tcp`.** A2's blind branch reds: UNOBSERVABLE is no longer
  true there.
- **The red push lengthens the job toward its bound.** Each red waits out its reach timeout, and
  each loading card's variants use the unmeasured settle window. Detected by the red push's own
  run; the head's run measures each card again.
- **A selector picks other tests.** Detected by the listing counts of section 3, measured on the
  build machine at the base and at each commit.

## 7. The planted suite's readings, per engine

For each loading card, in Chromium, WebKit and Firefox alike unless a cell says otherwise. "tcp +
path" is a reading whose `tcp` count is above 0 and which holds a key that starts with the card's
own path, `/<id>/` (`srcset`: both `/srcset/1` and `/srcset/2`); `{}` is the empty reading.

| card | reference frame | card frame | its own layer removed alone | each other layer removed alone |
|---|---|---|---|---|
| `img` | tcp + path | `{}` | W2: tcp + path | `{}` |
| `srcset` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `css-url` | tcp + path | `{}` | W2: tcp + path | `{}` |
| `css-import` | tcp + path | `{}` | W2: tcp + path | `{}` |
| `font` | tcp + path | `{}` | W2: tcp + path | `{}` |
| `media` | tcp + path | `{}` | W2: tcp + path | `{}` |
| `stylesheet` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `preload` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `prefetch` | tcp + path; `{}` in WebKit (UNOBSERVABLE) | `{}` | none (two or more layers hold it) | `{}` |
| `meta-refresh` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `base` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `nested-frame` | tcp + path | `{}` | W2: tcp + path | `{}` |
| `object` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `script-inline` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `script-src` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `event-handler` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |

At the red commit every "tcp + path" cell reads its path at the HTTP listener but nothing at
`tcp`, and its test fails: 47 pair tests (the sixteen cards in three engines, less WebKit's
`prefetch`) and 18 variant tests (the six W2-alone cards in three engines), 65 in all. Every other
test of the suite passes there, A5's among them, and WebKit's `prefetch` pair reads `{}` as its
blind branch expects. At the head every cell reads as written: each pair, each `W2 off` variant
that opens and each closed variant reads as A2, A3 and A5 say, and the card frame's `{}` and each
closed variant's `{}` now refuse a loading card's connection as well as its request.

## 8. Formal

Not required (ADR-664 D5): this delivery changes the card suite's planted table and its coverage
test and adds documents; it adds no state, protocol, concurrency or pure decision function. At
`36efa139`, no `@phx covers` line under `formal/` names a path under `web/`.

## 9. Mutation rows

None (ADR-664 D5). The band `S64100-S64199` stays unused. Web product code is proved by StrykerJS,
and this delivery changes none; the changed files are test code. The red commit is the mutant of the
aim, read killed by A1, A2 and A3.
