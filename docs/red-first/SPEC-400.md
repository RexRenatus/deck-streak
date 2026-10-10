# Red-first record: SPEC-400

SPEC-400 (R1 to R10, A1 to A13), decided by ADR-414. The SPEC, ADR-414 and the browser checks were
committed first, with no change to the app. A1 and A2 are browser checks, so their red is read in
CI's `web` check run on that commit, and each line below quotes the failure from that run.

```red-first
A1: red at 4199972eb4fd1c7647d60f6d5fa3a20a98d85839: Error: /: requests to Telegram's origin / expect(received).toBe(expected) // Object.is equality / Expected: 0 / Received: 1 (CI web job 114114467968, run 38018630039, tests/telegram-launch.spec.ts:45:66); the smoke test's red at the same sha: Error: expect(locator).toHaveCount(expected) failed / Expected: 0 / Received: 1 at tests/smoke.spec.ts:29:68
A1: green at 6700208a978d617d844a58fb583531545f21a965
A2: red at 4199972eb4fd1c7647d60f6d5fa3a20a98d85839: Error: the policy refused the script / Expected: true / Received: false (same job, tests/telegram-launch.spec.ts:76:61)
A2: green at 624a54f000887b89a4b9f278cc958d20a6c2b344
A3: not red: a launch's behaviour the base already had; the script was in every page, so the launch path passed at C1 (40 passed in CI)
A4: not red: a launch's behaviour the base already had; the script was in every page, so the reload path passed at C1 (40 passed in CI)
A5: red at ef14a8f851fc8b8b8d0b23aeaf0f2fb99bbca86e: AssertionError: #tgWebAppData=x: expected false to be true // Object.is equality
A5: green at 6700208a978d617d844a58fb583531545f21a965
A6: red at ef14a8f851fc8b8b8d0b23aeaf0f2fb99bbca86e: AssertionError: expected [] to deeply equal [ Array(1) ] (telegram-launch.test.ts:82:63, the script src list)
A6: green at 6700208a978d617d844a58fb583531545f21a965
A7: red at ef14a8f851fc8b8b8d0b23aeaf0f2fb99bbca86e: AssertionError: expected [] to deeply equal [ Array(1) ] (telegram-launch.test.ts:113:24, policies() against the oracle script-src 'self' 'wasm-unsafe-eval')
A7: green at 6700208a978d617d844a58fb583531545f21a965
A8: red at ef14a8f851fc8b8b8d0b23aeaf0f2fb99bbca86e: AssertionError: expected [] to deeply equal [ Array(1) ] (telegram-launch-hook.test.ts:46:63)
A8: green at 6700208a978d617d844a58fb583531545f21a965
A9: red at ef14a8f851fc8b8b8d0b23aeaf0f2fb99bbca86e: AssertionError: expected [] to deeply equal [ 'src/lib/telegram-launch.ts' ] (examined 101 Mini App source files)
A9: green at 6700208a978d617d844a58fb583531545f21a965
A10: red at ef14a8f851fc8b8b8d0b23aeaf0f2fb99bbca86e: AssertionError: src/hooks.client.ts imports the launch module: expected false to be true // Object.is equality
A10: green at 6700208a978d617d844a58fb583531545f21a965
A11: red at ef14a8f851fc8b8b8d0b23aeaf0f2fb99bbca86e: AssertionError: expected [ { …(2) } ] to deeply equal [] (csp.test.ts:100:30)
A11: green at 6700208a978d617d844a58fb583531545f21a965
A12: not red: it pins checks the base already passed, unchanged
A13: not red: the audits open a launch and pass at the base; they guard the restatement
```

Between A1's red and green, commit 6700208 edits `web/app/tests-study/study.spec.ts`: its comment at
lines 32 to 34 only, which the file manifest names; no assertion, title or line of code in that file
changes.

Between A2's red and green, commit 624a54f edits A2's test, `web/app/tests/telegram-launch.spec.ts`,
and its helper module `web/app/tests/launch-fragment.ts`, both in the file manifest. A2 was never
green at 6700208: its last assertion counted every request to Telegram's origin and expected 0, and
the default browser reports a fetch the page's policy refuses as a request that fails with the
text `csp` and gets no response, so it read `Expected: 0 / Received: 1` at the fix (CI web job
114121010443, run 38020730472, tests/telegram-launch.spec.ts:79:22). The three policy-violation
assertions are unchanged. The count is replaced by three assertions: the requests to Telegram's
origin are exactly `[{ url: TELEGRAM_SDK, failure: 'csp', response: null }]`; the stand-in's route
received 0 requests; and 0 requests reached the origin during the page load. Run locally against
the base tree (4199972) with the edited test copied in and each assertion made soft, all three are
red: the list holds two requests, each `failure: null` with `response: 200` (the shell's script
during the load and the appended one, both served by the stand-in); the route received 2
(`Expected: 0 / Received: 2`); and the page load made 1 (`Expected: 0 / Received: 1`); beside the
unchanged `the policy refused the script / Expected: true / Received: false`. At 624a54f, A2 runs
green in the same browser: `✓ tests/telegram-launch.spec.ts:56:1 › outside Telegram the page's
policy refuses a script from Telegram's origin`, `1 passed`. SPEC-400's A2 and ADR-414's D2 are
unchanged.
