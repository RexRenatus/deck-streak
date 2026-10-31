# SPEC-461: every planted card that acts on a click points at the TCP listener, so the web suite reads a followed link at the connection

- **Issues:** #761, closed by this delivery. **Context(s):** `miniapp` (`web/app`).
- **Decided by:** ADR-478 (the observable is the TCP listener's count of accepted connections; the
  test is the suite's own pair and single-layer tests over the re-aimed cards, reading `tcp` and
  each card's path, with a coverage pin; each red fails because no clicked card names the counter
  yet; one delivery of test code and documents; no rows, the bound kept, no formal entry, two
  pushes; the TCP listener restated as a request-reading listener on its own port, without the
  sentinel). It builds on ADR-352 D7, ADR-412 and ADR-413, and amends none of them.
- **Schematic:** `docs/schematics/card-frame-channels.md`, amended insert-only by an appended
  section 13 that places the counter and the controls on a followed link's path, per engine.
- **Amends:** SPEC-392, insert-only, by an appended section 9 on its #761 exclusion.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-461.md`.

## 1. The problem, measured

Every figure below was read at `83846ea3`, by `git show 83846ea3:<path>` with `grep -n` or a line
range, unless its line names another command.

- **The suite cannot see a followed link's bare connection.** Every planted card that acts on a
  click is built by `clicks()`, which aims it at the HTTP listener and expects a request path
  (`web/app/tests-card/planted.ts:69-72`). Eight cards are built so (`grep -c "^  clicks('"`
  reads 8): `nav-self`, `nav-top`, `nav-blank`, `download`, `ping`, `form`, `external-scheme` and
  `javascript-url` (`:169-174`, `:180`, `:185`). The HTTP listener counts requests by path and
  nothing else (`web/app/tests-card/listeners.ts:45-51`).
- **The suite already counts connections, elsewhere.** The TCP listener counts every connection it
  accepts and closes it at once, reading no request (`listeners.ts:52-55`), and a reading names the
  count `tcp` beside the HTTP listener's request paths (`:100-102`), so a reading at the TCP
  listener holds `tcp` alone. Three cards point at it (`grep -c 'l\.tcp'` reads 3): `preconnect`,
  `dns-prefetch` and `shadow-link` (`planted.ts:143-166`).
- **The HTTP listener's count is shared.** After each settle window the harness page requests a
  sentinel path at the HTTP listener (`listeners.ts:126-136`, `web/app/tests-card/card.spec.ts:95-99`).
- **The suite's structure holds the controls.** Each card is read from the reference frame
  (`card.spec.ts:113-119`), from the card frame (`:124-125`) and with each layer removed alone
  (`:130-141`), in Chromium, WebKit and Firefox on one worker
  (`web/app/playwright.card.config.ts:12-21`). The UNOBSERVABLE table holds 13 pairs
  (`grep -c "^  { engine: '"` reads 13), among them `ping` in Firefox and `external-scheme` in all
  three engines (`planted.ts:213-214`, `:217`, `:219`).
- **A sibling spec is collected as the configuration stands.** The card configuration's `testDir`
  is `tests-card` (`playwright.card.config.ts:10`), and it sets no `testMatch` or `testIgnore`
  (`grep -c -E 'testMatch|testIgnore'` reads 0).
- **The refusal.** W1, the sandbox with no token (`web/app/src/lib/card/CardFrame.svelte:19`,
  `web/app/src/lib/card/policy.js:18`), and W4, the page's `frame-src 'none'` (`policy.js:11`). The
  schematic's section 3 gives `nav-self` and `download` to W4 alone and `nav-top` and `nav-blank` to
  W1 alone (`docs/schematics/card-frame-channels.md:111-114`).
- **The iPhone and iPad suite counts every connection.** Its listener counts each accepted
  connection (`ios/CardProbeTests/Listeners.swift:27-28`, `:152-156`), and its planted test asserts
  zero from the card view for every card, with `nav-self` and `nav-blank` as the control that opens
  one (`ios/CardProbeTests/PlantedCardTests.swift:614`, `:623-637`).
