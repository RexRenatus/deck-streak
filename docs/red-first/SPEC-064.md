# Red-first record: SPEC-064

The tests were committed with stubs that held the "before" (414142119a37f16047fbf1d1b58f36145ccd53f5):
no Litestream, backup or drill unit, a backup script and a drill script that did nothing, and a
`litestream.yml` whose replica URL named no bucket variable. The whole test file ran there. All seven
criteria failed by assertion, each for its own reason, not by a compile error or an empty selection.

- A1: red at 4141421: None unexpectedly found in [None, None, None] (the three units are absent)
- A2: red at 4141421: 5 != 3 (the stub backup keeps every copy)
- A3: red at 4141421: 0 != 1 (the stub exits 0 on a corrupt copy)
- A4: red at 4141421: the drill script holds no `integrity_check`
- A5: red at 4141421: the units and the policy sentence the window needs are absent
- A6: red at 4141421: the two timers whose slots are compared are absent
- A7: red at 4141421: the files the census must examine are absent

The tests were amended after the red run, and each amendment made a check stricter or fixed the test's
own bug, none weakened one: a helper to read a unit by name, a `not None` assertion on the drill's
argument vector, a dict of failing drill cases, the WAL-sidecar assertions in A2, and a loopback
exclusion in A7's host-address pattern.

- A1: green at a8199c8e42c6da61f9ee272ed935375711b66904
- A2: green at a8199c8e42c6da61f9ee272ed935375711b66904
- A3: green at a8199c8e42c6da61f9ee272ed935375711b66904
- A4: green at a8199c8e42c6da61f9ee272ed935375711b66904
- A5: green at a8199c8e42c6da61f9ee272ed935375711b66904
- A6: green at a8199c8e42c6da61f9ee272ed935375711b66904
- A7: green at a8199c8e42c6da61f9ee272ed935375711b66904
