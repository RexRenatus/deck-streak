# Red-first record: SPEC-355

Every criterion's test was committed before the code it judges, against stubs that compile. A5
runs locally and in CI's `hygiene` job; A1 to A3 run on the macOS host, in the `card-isolation`
job's `swift test`; A4 and A6 to A12 run only on the simulators, so each of their reds and greens
is read from the pull request's own `apple-on-change` run, from the `harness` job's `CardProbe`
step, which tests `iPhone 17` and then `iPad (A16)` in one invocation, one destination after the
other. A simulator criterion is recorded green only when its test passed on both.

A5 was committed alone (7fa48227), over a tree with no switch: run locally from `scripts/tests` it
read `Lists differ: [] != ['ios/CardIsolation/Sources/CardIsolation/CardScripts.swift']`, the one
failure of the module's two tests. At the pull request's first head (a16e84d6), which carries the
switch's stub but not the controls, run 37371806959's `hygiene` job (attempt 2, job 111990672682)
read A5 the one failure of 994 python tests, by the five L8 and L9 tokens the factory lacks.

A1 to A3 were committed over stubs that compile (436d23f7): `decide` answers `.off` always, the
removal's source is empty, and the hold answers one byte and counts nothing. Run 37371807357's
`card-isolation` job (attempt 2, job 111990678263) compiled them and read all three red by
assertion; the package's other three tests passed, and the sweep did not run because `swift test`
failed first.

A4 and A6 to A12 were committed over the factory's stubs (748c31cf): no verdict is kept, and the
view installs neither L8 nor L9, so page JavaScript stays off in every view the factory builds.
The same run's `harness` job (attempt 2, job 111993988089) compiled the probe and read every one of
them red on both simulators, each by the same assertion on both; SPEC-349's own criteria in the
suite (the single-layer variants on the seven-layer base, the render and the peer connection with
page JavaScript on) passed on both. A4's row with L7 removed, the card handed over with a base
URL, finished loading on both. On `iPhone 17` alone, A4's first row, every control with the switch
on, also never finished loading the card; on `iPad (A16)` it did.

The same run measured five scripted references blind on both simulators, each reaching nothing
while its marker was set, and the declarations were corrected from that measurement alone
(723e3020), with no criterion's words, assertion or tolerance changed:

- `script-nav` and `script-form`: the reference without L5 and L9 allowed every navigation
  (`allowed=5` and `allowed=1`) and no load arrived, as SPEC-349's `nav-self` reads without L5
  (`allowed=1`, no path): the rule list refuses the document load. Their held set gains L3.
- `script-open`: the reference without L6 created no window, and removing L6 alone opened nothing;
  SPEC-349's `nav-blank` creates a window only with L5 and L6 both removed. Its held set gains L5,
  so no control holds it alone.
- `webrtc-srcdoc-frame`: the reference without L8 sent nothing, and the scripted view without L5,
  with no L8 built, sent its datagram: the gate cancels the frame's own navigation. Its held set
  gains L5, so no control holds it alone.
- `capture`: the card's document has no `navigator.mediaDevices` (its record read
  `{"capture":"absent"}`), so no capture request is made and no held set can open it. It is the
  one blind reference left, counted and pinned as `BLIND_SCRIPTED`: A7 asserts its blind set equals
  the pin and prints the count, so a new blind reference, or this one opening, reads red. L6's
  capture arm is therefore never asked in the probe.

With `script-open` held by L5 as well, L6 holds no scripted channel alone, so `DEPTH_SCRIPTED`
gains L6. `webtransport` read `absent` on both simulators, the engine having no such interface, as
its declaration allows. The `dns-prefetch` witness was not blind: the reference's lookup reached
it once on both simulators.

