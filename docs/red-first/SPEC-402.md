# Red-first record: SPEC-402

SPEC-402 (R1 to R6, A1 to A4). The SPEC, ADR-416, the schematic's amendment and the tests of A1 to A3
were committed first, over the old strip, and each of the three read red for its own criterion. The
`img` card's two image-set forms were then re-planted as their own `srcset` card, which turns A3
green and leaves A4's single-layer variant red in the planted suite, read in `card-sandbox`. The strip
and its re-parse check follow, and the planted criteria run the Vitest census that proves the suite
opens every card in both frames. Each red below is quoted from the run at its commit.

```red-first
A1: red at 2861585538f59fb0a6277583fe12264e4561ad1c: AssertionError: expected [ ...(3) ] to deeply equal [] (the three planted elements still carry their srcset)
A1: green at e69e27cd46a8c7a7721583e0000deab0a4b646b0
A2: red at 2861585538f59fb0a6277583fe12264e4561ad1c: AssertionError: expected { Object (srcdoc) } to strictly equal { refused: 'escaped' } (the first plant, an img, was kept)
A2: green at e69e27cd46a8c7a7721583e0000deab0a4b646b0
A3: red at 2861585538f59fb0a6277583fe12264e4561ad1c: AssertionError: expected [ 'base', 'bridge', ...(26) ] to deeply equal [ 'base', 'bridge', ...(27) ] (the schematic's srcset channel has no planted card)
A3: green at c70ad382e8a68913da545f8c7d717e3ce7007155
A4: red at c70ad382e8a68913da545f8c7d717e3ce7007155: Error: srcset opened with W2 off in chromium; Error: srcset opened with W2 off in webkit (card-sandbox, W2 off: srcset stays closed, paths /srcset/1 and /srcset/2 both reached)
A4: green at e69e27cd46a8c7a7721583e0000deab0a4b646b0
A5: red at fc2535fcdbf3ed1c76cd5aba8ea3745fce598ce9: AssertionError: expected [ …(6) ] to deeply equal [] (the six planted elements still carry their target)
A5: green at 4246e2f600b2b1769084ef39aea8af84381246ec
A6: red at fc2535fcdbf3ed1c76cd5aba8ea3745fce598ce9: AssertionError: <a target="_blank" href="https://cards.example/a">a</a>: expected { Object (srcdoc) } to strictly equal { refused: 'escaped' } (the first return, an a, was kept)
A6: green at 4246e2f600b2b1769084ef39aea8af84381246ec
A7: red at fc2535fcdbf3ed1c76cd5aba8ea3745fce598ce9: AssertionError: expected [ 'base', 'bridge', …(27) ] to deeply equal [ 'base', 'bridge', …(30) ] (29 planted ids against the schematic's 32 channels: nav-blank-area, nav-blank-svg and nav-show-new have no planted card)
A7: green at 4246e2f600b2b1769084ef39aea8af84381246ec
A8: red at fc2535fcdbf3ed1c76cd5aba8ea3745fce598ce9: AssertionError: expected { …(2) } to deeply equal { missing: [], extra: [] } (the schematic's one pair, webkit nav-blank-area W3, missing from 0 declared)
A8: green at 4246e2f600b2b1769084ef39aea8af84381246ec
A9: red at 39ce2450da8114a373723d204b06665de2ab6019: Error: nav-top opened with W1 off in chromium; Error: nothing reached /nav-top/ (card-sandbox job 117619366315, 25 failed and 470 passed (25.7m), 23 of them A9's: in Chromium, WebKit and Firefox, W1 off: nav-top, nav-blank, nav-blank-area and nav-blank-svg stays closed, each Error: <card> opened with W1 off in <engine>, and W4 off: nav-top, nav-blank and nav-blank-svg opens, Error: nothing reached /nav-top/, /nav-blank/ and tcp; in Chromium and Firefox, W4 off: nav-blank-area opens, Error: nothing reached tcp)
A9: green at 4246e2f600b2b1769084ef39aea8af84381246ec
A10: red at 39ce2450da8114a373723d204b06665de2ab6019: Error: nav-blank-area reached out of the card frame in webkit; Error: nav-blank-area opened with W2 off in webkit (card-sandbox job 117619366315, 25 failed and 470 passed (25.7m), 2 of them A10's: each received { "tcp": 1 } where {} was expected)
A10: green at 4246e2f600b2b1769084ef39aea8af84381246ec
```

The red of A3 above was read from the planted suite's existing assertion over the channel set: 28
planted ids against the schematic's 29 channels. The test's new `carriers` assertion (R4) comes
after it and did not run to a verdict at the red commit, so its own red was not observed. This
record claims no new red.

ADR-498's amendment (R7 to R12, A5 to A10) was committed in the same order. A5 to A8 read red
over the old strip at `fc2535fc`, which commits the documents and the unit and census checks, each
quoted above from a run of the whole unit suite at that sha (4 failed, 682 passed, of 686). One
commit between those reds and their greens changes a test file: `39ce2450` re-plants the cards in
`web/app/tests-card/planted.ts` and reads a declared bare connection in
`web/app/tests-card/card.spec.ts`. It turns A7 and A8 green itself, since the census then finds
each schematic channel and bare connection planted (the `web` check at that sha, job 117619366472,
failed A5 and A6 alone), and it leaves A5 and A6 red, since it touches neither `frame-document.ts`
nor its test. A9 and A10 read red at `39ce2450` in `card-sandbox`, and no commit between that red
and their green changes a test file: `4246e2f6` changes `web/app/src/lib/card/frame-document.ts`
alone. Inside A9, `nav-show-new`'s cells read green at `39ce2450` by design, since it is W1's own
channel, which the strip does not move (ADR-498 D9): in job 117619366315 each of Chromium, WebKit
and Firefox passed `nav-show-new: the reference frame reaches its listener and the card frame
reaches nothing`, `W1 off: nav-show-new opens`, `W2 off: nav-show-new stays closed`, `W3 off:
nav-show-new stays closed` and `W4 off: nav-show-new stays closed`. So did the cells A10 predicts
at the base: `W3 off: nav-blank-area stays closed, a bare connection in webkit` in each engine,
and WebKit's `nav-blank-svg: the reference frame reaches its listener and the card frame reaches
nothing`.
