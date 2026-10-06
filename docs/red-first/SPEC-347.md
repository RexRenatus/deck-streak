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
A6: red at 73a6bda1: run 37395776689, job `harness-wire`: RequestBytesTests.swift:68: XCTAssertEqual failed: ("[]") is not equal to ("[10, 4, 117, 115, 101, 114, 18, 2, 112, 119, 26, 18, 104, 116, 116, 112, 115, 58, 47, 47, 115, 46, 105, 110, 118, 97, 108, 105, 100, 47]") - SyncLoginRequest; RequestBytesTests.swift:79: XCTAssertEqual failed: ("[]") is not equal to ("[10, 4, 117, 115, 101, 114, 18, 2, 112, 119, 26, 0]") - SyncLoginRequest, an empty endpoint
A7: red at 73a6bda1: run 37395776689, job `harness-wire`: ResponseDecodingTests.swift:96: XCTAssertEqual failed: ("") is not equal to ("k1") - SyncAuth's host key; ResponseDecodingTests.swift:103: XCTAssertEqual failed: ("EngineMessage(message: "", kind: 0)") is not equal to ("EngineMessage(message: "denied", kind: 7)") - BackendError; ResponseDecodingTests.swift:110: XCTAssertEqual failed: ("EngineMessage(message: "", kind: 0)") is not equal to ("EngineMessage(message: "no", kind: 0)") - BackendError, its kind omitted
A8: red at 73a6bda1: run 37395776689, job `harness`, on the iPhone and on the iPad: ShellFlowTests.swift:43: XCTAssertEqual failed: ("[]") is not equal to ("["Default"]") - A8: a fresh install lists exactly one deck, the engine's default
A9: red at 73a6bda1: run 37395776689, job `harness`, on the iPhone and on the iPad: ShellFlowTests.swift:61: XCTAssertEqual failed: ("RefusedLogin(showsTheEnginesMessage: false, namesThePassword: false, signedIn: true)") is not equal to ("RefusedLogin(showsTheEnginesMessage: true, namesThePassword: false, signedIn: false)") - A9: a refused login shows the engine's message, which names no password, and stays signed out
A10: red at 73a6bda1: run 37395776689, job `harness`, on the iPhone and on the iPad: CredentialStoreTests.swift:24: XCTAssertEqual failed: ("nil") is not equal to ("Optional("k1")") - A10: the host key round-trips through the store; CredentialStoreTests.swift:58: failed - A10: the stored item's attributes were not read: status -34018; CredentialStoreTests.swift:32: XCTAssertEqual failed: ("nil") is not equal to the stored item's attributes - A10: the stored item carries R9's accessibility and is not synchronizable
A11: red at 73a6bda1: run 37395776689, job `harness`: on the iPhone, ShellFlowTests.swift:97: XCTAssertEqual failed: ("[[false, false], [false, false]]") is not equal to ("[[true, false], [false, true]]") - A11: on the iPhone the deck list shows first, and choosing a deck shows the detail; on the iPad, ShellFlowTests.swift:89: XCTAssertEqual failed: ("[false, true]") is not equal to ("[true, true]") - A11: on the iPad the deck list and the detail show side by side
A14: red at 0199b22a: run 37390492155, job `hygiene`: AssertionError: Lists differ: ['harness: 0 steps named "the app\'s tests[270 chars]s")'] != []; first extra element 'harness: 0 steps named "the app\'s tests, Debug, on the iPhone and then the iPad", not one'
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

Part 2's macOS reds are quoted from the pull request's own run 37395776689 at 73a6bda1
(ADR-350): A6 and A7 from the `harness-wire` job's step "the codec's tests, on the host", and A8
to A11 from the `harness` job's step "the app's tests, Debug, on the iPhone and then the iPad",
which tests on the iPhone first and on the iPad second. A14's red is quoted from the `hygiene` job
of run 37390492155 at 0199b22a, which pushed A14's test (ccdf350f) before the steps it reads.

- A8 to A11 were read from the `harness` job's second attempt: its first attempt ended at an
  earlier step the job already held, before the app's steps ran. A8, A9 and A10 each failed by the
  same assertions on both devices; A11 failed on each device by its own assertion, as quoted.
- 73a6bda1 is the app's first commit (R4 to R10), whose codec and store are stubs: it opens no
  collection and stores nothing. A6 and A7 read empty bytes and empty fields; A8 read no deck; A9
  read the stub signing in; A10 read no stored host key; A11 read no row on either device. The
  split view sits in `DeckListView.swift` (`ShellView`), because the app's entry has a decision
  ceiling of 0.
- A10 also read `status -34018` (`errSecMissingEntitlement`) where it reads the stored item's
  attributes, on both devices: the simulator refuses the unsigned test host the Keychain, the
  first risk of SPEC-347 section 6, whose fallback the app's test step then takes.
- 7ea8089b, between A14's red at 0199b22a and its green, edits `test_ci_workflows.py`: it lists
  the app tree census's two parser sites in `DYNAMIC_IMPORTS`, where the manifest had named
  `NOT_WORKFLOW_READS`; the read census sums both tables in one check. No assertion of A14 changed.
- f8764907 edits `test_one_static_library.py`: its planted second generator spec moves from
  `ios/app.yml`, which R4 makes a tracked file, to `ios/planted.yml`, a path no tree holds, with
  the same plants and the same refusals. It changes no assertion of A12, A13 or A14.
- ccdf350f, A14's test alone, falls between A12's and A13's red and their green; it changes
  neither census.
- The step "the app, archived unsigned for a device" runs under
  `if: ${{ !cancelled() && steps.app-tests.outcome != 'skipped' }}`, so it is read after a red
  test step too. At 73a6bda1 the archive built (`** ARCHIVE SUCCEEDED **`), and every
  required-reason category its executable imports is declared: DiskSpace (`_fstatfs`, `_statfs`)
  and FileTimestamp (`_fstat`, `_fstatat`, `_lstat`, `_stat`), none undeclared. A15 is a build,
  not a test, so the test-selection probe decides its place in the fence (SPEC-347 section 3).
- The commit that adds this paragraph takes that fallback: the step "the app's tests, Debug, on
  the iPhone and then the iPad" signs ad hoc on its command line (`CODE_SIGN_IDENTITY=-`, no team)
  where it turned code signing off, and the archive stays unsigned. A14's test changes with it,
  after A14's green: `APP_TEST_NEEDS` asks for `CODE_SIGN_IDENTITY=-` in place of
  `CODE_SIGNING_ALLOWED=NO`, and the planted test step that turns code signing off, the step as it
  stood before this commit, is refused for lacking the ad-hoc identity. That plant is the changed
  assertion's red, read in the same run as its green. No other assertion of A14 changed.
