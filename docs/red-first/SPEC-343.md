# Red-first record: SPEC-343

The SPEC, its schematic, ADR-354 and the context map's line were committed first, and the
manifests, the lockfile, the crate's skeleton and the tests' fakes next. Each criterion's test was
then committed before the code that turns it green, and each red below is quoted from the run at
the red commit.

```red-first
A10: red at 06e4914d: assertion `left == right` failed: left: ["http://push.synthetic.invalid", "http://push.synthetic.invalid:8080", "http://localhost.synthetic.invalid", ...] (all 13 refused origins admitted), right: []
A16: red at 06e4914d: assertion `left == right` failed: left: None, right: Some(OffTheList)
A10: green at e90b953f
A1: red at beb86050: assertion `left == right` failed: left: Failed(Request), right: Delivered
A2: red at beb86050: assertion `left == right` failed: left: Failed(Request), right: Delivered
A1: green at 7b9ca18d
A2: green at 7b9ca18d
A3: red at 8f2e2fbc: assertion `left == right` failed: 19 minutes on, the token is reused (left: a token with iat 1800001140, right: the token with iat 1800000000)
A3: green at 53852a8c
A4: red at 73a289aa: panicked: 3 request(s) did not arrive (the expired token was neither reminted nor resent)
A4: green at 90b3441d
A5: red at ac5afbbf: assertion `left == right` failed: left: (Failed(Unexpected), Failed(Unexpected)), right: (Gone { since: Some(UtcMillis(1799913600000)) }, Gone { since: Some(UtcMillis(1799996400000)) })
A5: green at 81bda77e
A6: red at a9311602: assertion `left == right` failed: left: (Failed(Unexpected), Failed(Unexpected)), right: (Rejected(Token), Rejected(Token))
A6: green at d0123026
A7: red at da6a3b00: assertion `left == right` failed: left: [Failed(Unexpected), Failed(Unexpected), Failed(Unexpected)], right: [RetryLater { after: None }, RetryLater { after: None }, RetryLater { after: None }]
A7: green at 7ff72f5a
A8: red at 6cbfda23: assertion `left == right` failed: left: Delivered, right: Rejected(TooLarge)
A8: green at b5703065
A9: red at 98e474e6: assertion `left == right` failed: a production device reaches the production service alone: left: (0, 1), right: (1, 0)
A9: green at db4a5446
A11: red at 75d30520: assertion `left == right` failed: left: Failed(Request), right: Delivered
A11: green at a4d02d47
A16: green at a4d02d47
A12: red at 7c902134: panicked: vapid t=..., k=... (the request carried no VAPID Authorization header)
A12: green at 9f1b37ea
A13: red at 8fb8bd97: assertion `left == right` failed: left: (None, None, None), right: (Some("60"), Some("normal"), Some("streak-day"))
A13: green at cc3f024d
A14: red at b27944f7: assertion `left == right` failed: left: (Failed(Unexpected), Failed(Unexpected)), right: (Gone { since: None }, Gone { since: None })
A14: green at fec66d8b
A15: red at ddc1a27d: assertion `left == right` failed: left: Failed(Unexpected), right: RetryLater { after: Some(120s) }
A15: green at 99336752
A17: red at 72bdf277: assertion `left == right` failed: left: Delivered, right: Rejected(TooLarge)
A17: green at 02ec8f8e
A18: red at c48b34eb: assertion `left == right` failed: a line per call: left: 0, right: 10
A18: green at 5d003542
A19: red at ca4c36f5: ApnsSender's Debug shows Client: ApnsSender { signer: Signer { .. }, development: Origin(..), production: Origin(..), .. }
A19: green at 15a76e08
A20: not red: the census judges the real tree and guards the crate's arrival; it has no behaviour of this delivery to be red for
A21: not red: the census judges the real tree and guards the crate's arrival; it has no behaviour of this delivery to be red for
A22: not red: the census judges the real tree and guards the crate's arrival; it has no behaviour of this delivery to be red for
A23: red at 926e100f: AssertionError: button 0 pressed: expected [] to deeply equal [ 'confirm' ]
A23: green at 79b5940f
A24: red at 926e100f: AssertionError: expected { index: +0, …(5) } to deeply equal { index: +0, …(5) } (the stub reported no pressed indices and no axes)
A24: green at 79b5940f
A25: red at 926e100f: AssertionError: again past it: expected [] to deeply equal [ 'again' ]
A25: green at 79b5940f
A26: red at 718f48eb: AssertionError: " " {}: expected null to be 'good'
A26: green at 8db2e8be
A27: red at 718f48eb: AssertionError: the control: a plain key fires: expected null to be 'good'
A27: green at 8db2e8be
A28: red at cbb7207a: AssertionError: expected [ +0, 'off' ] to deeply equal [ 1, 'requesting' ]
A28: green at 9b2c84f8
A29: red at cbb7207a: AssertionError: expected [ +0, 'off' ] to deeply equal [ 1, 'requesting' ]
A29: green at 9b2c84f8
A30: red at cbb7207a: AssertionError: expected [ +0, 'off' ] to deeply equal [ 1, 'requesting' ]
A30: green at 9b2c84f8
A31: red at cbb7207a: AssertionError: expected [ +0, 'off' ] to deeply equal [ 1, 'requesting' ]
A31: green at 9b2c84f8
A32: red at cbb7207a: AssertionError: off on want: expected { state: 'off', effect: 'none' } to deeply equal { state: 'requesting', …(1) }
A32: green at 9b2c84f8
A33: red at 866692fb: AssertionError: expected [ null, null, null, null ] to deeply equal [ Array(4) ]
A33: green at 1cb88b1d
A34: red at 866692fb: AssertionError: expected [ '/', '/about', '/badges', …(9) ] to deeply equal [ '/', '/about', '/badges', …(10) ]
A34: green at 1cb88b1d
```

