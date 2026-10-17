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

```red-first
A1: red at 8c5e0fcdf89be5801f73c850dfa14bd13c281b73: clicked cards not aimed at the TCP listener: expected [ 'nav-self', 'nav-top', …(5) ] to deeply equal []
A1: green at f34434736917fd7486529c1e28112225a53a52dc
A2: red at 8c5e0fcdf89be5801f73c850dfa14bd13c281b73: Error: nothing reached tcp, /download/ (card-sandbox job 117403743416, card-download-the-referenc-0cfd2--card-frame-reaches-nothing-chromium, 20 of the pair tests fail the same way)
A2: green at f34434736917fd7486529c1e28112225a53a52dc
A3: red at 8c5e0fcdf89be5801f73c850dfa14bd13c281b73: Error: nothing reached tcp, /nav-blank/ (card-sandbox job 117403743416, card-W1-off-nav-blank-opens-chromium, 12 of the variant tests fail the same way)
A3: green at f34434736917fd7486529c1e28112225a53a52dc
A4: not red: the planted suite as SPEC-341 and SPEC-398 keep it; at C2 it reads every clicked card's card frame empty, its connection and its path included, beside the controls that prove the counter (#761)
A5: not red: the TCP listener at 8c5e0fcdf89be5801f73c850dfa14bd13c281b73 already reads a connection with no request as tcp alone; A6 at f34434736917fd7486529c1e28112225a53a52dc is its control in the same run
A6: red at 8c5e0fcdf89be5801f73c850dfa14bd13c281b73: Error: the TCP listener served no request, Expected: 200, Received: "ECONNRESET" (card-sandbox job 117403743416, listeners-the-TCP-listener-9fdb0--reads-tcp-and-its-path-key-chromium, 3 tests fail the same way)
A6: green at f34434736917fd7486529c1e28112225a53a52dc
A7: not red: the sentinel names the HTTP listener at 8c5e0fcdf89be5801f73c850dfa14bd13c281b73 and after; A5 is its control in the same run
```

C2 is the one commit that changes a test file (`planted.ts` and `listeners.ts`) between A1 to A3's and
A6's red and their green. A1's test and `listeners.spec.ts` are unchanged since C1.

Main's control reading: the coverage test's refusing control card (`bare`, the card that declares
`tcp` alone) is not vacuous. With the `|| !declared` term deleted from the aimed-and-declared
predicate in a scratch edit, never committed, A1 alone reads red: `AssertionError: expected [
'aimed-at-requests' ] to deeply equal [ 'aimed-at-requests', …(1) ]`, the received list lacking
`aimed-at-connections`. The file was then restored byte for byte.
