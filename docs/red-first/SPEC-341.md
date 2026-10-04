# Red-first record: SPEC-341

SPEC-341, ADR-352, the channel schematic and the fragment were committed first (cd2cf6e8). The
census tests and the planted-coverage tests were committed alone (51813c1d), with a `planted.ts`
that exported empty tables and no render card, no card frame in `web/app/src` and no card job in
`ci.yml`. The unit tests followed over stubs (df0ccca8): a `policy.js` whose constants were empty,
a `frameDocument` that returned the card unchanged and a `CardFrame` with no sandbox. The planted
suite and its CI job came next (2c798782), and the implementation last (dc8d10e4).

```red-first
A1: red at df0ccca8: policy.test.ts: expected {} to deeply equal { 'default-src': [ '\'none\'' ], …(6) }
A2: red at df0ccca8: csp.test.ts "the page policy lets no frame navigate": expected undefined to deeply equal [ 'none' ]
A3: red at df0ccca8: frame-document.test.ts: expected [] to deeply equal [ …(3) ]
A4: red at df0ccca8: frame-document.test.ts: expected [ …(10) ] to deeply equal []
A5: red at df0ccca8: frame-document.test.ts: expected { Object (srcdoc) } to strictly equal { refused: 'escaped' }
A6: red at df0ccca8: card-frame.test.ts: expected null to be '' // Object.is equality
A7: red at 51813c1d: card-sinks.test.ts: expected {} to deeply equal { …(1) }
A7: green at df0ccca8
A8: red at 51813c1d: planted-coverage.test.ts: expected [] to deeply equal [ 'base', 'bridge', …(26) ]
A9: red at 51813c1d: planted-coverage.test.ts: tests-card/card.spec.ts does not exist: expected false to be true
A10: red at 51813c1d: planted-coverage.test.ts: expected [] to deeply equal [ 'W1', 'W2', 'W3', 'W4' ]
A11: red at 51813c1d: planted-coverage.test.ts: expected [] to deeply equal ArrayContaining ["webrtc", "bridge"]
A12: red at 51813c1d: planted-coverage.test.ts: planted.ts exports no render-proof card: expected null not to be null
A13: red at df0ccca8: CI run 37219603126 job hygiene: AssertionError: 'card-sandbox' not found in {...} : ci.yml has no card-sandbox job
A1: green at dc8d10e4
A2: green at dc8d10e4
A3: green at dc8d10e4
A4: green at dc8d10e4
A5: green at dc8d10e4
A6: green at dc8d10e4
A8: green at 2c798782
A9: green at 2c798782
A10: green at 2c798782
A11: green at 2c798782
A12: green at 2c798782
A13: green at b96fc07c
```