- **The job's time.** Five completed `ci.yml` runs on `dev`, the `card-sandbox` job, each under its
  bound (`.github/workflows/ci.yml:273`), read by `gh api` over each run's jobs:
  965 s (job 117379697051);
  1043 s (job 117228531110);
  1037 s (job 116852594522);
  1062 s (job 116852564311);
  1048 s (job 116828774378).
- **No formal cover names the surface.** `git grep -n '@phx covers' 83846ea3 -- formal` reads 251
  lines under `formal/`, and 0 name a path under `web/` or `docs/`.
- **No row anchors on a file this delivery changes.** Of the `scripts/mutation-rows.d` files, one
  row names a web path, S39801 on `web/app/playwright.card.config.ts`, which this delivery leaves
  unchanged.

## 2. Requirements

- **R1.** Every planted card that acts on a click points its link, form action or ping at the TCP
  listener and declares `tcp` and its own path as the arrivals that show it open: `planted.ts:71`
  reads ``return { id, alone, click: true, html: (l) => html(`${l.tcp}/${id}`), css: () => COVER, paths: ['tcp', `/${id}/`] };``.
  No clicked card names the HTTP listener.
- **R2.** The TCP listener is the observable, restated as a request-reading listener on its own
  port, without the sentinel: an HTTP server at the `tcp` origin that counts every connection it
  accepts as `tcp`, with or without a request, records each request's path under its own key in
  the reading, answers each request as the HTTP listener does, and closes its open connections at
  the end. The sentinel stays the HTTP listener's alone, and `reading()` is unchanged. Its header
  comment (`listeners.ts:1-6`) and the `Listener.tcp` comment (`planted.ts:18`) say so.
- **R3.** `web/app/tests-card/card.spec.ts`, the harness and both card configurations are
  unchanged. The suite's own tests then read, per clicked card and engine: the reference frame's
  follow at `tcp` and its own path (the control), the card frame's reading `{}` (no connection, no
  request), and the shipped frame with the card's own layer alone removed at `tcp` and its own path
  (the control), with every other layer removed alone at `{}`.
- **R4.** A coverage test pins R1 (A1): every clicked card names the TCP listener's origin, unless
  it is UNOBSERVABLE in every configured engine, names no HTTP listener origin, and expects exactly
  `tcp` and its own path; a planted card aimed the old way, and one that declares `tcp` alone, are
  each refused by name; and it prints its examined count after its behaviour assertion.
- **R5.** The UNOBSERVABLE table and the schematic's section 3 are unchanged (ADR-413).
- **R6.** No file of the card frame changes, and the only file under `web/app/src` that changes is
  `planted-coverage.test.ts`. A card frame read opening a connection is a finding that stops the
  delivery; its cure is a delivery of its own.
- **R7.** No workflow changes, and the `card-sandbox` job's bound stays as it is.
- **R8.** No mutation row: the band S46100-S46199 stays unused (section 9).
- **R9.** `docs/red-first/SPEC-461.md` holds, per criterion, one red and one green line or one
  not-red line, and names in prose every commit that changes a test file between a red and its
  green; `changelog.d/card-suite-connection-461.md` records the change.
- **R10.** SPEC-392 gains section 9, and the schematic gains section 13, each appended after its
  file's last line, insert-only.
- **R11.** The TCP listener's three facts, each proved by its own test in
  `web/app/tests-card/listeners.spec.ts` in each engine, the spec starting its own listeners as
  `card.spec.ts` does and reading each fact from `Listeners.reading`: (1) a connection that sends
  no request reads `tcp` and no path key (A5); (2) a served request reads `tcp` and its path key
  (A6); (3) the harness's sentinel never reaches the TCP listener (A7).

## 3. Acceptance criteria of SPEC-461

Each browser criterion is decided in CI's `card-sandbox` job, in Chromium, WebKit and Firefox in
one run; on the build machine the suite is only listed.

