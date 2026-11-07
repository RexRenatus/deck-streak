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
