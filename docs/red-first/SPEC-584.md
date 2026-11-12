# Red-first record: SPEC-584

A1 to A5's tests are committed before the text they judge, and each was read red at that commit
for its criterion's reason. A6 is a guard over the absent key table, so it is not red at the base;
its two plants and row S58406 prove it can fail. Each row's proof follows the fence.

```red-first
A1: red at 89b8931d: AssertionError: 'P3D' != 'P7D'
A1: green at d04e1e59
A2: red at 89b8931d: AssertionError: '`p7d`' not found in "- **the sync server's snapshots** are copies of the sync server's store
A2: green at d04e1e59
A3: red at 89b8931d: AssertionError: '`P7D`' not found in "\n- The release that carries
A3: green at d04e1e59
A4: red at 89b8931d: AssertionError: Lists differ: ['the disk snapshots', 'the second-region copy'] != []
A4: green at d04e1e59
A5: red at 89b8931d: AssertionError: Lists differ: ['ADR-064-deckstreak-backs-up-with-its-own[517 chars]dow'] != []
A5: green at d04e1e59
A6: not red: a guard over the absent key table; its plants and row S58406 prove it can fail
```

At the red commit, A2's failure is in the sync server's snapshots bullet, A3's in the runbook's
`Before the window` section, and A5's list holds four findings, one for each of ADR-064, ADR-351,
SPEC-021 and SPEC-340, each `its last section is not its SPEC-584 amendment with the window`.

## The rows and their proofs

Each row was proved by hand in its own fixture tree made from the green commit, never in the work
tree: the target's sha256 was recorded, the row's replacement was installed once by script, the
row's killer ran alone by its test name, its first failing assertion was read, and the saved bytes
were written back, the sha256 then reading equal to the recorded one.

- `S58401` sets the backups window in `privacy.json` back to `P3D`. A1's test fails first on the
  declared window: `AssertionError: 'P3D' != 'P7D'`.
- `S58402` sets the archives' period in `PRIVACY.md` back to `P30D`. A2's test fails first on the
  sync server's snapshots bullet: `'`p7d`' not found` in it.
- `S58403` sets the period the cutover runbook checks back to `P30D`. A3's test fails first on the
  `Before the window` section: `'`P7D`' not found` in it.
- `S58404` renames the disk snapshots' bullet. A4's test fails first naming the undisclosed copy:
  `Lists differ: ['the disk snapshots'] != []`.
- `S58405` renames ADR-351's amendment heading. A5's test fails first with one finding, ADR-351's
  `its last section is not its SPEC-584 amendment with the window`.
- `S58406` adds a crypto-erase claim to the replica's bullet while no key table exists. A6's test
  fails first: `Lists differ: ['PRIVACY.md claims crypto-erase before any migration creates
  user_data_keys'] != []`.
