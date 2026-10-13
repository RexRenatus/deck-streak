# Red-first record: SPEC-403

SPEC-403 (R1 to R11, A1 to A20), decided by ADR-417. The first push carried the SPEC, the ADR, the
amendments and the browser checks alone, with no change to the app. A1 to A3 are browser checks,
so their red is read in CI's `web` check run on that commit, and each line below quotes the
failure from that run. A6 to A13 were read red locally against a type-clean stub of the gate, and
A14 to A18 against a tree with no `/api/launch` route, which answers 404.

```red-first
A1: red at 9deb7e22fa6249f2a7467da1d40c97721cb685f6: Error: 401: requests to Telegram's origin / expect(received).toBe(expected) // Object.is equality / Expected: 0 / Received: 1 (CI web job 114213104720, run 38052092303, tests/launch-validation.spec.ts:45:68)
A1: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A2: red at 9deb7e22fa6249f2a7467da1d40c97721cb685f6: Error: the order of the requests / expect(received).toEqual(expected) // deep equality / - "launch" / + "telegram" only (same job, tests/launch-validation.spec.ts:72:46)
A2: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A3: red at 9deb7e22fa6249f2a7467da1d40c97721cb685f6: Error: abort: requests to Telegram's origin / expect(received).toBe(expected) // Object.is equality / Expected: 0 / Received: 1 (same job, tests/launch-validation.spec.ts:87:68)
A3: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A4: not red: the reload's behaviour the base already had (the tab's mark loads the script with no request); it guards the mark's move to acceptance
A5: not red: outside a launch the base sent no request either; it guards the new request against firing outside a launch
A6: red at b947d7b35c346378c8813bd2ec1d0c1595546926: AssertionError: expected null to be 'a=1&b=2' // Object.is equality (telegram-launch.test.ts, the stub's launchData)
A6: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A7: red at b947d7b35c346378c8813bd2ec1d0c1595546926: AssertionError: expected [ <script …(2)></script> ] to deeply equal [] (telegram-launch.test.ts, the first assertion: a script before any answer)
A7: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A8: red at b947d7b35c346378c8813bd2ec1d0c1595546926: AssertionError: expected false to be true // Object.is equality (telegram-launch.test.ts, a 200 leaves the start unsettled at the stub)
A8: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A9: red at b947d7b35c346378c8813bd2ec1d0c1595546926: AssertionError: 401: expected false to be true // Object.is equality (telegram-launch.test.ts)
A9: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A10: red at b947d7b35c346378c8813bd2ec1d0c1595546926: AssertionError: 429: expected false to be true // Object.is equality (telegram-launch.test.ts)
A10: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A11: red at b947d7b35c346378c8813bd2ec1d0c1595546926: AssertionError: expected false to be true // Object.is equality (telegram-launch.test.ts)
A11: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A12: red at b947d7b35c346378c8813bd2ec1d0c1595546926: AssertionError: expected true to be false // Object.is equality (telegram-launch.test.ts, the stub reads the earlier mark as a launch)
A12: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A13: red at b947d7b35c346378c8813bd2ec1d0c1595546926: AssertionError: expected [ <script …(2)></script> ] to deeply equal [] (telegram-launch-hook.test.ts, the first assertion)
A13: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A14: red at b947d7b35c346378c8813bd2ec1d0c1595546926: assertion `left == right` failed / left: 404 / right: 204 (crates/api/tests/session_routes.rs:590)
A14: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A15: red at b947d7b35c346378c8813bd2ec1d0c1595546926: assertion `left == right` failed / left: (404, "") / right: (401, "{\"reason\":\"init_data_invalid\"}") (crates/api/tests/session_routes.rs:642)
A15: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A16: red at b947d7b35c346378c8813bd2ec1d0c1595546926: assertion `left == right` failed: POST [("content-type", "application/json"), ("sec-fetch-site", "cross-site")] / left: (404, "") / right: (403, "{\"reason\":\"cross_site_request\"}") (crates/api/tests/session_routes.rs:688)
A16: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A17: red at b947d7b35c346378c8813bd2ec1d0c1595546926: assertion `left == right` failed / left: (404, "") / right: (401, "{\"reason\":\"init_data_invalid\"}") (crates/api/tests/session_routes.rs:726)
A17: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A18: red at b947d7b35c346378c8813bd2ec1d0c1595546926: assertion `left == right` failed / left: [404 x 30] / right: [401 x 30] (crates/api/tests/session_routes.rs:754)
A18: green at 77e7af4e66a703bbd716102568ae8aec78a13e8b
A19: not red: it pins SPEC-400's tests, restated with the launch accepted
A20: not red: it guards the re-citation
```

Notes on the lines above:

- A19 is not red as a criterion, though two of the restated unit tests read red against the stub for
  the new mark key: that is A6 to A12's stub, not a second record of A19.
- C3 also carries one test the SPEC's fence does not name, `the server accepts a launch only with a
  204, and says so with a boolean`, with two assertions on the deadline's timer, added after a local
  StrykerJS run over `web/app/src/lib/telegram-launch.ts` showed four survivors. The run read
  100.00 after it, with no mutant excused.
- The `web` run on the first push failed on exactly A1, A2 and A3 and passed the other 45 browser
  tests. The `fragment` check was red there because the changelog fragment arrives in the record
  commit.

Between the reds and the greens, commit 77e7af4 (the fix) also edits
`web/app/src/lib/telegram-launch.test.ts`, the unit test file of A6 to A12 and A19: it adds the test
named in the notes above, one `expect` on the deadline's timer in A8's test, and imports the gate's
`accepts`. No assertion of A1 to A3, which live in `web/app/tests/launch-validation.spec.ts`,
changes, and no existing assertion of A6 to A12 is loosened or removed. Commit b947d7b (the unit
and route tests) is the red sha of A6 to A18 and edits no file after it but through commit 77e7af4.

Corrections to the lines above (appended; no earlier line is changed):

- The fix commit 77e7af4 adds two `expect` lines to A8's test, not one: `expect(vi.getTimerCount()).toBe(0);` and `expect(send.mock.calls[0]![1].signal?.aborted).toBe(false);`. Derived with `git diff b947d7b3 77e7af4e -- web/app/src/lib/telegram-launch.test.ts`.
- A2's red quote `+ "telegram" only` paraphrases the log. In the job's web log `"telegram"` is a diff context line: the received list was `['telegram']` and the expected list was `['launch','telegram']`.
- A12's parenthetical "the stub reads the earlier mark as a launch" names the wrong cause. The quoted `expected true to be false` is the first assertion of that test (b947d7b3, telegram-launch.test.ts:266), red because the stub's `isLaunch` never reads the NEW accepted mark. The earlier-mark half is never reached. The red is still for A12's reason.
