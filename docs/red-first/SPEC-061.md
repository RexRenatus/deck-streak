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