```red-first
A1: red at 436d23f7: CardScriptsTests.swift:35: XCTAssertEqual failed: ("off(missing: Set([]))") is not equal to ("run") - switch on, present ["L1", "L3", "L4", "L5", "L6", "L7", "L8", "L9"] (run 37371807357 attempt 2, job card-isolation, host; Executed 1 test, with 1 failure; examined 512 cases)
A2: red at 436d23f7: PeerConnectionRemovalTests.swift:55: XCTAssertEqual failed: (21 names, first "RTCPeerConnection", "webkitRTCPeerConnection") is not equal to ("[]") - a peer-connection name survived the removal (run 37371807357 attempt 2, job card-isolation, host; Executed 1 test, with 1 failure; examined 33 planted global names)
A3: red at 436d23f7: ConnectionHoldTests.swift:43: XCTAssertEqual failed: ("1") is not equal to ("0") - attempt 1: the hold answered 1 byte(s), and so for attempts 2 and 3; ConnectionHoldTests.swift:50: ("0") is not equal to ("3") - the hold counted 0 of 3 connections (run 37371807357 attempt 2, job card-isolation, host; Executed 1 test, with 4 failures; examined 3 connection attempts)
A4: red at 748c31cf: FactoryTests.swift:90: XCTAssertEqual failed: ("nil") is not equal to ("Optional(CardIsolation.CardScripts.Verdict.run)") - every control, the switch on: the switch's verdict, and nil for each of the other nine rows; FactoryTests.swift:89: ("false") is not equal to ("true") - every control, the switch on: whether the card's script ran (run 37371807357 attempt 2, job harness, iPhone 17 and iPad (A16); on iPhone 17 also FactoryTests.swift:91: every control, the switch on: the view never finished loading the card)
A5: red at 7fa48227: AssertionError: Lists differ: [] != ['ios/CardIsolation/Sources/CardIsolation/CardScripts.swift'] (local, scripts/tests); still red at a16e84d6: Lists differ, CardWebViewFactory.swift lacks L8 (forMainFrameOnly: false), (.atDocumentStart), (in: .page) and L9 (allowFailover = false), (proxyConfigurations = [) (run 37371806959 attempt 2, job hygiene)
A6: red at 748c31cf: PlantedCardTests.swift:499: XCTAssertEqual failed: ("["nav-blank": 1, "nav-self": 1]") is not equal to ("[:]") - the cards whose card view opened a connection; PlantedCardTests.swift:510: ("0") is less than ("1") - the hold counted no refused attempt for nav-self, and for nav-blank (run 37371807357 attempt 2, job harness, iPhone 17 and iPad (A16); followed nav-self and nav-blank: without L9 connections=1)
A7: red at 748c31cf: PlantedCardTests.swift:683: XCTAssertEqual failed: (18 cards) is not equal to ("[]") - scripted cards whose script did not run in the scripted view, so their absence proves nothing; PlantedCardTests.swift:684: ("["script-nav", "script-form", "script-open", "webrtc-srcdoc-frame", "capture"]") is not equal to ("[]") - a blind reference, corrected at 723e3020 (run 37371807357 attempt 2, job harness, iPhone 17 and iPad (A16))
A8: red at 748c31cf: PlantedCardTests.swift:719: XCTAssertEqual failed: ("["webrtc-stun", "webrtc-turn-tcp", "webrtc-blank-frame", "webrtc-srcdoc-frame", "webrtc-written-frame"]") is not equal to ("[]") - peer-connection cards whose script did not run in the scripted view; PlantedCardTests.swift:720: ("["webrtc-srcdoc-frame"]") is not equal to ("[]") - a reference that sent nothing, corrected at 723e3020 (run 37371807357 attempt 2, job harness, iPhone 17 and iPad (A16))
A9: red at 748c31cf: PlantedCardTests.swift:784: XCTAssertTrue failed - the scripted card's record shows a bridge, or none; PlantedCardTests.swift:785: ("Optional(false)") is not equal to ("Optional(true)") - the scripted card's script did not run (run 37371807357 attempt 2, job harness, iPhone 17 and iPad (A16); the reference read the planted cookie and storage)
A10: red at 748c31cf: PlantedCardTests.swift:825: XCTAssertEqual failed: (measured) is not equal to (declared) - the channels each control opened when removed alone, the main frame's, blank frame's and written frame's peer connections opening under every control, every load under L3 and the srcdoc frame's under L5; PlantedCardTests.swift:827: ("[]") is not equal to (DEPTH_SCRIPTED) - the controls whose removal alone opened nothing (run 37371807357 attempt 2, job harness, iPhone 17 and iPad (A16); examined 144 single-control variants)
A11: red at 748c31cf: PlantedCardTests.swift:851: XCTAssertEqual failed: ("Optional(false)") is not equal to ("Optional(true)") - the scripted view's script did not run, so its zero proves nothing (run 37371807357 attempt 2, job harness, iPhone 17 and iPad (A16); lookup reference queries=1, shipped 0, scripted 0)
A12: red at 748c31cf: PlantedCardTests.swift:887: XCTAssertTrue failed - the scripted card's script did not run in the scripted view; PlantedCardTests.swift:888: ("Der Hund\n\nhint") is not equal to the reference's ("Der Hund\n\nthe dog\n\nhint") (run 37371807357 attempt 2, job harness, iPhone 17 and iPad (A16))
```
