# Red-first record: SPEC-061

The SPEC was promoted alone (f663988), then the tests of A1 to A7 were committed alone (f23ccbf).
There, A7 failed for its own reason, because none of the four rail-contract files existed and the
test refuses a population of zero; A1 to A6 failed because their scripts could not be opened, which
is not their criteria's reason. The next commit (936a51e) added a stub of each script, which lists,
judges and refuses nothing, and a contract that names no neutral value, and moved A2's committed
tree check after its planted refusals, so that each of A1 to A6 failed by assertion for its
criterion. A7 passed there, the stubs naming no private value. The pair lister (1f781f8) turned A1
and A2 green, the contract and the effective check (8654735) A3 to A5, and the guard check
(8c304c8) A6. Dev was merged next (8f96f5d), and the contract then named each job timer by its
template and its instance (6aa0b15), with every criterion green. Each red below was re-run from a
`git archive` export of the sha it cites.

```red-first
A1: red at 936a51e: AssertionError: Lists differ: [] != [{'unit': 'deck-streak-api.service', 'cred[168 chars]me'}] (the stub listed no pair)
A1: green at 1f781f8
A2: red at 936a51e: AssertionError: 0 != 1 : {"pairs": [], "optional": [], "examined": {"files": 0, "lines": 0}} (a planted LoadCredentialEncrypted= line was listed, not refused)
A2: green at 1f781f8
A3: red at 936a51e: AssertionError: Items in the second set but not the first: (the 14 neutral values the committed templates carry, none of them named by the contract)
A3: green at 8654735
A4: red at 936a51e: AssertionError: 0 != 1 (the API's template alone, both of its neutral values in force, passed)
A4: green at 8654735
A5: red at 936a51e: AssertionError: 0 != 1 (an effective LoadCredentialEncrypted= line, two secret-named Environment= variables, an off-socket credential and four foreign drop-ins each passed)
A5: green at 8654735
A6: red at 936a51e: AssertionError: 'examined 2 file(s)' not found in '' (the stub judged no file of a matching manifest)
A6: green at 8c304c8
A7: red at f23ccbf: AssertionError: examined 0 rail-contract file(s): the population is empty, so nothing was judged
A7: green at 936a51e
```

## Fix round

Review found that the two checks read some unit files otherwise than systemd, measured offline with
`systemd-analyze verify` and `systemd-analyze calendar`. Dev was merged first (1d8e641, with no
conflict), then the tests of A8 to A14 were committed alone (26d09bb). There A8 to A13 each failed
by assertion, because the checks passed every planted construct, and A14 passed, since the code
already reads the sync login in the `sync` job alone. Each fix then turned its criteria green in
turn: the line reading (f56357c) A8 and A9, the variables' names (d332596) A10, the section headers
(6e88634) A11, the neutral values (6d7d885) A12, and the other routes (bb7324e) A13. Each red and
green below was re-run from a `git archive` export of the sha it cites, and A1 to A7 stayed green at
each of them. The round was first committed on a branch this one supersedes; here A13's
secret-named variable is built at run time from its parts from the red commit on, so no committed
line reads to a secret scanner as a key, and every sha below is this branch's.

DISCLOSURE: A14's test body changed after its red commit (26d09bb), at f5449e7: it pairs each job's
name with its arm through one iterator instead of two stepped slices, whose text a privacy scrub
reads as an address. Its assertions and the population it examines are unchanged, and it still
fails on a planted maintenance arm that is handed the cycle and on a planted loader outside it. No
other criterion's test body changed after its red commit, and the round changed
`scripts/tests/test_deploy_templates.py` in one comment only (e68b8b6).

```red-first
A8: red at 26d09bb: AssertionError: 0 != 1 : { (a LoadCredential= from a file, hidden after a double backslash, a backslash and a space, a comment's backslash or a NUL, was listed without a refusal, and so was a byte-order mark)
A8: green at f56357c
A9: red at 26d09bb: AssertionError: 0 != 1 : effective-check: examined 1 unit(s), 2 file(s); 0 refusal(s) (a secret-named Environment= line hidden five ways, three resets systemd never reads, and a glued file header each passed)
A9: green at f56357c
A10: red at 26d09bb: AssertionError: 0 != 1 : effective-check: examined 1 unit(s), 2 file(s); 0 refusal(s) (four escaped names and one with a specifier each passed)
A10: green at d332596
A11: red at 26d09bb: AssertionError: 0 != 1 : effective-check: examined 1 unit(s), 2 file(s); 0 refusal(s) (the rail's drop-in under [ Service ] and [Service ] passed, and the lister listed a credential under [ Service ])
A11: green at 6e88634
A12: red at 26d09bb: AssertionError: 0 != 1 : effective-check: examined 1 unit(s), 2 file(s); 0 refusal(s) (seven spellings of a neutral path, eight zones that fire at UTC's instants, and a calendar with no zone each passed)
A12: green at 6d7d885
A13: red at 26d09bb: AssertionError: 0 != 1 : effective-check: examined 1 unit(s), 2 file(s); 0 refusal(s) (a secret-named PassEnvironment=, StandardInputText=, StandardInputData=, StandardInput=file: and a second EnvironmentFile= each passed)
A13: green at bb7324e
A14: not red: the census holds a fact the code already had, the runner handing the sync cycle to the sync job alone and the job role loading credentials inside it, which the rail's map now relies on
```
