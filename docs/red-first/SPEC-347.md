# Red-first record: SPEC-347

Part 1 of SPEC-347 (R1-R3, A1-A5). The SPEC, its schematic, ADR-358 and the fragment were
committed first (97dbd20a). Each criterion's test was then committed alone, before the code that
turns it green, and each red below is quoted from the run of its red commit's tests.

```red-first
A1: red at 901c9a05: assertion `left == right` failed: a login through the adapter answers with the server's host key and the endpoint it was sent; left: Err(NotAllowed { service: 1, method: 3 })
A2: red at 901c9a05: panicked at crates/ffi/tests/login.rs:111:9: a wrong password reaches the engine and the engine refuses it, not NotAllowed { service: 1, method: 3 }
A1: green at d2bb9d4c
A2: green at d2bb9d4c
A3: red at 42153266: assertion `left == right` failed: each endpoint is admitted or refused by its rule; left: [("an absent endpoint", None), ("an empty endpoint", None), ...], right: [("an absent endpoint", Some((InvalidInput, "the sync login names no endpoint"))), ("an empty endpoint", Some((InvalidInput, "the sync login names no endpoint"))), ...]
A4: red at 42153266: assertion `left == right` failed: the guard refuses the login before the engine sees it, by its rule; left: (NetworkError, "A network error occurred.\n\nError details: builder error for url ()"), right: (InvalidInput, "the sync login's endpoint is neither https nor plain http to a loopback address")
A3: green at f17ccef9
A4: green at f17ccef9
A5: not red: it is #623's parity test run unchanged, and rows S34700 and S34701 each break it
A12: red at 3cea6a34: AssertionError: Lists differ: ['ios/swift-roles.json: the register: miss[2089 chars]ted'] != []; first extra element 'ios/swift-roles.json: the register: missing': examined 28 files, 18 doors, 2 decisions, 2 admissions
A13: red at 3cea6a34: AssertionError: Lists differ: ['ios/App/Info.plist: the property list: missing', 'ios/Config/App.xcconfig: the settings: missing', 'ios/App/PrivacyInfo.xcprivacy: the privacy manifest: missing', 'ios/App/Sources/SyncCredentialStore.swift: the credential store: missing', 'ios/app.yml: the generator specs: missing'] != []: examined 0 plist keys, 0 settings, 0 privacy keys, 0 credential names, 1 generator specs
```

- A1 and A2 were read on R1's tree before rustfmt rewrapped two statements of `login.rs` below
  both failing lines; 901c9a05 differs from the tree that ran by formatting alone.
- d2bb9d4c, A1's and A2's green commit, also grows the core's table census (`tests/table.rs`): its
  native set gains the pair (1,3), an exact-set change tied to R1 and no filter. No assertion of A1
  or A2 changed between their red and their green.
- A3 and A4 ran at 42153266 over the stub guard that admitted every endpoint: A3 read the absent
  endpoint admitted, and A4 read the engine's own client refusing the scheme with its own kind, in
  the process, with no request sent. No test file changed between 42153266 and f17ccef9.

Part 2 (R4-R14, A6-A15). Its documents were committed first (a35b60ad). A12's and A13's censuses
were committed alone (3cea6a34), before the register and the app's tree they read, and each red
above is quoted from a run of the whole module from `scripts/tests` at that commit.

- A12 at 3cea6a34 failed on its live-tree subtest alone: every one of its 43 planted trees was
  accepted or refused by its rule's name. A13 failed on its live-tree subtest and on its four
  examined counts, which run after it and read 0 because the app's files did not exist.
- d40e5c97 widens A12's decision tokens: each call R11 lists also counts in its trailing-closure
  spelling (`.filter {`, `.filter{`). Three planted trees join: one accepts every call in both
  spellings at its exact count, and two each refuse one spelling by the budgets' name. Over its
  live tree A12 failed as at 3cea6a34, on the register alone, with the same counts, and its 46
  planted trees held; the widened census adds no problem to any of the 28 Swift files the tree
  then tracked. The three trees were seen red first, over the call-form tokens alone.
