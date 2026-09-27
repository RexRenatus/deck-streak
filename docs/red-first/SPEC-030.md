# Red-first record: SPEC-030

The tests were committed (f415814) before the implementation, with the temp-hygiene lint stubbed to
refuse nothing, and each criterion was run there for its own reason. A1 pins a trigger that was
already right, so it is disclosed as not red. A2's apparatus is the lint itself, so A2 was red once
the lint's rules landed (8162903) and before R5 repaired the leaking tests; the SPEC's census at
e05dfa5 named two leaking files, and the lint found a third, `scripts/tests/test_public_scrub.py`,
which R5 now repairs too (an amendment). Every sha below was re-run from a `git archive` export.

```red-first
A1: not red: ci.yml already ran on pull requests into and pushes to dev and main (SPEC-002); the test pins both triggers, and reads a planted workflow that drops main as leaving it
A2: red at 8162903: AssertionError: Lists differ: scripts/tests/test_pack_wiring.py:111, scripts/tests/test_public_scrub.py:27 and :61: tempfile.mkdtemp( and nothing in its function removes the path != []
A2: green at ea22de2
A3: red at f415814: AssertionError: Lists differ: [] != [13, 20, 27, 33, 38, 44] (the stubbed lint refused none of the six planted Rust leaks)
A3: green at 8162903
A4: red at f415814: AssertionError: Lists differ: [] != [17, 25, 30, 34, 39] (the stubbed lint refused none of the five planted Python leaks)
A4: green at 8162903
A5: red at f415814: AssertionError: Lists differ: [] != [12, 18, 23, 27] (the stubbed lint refused none of the four planted TypeScript leaks)
A5: green at 8162903
A6: red at f415814: AssertionError: Regex didn't match: '^FAIL STALE +planted-honesty:\* .*must say enforced' (the pending pack whose rows all passed exited 0)
A6: green at c559980
A7: red at f415814: AssertionError: Regex didn't match: '^FAIL STALE +planted-honesty:lifted .*#23' (the deferred row was never run)
A7: green at c559980
A8: red at f415814: AssertionError: Regex didn't match: '^ +deferred +planted-honesty:red deferred to #23; still exit 1' (no deferred row was run)
A8: green at c559980
A9: red at f415814: AssertionError: 0 != 1 : phxd pack list was called once (the runner probed its hard-coded packs without --skills-root)
A9: green at 1d96f67
A10: red at f415814: AssertionError: True is not false : .packs/VENDORED.json is vendored rule code
A10: green at 1d96f67
A11: red at f415814: AssertionError: False is not true : <0 lines for alpha> (the runner printed no line naming the unexpected red)
A11: green at 1d96f67
A12: red at f415814: AssertionError: False is not true : <0 lines for beta> (the runner had no expectation to find stale)
A12: green at 1d96f67
A13: red at f415814: AssertionError: False is not true : <0 lines for proxy-client-scan> (the runner reported the VOID scan as RED)
A13: green at 1d96f67
```

At 53f1640 the maintainer-box run with a phxd built from the vendored phoenix-v2 commit printed
`BOX PACKS OK: 11 pack(s), 3 pending`: eight packs with no unexpected red and no stale expectation
(35 expected red rows in seven packs), and seo-pipeline, ui-styles and the proxy scan pending on
#59, #60 and #29.