The reds at 51813c1d and the greens at 2c798782 were read from the whole Vitest files by path. CI's
`web` job at df0ccca8 (run 37219603126) read the same reds for A1 to A6 and A8 to A12 (A2 twice,
since the exact directive map of the first page-policy test gained `frame-src` too; `Test Files 5
failed | 55 passed`), and A7 green. CI's `hygiene` job read A13 as its one failure (`Ran 907
tests`, `FAILED (failures=1)`), from the stage log artifact `check-stage-logs-hygiene`. At
dc8d10e4 the whole Vitest suite reads `Test Files 60 passed (60)`, `Tests 311 passed (311)`, and
the card files print `examined 56 shipped source files under src`, `examined 7 planted sinks`,
`examined 10 planted elements`, `examined 3 escaping cards`, `examined 28 web channels in the
schematic`, `examined 2 channels the schematic calls unobservable`, `examined 2 loops over the
planted cards`, `examined 3 planted request-event reads` and `examined 4 layers in the schematic`.

## The planted suite (A9 to A12, Playwright)

A9 to A12's fence lines run the Vitest test that proves the suite's coverage (SPEC-341 section 3).
The Playwright suite itself, `tests-card/card.spec.ts` in `test:card`, was read as follows.

At 2c798782, over the stubs, Chromium ran locally: 144 tests, 54 failed, 90 passed, `examined 28
planted cards, 28 pairs, 112 variants in chromium`. The first failing assertion of each:

- A9, the `img` pair: `img reached out of the card frame in chromium`, `"/img/1": 1` and the
  card's other image paths;
- A10, `W1 off: img stays closed`: `img opened with W1 off in chromium`;
- A12, the render proof: `the card frame renders the card unlike the reference frame`;
- A11 is not red there: it measures the engines with card scripts on, through the harness's own
  frame, not the shipped one, and read `1 datagram(s) reached the UDP listener` and `2 parent
  message(s), origins ["null","null"]`.

Greens over the stubs that are NOT red-first evidence, each with its wrong reason, were measured
again at dc8d10e4, where each is green for its own reason:

- `css-import`, `font`: the stub frame dropped the card CSS, so nothing loaded;
- `nav-self`, `nav-top`, `nav-blank`, `download`, `ping`, `form`: the stub dropped the cover CSS,
  so the click at the frame's centre missed the link;
- `base`: the inherited page policy's `base-uri` blocked it (P, not a W layer);
- `object`, `script-inline`, `script-src`, `event-handler`, `javascript-url`, `webrtc`, `bridge`:
  the `srcdoc` document inherits the page policy, whose `object-src` and `script-src` stop them.

At dc8d10e4, the implementation, Chromium ran locally: 144 tests, 6 failed, 138 passed, `examined
28 planted cards, 28 pairs, 112 variants in chromium`. Every pair the card frame must close reached
nothing, the render proof was green, and the six reds were the schematic's predictions, not the
frame:

- the `preconnect` and `shadow-link` pairs and their `W3 off` variants: `nothing reached tcp`, the
  reference frame opened no preconnect connection within the suite's reach window;
- `W2 off: nested-frame stays closed`: `"/nested-frame/2": 1`, the nested `srcdoc` child's image;
- `W4 off: download stays closed`: `"/download/1": 1`, the link followed as a navigation.

f8637aad corrected `tests-card/planted.ts` and the schematic's web table together to that
measurement: `preconnect` and `shadow-link` are UNOBSERVABLE in Chromium, `nested-frame`'s layer
alone is W2 and `download`'s is W4. At f8637aad Chromium reads 144 passed, `examined 28 planted
cards, 28 pairs, 112 variants in chromium`, `scripts on, chromium: 1 datagram(s) reached the UDP
listener` and `scripts on, chromium: 2 parent message(s), origins ["null","null"]`, every message
from the card frame's own window.

The UNOBSERVABLE table measured in Chromium: `dns-prefetch` and `external-scheme` (declared for
both engines) and `preconnect` and `shadow-link` (measured), each reaching nothing from the
reference frame, which its pair asserts.

A13's green was read from the pull request's CI at b96fc07c (run 37223949189): the `hygiene` job
reads `Ran 916 tests` and `OK` in its stage log, and the `mutation-rows` job ran A13's test by
name as the killer of `S34101` and `S34102`, which read `KILLED: its killer passed without the
mutant and failed with it`.

At b96fc07c the same run's `card-sandbox` job ran the planted suite in both engines: 288 tests, 1
failed, 287 passed. Chromium read every test green, with `examined 28 planted cards, 28 pairs, 112
variants in chromium`. WebKit read `examined 28 planted cards, 28 pairs, 112 variants in webkit`,
`scripts on, webkit: 1 datagram(s) reached the UDP listener` and `scripts on, webkit: 2 parent
message(s), origins ["null","null"]`. In WebKit the `preconnect` and `shadow-link` pairs are green
and `W3 off` opens both, so W3's layer-alone channels are observable in WebKit. Its one red was the
`prefetch` pair, `nothing reached /prefetch/`: the reference frame sent no prefetch request. The
`nested-frame` and `download` cards opened with W2 off and with W4 off, as in Chromium.

23e1ef73 declared `prefetch` UNOBSERVABLE in WebKit, in `tests-card/planted.ts` and in the
schematic's web table together. Its layer alone stays `-`, since W3 and W2 each stop it.

| engine | UNOBSERVABLE, each reaching nothing from the reference frame, as its pair asserts |
|---|---|
| Chromium | `dns-prefetch`, `external-scheme` (declared for both engines); `preconnect`, `shadow-link` (measured) |
| WebKit | `dns-prefetch`, `external-scheme` (declared for both engines); `prefetch` (measured) |

## The page policy in the built page (A2's second half)

A2's Playwright half, `tests/card-policy.spec.ts` in `test:e2e`, was first read in CI at b96fc07c,
in the `web` job: red, for a reason in the test. It looked for the policy's meta element on the
page `vite preview` serves, which SvelteKit's server renders with the policy in a header, so the
page carries no meta element. The edge serves the fallback page the build writes, and the build
writes the policy there as a meta element. 4337b14 makes the test read that page, parsed by the
browser. Run locally in Chromium with a hand mutant that removed `frame-src` from
`svelte.config.js`, it was red for its own reason (the received directives held no
`frame-src 'none'`); with the file restored, its sha256 checked, it was green. The Vitest half of
A2 carries A2's fence lines, since the Playwright half was never read red at a commit.

## Mutation

StrykerJS mutated the three card files at f8637aad, with Playwright not running: 26 mutants, 26
killed, score 100.00, at a break threshold of 100 (`frame-document.ts` 22, `policy.js` 4).
`CardFrame.svelte` yields no mutant, since its script block holds no expression StrykerJS mutates;
the census (A7) and `card-frame.test.ts` (A6) hold its one sink and its sandbox. No mutant is
recorded as equivalent. `scripts/mutation-verdict.py` judged that report `verdict: ok`, `examined
26`.

The three hand rows `S34101` to `S34103` mutate `ci.yml`'s card-sandbox job and the aggregate
check's `needs`. Each `find` occurs once in `ci.yml`. Their killers live in `test_ci_workflows.py`,
so CI's `mutation-rows` job decides them.

CI read both at b96fc07c (run 37223949189). The `mutation-web` job: `Stryker examined 26 (Killed 26,
NoCoverage 0, Survived 0, Timeout 0)`, `verdict: ok`. The `mutation-rows` job: `S34101`, `S34102`
and `S34103` each `KILLED: its killer passed without the mutant and failed with it`, in `rows:
examined 46: killed 46, survived 0, void 0`.