| id | criterion | decided by | red first by |
|---|---|---|---|
| A1 | every planted card that acts on a click names the TCP listener and not the HTTP listener, unless UNOBSERVABLE in every configured engine, and expects exactly `tcp` and its own path; a card aimed the old way, or one declaring `tcp` alone, is refused by name | Vitest, `planted-coverage.test.ts` "every planted card that acts on a click points at the TCP listener, which counts every connection" | the red commit's markup, still aimed at the HTTP listener, read locally |
| A2 | each re-aimed card's follow from the reference frame reaches the TCP listener at `tcp` and its own path in each engine where the card is observable, and its follow from the card frame reaches nothing, no connection and no path included | Playwright, the pair tests "`<id>`: the reference frame reaches its listener and the card frame reaches nothing" for the seven cards with a destination | the red commit, read in CI: 20 cases read nothing at `tcp` |
| A3 | the shipped frame with the card's own refusal alone removed opens a connection the TCP listener counts, carrying the card's own path: `nav-self` and `download` with W4 off, `nav-top` and `nav-blank` with W1 off | Playwright, the variant tests "W4 off: `<id>` opens" and "W1 off: `<id>` opens" | the red commit, read in CI: 12 cases read nothing at `tcp` |
| A4 | the whole planted suite passes in Chromium, WebKit and Firefox at the head | Playwright, `test:card` whole | not red: the kept suite of SPEC-341 and SPEC-398 |
| A5 | a connection that sends no request reads `tcp` and no path key at the TCP listener | Playwright, `listeners.spec.ts` "the TCP listener: a connection that sends no request reads tcp and no path key" | not red: the TCP listener at the red commit already reads such a connection so; A6 at the head, in the same run, is its control |
| A6 | a request the TCP listener serves reads `tcp` and its path key | Playwright, `listeners.spec.ts` "the TCP listener: a served request reads tcp and its path key" | the red commit, read in CI: 3 cases, the TCP listener there closing each connection before its request |
| A7 | the harness's sentinel never reaches the TCP listener | Playwright, `listeners.spec.ts` "the TCP listener: the harness sentinel never reaches it" | not red: the sentinel names the HTTP listener at the red commit and after; A5 at the head, in the same run, is its control |

