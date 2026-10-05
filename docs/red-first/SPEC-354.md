# Red-first record: SPEC-354

The SPEC and ADR-365 were committed first, and the census test alone next. Each criterion's test
was committed before the change that turns it green, and each red below is quoted from the run at
the red commit. The red commit also moves the sync server's deny-list pin to the census's widened
value, so that test is red there for that reason (AssertionError: Tuples differ: ('any',) != ('any', 'link-local')) and green at the fix commit. The fix commit 06b23e7 also widens data in the census's helper `scripts/tests/_units.py` (it adds `IPAddressDeny` to the alert unit's and the paging units' admitted key lists, and widens the paging values for `IPAddressDeny` from `any` to `any` and `link-local`); it adds and rewrites no assertion.

```red-first
A1: red at b51dac23: AssertionError: Lists differ: ["deploy/systemd/deck-streak-alert@.servic[637 chars]sed"] != [] | First list contains 5 additional elements.
A3: red at b51dac23: AssertionError: {'a p[13 chars]nies a range that misses the endpoint': ['depl[642 chars]ed"]} != {'a p[13 chars]nies the link-local range': [], 'the alert tem[391 chars]ed"]}
A2: not red: the refusal is the census's own helper, committed with its test; its three plants are its positive controls
A1: green at 06b23e75
A3: green at 06b23e75
```

Correction (verify round 1, ruling 395): A1's quote on line 9 joins two lines of the replay output
with ` | `. The red line itself is `AssertionError: Lists differ:
["deploy/systemd/deck-streak-alert@.servic[637 chars]sed"] != []`, and `First list contains 5
additional elements.` is the output's next line. Line 6 overstates one clause: the paging units'
admitted key list already held `IPAddressDeny`, and 06b23e7 only rewords its comment there; the fix
commit adds `IPAddressDeny` to the alert unit's admitted key list.