A16's test was committed with A10's, before any web push code, and stayed red until A11's
implementation made the listed endpoint deliver: its red is quoted at 06e4914d and its green is
a4d02d47, so between those commits the record held a criterion red on purpose.

Beside the criteria, the tests labelled MUTATION COVERAGE hold branches no criterion examines. The
APNs set was committed at febd915b, three of them red (the device token's alphabet and length, the
error body's bound, and the rest of the status table) and green at 3136418d; the young-token,
two-calls, deadline, closed-port and key-parse tests were green at febd915b. The web push set was
committed at b8621c0f, its status table red (left: seven `Failed(Unexpected)`, right: the table)
and green at 2717f844; its collapse-key and subscription-parse tests were green at b8621c0f.

The harness's criteria were committed red the same way. A29 to A31 share A28's first line because
each test's first assertion is that the condition's rise requested the lock, which the stub never
did; each test's own assertions follow it. A34 is red only once the screen exists: its red commit
holds the screen's test and an empty page, so the route census finds a screen the table lacks, and
its green commit adds the route to the table, the path to the paths opened without a token, and the
manifest's row for that test file.

The harness's MUTATION COVERAGE tests hold branches no criterion examines. `a gamepad that leaves
is read again from a new baseline` was red at 926e100f (`the next snapshot: expected [] to deeply
equal [ 'flag' ]`) and green at 79b5940f. `one request is in flight at most, and a browser with no
wake lock reads unsupported` was red at cbb7207a (`expected 'off' to be 'cancelling'`) and green at
9b2c84f8. The screen's set (what each source logs, a disconnect's effects, the refusal's name, the
lock waiting for a gamepad, and closing the screen) and the side's moves were green when written,
at a2d7e96b. The screen's refusal test found that a `DOMException` made by another realm is no
instance of this realm's `Error`, so the holder recorded its whole text as its name; 33600830
reads the error's `name` instead, with its own coverage test of a refusal with a name, a text, a
non-string name, `undefined` and `null`.

StrykerJS over the harness's files then left mutants alive on terms that change nothing: the
stick's sign, which is one or minus one, a first snapshot's armed test, which is the unarmed one,
and a tag that is not text, which is in no set. f75abccf removes those terms, behaviour unchanged.
cef31b2a adds the coverage the rest asked for, green when written: the lock asks for the screen and
tells every change, a stick off the centre on the first snapshot fires only after it returns, and
the screen labels its parts. StrykerJS at that commit reads every mutant of the harness's files
killed.
