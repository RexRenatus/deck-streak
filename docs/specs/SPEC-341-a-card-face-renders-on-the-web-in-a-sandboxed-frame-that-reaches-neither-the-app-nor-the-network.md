# SPEC-341: a card face renders on the web in a sandboxed frame that reaches neither the app nor the network

- **Wave:** the app campaign, Phase 0 (SPEC-334 row 1.2, "the card HTML sandbox"; R6). **Issue:**
  #619 (the card HTML sandbox spike), its web half. **Context(s):** `miniapp` (`web/app/src`).
- **Decided by:** ADR-352 (this SPEC's own: card scripts off on both platforms, the web layer set,
  the channel inventory) and ADR-335 (card faces in an isolated web view). It closes SEC01-F13 (the
  card frame's navigation), SEC01-F14 (peer connections from card script) for the web, and
  SEC01-F15 (the card frame's other channels) for the web.
- **Schematic:** `docs/schematics/card-frame-channels.md` (the channel tables; this delivery adds
  it).
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-341.md`. **Mutation
  band:** S34100-S34199. #648 (the engine in a Worker) landed on `dev` first, so this delivery
  carries R13's composition.

## 1. The problem, measured

SPEC-334 R6 says a card face renders as HTML in a sandboxed, no-network frame and that a test proves
card JavaScript reaches neither the bridge nor the network. Nothing on `dev` does it yet:

| fact at dev `c56bbd11` | count | command |
|---|---|---|
| card frames, `srcdoc` or `iframe` elements in `web/app/src` | 0 | `git grep -n -E '[<]iframe\|srcdoc' c56bbd11 -- web/app/src` |
| `frame-src` or `sandbox` named in `web/` or `scripts/tests` (one comment about StrykerJS's own sandbox aside) | 0 | `git grep -n -E 'frame-src\|sandbox' c56bbd11 -- web scripts/tests` |
| `default-src` in the page policy (`web/app/svelte.config.js`) | 0 | `git grep -c default-src c56bbd11 -- web/app/svelte.config.js` |
| raw-HTML sinks in `web/app/src` and `web/app/tests` (`{@html`, `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `srcdoc`, `DOMParser`) | 0 | `git grep -n -E '\{@html\|innerHTML\|outerHTML\|insertAdjacentHTML\|document\.write\|srcdoc\|DOMParser' c56bbd11 -- web/app/src web/app/tests` |
| window message listeners in `web/app/src` | 0 | `git grep -n -E "addEventListener\(\s*['\"]message\|onmessage" c56bbd11 -- web/app/src` |

What the page policy means for a card frame, read from the HTML and CSP standards and from the
engines' code:

- A frame's own navigation, including one the frame starts itself, is checked against the
  EMBEDDING page's `frame-src`, which falls back to `child-src` and then `default-src`. The page
  sets none of the three, so a card frame could navigate itself anywhere (SEC01-F13). `navigate-to`,
  the directive that would have let a frame restrict itself, never shipped.
- An `about:srcdoc` document inherits the page's policy, so the page's hash-mode `script-src` would
  refuse every inline script in a `srcdoc` card frame. A card script could run only from a
  URL-loaded frame, which the edge's `frame-ancestors` (the Telegram web client alone) refuses to
  frame inside the app.
- No CSP directive either platform enforces governs a peer connection, and a STUN or TURN request
  can carry data (SEC01-F14). `link rel=preconnect` and `link rel=dns-prefetch` are not fetches,
  so a policy does not govern them either.

## 2. Requirements

R1. A card face renders on the web only through `CardFrame.svelte`: one `iframe` whose `sandbox`
    attribute is present and empty, whose document is the `srcdoc` that `frameDocument` builds,
    with no `src`, `allow` or `allowfullscreen` attribute (UX-05, RND-01).
R2. `frameDocument(html, css)` returns a document whose head opens with a
    `Content-Security-Policy` meta element carrying `FRAME_POLICY` exactly, then an
    `x-dns-prefetch-control` meta element set to `off`, then one `style` element holding the card
    CSS. `FRAME_POLICY` is `default-src 'none'; img-src data:; media-src data:; font-src data:;
    style-src 'unsafe-inline'; form-action 'none'; base-uri 'none'` (ADR-352 D3).
R3. `frameDocument` removes every `link`, `meta`, `base` and `template` element the card carries
    (a `template` can declare a shadow root the inert parse never sees), and keeps the card's other
    markup in order.
R4. `frameDocument` re-parses the document it composed and refuses the card, returning a refusal
    and no document, when the re-parsed head is anything but its own two `meta` elements and one
    `style` whose text is the card CSS, or the re-parsed body holds a `link`, `meta`, `base` or
    `template` element. A refused card renders no frame document and marks the frame element
    `data-card-refused` (fail closed).
R5. The page policy gains `'frame-src': ['none']`, taken from `policy.js`'s `PAGE_FRAME_SRC`, and
    no other change (SEC01-F13).
R6. No card script runs on the web: the sandbox carries no token, the frame policy names no script
    source, and the page's hash-mode `script-src` is inherited (ADR-352 D1; the SEC01-F14
    control).
R7. Card HTML enters the page's own document nowhere: shipped code under `web/app/src` holds no
    raw-HTML sink except `CardFrame.svelte`'s one `srcdoc`, and no window message listener
    (SEC01-F15).
R8. The planted suite plants one card for every channel the schematic's web table names, in
    Chromium and WebKit. Each card reaches its listener from a reference frame with every layer off,
    unless the suite's declared UNOBSERVABLE table names that engine and card, and that table equals
    the measured set exactly; and each card reaches nothing from the shipped card frame. The
    listeners count real arrivals (requests, TCP connections, UDP datagrams), never the browser's
    own request events.
R9. Removing one layer, every other layer on, opens exactly the channels the schematic gives that
    layer alone, in each engine where they are observable.
R10. With card scripts on (the measurement variant: an `allow-scripts` token and one admitted
     script source, every other layer on), the `webrtc` card reaches the UDP listener, and the
     `bridge` card's parent message arrives with origin `"null"` while the bridge stand-in, the
     page's storage and its BroadcastChannel stay unreached. These are measurements the ADR
     records, not behaviour that ships (SEC01-F14, SEC01-F15).
R11. The card renders: a screenshot of the card frame equals the reference frame's for the
     render-proof card, and differs from a blank frame's.
R12. A `card-sandbox` CI job runs the planted suite in both engines on every pull request, uploads
     whatever results the suite leaves, whether it passes or fails (a passing run leaves none), and the aggregate `ci` job needs it.
R13. The policy composes with #648: whichever of the two lands second makes `csp.test.ts`'s exact
     directive map and WEB-1's added-directive list agree, and leaves `frame-src` out of the engine
     harness's own header, whose cross-site frame test frames the harness in itself.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the card frame's policy fetches only `data:` images, media and fonts, and admits no script, connection, frame, object, form target or base | `web/app/src/lib/card/policy.test.ts` "the card frame's policy fetches only data and runs no script" |
| A2 | the page policy lets no frame navigate, and adds nothing else | `web/app/src/lib/csp.test.ts` "the page policy lets no frame navigate"; the Playwright `web/app/tests/card-policy.spec.ts` in `test:e2e` reads the built page's meta policy |
| A3 | the frame document opens with the frame policy and keeps the card's body | `web/app/src/lib/card/frame-document.test.ts` "the frame document opens with the frame policy and keeps the card's body" |
| A4 | the frame document drops every link, meta, base and template the card carries | `frame-document.test.ts` "the frame document drops every link, meta, base and template the card carries" |
| A5 | a card whose markup escapes the frame document is refused | `frame-document.test.ts` "a card whose markup escapes the frame document is refused" |
| A6 | the card frame is a sandboxed srcdoc frame with no token | `web/app/src/lib/card/card-frame.test.ts` "the card frame is a sandboxed srcdoc frame with no token" |
| A7 | card HTML reaches the page only through the card frame | `web/app/src/lib/card/card-sinks.test.ts` "card HTML reaches the page only through the card frame" |
| A8 | every channel in the schematic has a planted card, and the harness serves the page policy the app ships | `web/app/src/lib/card/planted-coverage.test.ts` "every channel in the schematic has a planted card" |
| A9 | a planted card reaches its listener from the reference frame and nothing from the card frame, in Chromium and WebKit | the Playwright `web/app/tests-card/card.spec.ts` in `test:card`; its structure by `planted-coverage.test.ts` "the planted suite runs every card in the reference frame and the card frame" |
| A10 | removing one layer opens exactly the channels the schematic gives that layer alone | `card.spec.ts`; `planted-coverage.test.ts` "every layer has a single-layer variant and a channel of its own" |
| A11 | with card scripts on, a peer connection reaches the UDP listener and a parent message arrives from origin null | `card.spec.ts`; `planted-coverage.test.ts` "the scripts-on measurement plants a peer connection and a parent message" |
| A12 | the card renders in the card frame as it does in the reference frame | `card.spec.ts`; `planted-coverage.test.ts` "the render proof compares the card frame with the reference and a blank frame" |
| A13 | the card-sandbox job runs the planted suite in both engines and the aggregate check needs it | `scripts/tests/test_ci_workflows.py` `the_card_sandbox_job_runs_the_planted_suite_in_both_engines` |

The tdd probe resolves no Playwright command, so A9 to A12 name their Playwright spec in the table
and their fence lines run the Vitest test that proves the spec's coverage, as SPEC-028 A4 does.

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/policy.test.ts -t "the card frame's policy fetches only data and runs no script"
A2: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page policy lets no frame navigate"
A3: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "the frame document opens with the frame policy and keeps the card's body"
A4: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "the frame document drops every link, meta, base and template the card carries"
A5: pnpm exec vitest run web/app/src/lib/card/frame-document.test.ts -t "a card whose markup escapes the frame document is refused"
A6: pnpm exec vitest run web/app/src/lib/card/card-frame.test.ts -t "the card frame is a sandboxed srcdoc frame with no token"
A7: pnpm exec vitest run web/app/src/lib/card/card-sinks.test.ts -t "card HTML reaches the page only through the card frame"
A8: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "every channel in the schematic has a planted card"
A9: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the planted suite runs every card in the reference frame and the card frame"
A10: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "every layer has a single-layer variant and a channel of its own"
A11: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the scripts-on measurement plants a peer connection and a parent message"
A12: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the render proof compares the card frame with the reference and a blank frame"
A13: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_card_sandbox_job_runs_the_planted_suite_in_both_engines
```

**The findings, closed:**

| finding | closed by |
|---|---|
| SEC01-F13 (the card frame's navigation) | R5; A2 (the policy), A9's `nav-self` pair (the planted navigation reaches nothing) and A10's `frame-src` variant (the page's `frame-src 'none'` alone holds it) |
| SEC01-F14 (peer connections from card script), web | R6; A9's `webrtc` pair (no peer connection from the card frame) and A11 (the measured reach that scripts off closes); the iPhone and iPad half is the iPhone and iPad delivery's |
| SEC01-F15 (the card frame's other channels), web | R2 to R4, R7; A3 to A7 and A9's every other pair, each channel named in the schematic with the layer that closes it; the iPhone and iPad half is the iPhone and iPad delivery's |

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/src/lib/card/policy.js` | `miniapp` | added: `PAGE_FRAME_SRC`, `FRAME_SANDBOX`, `FRAME_POLICY` |
| `web/app/src/lib/card/policy.test.ts` | `miniapp` | added (A1) |
| `web/app/src/lib/card/frame-document.ts` | `miniapp` | added: `frameDocument(html, css)` |
| `web/app/src/lib/card/frame-document.test.ts` | `miniapp` | added (A3, A4, A5) |
| `web/app/src/lib/card/CardFrame.svelte` | `miniapp` | added |
| `web/app/src/lib/card/card-frame.test.ts` | `miniapp` | added (A6) |
| `web/app/src/lib/card/card-sinks.test.ts` | `miniapp` | added (A7) |
| `web/app/src/lib/card/planted-coverage.test.ts` | `miniapp` | added (A8 to A12) |
| `web/app/svelte.config.js` | `miniapp` | changed: `'frame-src': PAGE_FRAME_SRC`, and the TypeScript project reads the card harness |
| `web/app/src/lib/csp.test.ts` | `miniapp` | changed: the exact map gains `frame-src`; a new test (A2) |
| `web/app/tests/card-policy.spec.ts` | `miniapp` | added: the built page's meta policy (A2) |
| `web/app/policy-header.ts` | `miniapp` | added: the policy-to-header writer the two harness servers share (R13) |
| `web/app/vite.card.config.ts` | `miniapp` | added: the card harness server |
| `web/app/playwright.card.config.ts` | `miniapp` | added: Chromium and WebKit, one worker |
| `web/app/tests-card/card.spec.ts` | `miniapp` | added (A9 to A12) |
| `web/app/tests-card/planted.ts` | `miniapp` | added: the planted-card table, the layer map, UNOBSERVABLE |
| `web/app/tests-card/listeners.ts` | `miniapp` | added: the HTTP, TCP and UDP listeners that count arrivals |
| `web/app/tests-card/harness/index.html` | `miniapp` | added: the shipped `CardFrame` under the page policy |
| `web/app/tests-card/harness/open.html` | `miniapp` | added: the reference frame, every layer off |
| `web/app/tests-card/harness/variant.html` | `miniapp` | added: one layer off, or the scripts-on measurement |
| `web/app/tests-card/harness/main.ts` | `miniapp` | added: mounts the frame, the bridge stand-ins and their counters |
| `web/app/tests-card/harness/planted/webrtc.js` | `miniapp` | added: the scripts-on variant's peer-connection card |
| `web/app/tests-card/harness/planted/bridge.js` | `miniapp` | added: the scripts-on variant's bridge card |
| `web/app/package.json` | `miniapp` | changed: `"test:card"` |
| `.github/workflows/ci.yml` | CI | changed: the `card-sandbox` job and the aggregate's `needs` |
| `scripts/tests/test_ci_workflows.py` | CI | changed: A13, and the aggregate layout's job list gains `card-sandbox` |
| `scripts/mutation-rows.d/S34100-S34199.json` | mutation | added: the three `ci.yml` rows |
| `docs/schematics/card-frame-channels.md` | docs | added |
| `docs/decisions/ADR-352-a-card-face-runs-no-script-and-each-platform-closes-every-other-channel-with-a-named-layer-a-planted-card-proves.md` | docs | added |
| `docs/red-first/SPEC-341.md` | docs | added |
| `changelog.d/card-frame-341.md` | docs | added |

## 5. What this does NOT do

- It does not render the iPhone and iPad card view: that is the iPhone and iPad delivery, which
  builds on the Swift harness (#616).
- It does not put the card frame on a study screen, translate the refusal for the learner, or
  choose the card's size and scrolling: the web study screens own those (#630).
- It does not render a card's media from the collection's media folder, nor a card template's
  fields: the frame admits `data:` sources only, and the engine in the browser and the study
  screens decide how media reaches a card (#648, #630).
- It does not run card scripts. A scripted card (a hint toggle, MathJax, a typed-answer helper)
  renders without its script until an owner decision on scripts, with the peer-connection control
  that would need, is recorded (#651).
- It does not test Firefox: the campaign's browsers are Chromium and WebKit, as #648's are
  (#652).
- It does not write the campaign's threat model (SEC01-F16): #653 owns it.

## 6. Risks

- **An engine's ICE agent skips the loopback interface,** so the reference `webrtc` card cannot
  reach the UDP listener and the F14 measurement reads blind. Detected by the reference pair: a
  reading of zero there fails A9 unless the engine and card are in UNOBSERVABLE, and A11 refuses an
  UNOBSERVABLE `webrtc` in both engines, because the measurement must be made in at least one.
- **An engine renders no `srcdoc` frame under `frame-src 'none'`.** The standards say a `srcdoc`
  document is not fetched, so the directive does not apply; the render-proof card (A12) decides it
  per engine, and a blank card frame fails it.
- **Mutation-XSS: the frame parses the composed string differently from the check.** Both parses
  read the same string with scripting off (an inert parser document, and a sandbox with no
  `allow-scripts`), so `noscript` and the other scripting-dependent elements parse alike; A5's
  planted escapes and A9's channels detect a difference that reaches a listener.
- **#648 lands first and its WEB-1 test pins the directive list,** so this delivery's `frame-src`
  turns WEB-1's test red, and the engine harness's cross-site frame test is blocked by the new
  directive. R13 assigns the fix to whichever lands second; the train's union run detects a miss.
- **A slow runner settles a treatment before a late arrival lands.** The treatment's fixed window is
  at least the reference's measured latency for that card times a stated factor, and a sentinel
  request from the harness page after the window proves the listener was still counting.

## 7. Amendments by SPEC-398: the planted suite runs in Firefox too

SPEC-398 (ADR-412) adds Firefox as the planted suite's third engine, run by the same
`card-sandbox` job. Where this SPEC says the suite runs in Chromium and WebKit, or in both
engines, read Chromium, WebKit and Firefox: R8, R12, A9, A13, and section 5's bullet on
Firefox, which SPEC-398 supersedes for the card frame (#652). A13's test still holds the
Chromium and WebKit install step; SPEC-398's A2 holds the Firefox step beside it.