Every criterion's test prints `examined N` where it enumerates, after its behaviour assertion.

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "every planted card that acts on a click points at the TCP listener, which counts every connection"
A2: pnpm --dir web/app exec playwright test --config playwright.card.config.ts -g "(nav-self|nav-top|nav-blank|download|ping|form|javascript-url): the reference frame reaches its listener and the card frame reaches nothing"
A3: pnpm --dir web/app exec playwright test --config playwright.card.config.ts -g "(W4 off: (nav-self|download)|W1 off: (nav-top|nav-blank)) opens"
A4: pnpm --dir web/app test:card
A5: pnpm --dir web/app exec playwright test --config playwright.card.config.ts -g "the TCP listener: a connection that sends no request reads tcp and no path key"
A6: pnpm --dir web/app exec playwright test --config playwright.card.config.ts -g "the TCP listener: a served request reads tcp and its path key"
A7: pnpm --dir web/app exec playwright test --config playwright.card.config.ts -g "the TCP listener: the harness sentinel never reaches it"
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/tests-card/planted.ts` | 1 | changed in place, line-neutral: `:71` the aim and the arrivals (R1), `:18`, `:69` and `:70` comment and parameter name (R2) |
| `web/app/tests-card/listeners.ts` | 1 | changed: the TCP listener restated (R2), the header comment `:1-6`, the import `:10`, the field `:42`, the handlers `:52-55`, the server `:64` and the close after `:139`; 18 lines added, 11 removed |
| `web/app/tests-card/listeners.spec.ts` | 1 | added: the TCP listener's three facts (R11) |
| `web/app/src/lib/card/planted-coverage.test.ts` | 1 | changed: A1's test inserted before the last line (R4) |
| `docs/specs/SPEC-461-every-planted-card-that-acts-on-a-click-points-at-the-tcp-listener-so-the-web-suite-reads-a-followed-link-at-the-connection.md` | 1 | added: this SPEC |
| `docs/decisions/ADR-478-the-web-card-suite-reads-a-followed-link-at-the-connection-every-planted-card-that-acts-on-a-click-points-at-the-tcp-listener.md` | 1 | added |
| `docs/specs/SPEC-392-the-iphone-and-ipad-card-view-strips-every-link-element-and-a-followed-link-opens-no-connection.md` | 1 | changed: section 9 appended, insert-only (R10) |
| `docs/schematics/card-frame-channels.md` | 1 | changed: section 13 appended, insert-only (R10) |
| `docs/red-first/SPEC-461.md` | 1 | added (R9) |
| `changelog.d/card-suite-connection-461.md` | 1 | added (R9) |

## 5. What this does NOT cover

- Firefox for the engine and study suites (#764).
- The iPhone and iPad suites, whose planted test already counts every connection for every card
  (#677).
- The simulator harness (#839).
- A bare connection that a loading card might open at the HTTP listener, which counts requests by
  path while the harness's sentinel shares its connections (#940).
- A speculative connection that only a stock browser build would open toward a link: the suite's
  Chromium is the headless shell, and its Chromium and Firefox open no `preconnect` connection
  under the suite (#941).
- Anything only a later CI run can measure, such as the job's time after the land (#761).

## 6. Risks

- **A re-aimed control reads blind at the head.** An engine's reference follow, or its single-layer
  follow, reaches nothing at `tcp`, or no path, where it made a request before. Detected by A2 or A3
  red at the head; the build stops, and a new UNOBSERVABLE pair would be a design change with its
  schematic cell (ADR-413), never made in the build.
- **The restated listener changes a `tcp` card's reading.** The pair test of `preconnect`,
  `dns-prefetch` or `shadow-link` reads a path key, or nothing, where its card is observable.
  Detected by that pair test red at the head; the build stops.
- **The card frame opens a connection.** A2's card-frame leg reads `tcp` at the head: #761's
  question is answered yes. Detected by A2 red at the head, in the engine that opened it; the build
  stops, and the frame's cure is its own delivery.
- **A late connection from the reference visit.** A connection the reference visit opened is
  counted after the card frame visit's reset. Detected as a card-frame `tcp` that a second run does
  not repeat; a green rerun is a measurement, never an admitted pass.
- **The red set differs from section 7.** A predicted red reads green, or another test reds.
  Detected by counting the red push's failures by title against section 7; the build stops.
- **A selector picks other tests.** A2's or A3's `-g` pattern selects other than 21 or 12 tests, or
  A5's, A6's or A7's other than 3.
  Detected by `--list` before the red commit.
- **A sibling changes a manifest file first.** Detected by the build's re-measure at its cut.

## 7. The planted suite's readings, per engine

For each clicked card, in Chromium, WebKit and Firefox alike unless a cell says otherwise. "tcp +
path" is a reading whose `tcp` count is above 0 and which holds a key that starts with the card's
own path, `/<id>/`; `{}` is the empty reading.

| card | reference frame | card frame | its own layer removed alone | each other layer removed alone |
|---|---|---|---|---|
| `nav-self` | tcp + path | `{}` | W4: tcp + path | `{}` |
| `nav-top` | tcp + path | `{}` | W1: tcp + path | `{}` |
| `nav-blank` | tcp + path | `{}` | W1: tcp + path | `{}` |
| `download` | tcp + path | `{}` | W4: tcp + path | `{}` |
| `ping` | tcp + path; `{}` in Firefox (UNOBSERVABLE) | `{}` | none (two or more layers hold it) | `{}` |
| `form` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `javascript-url` | tcp + path | `{}` | none (two or more layers hold it) | `{}` |
| `external-scheme` | `{}` (UNOBSERVABLE) | `{}` | none (two or more layers hold it) | `{}` |

At the red commit every "tcp + path" cell reads its path at the HTTP listener but nothing at
`tcp`, and its test fails: 20 pair tests (the six cards other than `ping` and `external-scheme` in
three engines, and `ping` in Chromium and WebKit) and 12 variant tests (four cards in three
engines), 32; and A6 fails in each engine, 3 more, because the TCP listener there closes each
connection before its request: 35 in all. Every other test of the suite passes there, A5's and
A7's among them. At the head every cell reads as written, and A5 to A7 pass in each engine.

## 8. Formal

NOT APPLICABLE for TLA+ and for Lean (ADR-478 D5): the counter is one worker's module state, read
and reset between visits that run one at a time; no pure function here decides a verdict; and no
formal cover can name a TypeScript file. At `83846ea3`, no `@phx covers` line names a path this
delivery changes.

## 9. Mutation rows

None (ADR-478 D5). The changed files are test code, which StrykerJS does not mutate and which no
rows table can kill with a test it runs. The red commit is the mutant of the aim, read killed by A1
to A3. The band S46100-S46199 stays unused.
