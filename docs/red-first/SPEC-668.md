# Red-first record: SPEC-668

Commit 1 (b7e7b60) adds the census module with a stub `verdict` that keeps its inputs and finds
nothing: it reads no entry, lists no site and refuses nothing. It adds every planted lock, the
SPEC and the ADR. At that commit each planted leg that needs the census to find something fails by
assertion, over the planted locks alone; the clean control and the level pin pass. Commit 2
(2e1ee51) puts the census in place of the stub and adds the listing and the rows. The real-lock
leg is read in the `hygiene` job of push 1, which runs commit 2.

```red-first
A1: not red: the census over the repository's own lock, read in CI at push 1; its arms are red over the planted locks
A2: not red: a pin on the level the census starts at; its level reads warn at b7e7b60 by its text
A3: red at b7e7b60: AssertionError: 'WARN duplicate-crate-version alpha (versions): 1.0.0 registry by planted-root; 2.0.0 registry by beta' not found in ['examined 0 duplicated name(s)']
A3: green at 2e1ee51
A4: not red: the clean control; it passes at b7e7b60
A5: red at b7e7b60: AssertionError: Lists differ: ['examined 0 duplicated name(s)'] != ['examined 1 duplicated name(s)', 'LISTED duplicate-[80 chars]oot']
A5: green at 2e1ee51
A6: red at b7e7b60: AssertionError: 'WARN duplicate-crate-version delta (sources): 1.0.0 registry by planted-root; 1.0.0 git by planted-root' not found in ['examined 0 duplicated name(s)']
A6: green at 2e1ee51
A7: red at b7e7b60: AssertionError: Lists differ: [] != ['STALE duplicate-crate-version alpha']
A7: green at 2e1ee51
A8: red at b7e7b60: AssertionError: AssertionError not raised
A8: green at 2e1ee51
A9: red at b7e7b60: AssertionError: Lists differ: [] != ['UNDECIDED duplicate-crate-version planted-root: alpha']
A9: green at 2e1ee51
```

The green commit `2e1ee51` carries the census, and `git diff --quiet 2e1ee51 HEAD` over the module and its planted files reads rc 0.
