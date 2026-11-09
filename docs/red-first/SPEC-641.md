# Red-first record: SPEC-641

SPEC-641 (R1 to R6, A1 to A5). The SPEC, ADR-664, the schematic and the changelog fragment were
committed first and alone. The coverage test of A1 and the declared arrivals of the sixteen loading
cards that `loads()` builds (`planted.ts`, which now names `tcp` beside each card's own path, and
beside the two image-set paths of `srcset`) were committed next, over the base's aim unchanged:
every loading card still points at the HTTP listener, which counts no connection. Each red is
therefore its criterion's assertion and never a missing name.

A1 is read locally, by the Vitest line in the SPEC, and fails naming the sixteen loading cards.
The browser reds are read in CI, in the pull request's `card-sandbox` job, the only place the card
suite runs: A2 and A3 fail because each loading card reaches its path at the HTTP listener and
nothing at `tcp`. SPEC-641 section 7 predicts 65 browser reds, 47 of A2's pair tests (the WebKit
`prefetch` pair is unobservable there and passes) and 18 of A3's variant tests; every other test
passes, A5's among them.

```red-first
A1: red at 21973238495a8e0254a867e6dc626d635490c201: loading cards not aimed at the TCP listener: expected [ 'img', 'srcset', 'css-url', …(13) ] to deeply equal []
A1: green at c2bf63fa72c9818f71a1f5a4a3c0e151cffc696f
A2: red at 21973238495a8e0254a867e6dc626d635490c201: Error: nothing reached tcp, /img/ (card-sandbox job 128555326404, 47 of the 48 pair tests fail the same way; the 48th, the WebKit prefetch pair, is unobservable there and passes)
A2: green at c2bf63fa72c9818f71a1f5a4a3c0e151cffc696f
A3: red at 21973238495a8e0254a867e6dc626d635490c201: Error: nothing reached tcp, /img/ (card-sandbox job 128555326404, 18 of the 18 variant tests fail the same way)
A3: green at c2bf63fa72c9818f71a1f5a4a3c0e151cffc696f
A4: not red: the planted suite as SPEC-341, SPEC-398 and SPEC-461 keep it; at c2bf63fa its card frame readings refuse a loading card's connection as well as its request
A5: not red: each single-layer variant of a loading card that stays closed reads nothing at 21973238 and at c2bf63fa; A3 is its control in the same run
```

c2bf63fa is the one commit between the reds and their green, and it changes only `planted.ts`: the
aim of `loads()`, its two parameter names and two comments. A1's test, `card.spec.ts`, the
listeners and the harness are the same at c2bf63fa as at 21973238, and the commit that carries these
lines changes only this record.
