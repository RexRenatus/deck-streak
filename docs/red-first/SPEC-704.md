# Red-first record: SPEC-704

SPEC-704's nine criteria are tests in `scripts/tests/test_ios_lane.py`, committed alone (59e7a84)
before the change to the signing step that makes them pass (f0082fb). SPEC-704, ADR-723, this
record, the changelog fragment and the band's mutation rows follow in the record commit. Each red
and green below was read at the named commit by one run of one test,
`python3 -B -m unittest discover -s scripts/tests -p test_ios_lane.py -k <test name>`, each run
reading `Ran 1 test`; at the green commit the module as a whole read `OK`.

```red-first
A1: red at 59e7a84: AssertionError: 0 != 1 : the profile line, once
A1: green at f0082fb
A2: red at 59e7a84: AssertionError: Lists differ: [] != ['profile shape: class der-no-plist, 128 d[58 chars]bet']
A2: green at f0082fb
A3: red at 59e7a84: AssertionError: Lists differ: [] != ['profile shape: class base64-text, 102 de[57 chars]bet']
A3: green at f0082fb
A4: red at 59e7a84: AssertionError: Lists differ: [] != ['profile shape: class binary-plist, 86 de[57 chars]bet']
A4: green at f0082fb
A5: red at 59e7a84: AssertionError: 0 != 1 : the profile line, once
A5: green at f0082fb
A6: red at 59e7a84: AssertionError: Lists differ: [] != ['profile shape: class other, 49 decoded b[50 chars]bet']
A6: green at f0082fb
A7: red at 59e7a84: AssertionError: 0 != 1 : the profile line, once
A7: green at f0082fb
A8: red at 59e7a84: AssertionError: Lists differ: ['pkcs12 read failed: class wrong-password; [27 chars]acy'] != ['profile shape: class der-
A8: green at f0082fb
A9: red at 59e7a84: AssertionError: Lists differ: ['pkcs12 read failed: class wrong-password; [27 chars]acy'] != ['profile shape: class der-
A9: green at f0082fb
```

Each red is an assertion on the missing line. The tests of the four classes that refuse
(`der-no-plist`, `base64-text`, `binary-plist` and `other`) also read red, at 59e7a84, on the
refusal: standard error's last line was the parse's `AttributeError` where the refusal belongs.
The profile fixture is unchanged, and the class it reads at the green commit is `der-xml-plist`.

The green commit `f0082fb` carries the signing step's change, and `git diff --quiet f0082fb HEAD` over `scripts/ios_lane.py` and `scripts/tests/test_ios_lane.py` reads rc 0.
