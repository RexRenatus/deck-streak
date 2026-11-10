# Red-first record: SPEC-652

The SPEC, ADR-674, the budget file with no entry, the web job's step and the map's entry were
committed first (6996d2fd). The tests came next, whole, over a stub that keeps every input but the
behaviour (d1cbd173): it reads its two options, prints `examined 0 route(s) and the shell` and
`PASS`, reads no manifest, measures nothing, refuses nothing and finds nothing, and `report`
returns 0 at either level. 8705b855 turns them green.

Each red is quoted from one run at d1cbd173 of `python3 -B -m unittest discover -s scripts/tests -p
test_web_route_size.py`, and each green is the same command at 8705b855, the head of the first
push, which CI ran.

```red-first
A1: red at d1cbd173: AssertionError: 'web-route-size: WARN OVER BUDGET /a: gzip-9 3888 > 3887 by 1' not found in 'web-route-size: examined 0 route(s) and the shell\nweb-route-size: PASS: every route within its budget\n'
A1: green at 8705b855
A2: red at d1cbd173: AssertionError: 'web-route-size: route /b/c: 3 file(s), gzip-9 3985, budget none' not found in 'web-route-size: examined 0 route(s) and the shell\nweb-route-size: PASS: every route within its budget\n'
A2: green at 8705b855
A3: red at d1cbd173: AssertionError: 'web-route-size: WARN GONE /gone: an entry for no route; remove it' not found in 'web-route-size: examined 0 route(s) and the shell\nweb-route-size: PASS: every route within its budget\n'
A3: green at 8705b855
A4: red at d1cbd173: AssertionError: 'web-route-size: WARN RATCHET /a: gzip-9 3888, budget 4913, more than 1024 under it; lower its entry to 3888' not found in 'web-route-size: examined 0 route(s) and the shell\nweb-route-size: PASS: every route within its budget\n'
A4: green at 8705b855
A5: red at d1cbd173: AssertionError: 'web-route-size: route (shell): 6 file(s), gzip-9 4672, budget 4672' not found in 'web-route-size: examined 0 route(s) and the shell\nweb-route-size: PASS: every route within its budget\n'
A5: green at 8705b855
A6: red at d1cbd173: AssertionError: 'web-route-size: VOID: examined 0 route(s)' not found in 'web-route-size: examined 0 route(s) and the shell\nweb-route-size: PASS: every route within its budget\n' : no page
A6: green at 8705b855
A7: red at d1cbd173: AssertionError: 'web-route-size: FAIL OVER BUDGET /a: gzip-9 300 > 299 by 1' not found in 'web-route-size: examined 0 route(s) and the shell\nweb-route-size: PASS: every route within its budget\n' : fail
A7: green at 8705b855
A8: not red: the stub keeps the check's two options, its first doc line and its bytecode setting, which are its inputs, so the help, the refusals and the load read the same over the stub as over the check
```
