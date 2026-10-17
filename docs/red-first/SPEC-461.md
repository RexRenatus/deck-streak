# Red-first record: SPEC-461

SPEC-461 (R1 to R11, A1 to A7). The SPEC, ADR-478, the amendments to SPEC-392 and the card frame
schematic, the coverage test of A1, the listener's own spec `web/app/tests-card/listeners.spec.ts`
(A5 to A7) and the declared arrivals of the eight clicked cards (`planted.ts:71`, which now names
`tcp` beside each card's own path) were committed first and alone, over the base's test helpers
unchanged: every clicked card is still aimed at the HTTP listener, and the TCP listener still
closes each connection before it reads a request. Each red is therefore its criterion's assertion
and never a missing name.

A1 is read locally, by the Vitest line in the SPEC, and fails naming `nav-self`, `nav-top`,
`nav-blank`, `download`, `ping`, `form` and `javascript-url`. The browser reds are read in CI, in
the pull request's `card-sandbox` job, which is the only place the card suite runs: A2 and A3
fail because the clicked cards reach their path at the HTTP listener and nothing at `tcp`, and A6
fails because the TCP listener there serves no request. SPEC-461 section 7 predicts 35 browser
reds, 20 of A2's pair tests, 12 of A3's variant tests and 3 of A6's; every other test passes, A5's
and A7's among them.
