# Red-first record: SPEC-340

SPEC-340, ADR-351 and the hardening schematic were committed first (f070cb5e). Each finding's tests
were then committed alone, with the cure absent, and read red by assertion; the cure followed in its
own commit. A5 and A9 are decided by CI and are recorded below with the reason.

```red-first
A1: red at 77863783: AssertionError: 'a synced device is lost' not found in the rekeyed step
A1: green at d935dc21
A2: red at 433a4c28: AssertionError: Lists differ: ['deck-streak'] != ['deck-streak-sync']
A2: green at f78c18ff
A3: red at 433a4c28: AssertionError: unexpectedly None : the archive is a unit of its own (SPEC-340 R3)
A3: green at f78c18ff
A4: red at 433a4c28: AssertionError: Lists differ: [] != ['gen-20300101T030000Z.tar'] : the database's run archives nothing
A4: green at f78c18ff
A5: not red: its test runs restore-drill.sh, which no builder executes (owner ruling 238 (5)); the cure is at f78c18ff and CI decides
A6: red at dadbf128: AssertionError: None != '192.0.2.7' : a refused sync login
A6: green at a582727d
A7: red at 5b73956f: AssertionError: Lists differ: [] != [['log']] : the block's access log
A7: green at 9788085d
A8: red at 6d2cccd5: AssertionError: Lists differ: [] != [['cargo', 'deny', '--manifest-path', '/tm[145 chars]es']]
A8: green at e7be3645
A9: not red: its module is test_release_workflow.py, which no builder runs (ruling 189); the test is written at 6d2cccd5, the cure is at e7be3645 and CI decides
A10: red at 31cdfb53: AssertionError: Lists differ: [] != ['localhost']
A10: green at 908aa9df
A11: red at e6b24734: AssertionError: Lists differ: [] != [['header_up', '-Cookie'], ['header_down', '-Set-Cookie']]
A11: green at 4346e9d0
A12: red at 13c49fbf: AssertionError: Tuples differ: (0, 'SYNC_HOST=127.0.0.1...') != (1, '') : rounds below the floor
A12: green at 15e8d87c
A13: red at 77863783: AssertionError: 'os.urandom(16)' not found in the started step : started: os.urandom(16)
A13: green at d935dc21
A14: red at d5c592fe: AssertionError: '`p30d`' not found in the sync server's snapshots bullet
A14: green at 9c832f05
A15: red at 3ff60c53: AssertionError: Lists differ: [['--[66 chars]Z.tar', ...] != [['--[66 chars]Z.tar.age', ...]] : only the sealed files are copied
A15: green at 445748a0
```

The mutation rows came after the cures (9f60764d). Proving them read two survivors, and the tests
that were too weak to kill them were strengthened alone (a58b405a): A4's test now also refuses a
copy with any one of its three settings missing (S33742), and A12's test now refuses a seven-digit
round count (S34007). Both rows read KILLED after it. S34003's killer is A5's test, so CI decides it.
