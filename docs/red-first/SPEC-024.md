# Red-first record: SPEC-024

The schematic was redrawn first (7c8cb12). The tests were then committed (bab6593) beside an
identity and an API whose public surface was in place and whose behaviour was stubbed: the
validator validated nothing and accepted any payload as user 0; the owner's gate admitted anyone
and logged the payload it was given, as a naive gate would; the credentials were never read; the
session store issued one constant id, remembered the last owner it opened for and never let a
session end; the cookie was a bare `deckstreak_session=<id>`; the extractor admitted every
request; and the API's cross-site bound, handshake bound and handshake body limit let everything
through. Before that commit, three tests were ordered so that each fails for its own criterion
against those stubs: A5 checks the payload whose hash leaves its signature out first, A11 reaches
its idle expiry once the stub store returns the owner it opened for, and A14's first thirty
handshakes assert only that none is throttled, their 401s checked after the thirty-first.

Every criterion was then run with the SPEC's own fenced command at bab6593: all sixteen failed by
assertion for their own criterion, each selecting one test. The implementation followed (fd245da),
where all sixteen passed. Between bab6593 and fd245da no criterion's test changed what it asserts:
four test files took lint and layout fixes only (`Duration::from_mins` and `from_hours`,
`saturating_sub`, a binding's name, `HeaderValue::as_bytes`, a type alias, a hex writer without
`format!` in a collect). One test outside the criteria, the handshake body limit's, dropped a
comparison of two constants that clippy refuses (`assertions_on_constants`); its 413 at one byte
over 16 KiB, which the shell's 2 MiB would have read, carries the same fact.

A1 failed on the user its accepted payload named, not on acceptance itself: the stub accepted
everything, so only the payload's user could show that nothing was validated or read.

```red-first
A1: red at bab6593: assertion `left == right` failed: left: TelegramUserId(0), right: TelegramUserId(4242) (the stub accepted the payload Python signed as user 0: it validated and read nothing)
A1: green at fd245da
A2: red at bab6593: a field changed after signing is refused: Caller { user: TelegramUserId(0) } (the payload with its user changed after signing was accepted)
A2: green at fd245da
A3: red at bab6593: another bot's payload is refused: Caller { user: TelegramUserId(0) }
A3: green at fd245da
A4: red at bab6593: a date outside the window is refused: Caller { user: TelegramUserId(0) } (launch data dated an hour and a second ago was accepted)
A4: green at fd245da
A5: red at bab6593: assertion `left == right` failed: left: Ok(Caller { user: TelegramUserId(0) }), right: Err(InitDataInvalid) (a payload whose hash left its signature out of the check string was accepted)
A5: green at fd245da
A6: red at bab6593: assertion `left == right` failed: left: [], right: ["init_data.rs"] (verify_slice is called nowhere in crates/identity/src)
A6: green at fd245da
A7: red at bab6593: valid, fresh launch data for another user is refused: Owner(..) (the gate admitted user 777)
A7: green at fd245da
A8: red at bab6593: the launch data reached the log: INFO deck_streak_identity::owner message=a handshake arrived payload=auth_date=1736942400&query_id=synthetic-query&user=...&hash=...
A8: green at fd245da
A9: red at bab6593: assertion `left == right` failed: left: "deckstreak_session", right: "__Host-deckstreak_session"
A9: green at fd245da
A10: red at bab6593: assertion `left == right` failed: an id was issued twice: left: 1, right: 5
A10: green at fd245da
A11: red at bab6593: assertion `left == right` failed: idle for 30 minutes: left: Some(Owner(..)), right: None
A11: green at fd245da
A12: red at bab6593: assertion `left == right` failed: left: 200, right: 401 (GET /api/me with no session answered the day)
A12: green at fd245da
A13: red at bab6593: assertion `left == right` failed: POST [("content-type", "application/json"), ("sec-fetch-site", "cross-site")]: left: (200, ""), right: (403, "{\"reason\":\"cross_site_request\"}")
A13: green at fd245da
A14: red at bab6593: assertion `left == right` failed: left: 200, right: 429 (the thirty-first handshake in the minute was judged)
A14: green at fd245da
A15: red at bab6593: assertion `left == right` failed: left: (200, "{\"study_day\":\"2025-01-14\"}"), right: (401, "{\"reason\":\"no_session\"}") (the cookie still admitted after the logout)
A15: green at fd245da
A16: red at bab6593: a missing credential refuses start: OwnerGate { key: WebAppKey(..), owner: Owner(..), freshness: Freshness { max_age: 3600s } } (the gate loaded with owner-user-id missing)
A16: green at fd245da
```


