# Red-first record: SPEC-692

SPEC-692's criteria's tests were committed before the code that makes them pass: A1 and
A2 at f0aef5a before 2f67452, A3 at 09efa60 before a1a280d, and A4 and A5 at 75fa7eb before
5ce236d. SPEC-692, ADR-711, the insert-only amendments of SPEC-352, ADR-363 and the lane
schematic, the changelog fragment and the rows follow in the record's commit. All five
criteria run on Linux (`test_ios_lane.py`): each red and green below was read there at
the named commit.

```red-first
A1: red at f0aef5a: AssertionError: Lists differ: [] != ['certificate shape: class der-sequence']
A1: green at 2f67452
A2: red at f0aef5a: AssertionError: Lists differ: [] != ['certificate shape: class other']
A2: green at 2f67452
A3: red at 09efa60: AssertionError: Lists differ: ['cer[49 chars] failed: class unrecognised; reading once more with -legacy'] != ['cer[49 chars] failed: class could-not-read; reading once more with -legacy']
A3: green at a1a280d
A4: red at 75fa7eb: AssertionError: Lists differ: [['op[356 chars]e', 'PBE-SHA1-3DES', '-keypbe', 'PBE-SHA1-3DES[1187 chars]db']] != [['op[356 chars]e', 'AES-256-CBC', '-keypbe', 'AES-256-CBC', '[1185 chars]db']]
A4: green at 5ce236d
A5: red at 75fa7eb: AssertionError: 1 != 2 : the keychain was offered 1 import(s)
A5: green at 5ce236d
```

SPEC-352's amended criteria A27
(`test_each_failed_certificate_read_prints_one_class_and_nothing_of_openssl`) and A28
(`test_a_failed_legacy_read_refuses_naming_both_classes_and_runs_no_further_tool`) read
red at f0aef5a for the absent shape line and green at 2f67452; no assertion was removed.
A3's test is A27's test, red again at 09efa60 for its two new cases and green at a1a280d.
The green commit `5ce236d` carries the signing step's change, and `git diff --quiet 5ce236d HEAD` over `scripts/ios_lane.py` and `scripts/tests/test_ios_lane.py` reads rc 0.
