# Red-first record: SPEC-064

The tests were committed with stubs that held the "before" (414142119a37f16047fbf1d1b58f36145ccd53f5):
no Litestream, backup or drill unit, a backup script and a drill script that did nothing, and a
`litestream.yml` whose replica URL named no bucket variable. The whole test file ran there. All seven
criteria failed by assertion, each for its own reason, not by a compile error or an empty selection.


The tests were amended after the red run; each amendment is named here with what it changed:
- A1: the drill's expected `ExecStart=` is the script itself, where the red test expected it under
  `/usr/bin/bash`; the backup unit's `EnvironmentFile=` is asserted empty, where the red test compared
  a placeholder with itself (stricter); and the replicator's expected `Type=` is `exec`, where it was
  `simple`.
- A2: the backups directory is asserted to hold only `.db` copies, beside the red test's exact
  listing of the state directory, which is unchanged (stricter).
- A4: the failing drill cases reach the census as a dict's items, which fixes the test's own bug, and
  a replica ahead of the live database joined them (stricter).
- A7: the host-address pattern excludes loopback (`127.`), which names no host, so a loopback address
  is no longer refused.

```red-first
A1: red at 4141421: None unexpectedly found in [None, None, None] (the three units are absent)
A2: red at 4141421: 5 != 3 (the stub backup keeps every copy)
A3: red at 4141421: 0 != 1 (the stub exits 0 on a corrupt copy)
A4: red at 4141421: the drill script holds no `integrity_check`
A5: red at 4141421: the units and the policy sentence the window needs are absent
A6: red at 4141421: the two timers whose slots are compared are absent
A7: red at 4141421: the files the census must examine are absent
A1: green at a8199c8e42c6da61f9ee272ed935375711b66904
A2: green at a8199c8e42c6da61f9ee272ed935375711b66904
A3: green at a8199c8e42c6da61f9ee272ed935375711b66904
A4: green at a8199c8e42c6da61f9ee272ed935375711b66904
A5: green at a8199c8e42c6da61f9ee272ed935375711b66904
A6: green at a8199c8e42c6da61f9ee272ed935375711b66904
A7: green at a8199c8e42c6da61f9ee272ed935375711b66904
```