## Addendum, 2026-09-30: a log capture cannot lose a line to another thread (#461)

The killer (`crates/kernel/tests/log_capture_class.rs`) was committed at 4e81676 beside
the helper in its plain form, whose two entries make the capture exactly as the workspace's tests
did: a thread-local default and nothing else. Both tests are red there by assertion, and
deterministically: A17 runs each scenario in a child of the test binary with one test thread, so
the dispatcher registry starts empty and no other test can register a second dispatcher, and A18
counts the tree. The helper gains its floor dispatcher and the thirteen capturing calls are routed
through it at 29a8b0f, where both are green. The tests were not changed between the two commits
but for a formatter's layout of one line.

```red-first
A17: red at 4e81676: 2 of 2 scenarios lost a line another thread reached first: ["scoped", "held"] (the capture, the only dispatcher registered, was never asked about a line a thread with no subscriber reached first)
A17: green at 29a8b0f
A18: red at 4e81676: capture population: 341 file(s) read; 13 raw capture(s), 0 routed, 1 global default(s) (assertion `left == right` failed, left: 0, right: 13)
A18: green at 29a8b0f
```

At 29a8b0f the killer prints `log capture scenarios: 2 run, 0 lost the line` and `capture
population: 341 file(s) read; 0 raw capture(s), 13 routed, 1 global default(s)`. The RED count at
4e81676 is 2 of 2 tests and 13 of 13 capturing calls unrouted.

The floor was then made the global default and the killer gained two scenarios in which another
thread's registration of the callsite straddles the capture's. Over the earlier helper, the
straddled scenarios lose the line and the plain ones do not; over the new floor all four keep it.

```text
A17: red over the earlier floor: 2 of 4 scenarios lost a line another thread reached first: ["scoped-straddled", "held-straddled"]; green: 4 run, 0 lost
```

The killer then gained a precondition check and a census of every spelling. Over the earlier killer, the census escapes and two helper variants that install the floor after a capture registers stay green. Over the new killer every escape is red and both variants fail.

```text
A17, A18: over the earlier killer the escapes are green and the floor-after variants survive; over the new killer the escapes are red and the variants fail with "registered before the floor was the global default"
```

A fourth round widened the census and added two refusals. Over the earlier killer, planted escapes of each kind stayed green: a plain `mod` or a path attribute that rustc resolves elsewhere, a path named by a Cargo manifest, a doctest in a tilde, four-backtick, indented, blockquote or attribute form, a callsite registered by hand, a rebuild of the interest cache from a reload handle, and an install through `with_current_subscriber`. Over the widened killer at 4a78fad every one is red, and the controls stay green. The helper now refuses a capture nested inside another on one thread, and a capture made after the production global default. Over the helper without the nested refusal, the new test fails with the refusal's name expected and nothing found; at 1d3809c it passes, and a capture held on another thread is still admitted.

The record's fence form holds one red and one green line per criterion, and these criteria already carry theirs above, so this round's red is stated in a text fence.

```text
A18: red over the earlier killer: every planted escape green; green at 4a78fad: every planted escape red, controls green, capture population 363 file(s) read; 0 raw capture(s), 13 routed, 1 global default(s)
A17: red over the helper without the refusal: the nested capture was not refused (left "", right the refusal's name); green at 1d3809c: 4 tests run, 0 failed
```

## Correction, 2026-10-01 (#511)

The text fence above labels one line `A17: red over the helper without the refusal`. It records the red of the companion test `a_capture_nested_inside_a_capture_on_one_thread_is_refused`, which pins the helper's refusal of a nested capture. A17's own acceptance command runs a different test, `a_capture_keeps_a_line_another_thread_reached_first`, and that test's red and green are the lines of the fence above under A17. The nested-capture test is defined in `crates/kernel/tests/log_capture_class.rs`.

The passed-test count of the pull request that delivered the amendment was taken with a name filter that left the drill-named targets out and, as issue #511 records, one further test that is not a drill target. A count without that filter needs cargo, which this correction did not run. The unfiltered count is read from CI's stage log of the `rust` job on the branch `dev`: at the merge of that pull request, `816 tests run: 816 passed`, and at the current head of `dev`, `878 tests run: 878 passed`. Both runs include the drill-named targets, so neither equals the filtered local count, and the figure that is one higher than the filtered count is unmeasured here.
