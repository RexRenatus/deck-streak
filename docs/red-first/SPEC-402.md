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
```
